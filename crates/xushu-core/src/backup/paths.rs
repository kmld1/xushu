//! 备份与恢复使用的文件系统安全边界。
//!
//! 这里不把字符串前缀比较当作安全判断：目录通过已打开的句柄与重解析点属性
//! 校验，文件以不跟随符号链接的方式打开，并在读取时强制大小上限。

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::claude::path_guard::{Directories, FileAccess};
use crate::error::CoreError;

pub(crate) const COPY_BUFFER_BYTES: usize = 64 * 1024;

/// Keep one desktop process in charge of database startup and file switching.
pub fn lock_data_directory(path: &Path) -> Result<File, CoreError> {
    #[cfg(unix)]
    let directory = Directories::open_sqlite_parent(path, true).map_err(map_path_error)?;
    #[cfg(windows)]
    std::fs::create_dir_all(path)?;
    #[cfg(windows)]
    let directory = secure_directory(path)?;
    let file = directory
        .open_file("xushu.db.lock", FileAccess::Lock)
        .map_err(map_path_error)?;
    file.try_lock().map_err(|error| match error {
        std::fs::TryLockError::WouldBlock => {
            CoreError::Conflict("另一个序枢进程正在使用此数据库".to_owned())
        }
        std::fs::TryLockError::Error(error) => CoreError::Io(error),
    })?;
    Ok(file)
}

pub(crate) fn secure_directory(path: &Path) -> Result<Directories, CoreError> {
    validate_absolute(path)?;
    Directories::open(path).map_err(map_path_error)
}

pub(super) fn private_data_directory(path: &Path, create: bool) -> Result<Directories, CoreError> {
    #[cfg(unix)]
    return Directories::open_sqlite_parent(path, create).map_err(map_path_error);
    #[cfg(windows)]
    if create {
        prepare_directory(path)
    } else {
        secure_directory(path)
    }
}

/// 校验或创建用户选择的备份目标目录。
///
/// 只创建最后一级目录；父目录必须已经存在且通过安全校验。这样不会因为一次
/// 输入同时创建多层目录而绕过对已存在祖先目录的重解析点检查。
pub(crate) fn prepare_directory(path: &Path) -> Result<Directories, CoreError> {
    validate_absolute(path)?;

    match secure_directory(path) {
        Err(CoreError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {}
        result => return result,
    }

    let parent = path.parent().ok_or(CoreError::UnsafePath)?;
    let name = path.file_name().ok_or(CoreError::UnsafePath)?;
    let parent = secure_directory(parent)?;
    parent.create_child(name).map_err(map_path_error)?;
    parent.open_child(name).map_err(map_path_error)
}

pub(crate) fn validate_absolute(path: &Path) -> Result<(), CoreError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(CoreError::UnsafePath);
    }

    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if let Some(Component::Prefix(prefix)) = path.components().next() {
            match prefix.kind() {
                Prefix::UNC(_, _) | Prefix::VerbatimUNC(_, _) | Prefix::DeviceNS(_) => {
                    return Err(CoreError::UnsafePath);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

pub(super) fn map_path_error(error: io::Error) -> CoreError {
    if error.kind() == io::ErrorKind::InvalidInput {
        CoreError::UnsafePath
    } else {
        CoreError::Io(error)
    }
}

fn open_file(path: &Path, access: FileAccess) -> Result<File, CoreError> {
    let parent = path.parent().ok_or(CoreError::UnsafePath)?;
    let name = path.file_name().ok_or(CoreError::UnsafePath)?;
    secure_directory(parent)?
        .open_file(name, access)
        .map_err(map_path_error)
}

#[cfg(unix)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SqliteAccess {
    Existing,
    OpenOrCreate,
    CreateNew,
}

#[cfg(unix)]
pub(crate) fn open_checked_sqlite(
    path: &Path,
    flags: rusqlite::OpenFlags,
    access: SqliteAccess,
) -> Result<rusqlite::Connection, CoreError> {
    use rusqlite::{Connection, OpenFlags};

    let absolute;
    let path = if path.is_absolute() {
        path
    } else {
        absolute = std::env::current_dir()?.join(path);
        &absolute
    };
    let parent_path = path.parent().ok_or(CoreError::UnsafePath)?;
    let parent = Directories::open_sqlite_parent(parent_path, access == SqliteAccess::OpenOrCreate)
        .map_err(map_path_error)?;
    let name = path.file_name().ok_or(CoreError::UnsafePath)?;
    let check_auxiliary = |directory: &Directories| -> Result<bool, CoreError> {
        let mut present = false;
        for suffix in ["-journal", "-wal", "-shm"] {
            let mut side_file = name.to_os_string();
            side_file.push(suffix);
            let entry = directory
                .sqlite_file_identity(&side_file)
                .map_err(map_path_error)?;
            present |= entry.is_some();
            if entry.is_some() && access == SqliteAccess::CreateNew {
                return Err(CoreError::Conflict(
                    "备份目标已有 SQLite 辅助文件".to_owned(),
                ));
            }
        }
        Ok(present)
    };
    let auxiliary_present = check_auxiliary(&parent)?;
    let existing = parent.sqlite_file_identity(name).map_err(map_path_error)?;
    if existing.is_none() && auxiliary_present {
        // A missing main database with leftover WAL/SHM/journal is not a normal
        // state. Creating an empty database here could apply or discard those
        // files without the user's knowledge, so require manual inspection.
        return Err(CoreError::Conflict(
            "数据库主文件缺失，但存在 SQLite 辅助文件；已拒绝自动创建新数据库".to_owned(),
        ));
    }
    if existing.is_some() && access == SqliteAccess::CreateNew {
        return Err(CoreError::Conflict("备份目标文件已存在".to_owned()));
    }
    if existing.is_none() && access != SqliteAccess::Existing {
        // SQLite's CREATE flag is not exclusive. Reserve the name with O_EXCL at
        // the authorized directory instead, before entering the path-based VFS.
        match parent.open_file(name, FileAccess::CreateNew) {
            Err(error)
                if access == SqliteAccess::OpenOrCreate
                    && error.kind() == io::ErrorKind::AlreadyExists => {}
            result => drop(result.map_err(map_path_error)?),
        }
    }
    let expected = parent
        .sqlite_file_identity(name)
        .map_err(map_path_error)?
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    if existing.is_some_and(|identity| identity != expected) {
        return Err(CoreError::UnsafePath);
    }
    let connection = Connection::open_with_flags(
        parent.canonical.join(name),
        flags.difference(OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_URI)
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    let after =
        Directories::open_sqlite_parent(&parent.canonical, false).map_err(map_path_error)?;
    if after.identity != parent.identity
        || after
            .sqlite_file_identity(name)
            .map_err(map_path_error)?
            .as_ref()
            != Some(&expected)
    {
        return Err(CoreError::UnsafePath);
    }
    check_auxiliary(&after)?;
    Ok(connection)
}

pub(crate) fn read_bounded_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>, CoreError> {
    read_bounded(open_file(path, FileAccess::Read)?, max_bytes)
}

pub(super) fn read_bounded(file: File, max_bytes: u64) -> Result<Vec<u8>, CoreError> {
    let size = file.metadata()?.len();
    if size > max_bytes {
        return Err(CoreError::Validation("文件超过允许的大小上限".to_owned()));
    }
    let mut bytes = Vec::with_capacity(size as usize);
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(CoreError::Io)?;
    if bytes.len() as u64 > max_bytes {
        return Err(CoreError::Validation("文件超过允许的大小上限".to_owned()));
    }
    Ok(bytes)
}

pub(crate) fn sha256_file(path: &Path, max_bytes: u64) -> Result<(String, u64), CoreError> {
    sha256_reader(&mut open_file(path, FileAccess::Read)?, max_bytes)
}

pub(super) fn sha256_reader(file: &mut File, max_bytes: u64) -> Result<(String, u64), CoreError> {
    let size = file.metadata()?.len();
    if size > max_bytes {
        return Err(CoreError::Validation(
            "备份数据文件超过允许的大小上限".to_owned(),
        ));
    }
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    let mut total = 0_u64;
    loop {
        let read = file.read(&mut buffer).map_err(CoreError::Io)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > max_bytes {
            return Err(CoreError::Validation(
                "备份数据文件超过允许的大小上限".to_owned(),
            ));
        }
        hasher.update(&buffer[..read]);
    }
    Ok((to_hex(&hasher.finalize()), total))
}

pub(super) fn copy_bounded(
    mut source_file: File,
    destination: impl FnOnce() -> Result<File, CoreError>,
    max_bytes: u64,
) -> Result<u64, CoreError> {
    let size = source_file.metadata()?.len();
    if size > max_bytes {
        return Err(CoreError::Validation(
            "备份数据文件超过允许的大小上限".to_owned(),
        ));
    }

    let mut destination_file = destination()?;

    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    let mut total = 0_u64;
    loop {
        let read = source_file.read(&mut buffer).map_err(CoreError::Io)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > max_bytes {
            return Err(CoreError::Validation(
                "备份数据文件超过允许的大小上限".to_owned(),
            ));
        }
        destination_file
            .write_all(&buffer[..read])
            .map_err(CoreError::Io)?;
    }
    destination_file.sync_all().map_err(CoreError::Io)?;

    if total != size {
        return Err(CoreError::Backup(
            "备份文件写入不完整，源文件在复制期间发生变化".to_owned(),
        ));
    }
    Ok(total)
}

pub(super) fn write_and_sync(mut file: File, bytes: &[u8]) -> Result<(), CoreError> {
    file.write_all(bytes).map_err(CoreError::Io)?;
    file.sync_all().map_err(CoreError::Io)?;
    Ok(())
}

/// 以同目录临时文件 + 原子替换写入小型状态文件（恢复日志）。
pub(crate) fn atomic_replace_file(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let parent = path.parent().ok_or(CoreError::UnsafePath)?;
    let guard = secure_directory(parent)?;
    atomic_replace_in(
        &guard,
        path.file_name().ok_or(CoreError::UnsafePath)?,
        bytes,
    )
}

pub(super) fn atomic_replace_in(
    guard: &Directories,
    file_name: &std::ffi::OsStr,
    bytes: &[u8],
) -> Result<(), CoreError> {
    let file_name = file_name.to_str().ok_or(CoreError::UnsafePath)?;
    guard.has_regular_file(file_name).map_err(map_path_error)?;
    let temporary = format!(".{file_name}.{}.tmp", uuid::Uuid::new_v4());
    let file = guard
        .open_file(&temporary, FileAccess::CreateNew)
        .map_err(map_path_error)?;
    let result = write_and_sync(file, bytes).and_then(|()| {
        guard.has_regular_file(file_name).map_err(map_path_error)?;
        replace_in(guard, &temporary, file_name).map_err(map_path_error)
    });
    if result.is_err() {
        let _ = guard.remove_file(&temporary);
    }
    result
}

fn replace_in(guard: &Directories, temporary: &str, file_name: &str) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let from: Vec<u16> = guard
            .canonical
            .join(temporary)
            .as_os_str()
            .encode_wide()
            .chain([0])
            .collect();
        let to: Vec<u16> = guard
            .canonical
            .join(file_name)
            .as_os_str()
            .encode_wide()
            .chain([0])
            .collect();
        let moved = unsafe {
            MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if moved == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(unix)]
    {
        guard.rename_file(temporary, file_name)
    }
}

pub(crate) fn free_space_bytes(path: &Path) -> Option<u64> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

        let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let mut available = 0_u64;
        let mut total = 0_u64;
        let mut free = 0_u64;
        let ok =
            unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut free) };
        if ok == 0 {
            None
        } else {
            Some(available)
        }
    }

    #[cfg(unix)]
    {
        secure_directory(path).ok()?.free_space_bytes()
    }
}

/// 只有能确认可用空间足够时才继续。
pub(crate) fn capacity_ok(required: u64, available: Option<u64>) -> bool {
    available.is_some_and(|free| free >= required)
}

pub(super) fn remove_known_files(guard: &Directories, names: &[&str]) -> Result<(), CoreError> {
    for name in names {
        match guard.remove_file(name) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(map_path_error(error)),
        }
    }
    Ok(())
}

/// 将 Windows canonicalize 产生的 `\\?\` 前缀去掉，用于本地 UI 与登记展示。
/// 文件操作仍使用 canonical 路径。
pub(crate) fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix("\\\\?\\") {
        if !rest.starts_with("UNC\\") {
            return rest.to_owned();
        }
    }
    text.into_owned()
}

pub(crate) fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

pub(crate) fn join_relative(directory: &Path, relative: &str) -> Result<PathBuf, CoreError> {
    let candidate = Path::new(relative);
    let mut components = candidate.components();
    let single_normal_name = matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
        && !candidate.is_absolute();
    if !single_normal_name {
        return Err(CoreError::Compatibility(
            "备份清单包含非法相对路径".to_owned(),
        ));
    }
    Ok(directory.join(candidate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn capacity_rule_is_conservative() {
        assert!(capacity_ok(100, Some(100)));
        assert!(!capacity_ok(100, None));
        assert!(!capacity_ok(101, Some(100)));
    }

    #[cfg(unix)]
    #[test]
    fn unix_capacity_is_measured_on_the_open_directory() {
        let directory = TempDir::new().unwrap();
        assert!(free_space_bytes(directory.path()).is_some());
        assert!(free_space_bytes(&directory.path().join("missing")).is_none());
    }

    #[test]
    fn relative_and_parent_paths_are_rejected() {
        assert!(join_relative(Path::new("C:\\backup"), "../escape.db").is_err());
        assert!(join_relative(Path::new("C:\\backup"), "nested/xushu.db").is_err());
        assert!(join_relative(Path::new("C:\\backup"), "xushu.db").is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn windows_unc_devices_and_parent_traversal_are_rejected_without_opening_them() {
        for path in [
            r"\\server\share\backup",
            r"\\?\UNC\server\share\backup",
            r"\\.\PhysicalDrive0",
            r"D:\folder\..\backup",
        ] {
            assert!(matches!(
                validate_absolute(Path::new(path)),
                Err(CoreError::UnsafePath)
            ));
        }
    }

    #[test]
    fn bounded_file_reads_reject_oversized_files() {
        let directory = TempDir::new().expect("temp dir");
        let path = directory.path().join("data.bin");
        fs::write(&path, b"12345").expect("write file");
        assert_eq!(read_bounded_file(&path, 5).expect("read").len(), 5);
        assert!(read_bounded_file(&path, 4).is_err());
    }

    #[test]
    fn atomic_replace_keeps_a_single_complete_file() {
        let directory = TempDir::new().expect("temp dir");
        let path = directory.path().join("journal.json");
        atomic_replace_file(&path, b"first").expect("first write");
        atomic_replace_file(&path, b"second").expect("second write");
        assert_eq!(fs::read(&path).expect("read"), b"second");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn regular_reads_and_atomic_writes_reject_redirected_entries() {
        use std::os::unix::fs::symlink;

        let fixture = TempDir::new().unwrap();
        let root = fixture.path().canonicalize().unwrap();
        let outside = root.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("journal"), b"untouched").unwrap();
        symlink(&outside, root.join("directory-link")).unwrap();
        symlink(outside.join("journal"), root.join("file-link")).unwrap();

        for path in [root.join("file-link"), root.join("directory-link/journal")] {
            assert!(matches!(
                read_bounded_file(&path, 32),
                Err(CoreError::UnsafePath)
            ));
            assert!(matches!(
                atomic_replace_file(&path, b"replacement"),
                Err(CoreError::UnsafePath)
            ));
        }
        assert_eq!(fs::read(outside.join("journal")).unwrap(), b"untouched");
    }
}

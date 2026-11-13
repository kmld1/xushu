//! 备份与恢复使用的文件系统安全边界。
//!
//! 这里不把字符串前缀比较当作安全判断：目录通过已打开的句柄与重解析点属性
//! 校验，文件以不跟随符号链接的方式打开，并在读取时强制大小上限。

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::claude::path_guard::{redirected, regular_file_options, Directories};
use crate::error::CoreError;

pub(crate) const COPY_BUFFER_BYTES: usize = 64 * 1024;

/// Keep one desktop process in charge of database startup and file switching.
pub fn lock_data_directory(path: &Path) -> Result<File, CoreError> {
    let directory = secure_directory(path)?;
    let mut options = regular_file_options();
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};
        options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
    }
    let file = options
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.canonical.join("xushu.db.lock"))
        .map_err(map_path_error)?;
    let metadata = file.metadata().map_err(map_path_error)?;
    if !metadata.is_file() || redirected(&metadata) {
        return Err(CoreError::UnsafePath);
    }
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

/// 校验或创建用户选择的备份目标目录。
///
/// 只创建最后一级目录；父目录必须已经存在且通过安全校验。这样不会因为一次
/// 输入同时创建多层目录而绕过对已存在祖先目录的重解析点检查。
pub(crate) fn prepare_directory(path: &Path) -> Result<Directories, CoreError> {
    validate_absolute(path)?;

    if path.exists() {
        let metadata = fs::symlink_metadata(path).map_err(map_path_error)?;
        if !metadata.is_dir() || redirected(&metadata) {
            return Err(CoreError::UnsafePath);
        }
        return secure_directory(path);
    }

    let parent = path.parent().ok_or(CoreError::UnsafePath)?;
    let _parent = secure_directory(parent)?;
    fs::create_dir(path).map_err(map_path_error)?;
    secure_directory(path)
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

fn map_path_error(error: io::Error) -> CoreError {
    if error.kind() == io::ErrorKind::InvalidInput {
        CoreError::UnsafePath
    } else {
        CoreError::Io(error)
    }
}

fn open_regular_file(path: &Path) -> Result<(File, u64), CoreError> {
    let file = regular_file_options()
        .read(true)
        .open(path)
        .map_err(map_path_error)?;
    let metadata = file.metadata().map_err(map_path_error)?;
    if !metadata.is_file() || redirected(&metadata) {
        return Err(CoreError::UnsafePath);
    }
    Ok((file, metadata.len()))
}

pub(crate) fn read_bounded_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>, CoreError> {
    let (file, size) = open_regular_file(path)?;
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
    let (mut file, size) = open_regular_file(path)?;
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

pub(crate) fn copy_regular_file_bounded(
    source: &Path,
    destination: &Path,
    max_bytes: u64,
) -> Result<u64, CoreError> {
    let (mut source_file, size) = open_regular_file(source)?;
    if size > max_bytes {
        return Err(CoreError::Validation(
            "备份数据文件超过允许的大小上限".to_owned(),
        ));
    }

    let mut destination_file = regular_file_options()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(map_path_error)?;

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

pub(crate) fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let mut file = regular_file_options()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(map_path_error)?;
    file.write_all(bytes).map_err(CoreError::Io)?;
    file.sync_all().map_err(CoreError::Io)?;
    Ok(())
}

/// 以同目录临时文件 + 原子替换写入小型状态文件（恢复日志）。
pub(crate) fn atomic_replace_file(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let parent = path.parent().ok_or(CoreError::UnsafePath)?;
    let _guard = secure_directory(parent)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(CoreError::UnsafePath)?;
    let temporary = parent.join(format!(".{file_name}.{}.tmp", uuid::Uuid::new_v4()));
    write_new_file(&temporary, bytes)?;

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let from: Vec<u16> = temporary.as_os_str().encode_wide().chain([0]).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let moved = unsafe {
            MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if moved == 0 {
            let error = io::Error::last_os_error();
            let _ = fs::remove_file(&temporary);
            return Err(CoreError::Io(error));
        }
    }

    #[cfg(not(windows))]
    {
        fs::rename(&temporary, path).map_err(CoreError::Io)?;
    }

    Ok(())
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

    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

/// 纯函数：给定可用空间与要求空间，返回是否足够。None 表示无法查询，不阻断。
pub(crate) fn capacity_ok(required: u64, available: Option<u64>) -> bool {
    available.is_none_or(|free| free >= required)
}

pub(crate) fn remove_known_files(directory: &Path, names: &[&str]) -> Result<(), CoreError> {
    let _guard = secure_directory(directory)?;
    for name in names {
        let path = directory.join(name);
        if path.exists() {
            fs::remove_file(&path).map_err(CoreError::Io)?;
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
    use tempfile::TempDir;

    #[test]
    fn capacity_rule_is_conservative() {
        assert!(capacity_ok(100, Some(100)));
        assert!(capacity_ok(100, None));
        assert!(!capacity_ok(101, Some(100)));
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
    }
}

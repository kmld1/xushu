use std::ffi::{OsStr, OsString};
#[cfg(any(windows, test))]
use std::fs;
use std::fs::{File, Metadata};
use std::io;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use rustix::fs::{self as unix, AtFlags, Mode, OFlags};

#[cfg(target_os = "macos")]
mod macos;

pub(crate) struct Directories {
    #[cfg(windows)]
    _handles: Vec<File>,
    #[cfg(unix)]
    handle: File,
    pub identity: String,
    pub canonical: PathBuf,
}

#[derive(Clone, Copy)]
pub(crate) enum FileAccess {
    Read,
    CreateNew,
    Lock,
}

impl Directories {
    #[cfg(windows)]
    pub fn open(path: &Path) -> io::Result<Self> {
        if !path.is_absolute() || path.components().any(|part| part == Component::ParentDir) {
            return Err(unsafe_path());
        }
        let ancestors: Vec<_> = path.ancestors().collect();
        let mut handles = Vec::new();
        for directory in ancestors.into_iter().rev() {
            if directory.parent().is_none() && !directory.has_root() {
                continue;
            }
            let file = open_directory(directory)?;
            let metadata = file.metadata()?;
            if redirected(&metadata) || !metadata.is_dir() {
                return Err(unsafe_path());
            }
            handles.push(file);
        }
        let canonical = fs::canonicalize(path)?;
        if crate::git::normalize_display_path(canonical.clone())
            != crate::git::normalize_display_path(path.to_owned())
        {
            return Err(unsafe_path());
        }
        let last = handles.last().ok_or_else(unsafe_path)?;
        Ok(Self {
            identity: identity(last)?,
            canonical,
            _handles: handles,
        })
    }

    #[cfg(unix)]
    pub fn open(path: &Path) -> io::Result<Self> {
        Self::open_unix(path, false, false)
    }

    /// SQLite resolves byte-string paths throughout the connection lifetime.
    /// Only pass it paths whose directory entries are controlled by this UID or root.
    #[cfg(unix)]
    pub fn open_sqlite_parent(path: &Path, create: bool) -> io::Result<Self> {
        let directory = Self::open_unix(path, true, create)?;
        use std::os::unix::fs::MetadataExt;
        let metadata = directory.handle.metadata()?;
        if metadata.uid() != rustix::process::geteuid().as_raw() || metadata.mode() & 0o022 != 0 {
            return Err(unsafe_path());
        }
        Ok(directory)
    }

    #[cfg(unix)]
    pub fn open_backup_target(path: &Path) -> io::Result<Self> {
        Self::open_unix(path, true, false)
    }

    #[cfg(unix)]
    fn open_unix(path: &Path, sqlite: bool, create: bool) -> io::Result<Self> {
        if !path.is_absolute() || path.components().any(|part| part == Component::ParentDir) {
            return Err(unsafe_path());
        }
        let mut root = Self::from_unix(
            unix::open("/", DIRECTORY_FLAGS, Mode::empty()).map_err(unix_path_error)?,
            PathBuf::from("/"),
        )?;
        if sqlite {
            root.check_sqlite_ancestor()?;
        }
        for part in path.components() {
            match part {
                Component::RootDir => {}
                Component::Normal(name) => {
                    #[cfg(target_os = "macos")]
                    if root.canonical == Path::new("/") && system_alias(&root, name) {
                        root = root.open_child("private")?;
                        if sqlite {
                            root.check_sqlite_ancestor()?;
                        }
                    }
                    root = match root.open_child(name) {
                        Err(error) if create && error.kind() == io::ErrorKind::NotFound => {
                            match root.create_child(name) {
                                Ok(()) => {}
                                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                                Err(error) => return Err(error),
                            }
                            root.open_child(name)?
                        }
                        result => result?,
                    };
                    if sqlite {
                        root.check_sqlite_ancestor()?;
                    }
                }
                _ => return Err(unsafe_path()),
            }
        }
        Ok(root)
    }

    #[cfg(unix)]
    fn check_sqlite_ancestor(&self) -> io::Result<()> {
        use std::os::unix::fs::MetadataExt;
        let metadata = self.handle.metadata()?;
        let uid = metadata.uid();
        let mode = metadata.mode();
        // A sticky ancestor such as /private/tmp protects entries owned by root
        // or the current user. Every subsequent directory is checked in turn.
        if (uid != 0 && uid != rustix::process::geteuid().as_raw())
            || (mode & 0o022 != 0 && mode & 0o1000 == 0)
        {
            return Err(unsafe_path());
        }
        #[cfg(target_os = "macos")]
        macos::check_sqlite_directory(&self.handle)?;
        Ok(())
    }

    #[cfg(unix)]
    pub fn sqlite_file_identity(&self, name: impl AsRef<OsStr>) -> io::Result<Option<String>> {
        let name = checked_name(name.as_ref())?;
        // Do not open and close the real file solely to stat it: closing an FD to
        // the same inode can drop this process's SQLite POSIX locks.
        match unix::statat(&self.handle, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat)
                if unix::FileType::from_raw_mode(stat.st_mode) == unix::FileType::RegularFile
                    && stat.st_uid == rustix::process::geteuid().as_raw()
                    && stat.st_nlink == 1
                    && stat.st_mode & 0o022 == 0 =>
            {
                #[cfg(target_os = "macos")]
                macos::check_sqlite_file(&self.canonical.join(name), &stat)?;
                Ok(Some(format!("{:x}:{:x}", stat.st_dev, stat.st_ino)))
            }
            Ok(_) => Err(unsafe_path()),
            Err(rustix::io::Errno::NOENT) => Ok(None),
            Err(error) => Err(unix_path_error(error)),
        }
    }

    #[cfg(unix)]
    fn from_unix(fd: std::os::fd::OwnedFd, canonical: PathBuf) -> io::Result<Self> {
        let handle = File::from(fd);
        Ok(Self {
            identity: identity(&handle)?,
            canonical,
            handle,
        })
    }

    pub fn open_child(&self, name: impl AsRef<OsStr>) -> io::Result<Self> {
        let name = checked_name(name.as_ref())?;
        #[cfg(windows)]
        return Self::open(&self.canonical.join(name));
        #[cfg(unix)]
        {
            let fd = unix::openat(&self.handle, name, DIRECTORY_FLAGS, Mode::empty())
                .map_err(unix_path_error)?;
            Self::from_unix(fd, self.canonical.join(name))
        }
    }

    pub fn create_child(&self, name: impl AsRef<OsStr>) -> io::Result<()> {
        let name = checked_name(name.as_ref())?;
        #[cfg(windows)]
        return fs::create_dir(self.canonical.join(name));
        #[cfg(unix)]
        unix::mkdirat(&self.handle, name, Mode::RWXU).map_err(unix_path_error)
    }

    pub fn open_file(&self, name: impl AsRef<OsStr>, access: FileAccess) -> io::Result<File> {
        let name = checked_name(name.as_ref())?;
        #[cfg(windows)]
        let file = {
            use std::fs::OpenOptions;
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::{
                FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
            };

            let mut options = OpenOptions::new();
            options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
            match access {
                FileAccess::Read => options.read(true),
                FileAccess::CreateNew => options.write(true).create_new(true),
                FileAccess::Lock => options
                    .read(true)
                    .write(true)
                    .create(true)
                    .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE),
            };
            options.open(self.canonical.join(name))?
        };
        #[cfg(unix)]
        let file = {
            let flags = match access {
                FileAccess::Read => OFlags::RDONLY,
                FileAccess::CreateNew => OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL,
                FileAccess::Lock => OFlags::RDWR | OFlags::CREATE,
            };
            File::from(
                unix::openat(
                    &self.handle,
                    name,
                    flags | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )
                .map_err(unix_path_error)?,
            )
        };
        let metadata = file.metadata()?;
        if redirected(&metadata) || !metadata.is_file() {
            return Err(unsafe_path());
        }
        Ok(file)
    }

    pub fn has_regular_file(&self, name: impl AsRef<OsStr>) -> io::Result<bool> {
        let name = checked_name(name.as_ref())?;
        #[cfg(windows)]
        match fs::symlink_metadata(self.canonical.join(name)) {
            Ok(metadata) if metadata.is_file() && !redirected(&metadata) => Ok(true),
            Ok(_) => Err(unsafe_path()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
        #[cfg(unix)]
        match unix::statat(&self.handle, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat)
                if unix::FileType::from_raw_mode(stat.st_mode) == unix::FileType::RegularFile =>
            {
                Ok(true)
            }
            Ok(_) => Err(unsafe_path()),
            Err(rustix::io::Errno::NOENT) => Ok(false),
            Err(error) => Err(unix_path_error(error)),
        }
    }

    #[cfg(unix)]
    pub fn read_link(&self, name: impl AsRef<OsStr>) -> io::Result<PathBuf> {
        use std::os::unix::ffi::OsStrExt;
        let name = checked_name(name.as_ref())?;
        let target = unix::readlinkat(&self.handle, name, Vec::new()).map_err(unix_path_error)?;
        Ok(PathBuf::from(OsStr::from_bytes(target.as_bytes())))
    }

    pub fn rename_file(&self, from: impl AsRef<OsStr>, to: impl AsRef<OsStr>) -> io::Result<()> {
        self.move_file(from, self, to)
    }

    pub fn publish_child(&self, from: &str, to: &str) -> io::Result<()> {
        let from = checked_name(from.as_ref())?;
        let to = checked_name(to.as_ref())?;
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        return unix::renameat_with(
            &self.handle,
            from,
            &self.handle,
            to,
            unix::RenameFlags::NOREPLACE,
        )
        .map_err(unix_path_error);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::MoveFileW;
            let from: Vec<u16> = self
                .canonical
                .join(from)
                .as_os_str()
                .encode_wide()
                .chain([0])
                .collect();
            let to: Vec<u16> = self
                .canonical
                .join(to)
                .as_os_str()
                .encode_wide()
                .chain([0])
                .collect();
            // SAFETY: both pointers address live, NUL-terminated Windows paths.
            if unsafe { MoveFileW(from.as_ptr(), to.as_ptr()) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
        #[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
        self.rename_file(from, to)
    }

    pub fn move_file(
        &self,
        from: impl AsRef<OsStr>,
        destination: &Self,
        to: impl AsRef<OsStr>,
    ) -> io::Result<()> {
        let from = checked_name(from.as_ref())?;
        let to = checked_name(to.as_ref())?;
        #[cfg(windows)]
        return fs::rename(self.canonical.join(from), destination.canonical.join(to));
        #[cfg(unix)]
        unix::renameat(&self.handle, from, &destination.handle, to).map_err(unix_path_error)
    }

    pub fn remove_file(&self, name: impl AsRef<OsStr>) -> io::Result<()> {
        let name = checked_name(name.as_ref())?;
        #[cfg(windows)]
        return fs::remove_file(self.canonical.join(name));
        #[cfg(unix)]
        unix::unlinkat(&self.handle, name, AtFlags::empty()).map_err(unix_path_error)
    }

    pub fn remove_child_dir(&self, name: impl AsRef<OsStr>) -> io::Result<()> {
        let name = checked_name(name.as_ref())?;
        #[cfg(windows)]
        return fs::remove_dir(self.canonical.join(name));
        #[cfg(unix)]
        unix::unlinkat(&self.handle, name, AtFlags::REMOVEDIR).map_err(unix_path_error)
    }

    pub fn modified(&self) -> io::Result<std::time::SystemTime> {
        #[cfg(windows)]
        let handle = self._handles.last().ok_or_else(unsafe_path)?;
        #[cfg(unix)]
        let handle = &self.handle;
        handle.metadata()?.modified()
    }

    #[cfg(unix)]
    pub fn free_space_bytes(&self) -> Option<u64> {
        let stats = unix::fstatvfs(&self.handle).ok()?;
        (stats.f_frsize != 0)
            .then(|| stats.f_bavail.checked_mul(stats.f_frsize))
            .flatten()
    }

    pub fn names(&self) -> io::Result<impl Iterator<Item = io::Result<OsString>>> {
        #[cfg(windows)]
        return Ok(fs::read_dir(&self.canonical)?.map(|entry| entry.map(|entry| entry.file_name())));
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            Ok(unix::Dir::read_from(&self.handle)?
                .map(|entry| {
                    entry.map(|entry| OsStr::from_bytes(entry.file_name().to_bytes()).to_owned())
                })
                .filter(|entry| {
                    !matches!(
                        entry.as_ref().ok().and_then(|name| name.to_str()),
                        Some("." | "..")
                    )
                })
                .map(|entry| entry.map_err(io::Error::from)))
        }
    }
}

fn checked_name(name: &OsStr) -> io::Result<&OsStr> {
    let mut parts = Path::new(name).components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(component)), None) if component == name => Ok(component),
        _ => Err(unsafe_path()),
    }
}

#[cfg(unix)]
const DIRECTORY_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

#[cfg(unix)]
fn unix_path_error(error: rustix::io::Errno) -> io::Error {
    use rustix::io::Errno;
    match error {
        Errno::LOOP | Errno::NOTDIR | Errno::ISDIR => unsafe_path(),
        _ => error.into(),
    }
}

#[cfg(target_os = "macos")]
fn system_alias(root: &Directories, name: &OsStr) -> bool {
    let expected = match name.to_str() {
        Some("var") => b"private/var".as_slice(),
        Some("tmp") => b"private/tmp".as_slice(),
        Some("etc") => b"private/etc".as_slice(),
        _ => return false,
    };
    unix::readlinkat(&root.handle, name, Vec::new())
        .is_ok_and(|target| target.as_bytes() == expected)
}

pub(crate) fn unsafe_path() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "unsafe settings path")
}

pub(crate) fn redirected(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
fn open_directory(path: &Path) -> io::Result<File> {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_LIST_DIRECTORY,
        FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    OpenOptions::new()
        .access_mode(FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(windows)]
pub(crate) fn identity(file: &File) -> io::Result<String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FileIdInfo, GetFileInformationByHandleEx, FILE_ID_INFO,
    };

    let mut info = FILE_ID_INFO::default();
    // The handle and output pointer remain valid for the call.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let file_id = u128::from_le_bytes(info.FileId.Identifier);
    if file_id == 0 {
        return Err(unsafe_path());
    }
    Ok(format!("{:x}:{file_id:032x}", info.VolumeSerialNumber))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn checked_windows_ancestor_cannot_be_renamed_while_pinned() {
        let target = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .join("target");
        fs::create_dir_all(&target).unwrap();
        let fixture = tempfile::TempDir::new_in(target).unwrap();
        let parent = fixture.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();
        let guard = Directories::open(&child).unwrap();
        let moved = fixture.path().join("moved");
        assert!(fs::rename(&parent, &moved).is_err());
        drop(guard);
        fs::rename(parent, moved).unwrap();
    }
}

#[cfg(unix)]
pub(crate) fn identity(file: &File) -> io::Result<String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    Ok(format!("{:x}:{:x}", metadata.dev(), metadata.ino()))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn temp_directory_and_normalized_spelling_have_the_same_identity() {
        let fixture = tempfile::TempDir::new().unwrap();
        let real = fixture.path().canonicalize().unwrap();
        let child = real.join("child");
        fs::create_dir(&child).unwrap();

        let guard = Directories::open(&fixture.path().join("./child")).unwrap();
        assert_eq!(guard.canonical, child);
        assert_eq!(guard.identity, Directories::open(&child).unwrap().identity);
        assert!(Directories::open(&child.join("..")).is_err());
    }

    #[test]
    fn direct_and_intermediate_symlinks_are_rejected_even_inside_the_root() {
        let fixture = tempfile::TempDir::new().unwrap();
        let root = fixture.path();
        fs::create_dir_all(root.join("real/child")).unwrap();
        symlink("real", root.join("link")).unwrap();

        assert!(Directories::open(&root.join("real/child")).is_ok());
        for path in [root.join("link"), root.join("link/child")] {
            assert_eq!(
                Directories::open(&path).err().unwrap().kind(),
                io::ErrorKind::InvalidInput
            );
        }
    }

    #[test]
    fn opened_directory_cannot_be_redirected_by_replacing_its_path() {
        use std::io::Write;

        let fixture = tempfile::TempDir::new().unwrap();
        let intended = fixture.path().join("intended");
        let moved = fixture.path().join("moved");
        let outside = fixture.path().join("outside");
        fs::create_dir(&intended).unwrap();
        fs::create_dir(&outside).unwrap();
        let guard = Directories::open(&intended).unwrap();
        fs::rename(&intended, &moved).unwrap();
        symlink(&outside, &intended).unwrap();

        guard
            .open_file("temporary", FileAccess::CreateNew)
            .unwrap()
            .write_all(b"pinned")
            .unwrap();
        guard.rename_file("temporary", "final").unwrap();
        assert_eq!(fs::read(moved.join("final")).unwrap(), b"pinned");
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
        assert!(Directories::open(&intended).is_err());
        assert!(guard
            .open_file("../outside/file", FileAccess::CreateNew)
            .is_err());
        guard.remove_file("final").unwrap();
        assert!(!moved.join("final").exists());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn publishing_a_directory_preserves_an_existing_destination() {
        let fixture = tempfile::TempDir::new().unwrap();
        let root = Directories::open(fixture.path()).unwrap();
        root.create_child("staged").unwrap();
        root.create_child("published").unwrap();
        let expected = root.open_child("published").unwrap().identity;
        let error = root.publish_child("staged", "published").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(root.open_child("published").unwrap().identity, expected);
        assert!(root.open_child("staged").is_ok());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn system_tmp_alias_is_equivalent_to_private_tmp() {
        let fixture = tempfile::TempDir::new_in("/tmp").unwrap();
        let alias = Directories::open(fixture.path()).unwrap();
        let physical = Directories::open(&fixture.path().canonicalize().unwrap()).unwrap();
        assert!(alias.canonical.starts_with("/private/tmp"));
        assert_eq!(alias.identity, physical.identity);
    }
}

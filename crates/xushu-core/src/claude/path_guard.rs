use std::fs::{self, File, Metadata, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct Directories {
    // On Windows these handles deny renaming or deleting the checked directories.
    _handles: Vec<File>,
    pub identity: String,
    pub canonical: PathBuf,
}

impl Directories {
    pub fn open(path: &Path) -> io::Result<Self> {
        if !path.is_absolute() {
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

#[cfg(unix)]
fn open_directory(path: &Path) -> io::Result<File> {
    if redirected(&fs::symlink_metadata(path)?) {
        return Err(unsafe_path());
    }
    File::open(path)
}

pub(crate) fn regular_file_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
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

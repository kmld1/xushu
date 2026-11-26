use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::File;
use std::io;
use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr::{self, NonNull};

use super::unsafe_path;

// Public Darwin APIs from sys/stat.h, sys/fcntl.h and sys/acl.h.
const FILESEC_ACL: c_int = 5;
const ACL_FIRST_ENTRY: c_int = 0;
const ACL_NEXT_ENTRY: c_int = -1;
const ACL_EXTENDED_ALLOW: c_int = 1;
const ACL_EXTENDED_DENY: c_int = 2;
const ACL_READ_DATA: u64 = 1 << 1;
const ACL_EXECUTE: u64 = 1 << 3;
const ACL_READ_ATTRIBUTES: u64 = 1 << 7;
const ACL_READ_EXTATTRIBUTES: u64 = 1 << 9;
const ACL_READ_SECURITY: u64 = 1 << 11;
const ACL_SYNCHRONIZE: u64 = 1 << 20;
const ACL_READ_ONLY: u64 = ACL_READ_DATA
    | ACL_EXECUTE
    | ACL_READ_ATTRIBUTES
    | ACL_READ_EXTATTRIBUTES
    | ACL_READ_SECURITY
    | ACL_SYNCHRONIZE;

unsafe extern "C" {
    fn filesec_init() -> *mut c_void;
    fn filesec_free(security: *mut c_void);
    fn filesec_query_property(security: *mut c_void, property: c_int, present: *mut c_int)
        -> c_int;
    fn filesec_get_property(security: *mut c_void, property: c_int, value: *mut c_void) -> c_int;
    #[cfg_attr(target_arch = "x86_64", link_name = "fstatx_np$INODE64")]
    fn fstatx_np(fd: c_int, stat: *mut libc::stat, security: *mut c_void) -> c_int;
    #[cfg_attr(target_arch = "x86_64", link_name = "lstatx_np$INODE64")]
    fn lstatx_np(path: *const c_char, stat: *mut libc::stat, security: *mut c_void) -> c_int;
    fn acl_free(acl: *mut c_void) -> c_int;
    fn acl_valid(acl: *mut c_void) -> c_int;
    fn acl_get_entry(acl: *mut c_void, id: c_int, entry: *mut *mut c_void) -> c_int;
    fn acl_get_tag_type(entry: *mut c_void, tag: *mut c_int) -> c_int;
    fn acl_get_permset_mask_np(entry: *mut c_void, mask: *mut u64) -> c_int;
}

struct Security(NonNull<c_void>);

impl Security {
    fn new() -> io::Result<Self> {
        // SAFETY: no arguments. A successful allocation is owned by Security.
        NonNull::new(unsafe { filesec_init() })
            .map(Self)
            .ok_or_else(io::Error::last_os_error)
    }

    fn check_acl(&self) -> io::Result<()> {
        let mut present = 0;
        // SAFETY: self owns a live filesec and present is a writable C int.
        check(unsafe { filesec_query_property(self.0.as_ptr(), FILESEC_ACL, &mut present) })?;
        if present == 0 {
            return Ok(());
        }
        let mut acl: *mut c_void = ptr::null_mut();
        // SAFETY: FILESEC_ACL writes an owned acl_t to this pointer slot.
        check(unsafe {
            filesec_get_property(self.0.as_ptr(), FILESEC_ACL, ptr::from_mut(&mut acl).cast())
        })?;
        let acl = Acl(NonNull::new(acl).ok_or_else(unsafe_path)?);
        acl.check()
    }
}

impl Drop for Security {
    fn drop(&mut self) {
        // SAFETY: the allocation from filesec_init is freed once.
        unsafe { filesec_free(self.0.as_ptr()) };
    }
}

struct Acl(NonNull<c_void>);

impl Acl {
    fn check(&self) -> io::Result<()> {
        // SAFETY: self owns a live ACL. Entries are borrowed while it is alive.
        unsafe {
            check(acl_valid(self.0.as_ptr()))?;
            let mut index = ACL_FIRST_ENTRY;
            loop {
                let mut entry = ptr::null_mut();
                if acl_get_entry(self.0.as_ptr(), index, &mut entry) != 0 {
                    let error = io::Error::last_os_error();
                    // Darwin returns EINVAL at the end of a valid ACL.
                    return if error.raw_os_error() == Some(libc::EINVAL) {
                        Ok(())
                    } else {
                        Err(error)
                    };
                }
                let mut tag = 0;
                check(acl_get_tag_type(entry, &mut tag))?;
                match tag {
                    ACL_EXTENDED_DENY => {}
                    ACL_EXTENDED_ALLOW => {
                        let mut mask = 0;
                        check(acl_get_permset_mask_np(entry, &mut mask))?;
                        if mask & !ACL_READ_ONLY != 0 {
                            return Err(unsafe_path());
                        }
                    }
                    _ => return Err(unsafe_path()),
                }
                index = ACL_NEXT_ENTRY;
            }
        }
    }
}

impl Drop for Acl {
    fn drop(&mut self) {
        // SAFETY: this is the owned ACL returned by filesec_get_property.
        unsafe { acl_free(self.0.as_ptr()) };
    }
}

fn check(result: c_int) -> io::Result<()> {
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub(super) fn check_sqlite_directory(directory: &File) -> io::Result<()> {
    let mut filesystem = MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: the borrowed File keeps the fd alive for these calls.
    check(unsafe { libc::fstatfs(directory.as_raw_fd(), filesystem.as_mut_ptr()) })?;
    // SAFETY: fstatfs succeeded and initialized the structure.
    let flags = unsafe { filesystem.assume_init() }.f_flags;
    if flags & libc::MNT_IGNORE_OWNERSHIP as u32 != 0 || flags & libc::MNT_LOCAL as u32 == 0 {
        return Err(unsafe_path());
    }
    let security = Security::new()?;
    let mut stat = MaybeUninit::uninit();
    // SAFETY: stat is writable and the fd and filesec remain live.
    check(unsafe {
        fstatx_np(
            directory.as_raw_fd(),
            stat.as_mut_ptr(),
            security.0.as_ptr(),
        )
    })?;
    security.check_acl()
}

pub(super) fn check_sqlite_file(path: &Path, expected: &rustix::fs::Stat) -> io::Result<()> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| unsafe_path())?;
    let security = Security::new()?;
    let mut stat = MaybeUninit::uninit();
    // SAFETY: the path is NUL terminated, stat is writable and the filesec is live.
    check(unsafe { lstatx_np(path.as_ptr(), stat.as_mut_ptr(), security.0.as_ptr()) })?;
    // SAFETY: lstatx_np succeeded and initialized the structure.
    let stat = unsafe { stat.assume_init() };
    if stat.st_dev != expected.st_dev
        || stat.st_ino != expected.st_ino
        || stat.st_uid != expected.st_uid
        || stat.st_mode != expected.st_mode
        || stat.st_nlink != expected.st_nlink
    {
        return Err(unsafe_path());
    }
    security.check_acl()
}

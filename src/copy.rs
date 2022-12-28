//! All of the code contained in this file has been extracted directly from the Rust Standard
//! Library solely so that we can make a few small tweaks to `std::fs::copy` in order to avoid
//! additional syscalls (namely `statx`/`fstat64` and `fchmodat`) which are unnecessary for our
//! use-case. See `open_to_and_set_permissions`, which contains the critical change.

#[rustfmt::skip] // Skip formatting to make this easier to rebase on upstream

use std::io;
use nix::libc;
use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::slice;

// ============= Begin std/src/sys/unix/mod.rs =============

fn cvt(t: libc::c_int) -> crate::io::Result<libc::c_int> {
    if t == -1 {
        Err(crate::io::Error::last_os_error())
    } else {
        Ok(t)
    }
}

// ============= Begin std/src/sys/common/small_c_string.rs =============

// Make sure to stay under 4096 so the compiler doesn't insert a probe frame:
// https://docs.rs/compiler_builtins/latest/compiler_builtins/probestack/index.html
#[cfg(not(target_os = "espidf"))]
const MAX_STACK_ALLOCATION: usize = 384;
#[cfg(target_os = "espidf")]
const MAX_STACK_ALLOCATION: usize = 32;

#[allow(non_snake_case)]
fn NUL_ERR() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "file name contained an unexpected NUL byte",
    )
}

#[inline]
fn run_path_with_cstr<T, F>(path: &Path, f: F) -> io::Result<T>
where
    F: FnOnce(&CStr) -> io::Result<T>,
{
    run_with_cstr(path.as_os_str().as_bytes(), f)
}

#[inline]
fn run_with_cstr<T, F>(bytes: &[u8], f: F) -> io::Result<T>
where
    F: FnOnce(&CStr) -> io::Result<T>,
{
    if bytes.len() >= MAX_STACK_ALLOCATION {
        return run_with_cstr_allocating(bytes, f);
    }

    let mut buf = MaybeUninit::<[u8; MAX_STACK_ALLOCATION]>::uninit();
    let buf_ptr = buf.as_mut_ptr() as *mut u8;

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), buf_ptr, bytes.len());
        buf_ptr.add(bytes.len()).write(0);
    }

    match CStr::from_bytes_with_nul(unsafe { slice::from_raw_parts(buf_ptr, bytes.len() + 1) }) {
        Ok(s) => f(s),
        Err(_) => Err(NUL_ERR()),
    }
}

#[cold]
#[inline(never)]
fn run_with_cstr_allocating<T, F>(bytes: &[u8], f: F) -> io::Result<T>
where
    F: FnOnce(&CStr) -> io::Result<T>,
{
    match CString::new(bytes) {
        Ok(s) => f(&s),
        Err(_) => Err(NUL_ERR()),
    }
}

// ============= Begin std/src/sys/unix/fs.rs =============

fn open_from(from: &Path) -> io::Result<(std::fs::File, std::fs::Metadata)> {
    use std::fs::File;

    let reader = File::open(from)?;
    let metadata = reader.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the source path is neither a regular file nor a symlink to a regular file",
        ));
    }
    Ok((reader, metadata))
}

#[cfg(target_os = "espidf")]
fn open_to_and_set_permissions(
    to: &Path,
    reader_metadata: std::fs::Metadata,
) -> io::Result<std::fs::File> {
    use std::fs::OpenOptions;
    let writer = OpenOptions::new().open(to)?;
    Ok(writer)
}

#[cfg(not(target_os = "espidf"))]
fn open_to_and_set_permissions(
    to: &Path,
    reader_metadata: std::fs::Metadata,
) -> io::Result<std::fs::File> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let perm = reader_metadata.permissions();
    let writer = OpenOptions::new()
        // create the file with the correct mode right away
        .mode(perm.mode())
        .write(true)
        .create(true)
        .truncate(true)
        .open(to)?;

    // IMPORTANT: Commenting out the below codeblock is the entire motivation for vendoring this
    // functionality from the standard library. We are always creating new files (and if we happen
    // to be racing with another process to create new files then the user has problems outside of
    // our control anyway), so there's no need to incur the cost of the additional
    // `statx`/`fstat64` and `fchmodat` syscalls that the below codeblock performs.

    /*
    let writer_metadata = writer.metadata()?;
    if writer_metadata.is_file() {
        // Set the correct file permissions, in case the file already existed.
        // Don't set the permissions on already existing non-files like
        // pipes/FIFOs or device nodes.
        writer.set_permissions(perm)?;
    }
    */
    Ok(writer)
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    target_os = "watchos",
)))]
pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    let (mut reader, reader_metadata) = open_from(from)?;
    let mut writer = open_to_and_set_permissions(to, reader_metadata)?;

    io::copy(&mut reader, &mut writer)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    let (mut reader, reader_metadata) = open_from(from)?;
    let max_len = u64::MAX;
    let mut writer = open_to_and_set_permissions(to, reader_metadata)?;

    use super::kernel_copy::{copy_regular_files, CopyResult};

    match copy_regular_files(reader.as_raw_fd(), writer.as_raw_fd(), max_len) {
        CopyResult::Ended(bytes) => Ok(bytes),
        CopyResult::Error(e, _) => Err(e),
        CopyResult::Fallback(written) => match io::copy::generic_copy(&mut reader, &mut writer) {
            Ok(bytes) => Ok(bytes + written),
            Err(e) => Err(e),
        },
    }
}

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "watchos"))]
pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    use std::sync::atomic::{AtomicBool, Ordering};

    const COPYFILE_ACL: u32 = 1 << 0;
    const COPYFILE_STAT: u32 = 1 << 1;
    const COPYFILE_XATTR: u32 = 1 << 2;
    const COPYFILE_DATA: u32 = 1 << 3;

    const COPYFILE_SECURITY: u32 = COPYFILE_STAT | COPYFILE_ACL;
    const COPYFILE_METADATA: u32 = COPYFILE_SECURITY | COPYFILE_XATTR;
    const COPYFILE_ALL: u32 = COPYFILE_METADATA | COPYFILE_DATA;

    const COPYFILE_STATE_COPIED: u32 = 8;

    #[allow(non_camel_case_types)]
    type copyfile_state_t = *mut libc::c_void;
    #[allow(non_camel_case_types)]
    type copyfile_flags_t = u32;

    extern "C" {
        fn fcopyfile(
            from: libc::c_int,
            to: libc::c_int,
            state: copyfile_state_t,
            flags: copyfile_flags_t,
        ) -> libc::c_int;
        fn copyfile_state_alloc() -> copyfile_state_t;
        fn copyfile_state_free(state: copyfile_state_t) -> libc::c_int;
        fn copyfile_state_get(
            state: copyfile_state_t,
            flag: u32,
            dst: *mut libc::c_void,
        ) -> libc::c_int;
    }

    struct FreeOnDrop(copyfile_state_t);
    impl Drop for FreeOnDrop {
        fn drop(&mut self) {
            // The code below ensures that `FreeOnDrop` is never a null pointer
            unsafe {
                // `copyfile_state_free` returns -1 if the `to` or `from` files
                // cannot be closed. However, this is not considered this an
                // error.
                copyfile_state_free(self.0);
            }
        }
    }

    // MacOS prior to 10.12 don't support `fclonefileat`
    // We store the availability in a global to avoid unnecessary syscalls
    static HAS_FCLONEFILEAT: AtomicBool = AtomicBool::new(true);

    let (reader, reader_metadata) = open_from(from)?;

    // Opportunistically attempt to create a copy-on-write clone of `from`
    // using `fclonefileat`.
    if HAS_FCLONEFILEAT.load(Ordering::Relaxed) {
        let clonefile_result = run_path_with_cstr(to, |to| {
            cvt(unsafe { libc::fclonefileat(reader.as_raw_fd(), libc::AT_FDCWD, to.as_ptr(), 0) })
        });
        match clonefile_result {
            Ok(_) => return Ok(reader_metadata.len()),
            Err(err) => match err.raw_os_error() {
                // `fclonefileat` will fail on non-APFS volumes, if the
                // destination already exists, or if the source and destination
                // are on different devices. In all these cases `fcopyfile`
                // should succeed.
                Some(libc::ENOTSUP) | Some(libc::EEXIST) | Some(libc::EXDEV) => (),
                Some(libc::ENOSYS) => HAS_FCLONEFILEAT.store(false, Ordering::Relaxed),
                _ => return Err(err),
            },
        }
    }

    // Fall back to using `fcopyfile` if `fclonefileat` does not succeed.
    let writer = open_to_and_set_permissions(to, reader_metadata)?;
    let writer_metadata = writer.metadata()?;

    // We ensure that `FreeOnDrop` never contains a null pointer so it is
    // always safe to call `copyfile_state_free`
    let state = unsafe {
        let state = copyfile_state_alloc();
        if state.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        FreeOnDrop(state)
    };

    let flags = if writer_metadata.is_file() {
        COPYFILE_ALL
    } else {
        COPYFILE_DATA
    };

    cvt(unsafe { fcopyfile(reader.as_raw_fd(), writer.as_raw_fd(), state.0, flags) })?;

    let mut bytes_copied: libc::off_t = 0;
    cvt(unsafe {
        copyfile_state_get(
            state.0,
            COPYFILE_STATE_COPIED,
            &mut bytes_copied as *mut libc::off_t as *mut libc::c_void,
        )
    })?;
    Ok(bytes_copied as u64)
}

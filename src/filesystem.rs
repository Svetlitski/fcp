//! A collection of utilities augmenting the standard library's filesystem capabilities, extending
//! them to cover the full gamut of POSIX file types, and wrapping them in order to improve the
//! usefulness of error messages by providing additional context.

use crate::error::{Error, Result};
use nix::sys::stat::Mode;
use nix::unistd;
use std::convert::TryInto;
use std::fs::{self, DirBuilder, DirEntry, File, Metadata, OpenOptions, Permissions, ReadDir};
use std::os::unix::fs::{self as unix, DirBuilderExt, FileTypeExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

macro_rules! wrap {
    ($namespace:ident, $function:ident, $payload:ty) => {
        pub fn $function<P: AsRef<Path>>(path: P) -> Result<$payload> {
            $namespace::$function(path.as_ref())
                .map_err(|err| Error::new(format!("{}: {}", path.as_ref().display(), err)))
        }
    };
}

macro_rules! wrap2 {
    ($function:ident, $namespace:ident, $payload:ty) => {
        pub fn $function<P: AsRef<Path>, Q: AsRef<Path>>(source: P, dest: Q) -> Result<$payload> {
            let (source, dest) = (source.as_ref(), dest.as_ref());
            $namespace::$function(source, dest).map_err(|err| {
                Error::new(format!("{}, {}: {}", source.display(), dest.display(), err))
            })
        }
    };
}

wrap!(fs, symlink_metadata, Metadata);
wrap!(fs, metadata, Metadata);
wrap!(fs, read_link, PathBuf);
wrap!(fs, read_dir, ReadDir);
wrap!(fs, remove_dir_all, ());
wrap!(fs, remove_file, ());
wrap!(fs, canonicalize, PathBuf);
wrap!(fs, create_dir_all, ());
wrap!(File, open, File);
wrap2!(symlink, unix, ());

macro_rules! make_error_message {
    ($path:ident) => {
        |err| Error::new(format!("{}: {}", $path.display(), err))
    };
}

pub fn entry_file_type(entry: &DirEntry) -> Result<FileType> {
    match entry.file_type() {
        Err(err) => Err(Error::new(format!("{}: {}", entry.path().display(), err))),
        Ok(file_type) => Ok(FileType::from(file_type)),
    }
}

pub fn create_dir<P: AsRef<Path>>(path: P, mode: u32) -> Result<()> {
    let path = path.as_ref();
    DirBuilder::new()
        .mode(mode)
        .create(path)
        .map_err(make_error_message!(path))
}

pub fn create<P: AsRef<Path>>(path: P, mode: u32) -> Result<File> {
    let path = path.as_ref();
    OpenOptions::new()
        .mode(mode)
        .truncate(true)
        .write(true)
        .create(true)
        .open(path)
        .map_err(make_error_message!(path))
}

pub fn mkfifo<P: AsRef<Path>>(path: P, permissions: Permissions) -> Result<()> {
    let path = path.as_ref();
    let mode = Mode::from_bits_truncate(permissions.mode().try_into()?);
    unistd::mkfifo(path, mode).map_err(make_error_message!(path))
}

#[derive(Debug)]
pub enum FileType {
    Regular,
    Directory,
    Symlink,
    Fifo,
    Socket,
    CharacterDevice,
    BlockDevice,
}

impl From<std::fs::FileType> for FileType {
    fn from(file_type: std::fs::FileType) -> Self {
        if file_type.is_file() {
            FileType::Regular
        } else if file_type.is_dir() {
            FileType::Directory
        } else if file_type.is_symlink() {
            FileType::Symlink
        } else if file_type.is_fifo() {
            FileType::Fifo
        } else if file_type.is_socket() {
            FileType::Socket
        } else if file_type.is_char_device() {
            FileType::CharacterDevice
        } else if file_type.is_block_device() {
            FileType::BlockDevice
        } else {
            unreachable!("file appears to exist but is an unknown type",);
        }
    }
}

pub fn file_type(path: &Path) -> Result<FileType> {
    Ok(FileType::from(symlink_metadata(path)?.file_type()))
}

pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(source: P, dest: Q) -> Result<u64> {
    let (source, dest) = (source.as_ref(), dest.as_ref());
    crate::copy::copy(source, dest)
        .map_err(|err| Error::new(format!("{}, {}: {}", source.display(), dest.display(), err)))
}

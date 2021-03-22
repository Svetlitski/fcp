use nix::sys::stat::Mode;
use nix::unistd;
use std::convert::TryInto;
use std::error::Error as BaseError;
use std::fmt;
use std::fs::{self, DirBuilder, File, Metadata, OpenOptions, Permissions, ReadDir};
use std::os::unix::fs::{self as unix, DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl<T: BaseError> From<T> for Error {
    fn from(other: T) -> Self {
        Error {
            message: other.to_string(),
        }
    }
}

impl Error {
    pub fn new(message: String) -> Self {
        Error { message }
    }
}

macro_rules! wrap {
    ($namespace:ident, $function:ident, $payload:ty) => {
        pub fn $function<P: AsRef<Path>>(path: P) -> Result<$payload, Error> {
            $namespace::$function(path.as_ref())
                .map_err(|err| Error::new(format!("{}: {}", path.as_ref().display(), err)))
        }
    };
}

macro_rules! wrap2 {
    ($function:ident, $namespace:ident, $payload:ty) => {
        pub fn $function<P: AsRef<Path>, Q: AsRef<Path>>(
            src: P,
            dst: Q,
        ) -> Result<$payload, Error> {
            $namespace::$function(src.as_ref(), dst.as_ref()).map_err(|err| {
                Error::new(format!(
                    "{}, {}: {}",
                    src.as_ref().display(),
                    dst.as_ref().display(),
                    err
                ))
            })
        }
    };
}

wrap!(fs, metadata, Metadata);
wrap!(fs, symlink_metadata, Metadata);
wrap!(fs, read_link, PathBuf);
wrap!(fs, read_dir, ReadDir);
wrap!(File, open, File);
wrap2!(symlink, unix, ());
wrap2!(copy, fs, u64);

macro_rules! make_error_message {
    ($path:ident) => {
        |err| Error::new(format!("{}: {}", $path.display(), err));
    };
}

pub fn create_dir<P: AsRef<Path>>(path: P, mode: u32) -> Result<(), Error> {
    let path = path.as_ref();
    DirBuilder::new()
        .mode(mode)
        .create(path)
        .map_err(make_error_message!(path))
}

pub fn create<P: AsRef<Path>>(path: P, mode: u32) -> Result<File, Error> {
    let path = path.as_ref();
    OpenOptions::new()
        .mode(mode)
        .truncate(true)
        .write(true)
        .create(true)
        .open(path)
        .map_err(make_error_message!(path))
}

pub fn mkfifo<P: AsRef<Path>>(path: P, permissions: Permissions) -> Result<(), Error> {
    let path = path.as_ref();
    let mode = Mode::from_bits_truncate(permissions.mode().try_into()?);
    unistd::mkfifo(path, mode).map_err(make_error_message!(path))
}

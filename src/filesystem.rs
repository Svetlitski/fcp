use std::error::Error as BaseError;
use std::fmt;
use std::fs::{self, DirBuilder, Metadata, ReadDir};
use std::os::unix::fs::{self as unix, DirBuilderExt};
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

impl BaseError for Error {}

macro_rules! wrap {
    ($function:ident, $payload:ty) => {
        pub fn $function<P: AsRef<Path>>(path: P) -> Result<$payload, Error> {
            fs::$function(path.as_ref()).map_err(|err| Error {
                message: format!("{}: {}", path.as_ref().display(), err),
            })
        }
    };
}

macro_rules! wrap2 {
    ($function:ident, $namespace:ident, $payload:ty) => {
        pub fn $function<P: AsRef<Path>, Q: AsRef<Path>>(
            src: P,
            dst: Q,
        ) -> Result<$payload, Error> {
            $namespace::$function(src.as_ref(), dst.as_ref()).map_err(|err| Error {
                message: format!(
                    "{}, {}: {}",
                    src.as_ref().display(),
                    dst.as_ref().display(),
                    err
                ),
            })
        }
    };
}

wrap!(metadata, Metadata);
wrap!(symlink_metadata, Metadata);
wrap!(read_link, PathBuf);
wrap!(read_dir, ReadDir);
wrap2!(symlink, unix, ());
wrap2!(copy, fs, u64);

pub fn create_dir<P: AsRef<Path>>(path: P, mode: u32) -> Result<(), Error> {
    DirBuilder::new()
        .mode(mode)
        .create(path.as_ref())
        .map_err(|err| Error {
            message: format!("{}: {}", path.as_ref().display(), err),
        })
}


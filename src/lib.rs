use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::array;
use std::fmt::Display;
use std::fs::Metadata;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process;

pub mod filesystem;

use crate::filesystem::{self as fs, Error, FileType};

pub fn fatal(message: impl Display) -> ! {
    eprintln!("{}", message);
    process::exit(1);
}

// The boolean returned signifies whether an error occurred (`true`) or not (`false`). The purpose
// of returning just a boolean instead of the underlying error itself is that we want to display
// the error to the user as soon as it occurs (as this makes for a better user-experience during
// long-running jobs) as opposed to propagating it upwards and printing all errors at the end.
// However, at the end of the process we still need to know whether or not an error occurred at any
// point in order to set the exit code appropriately.
fn copy_file(source: &Path, dest: &Path) -> bool {
    fn __copy_file(source: &Path, dest: &Path) -> Result<bool, Error> {
        match fs::file_type(source)? {
            FileType::Regular => {
                fs::copy(source, dest)?;
            }
            FileType::Directory(metadata) => return copy_directory((source, metadata), dest),
            FileType::Symlink => {
                fs::symlink(fs::read_link(source)?, dest)?;
            }
            FileType::Fifo(metadata) => {
                fs::mkfifo(dest, metadata.permissions())?;
            }
            FileType::Socket => {
                return Err(Error::new(format!(
                    "{}: sockets cannot be copied",
                    source.display(),
                )));
            }
            FileType::BlockDevice(metadata) | FileType::CharacterDevice(metadata) => {
                let mut source = fs::open(source)?;
                let mut dest = fs::create(dest, metadata.permissions().mode())?;
                io::copy(&mut source, &mut dest)?;
            }
        }
        Ok(false)
    }

    __copy_file(source, dest).unwrap_or_else(|err| {
        eprintln!("{}", err);
        true
    })
}

fn copy_directory(source: (&Path, Metadata), dest: &Path) -> Result<bool, Error> {
    let (source, metadata) = source;
    fs::create_dir(dest, metadata.permissions().mode())?;
    Ok(fs::read_dir(source)?
        .collect::<Box<_>>()
        .into_par_iter()
        .map(|entry| match entry {
            Ok(entry) => copy_file(&entry.path(), &dest.join(entry.file_name())),
            Err(err) => {
                eprintln!("{}", err);
                true
            }
        })
        .reduce(|| false, |a, b| a | b))
}

/// Copy each file in `sources` into the directory `dest`.
fn copy_many(sources: &[PathBuf], dest: &Path) -> bool {
    let metadata = fs::symlink_metadata(&dest).map_err(fatal).unwrap();
    if !metadata.is_dir() {
        fatal(format!("{} is not a directory", dest.display()));
    }
    sources
        .into_par_iter()
        .map(|source| {
            let file_name = match source.file_name() {
                Some(file_name) => file_name,
                None => {
                    eprintln!("{}: invalid file path", source.display());
                    return true;
                }
            };
            copy_file(&source, &dest.join(file_name))
        })
        .reduce(|| false, |a, b| a | b)
}

pub fn fcp(args: &[String]) -> bool {
    let args: Box<_> = args.iter().map(PathBuf::from).collect();
    match args.as_ref() {
        [] | [_] => fatal("Please provide at least two arguments (run 'fcp --help' for details)"),
        [source, dest] => match fs::symlink_metadata(dest) {
            Ok(metadata) if metadata.is_dir() => copy_many(array::from_ref(source), dest),
            _ => copy_file(source, dest),
        },
        [sources @ .., dest] => copy_many(sources, dest),
    }
}

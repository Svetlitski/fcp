use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::fmt::Display;
use std::os::unix;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::PathBuf;
use std::{fs, process};

pub fn fatal(message: impl Display) -> ! {
    eprintln!("{}", message);
    process::exit(1);
}

fn show_error(first: impl Display, second: impl Display) {
    eprintln!("{}: {}", first, second);
}

macro_rules! passthrough {
    ($_:expr) => {
        ", {}"
    };
}

/// Eliminates the boilerplate of calling a function which may fail,
/// and if so logging a detailed error message before returning from the
/// current function.
macro_rules! try_or_log {
    ($func:expr, $arg1:expr $(, $arg2:expr )?) => {
        match $func($arg1 $(, $arg2)?) {
            Ok(result) => result,
            Err(err) => {
                show_error(format!(concat!("{}" $(, passthrough!($arg2))?), $arg1.display() $(, $arg2.display())?), err);
                return;
            }
        }
    };
}

/// Copy each file in `sources` into the directory `dest`.
pub fn copy_many(sources: &[PathBuf], dest: &PathBuf) {
    let metadata = match fs::metadata(&dest) {
        Ok(metadata) => metadata,
        Err(err) => fatal(format!("{}: {}", dest.display(), err)),
    };
    if !metadata.is_dir() {
        fatal(format!("{} is not a directory", dest.display()));
    }
    sources.into_par_iter().for_each(|source| {
        let file_name = match source.file_name() {
            Some(file_name) => file_name,
            None => {
                show_error(source.display(), "file path cannot end with ..");
                return;
            }
        };
        let dest = dest.join(file_name);
        copy_file(&source, &dest);
    });
}

#[allow(clippy::needless_return)]
pub fn copy_file(source: &PathBuf, dest: &PathBuf) {
    let metadata = try_or_log!(fs::symlink_metadata, source);
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        let link = &try_or_log!(fs::read_link, source);
        try_or_log!(unix::fs::symlink, link, dest);
    } else if file_type.is_dir() {
        if let Err(err) = fs::DirBuilder::new()
            .mode(metadata.permissions().mode())
            .create(dest)
        {
            show_error(dest.display(), err);
            return;
        }
        try_or_log!(fs::read_dir, source)
            .collect::<Box<_>>()
            .into_par_iter()
            .for_each(|entry| match entry {
                Ok(entry) => copy_file(&entry.path(), &dest.join(entry.file_name())),
                Err(err) => eprintln!("{}", err),
            });
    } else {
        try_or_log!(fs::copy, source, dest);
    }
}

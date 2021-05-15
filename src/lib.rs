use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::fmt::Display;
use std::fs::Metadata;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process;

mod filesystem;
#[cfg(test)]
mod tests;

use crate::filesystem::{self as fs, FileType};

pub fn fatal(message: impl Display) -> ! {
    eprintln!("{}", message);
    process::exit(1);
}

pub fn copy_file(source: &Path, dest: &Path) {
    if let Err(err) = copy_file_impl(source, dest) {
        eprintln!("{}", err);
    }
}

fn copy_file_impl(source: &Path, dest: &Path) -> Result<(), fs::Error> {
    match fs::file_type(source)? {
        FileType::Regular => {
            fs::copy(source, dest)?;
        }
        FileType::Directory(metadata) => {
            copy_directory((source, metadata), dest)?;
        }
        FileType::Symlink => {
            fs::symlink(fs::read_link(source)?, dest)?;
        }
        FileType::Fifo(metadata) => {
            fs::mkfifo(dest, metadata.permissions())?;
        }
        FileType::Socket => {
            return Err(fs::Error::new(format!(
                "{}: {}",
                source.display(),
                "sockets cannot be copied"
            )));
        }
        FileType::BlockDevice(metadata) | FileType::CharacterDevice(metadata) => {
            let mut source = fs::open(source)?;
            let mut dest = fs::create(dest, metadata.permissions().mode())?;
            io::copy(&mut source, &mut dest)?;
        }
    }
    Ok(())
}

fn copy_directory(source: (&Path, Metadata), dest: &Path) -> Result<(), fs::Error> {
    let (source, metadata) = source;
    fs::create_dir(dest, metadata.permissions().mode())?;
    fs::read_dir(source)?
        .collect::<Box<_>>()
        .into_par_iter()
        .for_each(|entry| match entry {
            Ok(entry) => copy_file(&entry.path(), &dest.join(entry.file_name())),
            Err(err) => eprintln!("{}", err),
        });
    Ok(())
}

/// Copy each file in `sources` into the directory `dest`.
pub fn copy_many(sources: &[PathBuf], dest: &Path) {
    let metadata = fs::symlink_metadata(&dest).map_err(fatal).unwrap();
    if !metadata.is_dir() {
        fatal(format!("{} is not a directory", dest.display()));
    }
    sources.into_par_iter().for_each(|source| {
        let file_name = match source.file_name() {
            Some(file_name) => file_name,
            None => return eprintln!("{}: invalid file path", source.display()),
        };
        let dest = dest.join(file_name);
        copy_file(&source, &dest);
    });
}

pub fn fcp(args: &[String]) {
    let args: Box<_> = args.iter().map(PathBuf::from).collect();
    match args.len() {
        0 | 1 => fatal("Please provide at least two arguments"),
        2 => copy_file(args.first().unwrap(), args.last().unwrap()),
        _ => {
            let (dest, sources) = args.split_last().unwrap();
            copy_many(sources, dest);
        }
    }
}

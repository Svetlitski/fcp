use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::fmt::Display;
use std::io;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process;

mod filesystem;

use crate::filesystem as fs;

pub fn fatal(message: impl Display) -> ! {
    eprintln!("{}", message);
    process::exit(1);
}

/// Copy each file in `sources` into the directory `dest`.
pub fn copy_many(sources: &[PathBuf], dest: &Path) {
    let metadata = match fs::metadata(&dest) {
        Ok(metadata) => metadata,
        Err(err) => fatal(err),
    };
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

fn copy_file_impl(source: &Path, dest: &Path) -> Result<(), fs::Error> {
    let metadata = fs::symlink_metadata(source)?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        fs::symlink(fs::read_link(source)?, dest)?;
    } else if file_type.is_dir() {
        fs::create_dir(dest, metadata.permissions().mode())?;
        fs::read_dir(source)?
            .collect::<Box<_>>()
            .into_par_iter()
            .for_each(|entry| match entry {
                Ok(entry) => copy_file(&entry.path(), &dest.join(entry.file_name())),
                Err(err) => eprintln!("{}", err),
            });
    } else if file_type.is_fifo() {
        fs::mkfifo(dest, metadata.permissions())?;
    } else if file_type.is_socket() {
        return Err(fs::Error::new(format!(
            "{}: {}",
            source.display(),
            "cannot copy a socket"
        )));
    } else if file_type.is_char_device() || file_type.is_block_device() {
        let mut source = fs::open(source)?;
        let mut dest = fs::create(dest, metadata.permissions().mode())?;
        io::copy(&mut source, &mut dest)?;
    } else {
        fs::copy(source, dest)?;
    }

    Ok(())
}

pub fn copy_file(source: &Path, dest: &Path) {
    if let Err(err) = copy_file_impl(source, dest) {
        eprintln!("{}", err);
    }
}

pub fn fcp(args: Box<[String]>) {
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

use rayon::prelude::*;
use std::env;
use std::fmt::Display;
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let (source, dest) = (PathBuf::from(&args[0]), PathBuf::from(&args[1]));
    copy_file(source, dest);
}

fn show_error(first: impl Display, second: impl Display) {
    eprintln!("{}: {}", first, second);
}

fn copy_file(source: PathBuf, dest: PathBuf) -> Result<(), ()> {
    if fs::metadata(&source)
        .map_err(|err| show_error(source.display(), err))?
        .is_dir()
    {
        fs::create_dir(&dest).map_err(|err| show_error(dest.display(), err))?;
        fs::read_dir(&source)
            .map_err(|err| show_error(dest.display(), err))?
            .collect::<Vec<_>>()
            .into_par_iter()
            .for_each(|entry| match entry {
                Ok(entry) => {
                    copy_file(entry.path(), dest.join(entry.file_name()));
                }
                Err(err) => eprintln!("{}", err),
            });
    } else {
        fs::copy(&source, &dest)
            .map_err(|err| show_error(format!("{}, {}", source.display(), dest.display()), err))?;
    }
    Ok(())
}

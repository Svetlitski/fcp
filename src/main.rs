use rayon::prelude::*;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let (source, dest) = (PathBuf::from(&args[0]), PathBuf::from(&args[1]));
    copy_file(source, dest);
}

fn copy_file(source: PathBuf, dest: PathBuf) {
    if fs::metadata(&source).unwrap().is_dir() {
        fs::create_dir(&dest).unwrap();
        fs::read_dir(&source)
            .unwrap()
            .collect::<Vec<_>>()
            .into_par_iter()
            .for_each(|entry| {
                let entry = entry.unwrap();
                copy_file(entry.path(), dest.join(entry.file_name()));
            });
    } else {
        fs::copy(source, dest).unwrap();
    }
}

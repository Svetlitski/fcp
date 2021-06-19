use fcp::{self, filesystem as fs};
use lazy_static::lazy_static;
use rand::prelude::*;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;
use std::cell::RefCell;
use std::env;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::Once;
use std::thread_local;

lazy_static! {
    pub static ref FIXTURES_DIR: PathBuf = PathBuf::from("fixtures");
    pub static ref HYDRATED_DIR: PathBuf = FIXTURES_DIR.join("hydrated");
    pub static ref COPIES_DIR: PathBuf = FIXTURES_DIR.join("copies");
}

static INIT: Once = Once::new();

/// Must be called at the beginning of each test case and benchmark
pub fn initialize() {
    INIT.call_once(|| {
        if !HYDRATED_DIR.exists() {
            fs::create_dir(&*HYDRATED_DIR, 0o777).unwrap();
        }
        if !COPIES_DIR.exists() {
            fs::create_dir(&*COPIES_DIR, 0o777).unwrap();
        }
    });
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
enum FileKind {
    #[serde(rename = "file")]
    Regular {
        size: u64,
    },
    #[serde(rename = "link")]
    Symlink {
        target: PathBuf,
    },
    Directory {
        contents: Vec<FileStub>,
    },
    Fifo {
        size: u64,
    },
    Socket {},
}

#[derive(Debug, Deserialize)]
struct FileStub {
    name: String,
    mode: u32,
    #[serde(flatten)]
    kind: FileKind,
}

pub fn remove(path: &Path) {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
        .unwrap();
    }
}

pub fn hydrate_fixture(filename: &str) {
    let fixture_path = FIXTURES_DIR.join(filename);
    let output_path = HYDRATED_DIR.join(filename.strip_suffix(".json").unwrap());
    // We check if the file exists like this instead of via Path::exists
    // because we consider broken symlinks as still existing.
    if let Ok(output_meta) = fs::symlink_metadata(&output_path) {
        let fixture_modification_time = fs::symlink_metadata(&fixture_path)
            .unwrap()
            .modified()
            .unwrap();
        let output_creation_time = output_meta.created().unwrap();
        if fixture_modification_time < output_creation_time {
            return; // Fixture has already been hydrated, do nothing
        }
        remove(&output_path);
    }

    let mut files = serde_json::Deserializer::from_reader(fs::open(&fixture_path).unwrap());
    files.disable_recursion_limit();
    files
        .into_iter::<Vec<FileStub>>()
        .flat_map(Result::unwrap)
        .for_each(hydrate_file);
}

fn hydrate_file(file: FileStub) {
    let path = HYDRATED_DIR.join(file.name);
    let FileStub { mode, .. } = file;
    match file.kind {
        FileKind::Regular { size } => {
            let mut file = fs::create(path, mode).unwrap();
            let metadata = file.metadata().unwrap();
            if metadata.len() < size {
                file.seek(SeekFrom::End(0)).unwrap();
                let mut remaining: usize = (size - metadata.len()) as usize;
                let mut rng = Pcg64::from_rng(thread_rng()).unwrap();
                thread_local! {
                    static BUFFER: RefCell<[u8; 1<<16]> = RefCell::new([0; 1<<16]);
                }
                BUFFER.with(|buffer| {
                    let mut buffer = buffer.borrow_mut();
                    while remaining > 0 {
                        let bytes_to_process = std::cmp::min(remaining as usize, buffer.len());
                        let slice = &mut buffer[..bytes_to_process];
                        rng.fill(slice);
                        file.write_all(slice).unwrap();
                        remaining -= bytes_to_process;
                    }
                });
            }
        }
        FileKind::Symlink { target } => fs::symlink(HYDRATED_DIR.join(target), path).unwrap(),
        FileKind::Fifo { .. } => fs::mkfifo(path, PermissionsExt::from_mode(mode)).unwrap(),
        FileKind::Directory { contents } => {
            fs::create_dir(path, mode).unwrap();
            contents.into_par_iter().for_each(hydrate_file);
        }
        FileKind::Socket {} => {
            UnixListener::bind(path).unwrap();
        }
    }
}

pub fn fcp_executable_path() -> PathBuf {
    let mut executable = env::current_exe().unwrap();
    executable.pop();
    executable.pop();
    executable.push(format!("fcp{}", env::consts::EXE_SUFFIX));
    executable
}

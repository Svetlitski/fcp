use fcp::{self, filesystem as fs};
use lazy_static::lazy_static;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::os::unix::fs::PermissionsExt;
use std::str;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Output};

lazy_static! {
    static ref HYDRATED_DIR: PathBuf = PathBuf::from("fixtures/hydrated");
    static ref COPIES_DIR: PathBuf = PathBuf::from("fixtures/copies");
    static ref FIXTURES_DIR: PathBuf = PathBuf::from("fixtures");
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
enum FileStub {
    #[serde(rename = "file")]
    Regular {
        name: String,
        size: u64,
        mode: u32,
    },
    #[serde(rename = "link")]
    Symlink {
        name: String,
        target: PathBuf,
        mode: u32,
    },
    Directory {
        name: String,
        contents: Vec<FileStub>,
        mode: u32,
    },
    Fifo {
        name: String,
        size: u64,
        mode: u32,
    },
}

fn hydrate_fixture(filename: &str) {
    let files = serde_json::from_reader::<File, Vec<FileStub>>(
        fs::open(FIXTURES_DIR.join(filename)).unwrap(),
    )
    .unwrap();
    files.into_par_iter().for_each(hydrate_file);
}

fn hydrate_file(file: FileStub) {
    let path = match file {
        FileStub::Directory { ref name, .. }
        | FileStub::Regular { ref name, .. }
        | FileStub::Fifo { ref name, .. }
        | FileStub::Symlink { ref name, .. } => HYDRATED_DIR.join(name),
    };
    // Makes this function idempotent
    if path.exists() {
        return;
    }
    match file {
        FileStub::Regular { size, mode, .. } => {
            let mut file = fs::create(path, mode).unwrap();
            let metadata = file.metadata().unwrap();
            if metadata.len() < size {
                file.seek(SeekFrom::End(0)).unwrap();
                let mut random = fs::open("/dev/random").unwrap();
                let mut remaining: usize = (size - metadata.len()) as usize;
                let mut buffer = [0u8; 4096];
                while remaining > 0 {
                    let bytes_to_process = std::cmp::min(remaining as usize, buffer.len());
                    let slice = &mut buffer[..bytes_to_process];
                    random.read_exact(slice).unwrap();
                    file.write_all(slice).unwrap();
                    remaining -= bytes_to_process;
                }
            }
        }
        FileStub::Symlink { target, .. } => fs::symlink(HYDRATED_DIR.join(target), path).unwrap(),
        FileStub::Fifo { mode, .. } => fs::mkfifo(path, PermissionsExt::from_mode(mode)).unwrap(),
        FileStub::Directory { mode, contents, .. } => {
            fs::create_dir(path, mode).unwrap();
            contents.into_par_iter().for_each(hydrate_file);
        }
    }
}

fn diff(filename: &str) -> ExitStatus {
    let filename = filename.strip_suffix(".json").unwrap();
    Command::new("diff")
        .args(&[
            "-rq",
            HYDRATED_DIR.join(filename).to_str().unwrap(),
            COPIES_DIR.join(filename).to_str().unwrap(),
        ])
        .status()
        .unwrap()
}

fn copy_fixture(filename: &str) -> Output {
    let filename = filename.strip_suffix(".json").unwrap();
    let output = COPIES_DIR.join(filename);
    let _ = fs::remove_dir_all(&output);
    Command::new("./target/debug/fcp")
        .args(&[
            HYDRATED_DIR.join(filename).to_str().unwrap().to_string(),
            output.to_str().unwrap().to_string(),
        ])
        .output()
        .unwrap()
}

#[test]
fn regular_file() {
    hydrate_fixture("regular_file.json");
    let result = copy_fixture("regular_file.json");
    assert!(result.status.success());
    assert_eq!(str::from_utf8(&result.stderr).unwrap(), "");
    assert!(diff("regular_file.json").success());
}

#[test]
fn simple_directory() {
    hydrate_fixture("simple_directory.json");
    let result = copy_fixture("simple_directory.json");
    assert!(result.status.success());
    assert_eq!(str::from_utf8(&result.stderr).unwrap(), "");
    assert!(diff("simple_directory.json").success());
}

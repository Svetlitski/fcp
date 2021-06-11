use fcp::{self, filesystem as fs};
use lazy_static::lazy_static;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;
use std::env;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Output};
use std::str;

lazy_static! {
    static ref HYDRATED_DIR: PathBuf = PathBuf::from("fixtures/hydrated");
    static ref COPIES_DIR: PathBuf = PathBuf::from("fixtures/copies");
    static ref FIXTURES_DIR: PathBuf = PathBuf::from("fixtures");
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

fn hydrate_fixture(filename: &str) {
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
        fs::remove_dir_all(&output_path).unwrap();
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

fn diff(filename: &str) -> ExitStatus {
    let filename = filename.strip_suffix(".json").unwrap();
    Command::new("diff")
        .args(&[
            "-rq",
            "--no-dereference",
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
    let mut executable = env::current_exe().unwrap();
    executable.pop();
    executable.pop();
    executable.push(format!("fcp{}", env::consts::EXE_SUFFIX));
    Command::new(executable)
        .args(&[
            HYDRATED_DIR.join(filename).to_str().unwrap().to_string(),
            output.to_str().unwrap().to_string(),
        ])
        .output()
        .unwrap()
}

macro_rules! make_test {
    ($test_name:ident) => {
        #[test]
        fn $test_name() {
            let fixture_file = concat!(stringify!($test_name), ".json");
            hydrate_fixture(fixture_file);
            let result = copy_fixture(fixture_file);
            assert!(result.status.success());
            assert_eq!(str::from_utf8(&result.stderr).unwrap(), "");
            assert!(diff(fixture_file).success());
        }
    };
}

make_test!(regular_file);
make_test!(symlink);
make_test!(empty_directory);
make_test!(simple_directory);
make_test!(deep_directory);

#[test]
fn socket() {
    let fixture_file = "socket.json";
    hydrate_fixture(fixture_file);
    let result = copy_fixture(fixture_file);
    assert!(!result.status.success());
    assert!(str::from_utf8(&result.stderr)
        .unwrap()
        .contains("sockets cannot be copied"));
}

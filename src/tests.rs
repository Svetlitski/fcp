use crate::filesystem as fs;
use crate::{fatal, fcp};
use lazy_static::lazy_static;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

lazy_static! {
    static ref HYDRATED_DIR: PathBuf = PathBuf::from("fixtures/extracted");
    static ref COPIES_DIR: PathBuf = PathBuf::from("fixtures/copies");
    static ref FIXTURES_DIR: PathBuf = PathBuf::from("fixtures");
}

fn deserialize_mode<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    struct ModeVisitor;
    impl<'de> Visitor<'de> for ModeVisitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an octal integer in the format 0xxx")
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u32::from_str_radix(&value, 8).map_err(|err| E::custom(err.to_string()))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u32::from_str_radix(value, 8).map_err(|err| E::custom(err.to_string()))
        }
    }

    deserializer.deserialize_string(ModeVisitor)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
enum FileStub {
    File {
        name: String,
        size: u64,
        #[serde(deserialize_with = "deserialize_mode")]
        mode: u32,
    },
    #[serde(rename = "link")]
    Symlink {
        name: String,
        target: PathBuf,
        #[serde(deserialize_with = "deserialize_mode")]
        mode: u32,
    },
    Directory {
        name: String,
        contents: Vec<FileStub>,
        #[serde(deserialize_with = "deserialize_mode")]
        mode: u32,
    },
    Fifo {
        name: String,
        size: u64,
        #[serde(deserialize_with = "deserialize_mode")]
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
        | FileStub::File { ref name, .. }
        | FileStub::Fifo { ref name, .. }
        | FileStub::Symlink { ref name, .. } => HYDRATED_DIR.join(name),
    };
    match file {
        FileStub::File { size, mode, .. } => {
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
            "-r",
            HYDRATED_DIR.join(filename).to_str().unwrap(),
            COPIES_DIR.join(filename).to_str().unwrap(),
        ])
        .status()
        .unwrap()
}

#[test]
fn regular_file() {
    hydrate_fixture("test.json")
}

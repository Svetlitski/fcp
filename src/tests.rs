use crate::filesystem as fs;
use crate::{fatal, fcp};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use lazy_static::lazy_static;


lazy_static! {
    static ref OUTPUT_DIR: PathBuf = PathBuf::from("fixtures/extracted");
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
    let files =
        serde_json::from_reader::<File, Vec<FileStub>>(fs::open(filename).unwrap()).unwrap();
    println!("{:?}", files);
    files.into_par_iter().for_each(hydrate_file);
}

fn hydrate_file(file: FileStub) {

    match file {
        FileStub::File { name, size, mode } => {
            let mut file = fs::create(OUTPUT_DIR.join(name), mode).unwrap();
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
        FileStub::Symlink { name, target, .. } => fs::symlink(OUTPUT_DIR.join(target), OUTPUT_DIR.join(name)).unwrap(),
        FileStub::Fifo { name, mode, .. } => {
            fs::mkfifo(OUTPUT_DIR.join(name), PermissionsExt::from_mode(mode)).unwrap()
        }
        FileStub::Directory {
            name,
            mode,
            contents,
            ..
        } => {
            fs::create_dir(OUTPUT_DIR.join(name), mode).unwrap();
            contents.into_par_iter().for_each(hydrate_file);
        }
    }
}

#[test]
fn regular_file() {
    hydrate_fixture("fixtures/test.json")
}

use dev_utils::*;
use fcp::{self, filesystem as fs};
use std::ffi::OsStr;
use std::io::prelude::*;
use std::process::{Command, ExitStatus};
use std::string::String;

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

struct CommandResult {
    stderr: String,
    success: bool,
}

fn fcp_run<T: AsRef<OsStr>>(args: &[T]) -> CommandResult {
    let result = Command::new(fcp_executable_path())
        .args(args)
        .output()
        .unwrap();
    CommandResult {
        stderr: String::from_utf8(result.stderr).unwrap(),
        success: result.status.success(),
    }
}

fn copy_fixture(filename: &str) -> CommandResult {
    let filename = filename.strip_suffix(".json").unwrap();
    let destination = COPIES_DIR.join(filename);
    remove(&destination);
    fcp_run(&[HYDRATED_DIR.join(filename), destination])
}

macro_rules! make_test {
    ($(#[$attributes:meta])*
     $test_name:ident) => {
        #[test]
        $(#[$attributes])*
        fn $test_name() {
            initialize();
            let fixture_file = concat!(stringify!($test_name), ".json");
            hydrate_fixture(fixture_file);
            let result = copy_fixture(fixture_file);
            assert!(result.success);
            assert_eq!(result.stderr, "");
            assert!(diff(fixture_file).success());
        }
    };
}

make_test!(regular_file);
make_test!(symlink);
make_test!(empty_directory);
make_test!(simple_directory);
make_test!(deep_directory);
make_test!(
    #[ignore]
    linux
);
make_test!(
    #[ignore]
    large_files
);

#[test]
fn socket() {
    initialize();
    let fixture_file = "socket.json";
    hydrate_fixture(fixture_file);
    let result = copy_fixture(fixture_file);
    assert!(!result.success);
    assert!(result.stderr.contains("sockets cannot be copied"));
}

#[test]
fn fifo() {
    initialize();
    let fixture_file = "fifo.json";
    hydrate_fixture(fixture_file);
    let result = copy_fixture(fixture_file);
    assert!(result.success);
    let file_type =
        fs::file_type(&COPIES_DIR.join(fixture_file.strip_suffix(".json").unwrap())).unwrap();
    assert!(matches!(file_type, fs::FileType::Fifo(..)))
}

#[test]
fn character_device() {
    initialize();
    let destination = COPIES_DIR.join("character_device");
    remove(&destination);
    let contents = "Hello world\r";
    let result = Command::new("tests/character_device.exp")
        .args(&[
            fcp_executable_path().to_str().unwrap(),
            destination.to_str().unwrap(),
            contents,
        ])
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(String::from_utf8(result.stderr).unwrap(), "");
    assert!(destination.exists());
    let mut output = fs::open(destination).unwrap();
    let mut output_contents = Vec::with_capacity(contents.len());
    output.read_to_end(&mut output_contents).unwrap();
    assert_eq!(
        String::from_utf8(output_contents).unwrap(),
        contents.replace('\r', "\n")
    );
}

#[test]
fn too_few_arguments() {
    initialize();
    assert!(!fcp_run::<&str>(&[]).success);
    assert!(!fcp_run(&["source"]).success);
}

#[test]
fn source_does_not_exist() {
    initialize();
    let destination = COPIES_DIR.join("destination");
    let source = "nonexistent_source";
    remove(&destination);
    let result = fcp_run(&[source, destination.to_str().unwrap()]);
    assert!(!result.success);
    assert!(result.stderr.contains(source));
    assert!(!destination.exists());
}

#[test]
// A directory containing one.txt, two.txt, and three.txt
// where two.txt is inaccessible due to its permissions. We want
// to ensure that the error in copying two.txt is reported, but that
// the other files are still copied successfully.
fn partial_directory() {
    initialize();
    let fixture_file = "partial_directory.json";
    hydrate_fixture(fixture_file);
    let result = copy_fixture(fixture_file);
    assert!(!result.success);
    assert!(result.stderr.contains("partial_directory/two.txt"));
    for file in &["one.txt", "three.txt"] {
        let result = Command::new("diff")
            .args(&[
                "-q",
                HYDRATED_DIR
                    .join("partial_directory")
                    .join(file)
                    .to_str()
                    .unwrap(),
                COPIES_DIR
                    .join("partial_directory")
                    .join(file)
                    .to_str()
                    .unwrap(),
            ])
            .output()
            .unwrap();
        assert!(result.status.success());
    }
}

#[test]
fn copy_into() {
    initialize();
    let source = COPIES_DIR.join("empty");
    let destination = COPIES_DIR.join("temp");
    remove(&source);
    remove(&destination);
    fs::create(&source, 0o777).unwrap();
    fs::create_dir(&destination, 0o777).unwrap();
    let result = fcp_run(&[&source, &destination]);
    assert!(result.success);
    assert_eq!(result.stderr, "");
    assert!(destination.join("empty").exists());
}

#[test]
fn copy_many_into() {
    initialize();
    let fixture_file = "copy_many_into.json";
    let fixture_name = fixture_file.strip_suffix(".json").unwrap();
    hydrate_fixture(fixture_file);
    let source = HYDRATED_DIR.join(fixture_name);
    let destination = COPIES_DIR.join(fixture_name);
    remove(&destination);
    fs::create_dir(&destination, 0o777).unwrap();
    let mut file_paths = fs::read_dir(&source)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    file_paths.push(destination);
    let result = fcp_run(&file_paths);
    assert!(result.success);
    assert_eq!(result.stderr, "");
    assert!(diff(fixture_file).success());
}

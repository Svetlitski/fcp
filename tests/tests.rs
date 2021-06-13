use dev_utils::*;
use fcp::{self, filesystem as fs};
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

fn fcp_run(args: &[&str]) -> CommandResult {
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
    let output = COPIES_DIR.join(filename);
    remove(&output);
    fcp_run(&[
        HYDRATED_DIR.join(filename).to_str().unwrap(),
        output.to_str().unwrap(),
    ])
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
    let output_path = COPIES_DIR.join("character_device");
    remove(&output_path);
    let contents = "Hello world\r";
    let result = Command::new("tests/character_device.exp")
        .args(&[
            fcp_executable_path().to_str().unwrap(),
            output_path.to_str().unwrap(),
            contents,
        ])
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(String::from_utf8(result.stderr).unwrap(), "");
    assert!(output_path.exists());
    let mut output_file = fs::open(output_path).unwrap();
    let mut output_contents = Vec::with_capacity(contents.len());
    output_file.read_to_end(&mut output_contents).unwrap();
    assert_eq!(
        String::from_utf8(output_contents).unwrap(),
        contents.replace('\r', "\n")
    );
}

#[test]
fn too_few_arguments() {
    initialize();
    assert!(!fcp_run(&[]).success);
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
    let empty_path = COPIES_DIR.join("empty");
    let temp_dir_path = COPIES_DIR.join("temp");
    remove(&empty_path);
    remove(&temp_dir_path);
    fs::create(&empty_path, 0o777).unwrap();
    fs::create_dir(&temp_dir_path, 0o777).unwrap();
    let result = fcp_run(&[
        empty_path.to_str().unwrap(),
        temp_dir_path.to_str().unwrap(),
    ]);
    assert!(result.success);
    assert_eq!(result.stderr, "");
    assert!(temp_dir_path.join("empty").exists());
}

#[test]
fn copy_many_into() {
    initialize();
    let file_names = ["empty1", "empty2", "empty3"];
    let mut file_paths = file_names
        .iter()
        .map(|filename| COPIES_DIR.join(filename))
        .collect::<Vec<_>>();
    let temp_dir_path = COPIES_DIR.join("temp_many");
    for path in &file_paths {
        remove(&path);
        fs::create(&path, 0o777).unwrap();
    }
    remove(&temp_dir_path);
    fs::create_dir(&temp_dir_path, 0o777).unwrap();
    file_paths.push(temp_dir_path);
    let temp_dir_path = file_paths.last().unwrap();
    let result = fcp_run(
        &file_paths
            .iter()
            .map(|path| path.to_str().unwrap())
            .collect::<Box<_>>(),
    );
    assert!(result.success);
    assert_eq!(result.stderr, "");
    for name in &file_names {
        assert!(temp_dir_path.join(name).exists());
    }
}

use fcp::{copy_file, copy_many, fatal, normalize_path};
use std::env;
use std::path::PathBuf;
use std::string::ToString;

static HELP: &str = "\
fcp

USAGE:
\tfcp SOURCE DESTINATION_FILE
\tCopy SOURCE to DESTINATION_FILE, overwriting DESTINATION_FILE if it exists

\tfcp SOURCE ... DESTINATION_DIRECTORY
\tCopy each SOURCE into DESTINATION_DIRECTORY";

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        fatal(HELP);
    }
    let args = normalize_args(args);
    match args.len() {
        0 | 1 => fatal("Please provide at least two arguments"),
        2 => copy_file(args.first().unwrap(), args.last().unwrap()),
        _ => {
            let (dest, sources) = args.split_last().unwrap();
            copy_many(sources, dest);
        }
    }
}

fn normalize_args(args: Vec<String>) -> Vec<PathBuf> {
    let (args, errors): (Vec<_>, Vec<_>) = args
        .into_iter()
        .map(normalize_path)
        .partition(Result::is_ok);
    if !errors.is_empty() {
        fatal(
            errors
                .into_iter()
                .map(|error| error.unwrap_err().to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    args.into_iter().map(Result::unwrap).collect()
}

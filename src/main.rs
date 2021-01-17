use fcp::{copy_file, copy_many, fatal};
use std::env;
use std::path::PathBuf;

static HELP: &str = "\
fcp

USAGE:
\tfcp SOURCE DESTINATION_FILE
\tCopy SOURCE to DESTINATION_FILE, overwriting DESTINATION_FILE if it exists

\tfcp SOURCE ... DESTINATION_DIRECTORY
\tCopy each SOURCE into DESTINATION_DIRECTORY";

fn main() {
    let args: Box<_> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        fatal(HELP);
    }
    let args: Box<_> = args.iter().map(PathBuf::from).collect();
    match args.len() {
        0 | 1 => fatal("Please provide at least two arguments"),
        2 => copy_file(args.first().unwrap(), args.last().unwrap()),
        _ => {
            let (dest, sources) = args.split_last().unwrap();
            copy_many(sources, dest);
        }
    }
}

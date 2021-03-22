use fcp::{fatal, fcp};
use std::env;

static HELP: &str = "\
fcp

USAGE:
    fcp SOURCE DESTINATION_FILE
    Copy SOURCE to DESTINATION_FILE, overwriting DESTINATION_FILE if it exists

    fcp SOURCE ... DESTINATION_DIRECTORY
    Copy each SOURCE into DESTINATION_DIRECTORY";

fn main() {
    let args: Box<_> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        fatal(HELP);
    }
    fcp(args);
}

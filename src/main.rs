use fcp::{fatal, fcp};
use std::env;
use std::process;

static HELP: &str = concat!(
    "fcp ",
    env!("CARGO_PKG_VERSION"),
    "\n\n\
USAGE:
    fcp [OPTIONS] SOURCE DESTINATION_FILE
    Copy SOURCE to DESTINATION_FILE, overwriting DESTINATION_FILE if it exists

    fcp [OPTIONS] SOURCE ... DESTINATION_DIRECTORY
    Copy each SOURCE into DESTINATION_DIRECTORY

OPTIONS:
    -h, --help
            Output this usage information and exit.

    -V, --version
            Output version information and exit."
);

static VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Box<_> = env::args().skip(1).collect();
    for arg in args.iter() {
        match arg.as_str() {
            "-h" | "--help" => fatal(HELP),
            "-V" | "--version" => fatal(VERSION),
            _ => {}
        }
    }
    process::exit(fcp(&args) as i32);
}

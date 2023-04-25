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

    -n, --no-clobber
            Do not overwrite an existing file.

    -V, --version
            Output version information and exit."
);

static VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut no_clobber = false;
    args.retain(|arg| match arg.as_str() {
        "-h" | "--help" => fatal(HELP),
        "-n" | "--no-clobber" => {
            no_clobber = true;
            false
        },
        "-V" | "--version" => fatal(VERSION),
        _ => true,
    });
    process::exit(fcp(&args, no_clobber) as i32);
}

mod cli;
mod ghost;
mod operations;
mod uopt;
mod winstall;

use crate::cli::{get_options, Config};
use std::env::args;
use std::io::{stderr, stdout};

fn main() {
    let config = Config::from_env();

    let parsed = get_options(args().skip(1), config);
    let (arguments, options) = unwrap_or_exit(parsed);

    let outcome = operations::run(arguments, &options, (&mut stdout(), &mut stderr()));
    unwrap_or_exit(outcome);
}

fn unwrap_or_exit<T>(result: Result<T, winstall::Error>) -> T {
    match result {
        Ok(x) => x,
        Err(e) => {
            eprintln!("winstall: {}\nTry 'winstall --help' for more information.", e);
            std::process::exit(1);
        }
    }
}

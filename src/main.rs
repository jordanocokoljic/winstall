mod cli;
mod ghost;
mod operations;
mod uopt;
mod winstall;

use crate::cli::{get_options, Config};
use std::env::args;
use std::io::{stderr, stdout};

fn main() -> winstall::Result<()> {
    let config = Config::from_env();
    let parsed = get_options(args().skip(1), config);

    let (arguments, options) = match parsed {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{}\nTry 'winstall --help' for more information.", e);
            std::process::exit(1);
        }
    };

    operations::run(arguments, &options, (&mut stdout(), &mut stderr()))
}

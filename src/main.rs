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
    let (arguments, options) = get_options(args().skip(1), config)?;

    operations::run(arguments, &options, (&mut stdout(), &mut stderr()))
}

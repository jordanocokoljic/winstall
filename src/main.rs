mod cli;
mod ghost;
mod operations;
mod winstall;

use crate::cli::{get_options, Config};
use std::env::args;
use std::io::{stderr, stdout};

fn main() -> winstall::Result<()> {
    let config = Config::from_env();
    let (rest, options) = get_options(args().skip(1), config)?;

    operations::run(rest, &options, (&mut stdout(), &mut stderr()))
}

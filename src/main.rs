use std::env;

mod winstall;

fn main() -> Result<(), winstall::Error> {
    winstall::install(env::args().skip(1))?;
    Ok(())
}

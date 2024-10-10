mod winstall;

use std::env;

fn main() {
    let config = winstall::Config::from_env();
    let context = winstall::get_options(env::args().skip(1), config);
    println!("{context:?}");
}

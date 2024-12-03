use std::io;

use crate::cli::Options;
use crate::winstall;
use crate::winstall::Error;
use winstall::Result;

const VERSION: &str = include_str!("version.txt");
const HELP: &str = include_str!("usage.txt");

pub fn run(
    arguments: Vec<String>,
    options: &Options,
    (out, err): (&mut impl io::Write, &mut impl io::Write),
) -> Result<()> {
    if arguments.is_empty() {
        return Err(Error::MissingOperand("file".to_string()));
    }

    match arguments[0].as_str() {
        "--version" => {
            version(out);
            Ok(())
        }
        "--help" => {
            help(out);
            Ok(())
        }
        _ => install(arguments, options, (out, err)),
    }
}

fn install(
    _arguments: Vec<String>,
    _options: &Options,
    (_out, _err): (&mut impl io::Write, &mut impl io::Write),
) -> Result<()> {
    Ok(())
}

fn version(w: &mut impl io::Write) {
    write!(w, "{}", VERSION).expect("unable to write version message");
}

fn help(w: &mut impl io::Write) {
    write!(w, "{}", HELP).expect("unable to write help message");
}

#[cfg(test)]
mod tests {
    use crate::cli::Options;
    use crate::operations::{run, HELP, VERSION};
    use crate::winstall::Error;

    #[test]
    fn test_run_without_args_returns_error() {
        let io = (&mut Vec::<u8>::new(), &mut Vec::<u8>::new());

        let result = run(vec![], &Options::default(), io);
        assert!(result.is_err());
        assert_eq!(
            Error::MissingOperand("file".to_string()),
            result.unwrap_err()
        );
    }

    #[test]
    fn test_help_message() {
        let (out, err) = (&mut Vec::<u8>::new(), &mut Vec::<u8>::new());
        run(into(vec!["--help"]), &Options::default(), (out, err)).unwrap();

        assert_eq!(String::from_utf8(out.to_vec()).unwrap(), HELP);
    }

    #[test]
    fn test_version_message() {
        let (out, err) = (&mut Vec::<u8>::new(), &mut Vec::<u8>::new());
        run(into(vec!["--version"]), &Options::default(), (out, err)).unwrap();

        assert_eq!(String::from_utf8(out.to_vec()).unwrap(), VERSION);
    }

    #[test]
    fn test_version_contains_pkg_version() {
        let expected_prefix = format!("winstall {}", env!("CARGO_PKG_VERSION"));
        assert!(VERSION.starts_with(&expected_prefix))
    }

    fn into(from: Vec<&str>) -> Vec<String> {
        from.iter().map(|s| s.to_string()).collect()
    }
}

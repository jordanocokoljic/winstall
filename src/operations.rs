use crate::cli::{Action, Options};
use crate::winstall;
use crate::winstall::Error::MissingOperand;
use std::io;
use winstall::Result;

const VERSION: &str = include_str!("version.txt");
const HELP: &str = include_str!("usage.txt");

pub fn run(
    arguments: Vec<String>,
    options: &Options,
    (out, _err): (&mut impl io::Write, &mut impl io::Write),
) -> Result<()> {
    match options.alternate {
        Action::Help => help(out),
        Action::Version => version(out),
        Action::Install => install(arguments, options, (out, _err)),
    }
}

fn install(
    arguments: Vec<String>,
    _options: &Options,
    (_out, _err): (&mut impl io::Write, &mut impl io::Write),
) -> Result<()> {
    if arguments.is_empty() {
        return Err(MissingOperand("file".to_string()));
    }

    Ok(())
}

fn version(w: &mut impl io::Write) -> Result<()> {
    write!(w, "{}", VERSION).expect("unable to write version message");
    Ok(())
}

fn help(w: &mut impl io::Write) -> Result<()> {
    write!(w, "{}", HELP).expect("unable to write help message");
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cli::Options;
    use crate::operations::{help, install, version, HELP, VERSION};
    use crate::winstall::Error::MissingOperand;

    #[test]
    fn test_version() {
        let mut out = Vec::new();
        version(&mut out).expect("unable to write version message");

        assert_eq!(
            VERSION,
            String::from_utf8(out).expect("unable to convert version to string")
        );
    }

    #[test]
    fn test_help() {
        let mut out = Vec::new();
        help(&mut out).expect("unable to write help message");

        assert_eq!(
            HELP,
            String::from_utf8(out).expect("unable to convert help to string")
        );
    }

    #[test]
    fn test_install_no_arguments_causes_missing_file_operand() {
        let result = install(
            Vec::new(),
            &Options::default(),
            (&mut Vec::new(), &mut Vec::new()),
        );
        assert_eq!(result.unwrap_err(), MissingOperand("file".to_string()));
    }
}

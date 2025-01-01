use crate::cli::{Action, Options};
use crate::winstall;
use crate::winstall::Error::{DirectoryPermissionDenied, MissingOperand, PhBadWorkingDirectory};
use std::path::{Path, PathBuf};
use std::{env, fs, io};
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
        Action::Install => install(None::<&Path>, arguments, options, (out, _err)),
    }
}

fn install(
    path_override: Option<impl AsRef<Path>>,
    arguments: Vec<String>,
    options: &Options,
    (_out, _err): (&mut impl io::Write, &mut impl io::Write),
) -> Result<()> {
    fn create_dir_all(base: &Path, path: &str) -> Result<()> {
        let mut aggregate = PathBuf::new();
        let p = Path::new(path);

        for component in p.components() {
            let target = aggregate.join(component);

            match fs::create_dir(base.join(&target).as_path()) {
                Ok(_) => (),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
                Err(_) => return Err(DirectoryPermissionDenied(path_to_string(&aggregate))),
            };

            aggregate = target;
        }

        Ok(())
    }

    fn path_to_string(path: &PathBuf) -> String {
        path.strip_prefix("./")
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    if arguments.is_empty() {
        return Err(MissingOperand("file".to_string()));
    }

    let base = match path_override
        .map(|x| x.as_ref().to_path_buf())
        .or(env::current_dir().ok())
    {
        Some(p) => p,
        None => return Err(PhBadWorkingDirectory),
    };

    if options.directory_args {
        for arg in arguments {
            create_dir_all(&base, &arg)?;
        }

        return Ok(());
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
    use crate::ghost::EphemeralPath;
    use crate::operations::{help, install, version, HELP, VERSION};
    use crate::winstall::Error::{DirectoryPermissionDenied, MissingOperand};
    use std::path::Path;

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
            None::<&Path>,
            Vec::new(),
            &Options::default(),
            (&mut Vec::new(), &mut Vec::new()),
        );

        assert_eq!(result.unwrap_err(), MissingOperand("file".to_string()));
    }

    #[test]
    fn test_install_with_directory_args() {
        let ghost = EphemeralPath::new("test_install_with_directory_args");

        let result = install(
            Some(ghost.path()),
            vec!["a", "b/c"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>(),
            &Options {
                directory_args: true,
                ..Options::default()
            },
            (&mut Vec::new(), &mut Vec::new()),
        );

        assert!(result.is_ok());
        assert!(ghost.join("a").exists());
        assert!(ghost.join("b").exists());
        assert!(ghost.join("b").join("c").exists());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_install_with_directory_args_in_readonly_directory() {
        let path = Path::new("readonly_dir");

        assert!(
            path.exists() && path.join("b").exists(),
            "test environment is not clean: please run ./scripts/create_readonly.ps1"
        );

        let result = install(
            Some(path),
            vec!["b/c"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>(),
            &Options {
                directory_args: true,
                ..Options::default()
            },
            (&mut Vec::new(), &mut Vec::new()),
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            DirectoryPermissionDenied("b".to_string())
        );
    }
}

use std::env;

pub struct External {
    version_control: Option<String>,
    simple_backup_suffix: Option<String>,
}

impl External {
    pub fn from_env() -> Self {
        Self {
            version_control: env::var("VERSION_CONTROL").ok(),
            simple_backup_suffix: env::var("SIMPLE_BACKUP_SUFFIX").ok(),
        }
    }
}

#[derive(Debug, PartialEq)]
enum BackupKind {
    Unspecified,
    Specified(String),
}

#[derive(Debug, PartialEq)]
pub struct Provided {
    backup: Option<BackupKind>,
    suffix: Option<String>,
    verbose: bool,
    preserve_timestamps: bool,
    make_all_directories: bool,
    no_target_directory: bool,
    target_directory: Option<String>,
    arguments: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum ArgumentError {
    ArgumentRequired(String),
}

impl Provided {
    pub fn from_arguments(args: impl Iterator<Item = String>) -> Result<Self, ArgumentError> {
        let mut provided = Self {
            backup: None,
            suffix: None,
            verbose: false,
            preserve_timestamps: false,
            make_all_directories: false,
            no_target_directory: false,
            target_directory: None,
            arguments: Vec::new(),
        };

        let mut peekable = args.peekable();
        while let Some(arg) = peekable.next() {
            let mut split = arg.split('=');
            let (argument, parameter) = (unsafe { split.next().unwrap_unchecked() }, split.next());

            match (argument, parameter) {
                (a, None) => match a {
                    "-v" | "--verbose" => {
                        provided.verbose = true;
                        continue;
                    }
                    "-p" | "--preserve-timestamps" => {
                        provided.preserve_timestamps = true;
                        continue;
                    }
                    "-T" | "--no-target-directory" => {
                        provided.no_target_directory = true;
                        continue;
                    }
                    "-D" => {
                        provided.make_all_directories = true;
                        continue;
                    }
                    "-b" | "--backup" => {
                        provided.backup = Some(BackupKind::Unspecified);
                        continue;
                    }
                    "-S" => {
                        if let Some(suffix) = peekable.next() {
                            provided.suffix = Some(suffix);
                            continue;
                        }

                        return Err(ArgumentError::ArgumentRequired(arg));
                    }
                    "-t" => {
                        if let Some(directory) = peekable.next() {
                            provided.target_directory = Some(directory);
                            continue;
                        }

                        return Err(ArgumentError::ArgumentRequired(arg));
                    }
                    _ => (),
                },
                (a, Some(p)) => match a {
                    "--backup" => {
                        provided.backup = Some(BackupKind::Specified(p.to_owned()));
                        continue;
                    }
                    "--suffix" => {
                        provided.suffix = Some(p.to_owned());
                        continue;
                    }
                    "--target-directory" => {
                        provided.target_directory = Some(p.to_owned());
                        continue;
                    },
                    _ => (),
                },
            }

            provided.arguments.push(argument.to_owned());
        }

        Ok(provided)
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::{ArgumentError, BackupKind, External, Provided};
    use std::env;

    #[test]
    fn external_context_from_env_gets_correct_values() {
        env::set_var("VERSION_CONTROL", "my-version-control");
        env::set_var("SIMPLE_BACKUP_SUFFIX", "my-backup-suffix");

        let context = External::from_env();

        assert_eq!(
            context.version_control,
            Some("my-version-control".to_owned())
        );

        assert_eq!(
            context.simple_backup_suffix,
            Some("my-backup-suffix".to_owned())
        );

        env::remove_var("VERSION_CONTROL");
        env::remove_var("SIMPLE_BACKUP_SUFFIX");

        let context = External::from_env();
        assert_eq!(context.version_control, None);
        assert_eq!(context.simple_backup_suffix, None);
    }

    #[test]
    fn provided_parses_short_verbose() {
        let args = vec!["-v"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: None,
                verbose: true,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_parses_long_verbose() {
        let args = vec!["--verbose"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: None,
                verbose: true,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_parses_short_preserve_timestamps() {
        let args = vec!["-p"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: None,
                verbose: false,
                preserve_timestamps: true,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_parses_long_preserve_timestamps() {
        let args = vec!["--preserve-timestamps"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: None,
                verbose: false,
                preserve_timestamps: true,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_parses_short_no_target_directory() {
        let args = vec!["-T"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: None,
                verbose: false,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: true,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_parses_long_no_target_directory() {
        let args = vec!["--no-target-directory"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: None,
                verbose: false,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: true,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_parses_short_make_all_directories() {
        let args = vec!["-D"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: None,
                verbose: false,
                preserve_timestamps: false,
                make_all_directories: true,
                no_target_directory: false,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_parses_short_unspecified_backup() {
        let args = vec!["-b"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: Some(BackupKind::Unspecified),
                suffix: None,
                verbose: false,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_parses_long_unspecified_backup() {
        let args = vec!["--backup"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: Some(BackupKind::Unspecified),
                suffix: None,
                verbose: false,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_parses_long_specified_backup() {
        let args = vec!["--backup=option"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: Some(BackupKind::Specified("option".to_owned())),
                suffix: None,
                verbose: false,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_parses_short_suffix() {
        let args = vec!["-S", "option"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: Some("option".to_owned()),
                verbose: false,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_handles_missing_short_suffix() {
        let args = vec!["-S"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap_err(),
            ArgumentError::ArgumentRequired("-S".to_owned())
        );
    }

    #[test]
    fn provided_parses_long_suffix() {
        let args = vec!["--suffix=option"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: Some("option".to_owned()),
                verbose: false,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: None,
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_parses_short_target_directory() {
        let args = vec!["-t", "directory"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: None,
                verbose: false,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: Some("directory".to_owned()),
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_handles_missing_short_target_directory() {
        let args = vec!["-t"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap_err(),
            ArgumentError::ArgumentRequired("-t".to_owned())
        );
    }

    #[test]
    fn provided_parses_long_target_directory() {
        let args = vec!["--target-directory=directory"]
            .into_iter()
            .map(str::to_owned);

        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: None,
                verbose: false,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: Some("directory".to_owned()),
                arguments: Vec::new(),
            }
        );
    }

    #[test]
    fn provided_collects_arguments() {
        let args = vec!["a", "b", "c"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap(),
            Provided {
                backup: None,
                suffix: None,
                verbose: false,
                preserve_timestamps: false,
                make_all_directories: false,
                no_target_directory: false,
                target_directory: None,
                arguments: vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
            }
        )
    }
}

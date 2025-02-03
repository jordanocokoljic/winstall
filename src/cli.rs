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
}

#[derive(Debug, PartialEq)]
pub enum ParsingError {
    ArgumentRequired(String),
}

impl Provided {
    pub fn from_arguments(args: impl Iterator<Item = String>) -> Result<Self, ParsingError> {
        let mut provided = Self {
            backup: None,
            suffix: None,
            verbose: false,
            preserve_timestamps: false,
            make_all_directories: false,
            no_target_directory: false,
        };

        let mut peekable = args.peekable();
        while let Some(arg) = peekable.next() {
            let mut split = arg.split('=');
            let (argument, parameter) = (split.next(), split.next());

            match (argument, parameter) {
                (Some(a), None) => match a {
                    "-v" | "--verbose" => {
                        provided.verbose = true;
                    }
                    "-p" | "--preserve-timestamps" => {
                        provided.preserve_timestamps = true;
                    }
                    "-T" | "--no-target-directory" => {
                        provided.no_target_directory = true;
                    }
                    "-D" => {
                        provided.make_all_directories = true;
                    }
                    "-b" | "--backup" => {
                        provided.backup = Some(BackupKind::Unspecified);
                    }
                    "-S" => {
                        if let Some(suffix) = peekable.next() {
                            provided.suffix = Some(suffix);
                            continue;
                        }

                        return Err(ParsingError::ArgumentRequired(arg));
                    }
                    _ => (),
                },
                (Some(a), Some(p)) => match a {
                    "--backup" => {
                        provided.backup = Some(BackupKind::Specified(p.to_owned()));
                    }
                    "--suffix" => {
                        provided.suffix = Some(p.to_owned());
                    }
                    _ => (),
                },
                _ => (),
            }
        }

        Ok(provided)
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::{BackupKind, External, ParsingError, Provided};
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
    fn provided_parses_short_verbose_correctly() {
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
            }
        );
    }

    #[test]
    fn provided_parses_long_verbose_correctly() {
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
            }
        );
    }

    #[test]
    fn provided_parses_short_preserve_timestamps_correctly() {
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
            }
        );
    }

    #[test]
    fn provided_parses_long_preserve_timestamps_correctly() {
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
            }
        );
    }

    #[test]
    fn provided_parses_short_no_target_directory_correctly() {
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
            }
        );
    }

    #[test]
    fn provided_parses_long_no_target_directory_correctly() {
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
            }
        );
    }

    #[test]
    fn provided_parses_short_make_all_directories_correctly() {
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
            }
        );
    }

    #[test]
    fn provided_parses_short_unspecified_backup_correctly() {
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
            }
        );
    }

    #[test]
    fn provided_parses_long_unspecified_backup_correctly() {
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
            }
        );
    }

    #[test]
    fn provided_parses_long_specified_backup_correctly() {
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
            }
        );
    }

    #[test]
    fn provided_parses_short_suffix_correctly() {
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
            }
        );
    }

    #[test]
    fn provided_handles_missing_short_suffix_correctly() {
        let args = vec!["-S"].into_iter().map(str::to_owned);
        let provided = Provided::from_arguments(args);

        assert_eq!(
            provided.unwrap_err(),
            ParsingError::ArgumentRequired("-S".to_owned())
        );
    }

    #[test]
    fn provided_parses_long_suffix_correctly() {
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
            }
        );
    }
}

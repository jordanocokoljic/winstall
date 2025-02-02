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
pub struct Provided {
    backup: Option<String>,
    suffix: Option<String>,
    verbose: bool,
    preserve_timestamps: bool,
    make_all_directories: bool,
    no_target_directory: bool,
}

impl Provided {
    pub fn from_arguments(args: impl Iterator<Item = String>) -> Self {
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
            match arg.as_str() {
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
                _ => (),
            }
        }

        provided
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::{External, Provided};
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
            provided,
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
            provided,
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
            provided,
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
            provided,
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
            provided,
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
            provided,
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
            provided,
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
}

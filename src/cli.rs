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
}

impl Provided {
    pub fn from_arguments(args: impl Iterator<Item = String>) -> Self {
        let mut provided = Self {
            backup: None,
            suffix: None,
            verbose: false,
            preserve_timestamps: false,
            make_all_directories: false,
        };

        let mut peekable = args.peekable();
        while let Some(arg) = peekable.next() {
            match arg.as_str() {
                "-v" | "--verbose" => {
                    provided.verbose = true;
                }
                _ => (),
            }
        }

        provided
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use crate::cli::{External, Provided};

    #[test]
    fn external_context_from_env_gets_correct_values() {
        env::set_var("VERSION_CONTROL", "my-version-control");
        env::set_var("SIMPLE_BACKUP_SUFFIX", "my-backup-suffix");

        let context = External::from_env();
        assert_eq!(context.version_control, Some("my-version-control".to_owned()));
        assert_eq!(context.simple_backup_suffix, Some("my-backup-suffix".to_owned()));

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
            }
        );
    }
}
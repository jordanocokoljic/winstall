use crate::uopt::Hint;
use crate::winstall::{Error, Result};
use crate::{uopt, winstall};
use std::env;

#[derive(PartialEq, Clone, Copy, Debug)]
enum Backup {
    None,
    Numbered,
    Existing,
    Simple,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Options {
    verbose: bool,
    create_parents: bool,
    directory_args: bool,
    preserve_timestamps: bool,
    backup_type: Backup,
    backup_suffix: String,
    target_directory: Option<String>,
    no_target_directory: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            verbose: false,
            create_parents: false,
            directory_args: false,
            preserve_timestamps: false,
            backup_type: Backup::None,
            backup_suffix: "~".to_string(),
            target_directory: None,
            no_target_directory: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    version_control: Option<String>,
    simple_backup_suffix: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            version_control: env::var("VERSION_CONTROL").ok(),
            simple_backup_suffix: env::var("SIMPLE_BACKUP_SUFFIX").ok(),
        }
    }
}

struct Visitor {
    error: Option<Error>,
    config: Config,
    options: Options,
    arguments: Vec<String>,
}

impl Visitor {
    fn new(config: Config) -> Self {
        let mut options = Options::default();

        if let Some(suffix) = &config.simple_backup_suffix {
            options.backup_suffix = suffix.clone();
        }

        Self {
            error: None,
            config,
            options,
            arguments: Vec::new(),
        }
    }

    fn set_backup_from(&mut self, explicit: Option<&str>) {
        let mut set_backup = |indicator: &str, source: &str| {
            let strategy = match indicator {
                "none" | "off" => Ok(Backup::None),
                "simple" | "never" => Ok(Backup::Simple),
                "existing" | "nil" | "" => Ok(Backup::Existing),
                "numbered" | "t" => Ok(Backup::Numbered),
                var => Err(Error::InvalidArgument(var.to_string(), source.to_string())),
            };

            match strategy {
                Ok(strategy) => self.options.backup_type = strategy,
                Err(e) => self.error = Some(e),
            };
        };

        match (explicit, &self.config.simple_backup_suffix) {
            (Some(e), _) => set_backup(e, "backup type"),
            (None, Some(c)) => set_backup(c.as_ref(), "$VERSION_CONTROL"),
            (None, None) => self.options.backup_type = Backup::Existing,
        };
    }
}

impl uopt::Visitor for Visitor {
    fn visit_argument(&mut self, argument: &str) -> Option<Hint> {
        self.arguments.push(argument.to_string());
        Some(Hint::StopOptions)
    }

    fn visit_flag(&mut self, option: &str) -> Option<Hint> {
        match option {
            "v" | "verbose" => self.options.verbose = true,
            "D" => self.options.create_parents = true,
            "d" | "directory" => self.options.directory_args = true,
            "p" | "preserve-timestamps" => self.options.preserve_timestamps = true,
            "T" | "no-target-directory" => self.options.no_target_directory = true,
            "b" => self.set_backup_from(None),
            "backup" => return Some(Hint::Capture),
            "S" => return Some(Hint::Capture),
            "t" => return Some(Hint::Capture),
            "g" | "m" | "o" => return Some(Hint::Capture),
            "-C" | "--compare" | "-c" | "--debug" | "--preserve-context" | "--group" | "--mode"
            | "--owner" | "-s" | "--strip" | "--strip-program" => (),
            _ => (),
        };

        None
    }

    fn visit_parameter(&mut self, name: &str, parameter: Option<&str>) -> Option<Hint> {
        match name {
            "backup" => self.set_backup_from(parameter),
            "S" | "suffix" => match parameter {
                Some("") => self.options.backup_suffix = "~".to_string(),
                Some(suffix) => self.options.backup_suffix = suffix.to_string(),
                _ => (),
            },
            "t" | "target-directory" => {
                if let Some(dir) = parameter {
                    self.options.target_directory = Some(dir.to_string());
                }
            }
            _ => (),
        };

        None
    }
}

pub fn get_options<A: IntoIterator<Item = String>>(
    args: A,
    config: Config,
) -> Result<(Vec<String>, Options)> {
    let mut visitor = Visitor::new(config);
    uopt::visit(args.into_iter(), &mut visitor);

    match visitor.error {
        Some(e) => Err(e),
        None => Ok((visitor.arguments, visitor.options)),
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::{get_options, Backup, Config, Options};
    use crate::winstall::Error;
    use std::env;

    #[test]
    fn test_options_default() {
        let options = Options::default();
        assert_eq!(
            Options {
                verbose: false,
                create_parents: false,
                directory_args: false,
                preserve_timestamps: false,
                backup_type: Backup::None,
                backup_suffix: "~".to_string(),
                target_directory: None,
                no_target_directory: false,
            },
            options,
        );
    }

    #[test]
    fn test_get_options_simple() {
        struct TestCase<'a> {
            args: Vec<&'a str>,
            expected: Options,
        }

        let tests = vec![
            TestCase {
                args: vec!["--verbose"],
                expected: Options {
                    verbose: true,
                    ..Default::default()
                },
            },
            TestCase {
                args: vec!["-v"],
                expected: Options {
                    verbose: true,
                    ..Default::default()
                },
            },
            TestCase {
                args: vec!["-D"],
                expected: Options {
                    create_parents: true,
                    ..Default::default()
                },
            },
            TestCase {
                args: vec!["--directory"],
                expected: Options {
                    directory_args: true,
                    ..Default::default()
                },
            },
            TestCase {
                args: vec!["-d"],
                expected: Options {
                    directory_args: true,
                    ..Default::default()
                },
            },
            TestCase {
                args: vec!["--preserve-timestamps"],
                expected: Options {
                    preserve_timestamps: true,
                    ..Default::default()
                },
            },
            TestCase {
                args: vec!["-p"],
                expected: Options {
                    preserve_timestamps: true,
                    ..Default::default()
                },
            },
            TestCase {
                args: vec!["-T"],
                expected: Options {
                    no_target_directory: true,
                    ..Default::default()
                },
            },
            TestCase {
                args: vec!["--no-target-directory"],
                expected: Options {
                    no_target_directory: true,
                    ..Default::default()
                },
            },
        ];

        for test in tests {
            let config = Config::default();
            let args = test.args.iter().map(|arg| arg.to_string());
            let (_, outcome) = get_options(args, config).unwrap();
            assert_eq!(test.expected, outcome, "args: {:?}", test.args);
        }
    }

    #[test]
    fn test_get_options_backup() {
        const BACKUP_SETTINGS: &[(&str, Backup)] = &[
            ("none", Backup::None),
            ("off", Backup::None),
            ("simple", Backup::Simple),
            ("never", Backup::Simple),
            ("existing", Backup::Existing),
            ("nil", Backup::Existing),
            ("numbered", Backup::Numbered),
            ("t", Backup::Numbered),
        ];

        struct TestCase {
            argument: String,
            config_value: Option<String>,
            expected: Result<Options, Error>,
        }

        let mut tests = vec![
            TestCase {
                argument: "-b".to_string(),
                config_value: None,
                expected: Ok(Options {
                    backup_type: Backup::Existing,
                    ..Default::default()
                }),
            },
            TestCase {
                argument: "--backup".to_string(),
                config_value: None,
                expected: Ok(Options {
                    backup_type: Backup::Existing,
                    ..Default::default()
                }),
            },
            TestCase {
                argument: "-b".to_string(),
                config_value: Some("bad value".to_string()),
                expected: Err(Error::InvalidArgument(
                    "bad value".to_string(),
                    "$VERSION_CONTROL".to_string(),
                )),
            },
            TestCase {
                argument: "--backup=bad value".to_string(),
                config_value: None,
                expected: Err(Error::InvalidArgument(
                    "bad value".to_string(),
                    "backup type".to_string(),
                )),
            },
        ];

        for (config, backup) in BACKUP_SETTINGS {
            tests.push(TestCase {
                argument: "-b".to_string(),
                config_value: Some(config.to_string()),
                expected: Ok(Options {
                    backup_type: *backup,
                    ..Default::default()
                }),
            });

            tests.push(TestCase {
                argument: "--backup".to_string(),
                config_value: Some(config.to_string()),
                expected: Ok(Options {
                    backup_type: *backup,
                    ..Default::default()
                }),
            });

            tests.push(TestCase {
                argument: format!("--backup={}", config),
                config_value: None,
                expected: Ok(Options {
                    backup_type: *backup,
                    ..Default::default()
                }),
            });

            tests.push(TestCase {
                argument: "--backup=bad value".to_string(),
                config_value: Some(config.to_string()),
                expected: Err(Error::InvalidArgument(
                    "bad value".to_string(),
                    "backup type".to_string(),
                )),
            });

            for (nested_config, _) in BACKUP_SETTINGS {
                tests.push(TestCase {
                    argument: format!("--backup={}", config),
                    config_value: Some(nested_config.to_string()),
                    expected: Ok(Options {
                        backup_type: *backup,
                        ..Default::default()
                    }),
                });

                tests.push(TestCase {
                    argument: format!("--backup={}", config),
                    config_value: Some("bad value".to_string()),
                    expected: Ok(Options {
                        backup_type: *backup,
                        ..Default::default()
                    }),
                });
            }
        }

        for test in tests {
            let outcome = get_options(
                vec![test.argument.clone()].into_iter(),
                Config {
                    version_control: test.config_value.clone(),
                    ..Default::default()
                },
            )
            .map(|(_, o)| o);

            assert_eq!(
                test.expected, outcome,
                "arg: {:?}; config: {:?}",
                test.argument, test.config_value
            );
        }
    }

    #[test]
    fn test_get_options_suffix() {
        struct TestCase<'a> {
            args: Vec<&'a str>,
            config_suffix: Option<&'a str>,
            expected: Result<Options, Error>,
        }

        let tests = vec![
            TestCase {
                args: vec!["-S", "abc"],
                config_suffix: None,
                expected: Ok(Options {
                    backup_suffix: "abc".to_string(),
                    ..Default::default()
                }),
            },
            TestCase {
                args: vec!["-S", "abc"],
                config_suffix: Some("def"),
                expected: Ok(Options {
                    backup_suffix: "abc".to_string(),
                    ..Default::default()
                }),
            },
            TestCase {
                args: vec!["--suffix=abc"],
                config_suffix: None,
                expected: Ok(Options {
                    backup_suffix: "abc".to_string(),
                    ..Default::default()
                }),
            },
            TestCase {
                args: vec!["--suffix=abc"],
                config_suffix: Some("def"),
                expected: Ok(Options {
                    backup_suffix: "abc".to_string(),
                    ..Default::default()
                }),
            },
            TestCase {
                args: vec!["--suffix=abc"],
                config_suffix: Some("def"),
                expected: Ok(Options {
                    backup_suffix: "abc".to_string(),
                    ..Default::default()
                }),
            },
            TestCase {
                args: vec!["--suffix="],
                config_suffix: Some("def"),
                expected: Ok(Options {
                    backup_suffix: "~".to_string(),
                    ..Default::default()
                }),
            },
            TestCase {
                args: vec![],
                config_suffix: Some("abc"),
                expected: Ok(Options {
                    backup_suffix: "abc".to_string(),
                    ..Default::default()
                }),
            },
            TestCase {
                args: vec![],
                config_suffix: None,
                expected: Ok(Options {
                    backup_suffix: "~".to_string(),
                    ..Default::default()
                }),
            },
        ];

        for test in tests {
            let config = Config {
                simple_backup_suffix: test.config_suffix.map(|x| x.to_string()),
                ..Default::default()
            };

            let arguments = test.args.iter().map(|x| x.to_string());
            let outcome = get_options(arguments, config).map(|(_, o)| o);

            assert_eq!(
                test.expected, outcome,
                "args: {:?}; config_suffix: {:?}",
                test.args, test.config_suffix
            );
        }
    }

    #[test]
    fn test_get_options_target_directory() {
        struct TestCase<'a> {
            args: Vec<&'a str>,
            expected: Result<Options, Error>,
        }

        let tests = vec![
            TestCase {
                args: vec!["-t", "target_dir"],
                expected: Ok(Options {
                    target_directory: Some("target_dir".to_string()),
                    ..Default::default()
                }),
            },
            TestCase {
                args: vec!["--target-directory=target_dir"],
                expected: Ok(Options {
                    target_directory: Some("target_dir".to_string()),
                    ..Default::default()
                }),
            },
            TestCase {
                args: vec!["--target-directory="],
                expected: Ok(Options {
                    target_directory: Some("".to_string()),
                    ..Default::default()
                }),
            },
            TestCase {
                args: vec![],
                expected: Ok(Options {
                    target_directory: None,
                    ..Default::default()
                }),
            },
        ];

        for test in tests {
            let config = Config::default();
            let arguments = test.args.iter().map(|x| x.to_string());
            let outcome = get_options(arguments, config).map(|(_, o)| o);

            assert_eq!(test.expected, outcome, "args: {:?}", test.args)
        }
    }

    #[test]
    fn test_get_options_returns_unparsed_arguments() {
        struct TestCase<'a> {
            args: Vec<&'a str>,
            expected: Vec<&'a str>,
        }

        let tests = vec![
            TestCase {
                args: vec![
                    "--backup=existing",
                    "-D",
                    "abc.txt",
                    "def.txt",
                    "install/to",
                ],
                expected: vec!["abc.txt", "def.txt", "install/to"],
            },
            TestCase {
                args: vec!["-S", ".pre-install", "abc.txt", "move-to"],
                expected: vec!["abc.txt", "move-to"],
            },
            TestCase {
                args: vec!["-S", ".pre-install", "abc.txt", "-D", "move-to"],
                expected: vec!["abc.txt", "-D", "move-to"],
            },
        ];

        for test in tests {
            let config = Config::default();
            let arguments = test.args.iter().map(|x| x.to_string());
            let outcome = get_options(arguments, config).map(|(rest, _)| rest);

            let expected = Ok(test
                .expected
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>());

            assert_eq!(expected, outcome, "args: {:?}", test.args);
        }
    }

    #[test]
    fn test_get_options_parses_but_ignores_unix_specifics() {
        let tests = vec![
            vec!["-C", "--compare"],
            vec!["-c"],
            vec!["--debug"],
            vec!["-g", "wheel", "--group=wheel"],
            vec!["-m", "744", "--mode=744"],
            vec!["-o", "auser", "--owner=auser"],
            vec!["--preserve-context"],
            vec!["-s", "--strip"],
            vec!["--strip-program=program"],
        ];

        let expected = Ok(vec!["file.txt".to_string(), "install_dir".to_string()]);
        let statics = vec!["file.txt".to_string(), "install_dir".to_string()];

        for test_args in tests {
            let config = Config::default();
            let arguments = test_args
                .iter()
                .map(|x| x.to_string())
                .chain(statics.clone())
                .collect::<Vec<_>>();

            let outcome = get_options(arguments.clone(), config).map(|(rest, _)| rest);

            assert_eq!(expected, outcome, "args: {:?}", arguments);
        }
    }

    #[test]
    fn test_config_from_env() {
        env::remove_var("VERSION_CONTROL");
        env::remove_var("SIMPLE_BACKUP_SUFFIX");

        assert_eq!(
            Config {
                version_control: None,
                simple_backup_suffix: None,
            },
            Config::from_env()
        );

        env::set_var("VERSION_CONTROL", "first");
        env::set_var("SIMPLE_BACKUP_SUFFIX", "second");

        assert_eq!(
            Config {
                version_control: Some("first".to_string()),
                simple_backup_suffix: Some("second".to_string()),
            },
            Config::from_env(),
        )
    }
}

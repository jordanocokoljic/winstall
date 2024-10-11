use std::env;

fn main() -> Result<(), Error> {
    let config = Config::from_env();
    let (rest, options) = get_options(env::args().skip(1), config)?;
    Ok(())
}

#[derive(PartialEq, Debug)]
enum Error {
    InvalidArgument(String, String),
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum Backup {
    None,
    Numbered,
    Existing,
    Simple,
}

#[derive(PartialEq, Debug)]
struct Options {
    verbose: bool,
    create_parents: bool,
    directory_args: bool,
    preserve_timestamps: bool,
    backup_type: Backup,
    backup_suffix: String,
    target_directory: Option<String>,
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
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Config {
    version_control: Option<String>,
    simple_backup_suffix: Option<String>,
}

impl Config {
    fn from_env() -> Self {
        Self {
            version_control: env::var("VERSION_CONTROL").ok(),
            simple_backup_suffix: env::var("SIMPLE_BACKUP_SUFFIX").ok(),
        }
    }
}

fn get_options<A: IntoIterator<Item = String>>(
    args: A,
    config: Config,
) -> Result<(Vec<String>, Options), Error> {
    fn determine_backup_type(indicator: &str, source: &str) -> Result<Backup, Error> {
        match indicator {
            "none" | "off" => Ok(Backup::None),
            "simple" | "never" => Ok(Backup::Simple),
            "existing" | "nil" | "" => Ok(Backup::Existing),
            "numbered" | "t" => Ok(Backup::Numbered),
            var => Err(Error::InvalidArgument(var.to_string(), source.to_string())),
        }
    }

    let mut arguments = args.into_iter().peekable();
    let mut context = Options::default();

    if let Some(suffix) = config.simple_backup_suffix {
        context.backup_suffix = suffix;
    }

    while let Some(arg) = arguments.peek() {
        let mut split = arg.split("=");
        let opt_or_arg = split.next().unwrap();

        match opt_or_arg {
            "-v" | "--verbose" => context.verbose = true,
            "-D" => context.create_parents = true,
            "-d" | "--directory" => context.directory_args = true,
            "-p" | "--preserve-timestamps" => context.preserve_timestamps = true,
            "-b" => {
                context.backup_type = {
                    if let Some(vc) = &config.version_control {
                        determine_backup_type(vc, "$VERSION_CONTROL")?
                    } else {
                        Backup::Existing
                    }
                }
            }
            "--backup" => {
                context.backup_type = {
                    if let Some(specified) = split.next() {
                        determine_backup_type(specified, "backup type")?
                    } else if let Some(vc) = &config.version_control {
                        determine_backup_type(vc, "$VERSION_CONTROL")?
                    } else {
                        Backup::Existing
                    }
                }
            }
            "-S" => {
                arguments.next();

                if let Some(suffix) = arguments.peek() {
                    context.backup_suffix = suffix.to_string();
                }
            }
            "--suffix" => {
                context.backup_suffix = match split.next() {
                    Some("") | None => "~".to_string(),
                    Some(suffix) => suffix.to_string(),
                }
            }
            "-t" => {
                arguments.next();

                if let Some(target) = arguments.peek() {
                    context.target_directory = Some(target.to_string());
                }
            }
            "--target-directory" => {
                if let Some(target) = split.next() {
                    context.target_directory = Some(target.to_string());
                }
            }
            "-g" | "-m" | "-o" => {
                arguments.next();
            }
            "-C" | "--compare" | "-c" | "--debug" | "--preserve-context" | "--group" | "--mode"
            | "--owner" | "-s" | "--strip" | "--strip-program" => (),
            _ => break,
        }

        arguments.next();
    }

    Ok((arguments.collect(), context))
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::{get_options, Backup, Config, Error, Options};

    #[test]
    pub fn test_options_default() {
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
            },
            options,
        );
    }

    #[test]
    pub fn test_get_options_simple() {
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

use std::env;

fn main() {
    let config = winstall::Config {
        version_control: None,
    };
    let context = winstall::get_options(env::args().skip(1), &config);
    println!("{context:?}");
}

mod winstall {
    #[derive(PartialEq, Clone, Copy, Debug, Default)]
    pub enum Backup {
        #[default]
        None,
        Numbered,
        Existing,
        Simple,
    }

    #[derive(PartialEq, Debug, Default)]
    pub struct Options {
        verbose: bool,
        create_parents: bool,
        directory_args: bool,
        preserve_timestamps: bool,
        backup_type: Backup,
    }

    #[derive(PartialEq, Debug)]
    pub enum Error {
        InvalidArgument(String, String),
    }

    #[derive(Clone, Debug, Default)]
    pub struct Config {
        pub version_control: Option<String>,
    }

    pub fn get_options<A: Iterator<Item = String>>(
        args: A,
        config: &Config,
    ) -> Result<Options, Error> {
        let collected = args.collect::<Vec<_>>();
        let index_at_end = |index: usize| index < collected.len();

        /*
        The options that are supported from `install` are:
            [ ] --help*
            [ ] --version*
            [x] -b, --backup[=method]
            [x] -p, --preserve-timestamps
            [x] -d, --directory
            [x] -v, --verbose
            [ ] -t, --target-directory=DIRECTORY
            [x] -D
            [ ] -S, --suffix=SUFFIX

        Items marked with a * are halting options - the standard execution
        is prevented, and an alternate action takes place - so they're not
        context items but some sort of "Action".
        */

        let mut idx = 0;
        let mut context = Options::default();

        fn determine_backup_type(indicator: &str, source: &str) -> Result<Backup, Error> {
            match indicator {
                "none" | "off" => Ok(Backup::None),
                "simple" | "never" => Ok(Backup::Simple),
                "existing" | "nil" | "" => Ok(Backup::Existing),
                "numbered" | "t" => Ok(Backup::Numbered),
                var => Err(Error::InvalidArgument(var.to_string(), source.to_string())),
            }
        }

        while index_at_end(idx) {
            let mut split = collected[idx].split("=");
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
                _ => (),
            }

            idx += 1;
        }

        Ok(context)
    }

    #[cfg(test)]
    mod tests {
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

            let config = Config {
                version_control: None,
            };

            for test in tests {
                let args = test.args.iter().map(|arg| arg.to_string());
                let outcome = get_options(args, &config).unwrap();
                assert_eq!(test.expected, outcome, "args: {:?}", test.args);
            }
        }

        #[test]
        fn test_get_options_backup() {
            let settings = vec![
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

            for (config, backup) in &settings {
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

                for (nested_config, _) in &settings {
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
                    &Config {
                        version_control: test.config_value.clone(),
                        ..Default::default()
                    },
                );

                assert_eq!(
                    test.expected, outcome,
                    "arg: {:?}; config: {:?}",
                    test.argument, test.config_value
                );
            }
        }
    }
}

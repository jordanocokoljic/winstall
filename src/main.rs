use std::env;

fn main() {
    let config = winstall::Config{ version_control: None };
    let context = winstall::get_options(env::args().skip(1), &config);
    println!("{context:?}");
}

mod winstall {
    #[derive(PartialEq, Debug, Default)]
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

    #[derive(Clone)]
    pub struct Config {
        pub version_control: Option<String>,
    }

    pub fn get_options<A: Iterator<Item = String>>(args: A, config: &Config) -> Result<Options, Error> {
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

        while index_at_end(idx) {
            let opt_or_arg = collected[idx].split("=").nth(0).unwrap();
            match opt_or_arg {
                "-v" | "--verbose" => context.verbose = true,
                "-D" => context.create_parents = true,
                "-d" | "--directory" => context.directory_args = true,
                "-p" | "--preserve-timestamps" => context.preserve_timestamps = true,
                "-b" | "--backup" => {
                    context.backup_type =
                        match config.version_control.clone().unwrap_or_default().as_str() {
                            "none" | "off" => Backup::None,
                            "simple" | "never" => Backup::Simple,
                            "existing" | "nil" | "" => Backup::Existing,
                            "numbered" | "t" => Backup::Numbered,
                            var => {
                                return Err(Error::InvalidArgument(
                                    var.to_string(),
                                    "$VERSION_CONTROL".to_string(),
                                ))
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
        use super::{get_options, Backup, Error, Options, Config};

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
        pub fn test_get_options() {
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
                    args: vec!["--backup"],
                    expected: Options {
                        backup_type: Backup::Existing,
                        ..Default::default()
                    },
                },
                TestCase {
                    args: vec!["-b"],
                    expected: Options {
                        backup_type: Backup::Existing,
                        ..Default::default()
                    },
                },
            ];

            let config = Config{
                version_control: None,
            };

            for test in tests {
                let args = test.args.iter().map(|arg| arg.to_string());
                let outcome = get_options(args, &config).unwrap();
                assert_eq!(test.expected, outcome, "args: {:?}", test.args);
            }
        }

        #[test]
        fn get_options_reads_version_control_correctly() {
            struct TestCase<'a> {
                version_control: &'a str,
                expected: Result<Options, Error>,
            }

            let tests = vec![
                TestCase {
                    version_control: "none",
                    expected: Ok(Options {
                        backup_type: Backup::None,
                        ..Default::default()
                    }),
                },
                TestCase {
                    version_control: "off",
                    expected: Ok(Options {
                        backup_type: Backup::None,
                        ..Default::default()
                    }),
                },
                TestCase {
                    version_control: "simple",
                    expected: Ok(Options {
                        backup_type: Backup::Simple,
                        ..Default::default()
                    }),
                },
                TestCase {
                    version_control: "never",
                    expected: Ok(Options {
                        backup_type: Backup::Simple,
                        ..Default::default()
                    }),
                },
                TestCase {
                    version_control: "existing",
                    expected: Ok(Options {
                        backup_type: Backup::Existing,
                        ..Default::default()
                    }),
                },
                TestCase {
                    version_control: "nil",
                    expected: Ok(Options {
                        backup_type: Backup::Existing,
                        ..Default::default()
                    }),
                },
                TestCase {
                    version_control: "numbered",
                    expected: Ok(Options {
                        backup_type: Backup::Numbered,
                        ..Default::default()
                    }),
                },
                TestCase {
                    version_control: "t",
                    expected: Ok(Options {
                        backup_type: Backup::Numbered,
                        ..Default::default()
                    }),
                },
                TestCase {
                    version_control: "other",
                    expected: Err(Error::InvalidArgument(
                        "other".to_string(),
                        "$VERSION_CONTROL".to_string(),
                    )),
                },
            ];

            for test in tests {
                let config = Config {
                    version_control: Some(test.version_control.to_string()),
                };

                let outcome = get_options(vec!["-b".to_string()].into_iter(), &config);
                assert_eq!(
                    test.expected, outcome,
                    "env: $VERSION_CONTROL = {}",
                    test.version_control
                );
            }
        }
    }
}

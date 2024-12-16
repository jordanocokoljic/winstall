pub enum Hint {
    Capture,
    StopOptions,
    Halt,
}

pub trait Visitor {
    fn visit_argument(&mut self, argument: &str) -> Option<Hint>;
    fn visit_flag(&mut self, option: &str) -> Option<Hint>;
    fn visit_parameter(&mut self, name: &str, parameter: Option<&str>) -> Option<Hint>;
}

pub fn visit(arguments: impl Iterator<Item = String>, visitor: &mut impl Visitor) {
    let mut take_options = true;
    let mut peekable = arguments.peekable();

    fn is_long(argument: &str) -> bool {
        argument.starts_with("--")
            && if let Some(c) = &argument[2..].chars().next() {
                c.is_alphabetic()
            } else {
                false
            }
    }

    fn is_short(argument: &str) -> bool {
        argument.starts_with("-")
            && if let Some(c) = &argument[1..].chars().next() {
                c.is_alphabetic()
            } else {
                false
            }
    }

    while let Some(argument) = peekable.next() {
        match argument.as_str() {
            "--" if take_options => {
                take_options = false;
            }
            long if take_options && is_long(&argument) => 'long: {
                let trimmed = &long[2..];

                let mut split = trimmed.split("=");
                if let (Some(name), Some(parameter)) = (split.next(), split.next()) {
                    let hint = visitor.visit_parameter(name, Some(parameter));

                    match hint {
                        Some(Hint::StopOptions) => take_options = false,
                        Some(Hint::Halt) => return,
                        _ => (),
                    }

                    break 'long;
                }

                let hint = visitor.visit_flag(trimmed);
                if let Some(Hint::Capture) = hint {
                    let parameter = peekable.peek();

                    let parameter_hint = match parameter {
                        Some(param) => visitor.visit_parameter(trimmed, Some(param.as_str())),
                        None => visitor.visit_parameter(trimmed, None),
                    };

                    match parameter_hint {
                        Some(Hint::StopOptions) => take_options = false,
                        Some(Hint::Halt) => return,
                        _ => (),
                    }

                    peekable.next();
                }
            }
            short if take_options && is_short(&argument) => 'short: {
                let trimmed = &short[1..];
                let mut chars = trimmed.chars().enumerate().peekable();

                while let Some((from, _)) = chars.next() {
                    let peek = chars.peek();
                    let hint = visitor
                        .visit_flag(&trimmed[from..peek.map_or(trimmed.len(), |(to, _)| *to)]);

                    match hint {
                        Some(Hint::StopOptions) => {
                            take_options = false;
                        }
                        Some(Hint::Capture) => {
                            if let Some((to, _)) = peek {
                                let parameter_hint = visitor
                                    .visit_parameter(&trimmed[from..*to], Some(&trimmed[*to..]));
                                match parameter_hint {
                                    Some(Hint::StopOptions) => take_options = false,
                                    Some(Hint::Halt) => return,
                                    _ => (),
                                }

                                break 'short;
                            }

                            let parameter = peekable.peek();

                            let parameter_hint = match parameter {
                                Some(param) => {
                                    visitor.visit_parameter(trimmed, Some(param.as_str()))
                                }
                                None => visitor.visit_parameter(trimmed, None),
                            };

                            match parameter_hint {
                                Some(Hint::StopOptions) => take_options = false,
                                Some(Hint::Halt) => return,
                                _ => (),
                            }

                            peekable.next();
                        }
                        Some(Hint::Halt) => {
                            return;
                        }
                        None => (),
                    }
                }
            }
            arg => {
                let hint = visitor.visit_argument(arg);

                match hint {
                    Some(Hint::StopOptions) => take_options = false,
                    Some(Hint::Halt) => return,
                    _ => (),
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::uopt::tests::double::{CollectingVisitor, Expect, OverridableVisitor};
    use crate::uopt::{visit, Hint};

    #[test]
    fn test_visit_handles_arguments() {
        let args = ["alpha", "beta", "gamma"].map(str::to_string);
        let mut visitor = CollectingVisitor::new();

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Argument("alpha"),
                Expect::Argument("beta"),
                Expect::Argument("gamma"),
            ]
        );
    }

    #[test]
    fn test_visit_handles_long_form_options() {
        let args = ["--action", "--0chance", "--config=value", "--take="].map(str::to_string);
        let mut visitor = CollectingVisitor::new();

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Flag("action"),
                Expect::Argument("--0chance"),
                Expect::Parameter("config", Some("value")),
                Expect::Parameter("take", Some("")),
            ]
        );
    }

    #[test]
    fn test_visit_handles_double_dash_indicator() {
        let args = ["--option", "--", "--not-option", "argument", "--"].map(str::to_string);
        let mut visitor = CollectingVisitor::new();

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Flag("option"),
                Expect::Argument("--not-option"),
                Expect::Argument("argument"),
                Expect::Argument("--"),
            ]
        );
    }

    #[test]
    fn test_visit_can_capture_subsequent_item_with_long_form() {
        let args = ["--config", "value", "--oops"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |flag: &str| -> Option<Hint> {
                if flag == "config" || flag == "oops" {
                    Some(Hint::Capture)
                } else {
                    None
                }
            },
            OverridableVisitor::ignore_parameter,
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Parameter("config", Some("value")),
                Expect::Parameter("oops", None),
            ]
        );
    }

    #[test]
    fn test_visit_handles_short_form_options() {
        let args = ["-a", "-bc", "-1"].map(str::to_string);
        let mut visitor = CollectingVisitor::new();

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Flag("a"),
                Expect::Flag("b"),
                Expect::Flag("c"),
                Expect::Argument("-1")
            ]
        );
    }

    #[test]
    fn test_visit_can_capture_subsequent_item_with_short_form() {
        let args = ["-a", "value", "-b"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |flag: &str| -> Option<Hint> {
                if flag == "a" || flag == "b" {
                    Some(Hint::Capture)
                } else {
                    None
                }
            },
            OverridableVisitor::ignore_parameter,
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Parameter("a", Some("value")),
                Expect::Parameter("b", None),
            ]
        );
    }

    #[test]
    fn test_visit_can_capture_remainder_of_short_form_option_as_parameter() {
        let args = ["-avalue", "-b"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |flag: &str| -> Option<Hint> {
                if flag == "a" {
                    Some(Hint::Capture)
                } else {
                    None
                }
            },
            OverridableVisitor::ignore_parameter,
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![Expect::Parameter("a", Some("value")), Expect::Flag("b")]
        );
    }

    #[test]
    fn test_visit_recognizes_stop_options_from_argument() {
        let args = ["-a", "value", "-b"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            |_: &str| -> Option<Hint> { Some(Hint::StopOptions) },
            OverridableVisitor::ignore_flag,
            OverridableVisitor::ignore_parameter,
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Flag("a"),
                Expect::Argument("value"),
                Expect::Argument("-b")
            ]
        );
    }

    #[test]
    fn test_visit_recognizes_stop_options_from_unit_flag() {
        let args = ["abc", "-d", "-e"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |_: &str| -> Option<Hint> { Some(Hint::StopOptions) },
            OverridableVisitor::ignore_parameter,
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Argument("abc"),
                Expect::Flag("d"),
                Expect::Argument("-e")
            ],
        );
    }

    #[test]
    fn test_visit_recognizes_stop_options_from_composite_flag() {
        let args = ["abc", "-def", "-g"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |flag: &str| -> Option<Hint> {
                if flag == "d" {
                    Some(Hint::StopOptions)
                } else {
                    None
                }
            },
            OverridableVisitor::ignore_parameter,
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Argument("abc"),
                Expect::Flag("d"),
                Expect::Flag("e"),
                Expect::Flag("f"),
                Expect::Argument("-g"),
            ],
        );
    }

    #[test]
    fn test_visit_recognizes_stop_options_from_aggregate_parameter() {
        let args = ["abc", "--def=ghi", "--jkl"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            OverridableVisitor::ignore_flag,
            |flag: &str, _: Option<&str>| -> Option<Hint> {
                if flag == "def" {
                    Some(Hint::StopOptions)
                } else {
                    None
                }
            },
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Argument("abc"),
                Expect::Parameter("def", Some("ghi")),
                Expect::Argument("--jkl"),
            ],
        );
    }

    #[test]
    fn test_visit_recognizes_stop_options_from_capturing_long_parameter() {
        let args = ["abc", "--def", "ghi", "--jkl"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |flag: &str| -> Option<Hint> {
                if flag == "def" {
                    Some(Hint::Capture)
                } else {
                    None
                }
            },
            |flag: &str, _: Option<&str>| -> Option<Hint> {
                if flag == "def" {
                    Some(Hint::StopOptions)
                } else {
                    None
                }
            },
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Argument("abc"),
                Expect::Parameter("def", Some("ghi")),
                Expect::Argument("--jkl"),
            ]
        )
    }

    #[test]
    fn test_visit_recognizes_stop_options_from_capturing_short_parameter() {
        let args = ["abc", "-d", "efg", "-hij"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |flag: &str| -> Option<Hint> {
                if flag == "d" {
                    Some(Hint::Capture)
                } else {
                    None
                }
            },
            |flag: &str, _: Option<&str>| -> Option<Hint> {
                if flag == "d" {
                    Some(Hint::StopOptions)
                } else {
                    None
                }
            },
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Argument("abc"),
                Expect::Parameter("d", Some("efg")),
                Expect::Argument("-hij"),
            ]
        );
    }

    #[test]
    fn test_visit_recognizes_stop_options_from_capturing_short_aggregate_parameter() {
        let args = ["abc", "-defg", "-hij"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |flag: &str| -> Option<Hint> {
                if flag == "d" {
                    Some(Hint::Capture)
                } else {
                    None
                }
            },
            |flag: &str, _: Option<&str>| -> Option<Hint> {
                if flag == "d" {
                    Some(Hint::StopOptions)
                } else {
                    None
                }
            },
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Argument("abc"),
                Expect::Parameter("d", Some("efg")),
                Expect::Argument("-hij"),
            ]
        );
    }

    #[test]
    fn test_visit_recognizes_halt_from_argument() {
        let args = ["-a", "value", "-b"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            |_: &str| -> Option<Hint> { Some(Hint::Halt) },
            OverridableVisitor::ignore_flag,
            OverridableVisitor::ignore_parameter,
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![Expect::Flag("a"), Expect::Argument("value"),],
        );
    }

    #[test]
    fn test_visit_recognizes_halt_from_unit_flag() {
        let args = ["abc", "-d", "-e"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |_: &str| -> Option<Hint> { Some(Hint::Halt) },
            OverridableVisitor::ignore_parameter,
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![Expect::Argument("abc"), Expect::Flag("d"),],
        );
    }

    #[test]
    fn test_visit_recognizes_halt_from_composite_flag() {
        let args = ["abc", "-def", "-g"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |flag: &str| -> Option<Hint> {
                if flag == "e" {
                    Some(Hint::Halt)
                } else {
                    None
                }
            },
            OverridableVisitor::ignore_parameter,
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Argument("abc"),
                Expect::Flag("d"),
                Expect::Flag("e"),
            ],
        );
    }

    #[test]
    fn test_visit_recognizes_halt_from_aggregate_parameter() {
        let args = ["abc", "--def=ghi", "--jkl"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            OverridableVisitor::ignore_flag,
            |flag: &str, _: Option<&str>| -> Option<Hint> {
                if flag == "def" {
                    Some(Hint::Halt)
                } else {
                    None
                }
            },
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Argument("abc"),
                Expect::Parameter("def", Some("ghi")),
            ],
        );
    }

    #[test]
    fn test_visit_recognizes_halt_from_capturing_long_parameter() {
        let args = ["abc", "--def", "ghi", "--jkl"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |flag: &str| -> Option<Hint> {
                if flag == "def" {
                    Some(Hint::Capture)
                } else {
                    None
                }
            },
            |flag: &str, _: Option<&str>| -> Option<Hint> {
                if flag == "def" {
                    Some(Hint::Halt)
                } else {
                    None
                }
            },
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![
                Expect::Argument("abc"),
                Expect::Parameter("def", Some("ghi")),
            ]
        )
    }

    #[test]
    fn test_visit_recognizes_halt_from_capturing_short_parameter() {
        let args = ["abc", "-d", "efg", "-hij"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |flag: &str| -> Option<Hint> {
                if flag == "d" {
                    Some(Hint::Capture)
                } else {
                    None
                }
            },
            |flag: &str, _: Option<&str>| -> Option<Hint> {
                if flag == "d" {
                    Some(Hint::Halt)
                } else {
                    None
                }
            },
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![Expect::Argument("abc"), Expect::Parameter("d", Some("efg")),]
        );
    }

    #[test]
    fn test_visit_recognizes_halt_from_capturing_short_aggregate_parameter() {
        let args = ["abc", "-defg", "-hij"].map(str::to_string);
        let mut visitor = OverridableVisitor::new(
            OverridableVisitor::ignore_argument,
            |flag: &str| -> Option<Hint> {
                if flag == "d" {
                    Some(Hint::Capture)
                } else {
                    None
                }
            },
            |flag: &str, _: Option<&str>| -> Option<Hint> {
                if flag == "d" {
                    Some(Hint::Halt)
                } else {
                    None
                }
            },
        );

        visit(args.into_iter(), &mut visitor);

        assert_eq!(
            visitor.items(),
            vec![Expect::Argument("abc"), Expect::Parameter("d", Some("efg")),]
        );
    }

    mod double {
        use crate::uopt::{Hint, Visitor};
        use std::fmt::{Debug, Formatter};

        #[derive(Debug)]
        pub enum Expect<'a> {
            Argument(&'a str),
            Flag(&'a str),
            Parameter(&'a str, Option<&'a str>),
        }

        #[derive(Clone, Debug)]
        enum Item {
            Argument(String),
            Flag(String),
            Parameter(String, Option<String>),
        }

        #[derive(Clone)]
        pub struct ItemSet(Vec<Item>);

        impl ItemSet {
            fn new() -> Self {
                ItemSet(Vec::new())
            }

            fn push_argument(&mut self, argument: &str) {
                self.0.push(Item::Argument(argument.to_owned()));
            }

            fn push_flag(&mut self, flag: &str) {
                self.0.push(Item::Flag(flag.to_owned()));
            }

            fn push_parameter(&mut self, name: &str, value: Option<&str>) {
                self.0.push(Item::Parameter(
                    name.into(),
                    match value {
                        Some(param) => Some(param.to_owned()),
                        None => None,
                    },
                ));
            }
        }

        impl Debug for ItemSet {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:?}", self.0)
            }
        }

        impl PartialEq<Vec<Expect<'_>>> for ItemSet {
            fn eq(&self, other: &Vec<Expect<'_>>) -> bool {
                if self.0.len() != other.len() {
                    return false;
                }

                for (i, item) in self.0.iter().enumerate() {
                    let result = match (item, &other[i]) {
                        (Item::Argument(a), Expect::Argument(b)) => a.eq(b),
                        (Item::Flag(a), Expect::Flag(b)) => a.eq(b),
                        (Item::Parameter(an, ap), Expect::Parameter(bn, bp)) => {
                            an.eq(bn) && {
                                match (ap, bp) {
                                    (Some(a), Some(b)) => a.eq(b),
                                    (None, None) => true,
                                    _ => false,
                                }
                            }
                        }
                        _ => false,
                    };

                    if result == false {
                        return false;
                    }
                }

                true
            }
        }

        pub struct CollectingVisitor {
            items: ItemSet,
        }

        impl CollectingVisitor {
            pub fn new() -> Self {
                CollectingVisitor {
                    items: ItemSet::new(),
                }
            }

            pub fn items(&self) -> ItemSet {
                self.items.clone()
            }
        }

        impl Visitor for CollectingVisitor {
            fn visit_argument(&mut self, argument: &str) -> Option<Hint> {
                self.items.push_argument(argument);
                None
            }

            fn visit_flag(&mut self, option: &str) -> Option<Hint> {
                self.items.push_flag(option);
                None
            }

            fn visit_parameter(&mut self, name: &str, value: Option<&str>) -> Option<Hint> {
                self.items.push_parameter(name, value);
                None
            }
        }

        pub struct OverridableVisitor {
            collecting_visitor: CollectingVisitor,
            override_visit_argument: fn(&str) -> Option<Hint>,
            override_visit_flag: fn(&str) -> Option<Hint>,
            override_visit_parameter: fn(&str, Option<&str>) -> Option<Hint>,
        }

        impl OverridableVisitor {
            pub fn new(
                visit_argument: fn(&str) -> Option<Hint>,
                visit_flag: fn(&str) -> Option<Hint>,
                visit_parameter: fn(&str, Option<&str>) -> Option<Hint>,
            ) -> Self {
                Self {
                    collecting_visitor: CollectingVisitor::new(),
                    override_visit_argument: visit_argument,
                    override_visit_flag: visit_flag,
                    override_visit_parameter: visit_parameter,
                }
            }

            pub fn items(&self) -> ItemSet {
                self.collecting_visitor.items()
            }

            pub fn ignore_argument(_: &str) -> Option<Hint> {
                None
            }

            pub fn ignore_flag(_: &str) -> Option<Hint> {
                None
            }

            pub fn ignore_parameter(_: &str, _: Option<&str>) -> Option<Hint> {
                None
            }
        }

        impl Visitor for OverridableVisitor {
            fn visit_argument(&mut self, argument: &str) -> Option<Hint> {
                let hint = (self.override_visit_argument)(argument);

                match hint {
                    Some(Hint::Capture) => hint,
                    Some(Hint::StopOptions) | Some(Hint::Halt) | None => {
                        self.collecting_visitor.visit_argument(argument);
                        hint
                    }
                }
            }

            fn visit_flag(&mut self, option: &str) -> Option<Hint> {
                let hint = (self.override_visit_flag)(option);

                match hint {
                    Some(Hint::Capture) => hint,
                    Some(Hint::StopOptions) | Some(Hint::Halt) | None => {
                        self.collecting_visitor.visit_flag(option);
                        hint
                    }
                }
            }

            fn visit_parameter(&mut self, name: &str, parameter: Option<&str>) -> Option<Hint> {
                let hint = (self.override_visit_parameter)(name, parameter);

                match hint {
                    Some(Hint::Capture) => hint,
                    Some(Hint::StopOptions) | Some(Hint::Halt) | None => {
                        self.collecting_visitor.visit_parameter(name, parameter);
                        hint
                    }
                }
            }
        }
    }
}

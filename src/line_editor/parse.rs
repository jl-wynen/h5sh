use super::scanner::Scanner;
use super::text_range::TextRange;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    Call(CallExpression),
    #[allow(dead_code)] // exists for future use
    String(StringExpression),
    Noop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallExpression {
    pub function: StringExpression,
    pub arguments: Vec<Argument>,
    pub(super) range: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StringExpression {
    pub(super) range: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Argument {
    Plain(StringExpression),
    Long(StringExpression),
    Short(StringExpression),
}

impl CallExpression {
    pub fn get_args_str<'s>(&self, src: &'s str) -> &'s str {
        &src[self.function.range.end().as_index()..self.range.end().as_index()]
    }
}

impl StringExpression {
    pub fn get_content<'s>(&self, src: &'s str) -> &'s str {
        &src[self.range]
    }
}

impl Argument {
    pub(super) fn range(&self) -> TextRange {
        match self {
            Argument::Plain(expr) => expr.range,
            Argument::Long(expr) => expr.range,
            Argument::Short(expr) => expr.range,
        }
    }

    pub fn get_content<'s>(&self, src: &'s str) -> &'s str {
        &src[self.range()]
    }
}

#[derive(Debug)]
pub struct Parser<'a> {
    scanner: Scanner<'a>,
    current_range: TextRange,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            scanner: Scanner::new(src),
            current_range: Default::default(),
        }
    }

    pub fn parse(&mut self) -> Expression {
        self.parse_expression()
    }

    fn parse_expression(&mut self) -> Expression {
        let Some(call) = self.maybe_parse_call_expression() else {
            return Expression::Noop;
        };
        Expression::Call(call)
    }

    fn maybe_parse_call_expression(&mut self) -> Option<CallExpression> {
        let function = self.parse_string();
        if function.range.is_empty() {
            return None;
        }
        let mut call_range = function.range;

        let mut arguments = Vec::new();
        while let Some(arg) = self.maybe_parse_argument() {
            arguments.push(arg);
        }
        if let Some(arg) = arguments.last() {
            call_range.extend_to(arg.range().end());
        }

        let call = CallExpression {
            function,
            arguments,
            range: call_range,
        };
        Some(call)
    }

    fn parse_string(&mut self) -> StringExpression {
        self.parse_string_with_terminator(|_| false)
    }

    fn parse_string_with_terminator<T: Fn(char) -> bool>(
        &mut self,
        terminator: T,
    ) -> StringExpression {
        self.eat_whitespace();
        self.start_token();
        while !self.scanner.current().is_whitespace()
            && !self.scanner.is_finished()
            && !terminator(self.scanner.current())
        {
            self.eat();
        }
        // self.current_range.extend_to(self.scanner.current_index());
        StringExpression {
            range: self.current_range,
        }
    }

    fn maybe_parse_argument(&mut self) -> Option<Argument> {
        self.eat_whitespace();
        if self.scanner.current() == '-' {
            Some(self.parse_keyword_argument())
        } else {
            self.parse_plain_argument()
        }
    }

    fn parse_plain_argument(&mut self) -> Option<Argument> {
        let arg = self.parse_string();
        if arg.range.is_empty() {
            None
        } else {
            Some(Argument::Plain(arg))
        }
    }

    // This function assumes that scanner.current is the first '-' of the arg.
    fn parse_keyword_argument(&mut self) -> Argument {
        let start = self.scanner.current_index();
        self.start_token();
        if self.eat() == '-' {
            // skip over second '-'
            if self.eat().is_whitespace() {
                Argument::Long(StringExpression {
                    range: self.current_range,
                })
            } else {
                let mut arg = self.parse_string_with_terminator(|c| c == '=');
                arg.range.extend_backwards_to(start);
                if self.scanner.current() == '=' {
                    self.eat(); // skip '=', it has no actual syntactic meaning
                }
                Argument::Long(arg)
            }
        } else if self.scanner.current().is_whitespace() {
            Argument::Short(StringExpression {
                range: self.current_range,
            })
        } else {
            let mut arg = self.parse_string();
            arg.range.extend_backwards_to(start);
            Argument::Short(arg)
        }
    }

    fn eat(&mut self) -> char {
        let res = self.scanner.eat();
        self.current_range.extend_to(self.scanner.current_index());
        res
    }

    fn eat_whitespace(&mut self) {
        while self.scanner.current().is_whitespace() {
            self.eat();
        }
    }

    fn start_token(&mut self) {
        self.current_range = TextRange::start_new(self.scanner.current_index());
    }
}

#[cfg(test)]
mod tests {
    use super::Expression::Call;
    use super::*;

    #[test]
    fn parse_empty_line() {
        let line = "";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Expression::Noop;
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_empty_with_only_one_spaces() {
        let line = " ";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Expression::Noop;
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_empty_with_only_spaces() {
        let line = " \t";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Expression::Noop;
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_no_args() {
        let line = "command";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Call(CallExpression {
            function: StringExpression {
                range: TextRange::from((0, 7)),
            },
            arguments: Vec::new(),
            range: TextRange::from((0, 7)),
        });
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_no_args_single_char() {
        let line = "l";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Call(CallExpression {
            function: StringExpression {
                range: TextRange::from((0, 1)),
            },
            arguments: Vec::new(),
            range: TextRange::from((0, 1)),
        });
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_no_args_padding_front() {
        let line = " pwd";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Call(CallExpression {
            function: StringExpression {
                range: TextRange::from((1, 4)),
            },
            arguments: Vec::new(),
            range: TextRange::from((1, 4)),
        });
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_no_args_padding_back() {
        let line = "cd  ";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Call(CallExpression {
            function: StringExpression {
                range: TextRange::from((0, 2)),
            },
            arguments: Vec::new(),
            range: TextRange::from((0, 2)),
        });
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_args_plain() {
        let line = "cd /path";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Call(CallExpression {
            function: StringExpression {
                range: TextRange::from((0, 2)),
            },
            arguments: vec![Argument::Plain(StringExpression {
                range: TextRange::from((3, 8)),
            })],
            range: TextRange::from((0, 8)),
        });
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_args_plain_single_char() {
        let line = "cd .";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Call(CallExpression {
            function: StringExpression {
                range: TextRange::from((0, 2)),
            },
            arguments: vec![Argument::Plain(StringExpression {
                range: TextRange::from((3, 4)),
            })],
            range: TextRange::from((0, 4)),
        });
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_args_plain_plain() {
        let line = "foo /path  other.*";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Call(CallExpression {
            function: StringExpression {
                range: TextRange::from((0, 3)),
            },
            arguments: vec![
                Argument::Plain(StringExpression {
                    range: TextRange::from((4, 9)),
                }),
                Argument::Plain(StringExpression {
                    range: TextRange::from((11, 18)),
                }),
            ],
            range: TextRange::from((0, 18)),
        });
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_args_short() {
        let line = "ls -l";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Call(CallExpression {
            function: StringExpression {
                range: TextRange::from((0, 2)),
            },
            arguments: vec![Argument::Short(StringExpression {
                range: TextRange::from((3, 5)),
            })],
            range: TextRange::from((0, 5)),
        });
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_args_long() {
        let line = "ls --list";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Call(CallExpression {
            function: StringExpression {
                range: TextRange::from((0, 2)),
            },
            arguments: vec![Argument::Long(StringExpression {
                range: TextRange::from((3, 9)),
            })],
            range: TextRange::from((0, 9)),
        });
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_args_many() {
        let line = " function\targ1 -l short --long=value   --other-long\t /more/stuff -x  ";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Call(CallExpression {
            function: StringExpression {
                range: TextRange::from((1, 9)), // function
            },
            arguments: vec![
                Argument::Plain(StringExpression {
                    range: TextRange::from((10, 14)), // arg1
                }),
                Argument::Short(StringExpression {
                    range: TextRange::from((15, 17)), // -l
                }),
                Argument::Plain(StringExpression {
                    range: TextRange::from((18, 23)), // short
                }),
                Argument::Long(StringExpression {
                    range: TextRange::from((24, 30)), // --long
                }),
                Argument::Plain(StringExpression {
                    range: TextRange::from((31, 36)), // value
                }),
                Argument::Long(StringExpression {
                    range: TextRange::from((39, 51)), // --other-long
                }),
                Argument::Plain(StringExpression {
                    range: TextRange::from((53, 64)), // /more/stuff
                }),
                Argument::Short(StringExpression {
                    range: TextRange::from((65, 67)), // -x
                }),
            ],
            range: TextRange::from((1, 67)),
        });
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_space_after_dash() {
        let line = "f - short --  long";
        let mut parser = Parser::new(line);
        let parsed = parser.parse();
        let expected = Call(CallExpression {
            function: StringExpression {
                range: TextRange::from((0, 1)),
            },
            arguments: vec![
                Argument::Short(StringExpression {
                    range: TextRange::from((2, 3)), // -
                }),
                Argument::Plain(StringExpression {
                    range: TextRange::from((4, 9)), // short
                }),
                Argument::Long(StringExpression {
                    range: TextRange::from((10, 12)), // --
                }),
                Argument::Plain(StringExpression {
                    range: TextRange::from((14, 18)), // long
                }),
            ],
            range: TextRange::from((0, 18)),
        });
        assert_eq!(parsed, expected);
    }
}

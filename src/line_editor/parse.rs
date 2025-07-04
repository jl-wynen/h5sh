#[derive(Clone, Debug)]
pub(super) struct ParseError {
    what: &'static str,
    pos: usize,
}

pub(super) type Result<T> = std::result::Result<T, ParseError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Line<'a> {
    expressions: Vec<Expression<'a>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Expression<'a> {
    Call(CallExpression<'a>),
    String(StringExpression<'a>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CallExpression<'a> {
    function: StringExpression<'a>,
    arguments: Vec<Expression<'a>>,
    pos: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StringExpression<'a> {
    str: &'a str,
    pos: usize,
}

pub(super) fn parse_line(input: &str) -> Result<Line> {
    let (trimmed, starting_pos) = trim_whitespace_start(input, 0);
    if trimmed.is_empty() {
        return Ok(Line {
            expressions: Vec::new(),
        });
    }

    let (call, _) = parse_call(trimmed, starting_pos)?;
    Ok(Line {
        expressions: vec![Expression::Call(call)],
    })
}

fn parse_call(input: &str, pos: usize) -> Result<(CallExpression, usize)> {
    let (trimmed, starting_pos) = trim_whitespace_start(input, pos);
    let (function, pos) = parse_string(trimmed, starting_pos)?;
    let call = CallExpression {
        function,
        arguments: Vec::new(),
        pos: starting_pos,
    };
    Ok((call, pos))
}

fn parse_string(input: &str, pos: usize) -> Result<(StringExpression, usize)> {
    Ok((StringExpression { str: input, pos }, pos + input.len()))
}

fn trim_whitespace_start(input: &str, pos: usize) -> (&str, usize) {
    let trimmed = input.trim_start();
    let pos = pos + input.len() - trimmed.len();
    (trimmed, pos)
}

#[cfg(test)]
mod tests {
    use super::Expression::Call;
    use super::*;

    #[test]
    fn parse_empty_line() {
        let line = "";
        let parsed = parse_line(line).unwrap();
        let expected = Line {
            expressions: Vec::new(),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_empty_with_only_spaces() {
        let line = " \t";
        let parsed = parse_line(line).unwrap();
        let expected = Line {
            expressions: Vec::new(),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_no_args() {
        let line = "command";
        let parsed = parse_line(line).unwrap();
        let expected = Line {
            expressions: vec![Call(CallExpression {
                function: StringExpression {
                    str: "command",
                    pos: 0,
                },
                arguments: Vec::new(),
                pos: 0,
            })],
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_no_args_single_char() {
        let line = "l";
        let parsed = parse_line(line).unwrap();
        let expected = Line {
            expressions: vec![Call(CallExpression {
                function: StringExpression { str: "l", pos: 0 },
                arguments: Vec::new(),
                pos: 0,
            })],
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_no_args_padding_front() {
        let line = " pwd";
        let parsed = parse_line(line).unwrap();
        let expected = Line {
            expressions: vec![Call(CallExpression {
                function: StringExpression { str: "pwd", pos: 1 },
                arguments: Vec::new(),
                pos: 1,
            })],
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_command_no_args_padding_back() {
        let line = "cd  ";
        let parsed = parse_line(line).unwrap();
        let expected = Line {
            expressions: vec![Call(CallExpression {
                function: StringExpression { str: "cd", pos: 0 },
                arguments: Vec::new(),
                pos: 0,
            })],
        };
        assert_eq!(parsed, expected);
    }
}

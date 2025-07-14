use std::collections::HashSet;

use super::super::parse::{Argument, Expression, StringExpression};
use super::super::{text_index::TextIndex, text_range::TextRange};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    /// Text to display when listing alternatives.
    pub display: String,
    /// Text to insert in line.
    pub replacement: String,
}

impl rustyline::completion::Candidate for Candidate {
    fn display(&self) -> &str {
        self.display.as_str()
    }

    fn replacement(&self) -> &str {
        self.replacement.as_str()
    }
}

impl Candidate {
    fn from_prefix(text: &str, prefix: &str) -> Option<Self> {
        if text.starts_with(prefix) {
            Some(Self {
                display: text.into(),
                replacement: text[prefix.len()..].into(),
            })
        } else {
            None
        }
    }
}

pub fn complete(
    expression: &Expression,
    line: &str,
    pos: usize,
    commands: &HashSet<String>,
) -> rustyline::Result<(usize, Vec<Candidate>)> {
    let pos = TextIndex::from(pos);
    let c = classify_location(expression, pos);
    Ok(match c {
        LocationType::Command(range) if pos == range.end() => {
            command_completions(&line[range], commands)
        }
        LocationType::Path(range) if pos == range.end() => path_completions(),
        _ => (0, vec![]),
    })
}

fn command_completions(input: &str, commands: &HashSet<String>) -> (usize, Vec<Candidate>) {
    let candidates = commands
        .iter()
        .filter_map(|cmd| Candidate::from_prefix(cmd, input))
        .collect();
    (input.len(), candidates)
}

fn path_completions() -> (usize, Vec<Candidate>) {
    todo!()
}

#[derive(Clone, Copy, Debug)]
enum LocationType {
    Path(TextRange),
    Command(TextRange),
    Other(TextRange),
}

impl LocationType {
    fn some_if_contains(self, pos: TextIndex) -> Option<Self> {
        if self.contains(pos) { Some(self) } else { None }
    }

    // For auto-completion, a range contains a position if the pos is at the end,
    // because that is when we generate completions.
    fn contains(&self, pos: TextIndex) -> bool {
        matches!(
            self,
            LocationType::Path(range) |
            LocationType::Command(range) |
            LocationType::Other(range)
            if range.contains_or_end(pos)
        )
    }
}

fn classify_location(expression: &Expression, pos: TextIndex) -> LocationType {
    classify_location_expression(expression, pos)
        .unwrap_or(LocationType::Other(TextRange::default()))
}

fn classify_location_expression(expression: &Expression, pos: TextIndex) -> Option<LocationType> {
    match expression {
        Expression::String(string) => {
            // assume that any string might be a path
            LocationType::Path(string.range).some_if_contains(pos)
        }
        Expression::Call(call) => {
            if !call.range.contains_or_end(pos) {
                return None; // avoid scanning children
            }
            if call.function.range.contains_or_end(pos) {
                Some(LocationType::Command(call.function.range))
            } else {
                call.arguments
                    .iter()
                    .find_map(|arg| classify_location_argument(arg, pos))
            }
        }
        Expression::Noop => None,
    }
}

fn classify_location_argument(argument: &Argument, pos: TextIndex) -> Option<LocationType> {
    match argument {
        Argument::Plain(string) => LocationType::Path(string.range).some_if_contains(pos),
        Argument::Short(string) | Argument::Long(string) => {
            LocationType::Other(string.range).some_if_contains(pos)
        }
    }
}

fn maybe_location_type(
    string: &StringExpression,
    pos: TextIndex,
    ty: LocationType,
) -> Option<LocationType> {
    if string.range.contains_or_end(pos) {
        Some(ty)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::parse::Parser;
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn complete_empty_input() {
        let line = "";
        let expression = Parser::new(line).parse();
        let commands = HashSet::new();

        let (insertion, completions) = complete(&expression, line, 0, &commands).unwrap();

        assert_eq!(insertion, 0);
        assert_eq!(completions, vec![]);
    }

    #[test]
    fn complete_command_no_args() {
        let line = "co";
        let expression = Parser::new(line).parse();
        let commands = HashSet::from(["command".into()]);

        let (insertion, completions) = complete(&expression, line, 2, &commands).unwrap();

        let expected = vec![Candidate {
            display: "command".into(),
            replacement: "mmand".into(),
        }];
        assert_eq!(insertion, 2);
        assert_eq!(completions, expected);
    }

    #[test]
    fn complete_command_no_args_not_at_end() {
        let line = "comm";
        let expression = Parser::new(line).parse();
        let commands = HashSet::from(["command".into()]);

        let (insertion, completions) = complete(&expression, line, 2, &commands).unwrap();

        assert_eq!(insertion, 0);
        assert_eq!(completions, vec![]);
    }
}

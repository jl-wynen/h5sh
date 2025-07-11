use super::super::parse::{Argument, Expression, StringExpression};
use super::super::{text_index::TextIndex, text_range::TextRange};

pub type Candidate = rustyline::completion::Pair;

pub fn complete(expression: &Expression, pos: usize) -> rustyline::Result<(usize, Vec<Candidate>)> {
    let pos = TextIndex::from(pos);
    Ok(match classify_location(expression, pos) {
        LocationType::Command(range) if pos == range.end() => command_completions(),
        LocationType::Path(range) if pos == range.end() => path_completions(),
        _ => (0, vec![]),
    })
    // Ok((
    //     0,
    //     vec![Candidate {
    //         display: "/entry".to_string(),
    //         replacement: "repl".into(),
    //     }],
    // ))
}

fn command_completions() -> (usize, Vec<Candidate>) {
    todo!()
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

    fn contains(&self, pos: TextIndex) -> bool {
        matches!(
            self,
            LocationType::Path(range) |
            LocationType::Command(range) |
            LocationType::Other(range)
            if range.contains(pos)
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
            if !call.range.contains(pos) {
                return None; // avoid scanning children
            }
            if call.function.range.contains(pos) {
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
    if string.range.contains(pos) {
        Some(ty)
    } else {
        None
    }
}

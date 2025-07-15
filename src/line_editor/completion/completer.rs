use std::collections::HashSet;

use super::super::parse::{Argument, Expression};
use super::super::{text_index::TextIndex, text_range::TextRange};
use crate::h5::{self, FileCache, H5Path};

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
        text.strip_prefix(prefix).map(|stripped| Self {
            display: text.into(),
            replacement: stripped.into(),
        })
    }
}

pub fn complete<CacheValue, Children, LoadChildren>(
    expression: &Expression,
    line: &str,
    pos: usize,
    commands: &HashSet<String>,
    file_cache: &mut FileCache<CacheValue>,
    working_group: &H5Path,
    load_children: LoadChildren,
) -> rustyline::Result<(usize, Vec<Candidate>)>
where
    LoadChildren: Fn(&H5Path) -> h5::Result<Children>,
    Children: IntoIterator<Item = (H5Path, CacheValue, bool)>,
{
    let pos = TextIndex::from(pos);
    let candidates = match classify_location(expression, pos) {
        LocationType::Path(range) if pos == range.end() => {
            path_completions(&line[range], file_cache, working_group, load_children)
        }
        LocationType::Command(range) if pos == range.end() => {
            command_completions(&line[range], commands)
        }
        _ => vec![],
    };
    Ok((pos.as_index(), candidates))
}

fn command_completions(input: &str, commands: &HashSet<String>) -> Vec<Candidate> {
    commands
        .iter()
        .filter_map(|cmd| Candidate::from_prefix(cmd, input))
        .collect()
}

fn path_completions<CacheValue, Children, LoadChildren>(
    input: &str,
    file_cache: &mut FileCache<CacheValue>,
    working_group: &H5Path,
    load_children: LoadChildren,
) -> Vec<Candidate>
where
    LoadChildren: Fn(&H5Path) -> h5::Result<Children>,
    Children: IntoIterator<Item = (H5Path, CacheValue, bool)>,
{
    use super::simple_completer::path_completions;

    let current = working_group.join(&H5Path::from(input));
    path_completions(file_cache, &current, load_children)
        .into_iter()
        .map(|path| Candidate {
            display: path.name().to_string(),
            replacement: path
                .as_raw()
                .strip_prefix(current.as_raw())
                .unwrap_or("")
                .to_string(),
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::super::super::parse::Parser;
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    fn failing_load_children(_path: &H5Path) -> h5::Result<Vec<(H5Path, i32, bool)>> {
        panic!("Do not load children!");
    }

    fn child_loader() -> impl Fn(&H5Path) -> h5::Result<Vec<(H5Path, i32, bool)>> {
        let entries = HashMap::from([(
            H5Path::from("/entry"),
            vec![(H5Path::from("/entry/path"), 10, false)],
        )]);
        move |path| match entries.get(&path.normalized()) {
            Some(children) => Ok(children.clone()),
            None => Err(h5::H5Error::NotFound(path.clone())),
        }
    }

    #[test]
    fn complete_empty_input() {
        let line = "";
        let expression = Parser::new(line).parse();
        let commands = HashSet::new();
        let mut cache = FileCache::new();
        let cwd = H5Path::root();

        let (insertion, completions) = complete(
            &expression,
            line,
            0,
            &commands,
            &mut cache,
            &cwd,
            failing_load_children,
        )
        .unwrap();

        assert_eq!(insertion, 0);
        assert_eq!(completions, vec![]);
    }

    #[test]
    fn complete_command_no_args() {
        let line = "co";
        let expression = Parser::new(line).parse();
        let commands = HashSet::from(["command".into()]);
        let mut cache = FileCache::new();
        let cwd = H5Path::root();

        let (insertion, completions) = complete(
            &expression,
            line,
            2,
            &commands,
            &mut cache,
            &cwd,
            failing_load_children,
        )
        .unwrap();

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
        let mut cache = FileCache::new();
        let cwd = H5Path::root();

        let (_, completions) = complete(
            &expression,
            line,
            2,
            &commands,
            &mut cache,
            &cwd,
            failing_load_children,
        )
        .unwrap();

        assert_eq!(completions, vec![]);
    }

    #[test]
    fn complete_command_one_arg() {
        let line = "co --flag";
        let expression = Parser::new(line).parse();
        let commands = HashSet::from(["command".into()]);
        let mut cache = FileCache::new();
        let cwd = H5Path::root();

        let (insertion, completions) = complete(
            &expression,
            line,
            2,
            &commands,
            &mut cache,
            &cwd,
            failing_load_children,
        )
        .unwrap();

        let expected = vec![Candidate {
            display: "command".into(),
            replacement: "mmand".into(),
        }];
        assert_eq!(insertion, 2);
        assert_eq!(completions, expected);
    }

    #[test]
    fn complete_path_single_arg_absolute_in_cwd() {
        let line = "ls /pa";
        let expression = Parser::new(line).parse();
        let commands = HashSet::new();
        let mut cache = FileCache::new();
        cache.insert_group(&H5Path::from("/"), -1);

        fn load_children(path: &H5Path) -> h5::Result<Vec<(H5Path, i32, bool)>> {
            let entries =
                HashMap::from([(H5Path::from("/"), vec![(H5Path::from("/path"), 1, false)])]);
            match entries.get(&path.normalized()) {
                Some(children) => Ok(children.clone()),
                None => Err(h5::H5Error::NotFound(path.clone())),
            }
        }

        let (insertion, completions) = complete(
            &expression,
            line,
            6,
            &commands,
            &mut cache,
            &H5Path::root(),
            load_children,
        )
        .unwrap();

        let expected = vec![Candidate {
            display: "path".into(),
            replacement: "th".into(),
        }];
        assert_eq!(insertion, 6);
        assert_eq!(completions, expected);
    }

    #[test]
    fn complete_path_single_arg_absolute_nested() {
        let line = "ls /entry/p";
        let expression = Parser::new(line).parse();
        let commands = HashSet::new();
        let mut cache = FileCache::new();
        let root = cache.insert_group(&H5Path::from("/"), -1);
        cache
            .insert_children(root, [(H5Path::from("/entry"), 2, true)])
            .unwrap();

        let (insertion, completions) = complete(
            &expression,
            line,
            11,
            &commands,
            &mut cache,
            &H5Path::root(),
            child_loader(),
        )
        .unwrap();

        let expected = vec![Candidate {
            display: "path".into(),
            replacement: "ath".into(),
        }];
        assert_eq!(insertion, 11);
        assert_eq!(completions, expected);
    }

    #[test]
    fn complete_path_single_arg_absolute_from_child() {
        let line = "ls /entry/p";
        let expression = Parser::new(line).parse();
        let commands = HashSet::new();
        let mut cache = FileCache::new();
        let root = cache.insert_group(&H5Path::from("/"), -1);
        cache
            .insert_children(root, [(H5Path::from("/entry"), 2, true)])
            .unwrap();

        let (insertion, completions) = complete(
            &expression,
            line,
            11,
            &commands,
            &mut cache,
            &H5Path::from("/entry"),
            child_loader(),
        )
        .unwrap();

        let expected = vec![Candidate {
            display: "path".into(),
            replacement: "ath".into(),
        }];
        assert_eq!(insertion, 11);
        assert_eq!(completions, expected);
    }

    #[test]
    fn complete_path_single_arg_relative_in_cwd() {
        let line = "ls p";
        let expression = Parser::new(line).parse();
        let commands = HashSet::new();
        let mut cache = FileCache::new();
        let root = cache.insert_group(&H5Path::from("/"), -1);
        cache
            .insert_children(root, [(H5Path::from("/entry"), 2, true)])
            .unwrap();

        let (insertion, completions) = complete(
            &expression,
            line,
            4,
            &commands,
            &mut cache,
            &H5Path::from("/entry"),
            child_loader(),
        )
        .unwrap();

        let expected = vec![Candidate {
            display: "path".into(),
            replacement: "ath".into(),
        }];
        assert_eq!(insertion, 4);
        assert_eq!(completions, expected);
    }

    #[test]
    fn complete_path_single_arg_relative_nested() {
        let line = "ls entry/p";
        let expression = Parser::new(line).parse();
        let commands = HashSet::new();
        let mut cache = FileCache::new();
        let root = cache.insert_group(&H5Path::from("/"), -1);
        cache
            .insert_children(root, [(H5Path::from("/entry"), 2, true)])
            .unwrap();

        let (insertion, completions) = complete(
            &expression,
            line,
            10,
            &commands,
            &mut cache,
            &H5Path::root(),
            child_loader(),
        )
        .unwrap();

        let expected = vec![Candidate {
            display: "path".into(),
            replacement: "ath".into(),
        }];
        assert_eq!(insertion, 10);
        assert_eq!(completions, expected);
    }
}

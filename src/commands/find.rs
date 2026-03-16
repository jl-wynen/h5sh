use crate::cmd::{CmdResult, Command, CommandError, CommandOutcome};
use crate::data::load_and_format_data;
use crate::h5::{H5File, H5Group, H5Object, H5Path};
use crate::output::{
    Printer,
    style::{DATASET_CHARACTER, GROUP_CHARACTER},
};
use crate::shell::Shell;
use bumpalo::{Bump, collections::String as BumpString};
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};
use crossterm::{
    ExecutableCommand, QueueableCommand,
    style::{Attribute, Print, ResetColor, SetAttribute},
};
use regex::{Match, Regex};
use std::io::{Write, stdout};
use std::ops::Deref;
use std::str::FromStr;

#[derive(Clone, Copy, Default)]
pub struct Find;

impl Command for Find {
    fn run(&self, args: ArgMatches, shell: &Shell, file: &H5File) -> CmdResult {
        let Ok(args) = Arguments::from_arg_matches(&args) else {
            return Err(CommandError::Critical("Failed to extract args".to_string()));
        };
        let absolute_target = shell.resolve_path(&args.target);
        match args.pattern {
            Pattern::Name(name) => {
                find_name(
                    file,
                    args.target,
                    absolute_target,
                    name,
                    !args.non_recursive,
                    shell.printer(),
                )?;
            }
            Pattern::Attr { name, value } => {
                find_attr(
                    file,
                    args.target,
                    absolute_target,
                    name,
                    value,
                    !args.non_recursive,
                    shell.printer(),
                )?;
            }
        }
        Ok(CommandOutcome::KeepRunning)
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

#[derive(Parser, Debug)]
#[command(
    name("find"),
    verbatim_doc_comment,
    after_help = "Examples:

Find all locations with 'monitor' in their name:
  find monitor

Find all locations that have an attr with 'tag' in its name:
  find @tag

Find all locations that have an attr with 'NX_class' in its name
and a value that matches 'NXmonitor':
  find @NX_class=NXmonitor

Find all locations that are named exactly 'sample':
  find ^sample$"
)]
/// Find datasets and groups.
///
/// The input is a regex, and the implementation searches for matches
/// in dataset, group, attribute names.
struct Arguments {
    /// Pattern to search for.
    pattern: Pattern,

    /// Search at this path.
    #[arg(default_value = ".")]
    target: H5Path,

    /// Do not search groups recursively.
    #[arg(short = 'R', long = "nr", default_value_t = false)]
    non_recursive: bool,
}

#[derive(Clone, Debug)]
enum Pattern {
    Name(Regex),
    Attr { name: Regex, value: Option<Regex> },
}

fn find_name(
    file: &H5File,
    target: H5Path,
    absolute_target: H5Path,
    pattern: Regex,
    recursive: bool,
    printer: &Printer,
) -> CmdResult {
    match file.load(&absolute_target)? {
        H5Object::Group(group) => {
            find_name_in_group(group, target, absolute_target, &pattern, recursive, printer)
        }
        H5Object::Dataset(_) => match_name_dataset(target, &pattern, printer),
        H5Object::Attribute(_) => Err(CommandError::Error("Is an attribute".to_string())),
    }
}

fn find_name_in_group(
    group: H5Group,
    target: H5Path,
    absolute_target: H5Path,
    pattern: &Regex,
    recursive: bool,
    printer: &Printer,
) -> CmdResult {
    let mut stdout = stdout();
    for child in group.load_children()?.into_iter() {
        let path = child.path().relative_to(&absolute_target);
        if let Some(mat) = pattern.find(path.as_raw()) {
            write_matched_path(
                &mut stdout,
                &target,
                &path,
                mat,
                child.location_type(),
                printer,
            )?;
        };
        if recursive && let H5Object::Group(child_group) = child {
            let child_path = child_group.path().clone();
            find_name_in_group(
                child_group,
                child_path,
                absolute_target.join(&path),
                pattern,
                recursive,
                printer,
            )?;
        }
    }
    stdout.flush()?;
    Ok(CommandOutcome::KeepRunning)
}

fn match_name_dataset(target: H5Path, pattern: &Regex, printer: &Printer) -> CmdResult {
    if let Some(mat) = pattern.find(target.as_raw()) {
        let mut stdout = stdout();
        write_matched_path(
            &mut stdout,
            &H5Path::from("."),
            &target,
            mat,
            hdf5::LocationType::Dataset,
            printer,
        )?;
        stdout.flush()?;
    };
    Ok(CommandOutcome::KeepRunning)
}

fn write_matched_path<'q, Q: QueueableCommand>(
    queue: &'q mut Q,
    target: &H5Path,
    path: &H5Path,
    mat: Match,
    location_type: hdf5::LocationType,
    printer: &Printer,
) -> std::io::Result<&'q mut Q> {
    if !target.is_current() {
        queue.queue(&printer.style().group)?.queue(Print(target))?;
        if !target.as_raw().ends_with('/') {
            queue.queue(Print('/'))?;
        }

        if location_type == hdf5::LocationType::Dataset {
            // Switch to the dataset style
            queue
                .queue(ResetColor)?
                .queue(SetAttribute(Attribute::Reset))?
                .queue(&printer.style().dataset)?;
        } // else: stick with the group style
    } else {
        match location_type {
            hdf5::LocationType::Dataset => {
                queue.queue(&printer.style().dataset)?;
            }
            hdf5::LocationType::Group => {
                queue.queue(&printer.style().group)?;
            }
            _ => {
                queue.queue(SetAttribute(Attribute::Reset))?;
            }
        }
    }

    queue_with_highlight_range(queue, path.as_raw(), mat.range())?;

    queue
        .queue(ResetColor)?
        .queue(SetAttribute(Attribute::Reset))?;
    let character = match location_type {
        hdf5::LocationType::Dataset => DATASET_CHARACTER,
        hdf5::LocationType::Group => GROUP_CHARACTER,
        _ => None,
    };
    if let Some(character) = character {
        queue.queue(Print(character))?;
    }

    queue.queue(Print("\n"))
}

fn find_attr(
    file: &H5File,
    target: H5Path,
    absolute_target: H5Path,
    key: Regex,
    value: Option<Regex>,
    recursive: bool,
    printer: &Printer,
) -> CmdResult {
    let mut stdout = stdout();
    match file.load(&absolute_target)? {
        H5Object::Group(group) => {
            find_attr_in_group(&mut stdout, group, target, &key, &value, recursive, printer)?;
        }
        H5Object::Dataset(dataset) => {
            find_attr_in_location(
                &mut stdout,
                dataset.underlying().deref(),
                dataset.path(),
                &key,
                &value,
                printer,
            )?;
        }
        H5Object::Attribute(_) => {
            return Err(CommandError::Error("Is an attribute".to_string()));
        }
    }
    stdout.flush()?;
    Ok(CommandOutcome::KeepRunning)
}

fn find_attr_in_group<Q: QueueableCommand>(
    queue: &mut Q,
    group: H5Group,
    target: H5Path,
    key: &Regex,
    value: &Option<Regex>,
    recursive: bool,
    printer: &Printer,
) -> CmdResult {
    find_attr_in_location(queue, group.underlying(), &target, key, value, printer)?;
    for child in group.load_children()? {
        match child {
            H5Object::Group(group) => {
                if recursive {
                    let child_path = target.join(group.path());
                    // dbg!("recurse", &group, &child_path, &target);
                    find_attr_in_group(queue, group, child_path, key, value, recursive, printer)?;
                } else {
                    // Search in the child group; would otherwise be handled by recursion.
                    find_attr_in_location(
                        queue,
                        group.underlying(),
                        group.path(),
                        key,
                        value,
                        printer,
                    )?;
                }
            }
            H5Object::Dataset(dataset) => {
                find_attr_in_location(
                    queue,
                    dataset.underlying().deref(),
                    &target.join(dataset.path()),
                    key,
                    value,
                    printer,
                )?;
            }
            H5Object::Attribute(_) => {
                return Err(CommandError::Error("Is an attribute".to_string()));
            }
        }
    }
    Ok(CommandOutcome::KeepRunning)
}

fn find_attr_in_location<Q: QueueableCommand, L>(
    queue: &mut Q,
    location: &L,
    target: &H5Path,
    key: &Regex,
    value: &Option<Regex>,
    printer: &Printer,
) -> CmdResult
where
    L: Deref<Target = hdf5::Location>,
{
    let mut buffer: Vec<u8> = Vec::new();
    for attr_name in location.attr_names()? {
        match_attr(&mut buffer, location, &attr_name, key, value, printer)?;
    }
    if !buffer.is_empty() {
        let bump = Bump::new();
        queue
            .queue(Print(printer.format_location_path(target, location, &bump)))?
            .queue(Print('\n'))?
            .queue(Print(String::from_utf8(buffer).unwrap_or_default()))?;
    }
    Ok(CommandOutcome::KeepRunning)
}

fn match_attr<E: ExecutableCommand + QueueableCommand>(
    buffer: &mut E,
    location: &hdf5::Location,
    attr_name: &str,
    key_pattern: &Regex,
    value_pattern: &Option<Regex>,
    printer: &Printer,
) -> CmdResult {
    let Some(key_match) = key_pattern.find(attr_name) else {
        return Ok(CommandOutcome::KeepRunning);
    };

    let bump = Bump::new();

    let value = load_and_format_data(&location.attr(attr_name)?, None, None, printer, &bump)
        .unwrap_or_else(|err| {
            use std::fmt::Write;
            let mut out = BumpString::new_in(&bump);
            let _ = write!(out, "Failed to load attribute: {err}");
            out
        });

    match value_pattern {
        Some(value_pattern) => {
            let Some(value_match) = value_pattern.find(&value) else {
                return Ok(CommandOutcome::KeepRunning);
            };
            write_attr_match(
                buffer,
                attr_name,
                &value,
                key_match,
                Some(value_match),
                printer,
            )?;
        }
        None => {
            write_attr_match(buffer, attr_name, &value, key_match, None, printer)?;
        }
    }
    Ok(CommandOutcome::KeepRunning)
}

fn write_attr_match<'e, Q: QueueableCommand>(
    queue: &'e mut Q,
    attr_name: &str,
    attr_value: &str,
    key_match: Match,
    value_match: Option<Match>,
    printer: &Printer,
) -> std::io::Result<&'e mut Q> {
    queue
        .queue(Print("  "))?
        .queue(&printer.style().attribute)?;
    queue_with_highlight_range(queue, attr_name, key_match.range())?;
    queue
        .queue(ResetColor)?
        .queue(Print(" = "))?
        .queue(SetAttribute(Attribute::Reset))?;

    if let Some(mat) = value_match {
        queue_with_highlight_range(queue, attr_value, mat.range())?;
    } else {
        queue.queue(Print(attr_value))?;
    }

    queue.queue(Print('\n'))
}

fn queue_with_highlight_range<'q, Q: QueueableCommand>(
    queue: &'q mut Q,
    string: &str,
    range: core::ops::Range<usize>,
) -> std::io::Result<&'q mut Q> {
    queue
        .queue(Print(&string[..range.start]))?
        .queue(SetAttribute(Attribute::Underlined))?
        .queue(Print(&string[range.clone()]))?
        .queue(SetAttribute(Attribute::NoUnderline))?
        .queue(Print(&string[range.end..]))
}

impl FromStr for Pattern {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix('@') {
            let mut parts = s.splitn(2, '=');
            let name = Regex::new(parts.next().ok_or_else(|| anyhow::anyhow!("Bad pattern"))?)?;

            if let Some(value) = parts.next() {
                Ok(Pattern::Attr {
                    name,
                    value: Some(Regex::new(value)?),
                })
            } else {
                Ok(Pattern::Attr { name, value: None })
            }
        } else {
            Ok(Pattern::Name(Regex::new(s)?))
        }
    }
}

#[cfg(test)]
mod tests {
    mod parser {
        use super::super::*;

        fn parse_args(args: &[&str]) -> Arguments {
            Arguments::from_arg_matches(
                &Find
                    .arg_parser()
                    .no_binary_name(true)
                    .try_get_matches_from(args)
                    .unwrap(),
            )
            .unwrap()
        }

        fn assert_pattern_name(pattern: &Pattern, expected: &str) {
            match pattern {
                Pattern::Name(name) => {
                    assert_eq!(name.as_str(), expected);
                }
                _ => {
                    panic!("expected a Name pattern, got Attr")
                }
            }
        }

        fn assert_pattern_attr_key(pattern: &Pattern, expected: &str) {
            match pattern {
                Pattern::Attr { name, value } => {
                    assert_eq!(name.as_str(), expected);
                    assert!(value.is_none());
                }
                _ => {
                    panic!("expected an Attr pattern, got Name")
                }
            }
        }

        fn assert_pattern_attr_key_value(
            pattern: &Pattern,
            expected_name: &str,
            expected_value: &str,
        ) {
            match pattern {
                Pattern::Attr { name, value } => {
                    assert_eq!(name.as_str(), expected_name);
                    assert_eq!(value.as_ref().unwrap().as_str(), expected_value);
                }
                _ => {
                    panic!("expected an Attr pattern, got Name")
                }
            }
        }

        #[test]
        fn parse_pattern_name() {
            let args = parse_args(&["some_name"]);
            assert_pattern_name(&args.pattern, "some_name");
            assert_eq!(args.target, H5Path::from("."));
        }

        #[test]
        fn parse_pattern_name_with_target() {
            let args = parse_args(&["needle", "farm/barn"]);
            assert_pattern_name(&args.pattern, "needle");
            assert_eq!(args.target, H5Path::from("farm/barn"));
        }

        #[test]
        fn parse_pattern_name_with_equal() {
            let args = parse_args(&["a=b"]);
            assert_pattern_name(&args.pattern, "a=b");
            assert_eq!(args.target, H5Path::from("."));
        }

        #[test]
        fn parse_pattern_name_with_trailing_equal() {
            let args = parse_args(&["name="]);
            assert_pattern_name(&args.pattern, "name=");
            assert_eq!(args.target, H5Path::from("."));
        }

        #[test]
        fn parse_pattern_attribute_name() {
            let args = parse_args(&["@key"]);
            assert_pattern_attr_key(&args.pattern, "key");
            assert_eq!(args.target, H5Path::from("."));
        }

        #[test]
        fn parse_pattern_attribute_with_value() {
            let args = parse_args(&["@abc=2hs"]);
            assert_pattern_attr_key_value(&args.pattern, "abc", "2hs");
            assert_eq!(args.target, H5Path::from("."));
        }

        #[test]
        fn parse_pattern_attribute_with_value_extra_equal() {
            let args = parse_args(&["@asd=qwe=zxc"]);
            assert_pattern_attr_key_value(&args.pattern, "asd", "qwe=zxc");
            assert_eq!(args.target, H5Path::from("."));
        }

        #[test]
        fn parse_pattern_attribute_with_value_trailing_equal() {
            let args = parse_args(&["@asd=qwe="]);
            assert_pattern_attr_key_value(&args.pattern, "asd", "qwe=");
            assert_eq!(args.target, H5Path::from("."));
        }

        #[test]
        fn parse_pattern_attribute_with_empty_value() {
            let args = parse_args(&["@abc="]);
            assert_pattern_attr_key_value(&args.pattern, "abc", "");
            assert_eq!(args.target, H5Path::from("."));
        }

        #[test]
        fn parse_pattern_attribute_without_key() {
            let args = parse_args(&["@=value"]);
            assert_pattern_attr_key_value(&args.pattern, "", "value");
            assert_eq!(args.target, H5Path::from("."));
        }

        #[test]
        fn parse_pattern_attribute_with_value_with_target() {
            let args = parse_args(&["@2_3=iU", "/entry/path/"]);
            assert_pattern_attr_key_value(&args.pattern, "2_3", "iU");
            assert_eq!(args.target, H5Path::from("/entry/path/"));
        }
    }

    mod printer {
        use super::super::*;
        use crossterm::execute;

        fn write_match(
            target: &H5Path,
            path: &H5Path,
            pattern: &str,
            location_type: hdf5::LocationType,
        ) -> String {
            let mat = Regex::new(pattern).unwrap().find(path.as_raw()).unwrap();

            let printer = Printer::new();
            let mut buffer: Vec<u8> = Vec::new();
            write_matched_path(&mut buffer, target, path, mat, location_type, &printer).unwrap();
            String::from_utf8(buffer).unwrap()
        }

        #[test]
        fn write_match_dataset_in_cwd() {
            let target = H5Path::from(".");
            let path = H5Path::from("foo");
            let res = write_match(&target, &path, "oo", hdf5::LocationType::Dataset);

            let mut buffer: Vec<u8> = Vec::new();
            execute!(
                buffer,
                Print("f"),
                SetAttribute(Attribute::Underlined),
                Print("oo"),
                SetAttribute(Attribute::NoUnderline),
                ResetColor,
                SetAttribute(Attribute::Reset),
                Print("\n")
            )
            .unwrap();
            let expected = String::from_utf8(buffer).unwrap();

            assert_eq!(res, expected);
        }

        #[test]
        fn write_match_group_in_cwd() {
            let target = H5Path::from(".");
            let path = H5Path::from("group");
            let res = write_match(&target, &path, "rou", hdf5::LocationType::Group);

            let mut buffer: Vec<u8> = Vec::new();
            execute!(
                buffer,
                &Printer::new().style().group,
                Print("g"),
                SetAttribute(Attribute::Underlined),
                Print("rou"),
                SetAttribute(Attribute::NoUnderline),
                Print("p"),
                // Some extra reset in case the dataset style is different from the default:
                ResetColor,
                SetAttribute(Attribute::Reset),
                Print("/\n")
            )
            .unwrap();
            let expected = String::from_utf8(buffer).unwrap();

            assert_eq!(res, expected);
        }

        #[test]
        fn write_match_dataset_in_subdir_full_match() {
            let target = H5Path::from("folder");
            let path = H5Path::from("foo");
            let res = write_match(&target, &path, "foo", hdf5::LocationType::Dataset);

            let mut buffer: Vec<u8> = Vec::new();
            execute!(
                buffer,
                &Printer::new().style().group,
                Print("folder/"),
                ResetColor,
                SetAttribute(Attribute::Reset),
                SetAttribute(Attribute::Underlined),
                Print("foo"),
                SetAttribute(Attribute::NoUnderline),
                ResetColor,
                SetAttribute(Attribute::Reset),
                Print("\n")
            )
            .unwrap();
            let expected = String::from_utf8(buffer).unwrap();

            assert_eq!(res, expected);
        }

        #[test]
        fn write_match_dataset_in_subdir_match_end() {
            let target = H5Path::from("folder");
            let path = H5Path::from("foo");
            let res = write_match(&target, &path, "oo", hdf5::LocationType::Dataset);

            let mut buffer: Vec<u8> = Vec::new();
            execute!(
                buffer,
                &Printer::new().style().group,
                Print("folder/"),
                ResetColor,
                SetAttribute(Attribute::Reset),
                Print("f"),
                SetAttribute(Attribute::Underlined),
                Print("oo"),
                SetAttribute(Attribute::NoUnderline),
                ResetColor,
                SetAttribute(Attribute::Reset),
                Print("\n")
            )
            .unwrap();
            let expected = String::from_utf8(buffer).unwrap();

            assert_eq!(res, expected);
        }

        #[test]
        fn write_match_group_in_subdir() {
            let target = H5Path::from("folder");
            let path = H5Path::from("group");
            let res = write_match(&target, &path, "rou", hdf5::LocationType::Group);

            let mut buffer: Vec<u8> = Vec::new();
            execute!(
                buffer,
                &Printer::new().style().group,
                Print("folder/g"),
                SetAttribute(Attribute::Underlined),
                Print("rou"),
                SetAttribute(Attribute::NoUnderline),
                Print("p"),
                ResetColor,
                SetAttribute(Attribute::Reset),
                Print("/\n")
            )
            .unwrap();
            let expected = String::from_utf8(buffer).unwrap();

            assert_eq!(res, expected);
        }
    }
}

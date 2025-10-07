use crate::cmd::{CmdResult, Command, CommandError, CommandOutcome};
use crate::h5::{H5Dataset, H5File, H5Group, H5Object, H5Path};
use crate::output::Printer;
use crate::shell::Shell;
use bumpalo::{Bump, collections::String as BumpString};
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};
use crossterm::style::{Attribute, SetAttribute};
use crossterm::{
    QueueableCommand,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io::{Write, stdout};
use std::str::FromStr;

#[derive(Clone, Copy, Default)]
pub struct Find;

impl Command for Find {
    fn run(&self, args: ArgMatches, shell: &Shell, file: &H5File) -> CmdResult {
        let Ok(args) = Arguments::from_arg_matches(&args) else {
            return Err(CommandError::Critical("Failed to extract args".to_string()));
        };
        let target = shell.resolve_path(&args.target);
        find(file, target, args.pattern, shell.printer())?;
        Ok(CommandOutcome::KeepRunning)
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

/// Find datasets and groups.
///
/// Examples:
///
/// Find all locations with 'monitor' in their name:
///   find monitor
///
/// Find all locations that have an attr with 'tag' in its name:
///   find @tag
///
/// Find all locations that have an attr with 'NX_class' in its name
/// and a value that matches 'NXmonitor':
///   find @NX_class=NXmonitor
#[derive(Parser, Debug)]
#[command(name("find"))]
struct Arguments {
    /// Pattern to search for.
    pattern: Pattern,

    /// Search at this path.
    #[arg(default_value = ".")]
    target: H5Path,

    /// Search groups recursively.
    #[arg(short = 'r', long, default_value_t = false)]
    recursive: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Pattern {
    Name(String),
    Attr { name: String, value: Option<String> },
}

fn find(file: &H5File, target: H5Path, pattern: Pattern, printer: &Printer) -> CmdResult {
    match file.load(&target)? {
        H5Object::Group(group) => find_in_group(group, &pattern, printer),
        H5Object::Dataset(dataset) => match_dataset(dataset, &pattern, printer),
        H5Object::Attribute(_) => Err(CommandError::Error("Is an attribute".to_string())),
    }
}

fn find_in_group(group: H5Group, pattern: &Pattern, printer: &Printer) -> CmdResult {
    let bump = Bump::new();
    let mut stdout = stdout();
    for (path, info) in group.load_child_locations()?.into_iter() {
        let name = path.name();
        if !pattern.matches(name) {
            continue;
        }
        stdout
            .queue(SetAttribute(Attribute::Underlined))?
            .queue(Print(format_name_in(name, &info, printer, &bump)))?
            .queue(SetAttribute(Attribute::Reset))?
            .queue(Print("\n"))?;
    }
    stdout.flush()?;
    Ok(CommandOutcome::KeepRunning)
}

fn match_dataset(dataset: H5Dataset, pattern: &Pattern, printer: &Printer) -> CmdResult {
    let name = dataset.path().name();
    if pattern.matches(name) {
        let bump = Bump::new();
        printer.println(printer.apply_style_dataset_in(name, &bump));
    }
    Ok(CommandOutcome::KeepRunning)
}

fn format_name_in<'alloc>(
    name: &str,
    location_info: &hdf5::LocationInfo,
    printer: &Printer,
    bump: &'alloc Bump,
) -> BumpString<'alloc> {
    // TODO show symbol
    // TODO highlight match
    match location_info.loc_type {
        hdf5::LocationType::Dataset => printer.apply_style_dataset_in(name, bump),
        hdf5::LocationType::Group => printer.apply_style_group_in(name, bump),
        _ => BumpString::from_str_in(name, bump), // should never happen
    }
}

impl Pattern {
    fn matches(&self, text: &str) -> bool {
        match self {
            Pattern::Name(name) => text.contains(name),
            Pattern::Attr { name, value } => {
                todo!("attr matching")
            }
        }
    }
}

impl FromStr for Pattern {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix('@') {
            let mut parts = s.splitn(2, '=');
            let name = parts
                .next()
                .ok_or_else(|| "Bad pattern".to_string())?
                .to_string();

            if let Some(value) = parts.next() {
                Ok(Pattern::Attr {
                    name,
                    value: Some(value.to_string()),
                })
            } else {
                Ok(Pattern::Attr { name, value: None })
            }
        } else {
            Ok(Pattern::Name(s.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn parse_pattern_name() {
        let args = parse_args(&["some_name"]);
        assert_eq!(args.pattern, Pattern::Name("some_name".to_string()));
        assert_eq!(args.target, H5Path::from("."));
    }

    #[test]
    fn parse_pattern_name_with_target() {
        let args = parse_args(&["needle", "farm/barn"]);
        assert_eq!(args.pattern, Pattern::Name("needle".to_string()));
        assert_eq!(args.target, H5Path::from("farm/barn"));
    }

    #[test]
    fn parse_pattern_name_with_equal() {
        let args = parse_args(&["a=b"]);
        assert_eq!(args.pattern, Pattern::Name("a=b".to_string()));
        assert_eq!(args.target, H5Path::from("."));
    }

    #[test]
    fn parse_pattern_name_with_trailing_equal() {
        let args = parse_args(&["name="]);
        assert_eq!(args.pattern, Pattern::Name("name=".to_string()));
        assert_eq!(args.target, H5Path::from("."));
    }

    #[test]
    fn parse_pattern_attribute_name() {
        let args = parse_args(&["@key"]);
        assert_eq!(
            args.pattern,
            Pattern::Attr {
                name: "key".to_string(),
                value: None
            }
        );
        assert_eq!(args.target, H5Path::from("."));
    }

    #[test]
    fn parse_pattern_attribute_with_value() {
        let args = parse_args(&["@abc=2hs"]);
        assert_eq!(
            args.pattern,
            Pattern::Attr {
                name: "abc".to_string(),
                value: Some("2hs".to_string())
            }
        );
        assert_eq!(args.target, H5Path::from("."));
    }

    #[test]
    fn parse_pattern_attribute_with_value_extra_equal() {
        let args = parse_args(&["@asd=qwe=zxc"]);
        assert_eq!(
            args.pattern,
            Pattern::Attr {
                name: "asd".to_string(),
                value: Some("qwe=zxc".to_string())
            }
        );
        assert_eq!(args.target, H5Path::from("."));
    }

    #[test]
    fn parse_pattern_attribute_with_value_trailing_equal() {
        let args = parse_args(&["@asd=qwe="]);
        assert_eq!(
            args.pattern,
            Pattern::Attr {
                name: "asd".to_string(),
                value: Some("qwe=".to_string())
            }
        );
        assert_eq!(args.target, H5Path::from("."));
    }

    #[test]
    fn parse_pattern_attribute_with_empty_value() {
        let args = parse_args(&["@abc="]);
        assert_eq!(
            args.pattern,
            Pattern::Attr {
                name: "abc".to_string(),
                value: Some("".to_string())
            }
        );
        assert_eq!(args.target, H5Path::from("."));
    }

    #[test]
    fn parse_pattern_attribute_without_key() {
        let args = parse_args(&["@=value"]);
        assert_eq!(
            args.pattern,
            Pattern::Attr {
                name: "".to_string(),
                value: Some("value".to_string())
            }
        );
        assert_eq!(args.target, H5Path::from("."));
    }

    #[test]
    fn parse_pattern_attribute_with_value_with_target() {
        let args = parse_args(&["@2_3=iU", "/entry/path/"]);
        assert_eq!(
            args.pattern,
            Pattern::Attr {
                name: "2_3".to_string(),
                value: Some("iU".to_string())
            }
        );
        assert_eq!(args.target, H5Path::from("/entry/path/"));
    }
}

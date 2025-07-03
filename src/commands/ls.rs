use chrono::{DateTime, Utc};
use clap::{ArgGroup, ArgMatches, CommandFactory, FromArgMatches, Parser};
use crossterm::{
    QueueableCommand,
    cursor::MoveRight,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io::{Write, stdout};

use crate::cmd::{CmdResult, Command, CommandError};
use crate::h5::{H5Dataset, H5File, H5Group, H5Object, H5Path};
use crate::output::Printer;
use crate::shell::Shell;

#[derive(Clone, Copy, Default)]
pub struct Ls;

impl Command for Ls {
    fn run(&self, args: ArgMatches, shell: &mut Shell, file: &mut H5File) -> CmdResult {
        let Ok(args) = Arguments::from_arg_matches(&args) else {
            return Err(CommandError::Critical("Failed to extract args".to_string()));
        };
        let options = Options::from_args(&args);
        let target = shell.resolve_path(&args.path);

        match file.load(&target)? {
            H5Object::Group(group) => {
                print_objects(file.load_children(group)?, shell.printer(), options);
            }
            dataset @ H5Object::Dataset(_) => {
                print_objects(std::iter::once(dataset), shell.printer(), options);
            }
        }
        Ok(())
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

/// List group contents.
#[derive(Parser, Debug)]
#[command(name("ls"))]
#[clap(group(ArgGroup::new("sort").args(&["name", "ty"])))]
struct Arguments {
    /// List children of this path.
    #[arg(default_value = ".")]
    path: H5Path,

    /// Show object metadata in a table.
    #[arg(short = 'l', default_value_t = false)]
    long: bool,

    /// Sort by name (default).
    #[arg(long)]
    name: bool,

    /// Sort by object type.
    #[arg(short = 't', long = "type")]
    ty: bool,
}

struct Options {
    long: bool,
    sort_by: SortBy,
}

enum SortBy {
    Name,
    Type,
}

impl Options {
    fn from_args(args: &Arguments) -> Self {
        let sort_by = if args.ty {
            SortBy::Type
        } else {
            // This case must be at the end, we sort by name if the flag is
            // explicitly provided or if no flag is provided.
            SortBy::Name
        };

        Options {
            long: args.long,
            sort_by,
        }
    }
}

fn print_objects<It: Iterator<Item = H5Object>>(objects: It, printer: &Printer, options: Options) {
    let objects_vec: Vec<_> = objects.collect();
    let mut objects = objects_vec
        .iter()
        .map(|obj| (obj.path().name(), obj))
        .collect();
    sort_objects(&mut objects, options.sort_by);
    if options.long {
        // let _ = print_entry_list(entries, printer);
    } else {
        print_object_grid(objects, printer)
    }
}

fn print_object_grid(objects: Vec<(&str, &H5Object)>, printer: &Printer) {
    printer.print_grid(
        objects
            .into_iter()
            .map(|(name, object)| format_object_name(name, object, printer))
            .collect(),
    );
}

fn format_object_name(name: &str, object: &H5Object, printer: &Printer) -> String {
    match object {
        H5Object::Dataset(_) => printer.apply_style_dataset(name),
        H5Object::Group(_) => {
            let mut formatted = printer.apply_style_group(name);
            formatted.push('/');
            formatted
        }
    }
}

fn sort_objects(objects: &mut Vec<(&str, &H5Object)>, sort_by: SortBy) {
    // By name first to get sorted subgroups where other sorts are ambiguous.
    sort_objects_by_name(objects);
    match sort_by {
        SortBy::Name => { /* already sorted above */ }
        SortBy::Type => {
            sort_objects_by_type(objects);
        }
    }
}

fn sort_objects_by_name(objects: &mut Vec<(&str, &H5Object)>) {
    objects.sort_by_key(|(name, _)| *name);
}

fn sort_objects_by_type(objects: &mut Vec<(&str, &H5Object)>) {
    objects.sort_by(|(_, a), (_, b)| match (a, b) {
        (H5Object::Dataset(_), H5Object::Group(_)) => std::cmp::Ordering::Greater,
        (H5Object::Group(_), H5Object::Dataset(_)) => std::cmp::Ordering::Less,
        _ => std::cmp::Ordering::Equal,
    });
}

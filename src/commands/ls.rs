use bumpalo::Bump;
use clap::{ArgGroup, ArgMatches, CommandFactory, FromArgMatches, Parser};
use std::io::stdout;

use crate::cmd::{CmdResult, Command, CommandError, CommandOutcome};
use crate::h5::{H5File, H5Object, H5Path};
use crate::output::Printer;
use crate::shell::Shell;

#[derive(Clone, Copy, Default)]
pub struct Ls;

impl Command for Ls {
    fn run(&self, args: ArgMatches, shell: &Shell, file: &H5File) -> CmdResult {
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
            H5Object::Attribute(_) => {
                return Err(CommandError::Error("Is an attribute".to_string()));
            }
        }
        Ok(CommandOutcome::KeepRunning)
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

    /// Do not show the content of datasets.
    #[arg(short = 'c', long)]
    no_content: bool,

    /// Sort by object type.
    #[arg(short = 't', long = "type")]
    ty: bool,
}

struct Options {
    long: bool,
    sort_by: SortBy,
    show_content: bool,
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
            show_content: !args.no_content,
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
        let _ = print_object_table(objects, printer, options.show_content);
    } else {
        print_object_grid(objects, printer)
    }
}

fn print_object_grid(objects: Vec<(&str, &H5Object)>, printer: &Printer) {
    let bump = Bump::new();
    printer.print_grid(
        objects
            .into_iter()
            .map(|(name, object)| printer.format_object_name(name, object, &bump))
            .collect(),
    );
}

fn print_object_table(
    objects: Vec<(&str, &H5Object)>,
    printer: &Printer,
    show_content: bool,
) -> std::io::Result<()> {
    printer
        .queue_object_table(&mut stdout(), objects, show_content)
        .map(|_| ())
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

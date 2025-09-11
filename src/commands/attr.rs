use crate::cmd::{CmdResult, Command, CommandError, CommandOutcome};
use crate::data::load_and_format_data;
use crate::h5;
use crate::h5::{H5Attribute, H5File, H5Object, H5Path};
use crate::output::Printer;
use crate::shell::Shell;
use bumpalo::{Bump, collections::String as BumpString};
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};
use crossterm::{
    QueueableCommand,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io;
use std::io::stdout;

#[derive(Clone, Copy, Default)]
pub struct Attr;

impl Command for Attr {
    fn run(&self, args: ArgMatches, shell: &Shell, file: &H5File) -> CmdResult {
        let Ok(args) = Arguments::from_arg_matches(&args) else {
            return Err(CommandError::Critical("Failed to extract args".to_string()));
        };
        let parent_object = load_parent_object(&args.path, shell, file)?;
        let attr_names = collect_attributes(&parent_object, args.attr)?;
        show_attrs(parent_object, attr_names, shell.printer())?;
        Ok(CommandOutcome::KeepRunning)
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

/// Print attributes.
#[derive(Parser, Debug)]
#[command(name("a"))]
struct Arguments {
    /// Path of a dataset or group.
    path: Option<H5Path>,

    /// Name of attributes to print.
    attr: Option<Vec<String>>,
}

// TODO try to use the table logic in ls
fn show_attrs(
    parent_object: H5Object,
    attr_names: Vec<String>,
    printer: &Printer,
) -> io::Result<()> {
    let bump = Bump::new();
    let name_column_width = attr_names.iter().map(|name| name.len()).max().unwrap_or(0);
    let max_value_width = printer
        .terminal_size()
        .0
        .saturating_sub(name_column_width as u16 - 2) as usize;

    let mut stdout = stdout();
    for name in attr_names {
        queue_attr_name(&mut stdout, &name)?;
        // To account for ": " after attr names, we ignore that string for both the
        // name column and the individual name lengths.
        printer.queue_padding(&mut stdout, name_column_width.saturating_sub(name.len()))?;
        load_and_queue_attr_data(
            &mut stdout,
            &parent_object,
            &name,
            max_value_width,
            printer,
            &bump,
        )?;
        stdout.queue(Print('\n'))?;
    }
    Ok(())
}

fn queue_attr_name<'q, Q: QueueableCommand>(queue: &'q mut Q, name: &str) -> io::Result<&'q mut Q> {
    queue
        .queue(SetForegroundColor(Color::DarkCyan))?
        .queue(Print(name))?
        .queue(ResetColor)?
        .queue(Print(": "))
}

fn load_and_queue_attr_data<'q, Q: QueueableCommand>(
    queue: &'q mut Q,
    parent_object: &H5Object,
    attr_name: &str,
    max_width: usize,
    printer: &Printer,
    bump: &Bump,
) -> io::Result<&'q mut Q> {
    match load_and_format_attr_data(parent_object, attr_name, max_width, printer, bump) {
        Ok(formatted) => queue.queue(Print(&formatted)),
        Err(err) => queue_error(queue, &err.to_string()),
    }
}

fn load_and_format_attr_data<'alloc>(
    parent_object: &H5Object,
    attr_name: &str,
    max_width: usize,
    printer: &Printer,
    bump: &'alloc Bump,
) -> h5::Result<BumpString<'alloc>> {
    let attr = get_attr(parent_object, attr_name)?;
    load_and_format_data(&attr, None, Some(max_width), printer, bump)
}

fn queue_error<'q, Q: QueueableCommand>(queue: &'q mut Q, message: &str) -> io::Result<&'q mut Q> {
    queue
        .queue(SetForegroundColor(Color::Red))?
        .queue(Print("Error: "))?
        .queue(Print(message))?
        .queue(ResetColor)
}

fn get_attr(parent_object: &H5Object, attr_name: &str) -> h5::Result<H5Attribute> {
    match parent_object {
        H5Object::Group(group) => group.attr(attr_name),
        H5Object::Dataset(dataset) => dataset.attr(attr_name),
        H5Object::Attribute(_) => Err(h5::H5Error::Other(
            "Attributes do not have attributes".into(),
        )),
    }
}

fn load_parent_object(path: &Option<H5Path>, shell: &Shell, file: &H5File) -> h5::Result<H5Object> {
    if let Some(path) = path.as_ref() {
        file.load(&shell.resolve_path(path))
    } else {
        file.load(shell.get_working_group())
    }
}

fn collect_attributes(
    parent_object: &H5Object,
    attrs: Option<Vec<String>>,
) -> h5::Result<Vec<String>> {
    if let Some(attrs) = attrs {
        Ok(attrs)
    } else {
        match parent_object {
            H5Object::Group(group) => group.attr_names(),
            H5Object::Dataset(dataset) => dataset.attr_names(),
            H5Object::Attribute(_) => Err(h5::H5Error::Other(
                "Attributes do not have attributes".into(),
            )),
        }
    }
}

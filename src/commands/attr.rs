use crate::cmd::{CmdResult, Command, CommandError, CommandOutcome};
use crate::h5;
use crate::h5::{H5File, H5Object, H5Path};
use crate::output::Printer;
use crate::shell::Shell;
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};
use std::io::{self, stdout};

#[derive(Clone, Copy, Default)]
pub struct Attr;

impl Command for Attr {
    fn run(&self, args: ArgMatches, shell: &Shell, file: &H5File) -> CmdResult {
        let Ok(args) = Arguments::from_arg_matches(&args) else {
            return Err(CommandError::Critical("Failed to extract args".to_string()));
        };
        let parent_object = load_parent_object(&args.path, shell, file)?;
        let attr_names = collect_attributes(&parent_object, args.attr)?;
        let attrs = load_attributes(&parent_object, attr_names.as_ref())?;
        show_attrs(attr_names, attrs, shell.printer())?;
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

fn show_attrs(attr_names: Vec<String>, attrs: Vec<H5Object>, printer: &Printer) -> io::Result<()> {
    let objects: Vec<_> = attr_names
        .iter()
        .zip(attrs.iter())
        .map(|(name, attr)| (name.as_str(), attr))
        .collect();
    printer
        .queue_object_table(&mut stdout(), &objects, true)
        .map(|_| ())
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

fn load_attributes(parent_object: &H5Object, attr_names: &[String]) -> h5::Result<Vec<H5Object>> {
    let mut attrs = Vec::with_capacity(attr_names.len());
    match parent_object {
        H5Object::Group(group) => {
            for name in attr_names {
                attrs.push(group.attr(name)?.into());
            }
        }
        H5Object::Dataset(dataset) => {
            for name in attr_names {
                attrs.push(dataset.attr(name)?.into());
            }
        }
        H5Object::Attribute(_) => {
            return Err(h5::H5Error::Other(
                "Attributes do not have attributes".into(),
            ));
        }
    }
    Ok(attrs)
}

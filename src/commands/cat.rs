use bumpalo::Bump;
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};

use crate::cmd::{CmdResult, Command, CommandError, CommandOutcome};
use crate::data::load_and_format_data;
use crate::h5::{H5Dataset, H5File, H5Object, H5Path};
use crate::output::Printer;
use crate::shell::Shell;

#[derive(Clone, Copy, Default)]
pub struct Cat;

impl Command for Cat {
    fn run(&self, args: ArgMatches, shell: &Shell, file: &H5File) -> CmdResult {
        let Ok(args) = Arguments::from_arg_matches(&args) else {
            return Err(CommandError::Critical("Failed to extract args".to_string()));
        };
        let full_path = shell.resolve_path(&args.path);
        match file.load(&full_path) {
            Ok(object) => match object {
                H5Object::Group(_) => Err(CommandError::Error(format!("Is a group: {full_path}"))),
                H5Object::Dataset(dataset) => cat_dataset(dataset, shell.printer()),
            },
            Err(err) => Err(err.into()),
        }
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

/// Print the contents of a dataset.
#[derive(Parser, Debug)]
#[command(name("cat"))]
struct Arguments {
    /// Path of a dataset.
    path: H5Path,
}

fn cat_dataset(dataset: H5Dataset, printer: &Printer) -> CmdResult {
    let bump = Bump::new();
    let formatted = load_and_format_data(&dataset, None, None, printer, &bump)?;
    println!("{formatted}");
    Ok(CommandOutcome::KeepRunning)
}

use bumpalo::Bump;
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};
use hdf5::{
    Dimension,
    types::{TypeDescriptor, VarLenUnicode},
};

use crate::cmd::{CmdResult, Command, CommandError, CommandOutcome};
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
    match dataset.type_descriptor() {
        Err(err) => Err(CommandError::Error(format!("Unable to read dtype: {err}"))),
        Ok(TypeDescriptor::VarLenUnicode) => cat_var_len_unicode(dataset),
        Ok(descriptor) => {
            let bump = Bump::new();
            Err(CommandError::Error(format!(
                "dtype not supported: {}",
                printer.format_dtype(&descriptor, &bump)
            )))
        }
    }
}

fn cat_var_len_unicode(dataset: H5Dataset) -> CmdResult {
    let shape = dataset.underlying().shape();
    if shape.ndim() != 0 {
        return Err(CommandError::Error(
            "Can only cat 0-dimensional datasets".to_string(),
        ));
    }
    let content = dataset.underlying().read_scalar::<VarLenUnicode>()?;
    println!("{content}");
    Ok(CommandOutcome::KeepRunning)
}

use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};

use crate::cmd::{CmdResult, Command, CommandError};
use crate::h5::{H5File, H5Object, H5Path};
use crate::shell::Shell;

#[derive(Clone, Copy, Default)]
pub struct Cd;

impl Command for Cd {
    fn run(&self, args: ArgMatches, shell: &mut Shell, file: &H5File) -> CmdResult {
        let Ok(args) = Arguments::from_arg_matches(&args) else {
            return Err(CommandError::Critical("Failed to extract args".to_string()));
        };
        let full_path = shell.resolve_path(&args.path);
        match file.load(&full_path) {
            Ok(object) => match object {
                H5Object::Group(_) => {
                    shell.set_working_dir(full_path);
                    Ok(())
                }
                _ => Err(CommandError::Error(format!("Not a group: {full_path}"))),
            },
            Err(err) => Err(err.into()),
        }
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

/// Change group.
#[derive(Parser, Debug)]
#[command(name("cd"))]
struct Arguments {
    /// Path to change into.
    path: H5Path,
}

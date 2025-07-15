use clap::{ArgMatches, CommandFactory, Parser};

use crate::cmd::{CmdResult, Command, CommandOutcome};
use crate::h5::H5File;
use crate::shell::Shell;

#[derive(Clone, Copy, Default)]
pub struct Exit;

impl Command for Exit {
    fn run(&self, _: ArgMatches, _: &Shell, _: &H5File) -> CmdResult {
        Ok(CommandOutcome::ExitSuccess)
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

/// Exit h5sh.
#[derive(Parser, Debug)]
#[command(name("exit"))]
struct Arguments {}

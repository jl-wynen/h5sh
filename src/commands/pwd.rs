use clap::{ArgMatches, CommandFactory, Parser};

use crate::cmd::{CmdResult, Command};
use crate::h5::H5File;
use crate::shell::Shell;

#[derive(Clone, Copy, Default)]
pub struct Pwd;

impl Command for Pwd {
    fn run(&self, _args: ArgMatches, shell: &mut Shell, _file: &mut H5File) -> CmdResult {
        shell
            .printer()
            .println(format!("{}", shell.get_working_dir()));
        Ok(())
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

/// Print working directory (group).
#[derive(Parser, Debug)]
#[command(name("pwd"))]
struct Arguments {}

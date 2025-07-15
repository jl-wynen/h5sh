use indexmap::IndexMap;
use std::fmt::Display;
use std::rc::Rc;

use crate::commands;
use crate::h5::{H5Error, H5File, H5Path};
use crate::shell::Shell;

pub trait Command {
    fn run(&self, args: clap::ArgMatches, shell: &Shell, file: &H5File) -> CmdResult;

    fn arg_parser(&self) -> clap::Command;
}

pub type CommandMap = IndexMap<String, Rc<dyn Command>>;

pub fn commands() -> CommandMap {
    let mut cmds = CommandMap::new();
    cmds.insert("cd".to_string(), Rc::new(commands::Cd));
    cmds.insert("exit".to_string(), Rc::new(commands::Exit));
    cmds.insert("help".to_string(), Rc::new(commands::Help));
    cmds.insert("ls".to_string(), Rc::new(commands::Ls));
    cmds.insert("pwd".to_string(), Rc::new(commands::Pwd));
    cmds
}

#[derive(Clone, Debug)]
pub enum CommandOutcome {
    /// Keep the shell running and process the next command.
    KeepRunning,
    /// Change the working group.
    ChangeWorkingGroup(H5Path),
    /// Exit the shell after a failure without processing further commands.
    ExitFailure,
    /// Exit the shell without processing further commands.
    ExitSuccess,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandError {
    /// The command failed.
    Error(String),
    /// The command failed and printed its own error message.
    NoMessage,
    /// The command failed and recovery is not reliably possible.
    Critical(String),
}

pub type CmdResult = Result<CommandOutcome, CommandError>;

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::Error(msg) => f.write_str(msg),
            CommandError::NoMessage => Ok(()),
            CommandError::Critical(msg) => f.write_str(msg),
        }
    }
}

impl From<H5Error> for CommandError {
    fn from(err: H5Error) -> Self {
        CommandError::Error(err.to_string())
    }
}

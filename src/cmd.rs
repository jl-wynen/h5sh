// use crate::commands::{Cd, Exit, Help, Ls, Pwd};
use crate::h5::{H5Error, H5File};
use crate::shell::Shell;
use indexmap::IndexMap;
use std::fmt::Display;
use std::rc::Rc;

pub trait Command {
    fn run(&self, args: clap::ArgMatches, shell: &mut Shell, file: &mut H5File) -> CmdResult;

    fn arg_parser(&self) -> clap::Command;
}

pub type CommandMap = IndexMap<String, Rc<dyn Command>>;

pub fn commands() -> CommandMap {
    let mut cmds = CommandMap::new();
    // cmds.insert("cd".to_string(), Rc::new(Cd));
    // cmds.insert("exit".to_string(), Rc::new(Exit));
    // cmds.insert("help".to_string(), Rc::new(Help));
    // cmds.insert("ls".to_string(), Rc::new(Ls));
    // cmds.insert("pwd".to_string(), Rc::new(Pwd));
    cmds
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CmdError {
    /// The command failed.
    Error(String),
    /// The command failed and printed its own error message.
    NoMessage,
    /// The command failed and recovery is not reliably possible.
    Critical(String),
    /// Not really an error, the command wants the shell to exit.
    Exit,
}

pub type CmdResult = Result<(), CmdError>;

impl Display for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmdError::Error(msg) => f.write_str(msg),
            CmdError::NoMessage => Ok(()),
            CmdError::Critical(msg) => f.write_str(msg),
            CmdError::Exit => Ok(()),
        }
    }
}

impl From<H5Error> for CmdError {
    fn from(err: H5Error) -> Self {
        CmdError::Error(err.to_string())
    }
}

use clap::{ArgMatches, CommandFactory, Parser, builder::StyledStr};
use crossterm::{
    QueueableCommand,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io::{Write, stdout};

use crate::cmd::{CmdResult, Command, CommandOutcome};
use crate::h5::H5File;
use crate::shell::Shell;

#[derive(Clone, Copy, Default)]
pub struct Help;

impl Command for Help {
    fn run(&self, _args: ArgMatches, shell: &Shell, _file: &H5File) -> CmdResult {
        let mut descriptions: Vec<_> = shell
            .commands()
            .iter_base_commands()
            .map(|(name, cmd)| {
                (
                    name,
                    cmd.arg_parser()
                        .get_about()
                        .unwrap_or_else(Default::default)
                        .clone(),
                )
            })
            .collect();
        descriptions.sort_by_key(|(name, _)| *name);
        let name_length = descriptions
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(0);
        let _ = print_commands(descriptions, name_length);
        Ok(CommandOutcome::KeepRunning)
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

/// Print available commands.
#[derive(Parser, Debug)]
#[command(name("help"))]
struct Arguments {}

fn print_commands(
    descriptions: Vec<(&String, StyledStr)>,
    name_length: usize,
) -> std::io::Result<()> {
    let mut stdout = stdout();
    for (name, description) in descriptions {
        print_command_help(&mut stdout, name, description, name_length)?;
    }
    stdout.flush()?;
    Ok(())
}

fn print_command_help<Q: QueueableCommand>(
    queue: &mut Q,
    name: &str,
    description: StyledStr,
    name_length: usize,
) -> std::io::Result<()> {
    queue
        .queue(SetForegroundColor(Color::Blue))?
        .queue(Print(format!("{name:name_length$}")))?
        .queue(ResetColor)?
        .queue(Print("  "))?
        .queue(Print(description))?
        .queue(Print("\n"))?;
    Ok(())
}

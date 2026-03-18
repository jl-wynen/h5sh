use crate::cmd::{CmdResult, Command, CommandOutcome};
use crate::h5::H5File;
use crate::shell::Shell;

use crate::output::Style;
use clap::{ArgMatches, CommandFactory, Parser};
use crossterm::{QueueableCommand, style::Print};
use std::io::{Write, stdout};

#[derive(Clone, Copy, Default)]
pub struct Help;

impl Command for Help {
    fn run(&self, _args: ArgMatches, shell: &Shell, _file: &H5File) -> CmdResult {
        let _ = print_help(shell);
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

fn print_help(shell: &Shell) -> std::io::Result<()> {
    let mut stdout = stdout();

    stdout.queue(Print("Commands:\n"))?;
    let base_commands = collect_base_commands(shell);
    print_table(&mut stdout, base_commands, shell.printer().style())?;

    stdout.queue(Print("Aliases:\n"))?;
    let aliases = collect_aliases(shell);
    print_table(&mut stdout, aliases, shell.printer().style())?;

    stdout.flush()
}

fn collect_base_commands(shell: &Shell) -> Vec<(&str, String)> {
    shell
        .commands()
        .iter_base_commands()
        .map(|(name, cmd)| {
            (
                name,
                cmd.arg_parser()
                    .get_about()
                    .map_or_else(Default::default, ToString::to_string),
            )
        })
        .collect()
}

fn collect_aliases(shell: &Shell) -> Vec<(&str, &str)> {
    shell.commands().iter_aliases().collect()
}

fn print_table<Q: QueueableCommand, D: std::fmt::Display>(
    queue: &mut Q,
    mut rows: Vec<(&str, D)>,
    style: &Style,
) -> std::io::Result<()> {
    rows.sort_by_key(|(name, _)| *name);
    let key_length = rows.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    for (key, description) in rows {
        print_row(queue, key, description, key_length, style)?;
    }
    Ok(())
}

fn print_row<Q: QueueableCommand, D: std::fmt::Display>(
    queue: &mut Q,
    key: &str,
    description: D,
    key_length: usize,
    style: &Style,
) -> std::io::Result<()> {
    queue
        .queue(&style.editor.command)?
        .queue(Print(format!("  {key:key_length$}")))?
        .queue(style.reset())?
        .queue(Print(format!("  {description}\n")))?;
    Ok(())
}

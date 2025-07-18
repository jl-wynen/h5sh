use clap::{Arg, Args, CommandFactory, Parser};
use crossterm::{
    queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use std::path::PathBuf;
use std::process::exit;

/// Interactive shell for HDF5 files.
#[derive(Parser, Debug)]
#[command(version, about, long_about)]
#[command(propagate_version = true)]
struct CliArguments {
    #[command(flatten)]
    open: Option<CliOpenArgs>,

    #[command(subcommand)]
    command: Option<CliCommands>,

    /// Enable extra output.
    #[arg(short, long)]
    verbose: bool,
}

#[derive(clap::Subcommand, Debug)]
enum CliCommands {
    /// (Default) Open a HDF5 file.
    Open(CliOpenArgs),
}

#[derive(Args, Debug)]
struct CliOpenArgs {
    /// HDF5 file to open.
    pub path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct Arguments {
    pub command: Commands,
    pub verbose: bool,
}

#[derive(Debug)]
pub enum Commands {
    Open(OpenArgs),
}

#[derive(Debug)]
pub struct OpenArgs {
    pub path: PathBuf,
}

impl Arguments {
    pub fn parse() -> Self {
        let raw = CliArguments::parse();
        normalize_arguments(raw)
    }
}

fn normalize_arguments(args: CliArguments) -> Arguments {
    match (args.open, args.command) {
        (Some(_), Some(_)) => {
            usage_error("Either specify a path directly or use 'open' but not both.");
        }
        (None, None) => {
            usage_error("Specify a path to open or a sub command.");
        }
        (Some(open_args), None) => Arguments {
            command: normalize_command(CliCommands::Open(open_args)),
            verbose: args.verbose,
        },
        (None, Some(commands)) => Arguments {
            command: normalize_command(commands),
            verbose: args.verbose,
        },
    }
}

fn normalize_command(command: CliCommands) -> Commands {
    match command {
        CliCommands::Open(open_args) => Commands::Open(normalize_open_args(open_args)),
    }
}

fn normalize_open_args(open_args: CliOpenArgs) -> OpenArgs {
    let Some(path) = open_args.path else {
        usage_error("Specify a path to open.");
    };
    OpenArgs { path }
}

fn usage_error(message: &str) -> ! {
    let _ = queue!(
        std::io::stdout(),
        SetForegroundColor(Color::DarkRed),
        Print("error: "),
        ResetColor,
        Print(message),
        Print("\n\n"),
        Print(CliArguments::command().render_usage()),
        Print("\n\nFor more information try '"),
        SetAttribute(Attribute::Bold),
        Print("--help"),
        SetAttribute(Attribute::NoBold),
        Print("'\n"),
    );
    exit(2);
}

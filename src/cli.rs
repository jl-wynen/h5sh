use crate::h5::Locking;
use clap::{Args, CommandFactory, Parser, ValueEnum};
use crossterm::{queue, style::Print};
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
    /// Manage the h5sh executable.
    #[command(name = "self")]
    Self_(CliSelfArgs),
}

#[derive(Args, Debug)]
struct CliOpenArgs {
    /// HDF5 file to open.
    pub path: Option<PathBuf>,

    /// Execute a command in h5sh and exit.
    #[arg(short, long)]
    pub command: Option<String>,

    /// Control color output.
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,

    /// Enable / disable HDF5 file locking.
    #[arg(long, value_enum, default_value_t = LockingChoice::Auto)]
    pub locking: LockingChoice,
}

#[derive(Args, Debug)]
struct CliSelfArgs {
    #[command(subcommand)]
    command: SelfCommand,
}

#[derive(clap::Subcommand, Debug)]
pub enum SelfCommand {
    /// Update h5py.
    Update,
    /// Uninstall h5py.
    Uninstall,
}

#[derive(Debug)]
pub struct Arguments {
    pub command: Commands,
    pub verbose: bool,
}

#[derive(Debug)]
pub enum Commands {
    Open(OpenArgs),
    Self_(SelfArgs),
}

#[derive(Debug)]
pub struct OpenArgs {
    pub path: PathBuf,
    pub command: Option<String>,
    pub color: bool,
    pub locking: Locking,
}

#[derive(Debug)]
pub struct SelfArgs {
    pub command: SelfCommand,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum LockingChoice {
    Auto,
    True,
    False,
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
        CliCommands::Self_(self_args) => Commands::Self_(SelfArgs {
            command: self_args.command,
        }),
    }
}

fn normalize_open_args(open_args: CliOpenArgs) -> OpenArgs {
    let Some(path) = open_args.path else {
        usage_error("Specify a path to open.");
    };
    OpenArgs {
        path,
        command: open_args.command,
        color: normalize_color_choice(open_args.color),
        locking: normalize_locking_choice(open_args.locking),
    }
}

fn normalize_color_choice(arg: ColorChoice) -> bool {
    match arg {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => !std::env::var("NO_COLOR").is_ok_and(|value| !value.is_empty()),
    }
}

fn normalize_locking_choice(arg: LockingChoice) -> Locking {
    match arg {
        LockingChoice::Auto => Locking::Auto,
        LockingChoice::True => Locking::True,
        LockingChoice::False => Locking::False,
    }
}

fn usage_error(message: &str) -> ! {
    let _ = queue!(
        std::io::stdout(),
        Print("error: "),
        Print(message),
        Print("\n\n"),
        Print(CliArguments::command().render_usage()),
        Print("\n\nFor more information try '--help'\n"),
    );
    exit(2);
}

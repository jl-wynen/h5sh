mod cmd;
mod commands;
mod h5;
mod line_editor;
mod output;
mod shell;

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

use cmd::CommandOutcome;
use line_editor::Poll;

fn main() -> ExitCode {
    let args = Arguments::parse();
    // TODO setup logging
    let mut shell = shell::Shell::new();
    let h5file = match h5::H5File::open(args.path.clone()) {
        Ok(h5file) => h5file,
        Err(err) => {
            shell
                .printer()
                .print_shell_error(format!("Failed to open file: {err}"));
            return ExitCode::FAILURE;
        }
    };

    let Ok(mut editor) = shell.start_editor() else {
        shell.printer().print_shell_error("Failed to start editor");
        return ExitCode::FAILURE;
    };
    let mut exit_code = ExitCode::SUCCESS;
    loop {
        match editor.poll() {
            Poll::Cmd(input) => match shell.parse_and_execute_input(&input, &h5file) {
                CommandOutcome::ExitFailure => {
                    exit_code = ExitCode::FAILURE;
                    break;
                }
                CommandOutcome::ExitSuccess => {
                    exit_code = ExitCode::SUCCESS;
                    break;
                }
                CommandOutcome::KeepRunning => {}
            },
            Poll::Skip => {}
            Poll::Exit => break,
            Poll::Error(err) => {
                println!("ERROR {err}");
            }
        }
    }

    editor.save_history().unwrap();
    exit_code
}

/// Interactive shell for HDF5 files.
#[derive(Parser, Debug)]
#[command(version, about, long_about)]
struct Arguments {
    /// HDF5 file to open.
    path: PathBuf,

    /// Enable extra output.
    #[arg(short, long)]
    verbose: bool,
}

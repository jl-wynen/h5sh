mod cli;
mod cmd;
mod commands;
mod h5;
mod line_editor;
mod output;
mod prompt;
mod shell;

use log::{LevelFilter, error};
use simple_logger::SimpleLogger;
use std::process::ExitCode;

use cmd::CommandOutcome;
use line_editor::Poll;

fn main() -> ExitCode {
    let args = cli::Arguments::parse();
    configure_logging(args.verbose);
    match args.command {
        cli::Commands::Open(args) => open_file(args),
        cli::Commands::Self_(args) => self_command(args),
    }
}

fn open_file(args: cli::OpenArgs) -> ExitCode {
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

    let Ok(mut editor) = shell.start_editor(&h5file) else {
        shell.printer().print_shell_error("Failed to start editor");
        return ExitCode::FAILURE;
    };
    let mut exit_code = ExitCode::SUCCESS;
    loop {
        match editor.poll(&shell, &h5file) {
            Poll::Cmd(input) => match shell.parse_and_execute_input(&input, &h5file) {
                CommandOutcome::KeepRunning => {}
                CommandOutcome::ChangeWorkingGroup(new_working_group) => {
                    shell.set_working_group(new_working_group.clone());
                    editor.set_working_group(new_working_group);
                }
                CommandOutcome::ExitFailure => {
                    exit_code = ExitCode::FAILURE;
                    break;
                }
                CommandOutcome::ExitSuccess => {
                    exit_code = ExitCode::SUCCESS;
                    break;
                }
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

fn self_command(args: cli::SelfArgs) -> ExitCode {
    match args.command {
        cli::SelfCommand::Update => update_self(),
        cli::SelfCommand::Uninstall => uninstall_self(),
    }
}

fn update_self() -> ExitCode {
    match run_self_update() {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            println!();
            error!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn run_self_update() -> anyhow::Result<()> {
    use anyhow::bail;

    let result = self_update::backends::github::Update::configure()
        .repo_owner("jl-wynen")
        .repo_name("h5sh")
        .bin_name("h5sh")
        .show_download_progress(true)
        .current_version(self_update::cargo_crate_version!())
        .build()?
        .update();
    match result {
        Ok(status) => {
            println!("{status}");
            Ok(())
        }
        Err(err) => match err {
            self_update::errors::Error::Update(msg) if msg.contains("aborted") => Ok(()),
            _ => bail!(err.to_string()),
        },
    }
}

fn uninstall_self() -> ExitCode {
    match self_replace::self_delete() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            error!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn configure_logging(verbose: bool) {
    SimpleLogger::new()
        .with_level(if verbose {
            LevelFilter::Info
        } else {
            LevelFilter::Warn
        })
        .init()
        .unwrap();
}

mod cli;
mod cmd;
mod commands;
mod data;
mod h5;
mod line_editor;
mod output;
mod prompt;
mod shell;

use cmd::CommandOutcome;
use line_editor::Poll;
use log::{LevelFilter, error};
use ndarray::Ix0;
use simple_logger::SimpleLogger;
use std::process::ExitCode;

fn make_file() {
    use core::str::FromStr;
    use ndarray::{Array, Array0, Array1};
    use ndarray::{Ix1, Ix2};
    let file = hdf5::File::create("asd.h5").unwrap();

    let scalars_group = file.create_group("scalars").unwrap();
    let data = hdf5::types::VarLenAscii::from_ascii("asd".as_bytes()).unwrap();
    scalars_group
        .new_dataset_builder()
        .with_data(&Array0::from_elem((), data))
        .create("ascii")
        .unwrap();
    let data = hdf5::types::VarLenUnicode::from_str("asd").unwrap();
    scalars_group
        .new_dataset_builder()
        .with_data(&Array0::from_elem((), data))
        .create("utf-8")
        .unwrap();
    let data = hdf5::types::FixedAscii::<3>::from_ascii("asd".as_bytes()).unwrap();
    scalars_group
        .new_dataset_builder()
        .with_data(&Array0::from_elem((), data))
        .create("ascii(3)")
        .unwrap();
    let data = hdf5::types::FixedUnicode::<4>::from_str("åsd").unwrap();
    scalars_group
        .new_dataset_builder()
        .with_data(&Array0::from_elem((), data))
        .create("utf-8(4)")
        .unwrap();
    scalars_group
        .new_dataset_builder()
        .with_data(&Array::<i32, Ix0>::from_elem((), 2))
        .create("i32")
        .unwrap();

    let d1_group = file.create_group("1d").unwrap();
    d1_group
        .new_dataset_builder()
        .with_data(&Array::<i8, Ix1>::from(vec![1, 2]))
        .create("i8")
        .unwrap();
    d1_group
        .new_dataset_builder()
        .with_data(&Array::<i16, Ix1>::from(vec![1, 2]))
        .create("i16")
        .unwrap();
    d1_group
        .new_dataset_builder()
        .with_data(&Array::<i32, Ix1>::from(vec![1, 2]))
        .create("i32")
        .unwrap();
    d1_group
        .new_dataset_builder()
        .with_data(&Array::<i64, Ix1>::from(vec![1, 2]))
        .create("i64")
        .unwrap();
    d1_group
        .new_dataset_builder()
        .with_data(&Array::<f32, Ix1>::from(vec![1.2, 5.4, 7.9]))
        .create("f32")
        .unwrap();

    let d2_group = file.create_group("2d").unwrap();
    d2_group
        .new_dataset_builder()
        .with_data(&Array::<i32, Ix2>::from(vec![[-1, 2], [3, 4]]))
        .create("i32")
        .unwrap();
    d2_group
        .new_dataset_builder()
        .with_data(&Array::<u64, Ix2>::from(vec![[1, 2], [3, 4]]))
        .create("u64")
        .unwrap();
}

fn main() -> ExitCode {
    // make_file();
    // return ExitCode::SUCCESS;

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

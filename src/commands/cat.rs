use bumpalo::Bump;
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};
use hdf5::{
    H5Type,
    types::{
        FixedAscii, FixedUnicode, FloatSize, IntSize, TypeDescriptor, VarLenAscii, VarLenUnicode,
    },
};
use ndarray::IxDyn;
use std::fmt::Display;

use crate::cmd::{CmdResult, Command, CommandError, CommandOutcome};
use crate::h5::{H5Dataset, H5File, H5Object, H5Path};
use crate::output::Printer;
use crate::shell::Shell;

#[derive(Clone, Copy, Default)]
pub struct Cat;

impl Command for Cat {
    fn run(&self, args: ArgMatches, shell: &Shell, file: &H5File) -> CmdResult {
        let Ok(args) = Arguments::from_arg_matches(&args) else {
            return Err(CommandError::Critical("Failed to extract args".to_string()));
        };
        let full_path = shell.resolve_path(&args.path);
        match file.load(&full_path) {
            Ok(object) => match object {
                H5Object::Group(_) => Err(CommandError::Error(format!("Is a group: {full_path}"))),
                H5Object::Dataset(dataset) => cat_dataset(dataset, shell.printer()),
            },
            Err(err) => Err(err.into()),
        }
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

/// Print the contents of a dataset.
#[derive(Parser, Debug)]
#[command(name("cat"))]
struct Arguments {
    /// Path of a dataset.
    path: H5Path,
}

fn cat_dataset(dataset: H5Dataset, printer: &Printer) -> CmdResult {
    match dataset.type_descriptor() {
        Err(err) => Err(CommandError::Error(format!("Unable to read dtype: {err}"))),
        Ok(TypeDescriptor::VarLenUnicode) => cat_var_len_unicode(dataset),
        Ok(TypeDescriptor::VarLenAscii) => cat_var_len_ascii(dataset),
        Ok(TypeDescriptor::FixedAscii(n)) => cat_fixed_len_ascii(dataset, n),
        Ok(TypeDescriptor::FixedUnicode(n)) => cat_fixed_len_unicode(dataset, n),
        Ok(TypeDescriptor::Float(float_size)) => cat_float(dataset, float_size),
        Ok(TypeDescriptor::Integer(int_size)) => cat_signed_integer(dataset, int_size),
        Ok(TypeDescriptor::Unsigned(int_size)) => cat_unsigned_integer(dataset, int_size),
        Ok(TypeDescriptor::Boolean) => cat_bool(dataset),
        Ok(descriptor) => {
            let bump = Bump::new();
            Err(CommandError::Error(format!(
                "dtype not supported: {}",
                printer.format_dtype(&descriptor, &bump)
            )))
        }
    }
}

fn cat_var_len_unicode(dataset: H5Dataset) -> CmdResult {
    load_and_print::<VarLenUnicode>(dataset)
}

fn cat_var_len_ascii(dataset: H5Dataset) -> CmdResult {
    load_and_print::<VarLenAscii>(dataset)
}

fn cat_fixed_len_ascii(dataset: H5Dataset, n: usize) -> CmdResult {
    const MAX_N: usize = 32;
    if n > MAX_N {
        return Err(CommandError::Error(format!(
            "Can only read fixed-length strings of up to {MAX_N} bytes"
        )));
    }
    load_and_print::<FixedAscii<MAX_N>>(dataset)
}

fn cat_fixed_len_unicode(dataset: H5Dataset, n: usize) -> CmdResult {
    const MAX_N: usize = 32;
    if n > MAX_N {
        return Err(CommandError::Error(format!(
            "Can only read fixed-length strings of up to {MAX_N} bytes"
        )));
    }
    load_and_print::<FixedUnicode<MAX_N>>(dataset)
}

fn cat_float(dataset: H5Dataset, float_size: FloatSize) -> CmdResult {
    match float_size {
        FloatSize::U8 => load_and_print::<f64>(dataset),
        FloatSize::U4 => load_and_print::<f32>(dataset),
        // f16 is unstable, so approximate using f32
        FloatSize::U2 => load_and_print::<f32>(dataset),
    }
}

fn cat_signed_integer(dataset: H5Dataset, int_size: IntSize) -> CmdResult {
    match int_size {
        IntSize::U8 => load_and_print::<i64>(dataset),
        IntSize::U4 => load_and_print::<i32>(dataset),
        IntSize::U2 => load_and_print::<i16>(dataset),
        IntSize::U1 => load_and_print::<i8>(dataset),
    }
}

fn cat_unsigned_integer(dataset: H5Dataset, int_size: IntSize) -> CmdResult {
    match int_size {
        IntSize::U8 => load_and_print::<u64>(dataset),
        IntSize::U4 => load_and_print::<u32>(dataset),
        IntSize::U2 => load_and_print::<u16>(dataset),
        IntSize::U1 => load_and_print::<u8>(dataset),
    }
}

fn cat_bool(dataset: H5Dataset) -> CmdResult {
    load_and_print::<bool>(dataset)
}

fn load_and_print<T: H5Type + Display>(dataset: H5Dataset) -> CmdResult {
    match dataset.underlying().read::<T, IxDyn>() {
        Ok(content) => {
            println!("{content}");
            Ok(CommandOutcome::KeepRunning)
        }
        Err(err) => Err(CommandError::Error(err.to_string())),
    }
}

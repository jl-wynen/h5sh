use crate::cmd::{CmdResult, Command, CommandError, CommandOutcome};
use crate::h5;
use crate::h5::{H5Attribute, H5Dataset, H5File, H5Group, H5Object, H5Path};
use crate::output::Printer;
use crate::shell::Shell;
use bumpalo::{
    Bump,
    collections::{String as BumpString, Vec as BumpVec},
};
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};
use crossterm::{ExecutableCommand, style::Print};
use hdf5::types::TypeDescriptor;
use std::ops::Deref;

#[derive(Clone, Copy, Default)]
pub struct Inspect;

impl Command for Inspect {
    fn run(&self, args: ArgMatches, shell: &Shell, file: &H5File) -> CmdResult {
        let Ok(args) = Arguments::from_arg_matches(&args) else {
            return Err(CommandError::Critical("Failed to extract args".to_string()));
        };
        let full_path = shell.resolve_path(&args.path);
        match file.load(&full_path) {
            Ok(object) => match object {
                H5Object::Group(group) => inspect_group(group, shell.printer()),
                H5Object::Dataset(dataset) => inspect_dataset(dataset, shell.printer()),
                H5Object::Attribute(attribute) => inspect_attr(attribute, shell.printer()),
            },
            Err(err) => Err(err.into()),
        }
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

/// Print an overview of an object.
#[derive(Parser, Debug)]
#[command(name("inspect"))]
struct Arguments {
    /// Path of a dataset or group.
    path: H5Path,
}

fn inspect_group(group: H5Group, printer: &Printer) -> CmdResult {
    let bump = Bump::new();
    printer.println("Group");
    Ok(CommandOutcome::KeepRunning)
}

fn inspect_dataset(dataset: H5Dataset, printer: &Printer) -> CmdResult {
    let bump = Bump::new();

    let mut buffer = BumpVec::<u8>::new_in(&bump);
    buffer
        .execute(&printer.style().emphasis)?
        .execute(Print("Dataset"))?
        .execute(printer.style().reset())?
        .execute(Print("      "))?;
    buffer.append(&mut load_and_inspect_data(&dataset, printer, &bump)?);
    printer.println(BumpString::from_utf8_lossy_in(&buffer, &bump));
    Ok(CommandOutcome::KeepRunning)
}

fn inspect_attr(attr: H5Attribute, printer: &Printer) -> CmdResult {
    let bump = Bump::new();
    let mut buffer = BumpVec::<u8>::new_in(&bump);
    buffer
        .execute(&printer.style().emphasis)?
        .execute(Print("Attribute"))?
        .execute(printer.style().reset())?
        .execute(Print("    "))?;
    Ok(CommandOutcome::KeepRunning)
}

pub fn load_and_inspect_data<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError> {
    match container.dtype()?.to_descriptor()? {
        TypeDescriptor::VarLenUnicode => inspect::var_len_unicode(container, printer, bump),
        // TypeDescriptor::VarLenAscii => crate::data::load_and_format::var_len_ascii(
        //     container, max_elem, max_width, printer, bump,
        // ),
        // TypeDescriptor::FixedUnicode(n) => crate::data::load_and_format::fixed_len_unicode(
        //     container, n, max_elem, max_width, printer, bump,
        // ),
        // TypeDescriptor::FixedAscii(n) => crate::data::load_and_format::fixed_len_ascii(
        //     container, n, max_elem, max_width, printer, bump,
        // ),
        // TypeDescriptor::Float(float_size) => crate::data::load_and_format::float(
        //     container, float_size, max_elem, max_width, printer, bump,
        // ),
        // TypeDescriptor::Integer(int_size) => crate::data::load_and_format::signed_integer(
        //     container, int_size, max_elem, max_width, printer, bump,
        // ),
        // TypeDescriptor::Unsigned(int_size) => crate::data::load_and_format::unsigned_integer(
        //     container, int_size, max_elem, max_width, printer, bump,
        // ),
        // TypeDescriptor::Boolean => {
        //     crate::data::load_and_format::bool(container, max_elem, max_width, printer, bump)
        // }
        descriptor => Err(CommandError::Error(format!(
            "dtype not supported: {}",
            printer.format_dtype(&descriptor, bump)
        ))),
    }
}

mod inspect {
    use super::*;
    use bumpalo::format;
    use hdf5::H5Type;
    use hdf5::types::{FixedAscii, FixedUnicode, FloatSize, IntSize, VarLenAscii, VarLenUnicode};
    use ndarray::{IxDyn, s};
    use std::fmt::{Display, Write};

    pub(super) fn var_len_unicode<'alloc>(
        container: &impl Deref<Target = hdf5::Container>,
        printer: &Printer,
        bump: &'alloc Bump,
    ) -> Result<BumpVec<'alloc, u8>, CommandError> {
        string::<VarLenUnicode>(container, printer, bump)
    }

    fn string<'alloc, T: H5Type + Display>(
        container: &impl Deref<Target = hdf5::Container>,
        printer: &Printer,
        bump: &'alloc Bump,
    ) -> Result<BumpVec<'alloc, u8>, CommandError> {
        let content = container.read::<T, IxDyn>()?;

        let mut buffer = BumpVec::<u8>::new_in(bump);
        print_label(&mut buffer, "DType", printer)?
            .execute(Print(printer.format_dtype(&T::type_descriptor(), bump)))?
            .execute(Print("\n"))?;
        print_label(&mut buffer, "Shape", printer)?
            .execute(Print(format!(
                in bump,
                "{:?}",
                content.shape()
            )))?
            .execute(Print("  "))?;
        print_label(&mut buffer, "N elements", printer)?.execute(Print(format!(
            in bump,
            "{}",
            content.len()
        )))?;

        Ok(buffer)
    }

    fn print_label<'e, E: ExecutableCommand>(
        e: &'e mut E,
        label: &str,
        printer: &Printer,
    ) -> std::io::Result<&'e mut E> {
        e.execute(&printer.style().size)?
            .execute(Print(label))?
            .execute(&printer.style().reset())?
            .execute(Print(": "))
    }
}

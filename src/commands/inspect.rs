use crate::cmd::{CmdResult, Command, CommandError, CommandOutcome};
use crate::h5::{H5Attribute, H5Dataset, H5File, H5Group, H5Object, ObjectPath};
use crate::output::{
    Printer,
    style::{ATTRIBUTE_CHARACTER, DATASET_CHARACTER, GROUP_CHARACTER},
};
use crate::shell::Shell;
use bumpalo::{
    Bump,
    collections::{String as BumpString, Vec as BumpVec},
    format,
};
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};
use crossterm::{ExecutableCommand, style::Print};
use hdf5::{
    H5Type,
    types::{
        FixedAscii, FixedUnicode, FloatSize, IntSize, TypeDescriptor, VarLenAscii, VarLenUnicode,
    },
};
use ndarray::IxDyn;
use std::fmt::Display;
use std::ops::Deref;

#[derive(Clone, Copy, Default)]
pub struct Inspect;

impl Command for Inspect {
    fn run(&self, args: ArgMatches, shell: &Shell, file: &H5File) -> CmdResult {
        let Ok(args) = Arguments::from_arg_matches(&args) else {
            return Err(CommandError::Critical("Failed to extract args".to_string()));
        };
        let full_path = ObjectPath {
            location_path: shell.resolve_path(&args.path.location_path),
            attr_name: args.path.attr_name,
        };
        match file.load_object(&full_path) {
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
    /// Path of a dataset, group, or attribute.
    path: ObjectPath,
}

fn inspect_group(group: H5Group, printer: &Printer) -> CmdResult {
    let locations = match group.load_child_locations() {
        Ok(loc) => loc,
        Err(err) => {
            return Err(CommandError::Error(std::format!(
                "Failed to load children of {}: {err}",
                group.path()
            )));
        }
    };

    let (n_group, n_ds, n_dtype, n_map) = locations.iter().fold(
        (0, 0, 0, 0),
        |(n_group, n_ds, n_dtype, n_map), (_, loc)| match loc.loc_type {
            hdf5::LocationType::Group => (n_group + 1, n_ds, n_dtype, n_map),
            hdf5::LocationType::Dataset => (n_group, n_ds + 1, n_dtype, n_map),
            hdf5::LocationType::NamedDatatype => (n_group, n_ds, n_dtype + 1, n_map),
            hdf5::LocationType::TypeMap => (n_group, n_ds, n_dtype, n_map + 1),
        },
    );

    let bump = Bump::new();
    let mut buffer = BumpVec::<u8>::new_in(&bump);
    write_title(
        &mut buffer,
        "Group",
        &printer.style().group,
        GROUP_CHARACTER,
        printer,
    )?;
    buffer.execute(Print('\n'))?;

    write_item(&mut buffer, "Groups", n_group, printer)?;
    buffer.execute(Print("  "))?;
    write_item(&mut buffer, "Datasets", n_ds, printer)?;
    buffer.execute(Print('\n'))?;
    write_item(&mut buffer, "Named datatypes", n_dtype, printer)?;
    buffer.execute(Print("  "))?;
    write_item(&mut buffer, "Type maps", n_map, printer)?;

    printer.println(BumpString::from_utf8_lossy_in(&buffer, &bump));
    Ok(CommandOutcome::KeepRunning)
}

fn inspect_dataset(dataset: H5Dataset, printer: &Printer) -> CmdResult {
    let bump = Bump::new();

    let mut buffer = BumpVec::<u8>::new_in(&bump);
    write_title(
        &mut buffer,
        "Dataset",
        &printer.style().dataset,
        DATASET_CHARACTER,
        printer,
    )?;
    buffer.execute(Print("      "))?;
    buffer.append(&mut load_and_inspect_data(&dataset, printer, &bump)?);

    printer.println(BumpString::from_utf8_lossy_in(&buffer, &bump));
    Ok(CommandOutcome::KeepRunning)
}

fn inspect_attr(attr: H5Attribute, printer: &Printer) -> CmdResult {
    let bump = Bump::new();
    let mut buffer = BumpVec::<u8>::new_in(&bump);

    write_title(
        &mut buffer,
        "Attribute",
        &printer.style().attribute,
        ATTRIBUTE_CHARACTER,
        printer,
    )?;
    buffer.execute(Print("      "))?;
    buffer.append(&mut load_and_inspect_data(&attr, printer, &bump)?);

    printer.println(BumpString::from_utf8_lossy_in(&buffer, &bump));
    Ok(CommandOutcome::KeepRunning)
}

pub fn load_and_inspect_data<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError> {
    match container.dtype()?.to_descriptor()? {
        TypeDescriptor::VarLenUnicode => var_len_unicode(container, printer, bump),
        TypeDescriptor::VarLenAscii => var_len_ascii(container, printer, bump),
        TypeDescriptor::FixedUnicode(n) => fixed_len_unicode(container, n, printer, bump),
        TypeDescriptor::FixedAscii(n) => fixed_len_ascii(container, n, printer, bump),
        TypeDescriptor::Float(float_size) => any_float(container, float_size, printer, bump),
        TypeDescriptor::Integer(int_size) => any_signed_int(container, int_size, printer, bump),
        TypeDescriptor::Unsigned(int_size) => any_unsigned_int(container, int_size, printer, bump),
        TypeDescriptor::Boolean => boolean(container, printer, bump),
        descriptor => Err(CommandError::Error(std::format!(
            "dtype not supported: {}",
            printer.format_dtype(&descriptor, bump)
        ))),
    }
}

fn var_len_unicode<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError> {
    string::<VarLenUnicode>(container, printer, bump)
}

fn var_len_ascii<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError> {
    string::<VarLenAscii>(container, printer, bump)
}

fn fixed_len_unicode<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    n: usize,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError> {
    const MAX_N: usize = 1024;
    if n > MAX_N {
        return Err(CommandError::Error(std::format!(
            "Can only read fixed-length strings of up to {MAX_N} bytes"
        )));
    }
    string::<FixedUnicode<MAX_N>>(container, printer, bump)
}

fn fixed_len_ascii<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    n: usize,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError> {
    const MAX_N: usize = 1024;
    if n > MAX_N {
        return Err(CommandError::Error(std::format!(
            "Can only read fixed-length strings of up to {MAX_N} bytes"
        )));
    }
    string::<FixedAscii<MAX_N>>(container, printer, bump)
}

fn string<'alloc, T: H5Type + Display + Deref<Target = str>>(
    container: &impl Deref<Target = hdf5::Container>,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError> {
    let content = container.read::<T, IxDyn>()?;
    let mut buffer = BumpVec::<u8>::new_in(bump);

    write_item(
        &mut buffer,
        "DType",
        printer.format_dtype(&T::type_descriptor(), bump),
        printer,
    )?
    .execute(Print('\n'))?;

    write_item_debug(&mut buffer, "Shape", content.shape(), printer, bump)?.execute(Print("  "))?;
    write_item(&mut buffer, "Volume", content.len(), printer)?.execute(Print('\n'))?;

    let all_ascii = content.iter().all(|item| item.is_ascii());
    write_item(&mut buffer, "All ASCII", all_ascii, printer)?;

    Ok(buffer)
}

fn any_float<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    float_size: FloatSize,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError> {
    match float_size {
        FloatSize::U8 => number::<f64>(container, printer, bump),
        FloatSize::U4 => number::<f32>(container, printer, bump),
        // f16 is unstable, so approximate using f32
        FloatSize::U2 => number::<f32>(container, printer, bump),
    }
}

fn any_signed_int<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    int_size: IntSize,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError> {
    match int_size {
        IntSize::U8 => number::<i64>(container, printer, bump),
        IntSize::U4 => number::<i32>(container, printer, bump),
        IntSize::U2 => number::<i16>(container, printer, bump),
        IntSize::U1 => number::<i8>(container, printer, bump),
    }
}

fn any_unsigned_int<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    int_size: IntSize,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError> {
    match int_size {
        IntSize::U8 => number::<u64>(container, printer, bump),
        IntSize::U4 => number::<u32>(container, printer, bump),
        IntSize::U2 => number::<u16>(container, printer, bump),
        IntSize::U1 => number::<u8>(container, printer, bump),
    }
}

trait IsNonFinite {
    fn can_be_non_finite() -> bool {
        false
    }

    fn is_nan(&self) -> bool {
        false
    }

    fn is_inf(&self) -> bool {
        false
    }
}

impl IsNonFinite for f64 {
    fn can_be_non_finite() -> bool {
        true
    }

    fn is_nan(&self) -> bool {
        f64::is_nan(*self)
    }

    fn is_inf(&self) -> bool {
        f64::is_infinite(*self)
    }
}

impl IsNonFinite for f32 {
    fn can_be_non_finite() -> bool {
        true
    }

    fn is_nan(&self) -> bool {
        f32::is_nan(*self)
    }

    fn is_inf(&self) -> bool {
        f32::is_infinite(*self)
    }
}

impl IsNonFinite for i64 {}
impl IsNonFinite for i32 {}
impl IsNonFinite for i16 {}
impl IsNonFinite for i8 {}
impl IsNonFinite for u64 {}
impl IsNonFinite for u32 {}
impl IsNonFinite for u16 {}
impl IsNonFinite for u8 {}

trait Number:
    std::ops::Add<Output = Self>
    + std::ops::Div<Output = Self>
    + std::ops::Sub<Output = Self>
    + Copy
    + IsNonFinite
    + PartialEq
    + PartialOrd
{
    fn zero() -> Self;
    fn as_f64(self) -> f64;
    fn is_negative(self) -> bool;
}

macro_rules! impl_number {
    ($($t:ty),* $(,)?) => {
        $(
            impl Number for $t {
                fn zero() -> Self { 0 as $t }
                fn as_f64(self) -> f64 { self as f64 }
                fn is_negative(self) -> bool { self < 0 as $t }
            }
        )*
    };
}

impl_number!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64);

struct NumberAccumulator<T> {
    min: T,
    max: T,
    // Accumulate in f64 for the best precision.
    // Otherwise, we can get overflow or underflow.
    mean: f64,
    n_normal: usize,
    n_zeros: usize,
    n_nan: usize,
    n_inf: usize,
}

impl<T: Number> NumberAccumulator<T> {
    fn accumulate(acc: Option<Self>, x: &T) -> Option<Self> {
        match acc {
            Some(acc) => {
                let new_n = acc.n_normal + if Self::is_normal(*x) { 1 } else { 0 };
                Some(Self {
                    min: Self::nan_min(acc.min, *x),
                    max: Self::nan_max(acc.max, *x),
                    mean: Self::nan_mean(acc.mean, *x, new_n),
                    n_normal: new_n,
                    n_zeros: acc.n_zeros + if *x == T::zero() { 1 } else { 0 },
                    n_nan: if x.is_nan() { acc.n_nan + 1 } else { acc.n_nan },
                    n_inf: if x.is_inf() { acc.n_inf + 1 } else { acc.n_inf },
                })
            }
            None => Some(Self {
                min: *x,
                max: *x,
                mean: x.as_f64(),
                n_normal: if Self::is_normal(*x) { 1 } else { 0 },
                n_zeros: if *x == T::zero() { 1 } else { 0 },
                n_nan: if x.is_nan() { 1 } else { 0 },
                n_inf: if x.is_inf() { 1 } else { 0 },
            }),
        }
    }

    fn nan_min(acc: T, x: T) -> T {
        if !Self::is_normal(acc) {
            if !Self::is_normal(x) {
                if x.is_negative() { x } else { acc }
            } else {
                x
            }
        } else if !Self::is_normal(x) {
            acc
        } else {
            if x < acc { x } else { acc }
        }
    }

    fn nan_max(acc: T, x: T) -> T {
        if !Self::is_normal(acc) {
            if !Self::is_normal(x) {
                if x.is_negative() { acc } else { x }
            } else {
                x
            }
        } else if !Self::is_normal(x) {
            acc
        } else {
            if x > acc { x } else { acc }
        }
    }

    fn nan_mean(acc: f64, x: T, new_n: usize) -> f64 {
        if !Self::is_normal(acc) {
            x.as_f64()
        } else if !Self::is_normal(x) {
            acc
        } else {
            acc + (x.as_f64() - acc) / new_n as f64
        }
    }

    fn is_normal<X: Number>(x: X) -> bool {
        !x.is_nan() && !x.is_inf()
    }
}

fn number<'alloc, T>(
    container: &impl Deref<Target = hdf5::Container>,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError>
where
    T: Number + H5Type + Display + std::fmt::Debug,
{
    let content = container.read::<T, IxDyn>()?;

    let acc = content.iter().fold(None, NumberAccumulator::accumulate);
    let volume = content.len();

    let mut buffer = BumpVec::<u8>::new_in(bump);

    write_item(
        &mut buffer,
        "DType",
        printer.format_dtype(&T::type_descriptor(), bump),
        printer,
    )?
    .execute(Print('\n'))?;

    write_item_debug(&mut buffer, "Shape", content.shape(), printer, bump)?.execute(Print("  "))?;
    write_item(&mut buffer, "Volume", volume, printer)?.execute(Print("  "))?;
    write_item(
        &mut buffer,
        "Size",
        printer.format_human_size_in(
            volume as u64 * T::type_descriptor().size() as u64,
            false,
            bump,
        ),
        printer,
    )?
    .execute(Print('\n'))?;

    if let Some(acc) = acc {
        write_item_debug(&mut buffer, "Range", [acc.min, acc.max], printer, bump)?
            .execute(Print("  "))?;
        write_item(&mut buffer, "Mean", acc.mean, printer)?.execute(Print('\n'))?;
        write_item(&mut buffer, "Zeros", acc.n_zeros, printer)?;
        if T::can_be_non_finite() {
            buffer.execute(Print("  "))?;
            write_item(&mut buffer, "NaNs", acc.n_nan, printer)?.execute(Print("  "))?;
            write_item(&mut buffer, "Infs", acc.n_inf, printer)?;
        }
    }

    Ok(buffer)
}

fn boolean<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    printer: &Printer,
    bump: &'alloc Bump,
) -> Result<BumpVec<'alloc, u8>, CommandError> {
    let content = container.read::<bool, IxDyn>()?;

    let volume = content.len();

    let mut buffer = BumpVec::<u8>::new_in(bump);

    write_item(
        &mut buffer,
        "DType",
        printer.format_dtype(&bool::type_descriptor(), bump),
        printer,
    )?
    .execute(Print('\n'))?;

    write_item_debug(&mut buffer, "Shape", content.shape(), printer, bump)?.execute(Print("  "))?;
    write_item(&mut buffer, "Volume", volume, printer)?.execute(Print("  "))?;
    write_item(
        &mut buffer,
        "Size",
        printer.format_human_size_in(
            volume as u64 * bool::type_descriptor().size() as u64,
            false,
            bump,
        ),
        printer,
    )?
    .execute(Print('\n'))?;

    if volume > 0 {
        let (all, any) = content
            .iter()
            .fold((true, false), |(all, any), &x| (all && x, any || x));
        write_item(&mut buffer, "All true", all, printer)?.execute(Print("  "))?;
        write_item(&mut buffer, "Any true", any, printer)?;
    }

    Ok(buffer)
}

fn write_item<'e, E: ExecutableCommand, T: Display>(
    e: &'e mut E,
    label: &str,
    value: T,
    printer: &Printer,
) -> std::io::Result<&'e mut E> {
    write_label(e, label, printer)?.execute(Print(value))
}

fn write_item_debug<'e, E: ExecutableCommand, T: std::fmt::Debug>(
    e: &'e mut E,
    label: &str,
    value: T,
    printer: &Printer,
    bump: &Bump,
) -> std::io::Result<&'e mut E> {
    write_label(e, label, printer)?.execute(Print(format!(in bump, "{:?}", value)))
}

fn write_label<'e, E: ExecutableCommand>(
    e: &'e mut E,
    label: &str,
    printer: &Printer,
) -> std::io::Result<&'e mut E> {
    e.execute(&printer.style().size)?
        .execute(Print(label))?
        .execute(printer.style().reset())?
        .execute(Print(": "))
}

fn write_title<'e, E: ExecutableCommand>(
    e: &'e mut E,
    title: &str,
    style: &crate::output::style::Item,
    character: Option<char>,
    printer: &Printer,
) -> std::io::Result<&'e mut E> {
    e.execute(style)?
        .execute(&printer.style().emphasis)?
        .execute(Print(title))?
        .execute(printer.style().reset())?;
    if let Some(c) = character {
        e.execute(Print(c))?;
    }
    Ok(e)
}

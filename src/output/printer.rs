use super::Style;
use crate::cmd::CommandError;
use crate::h5::H5Object;
use bumpalo::{
    Bump,
    collections::{String as BumpString, Vec as BumpVec},
};
use crossterm::{
    QueueableCommand, execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use hdf5::types::{FloatSize, IntSize, Reference, TypeDescriptor};
use std::fmt::{Display, Formatter};
use std::io::{Write, stderr, stdout};
use term_grid::{Direction, Filling, Grid, GridOptions};

pub struct Printer {
    style: Style,
}

impl Printer {
    pub fn new() -> Self {
        Self {
            style: Style::new(),
        }
    }

    pub fn println<T: Display>(&self, line: T) {
        println!("{line}");
    }

    pub fn print_grid<T: AsRef<str>>(&self, cells: Vec<T>) {
        let grid = Grid::new(
            cells,
            GridOptions {
                filling: Filling::Spaces(2),
                direction: Direction::TopToBottom,
                width: terminal_width(),
            },
        );
        let _ = stdout().write_all(grid.to_string().as_bytes());
    }

    pub fn print_cmd_error(&self, error: &CommandError) {
        let mut stderr = stderr();
        match error {
            CommandError::Error(message) => {
                let _ = queue!(
                    stderr,
                    SetForegroundColor(Color::DarkRed),
                    Print("Error: "),
                    Print(message),
                    ResetColor,
                    Print("\n"),
                );
            }
            CommandError::NoMessage => {}
            CommandError::Critical(message) => {
                let _ = queue!(
                    stderr,
                    SetForegroundColor(Color::Red),
                    Print("CRITICAL ERROR: "),
                    SetForegroundColor(Color::DarkRed),
                    Print(message),
                    ResetColor,
                    Print("\n"),
                );
            }
        }
        let _ = stderr.flush();
    }

    pub fn print_shell_error<M: Display>(&self, message: M) {
        let mut stderr = stderr();
        let _ = queue!(
            stderr,
            SetForegroundColor(Color::DarkRed),
            Print(message),
            ResetColor,
            Print("\n"),
        );
        let _ = stderr.flush();
    }

    pub fn format_object_name<'alloc>(
        &self,
        name: &str,
        object: &H5Object,
        bump: &'alloc Bump,
    ) -> BumpString<'alloc> {
        let mut buffer = BumpVec::<u8>::new_in(bump);
        let (style, character) = match object {
            H5Object::Dataset(_) => (&self.style().dataset, ' '),
            H5Object::Group(_) => (&self.style().group, '/'),
            H5Object::Attribute(_) => (&self.style().attribute, '@'),
        };
        let _ = execute!(
            buffer,
            style,
            Print(name),
            ResetColor,
            SetAttribute(Attribute::Reset),
            Print(character),
        );
        BumpString::from_utf8(buffer).unwrap_or_else(|_| BumpString::new_in(bump))
    }

    pub fn style(&self) -> &Style {
        &self.style
    }

    pub fn format_human_size_in<'alloc>(
        &self,
        size: u64,
        short: bool,
        bump: &'alloc Bump,
    ) -> BumpString<'alloc> {
        use std::fmt::Write;
        let mut out = BumpString::new_in(bump);

        let units = if short {
            &BYTE_UNITS_SHORT
        } else {
            &BYTE_UNITS_LONG
        };
        let mut size = size;
        for unit in units.iter() {
            if size < 1024 {
                let _ = write!(&mut out, "{size}{unit}");
                return out;
            }
            size /= 1024;
        }
        let _ = write!(&mut out, "{size}{}", units[units.len() - 1]);
        out
    }

    pub fn format_dtype<'alloc>(
        &self,
        type_descriptor: &TypeDescriptor,
        bump: &'alloc Bump,
    ) -> BumpString<'alloc> {
        use std::fmt::Write;
        let mut out = BumpString::new_in(bump);
        let _ = match type_descriptor {
            TypeDescriptor::Integer(IntSize::U1) => write!(&mut out, "i8"),
            TypeDescriptor::Integer(IntSize::U2) => write!(&mut out, "i16"),
            TypeDescriptor::Integer(IntSize::U4) => write!(&mut out, "i32"),
            TypeDescriptor::Integer(IntSize::U8) => write!(&mut out, "i64"),
            TypeDescriptor::Unsigned(IntSize::U1) => write!(&mut out, "u8"),
            TypeDescriptor::Unsigned(IntSize::U2) => write!(&mut out, "u16"),
            TypeDescriptor::Unsigned(IntSize::U4) => write!(&mut out, "u32"),
            TypeDescriptor::Unsigned(IntSize::U8) => write!(&mut out, "u64"),
            TypeDescriptor::Float(FloatSize::U2) => write!(&mut out, "f16"),
            TypeDescriptor::Float(FloatSize::U4) => write!(&mut out, "f32"),
            TypeDescriptor::Float(FloatSize::U8) => write!(&mut out, "f64"),
            TypeDescriptor::Boolean => write!(&mut out, "bool"),
            TypeDescriptor::Enum(tp) => write!(&mut out, "enum ({})", tp.base_type()),
            TypeDescriptor::Compound(tp) => {
                write!(&mut out, "compound ({})", tp.fields.len())
            }
            TypeDescriptor::FixedArray(tp, n) => write!(&mut out, "[{tp}; {n}]"),
            TypeDescriptor::FixedAscii(n) => write!(&mut out, "ascii({n})"),
            TypeDescriptor::FixedUnicode(n) => write!(&mut out, "utf-8({n})"),
            TypeDescriptor::VarLenArray(tp) => write!(&mut out, "[{tp}]"),
            TypeDescriptor::VarLenAscii => write!(&mut out, "ascii"),
            TypeDescriptor::VarLenUnicode => write!(&mut out, "utf-8"),
            TypeDescriptor::Reference(Reference::Object) => write!(&mut out, "ref (object)"),
            TypeDescriptor::Reference(Reference::Region) => write!(&mut out, "ref (region)"),
            TypeDescriptor::Reference(Reference::Std) => write!(&mut out, "ref"),
        };
        out
    }

    pub fn queue_object_table<'q, Q: Write>(
        &self,
        queue: &'q mut Q,
        objects: &[(&str, &H5Object)],
        show_content: bool,
    ) -> std::io::Result<&'q mut Q> {
        super::table::queue_object_table(queue, objects, self, show_content)
    }

    pub fn queue_padding(&self, out: &mut impl Write, padding: usize) -> std::io::Result<()> {
        if padding > 0 {
            out.queue(Print(Padding(padding)))?;
        }
        Ok(())
    }

    pub fn terminal_size(&self) -> (u16, u16) {
        crossterm::terminal::size().unwrap_or((48, 128))
    }
}

const BYTE_UNITS_SHORT: [&str; 5] = ["B ", "Ki", "Mi", "Gi", "Ti"];
const BYTE_UNITS_LONG: [&str; 5] = ["B  ", "KiB", "MiB", "GiB", "TiB"];

fn terminal_width() -> usize {
    crossterm::terminal::window_size().map_or(96, |size| size.columns as usize)
}

const PADDING_BUFFER: &str = "                                    ";
struct Padding(usize);

impl Display for Padding {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(PADDING_BUFFER.get(0..self.0).unwrap_or(PADDING_BUFFER))
    }
}

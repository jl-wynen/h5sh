use bumpalo::collections::CollectIn;
use bumpalo::{
    Bump,
    collections::{String as BumpString, Vec as BumpVec},
};
use clap::{ArgGroup, ArgMatches, CommandFactory, FromArgMatches, Parser};
use crossterm::{
    ExecutableCommand, QueueableCommand,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::fmt::{Display, Formatter};
use std::io::{Write, stdout};

use crate::cmd::{CmdResult, Command, CommandError, CommandOutcome};
use crate::data::load_and_format_data;
use crate::h5::{H5Dataset, H5File, H5Object, H5Path};
use crate::output::Printer;
use crate::shell::Shell;

#[derive(Clone, Copy, Default)]
pub struct Ls;

impl Command for Ls {
    fn run(&self, args: ArgMatches, shell: &Shell, file: &H5File) -> CmdResult {
        let Ok(args) = Arguments::from_arg_matches(&args) else {
            return Err(CommandError::Critical("Failed to extract args".to_string()));
        };
        let options = Options::from_args(&args);
        let target = shell.resolve_path(&args.path);

        match file.load(&target)? {
            H5Object::Group(group) => {
                print_objects(file.load_children(group)?, shell.printer(), options);
            }
            dataset @ H5Object::Dataset(_) => {
                print_objects(std::iter::once(dataset), shell.printer(), options);
            }
        }
        Ok(CommandOutcome::KeepRunning)
    }

    fn arg_parser(&self) -> clap::Command {
        Arguments::command()
    }
}

/// List group contents.
#[derive(Parser, Debug)]
#[command(name("ls"))]
#[clap(group(ArgGroup::new("sort").args(&["name", "ty"])))]
struct Arguments {
    /// List children of this path.
    #[arg(default_value = ".")]
    path: H5Path,

    /// Show object metadata in a table.
    #[arg(short = 'l', default_value_t = false)]
    long: bool,

    /// Sort by name (default).
    #[arg(long)]
    name: bool,

    /// Do not show the content of datasets.
    #[arg(short = 'c', long)]
    no_content: bool,

    /// Sort by object type.
    #[arg(short = 't', long = "type")]
    ty: bool,
}

struct Options {
    long: bool,
    sort_by: SortBy,
    show_content: bool,
}

enum SortBy {
    Name,
    Type,
}

impl Options {
    fn from_args(args: &Arguments) -> Self {
        let sort_by = if args.ty {
            SortBy::Type
        } else {
            // This case must be at the end, we sort by name if the flag is
            // explicitly provided or if no flag is provided.
            SortBy::Name
        };

        Options {
            long: args.long,
            show_content: !args.no_content,
            sort_by,
        }
    }
}

fn print_objects<It: Iterator<Item = H5Object>>(objects: It, printer: &Printer, options: Options) {
    let objects_vec: Vec<_> = objects.collect();
    let mut objects = objects_vec
        .iter()
        .map(|obj| (obj.path().name(), obj))
        .collect();
    sort_objects(&mut objects, options.sort_by);
    if options.long {
        let _ = print_object_table(objects, printer, options.show_content);
    } else {
        print_object_grid(objects, printer)
    }
}

fn print_object_grid(objects: Vec<(&str, &H5Object)>, printer: &Printer) {
    let bump = Bump::new();
    printer.print_grid(
        objects
            .into_iter()
            .map(|(name, object)| format_object_name(name, object, printer, &bump))
            .collect(),
    );
}

fn print_object_table(
    objects: Vec<(&str, &H5Object)>,
    printer: &Printer,
    show_content: bool,
) -> std::io::Result<()> {
    let bump = Bump::new();
    let n_rows = objects.len();

    let mut columns = Vec::with_capacity(5);
    columns.push(build_shape_column(&bump, &objects)?);
    columns.push(build_size_column(&bump, &objects, printer)?);
    columns.push(build_dtype_column(&bump, &objects, printer)?);
    columns.push(build_name_column(&bump, &objects, printer)?);

    let mut widths: BumpVec<_> = columns.iter().map(Column::max_width).collect_in(&bump);

    if show_content {
        let used_width: usize = widths.iter().sum();
        let (full_width, _) = printer.terminal_size();
        // -4 for spacing between columns
        let available_width = full_width as usize - used_width - 5 - 1;

        let content_column = build_content_column(&bump, &objects, available_width, printer)?;
        widths.push(content_column.max_width());
        columns.push(content_column);
    }

    print_table_columns(columns, &widths, n_rows)
}

fn print_table_columns(
    columns: Vec<Column>,
    widths: &[usize],
    n_rows: usize,
) -> std::io::Result<()> {
    let mut stdout = stdout();
    for i_row in 0..n_rows {
        for (i_col, (column, &width)) in Iterator::zip(columns.iter(), widths.iter()).enumerate() {
            let padding = width - column.widths[i_row];
            if !column.left_aligned {
                queue_padding(&mut stdout, padding)?;
            }
            stdout.queue(Print(column.formatted[i_row].as_str()))?;
            if column.left_aligned {
                queue_padding(&mut stdout, padding)?;
            }
            if i_col < columns.len() - 1 {
                stdout.queue(Print(' '))?;
            }
        }
        stdout.queue(Print('\n'))?;
    }
    stdout.flush()
}

fn queue_padding(out: &mut impl Write, padding: usize) -> std::io::Result<()> {
    if padding > 0 {
        out.queue(Print(Padding(padding)))?;
    }
    Ok(())
}

struct Column<'alloc> {
    widths: BumpVec<'alloc, usize>,
    formatted: BumpVec<'alloc, BumpString<'alloc>>,
    left_aligned: bool,
}

impl<'alloc> Column<'alloc> {
    fn max_width(&self) -> usize {
        self.widths.iter().max().copied().unwrap_or(0)
    }
}

fn build_name_column<'alloc>(
    bump: &'alloc Bump,
    objects: &[(&str, &H5Object)],
    printer: &Printer,
) -> std::io::Result<Column<'alloc>> {
    let mut column = Column {
        widths: BumpVec::with_capacity_in(objects.len(), bump),
        formatted: BumpVec::with_capacity_in(objects.len(), bump),
        left_aligned: true,
    };
    for (name, object) in objects {
        // +1 for symbol (e.g. '/' for groups)
        column.widths.push(name.len() + 1);
        column
            .formatted
            .push(format_object_name(name, object, printer, bump));
    }
    Ok(column)
}

fn format_object_name<'alloc>(
    name: &str,
    object: &H5Object,
    printer: &Printer,
    bump: &'alloc Bump,
) -> BumpString<'alloc> {
    match object {
        H5Object::Dataset(_) => printer.apply_style_dataset_in(name, bump),
        H5Object::Group(_) => {
            let mut formatted = printer.apply_style_group_in(name, bump);
            formatted.push('/');
            formatted
        }
    }
}

fn build_size_column<'alloc>(
    bump: &'alloc Bump,
    objects: &[(&str, &H5Object)],
    printer: &Printer,
) -> std::io::Result<Column<'alloc>> {
    let mut column = Column {
        widths: BumpVec::with_capacity_in(objects.len(), bump),
        formatted: BumpVec::with_capacity_in(objects.len(), bump),
        left_aligned: false,
    };
    for (_, object) in objects {
        match object {
            H5Object::Dataset(dataset) => {
                let size =
                    printer.format_human_size_in(dataset.underlying().storage_size(), true, bump);

                column.widths.push(size.len());

                let mut buffer = BumpVec::<u8>::new_in(bump);
                buffer
                    .execute(SetForegroundColor(Color::DarkGreen))?
                    .execute(Print(size))?
                    .execute(ResetColor)?;
                column.formatted.push(
                    BumpString::from_utf8(buffer).unwrap_or_else(|_| BumpString::new_in(bump)),
                )
            }
            H5Object::Group(_) => {
                column.widths.push(0);
                column.formatted.push(BumpString::new_in(bump));
            }
        }
    }
    Ok(column)
}

fn build_shape_column<'alloc>(
    bump: &'alloc Bump,
    objects: &[(&str, &H5Object)],
) -> std::io::Result<Column<'alloc>> {
    let mut column = Column {
        widths: BumpVec::with_capacity_in(objects.len(), bump),
        formatted: BumpVec::with_capacity_in(objects.len(), bump),
        left_aligned: true,
    };
    for (_, object) in objects {
        match object {
            H5Object::Dataset(dataset) => {
                let shape = dataset.underlying().shape();
                let (width, formatted) = format_shape(&shape, bump)?;
                column.widths.push(width);
                column.formatted.push(formatted);
            }
            H5Object::Group(_) => {
                column.widths.push(0);
                column.formatted.push(BumpString::new_in(bump));
            }
        }
    }
    Ok(column)
}

fn format_shape<'alloc>(
    shape: &[usize],
    bump: &'alloc Bump,
) -> std::io::Result<(usize, BumpString<'alloc>)> {
    let mut width = 2; // initial value for parentheses
    let mut buffer = BumpVec::<u8>::new_in(bump);
    buffer.execute(Print("("))?;
    let mut first = true;
    for dim in shape {
        if !first {
            buffer.execute(Print(", "))?;
            width += 2;
        } else {
            first = false;
        }
        let dim_str = dim.to_string();
        width += dim_str.len();
        buffer
            .execute(SetForegroundColor(Color::DarkCyan))?
            .execute(Print(dim_str))?
            .execute(ResetColor)?;
    }
    buffer.execute(Print(")"))?;
    Ok((
        width,
        BumpString::from_utf8(buffer).unwrap_or_else(|_| BumpString::new_in(bump)),
    ))
}

fn build_dtype_column<'alloc>(
    bump: &'alloc Bump,
    objects: &[(&str, &H5Object)],
    printer: &Printer,
) -> std::io::Result<Column<'alloc>> {
    let mut column = Column {
        widths: BumpVec::with_capacity_in(objects.len(), bump),
        formatted: BumpVec::with_capacity_in(objects.len(), bump),
        left_aligned: true,
    };
    for (_, object) in objects {
        match object {
            H5Object::Dataset(dataset) => {
                let (width, formatted) = if let Ok(descriptor) = dataset.type_descriptor() {
                    format_dtype(&descriptor, printer, bump)?
                } else {
                    format_unknown_dtype(bump)?
                };
                column.widths.push(width);
                column.formatted.push(formatted);
            }
            H5Object::Group(_) => {
                column.widths.push(3);
                column.formatted.push(BumpString::from_str_in("grp", bump));
            }
        }
    }
    Ok(column)
}

fn format_dtype<'alloc>(
    descriptor: &hdf5::types::TypeDescriptor,
    printer: &Printer,
    bump: &'alloc Bump,
) -> std::io::Result<(usize, BumpString<'alloc>)> {
    let dtype = printer.format_dtype(descriptor, bump);
    let width = dtype.len();
    let mut buffer = BumpVec::<u8>::new_in(bump);
    buffer
        .execute(SetForegroundColor(Color::DarkMagenta))?
        .execute(Print(dtype))?
        .execute(ResetColor)?;
    let formatted = BumpString::from_utf8(buffer).unwrap_or_else(|_| BumpString::new_in(bump));
    Ok((width, formatted))
}

fn format_unknown_dtype(bump: &Bump) -> std::io::Result<(usize, BumpString)> {
    let mut buffer = BumpVec::<u8>::new_in(bump);
    buffer
        .execute(Print('<'))?
        .execute(SetForegroundColor(Color::DarkMagenta))?
        .execute(Print('?'))?
        .execute(ResetColor)?
        .execute(Print('>'))?;
    let formatted = BumpString::from_utf8(buffer).unwrap_or_else(|_| BumpString::new_in(bump));
    Ok((3, formatted))
}

fn build_content_column<'alloc>(
    bump: &'alloc Bump,
    objects: &[(&str, &H5Object)],
    width: usize,
    printer: &Printer,
) -> std::io::Result<Column<'alloc>> {
    let mut column = Column {
        widths: BumpVec::with_capacity_in(objects.len(), bump),
        formatted: BumpVec::with_capacity_in(objects.len(), bump),
        left_aligned: true,
    };
    for (_, object) in objects {
        match object {
            H5Object::Dataset(dataset) => {
                let formatted = format_dataset_content(dataset, width, printer, bump);
                column.widths.push(formatted.len());
                column.formatted.push(formatted);
            }
            H5Object::Group(_) => {
                column.widths.push(0);
                column.formatted.push(BumpString::new_in(bump));
            }
        }
    }
    Ok(column)
}

fn format_dataset_content<'alloc>(
    dataset: &H5Dataset,
    width: usize,
    printer: &Printer,
    bump: &'alloc Bump,
) -> BumpString<'alloc> {
    // TODO colour
    let formatted = load_and_format_data(dataset, Some(5), printer, bump).unwrap_or_else(|err| {
        use std::fmt::Write;
        let mut message = BumpString::new_in(bump);
        let _ = write!(&mut message, "{err}"); // TODO colour
        message
    });
    formatted
        .chars()
        // Replace all whitespace to avoid line breaks or large jumps
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .take(width)
        .collect_in(bump)
}

const PADDING_BUFFER: &str = "                                    ";
struct Padding(usize);

impl Display for Padding {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(PADDING_BUFFER.get(0..self.0).unwrap_or(PADDING_BUFFER))
    }
}

fn sort_objects(objects: &mut Vec<(&str, &H5Object)>, sort_by: SortBy) {
    // By name first to get sorted subgroups where other sorts are ambiguous.
    sort_objects_by_name(objects);
    match sort_by {
        SortBy::Name => { /* already sorted above */ }
        SortBy::Type => {
            sort_objects_by_type(objects);
        }
    }
}

fn sort_objects_by_name(objects: &mut Vec<(&str, &H5Object)>) {
    objects.sort_by_key(|(name, _)| *name);
}

fn sort_objects_by_type(objects: &mut Vec<(&str, &H5Object)>) {
    objects.sort_by(|(_, a), (_, b)| match (a, b) {
        (H5Object::Dataset(_), H5Object::Group(_)) => std::cmp::Ordering::Greater,
        (H5Object::Group(_), H5Object::Dataset(_)) => std::cmp::Ordering::Less,
        _ => std::cmp::Ordering::Equal,
    });
}

use crate::data::load_and_format_data;
use crate::h5::H5Object;
use crate::output::Printer;
use bumpalo::{
    Bump,
    collections::{CollectIn, String as BumpString, Vec as BumpVec},
};
use crossterm::{
    ExecutableCommand, QueueableCommand,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io::Write;
use std::ops::Deref;

pub(super) fn queue_object_table<'q, Q: Write>(
    queue: &'q mut Q,
    objects: &[(&str, &H5Object)],
    printer: &Printer,
    show_content: bool,
) -> std::io::Result<&'q mut Q> {
    let bump = Bump::new();
    let n_rows = objects.len();

    let mut columns = Vec::with_capacity(5);
    columns.push(build_shape_column(&bump, objects)?);
    columns.push(build_size_column(&bump, objects, printer)?);
    columns.push(build_dtype_column(&bump, objects, printer)?);
    columns.push(build_name_column(&bump, objects, printer)?);

    let mut widths: BumpVec<_> = columns.iter().map(Column::max_width).collect_in(&bump);

    if show_content {
        let used_width: usize = widths.iter().sum();
        let (full_width, _) = printer.terminal_size();
        // -4 for spacing between columns
        let available_width = full_width as usize - used_width - 5 - 1;

        let content_column = build_content_column(&bump, objects, available_width, printer)?;
        widths.push(content_column.max_width());
        columns.push(content_column);
    }

    queue_table_columns(queue, columns, &widths, n_rows, printer)
}

fn queue_table_columns<'q, Q: Write>(
    queue: &'q mut Q,
    columns: Vec<Column>,
    widths: &[usize],
    n_rows: usize,
    printer: &Printer,
) -> std::io::Result<&'q mut Q> {
    for i_row in 0..n_rows {
        for (i_col, (column, &width)) in Iterator::zip(columns.iter(), widths.iter()).enumerate() {
            let padding = width.saturating_sub(column.widths[i_row]);
            if !column.left_aligned {
                printer.queue_padding(queue, padding)?;
            }
            queue.queue(Print(column.formatted[i_row].as_str()))?;
            if column.left_aligned {
                printer.queue_padding(queue, padding)?;
            }
            if i_col < columns.len() - 1 {
                queue.queue(Print(' '))?;
            }
        }
        queue.queue(Print('\n'))?;
    }
    queue.flush()?;
    Ok(queue)
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
            .push(printer.format_object_name(name, object, bump));
    }
    Ok(column)
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
        let (width, formatted) = match object {
            H5Object::Dataset(dataset) => {
                format_size(dataset.underlying().storage_size(), printer, bump)?
            }
            H5Object::Group(_) => (0, BumpString::new_in(bump)),
            H5Object::Attribute(attr) => {
                format_size(attr.underlying().storage_size(), printer, bump)?
            }
        };
        column.widths.push(width);
        column.formatted.push(formatted);
    }
    Ok(column)
}

fn format_size<'alloc>(
    size: u64,
    printer: &Printer,
    bump: &'alloc Bump,
) -> std::io::Result<(usize, BumpString<'alloc>)> {
    let size = printer.format_human_size_in(size, true, bump);
    let width = size.len();

    let mut buffer = BumpVec::<u8>::new_in(bump);
    buffer
        .execute(SetForegroundColor(Color::DarkGreen))?
        .execute(Print(size))?
        .execute(ResetColor)?;
    let formatted = BumpString::from_utf8(buffer).unwrap_or_else(|_| BumpString::new_in(bump));

    Ok((width, formatted))
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
        let (width, formatted) = match object {
            H5Object::Dataset(dataset) => format_shape(&dataset.shape(), bump)?,
            H5Object::Group(_) => (0, BumpString::new_in(bump)),
            H5Object::Attribute(attr) => format_shape(&attr.shape(), bump)?,
        };
        column.widths.push(width);
        column.formatted.push(formatted);
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
        let (width, formatted) = match object {
            H5Object::Dataset(dataset) => format_dtype_of(dataset, printer, bump)?,
            H5Object::Group(_) => (3, BumpString::from_str_in("grp", bump)),
            H5Object::Attribute(attr) => format_dtype_of(attr, printer, bump)?,
        };
        column.widths.push(width);
        column.formatted.push(formatted);
    }
    Ok(column)
}

fn format_dtype_of<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    printer: &Printer,
    bump: &'alloc Bump,
) -> std::io::Result<(usize, BumpString<'alloc>)> {
    if let Ok(descriptor) = container.dtype()?.to_descriptor() {
        format_known_dtype(&descriptor, printer, bump)
    } else {
        format_unknown_dtype(bump)
    }
}

fn format_known_dtype<'alloc>(
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
        let formatted = match object {
            H5Object::Dataset(dataset) => format_content(dataset, width, printer, bump)
                .unwrap_or_else(|_| {
                    data_failure_message(bump).unwrap_or_else(|_| BumpString::new_in(bump))
                }),
            H5Object::Group(_) => BumpString::new_in(bump),
            H5Object::Attribute(attr) => {
                format_content(attr, width, printer, bump).unwrap_or_else(|_| {
                    data_failure_message(bump).unwrap_or_else(|_| BumpString::new_in(bump))
                })
            }
        };
        column.widths.push(formatted.len());
        column.formatted.push(formatted);
    }
    Ok(column)
}

fn format_content<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    width: usize,
    printer: &Printer,
    bump: &'alloc Bump,
) -> std::io::Result<BumpString<'alloc>> {
    if container.ndim() > 1 {
        data_placeholder(bump)
    } else {
        let formatted = load_and_format_data(container, Some(8), Some(width), printer, bump)
            .unwrap_or_else(|err| {
                use std::fmt::Write;
                let mut message = BumpString::new_in(bump);
                let _ = write!(&mut message, "{err}");
                message
            });
        Ok(formatted
            .chars()
            // Replace all whitespace to avoid line breaks or large jumps
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect_in(bump))
    }
}

fn data_placeholder(bump: &Bump) -> std::io::Result<BumpString> {
    let mut buffer = BumpVec::<u8>::new_in(bump);
    buffer
        .execute(SetForegroundColor(Color::DarkGrey))?
        .execute(Print("[...]"))?
        .execute(ResetColor)?;
    Ok(BumpString::from_utf8(buffer).unwrap_or_else(|_| BumpString::new_in(bump)))
}

fn data_failure_message(bump: &Bump) -> std::io::Result<BumpString> {
    let mut buffer = BumpVec::<u8>::new_in(bump);
    buffer
        .execute(SetForegroundColor(Color::DarkRed))?
        .execute(Print("Failed to load data"))?
        .execute(ResetColor)?;
    Ok(BumpString::from_utf8(buffer).unwrap_or_else(|_| BumpString::new_in(bump)))
}

use crate::h5::{self, PartialData};
use crate::output::Printer;
use bumpalo::{Bump, collections::String as BumpString};
use crossterm::{
    ExecutableCommand,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use hdf5::{H5Type, types::TypeDescriptor};
use std::fmt::Display;
use std::ops::Deref;

pub fn load_and_format_data<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    max_elem: Option<usize>,
    max_width: Option<usize>,
    printer: &Printer,
    bump: &'alloc Bump,
) -> h5::Result<BumpString<'alloc>> {
    match container.dtype()?.to_descriptor()? {
        TypeDescriptor::VarLenUnicode => {
            load_and_format::var_len_unicode(container, max_elem, max_width, bump)
        }
        TypeDescriptor::VarLenAscii => {
            load_and_format::var_len_ascii(container, max_elem, max_width, bump)
        }
        TypeDescriptor::FixedUnicode(n) => {
            load_and_format::fixed_len_unicode(container, n, max_elem, max_width, bump)
        }
        TypeDescriptor::FixedAscii(n) => {
            load_and_format::fixed_len_ascii(container, n, max_elem, max_width, bump)
        }
        TypeDescriptor::Float(float_size) => {
            load_and_format::float(container, float_size, max_elem, max_width, bump)
        }
        TypeDescriptor::Integer(int_size) => {
            load_and_format::signed_integer(container, int_size, max_elem, max_width, bump)
        }
        TypeDescriptor::Unsigned(int_size) => {
            load_and_format::unsigned_integer(container, int_size, max_elem, max_width, bump)
        }
        TypeDescriptor::Boolean => load_and_format::bool(container, max_elem, max_width, bump),
        descriptor => Err(h5::H5Error::Other(format!(
            "dtype not supported: {}",
            printer.format_dtype(&descriptor, bump)
        ))),
    }
}

mod load_and_format {
    use super::*;
    use crate::h5::H5Error;

    use hdf5::types::{FixedAscii, FixedUnicode, FloatSize, IntSize, VarLenAscii, VarLenUnicode};
    use ndarray::{IxDyn, s};

    pub(super) fn var_len_unicode<'alloc>(
        container: &impl Deref<Target = hdf5::Container>,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        load_and_format::<VarLenUnicode>(container, max_elem, max_width, bump)
    }

    pub(super) fn var_len_ascii<'alloc>(
        container: &impl Deref<Target = hdf5::Container>,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        load_and_format::<VarLenAscii>(container, max_elem, max_width, bump)
    }

    pub(super) fn fixed_len_unicode<'alloc>(
        container: &impl Deref<Target = hdf5::Container>,
        n: usize,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        const MAX_N: usize = 1024;
        if n > MAX_N {
            return Err(H5Error::Other(format!(
                "Can only read fixed-length strings of up to {MAX_N} bytes"
            )));
        }
        load_and_format::<FixedUnicode<MAX_N>>(container, max_elem, max_width, bump)
    }

    pub(super) fn fixed_len_ascii<'alloc>(
        container: &impl Deref<Target = hdf5::Container>,
        n: usize,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        const MAX_N: usize = 1024;
        if n > MAX_N {
            return Err(H5Error::Other(format!(
                "Can only read fixed-length strings of up to {MAX_N} bytes"
            )));
        }
        load_and_format::<FixedAscii<MAX_N>>(container, max_elem, max_width, bump)
    }

    pub(super) fn float<'alloc>(
        container: &impl Deref<Target = hdf5::Container>,
        float_size: FloatSize,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        match float_size {
            FloatSize::U8 => load_and_format::<f64>(container, max_elem, max_width, bump),
            FloatSize::U4 => load_and_format::<f32>(container, max_elem, max_width, bump),
            // f16 is unstable, so approximate using f32
            FloatSize::U2 => load_and_format::<f32>(container, max_elem, max_width, bump),
        }
    }

    pub(super) fn signed_integer<'alloc>(
        container: &impl Deref<Target = hdf5::Container>,
        int_size: IntSize,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        match int_size {
            IntSize::U8 => load_and_format::<i64>(container, max_elem, max_width, bump),
            IntSize::U4 => load_and_format::<i32>(container, max_elem, max_width, bump),
            IntSize::U2 => load_and_format::<i16>(container, max_elem, max_width, bump),
            IntSize::U1 => load_and_format::<i8>(container, max_elem, max_width, bump),
        }
    }

    pub(super) fn unsigned_integer<'alloc>(
        container: &impl Deref<Target = hdf5::Container>,
        int_size: IntSize,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        match int_size {
            IntSize::U8 => load_and_format::<u64>(container, max_elem, max_width, bump),
            IntSize::U4 => load_and_format::<u32>(container, max_elem, max_width, bump),
            IntSize::U2 => load_and_format::<u16>(container, max_elem, max_width, bump),
            IntSize::U1 => load_and_format::<u8>(container, max_elem, max_width, bump),
        }
    }

    pub(super) fn bool<'alloc>(
        container: &impl Deref<Target = hdf5::Container>,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        load_and_format::<bool>(container, max_elem, max_width, bump)
    }

    // Note that the max_width handling assumes that
    // the formatted array contains no escape sequences.
    fn load_and_format<'alloc, T: H5Type + Display>(
        container: &impl Deref<Target = hdf5::Container>,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        use std::fmt::Write;

        let content = if let Some(max_elem) = max_elem {
            read_first_n::<T>(container, max_elem)
        } else {
            Ok(container.read::<T, IxDyn>().map(PartialData::Full)?)
        }?;

        let mut out = BumpString::new_in(bump);

        let mut buffer: Vec<u8> = Vec::new();
        buffer.execute(Print(content.array())).unwrap();

        if write!(&mut out, "{}", content.array()).is_err() {
            let _ = write!(&mut out, "<failed write>");
        };
        if matches!(content, PartialData::Full(_)) {
            if let Some(max_width) = max_width {
                if max_width < out.len() {
                    out.truncate(max_width.saturating_sub(4));
                    out.push_str(&trailing_ellipses);
                }
            }
        } else {
            out.truncate(out.len().saturating_sub(1)); // remove final ']'
            if let Some(max_width) = max_width {
                let padded_width = out.len() + 4; // padded with " ..."
                if max_width >= padded_width {
                    out.push_str(&trailing_ellipses);
                } else {
                    out.truncate(max_width.saturating_sub(4));
                    out.push_str(&trailing_ellipses);
                }
            } else {
                out.push_str(&trailing_ellipses);
            }
        }
        Ok(out)
    }

    lazy_static::lazy_static! {
        static ref trailing_ellipses: String = {
            let mut buffer: Vec<u8> = Vec::new();
            buffer.execute(Print(" ")).unwrap();
            buffer.execute(SetForegroundColor(Color::DarkGrey)).unwrap();
            buffer.execute(Print("...")).unwrap();
            buffer.execute(ResetColor).unwrap();
            String::from_utf8(buffer).unwrap()
        };
    }

    fn read_first_n<T: H5Type>(
        container: &impl Deref<Target = hdf5::Container>,
        n: usize,
    ) -> h5::Result<PartialData<T>> {
        match container.shape()[..] {
            [] => Ok(PartialData::Full(container.read()?)),
            [size] => {
                let array = container.read_slice(s![..(n.min(size))])?;
                if n < size {
                    Ok(PartialData::FirstN(array))
                } else {
                    Ok(PartialData::Full(array))
                }
            }
            _ => Err(H5Error::Other(
                "Reading first n elements is only supported for scalar and 1d data.".to_string(),
            )),
        }
    }
}

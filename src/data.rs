use bumpalo::{Bump, collections::String as BumpString};
use hdf5::{H5Type, types::TypeDescriptor};
use std::fmt::Display;

use crate::h5::{self, H5Dataset, PartialData};
use crate::output::Printer;

pub fn load_and_format_data<'alloc>(
    dataset: &H5Dataset,
    max_elem: Option<usize>,
    max_width: Option<usize>,
    printer: &Printer,
    bump: &'alloc Bump,
) -> h5::Result<BumpString<'alloc>> {
    match dataset.type_descriptor()? {
        TypeDescriptor::VarLenUnicode => {
            load_and_format::var_len_unicode(dataset, max_elem, max_width, bump)
        }
        TypeDescriptor::VarLenAscii => {
            load_and_format::var_len_ascii(dataset, max_elem, max_width, bump)
        }
        TypeDescriptor::FixedUnicode(n) => {
            load_and_format::fixed_len_unicode(dataset, n, max_elem, max_width, bump)
        }
        TypeDescriptor::FixedAscii(n) => {
            load_and_format::fixed_len_ascii(dataset, n, max_elem, max_width, bump)
        }
        TypeDescriptor::Float(float_size) => {
            load_and_format::float(dataset, float_size, max_elem, max_width, bump)
        }
        TypeDescriptor::Integer(int_size) => {
            load_and_format::signed_integer(dataset, int_size, max_elem, max_width, bump)
        }
        TypeDescriptor::Unsigned(int_size) => {
            load_and_format::unsigned_integer(dataset, int_size, max_elem, max_width, bump)
        }
        TypeDescriptor::Boolean => load_and_format::bool(dataset, max_elem, max_width, bump),
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

    pub(super) fn var_len_unicode<'alloc>(
        dataset: &H5Dataset,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        load_and_format::<VarLenUnicode>(dataset, max_elem, max_width, bump)
    }

    pub(super) fn var_len_ascii<'alloc>(
        dataset: &H5Dataset,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        load_and_format::<VarLenAscii>(dataset, max_elem, max_width, bump)
    }

    pub(super) fn fixed_len_unicode<'alloc>(
        dataset: &H5Dataset,
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
        load_and_format::<FixedUnicode<MAX_N>>(dataset, max_elem, max_width, bump)
    }

    pub(super) fn fixed_len_ascii<'alloc>(
        dataset: &H5Dataset,
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
        load_and_format::<FixedAscii<MAX_N>>(dataset, max_elem, max_width, bump)
    }

    pub(super) fn float<'alloc>(
        dataset: &H5Dataset,
        float_size: FloatSize,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        match float_size {
            FloatSize::U8 => load_and_format::<f64>(dataset, max_elem, max_width, bump),
            FloatSize::U4 => load_and_format::<f32>(dataset, max_elem, max_width, bump),
            // f16 is unstable, so approximate using f32
            FloatSize::U2 => load_and_format::<f32>(dataset, max_elem, max_width, bump),
        }
    }

    pub(super) fn signed_integer<'alloc>(
        dataset: &H5Dataset,
        int_size: IntSize,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        match int_size {
            IntSize::U8 => load_and_format::<i64>(dataset, max_elem, max_width, bump),
            IntSize::U4 => load_and_format::<i32>(dataset, max_elem, max_width, bump),
            IntSize::U2 => load_and_format::<i16>(dataset, max_elem, max_width, bump),
            IntSize::U1 => load_and_format::<i8>(dataset, max_elem, max_width, bump),
        }
    }

    pub(super) fn unsigned_integer<'alloc>(
        dataset: &H5Dataset,
        int_size: IntSize,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        match int_size {
            IntSize::U8 => load_and_format::<u64>(dataset, max_elem, max_width, bump),
            IntSize::U4 => load_and_format::<u32>(dataset, max_elem, max_width, bump),
            IntSize::U2 => load_and_format::<u16>(dataset, max_elem, max_width, bump),
            IntSize::U1 => load_and_format::<u8>(dataset, max_elem, max_width, bump),
        }
    }

    pub(super) fn bool<'alloc>(
        dataset: &H5Dataset,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        load_and_format::<bool>(dataset, max_elem, max_width, bump)
    }

    // Note that the max_width handling assumes that
    // the formatted array contains no escape sequences.
    fn load_and_format<'alloc, T: H5Type + Display>(
        dataset: &H5Dataset,
        max_elem: Option<usize>,
        max_width: Option<usize>,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        use std::fmt::Write;

        let content = if let Some(max_elem) = max_elem {
            dataset.read_first_n::<T>(max_elem)
        } else {
            dataset.read::<T>().map(PartialData::Full)
        }?;

        let mut out = BumpString::new_in(bump);

        if write!(&mut out, "{}", content.array()).is_err() {
            let _ = write!(&mut out, "<failed write>");
        };
        if matches!(content, PartialData::Full(_)) {
            if let Some(max_width) = max_width {
                if max_width < out.len() {
                    out.truncate(max_width.saturating_sub(4));
                    out.push_str(" ...");
                }
            }
        } else if let Some(max_width) = max_width {
            let padded_width = out.len() + 4; // padded with " ..."
            if max_width >= padded_width {
                out.push_str(" ...");
            } else {
                out.truncate(max_width.saturating_sub(4));
                out.push_str(" ...");
            }
        } else {
            out.push_str(" ...");
        }
        Ok(out)
    }
}

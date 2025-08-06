use bumpalo::{Bump, collections::String as BumpString};
use hdf5::{H5Type, types::TypeDescriptor};
use std::fmt::Display;

use crate::h5::{self, H5Dataset};
use crate::output::Printer;

pub fn load_and_format_data<'alloc>(
    dataset: &H5Dataset,
    printer: &Printer,
    bump: &'alloc Bump,
) -> h5::Result<BumpString<'alloc>> {
    match dataset.type_descriptor()? {
        TypeDescriptor::VarLenUnicode => load_and_format::var_len_unicode(dataset, bump),
        TypeDescriptor::VarLenAscii => load_and_format::var_len_ascii(dataset, bump),
        TypeDescriptor::FixedUnicode(n) => load_and_format::fixed_len_unicode(dataset, n, bump),
        TypeDescriptor::FixedAscii(n) => load_and_format::fixed_len_ascii(dataset, n, bump),
        TypeDescriptor::Float(float_size) => load_and_format::float(dataset, float_size, bump),
        TypeDescriptor::Integer(int_size) => {
            load_and_format::signed_integer(dataset, int_size, bump)
        }
        TypeDescriptor::Unsigned(int_size) => {
            load_and_format::unsigned_integer(dataset, int_size, bump)
        }
        TypeDescriptor::Boolean => load_and_format::bool(dataset, bump),
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
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        load_and_format::<VarLenUnicode>(dataset, bump)
    }

    pub(super) fn var_len_ascii<'alloc>(
        dataset: &H5Dataset,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        load_and_format::<VarLenAscii>(dataset, bump)
    }

    pub(super) fn fixed_len_unicode<'alloc>(
        dataset: &H5Dataset,
        n: usize,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        const MAX_N: usize = 32;
        if n > MAX_N {
            return Err(H5Error::Other(format!(
                "Can only read fixed-length strings of up to {MAX_N} bytes"
            )));
        }
        load_and_format::<FixedUnicode<MAX_N>>(dataset, bump)
    }

    pub(super) fn fixed_len_ascii<'alloc>(
        dataset: &H5Dataset,
        n: usize,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        const MAX_N: usize = 32;
        if n > MAX_N {
            return Err(H5Error::Other(format!(
                "Can only read fixed-length strings of up to {MAX_N} bytes"
            )));
        }
        load_and_format::<FixedAscii<MAX_N>>(dataset, bump)
    }

    pub(super) fn float<'alloc>(
        dataset: &H5Dataset,
        float_size: FloatSize,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        match float_size {
            FloatSize::U8 => load_and_format::<f64>(dataset, bump),
            FloatSize::U4 => load_and_format::<f32>(dataset, bump),
            // f16 is unstable, so approximate using f32
            FloatSize::U2 => load_and_format::<f32>(dataset, bump),
        }
    }

    pub(super) fn signed_integer<'alloc>(
        dataset: &H5Dataset,
        int_size: IntSize,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        match int_size {
            IntSize::U8 => load_and_format::<i64>(dataset, bump),
            IntSize::U4 => load_and_format::<i32>(dataset, bump),
            IntSize::U2 => load_and_format::<i16>(dataset, bump),
            IntSize::U1 => load_and_format::<i8>(dataset, bump),
        }
    }

    pub(super) fn unsigned_integer<'alloc>(
        dataset: &H5Dataset,
        int_size: IntSize,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        match int_size {
            IntSize::U8 => load_and_format::<u64>(dataset, bump),
            IntSize::U4 => load_and_format::<u32>(dataset, bump),
            IntSize::U2 => load_and_format::<u16>(dataset, bump),
            IntSize::U1 => load_and_format::<u8>(dataset, bump),
        }
    }

    pub(super) fn bool<'alloc>(
        dataset: &H5Dataset,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        load_and_format::<bool>(dataset, bump)
    }

    fn load_and_format<'alloc, T: H5Type + Display>(
        dataset: &H5Dataset,
        bump: &'alloc Bump,
    ) -> h5::Result<BumpString<'alloc>> {
        use std::fmt::Write;

        let content = dataset.read::<T>()?;

        let mut out = BumpString::new_in(bump);
        if write!(&mut out, "{content}").is_err() {
            let _ = write!(&mut out, "<failed write>");
        };
        Ok(out)
    }
}

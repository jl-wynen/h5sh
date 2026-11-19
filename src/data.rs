use crate::h5::{self, PartialData};
use crate::output::Printer;
use bumpalo::{Bump, collections::String as BumpString};
use crossterm::{ExecutableCommand, style::Print};
use hdf5::{
    H5Type,
    types::{Reference, TypeDescriptor},
};
use std::fmt::Display;
use std::ops::Deref;

pub fn load_and_format_data<'alloc>(
    container: &impl Deref<Target = hdf5::Container>,
    max_elem: Option<usize>,
    max_width: Option<usize>,
    printer: &Printer,
    bump: &'alloc Bump,
) -> h5::Result<BumpString<'alloc>> {
    let descriptor = container.dtype()?.to_descriptor()?;
    let visitor = load_and_format::LoadAndFormat {
        container,
        max_elem,
        max_width,
        printer,
        bump,
    };
    load_and_format::dispatch(&descriptor, visitor).unwrap_or_else(|| {
        Err(h5::H5Error::Other(format!(
            "dtype not supported: {}",
            printer.format_dtype(&descriptor, bump)
        )))
    })
}

mod load_and_format {
    use super::*;
    use crate::h5::H5Error;
    use std::fmt;

    use hdf5::types::{
        FixedAscii, FixedUnicode, FloatSize, IntSize, VarLenArray, VarLenAscii, VarLenUnicode,
    };
    use ndarray::{IxDyn, s};

    /// Maximum supported length of fixed-length strings.
    const MAX_N: usize = 1024;

    /// An operation that can be performed with a concrete element type.
    pub(super) trait Visitor: Sized {
        type Output;
        fn visit<T: Element>(self) -> Self::Output;
    }

    /// An element type that can be read and rendered.
    pub(super) trait Element: H5Type {
        fn fmt_elem(&self, loc: &hdf5::Location, f: &mut fmt::Formatter<'_>) -> fmt::Result;

        /// Visit `VarLenArray<Self>`.
        /// Returns `None` if `Self` can't be a var-len array element (requires `Copy`).
        fn visit_vlen_array<V: Visitor>(_visitor: V) -> Option<V::Output> {
            None
        }
    }

    macro_rules! display_element_copy {
        ($($t:ty),* $(,)?) => {$(
            impl Element for $t {
                fn fmt_elem(&self, _: &hdf5::Location, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::Display::fmt(self, f)
                }
                fn visit_vlen_array<V: Visitor>(visitor: V) -> Option<V::Output> {
                    Some(visitor.visit::<VarLenArray<Self>>())
                }
            }
        )*};
    }

    macro_rules! display_element_non_copy {
        ( $($t:ty),* $(,)?) => {$(
            impl Element for $t {
                fn fmt_elem(&self, _: &hdf5::Location, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::Display::fmt(self, f)
                }
            }
        )*};
    }

    display_element_copy!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool,
                     FixedAscii<MAX_N>, FixedUnicode<MAX_N>);
    display_element_non_copy!(VarLenAscii, VarLenUnicode);

    impl Element for hdf5::ObjectReference1 {
        fn fmt_elem(&self, loc: &hdf5::Location, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&describe_reference(self, loc))
        }
        fn visit_vlen_array<V: Visitor>(visitor: V) -> Option<V::Output> {
            Some(visitor.visit::<VarLenArray<Self>>())
        }
    }

    impl Element for hdf5::ObjectReference2 {
        fn fmt_elem(&self, loc: &hdf5::Location, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&describe_reference(self, loc))
        }
    }

    // `VarLenArray` is not `Copy`, so the default `visit_vlen` rejects nested var-len arrays.
    impl<T: Element + Copy> Element for VarLenArray<T> {
        fn fmt_elem(&self, loc: &hdf5::Location, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("[")?;
            for (i, x) in self.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                x.fmt_elem(loc, f)?;
            }
            f.write_str("]")
        }
    }

    /// Call `visitor` with the Rust type corresponding to `desc`.
    /// Returns `None` if the type is not supported.
    pub(super) fn dispatch<V: Visitor>(desc: &TypeDescriptor, visitor: V) -> Option<V::Output> {
        match desc {
            TypeDescriptor::VarLenArray(inner) => {
                dispatch_scalar(inner, VarLen(visitor)).flatten()
            }
            _ => dispatch_scalar(desc, visitor),
        }
    }

    fn dispatch_scalar<V: Visitor>(desc: &TypeDescriptor, v: V) -> Option<V::Output> {
        use TypeDescriptor as TD;
        Some(match desc {
            TD::Integer(IntSize::U1) => v.visit::<i8>(),
            TD::Integer(IntSize::U2) => v.visit::<i16>(),
            TD::Integer(IntSize::U4) => v.visit::<i32>(),
            TD::Integer(IntSize::U8) => v.visit::<i64>(),
            TD::Unsigned(IntSize::U1) => v.visit::<u8>(),
            TD::Unsigned(IntSize::U2) => v.visit::<u16>(),
            TD::Unsigned(IntSize::U4) => v.visit::<u32>(),
            TD::Unsigned(IntSize::U8) => v.visit::<u64>(),
            TD::Float(FloatSize::U8) => v.visit::<f64>(),
            // f16 is unstable, so approximate using f32
            TD::Float(FloatSize::U4 | FloatSize::U2) => v.visit::<f32>(),
            TD::Boolean => v.visit::<bool>(),
            TD::FixedAscii(n) if *n <= MAX_N => v.visit::<FixedAscii<MAX_N>>(),
            TD::FixedUnicode(n) if *n <= MAX_N => v.visit::<FixedUnicode<MAX_N>>(),
            TD::VarLenAscii => v.visit::<VarLenAscii>(),
            TD::VarLenUnicode => v.visit::<VarLenUnicode>(),
            // Legacy object reference (hobj_ref_t)
            TD::Reference(Reference::Object) => v.visit::<hdf5::ObjectReference1>(),
            // Modern reference (HDF5 >= 1.12, H5R_ref_t)
            TD::Reference(Reference::Std) => v.visit::<hdf5::ObjectReference2>(),
            // Legacy dataset-region references are not available in hdf5-metno
            _ => return None,
        })
    }

    /// Wraps a visitor to visit `VarLenArray<T>` instead of `T`.
    struct VarLen<V>(V);

    impl<V: Visitor> Visitor for VarLen<V> {
        type Output = Option<V::Output>;
        fn visit<T: Element>(self) -> Self::Output {
            T::visit_vlen_array(self.0)
        }
    }

    pub(super) struct LoadAndFormat<'a, 'alloc, C> {
        pub container: &'a C,
        pub max_elem: Option<usize>,
        pub max_width: Option<usize>,
        pub printer: &'a Printer,
        pub bump: &'alloc Bump,
    }

    impl<'alloc, C: Deref<Target = hdf5::Container>> Visitor for LoadAndFormat<'_, 'alloc, C> {
        type Output = h5::Result<BumpString<'alloc>>;

        fn visit<T: Element>(self) -> Self::Output {
            let content = read::<T>(self.container, self.max_elem)?;
            let loc: &hdf5::Location = self.container;
            let shown = content.map(|value| Shown { value, loc });
            Ok(format_content(&shown, self.max_width, self.printer, self.bump))
        }
    }

    /// Displays an element using its `Element` implementation.
    struct Shown<'a, T> {
        value: &'a T,
        loc: &'a hdf5::Location,
    }

    impl<T: Element> Display for Shown<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.value.fmt_elem(self.loc, f)
        }
    }

    fn read<T: H5Type>(
        container: &impl Deref<Target = hdf5::Container>,
        max_elem: Option<usize>,
    ) -> h5::Result<PartialData<T>> {
        if let Some(max_elem) = max_elem {
            read_first_n::<T>(container, max_elem)
        } else {
            Ok(container.read::<T, IxDyn>().map(PartialData::Full)?)
        }
    }

    fn describe_reference<R: hdf5::ObjectReference + H5Type>(
        r: &R,
        location: &hdf5::Location,
    ) -> String {
        let name = match r.dereference(location) {
            Ok(hdf5::ReferencedObject::Group(g)) => g.name(),
            Ok(hdf5::ReferencedObject::Dataset(d)) => d.name(),
            // Datatype derefs to Object, not Location, so there is no `.name()`
            Ok(hdf5::ReferencedObject::Datatype(d)) => d.to_string(),
            // Null / dangling references fail to dereference
            Err(error) => {
                if ref_is_null(r) {
                    "NULL".to_string()
                } else {
                    format!("<{error}>")
                }
            }
        };
        if name.is_empty() {
            "<anonymous>".to_string()
        } else {
            name
        }
    }

    fn ref_is_null<R: hdf5::ObjectReference + H5Type>(r: &R) -> bool {
        // SAFETY: R is a plain-old-data H5Type (hobj_ref_t / H5R_ref_t) with no padding
        // See test references_are_compatible_with_c_layout
        let bytes =
            unsafe { std::slice::from_raw_parts((r as *const R).cast::<u8>(), size_of::<R>()) };
        // Check for 0xff to handle legacy refs
        bytes.iter().all(|&b| b == 0) || bytes.iter().all(|&b| b == 0xff)
    }

    // Note that the max_width handling assumes that
    // the formatted array contains no escape sequences.
    fn format_content<'alloc, T: Display>(
        content: &PartialData<T>,
        max_width: Option<usize>,
        printer: &Printer,
        bump: &'alloc Bump,
    ) -> BumpString<'alloc> {
        use std::fmt::Write;

        let mut out = BumpString::new_in(bump);
        if write!(&mut out, "{}", content.array()).is_err() {
            let _ = write!(&mut out, "<failed write>");
        };
        if matches!(content, PartialData::Full(_)) {
            if let Some(max_width) = max_width
                && max_width < out.len()
            {
                out.truncate(max_width.saturating_sub(4));
                out.push_str(&trailing_ellipses(printer));
            }
        } else {
            out.truncate(out.len().saturating_sub(1)); // remove final ']'
            if let Some(max_width) = max_width {
                let padded_width = out.len() + 4;
                if max_width < padded_width {
                    out.truncate(max_width.saturating_sub(4));
                }
            }
            out.push_str(&trailing_ellipses(printer));
        }
        out
    }

    fn trailing_ellipses(printer: &Printer) -> String {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.execute(Print(" ")).unwrap();
        buffer.execute(&printer.style().placeholder).unwrap();
        buffer.execute(Print("...")).unwrap();
        buffer.execute(printer.style().reset()).unwrap();
        String::from_utf8(buffer).unwrap()
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

#[cfg(test)]
mod tests {
    #[test]
    fn references_are_compatible_with_c_layout() {
        use hdf5::{ObjectReference1, ObjectReference2};
        use hdf5_metno_sys::h5r::{H5R_ref_t, hobj_ref_t};

        assert_eq!(size_of::<ObjectReference1>(), size_of::<hobj_ref_t>());
        assert_eq!(align_of::<ObjectReference1>(), align_of::<hobj_ref_t>());
        assert_eq!(size_of::<ObjectReference2>(), size_of::<H5R_ref_t>());
        assert_eq!(align_of::<ObjectReference2>(), align_of::<H5R_ref_t>());
    }
}

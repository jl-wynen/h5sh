# /// script
# requires-python = ">=3.12"
# dependencies = [
#   "h5py",
#   "numpy",
# ]
# ///
"""
Generate test HDF5 files.
"""

from pathlib import Path

import h5py as h5
import numpy as np

DATA_DIR = Path(__file__).resolve().parent.parent.joinpath("tests", "data")


def main() -> None:
    with h5.File(DATA_DIR / "test.h5", "w") as f:
        base = f.create_group("base")

        utf8 = base.create_dataset("label-utf8", (), dtype=h5.string_dtype())
        utf8[()] = "This is a UTF-8 dataset"
        utf8.attrs["testo1"] = "test attribute 1"
        utf8.attrs["testo2"] = "another attribute"
        fixed_ascii = base.create_dataset(
            "short", (), dtype=h5.string_dtype("ascii", length=6)
        )
        fixed_ascii[()] = "shorty"
        base.create_dataset("long_array", data=np.arange(1030, dtype="float64"))

        base.create_group("g_empty")

        subgroup = base.create_group("sub-group")
        subgroup.attrs["class"] = "TestGroup"
        subgroup.attrs["array"] = np.array([1, 2, 5, 6], dtype=np.int64)
        subgroup.attrs.create("ascii", "English only", dtype=h5.string_dtype("ascii"))

        nested_ds = subgroup.create_dataset("nested_ds", data=np.array([-1, 10]))
        nested_ds.attrs["testo"] = "nested ds"

        booleans = base.create_group("booleans")
        booleans.create_dataset("all_true", data=np.array([True] * 3))
        booleans.create_dataset("all_false", data=np.array([False] * 4))
        booleans.create_dataset(
            "mixed", data=np.array([True, False, False, True, True])
        )

        arrays = base.create_group("arrays")
        f32_array = arrays.create_dataset("all_normal_f32", data=np.array([10.5, -0.2, 4.44], dtype="float32"))
        arrays.create_dataset("some_nan_inf", data=np.array([[1.0, np.nan], [np.inf, -np.inf], [2.1, 4.1]]))
        arrays.create_dataset("all_inf", data=np.array([-np.inf, np.inf, -np.inf]))
        arrays.create_dataset("ints", data=np.array([-6, -28, 8, 0]))

        base.create_dataset("single_ref", data=np.array(f32_array.ref, dtype=h5.ref_dtype))
        base.create_dataset("multi_ref", data=np.array([utf8.ref, booleans.ref, f32_array.ref], dtype=h5.ref_dtype))

        bad_refs_group = f.create_group("bad_refs")
        ds = bad_refs_group.create_dataset("ds", shape=(1,), dtype="int32")
        bad_refs_group.create_dataset("dangling", data=np.array(ds.ref, dtype=h5.ref_dtype))
        del bad_refs_group["ds"]  # make ref dangling
        null = bad_refs_group.create_dataset("null", shape=(2,), dtype=h5.ref_dtype)
        null[1] = f32_array.ref

        dim_scales = f.create_group("dim_scales")
        x = dim_scales.create_dataset("x", data=np.arange(3))
        x.make_scale("xx")
        x2 = dim_scales.create_dataset("x2", data=2*np.arange(3))
        x2.make_scale()
        y = dim_scales.create_dataset("y", data=-np.arange(2))
        y.make_scale("Y")
        d1 = dim_scales.create_dataset("d1", data=np.full((3, 2), 10))
        d1.dims[1].label = "ly"
        d1.dims[0].attach_scale(x)
        d1.dims[1].attach_scale(y)
        d1.dims[0].attach_scale(x2)
        d2 = dim_scales.create_dataset("d2", data=np.full(2, -5))
        d2.dims[0].attach_scale(x)


if __name__ == "__main__":
    main()

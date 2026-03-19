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
        fixed_ascii = base.create_dataset("short", (), dtype=h5.string_dtype('ascii', length=6))
        fixed_ascii[()] = "shorty"
        base.create_dataset("long_array", data=np.arange(1030, dtype='float64'))

        base.create_group("g_empty")

        subgroup = base.create_group("sub-group")
        subgroup.attrs["class"] = "TestGroup"
        subgroup.attrs["array"] = np.array([1, 2, 5, 6], dtype=np.int64)
        subgroup.attrs.create("ascii", "English only", dtype=h5.string_dtype('ascii'))

        nested_ds = subgroup.create_dataset("nested_ds", data=np.array([-1, 10]))
        nested_ds.attrs["testo"] = "nested ds"


if __name__ == "__main__":
    main()

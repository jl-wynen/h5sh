# /// script
# requires-python = ">=3.12"
# dependencies = [
#   "h5py",
# ]
# ///
"""
Generate test HDF5 files.
"""

from pathlib import Path
import h5py as h5

DATA_DIR = Path(__file__).resolve().parent.parent.joinpath("tests", "data")


def main() -> None:
    with h5.File(DATA_DIR / "test.h5", "w") as f:
        base = f.create_group("base")

        utf8 = base.create_dataset("label-utf8", (), dtype=h5.string_dtype())
        utf8[()] = "This is a UTF-8 dataset"

        subgroup = base.create_group("sub-group")
        base.create_group("g_empty")


if __name__ == "__main__":
    main()

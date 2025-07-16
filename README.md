# h5sh - Interactive shell for HDF5 files

## Installation

1. Download the binary for you platform from the GitHub [releases](https://github.com/jl-wynen/h5sh/releases/latest).
2. (Optionally) download the corresponding checksum file
   - Validate with `sha256sum -c h5sh-*.sha256` (Unix)
3. Copy the binary to a location on your PATH.

## Usage

Open a file:
```bash
$ h5sh path/to/file.hdf5
```
Then navigate the file like any other POSIX shell with `cd`, `ls`, and `pwd`.

Exit she shell using the `exit` command or by pressing Ctrl+D.

## Getting help

Use the `help` command in the shell to get a list of all available commands.
And use the `--help` flag on a command to get detailed help about that command.

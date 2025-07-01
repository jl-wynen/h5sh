mod h5;
mod line_editor;

use line_editor::{LineEditor, Poll};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    // let fname = PathBuf::from("data/977695_00069191.hdf");
    // let file = h5::H5File::open(fname).unwrap();
    // let entry = file.load(&h5::H5Path::from("entry".to_string())).unwrap();
    // dbg!(&entry);

    let mut editor = LineEditor::new().unwrap();
    let mut exit_code = ExitCode::SUCCESS;
    loop {
        match editor.poll() {
            Poll::Cmd(input) => {
                println!("CMD: '{input}'");
            }
            Poll::Skip => {}
            Poll::Exit => break,
            Poll::Error(err) => {
                println!("ERROR {err}");
            }
        }
    }

    editor.save_history().unwrap();
    exit_code
}

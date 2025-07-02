use std::rc::Rc;

use crate::cmd::{self, Command};
use crate::h5::H5Path;
use crate::line_editor::LineEditor;
use crate::output::Printer;

pub struct Shell {
    working_dir: H5Path,
    printer: Printer,
    commands: cmd::CommandMap,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            working_dir: H5Path::root(),
            printer: Printer::new(),
            commands: cmd::commands(),
        }
    }

    pub fn printer(&self) -> &Printer {
        &self.printer
    }

    pub fn commands(&self) -> &cmd::CommandMap {
        &self.commands
    }

    pub fn get_command(&self, name: &str) -> Option<&dyn Command> {
        self.commands.get(name).map(|c| c.as_ref())
    }

    pub fn get_working_dir(&self) -> &H5Path {
        &self.working_dir
    }

    pub fn set_working_dir(&mut self, path: H5Path) {
        self.working_dir = path;
    }

    pub fn resolve_path(&self, path: &H5Path) -> H5Path {
        self.working_dir.join(path).resolve()
    }

    pub fn start_editor(&self) -> rustyline::Result<LineEditor> {
        LineEditor::new()
    }
}

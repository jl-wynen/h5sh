use super::{Editor, Poll};
use crate::h5::{H5File, H5Path};
use crate::shell::Shell;

pub struct BatchEditor {
    commands: Vec<String>,
    current_index: usize,
}

impl BatchEditor {
    pub fn new(commands: Vec<String>) -> Self {
        Self {
            commands,
            current_index: 0,
        }
    }
}

impl<'f> Editor<'f> for BatchEditor {
    fn poll(&mut self, _: &Shell, _: &H5File) -> Poll {
        let result = self
            .commands
            .get(self.current_index)
            .map_or(Poll::Exit, |cmd| {
                if cmd.is_empty() {
                    Poll::Error("No command provided".to_string())
                } else {
                    Poll::Cmd(cmd.clone())
                }
            });
        self.current_index += 1;
        result
    }

    fn set_working_group(&mut self, _: H5Path) {
        // nothing to do (this editor does not consult the CWD)
    }
}

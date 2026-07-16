use crate::h5::{H5File, H5Path};
use crate::shell::Shell;

pub trait Editor<'f> {
    fn poll(&mut self, shell: &Shell, h5file: &'f H5File) -> Poll;
    fn set_working_group(&mut self, group: H5Path);
}

#[derive(Debug)]
pub enum Poll {
    Cmd(String),
    Error(String),
    Skip,
    Exit,
}

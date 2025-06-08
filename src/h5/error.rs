use super::H5Path;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum H5Error {
    NotFound(H5Path),
    Other(String),
}

pub type Result<T> = std::result::Result<T, H5Error>;

impl Display for H5Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            H5Error::NotFound(name) => write!(f, "Not found: '{}'", name),
            H5Error::Other(msg) => write!(f, "HDF5 error: '{}'", msg),
        }
    }
}

impl From<hdf5::Error> for H5Error {
    fn from(error: hdf5::Error) -> Self {
        match error {
            hdf5::Error::Internal(message) => H5Error::Other(message),
            hdf5::Error::HDF5(stack) => {
                let Ok(stack) = stack.expand() else {
                    return H5Error::Other("Unable to get error stack".to_string());
                };
                let Some(last) = stack.last() else {
                    return H5Error::Other(stack.detail().unwrap_or("?".to_string()));
                };
                if last.desc().contains("doesn't exist") {
                    H5Error::NotFound(String::new().into()) // needs to be filled in later
                } else {
                    H5Error::Other(stack.detail().unwrap_or("?".to_string()))
                }
            }
        }
    }
}

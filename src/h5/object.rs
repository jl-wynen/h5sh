use crate::h5::Result;
use crate::h5::{H5Error, H5Path};

#[derive(Clone, Debug)]
pub struct H5Dataset {
    path: H5Path,
}

#[derive(Clone, Debug)]
pub struct H5Group {
    path: H5Path,
}

#[derive(Clone, Debug)]
pub enum H5Object {
    Dataset(H5Dataset),
    Group(H5Group),
}

impl H5Dataset {
    pub fn path(&self) -> &H5Path {
        &self.path
    }
}

impl H5Group {
    pub fn path(&self) -> &H5Path {
        &self.path
    }
}

impl H5Object {
    pub fn from_location(path: H5Path, location: &hdf5::Location) -> Result<Self> {
        if let Ok(group) = location.as_group() {
            Ok(H5Group { path }.into())
        } else if let Ok(dataset) = location.as_dataset() {
            Ok(H5Object::Dataset(H5Dataset { path }))
        } else {
            Err(H5Error::Other(
                "Location does not contain a supported H6Object".to_string(),
            ))
        }
    }
}

impl From<H5Dataset> for H5Object {
    fn from(dataset: H5Dataset) -> Self {
        H5Object::Dataset(dataset)
    }
}

impl From<H5Group> for H5Object {
    fn from(group: H5Group) -> Self {
        H5Object::Group(group)
    }
}

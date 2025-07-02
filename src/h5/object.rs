use hdf5::LocationType;

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
        match location.loc_type() {
            Ok(LocationType::Group) => Ok(H5Group { path }.into()),
            Ok(LocationType::Dataset) => Ok(H5Object::Dataset(H5Dataset { path })),
            Ok(_) => Err(H5Error::Other("Unsupported location type".to_string())),
            Err(e) => Err(H5Error::Other(format!(
                "Unable to determine location type: {}",
                e.to_string()
            ))),
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

use hdf5::LocationType;

use crate::h5::Result;
use crate::h5::{H5Error, H5Path};

#[derive(Clone, Debug)]
pub struct H5Dataset {
    path: H5Path,
    dataset: hdf5::Dataset,
}

#[derive(Clone, Debug)]
pub struct H5Group {
    path: H5Path,
    group: hdf5::Group,
}

#[derive(Clone, Debug)]
pub enum H5Object {
    Dataset(H5Dataset),
    Group(H5Group),
}

impl H5Dataset {
    pub fn from_underlying(underlying: hdf5::Dataset) -> Self {
        Self {
            path: underlying.name().into(),
            dataset: underlying,
        }
    }

    pub fn underlying(&self) -> &hdf5::Dataset {
        &self.dataset
    }

    pub fn path(&self) -> &H5Path {
        &self.path
    }

    pub fn location_info(&self) -> hdf5::Result<hdf5::LocationInfo> {
        self.underlying().loc_info()
    }
}

impl H5Group {
    pub fn from_underlying(underlying: hdf5::Group) -> Self {
        Self {
            path: underlying.name().into(),
            group: underlying,
        }
    }

    pub fn underlying(&self) -> &hdf5::Group {
        &self.group
    }

    pub fn path(&self) -> &H5Path {
        &self.path
    }

    pub fn location_info(&self) -> hdf5::Result<hdf5::LocationInfo> {
        self.underlying().loc_info()
    }
}

impl H5Object {
    pub fn from_location(path: H5Path, location: &hdf5::Location) -> Result<Self> {
        match location.loc_type() {
            Ok(LocationType::Group) => Ok(H5Group {
                path,
                group: location.as_group()?,
            }
            .into()),
            Ok(LocationType::Dataset) => Ok(H5Object::Dataset(H5Dataset {
                path,
                dataset: location.as_dataset()?,
            })),
            Ok(_) => Err(H5Error::Other("Unsupported location type".to_string())),
            Err(e) => Err(H5Error::Other(format!(
                "Unable to determine location type: {e}"
            ))),
        }
    }

    pub fn path(&self) -> &H5Path {
        match self {
            H5Object::Dataset(dataset) => dataset.path(),
            H5Object::Group(group) => group.path(),
        }
    }

    pub fn location_info(&self) -> hdf5::Result<hdf5::LocationInfo> {
        match self {
            H5Object::Dataset(dataset) => dataset.location_info(),
            H5Object::Group(group) => group.location_info(),
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

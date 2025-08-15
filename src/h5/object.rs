use hdf5::{H5Type, LocationType};
use ndarray::{Array, IxDyn, s};

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

    pub fn type_descriptor(&self) -> Result<hdf5::types::TypeDescriptor> {
        Ok(self.underlying().dtype()?.to_descriptor()?)
    }

    pub fn ndim(&self) -> usize {
        self.underlying().ndim()
    }

    pub fn read<T: H5Type>(&self) -> Result<Array<T, IxDyn>> {
        Ok(self.underlying().read()?)
    }

    pub fn read_first_n<T: H5Type>(&self, n: usize) -> Result<Array<T, IxDyn>> {
        match self.underlying().shape()[..] {
            [] => self.read(),
            [size] => Ok(self.underlying().read_slice(s![..(n.min(size))])?),
            _ => Err(H5Error::Other(
                "Reading first n elements is only supported for scalar and 1d data.".to_string(),
            )),
        }
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

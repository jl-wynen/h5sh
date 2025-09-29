use crate::h5::{H5Error, H5Path, Result};
use hdf5::{LocationInfo, LocationType};
use ndarray::{Array, IxDyn};
use std::ops::Deref;

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
pub struct H5Attribute {
    attribute: hdf5::Attribute,
}

#[derive(Clone, Debug)]
pub enum H5Object {
    Dataset(H5Dataset),
    Group(H5Group),
    Attribute(H5Attribute),
}

#[derive(Clone, Debug)]
pub enum PartialData<T> {
    Full(Array<T, IxDyn>),
    FirstN(Array<T, IxDyn>),
}

impl H5Dataset {
    pub fn from_underlying_with_path(underlying: hdf5::Dataset, path: H5Path) -> Self {
        Self {
            path,
            dataset: underlying,
        }
    }

    pub fn underlying(&self) -> &hdf5::Dataset {
        &self.dataset
    }

    pub fn path(&self) -> &H5Path {
        &self.path
    }

    pub fn location_info(&self) -> hdf5::Result<LocationInfo> {
        self.underlying().loc_info()
    }

    pub fn attr_names(&self) -> Result<Vec<String>> {
        Ok(self.underlying().attr_names()?)
    }

    pub fn attr(&self, name: &str) -> Result<H5Attribute> {
        Ok(H5Attribute::from_underlying(self.underlying().attr(name)?))
    }
}

impl H5Group {
    pub fn from_underlying_with_path(underlying: hdf5::Group, path: H5Path) -> Self {
        Self {
            path,
            group: underlying,
        }
    }

    pub fn underlying(&self) -> &hdf5::Group {
        &self.group
    }

    pub fn path(&self) -> &H5Path {
        &self.path
    }

    pub fn location_info(&self) -> hdf5::Result<LocationInfo> {
        self.underlying().loc_info()
    }

    pub fn attr_names(&self) -> Result<Vec<String>> {
        Ok(self.underlying().attr_names()?)
    }

    pub fn attr(&self, name: &str) -> Result<H5Attribute> {
        Ok(H5Attribute::from_underlying(self.underlying().attr(name)?))
    }

    pub fn load_children(&self) -> Result<Vec<H5Object>> {
        fn load_child(parent: &hdf5::Group, name: &str) -> Result<H5Object> {
            match parent.loc_type_by_name(name)? {
                LocationType::Dataset => Ok(H5Dataset::from_underlying_with_path(
                    parent.dataset(name)?,
                    name.into(),
                )
                .into()),
                LocationType::Group => {
                    Ok(H5Group::from_underlying_with_path(parent.group(name)?, name.into()).into())
                }
                _ => Err(H5Error::Other("unsupported location type: ".into())),
            }
        }

        Ok(self
            .underlying()
            .iter_visit_default(Vec::new(), |group, name, _, children| {
                if let Ok(child) = load_child(group, name) {
                    children.push(child);
                }
                // Skip errors. We have no good way of reporting errors here.
                true
            })?)
    }

    pub fn load_child_locations(&self) -> Result<Vec<(H5Path, LocationInfo)>> {
        Ok(self
            .underlying()
            .iter_visit_default(Vec::new(), |group, name, _, children| {
                if let Ok(info) = group.loc_info_by_name(name) {
                    children.push((self.path.join(&H5Path::from(name)), info));
                }
                // Skip errors. We have no good way of reporting errors here.
                true
            })?)
    }
}

impl H5Attribute {
    pub fn from_underlying(underlying: hdf5::Attribute) -> Self {
        Self {
            attribute: underlying,
        }
    }

    pub fn underlying(&self) -> &hdf5::Attribute {
        &self.attribute
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
            H5Object::Attribute(_) => todo!("path"),
        }
    }

    pub fn location_info(&self) -> hdf5::Result<hdf5::LocationInfo> {
        match self {
            H5Object::Dataset(dataset) => dataset.location_info(),
            H5Object::Group(group) => group.location_info(),
            H5Object::Attribute(_) => todo!("location info"),
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

impl From<H5Attribute> for H5Object {
    fn from(attr: H5Attribute) -> Self {
        H5Object::Attribute(attr)
    }
}

impl Deref for H5Dataset {
    type Target = hdf5::Container;

    fn deref(&self) -> &Self::Target {
        self.underlying()
    }
}

impl Deref for H5Attribute {
    type Target = hdf5::Container;

    fn deref(&self) -> &Self::Target {
        self.underlying()
    }
}

impl<T> PartialData<T> {
    pub fn array(&self) -> &Array<T, IxDyn> {
        match self {
            PartialData::Full(array) => array,
            PartialData::FirstN(array) => array,
        }
    }
}

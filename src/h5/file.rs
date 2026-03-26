use super::error::{H5Error, Result};
use super::object::H5Object;
use super::path::H5Path;
use crate::h5::ObjectPath;
use log::info;
use std::path::PathBuf;

#[derive(Debug)]
pub struct H5File {
    file: hdf5::File,
}

impl H5File {
    pub fn open(path: PathBuf) -> std::io::Result<Self> {
        info!("Opening file: {:}", path.display());
        let file = hdf5::File::open(path)?;
        Ok(Self { file })
    }

    pub fn filename(&self) -> String {
        self.file.filename()
    }

    pub fn load<L: LocationSpec>(&self, location: L) -> Result<H5Object> {
        let location = location.into_location(&self.file)?;
        H5Object::from_location(H5Path::from(location.name()), &location)
    }

    pub fn load_object(&self, path: &ObjectPath) -> Result<H5Object> {
        match self.load(&path.location_path)? {
            H5Object::Group(group) => match &path.attr_name {
                Some(attr_name) => group.attr(attr_name).map(H5Object::Attribute),
                None => Ok(H5Object::Group(group)),
            },
            H5Object::Dataset(dataset) => match &path.attr_name {
                Some(attr_name) => dataset.attr(attr_name).map(H5Object::Attribute),
                None => Ok(H5Object::Dataset(dataset)),
            },
            H5Object::Attribute(_) => {
                // This should never happen.
                Err(H5Error::Other("Did not expect an attribute".to_string()))
            }
        }
    }
}

pub trait LocationSpec {
    fn into_location(self, file: &hdf5::File) -> Result<hdf5::Location>;
}

impl LocationSpec for hdf5::LocationToken {
    fn into_location(self, file: &hdf5::File) -> Result<hdf5::Location> {
        Ok(file.open_by_token(self)?)
    }
}

impl LocationSpec for &H5Path {
    fn into_location(self, file: &hdf5::File) -> Result<hdf5::Location> {
        match file.loc_info_by_name(self.as_raw()).map_err(|e| e.into()) {
            Ok(location_info) => location_info.token.into_location(file),
            Err(H5Error::NotFound(_)) => Err(H5Error::NotFound(self.clone())),
            Err(err) => Err(err),
        }
    }
}

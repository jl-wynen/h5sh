use std::path::PathBuf;

use super::error::{H5Error, Result};
use super::object::{H5Dataset, H5Group, H5Object};
use super::path::H5Path;

#[derive(Debug)]
pub struct H5File {
    file: hdf5::File,
}

impl H5File {
    pub fn open(path: PathBuf) -> std::io::Result<Self> {
        let file = hdf5::File::open(path)?;
        Ok(Self { file })
    }

    pub fn load<L: LocationSpec>(&self, location: L) -> Result<H5Object> {
        let location = location.into_location(&self.file)?;
        H5Object::from_location(H5Path::from(location.name()), &location)
    }

    pub fn load_children(&self, group: H5Group) -> Result<impl Iterator<Item = H5Object>> {
        let group = group.underlying();
        let groups = group
            .groups()?
            .into_iter()
            .map(|group| H5Group::from_underlying(group).into());
        let datasets = group
            .datasets()?
            .into_iter()
            .map(|dataset| H5Dataset::from_underlying(dataset).into());
        Ok(groups.chain(datasets))
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

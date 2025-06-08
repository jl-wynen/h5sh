pub(crate) struct H5FileCache {}

#[derive(Debug)]
pub struct H5Object {
    location_info: hdf5::LocationInfo,
}

impl H5Object {
    pub fn from_location_info(location_info: hdf5::LocationInfo) -> Self {
        Self { location_info }
    }

    pub(crate) fn location_token(&self) -> &hdf5::LocationToken {
        &self.location_info.token
    }
}

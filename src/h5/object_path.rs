use crate::h5::H5Path;
use std::fmt::Display;

/// A path to a HDF5 object.
///
/// This can be a location (`attr_path == None`)
/// or an attribute (`attr_path != None`).
///
/// Object paths are encoded as
/// `location_path@attr_path`.
#[derive(Clone, Debug)]
pub struct ObjectPath {
    pub location_path: H5Path,
    pub attr_name: Option<String>,
}

impl From<String> for ObjectPath {
    fn from(value: String) -> Self {
        match value.split_once('@') {
            Some((a, b)) => Self {
                location_path: a.into(),
                attr_name: Some(b.into()),
            },
            None => Self {
                location_path: value.into(),
                attr_name: None,
            },
        }
    }
}

impl Display for ObjectPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.attr_name {
            Some(attr) => write!(f, "{}@{}", self.location_path, attr),
            None => write!(f, "{}", self.location_path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_path_from_string_no_attr() {
        let raw = "a/path".to_string();
        let path = ObjectPath::from(raw);
        assert_eq!(path.location_path, H5Path::from("a/path"));
        assert_eq!(path.attr_name, None);
    }

    #[test]
    fn object_path_from_string_empty() {
        let raw = "".to_string();
        let path = ObjectPath::from(raw);
        assert_eq!(path.location_path, H5Path::from(""));
        assert_eq!(path.attr_name, None);
    }

    #[test]
    fn object_path_from_string_only_attr() {
        let raw = "@my_attr".to_string();
        let path = ObjectPath::from(raw);
        assert_eq!(path.location_path, H5Path::from(""));
        assert_eq!(path.attr_name, Some("my_attr".to_string()));
    }

    #[test]
    fn object_path_from_string_only_attr_empty() {
        let raw = "@".to_string();
        let path = ObjectPath::from(raw);
        assert_eq!(path.location_path, H5Path::from(""));
        assert_eq!(path.attr_name, Some("".to_string()));
    }

    #[test]
    fn object_path_from_string_full() {
        let raw = "long/base/path@an_attribute".to_string();
        let path = ObjectPath::from(raw);
        assert_eq!(path.location_path, H5Path::from("long/base/path"));
        assert_eq!(path.attr_name, Some("an_attribute".to_string()));
    }

    #[test]
    fn object_path_from_string_full_trailing_slash() {
        let raw = "long/base/@an_attribute".to_string();
        let path = ObjectPath::from(raw);
        assert_eq!(path.location_path, H5Path::from("long/base/"));
        assert_eq!(path.attr_name, Some("an_attribute".to_string()));
    }
}

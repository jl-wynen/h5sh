use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct H5Path {
    raw: String,
}

impl H5Path {
    pub fn root() -> Self {
        Self { raw: "/".into() }
    }

    pub fn is_absolute(&self) -> bool {
        self.raw.starts_with("/")
    }

    pub fn join(&self, other: &H5Path) -> H5Path {
        if other.is_absolute() {
            other.clone()
        } else if self.raw.ends_with("/") {
            Self {
                raw: format!("{}{}", self.raw, other.raw),
            }
        } else {
            Self {
                raw: format!("{}/{}", self.raw, other.raw),
            }
        }
    }

    pub fn name(&self) -> &str {
        let raw = self.raw.trim_end_matches('/');
        raw.rsplit_once('/').map_or(raw, |(_, name)| name)
    }

    pub fn split_parent(&self) -> (Self, &str) {
        if self.raw == "/" {
            (self.clone(), "")
        } else {
            self.raw.rsplit_once('/').map_or_else(
                || (Self::root(), ""),
                |(parent, name)| (Self::from(parent.to_string()), name),
            )
        }
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.raw.split('/')
    }

    pub fn normalized(&self) -> Self {
        Self {
            raw: self.raw.trim_end_matches('/').to_string(),
        }
    }

    pub fn as_raw(&self) -> &str {
        &self.raw
    }
}

impl From<String> for H5Path {
    fn from(value: String) -> Self {
        Self { raw: value }
    }
}

impl From<&str> for H5Path {
    fn from(value: &str) -> Self {
        Self {
            raw: value.to_string(),
        }
    }
}

impl Display for H5Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn from_empty_string() {
        let path = H5Path::from("");
        let expected = "".to_string();
        assert_eq!(path.to_string(), expected);
    }

    #[test]
    fn from_some_string() {
        let path = H5Path::from("/base/group/ds");
        let expected = "/base/group/ds".to_string();
        assert_eq!(path.to_string(), expected);
    }

    #[test]
    fn from_some_string_preserves_slashes() {
        let path = H5Path::from("/base//ds//");
        let expected = "/base//ds//".to_string();
        assert_eq!(path.to_string(), expected);
    }

    #[test]
    fn is_absolute_root() {
        let path = H5Path::root();
        assert!(path.is_absolute());
    }

    #[test]
    fn is_absolute_true() {
        let path = H5Path::from("/base/group/ds");
        assert!(path.is_absolute());
    }

    #[test]
    fn is_absolute_empty() {
        let path = H5Path::from("");
        assert!(!path.is_absolute());
    }

    #[test]
    fn is_absolute_false() {
        let path = H5Path::from("group/ds");
        assert!(!path.is_absolute());
    }

    #[test]
    fn join_absolute_relative() {
        let a = H5Path::from("/a/b");
        let b = H5Path::from("c/d");
        let expected = H5Path::from("/a/b/c/d");
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn join_absolute_absolute() {
        let a = H5Path::from("/a/b");
        let b = H5Path::from("/c/d");
        let expected = H5Path::from("/c/d");
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn join_relative_absolute() {
        let a = H5Path::from("a/b");
        let b = H5Path::from("/c/d");
        let expected = H5Path::from("/c/d");
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn join_relative_relative() {
        let a = H5Path::from("a/b");
        let b = H5Path::from("c/d");
        let expected = H5Path::from("a/b/c/d");
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn join_root_relative() {
        let a = H5Path::root();
        let b = H5Path::from("c/d");
        let expected = H5Path::from("/c/d");
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn join_root_absolute() {
        let a = H5Path::root();
        let b = H5Path::from("/c/d");
        let expected = H5Path::from("/c/d");
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn name_root() {
        let path = H5Path::root();
        assert_eq!(path.name(), "");
    }

    #[test]
    fn name_single() {
        let path = H5Path::from("object");
        assert_eq!(path.name(), "object");
    }

    #[test]
    fn name_two_segments() {
        let path = H5Path::from("group/ds");
        assert_eq!(path.name(), "ds");
    }

    #[test]
    fn name_trailing_slash() {
        let path = H5Path::from("group/object/");
        assert_eq!(path.name(), "object");
    }
}

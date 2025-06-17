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
        self.raw
            .rsplit_once('/')
            .map_or_else(|| self.raw.as_str(), |(_, name)| name)
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.raw.split('/')
    }

    pub fn as_raw(&self) -> &str {
        &self.raw
    }
}

impl From<String> for H5Path {
    fn from(value: String) -> Self {
        if let Some(mut stripped) = value.strip_suffix('/') {
            while let Some(s) = stripped.strip_suffix('/') {
                stripped = s;
            }
            Self {
                raw: stripped.into(),
            }
        } else {
            Self { raw: value }
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
        let path = H5Path::from("".to_string());
        let expected = "".to_string();
        assert_eq!(path.to_string(), expected);
    }

    #[test]
    fn from_some_string() {
        let path = H5Path::from("/base/group/ds".to_string());
        let expected = "/base/group/ds".to_string();
        assert_eq!(path.to_string(), expected);
    }

    #[test]
    fn from_some_string_strips_slash_suffix() {
        let path = H5Path::from("/base//ds//".to_string());
        let expected = "/base//ds".to_string();
        assert_eq!(path.to_string(), expected);
    }

    #[test]
    fn is_absolute_root() {
        let path = H5Path::root();
        assert!(path.is_absolute());
    }

    #[test]
    fn is_absolute_true() {
        let path = H5Path::from("/base/group/ds".to_string());
        assert!(path.is_absolute());
    }

    #[test]
    fn is_absolute_empty() {
        let path = H5Path::from("".to_string());
        assert!(!path.is_absolute());
    }

    #[test]
    fn is_absolute_false() {
        let path = H5Path::from("group/ds".to_string());
        assert!(!path.is_absolute());
    }

    #[test]
    fn join_absolute_relative() {
        let a = H5Path::from("/a/b".to_string());
        let b = H5Path::from("c/d".to_string());
        let expected = H5Path::from("/a/b/c/d".to_string());
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn join_absolute_absolute() {
        let a = H5Path::from("/a/b".to_string());
        let b = H5Path::from("/c/d".to_string());
        let expected = H5Path::from("/c/d".to_string());
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn join_relative_absolute() {
        let a = H5Path::from("a/b".to_string());
        let b = H5Path::from("/c/d".to_string());
        let expected = H5Path::from("/c/d".to_string());
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn join_relative_relative() {
        let a = H5Path::from("a/b".to_string());
        let b = H5Path::from("c/d".to_string());
        let expected = H5Path::from("a/b/c/d".to_string());
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn join_root_relative() {
        let a = H5Path::root();
        let b = H5Path::from("c/d".to_string());
        let expected = H5Path::from("/c/d".to_string());
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn join_root_absolute() {
        let a = H5Path::root();
        let b = H5Path::from("/c/d".to_string());
        let expected = H5Path::from("/c/d".to_string());
        assert_eq!(a.join(&b), expected);
    }

    #[test]
    fn name_root() {
        let path = H5Path::root();
        assert_eq!(path.name(), "");
    }

    #[test]
    fn name_single() {
        let path = H5Path::from("object".to_string());
        assert_eq!(path.name(), "object");
    }

    #[test]
    fn name_two_segments() {
        let path = H5Path::from("group/ds".to_string());
        assert_eq!(path.name(), "ds");
    }

    #[test]
    fn name_trailing_slash() {
        let path = H5Path::from("group/object/".to_string());
        assert_eq!(path.name(), "object");
    }
}

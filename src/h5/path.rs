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

    pub fn push(&mut self, segment: &str) {
        if !self.raw.ends_with('/') {
            self.raw.push('/');
        }
        self.raw.push_str(segment);
    }

    pub fn name(&self) -> &str {
        let raw = self.raw.trim_end_matches('/');
        raw.rsplit_once('/').map_or(raw, |(_, name)| name)
    }

    pub fn parent(&self) -> Self {
        self.raw.rsplit_once('/').map_or_else(
            || Self::from(""),
            |(p, _)| {
                if p.is_empty() && self.is_absolute() {
                    Self::root()
                } else {
                    Self::from(p)
                }
            },
        )
    }

    pub fn split_parent(&self) -> (Self, &str) {
        self.raw.rsplit_once('/').map_or_else(
            || (Self::from(""), self.as_raw()),
            |(parent, name)| {
                if parent.is_empty() && self.is_absolute() {
                    (Self::root(), name)
                } else {
                    (Self::from(parent.to_string()), name)
                }
            },
        )
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.raw.split('/').filter(|s| !s.is_empty())
    }

    pub fn normalized(&self) -> Self {
        if self.raw == "/" {
            self.clone()
        } else {
            Self {
                raw: self.raw.trim_end_matches('/').to_string(),
            }
        }
    }

    pub fn resolve(&self) -> Self {
        let mut segments = Vec::with_capacity(2);
        for segment in self.segments() {
            if segment == ".." {
                segments.pop();
            } else if !segment.is_empty() {
                segments.push(segment);
            }
        }
        let mut new = segments.join("/");
        if self.is_absolute() {
            new = format!("/{}", new);
        }
        Self { raw: new }
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

    #[test]
    fn resolve_empty_path() {
        let path = H5Path::from("".to_string());
        let resolved = path.resolve();
        let expected = H5Path::from("".to_string());
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_root_path() {
        let path = H5Path::from("/".to_string());
        let resolved = path.resolve();
        let expected = H5Path::from("/".to_string());
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_absolute_path() {
        let path = H5Path::from("/dir/group/ds".to_string());
        let resolved = path.resolve();
        let expected = H5Path::from("/dir/group/ds".to_string());
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_one_up_segment() {
        let path = H5Path::from("/a/b/../c".to_string());
        let resolved = path.resolve();
        let expected = H5Path::from("/a/c".to_string());
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_many_up_segment() {
        let path = H5Path::from("/a/../b/c/../../d/e".to_string());
        let resolved = path.resolve();
        let expected = H5Path::from("/d/e".to_string());
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_only_up_segment() {
        let path = H5Path::from("..".to_string());
        let resolved = path.resolve();
        let expected = H5Path::from("".to_string());
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_only_up_from_root_segment() {
        let path = H5Path::from("/..".to_string());
        let resolved = path.resolve();
        let expected = H5Path::from("/".to_string());
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_initial_up_segment() {
        let path = H5Path::from("../a".to_string());
        let resolved = path.resolve();
        let expected = H5Path::from("a".to_string());
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_initial_up_past_root() {
        let path = H5Path::from("/a/b/c/../../../../".to_string());
        let resolved = path.resolve();
        let expected = H5Path::from("/".to_string());
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_removes_duplicate_slashes() {
        let path = H5Path::from("/a//b/c//..///d".to_string());
        let resolved = path.resolve();
        let expected = H5Path::from("/a/b/d".to_string());
        assert_eq!(resolved, expected);
    }
}

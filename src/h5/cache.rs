use indexmap::IndexMap;
use smallvec::SmallVec;

use super::path::H5Path;

// Generic struct to simplify testing.
#[derive(Clone, Debug, Default)]
pub struct FileCache<Entry> {
    objects: IndexMap<H5Path, Entry>,
}

pub type H5FileCache = FileCache<CacheEntry>;

impl<Entry> FileCache<Entry> {
    pub fn get<Key: CacheKey<Entry>>(&self, key: Key) -> Option<&Entry> {
        key.get_cache_entry(&self.objects)
    }

    pub fn insert(&mut self, path: H5Path, data: Entry) -> EntryId {
        self.objects.insert_full(path, data).0.into()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EntryId(usize);

impl From<usize> for EntryId {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

pub trait CacheKey<Entry> {
    fn get_cache_entry<'m>(&self, objects: &'m IndexMap<H5Path, Entry>) -> Option<&'m Entry>;
}

impl<Entry> CacheKey<Entry> for H5Path {
    fn get_cache_entry<'m>(&self, objects: &'m IndexMap<H5Path, Entry>) -> Option<&'m Entry> {
        objects.get(self)
    }
}

impl<Entry> CacheKey<Entry> for &H5Path {
    fn get_cache_entry<'m>(&self, objects: &'m IndexMap<H5Path, Entry>) -> Option<&'m Entry> {
        objects.get(*self)
    }
}

impl<Entry> CacheKey<Entry> for EntryId {
    fn get_cache_entry<'m>(&self, objects: &'m IndexMap<H5Path, Entry>) -> Option<&'m Entry> {
        objects.get_index(self.0).map(|(_, entry)| entry)
    }
}

#[derive(Debug)]
pub struct CacheEntry {
    location_info: hdf5::LocationInfo,
    children: Option<SmallVec<EntryId, 4>>,
}

impl CacheEntry {
    pub fn from_location_info(location_info: hdf5::LocationInfo) -> Self {
        Self {
            location_info,
            children: None,
        }
    }

    pub fn location_token(&self) -> &hdf5::LocationToken {
        &self.location_info.token
    }

    pub fn children(&self) -> Option<&[EntryId]> {
        self.children.as_ref().map(|ids| ids.as_slice())
    }

    pub fn set_children<C: Into<SmallVec<EntryId, 4>>>(&mut self, children: C) {
        self.children = Some(children.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn get_entry_by_path() {
        let cache = {
            let mut cache = FileCache::<i32>::default();
            cache.insert(H5Path::from("/root".to_string()), 4);
            cache.insert(H5Path::from("/root/a".to_string()), 6);
            cache.insert(H5Path::from("/root/b".to_string()), 9);
            cache
        };
        assert_eq!(cache.get(H5Path::from("/root".to_string())), Some(&4));
        assert_eq!(cache.get(H5Path::from("/root/a".to_string())), Some(&6));
        assert_eq!(cache.get(H5Path::from("/root/b".to_string())), Some(&9));
    }

    #[test]
    fn get_entry_by_id() {
        let (cache, (root, a, b)) = {
            let mut cache = FileCache::<i32>::default();
            let root = cache.insert(H5Path::from("/root".to_string()), 4);
            let a = cache.insert(H5Path::from("/root/a".to_string()), 6);
            let b = cache.insert(H5Path::from("/root/b".to_string()), 9);
            (cache, (root, a, b))
        };
        assert_eq!(cache.get(root), Some(&4));
        assert_eq!(cache.get(a), Some(&6));
        assert_eq!(cache.get(b), Some(&9));
    }
}

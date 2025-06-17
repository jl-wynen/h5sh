use indexmap::IndexMap;
use smallvec::SmallVec;

use super::path::H5Path;

// The struct is generic struct to simplify testing.
#[derive(Clone, Debug, Default)]
pub struct FileCache<Value> {
    objects: IndexMap<H5Path, Entry<Value>>,
}

pub type H5FileCache = FileCache<CacheValue>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EntryId(usize);

#[derive(Clone, Debug)]
pub struct Entry<Value> {
    value: Value,
    children: Option<SmallVec<EntryId, 4>>,
}

impl<Value> FileCache<Value> {
    pub fn get<Key: CacheKey<Entry<Value>>>(&self, key: Key) -> Option<&Entry<Value>> {
        key.get_cache_entry(&self.objects)
    }

    pub fn get_with_id(&self, key: &H5Path) -> Option<(EntryId, &Entry<Value>)> {
        self.objects
            .get_full(key)
            .map(|(index, _, entry)| (index.into(), entry))
    }

    pub fn insert(&mut self, path: H5Path, data: Value) -> EntryId {
        self.objects.insert_full(path, Entry::from(data)).0.into()
    }
}

impl<Value> Entry<Value> {
    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    pub fn children(&self) -> Option<&[EntryId]> {
        self.children.as_ref().map(|ids| ids.as_slice())
    }

    pub fn set_children<C: Into<SmallVec<EntryId, 4>>>(&mut self, children: C) {
        self.children = Some(children.into());
    }
}

impl From<usize> for EntryId {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

impl<Value> From<Value> for Entry<Value> {
    fn from(value: Value) -> Self {
        Self {
            value,
            children: None,
        }
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
pub struct CacheValue {
    location_info: hdf5::LocationInfo,
}

impl CacheValue {
    pub fn from_location_info(location_info: hdf5::LocationInfo) -> Self {
        Self { location_info }
    }

    pub fn location_token(&self) -> &hdf5::LocationToken {
        &self.location_info.token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn get_entry_by_path() {
        let root = H5Path::from("/root".to_string());
        let a = H5Path::from("/root/a".to_string());
        let b = H5Path::from("/root/b".to_string());
        let cache = {
            let mut cache = FileCache::<i32>::default();
            cache.insert(root.clone(), 4);
            cache.insert(a.clone(), 6);
            cache.insert(b.clone(), 9);
            cache
        };
        assert_eq!(cache.get(root).unwrap().value(), &4);
        assert_eq!(cache.get(a).unwrap().value(), &6);
        assert_eq!(cache.get(b).unwrap().value(), &9);
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
        assert_eq!(cache.get(root).unwrap().value(), &4);
        assert_eq!(cache.get(a).unwrap().value(), &6);
        assert_eq!(cache.get(b).unwrap().value(), &9);
    }

    #[test]
    fn inserting_does_not_populate_children() {
        let root = H5Path::from("/root".to_string());
        let a = H5Path::from("/root/a".to_string());
        let b = H5Path::from("/root/b".to_string());
        let cache = {
            let mut cache = FileCache::<i32>::default();
            cache.insert(root.clone(), 4);
            cache.insert(a.clone(), 6);
            cache.insert(b.clone(), 9);
            cache
        };
        assert_eq!(cache.get(root).unwrap().children(), None);
        assert_eq!(cache.get(a).unwrap().children(), None);
        assert_eq!(cache.get(b).unwrap().children(), None);
    }
}

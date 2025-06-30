use anyhow::{Result, bail};
use indexmap::IndexMap;
use smallvec::SmallVec;

use super::path::H5Path;

// This is a generic struct to simplify testing.
#[derive(Clone, Debug, Default)]
pub struct FileCache<Value> {
    objects: IndexMap<H5Path, Entry<Value>>,
}

pub type H5FileCache = FileCache<CacheValue>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EntryId(usize);

#[derive(Clone, Debug)]
pub enum Entry<Value> {
    Group {
        value: Value,
        children: Option<SmallVec<EntryId, 4>>,
    },
    Leaf {
        value: Value,
    },
}

pub use Entry::{Group, Leaf};

impl<Value> FileCache<Value> {
    pub fn contains_key(&self, key: &H5Path) -> bool {
        self.objects.contains_key(key)
    }

    pub fn get<Key: CacheKey<Entry<Value>>>(&self, key: Key) -> Option<&Entry<Value>> {
        key.get_cache_entry(&self.objects)
    }

    pub fn get_mut<Key: CacheKey<Entry<Value>>>(&mut self, key: Key) -> Option<&mut Entry<Value>> {
        key.get_cache_entry_mut(&mut self.objects)
    }

    pub fn get_with_id(&self, key: &H5Path) -> Option<(EntryId, &Entry<Value>)> {
        self.objects
            .get_full(key)
            .map(|(index, _, entry)| (index.into(), entry))
    }

    pub fn get_key(&self, id: EntryId) -> Option<&H5Path> {
        self.objects.get_index(id.0).map(|(key, _)| key)
    }

    pub fn insert_group(&mut self, path: &H5Path, value: Value) -> EntryId {
        self.insert_entry(
            path,
            Group {
                value,
                children: None,
            },
        )
    }

    pub fn insert_leaf(&mut self, path: &H5Path, value: Value) -> EntryId {
        self.insert_entry(path, Leaf { value })
    }

    pub fn insert_entry(&mut self, path: &H5Path, entry: Entry<Value>) -> EntryId {
        self.objects.insert_full(path.normalized(), entry).0.into()
    }

    pub fn insert_children<Key, Values>(&mut self, parent: Key, children: Values) -> Result<()>
    where
        Key: CacheKey<Entry<Value>>,
        Values: IntoIterator<Item = (H5Path, Value, bool)>,
    {
        if !parent.is_in_cache(&self.objects) {
            bail!("Parent does not exist in cache");
        }
        let child_ids = children
            .into_iter()
            .map(|(path, data, is_group)| {
                if is_group {
                    self.insert_group(&path, data)
                } else {
                    self.insert_leaf(&path, data)
                }
            })
            .collect::<SmallVec<_, 4>>();
        let parent = self.get_mut(parent).unwrap();
        parent.insert_children(child_ids)
    }
}

impl<Value> Entry<Value> {
    pub fn value(&self) -> &Value {
        match self {
            Group { value, .. } => value,
            Leaf { value } => value,
        }
    }

    pub fn value_mut(&mut self) -> &mut Value {
        match self {
            Group { value, .. } => value,
            Leaf { value } => value,
        }
    }

    pub fn insert_children<C: IntoIterator<Item = EntryId>>(&mut self, children: C) -> Result<()> {
        match self {
            Leaf { .. } => bail!("Cannot insert children into a leaf"),
            Group { children: c, .. } => {
                c.get_or_insert_with(SmallVec::new).extend(children);
                Ok(())
            }
        }
    }
}

impl From<usize> for EntryId {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

pub trait CacheKey<Entry> {
    fn get_cache_entry<'m>(&self, objects: &'m IndexMap<H5Path, Entry>) -> Option<&'m Entry>;
    fn get_cache_entry_mut<'m>(
        &self,
        objects: &'m mut IndexMap<H5Path, Entry>,
    ) -> Option<&'m mut Entry>;
    fn is_in_cache(&self, objects: &IndexMap<H5Path, Entry>) -> bool;
}

impl<Entry> CacheKey<Entry> for H5Path {
    fn get_cache_entry<'m>(&self, objects: &'m IndexMap<H5Path, Entry>) -> Option<&'m Entry> {
        objects.get(&self.normalized())
    }

    fn get_cache_entry_mut<'m>(
        &self,
        objects: &'m mut IndexMap<H5Path, Entry>,
    ) -> Option<&'m mut Entry> {
        objects.get_mut(&self.normalized())
    }

    fn is_in_cache(&self, objects: &IndexMap<H5Path, Entry>) -> bool {
        objects.contains_key(&self.normalized())
    }
}

impl<Entry> CacheKey<Entry> for &H5Path {
    fn get_cache_entry<'m>(&self, objects: &'m IndexMap<H5Path, Entry>) -> Option<&'m Entry> {
        objects.get(&self.normalized())
    }

    fn get_cache_entry_mut<'m>(
        &self,
        objects: &'m mut IndexMap<H5Path, Entry>,
    ) -> Option<&'m mut Entry> {
        objects.get_mut(&self.normalized())
    }

    fn is_in_cache(&self, objects: &IndexMap<H5Path, Entry>) -> bool {
        objects.contains_key(&self.normalized())
    }
}

impl<Entry> CacheKey<Entry> for EntryId {
    fn get_cache_entry<'m>(&self, objects: &'m IndexMap<H5Path, Entry>) -> Option<&'m Entry> {
        objects.get_index(self.0).map(|(_, entry)| entry)
    }

    fn get_cache_entry_mut<'m>(
        &self,
        objects: &'m mut IndexMap<H5Path, Entry>,
    ) -> Option<&'m mut Entry> {
        objects.get_index_mut(self.0).map(|(_, entry)| entry)
    }

    fn is_in_cache(&self, objects: &IndexMap<H5Path, Entry>) -> bool {
        objects.len() > self.0
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
    use smallvec::smallvec;

    fn assert_children(entry: &Entry<i32>, expected: Option<SmallVec<EntryId, 4>>) {
        match entry {
            Group { children, .. } => {
                assert_eq!(*children, expected);
            }
            Leaf { .. } => {
                assert!(false)
            }
        }
    }

    fn assert_leaf(entry: &Entry<i32>) {
        assert!(matches!(entry, Leaf { .. }));
    }

    #[test]
    fn get_entry_by_path() {
        let root = H5Path::from("/root");
        let a = H5Path::from("/root/a");
        let b = H5Path::from("/root/b");
        let cache = {
            let mut cache = FileCache::<i32>::default();
            cache.insert_group(&root, 4);
            cache.insert_leaf(&a, 6);
            cache.insert_group(&b, 9);
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
            let root = cache.insert_group(&H5Path::from("/root"), 4);
            let a = cache.insert_leaf(&H5Path::from("/root/a"), 6);
            let b = cache.insert_group(&H5Path::from("/root/b"), 9);
            (cache, (root, a, b))
        };
        assert_eq!(cache.get(root).unwrap().value(), &4);
        assert_eq!(cache.get(a).unwrap().value(), &6);
        assert_eq!(cache.get(b).unwrap().value(), &9);
    }

    #[test]
    fn get_entry_by_path_auto_normalized() {
        let cache = {
            let mut cache = FileCache::<i32>::default();
            cache.insert_group(&H5Path::from("/root/"), 4);
            cache
        };
        assert_eq!(cache.get(H5Path::from("/root")).unwrap().value(), &4);
        assert_eq!(cache.get(H5Path::from("/root/")).unwrap().value(), &4);
        assert_eq!(cache.get(H5Path::from("/root//")).unwrap().value(), &4);
    }

    #[test]
    fn inserting_does_not_populate_children() {
        let root = H5Path::from("/root");
        let a = H5Path::from("/root/a");
        let b = H5Path::from("/root/b");
        let cache = {
            let mut cache = FileCache::<i32>::default();
            cache.insert_group(&root, 4);
            cache.insert_leaf(&a, 6);
            cache.insert_group(&b, 9);
            cache
        };
        assert_children(cache.get(root).unwrap(), None);
        assert_leaf(cache.get(a).unwrap());
        assert_children(cache.get(b).unwrap(), None);
    }

    #[test]
    fn insert_children() {
        let root = H5Path::from("/root");
        let a = H5Path::from("/root/a");
        let b = H5Path::from("/root/b");
        let c = H5Path::from("/root/b/c");
        let cache = {
            let mut cache = FileCache::<i32>::default();
            let root_id = cache.insert_group(&root, 4);
            cache
                .insert_children(root_id, [(a.clone(), 6, false), (b.clone(), 9, true)])
                .unwrap();
            cache
                .insert_children(b.clone(), std::iter::once((c.clone(), 11, true)))
                .unwrap();
            cache
        };
        let a_id = cache.get_with_id(&a).unwrap().0;
        let b_id = cache.get_with_id(&b).unwrap().0;
        let c_id = cache.get_with_id(&c).unwrap().0;
        let root_entry = cache.get(root).unwrap();
        assert_children(root_entry, Some(smallvec![a_id, b_id]));
        let b_entry = cache.get(b).unwrap();
        assert_children(b_entry, Some(smallvec![c_id]));
    }
}

use indexmap::{
    IndexMap,
    map::Entry::{Occupied, Vacant},
};
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

    pub fn insert(&mut self, path: H5Path, data: Value) -> EntryId {
        self.objects.insert_full(path, Entry::from(data)).0.into()
    }

    #[must_use = "This method does not modify the cache and returns false if the parent does not exist"]
    pub fn insert_children<
        Key: CacheKey<Entry<Value>>,
        Values: IntoIterator<Item = (H5Path, Value)>,
    >(
        &mut self,
        parent: Key,
        children: Values,
    ) -> bool {
        if !parent.is_in_cache(&self.objects) {
            return false;
        }
        let child_ids = children
            .into_iter()
            .map(|(path, data)| self.insert(path, data))
            .collect::<SmallVec<_, 4>>();
        let parent = self.get_mut(parent).unwrap();
        parent.insert_children(child_ids);
        true
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

    pub fn insert_children<C: IntoIterator<Item = EntryId>>(&mut self, children: C) {
        self.children
            .get_or_insert_with(SmallVec::new)
            .extend(children);
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
    fn get_cache_entry_mut<'m>(
        &self,
        objects: &'m mut IndexMap<H5Path, Entry>,
    ) -> Option<&'m mut Entry>;
    fn is_in_cache(&self, objects: &IndexMap<H5Path, Entry>) -> bool;
}

impl<Entry> CacheKey<Entry> for H5Path {
    fn get_cache_entry<'m>(&self, objects: &'m IndexMap<H5Path, Entry>) -> Option<&'m Entry> {
        objects.get(self)
    }

    fn get_cache_entry_mut<'m>(
        &self,
        objects: &'m mut IndexMap<H5Path, Entry>,
    ) -> Option<&'m mut Entry> {
        objects.get_mut(self)
    }

    fn is_in_cache(&self, objects: &IndexMap<H5Path, Entry>) -> bool {
        objects.contains_key(self)
    }
}

impl<Entry> CacheKey<Entry> for &H5Path {
    fn get_cache_entry<'m>(&self, objects: &'m IndexMap<H5Path, Entry>) -> Option<&'m Entry> {
        objects.get(*self)
    }

    fn get_cache_entry_mut<'m>(
        &self,
        objects: &'m mut IndexMap<H5Path, Entry>,
    ) -> Option<&'m mut Entry> {
        objects.get_mut(*self)
    }

    fn is_in_cache(&self, objects: &IndexMap<H5Path, Entry>) -> bool {
        objects.contains_key(*self)
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

    #[test]
    fn insert_children() {
        let root = H5Path::from("/root".to_string());
        let a = H5Path::from("/root/a".to_string());
        let b = H5Path::from("/root/b".to_string());
        let c = H5Path::from("/root/b/c".to_string());
        let cache = {
            let mut cache = FileCache::<i32>::default();
            let root_id = cache.insert(root.clone(), 4);
            cache.insert_children(root_id, [(a.clone(), 6), (b.clone(), 9)]);
            cache.insert_children(b.clone(), std::iter::once((c.clone(), 11)));
            cache
        };
        let a_id = cache.get_with_id(&a).unwrap().0;
        let b_id = cache.get_with_id(&b).unwrap().0;
        let c_id = cache.get_with_id(&c).unwrap().0;
        let root_entry = cache.get(root).unwrap();
        assert_eq!(root_entry.children, Some(smallvec![a_id, b_id]));
        let b_entry = cache.get(b).unwrap();
        assert_eq!(b_entry.children, Some(smallvec![c_id]));
    }
}

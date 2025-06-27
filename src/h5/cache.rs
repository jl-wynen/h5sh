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

    pub fn insert_child<Key: CacheKey<Entry<Value>>>(
        &mut self,
        parent: Key,
        path: H5Path,
        data: Value,
    ) -> Option<EntryId> {
        // This implementation ensures that the child is only inserted if the parent
        // exists. It uses unsafe code with pointers to bypass the borrow checker which
        // would otherwise not allow getting the parent first and then inserting the
        // child. Alternatively, we could check whether the parent exists first,
        // insert the child, and then get_mut the parent. But that would require
        // extra hash operations.
        self.objects.reserve(1);
        let parent = self.get_mut(parent)? as *mut Entry<Value>;
        let child_id = self.insert(path, data);
        // Safety: The call to reserve above ensures that there is enough space in the
        // map for the child, so the parent is not moved and the pointer stays valid.
        unsafe {
            (*parent)
                .children
                .get_or_insert_with(SmallVec::new)
                .push(child_id);
        }
        Some(child_id)
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
    fn get_cache_entry_mut<'m>(
        &self,
        objects: &'m mut IndexMap<H5Path, Entry>,
    ) -> Option<&'m mut Entry>;
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
        let cache = {
            let mut cache = FileCache::<i32>::default();
            let root_id = cache.insert(root.clone(), 4);
            cache.insert_child(root_id, a.clone(), 6);
            cache.insert_child(root.clone(), b.clone(), 9);
            cache
        };
        let a_id = cache.get_with_id(&a).unwrap().0;
        let b_id = cache.get_with_id(&b).unwrap().0;
        let root_entry = cache.get(root).unwrap();
        assert_eq!(root_entry.children, Some(smallvec![a_id, b_id]));
    }
}

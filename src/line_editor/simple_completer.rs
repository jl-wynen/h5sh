use crate::h5::{
    CacheEntry, CacheEntryId, FileCache, H5Path,
    cache::{Group, Leaf},
};
use smallvec::{SmallVec, smallvec};

pub(super) fn path_completions<Value>(
    cache: &FileCache<Value>,
    current: &H5Path,
) -> SmallVec<H5Path, 4> {
    if let Some(entry) = cache.get(current) {
        if entry.is_leaf() || !current.as_raw().ends_with('/') {
            return smallvec![finalize_entry_path(current.as_raw(), entry)];
        }
    }

    let (parent, name) = current.split_parent();
    if let Some(candidates) = get_all_children(&parent, cache) {
        complete_from_children(cache, candidates, current.as_raw())
    } else {
        smallvec![]
    }
}

fn finalize_entry_path<Value>(path: &str, entry: &CacheEntry<Value>) -> H5Path {
    match entry {
        Leaf { .. } => path.into(),
        Group { .. } => {
            if path.ends_with('/') {
                path.into()
            } else {
                H5Path::from(format!("{path}/"))
            }
        }
    }
}

fn get_all_children<'c, Value>(
    path: &H5Path,
    cache: &'c FileCache<Value>,
) -> Option<&'c SmallVec<CacheEntryId, 4>> {
    if let Some(entry) = cache.get(path) {
        match entry {
            Group {
                children: Some(children),
                ..
            } => Some(children),
            Group { .. } => {
                todo!("load children")
            }
            Leaf { .. } => None,
        }
    } else {
        None // TODO recurse
    }
}

fn complete_from_children<Value>(
    cache: &FileCache<Value>,
    children: &[CacheEntryId],
    name: &str,
) -> SmallVec<H5Path, 4> {
    children
        .iter()
        .filter_map(|child_id| cache.get_key(*child_id))
        .filter(|candidate| candidate.as_raw().starts_with(name))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::h5::{FileCache, H5Path};
    use anyhow::Result;
    use std::collections::HashSet;
    use std::hash::Hash;

    /*
        /
        |- stem
        |- base/
           |- aa/
           |  |- (?)
           |
           |- bb/
           |  |- dd
           |  |- cc/
           |     | (no children)
           |
           |- ee
    */
    fn make_cache() -> Result<FileCache<i32>> {
        let mut cache = FileCache::<i32>::default();
        let root = cache.insert_group(&H5Path::from("/"), -1);
        cache.insert_children(
            root,
            [
                (H5Path::from("/stem"), 1, false),
                (H5Path::from("/base"), 2, true),
            ],
        )?;
        cache.insert_children(
            H5Path::from("/base"),
            [
                (H5Path::from("/base/aa"), 3, true),
                (H5Path::from("/base/bb"), 4, true),
                (H5Path::from("/base/ee"), 5, false),
            ],
        )?;
        cache.insert_children(
            H5Path::from("/base/bb"),
            [
                (H5Path::from("/base/bb/dd"), 6, false),
                (H5Path::from("/base/bb/cc"), 7, true),
            ],
        )?;
        cache.insert_children(H5Path::from("/base/bb/cc"), [])?;
        Ok(cache)
    }

    fn assert_unordered_eq<A, B, X>(a: A, b: B)
    where
        A: IntoIterator<Item = X>,
        B: IntoIterator<Item = X>,
        X: PartialEq + Eq + std::fmt::Debug + Hash,
    {
        let a: HashSet<_> = a.into_iter().collect();
        let b: HashSet<_> = b.into_iter().collect();
        assert_eq!(a, b);
    }

    #[test]
    fn no_completions_if_path_not_in_cache() {
        let cache = make_cache().unwrap();
        let results = path_completions(&cache, &H5Path::from("/other"));
        assert!(results.is_empty());
    }

    #[test]
    fn completion_returns_leaf_path() {
        let cache = make_cache().unwrap();
        let results = path_completions(&cache, &H5Path::from("/stem"));
        assert_unordered_eq(results, [H5Path::from("/stem")]);
        let results = path_completions(&cache, &H5Path::from("/stem/"));
        assert_unordered_eq(results, [H5Path::from("/stem/")]);
        let results = path_completions(&cache, &H5Path::from("/base/ee"));
        assert_unordered_eq(results, [H5Path::from("/base/ee")]);
        let results = path_completions(&cache, &H5Path::from("/base/ee/"));
        assert_unordered_eq(results, [H5Path::from("/base/ee/")]);
    }

    #[test]
    fn completion_returns_group_path_with_slash() {
        let cache = make_cache().unwrap();
        let results = path_completions(&cache, &H5Path::from("/base"));
        assert_unordered_eq(results, [H5Path::from("/base/")]);
        let results = path_completions(&cache, &H5Path::from("/base/bb"));
        assert_unordered_eq(results, [H5Path::from("/base/bb/")]);
    }

    #[test]
    fn completion_returns_all_children() {
        let cache = make_cache().unwrap();
        let results = path_completions(&cache, &H5Path::from("/"));
        assert_unordered_eq(results, [H5Path::from("/base"), H5Path::from("/stem")]);
        let results = path_completions(&cache, &H5Path::from("/base/"));
        assert_unordered_eq(
            results,
            [
                H5Path::from("/base/aa"),
                H5Path::from("/base/bb"),
                H5Path::from("/base/ee"),
            ],
        );
        let results = path_completions(&cache, &H5Path::from("/base/bb/"));
        assert_unordered_eq(
            results,
            [H5Path::from("/base/bb/cc"), H5Path::from("/base/bb/dd")],
        );
    }

    #[test]
    fn completion_returns_all_children_empty() {
        let cache = make_cache().unwrap();
        let results = path_completions(&cache, &H5Path::from("/base/bb/cc/"));
        assert_unordered_eq(results, []);
    }

    #[test]
    fn completion_finishes_unique_path() {
        let cache = make_cache().unwrap();
        for part in ["/b", "/ba", "/bas"] {
            let results = path_completions(&cache, &H5Path::from(part));
            assert_unordered_eq(results, [H5Path::from("/base")]);
        }
        for part in ["/s", "/st", "/ste"] {
            let results = path_completions(&cache, &H5Path::from(part));
            assert_unordered_eq(results, [H5Path::from("/stem")]);
        }
        let results = path_completions(&cache, &H5Path::from("/base/a"));
        assert_unordered_eq(results, [H5Path::from("/base/aa")]);
    }

    // TODO load children
}

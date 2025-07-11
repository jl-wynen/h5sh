use crate::h5::{
    self, CacheEntry, CacheEntryId, FileCache, H5Path,
    cache::{Group, Leaf},
};
use smallvec::{SmallVec, smallvec};

pub(super) fn path_completions<Value, LoadChildren, Children>(
    cache: &mut FileCache<Value>,
    current: &H5Path,
    load_children: LoadChildren,
) -> SmallVec<H5Path, 4>
where
    LoadChildren: Fn(&H5Path) -> h5::Result<Children>,
    Children: IntoIterator<Item = (H5Path, Value, bool)>,
{
    if let Some(entry) = cache.get(current) {
        if entry.is_leaf() || !current.as_raw().ends_with('/') {
            return smallvec![finalize_entry_path(current.as_raw(), entry)];
        }
    }

    let parent = current.parent();
    if let Some(candidates) = get_all_children(&parent, cache, load_children) {
        complete_from_children(candidates, current.as_raw())
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

fn get_all_children<'c, Value, LoadChildren, Children>(
    path: &H5Path,
    cache: &'c mut FileCache<Value>,
    load_children: LoadChildren,
) -> Option<impl Iterator<Item = &'c H5Path>>
where
    LoadChildren: Fn(&H5Path) -> h5::Result<Children>,
    Children: IntoIterator<Item = (H5Path, Value, bool)>,
{
    // The children might already be loaded, if so, bypass the (somewhat) search and
    // load mechanism.
    if !matches!(
        cache.get(path),
        Some(Group {
            children: Some(_),
            ..
        })
    ) {
        let (ancestor_path, remaining_segments) = find_deepest_available_ancestor(path, cache);
        load_children_of_all(remaining_segments, &ancestor_path, cache, load_children).ok()?;
    }

    match cache.get(path) {
        Some(Group {
            children: Some(children),
            ..
        }) => Some(
            children
                .iter()
                .filter_map(|id: &CacheEntryId| cache.get_key(*id)),
        ),
        _ => None,
    }
}

fn find_deepest_available_ancestor<'p, Value>(
    path: &'p H5Path,
    cache: &FileCache<Value>,
) -> (H5Path, SmallVec<&'p str, 2>) {
    let mut segments = path.segments();
    let mut existing = H5Path::root();
    loop {
        let Some(segment) = segments.next() else {
            return (existing, SmallVec::new());
        };
        let mut new_path = existing.clone();
        new_path.push(segment);
        if !cache.contains_key(&new_path) {
            return (existing, std::iter::once(segment).chain(segments).collect());
        }
        existing = new_path;
    }
}

fn load_children_of_all<Value, LoadChildren, Children>(
    segments: SmallVec<&str, 2>,
    starting_path: &H5Path,
    cache: &mut FileCache<Value>,
    load_children: LoadChildren,
) -> h5::Result<()>
where
    LoadChildren: Fn(&H5Path) -> h5::Result<Children>,
    Children: IntoIterator<Item = (H5Path, Value, bool)>,
{
    let mut path = starting_path.clone();
    for segment in segments {
        load_children_of(&path, cache, &load_children)?;
        // else: The requested path does not exist in the file and cannot be completed.
        path.push(segment);
    }
    // Load children of the target path itself:
    load_children_of(&path, cache, &load_children)?;
    Ok(())
}

fn load_children_of<Value, LoadChildren, Children>(
    path: &H5Path,
    cache: &mut FileCache<Value>,
    load_children: LoadChildren,
) -> h5::Result<()>
where
    LoadChildren: Fn(&H5Path) -> h5::Result<Children>,
    Children: IntoIterator<Item = (H5Path, Value, bool)>,
{
    // Only load and insert children if they have not already been loaded.
    if let Some(Group { children: None, .. }) = cache.get(path) {
        let c = load_children(path)
            .unwrap()
            .into_iter()
            .map(|(path, value, is_group)| (path, is_group))
            .collect::<Vec<_>>();
        let _ = cache.insert_children(path.clone(), load_children(path)?);
    }
    Ok(())
}

fn complete_from_children<'a>(
    children: impl Iterator<Item = &'a H5Path>,
    name: &str,
) -> SmallVec<H5Path, 4> {
    children
        .filter(|candidate| candidate.as_raw().starts_with(name))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::h5::{FileCache, H5Path};
    use anyhow::Result;
    use std::collections::{HashMap, HashSet};
    use std::hash::Hash;

    /*
        /
        |- stem
        |- base/
           |- ee
           |
           |- bb/
           |  |- dd
           |  |- d1
           |  |- d12
           |  |- cc/
           |     | (no children)
           |
           |- aa/
              (children initially not loaded)
              |- xx
              |- yy/
                 |- z1
                 |- z2
                 |- z12
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
                (H5Path::from("/base/bb/d1"), 61, false),
                (H5Path::from("/base/bb/d12"), 612, false),
                (H5Path::from("/base/bb/cc"), 7, true),
            ],
        )?;
        cache.insert_children(H5Path::from("/base/bb/cc"), [])?;
        Ok(cache)
    }

    fn child_loader() -> impl Fn(&H5Path) -> h5::Result<Vec<(H5Path, i32, bool)>> {
        let extra_children = HashMap::from([
            (
                H5Path::from("/base/aa"),
                vec![
                    (H5Path::from("/base/aa/xx"), 100, false),
                    (H5Path::from("/base/aa/yy"), 101, true),
                ],
            ),
            (
                H5Path::from("/base/aa/yy"),
                vec![
                    (H5Path::from("/base/aa/yy/z1"), 102, false),
                    (H5Path::from("/base/aa/yy/z2"), 103, false),
                    (H5Path::from("/base/aa/yy/z12"), 104, false),
                ],
            ),
        ]);

        move |path| match extra_children.get(&path.normalized()) {
            Some(children) => Ok(children.clone()),
            None => Err(h5::H5Error::NotFound(path.clone())),
        }
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
        let mut cache = make_cache().unwrap();
        let load_children = child_loader();
        let results = path_completions(&mut cache, &H5Path::from("/other"), &load_children);
        assert!(results.is_empty());
        let results = path_completions(&mut cache, &H5Path::from("/other/nested"), &load_children);
        assert!(results.is_empty());
        let results = path_completions(&mut cache, &H5Path::from("/base/nested2"), &load_children);
        assert!(results.is_empty());
        let results = path_completions(&mut cache, &H5Path::from("/base/aa/deep"), &load_children);
        assert!(results.is_empty());
        let results = path_completions(
            &mut cache,
            &H5Path::from("/base/aa/yy/deep"),
            &load_children,
        );
        assert!(results.is_empty());
        let results = path_completions(&mut cache, &H5Path::from("/base/bb/deep"), &load_children);
        assert!(results.is_empty());
    }

    #[test]
    fn completion_returns_leaf_path() {
        let mut cache = make_cache().unwrap();
        let load_children = child_loader();
        let results = path_completions(&mut cache, &H5Path::from("/stem"), &load_children);
        assert_unordered_eq(results, [H5Path::from("/stem")]);
        let results = path_completions(&mut cache, &H5Path::from("/stem/"), &load_children);
        assert_unordered_eq(results, [H5Path::from("/stem/")]);
        let results = path_completions(&mut cache, &H5Path::from("/base/ee"), &load_children);
        assert_unordered_eq(results, [H5Path::from("/base/ee")]);
        let results = path_completions(&mut cache, &H5Path::from("/base/ee/"), &load_children);
        assert_unordered_eq(results, [H5Path::from("/base/ee/")]);
    }

    #[test]
    fn completion_returns_group_path_with_slash() {
        let mut cache = make_cache().unwrap();
        let load_children = child_loader();
        let results = path_completions(&mut cache, &H5Path::from("/base"), &load_children);
        assert_unordered_eq(results, [H5Path::from("/base/")]);
        let results = path_completions(&mut cache, &H5Path::from("/base/bb"), &load_children);
        assert_unordered_eq(results, [H5Path::from("/base/bb/")]);
    }

    #[test]
    fn completion_returns_all_children() {
        let mut cache = make_cache().unwrap();
        let load_children = child_loader();
        let results = path_completions(&mut cache, &H5Path::from("/"), &load_children);
        assert_unordered_eq(results, [H5Path::from("/base"), H5Path::from("/stem")]);
        let results = path_completions(&mut cache, &H5Path::from("/base/"), &load_children);
        assert_unordered_eq(
            results,
            [
                H5Path::from("/base/aa"),
                H5Path::from("/base/bb"),
                H5Path::from("/base/ee"),
            ],
        );
        let results = path_completions(&mut cache, &H5Path::from("/base/bb/"), &load_children);
        assert_unordered_eq(
            results,
            [
                H5Path::from("/base/bb/cc"),
                H5Path::from("/base/bb/dd"),
                H5Path::from("/base/bb/d1"),
                H5Path::from("/base/bb/d12"),
            ],
        );
    }

    #[test]
    fn completion_returns_all_children_empty() {
        let mut cache = make_cache().unwrap();
        let load_children = child_loader();
        let results = path_completions(&mut cache, &H5Path::from("/base/bb/cc/"), load_children);
        assert_unordered_eq(results, []);
    }

    #[test]
    fn completion_returns_all_children_after_loading() {
        let mut cache = make_cache().unwrap();
        let load_children = child_loader();
        let results = path_completions(&mut cache, &H5Path::from("/base/aa/"), load_children);
        assert_unordered_eq(
            results,
            [H5Path::from("/base/aa/xx"), H5Path::from("/base/aa/yy")],
        );
    }

    #[test]
    fn completion_returns_all_children_after_loading_nested() {
        let mut cache = make_cache().unwrap();
        let load_children = child_loader();
        let results = path_completions(&mut cache, &H5Path::from("/base/aa/yy/"), load_children);
        assert_unordered_eq(
            results,
            [
                H5Path::from("/base/aa/yy/z1"),
                H5Path::from("/base/aa/yy/z2"),
                H5Path::from("/base/aa/yy/z12"),
            ],
        );
    }

    #[test]
    fn completion_finishes_unique_path() {
        let mut cache = make_cache().unwrap();
        let load_children = child_loader();
        for part in ["/b", "/ba", "/bas"] {
            let results = path_completions(&mut cache, &H5Path::from(part), &load_children);
            assert_unordered_eq(results, [H5Path::from("/base")]);
        }
        for part in ["/s", "/st", "/ste"] {
            let results = path_completions(&mut cache, &H5Path::from(part), &load_children);
            assert_unordered_eq(results, [H5Path::from("/stem")]);
        }
        let results = path_completions(&mut cache, &H5Path::from("/base/a"), &load_children);
        assert_unordered_eq(results, [H5Path::from("/base/aa")]);
    }

    #[test]
    fn completion_finishes_unique_path_after_loading() {
        let mut cache = make_cache().unwrap();
        let load_children = child_loader();
        let results = path_completions(&mut cache, &H5Path::from("/base/aa/x"), load_children);
        assert_unordered_eq(results, [H5Path::from("/base/aa/xx")]);
    }

    #[test]
    fn completion_finishes_non_unique_paths() {
        let mut cache = make_cache().unwrap();
        let load_children = child_loader();
        let results = path_completions(&mut cache, &H5Path::from("/base/bb/d"), load_children);
        assert_unordered_eq(
            results,
            [
                H5Path::from("/base/bb/dd"),
                H5Path::from("/base/bb/d1"),
                H5Path::from("/base/bb/d12"),
            ],
        );
    }

    #[test]
    fn completion_finishes_non_unique_paths_after_loading_nested() {
        let mut cache = make_cache().unwrap();
        let load_children = child_loader();
        let results = path_completions(&mut cache, &H5Path::from("/base/aa/yy/z1"), load_children);
        assert_unordered_eq(
            results,
            [
                H5Path::from("/base/aa/yy/z1"),
                H5Path::from("/base/aa/yy/z12"),
            ],
        );
    }
}

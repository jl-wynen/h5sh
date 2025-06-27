use crate::h5::{EntryId, FileCache, H5Path};
use smallvec::{SmallVec, smallvec};

pub(super) fn path_completions<Value>(
    cache: &FileCache<Value>,
    current: &H5Path,
) -> SmallVec<H5Path, 4> {
    if cache.contains_key(current) {
        smallvec![current.clone()]
    } else {
        let (parent, name) = current.split_parent();
        let Some(parent_node) = cache.get(parent) else {
            return Default::default();
        };
        match parent_node.children() {
            Some(children) => complete_from_children(cache, children, name),
            None => {
                todo!("load children and check them");
            }
        }
    }
}

fn complete_from_children<Value>(
    cache: &FileCache<Value>,
    children: &[EntryId],
    name: &str,
) -> SmallVec<H5Path, 4> {
    children
        .iter()
        .filter_map(|child_id| cache.get_key(*child_id))
        .filter(|candidate| name.starts_with(candidate.name()))
        .cloned()
        .collect()
}

fn search_up_to_last_known<Entry>(
    cache: &FileCache<Entry>,
    segments: &[&str],
) -> Option<(EntryId, usize)> {
    // let mut path = H5Path::from(segments.first()?.to_string());
    // let (id, node) = cache.get_with_id(&path)?;
    // for segment in segments.iter().skip(1) {
    //     todo!("Search children")
    // }
    for segment in segments[..segments.len() - 1].iter() {
        todo!("walk parents")
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::h5::{FileCache, H5Path};

    #[test]
    fn no_completions_if_path_not_in_cache() {
        let cache = {
            let mut cache = FileCache::<i32>::default();
            cache.insert(H5Path::from("/".to_string()), -1);
            cache.insert(H5Path::from("/root".to_string()), 4);
            cache.insert(H5Path::from("/root/a".to_string()), 6);
            cache.insert(H5Path::from("/root/b".to_string()), 9);
            cache
        };
        let results = path_completions(&cache, &H5Path::from("/other".to_string()));
        assert!(results.is_empty());
    }

    #[test]
    fn completion_returns_root_path() {
        let cache = {
            let mut cache = FileCache::<i32>::default();
            cache.insert(H5Path::from("/".to_string()), -1);
            cache.insert(H5Path::from("/root".to_string()), 4);
            cache.insert(H5Path::from("/root/a".to_string()), 6);
            cache.insert(H5Path::from("/root/b".to_string()), 9);
            cache
        };
        let results = path_completions(&cache, &H5Path::from("/root".to_string()));
        assert_eq!(results.to_vec(), vec![H5Path::from("/root".to_string())]);
    }

    #[test]
    fn completion_finishes_root_path() {
        let cache = {
            let mut cache = FileCache::<i32>::default();
            let root_id = cache.insert(H5Path::from("/".to_string()), -1);
            cache.insert_child(root_id, H5Path::from("/root".to_string()), 4);
            cache.insert_child(root_id, H5Path::from("/root/a".to_string()), 6);
            cache.insert_child(root_id, H5Path::from("/root/b".to_string()), 9);
            cache
        };
        let results = path_completions(&cache, &H5Path::from("/ro".to_string()));
        assert_eq!(results.to_vec(), vec![H5Path::from("/root".to_string())]);
    }
}

use crate::h5::{EntryId, FileCache, H5Path};
use smallvec::{SmallVec, smallvec};
use std::ptr::write;

pub(super) fn path_completions<Entry>(
    cache: &FileCache<Entry>,
    current: &H5Path,
) -> SmallVec<H5Path, 4> {
    if cache.get(current).is_some() {
        smallvec![current.clone()]
    } else {
        let segments: Vec<_> = current.segments().collect();
        Default::default()
    }
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
            cache.insert(H5Path::from("/root".to_string()), 4);
            cache.insert(H5Path::from("/root/a".to_string()), 6);
            cache.insert(H5Path::from("/root/b".to_string()), 9);
            cache
        };
        let results = path_completions(&cache, &H5Path::from("/root".to_string()));
        assert_eq!(results.to_vec(), vec![H5Path::from("/root".to_string())]);
    }

    #[test]
    fn completion_finished_root_path() {
        let cache = {
            let mut cache = FileCache::<i32>::default();
            cache.insert(H5Path::from("/root".to_string()), 4);
            cache.insert(H5Path::from("/root/a".to_string()), 6);
            cache.insert(H5Path::from("/root/b".to_string()), 9);
            cache
        };
        let results = path_completions(&cache, &H5Path::from("/ro".to_string()));
        assert_eq!(results.to_vec(), vec![H5Path::from("/root".to_string())]);
    }
}

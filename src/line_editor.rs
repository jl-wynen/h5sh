mod complete {
    use crate::h5::{FileCache, H5Path};
    use smallvec::SmallVec;

    pub(super) fn path_completions<Entry>(
        cache: &FileCache<Entry>,
        current: &H5Path,
    ) -> SmallVec<H5Path, 4> {
        todo!()
    }
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
            cache
        };
        assert!(complete::path_completions(&cache, &H5Path::from("/other".to_string())).is_empty());
    }
}

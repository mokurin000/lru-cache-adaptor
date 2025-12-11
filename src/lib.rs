use std::path::PathBuf;

use disklru::Store;
use serde::{Serialize, de::DeserializeOwned};

mod error;
mod utils;

pub use disklru;
pub type LRUResult<K> = std::result::Result<K, LRUError>;

pub use error::LRUError;
use utils::remove_file_get_size;

#[derive(Debug)]
pub struct LruCache<K, V>
where
    K: Serialize + DeserializeOwned + Eq,
    V: Serialize + DeserializeOwned,
{
    inner: disklru::Store<K, V>,
}

impl<K, V> LruCache<K, V>
where
    K: Serialize + DeserializeOwned + Eq,
    V: Serialize + DeserializeOwned,
{
    pub fn new(store: disklru::Store<K, V>) -> Self {
        Self { inner: store }
    }

    /// Try access by an existing key
    ///
    /// If not found, returns `None`.
    pub fn access(&mut self, key: &K) -> LRUResult<Option<V>> {
        Ok(self.inner.get(key)?)
    }

    /// Return `Some(V)` if a value was replaced.
    pub fn insert(&mut self, key: &K, value: &V) -> LRUResult<Option<V>> {
        Ok(self.inner.insert(key, value)?)
    }

    pub fn most_recently_used(&self) -> LRUResult<Option<K>> {
        Ok(self.inner.mru()?)
    }
    pub fn most_recently_used_value(&mut self) -> LRUResult<Option<V>> {
        let Some(mru) = self.most_recently_used()? else {
            return Ok(None);
        };
        Ok(self.inner.get(&mru)?)
    }

    pub fn least_recently_used(&self) -> LRUResult<Option<K>> {
        Ok(self.inner.lru()?)
    }
    pub fn least_recently_used_value(&mut self) -> LRUResult<Option<V>> {
        Ok(self.inner.get_lru()?)
    }
}

impl<K> LruCache<K, PathBuf>
where
    K: Serialize + DeserializeOwned + Eq,
{
    /// Returns `Ok(Some(filesize))` on successful deletion, `filesize` in bytes.
    ///
    /// This approach can be considered rather conservative.
    ///
    /// While `disklru` was built upon `sled`, which utilizes [LSM-Tree like data structure][0],
    /// to reduce random seeking, write amplification will be huge.
    ///
    /// [0]: https://github.com/spacejam/sled?tab=readme-ov-file#performance
    pub fn remove_lru_file(&mut self) -> LRUResult<Option<u64>> {
        let Some(file) = self.least_recently_used_value()? else {
            return Ok(None);
        };
        remove_file_get_size(file)
    }

    /// Similar to [`remove_lru_file`](Self::remove_lru_file), but with custom key.
    ///
    /// Note: both functions **does not** pop the key-value pair out.
    pub fn remove_file(&mut self, key: &K) -> LRUResult<Option<u64>> {
        let Some(file) = self.access(key)? else {
            return Ok(None);
        };

        remove_file_get_size(file)
    }

    /// Remove old file pointed by `key` if existing,
    ///
    /// - `exceed_size`: exceeded size to place the new image, in bytes.
    ///
    /// For example, in an 2000 MiB pool that only 0.5 MiB are available,
    /// to insert a new 0.7 MiB file, we need to remove files of at least 0.3 MiB.
    ///
    /// Thus, pass 314573 (ceil(0.3*1024*1024)) to exceed.
    ///
    /// exceed = newfile_size - avaliable_capacity
    ///
    /// (Do NOT minus the old file size here, unless you had manually removed it!)
    ///
    /// ## Note
    ///
    /// On failed file deletion, this function will fail early.
    ///
    /// If after deleting every file, exceed_size is still positive,
    /// this will fail with [`LRUError`](LRUError)::
    ///
    /// Carefully handle such large file before calling this, or your cache maybe cleaned by accident.
    pub fn insert_new_file(
        &mut self,
        key: &K,
        path: &PathBuf,
        mut exceed_size: isize,
    ) -> LRUResult<Option<PathBuf>> {
        let mut old_value = None;

        // try to remove old file, on confliction
        if let Some(file) = self.access(key)? {
            if let Ok(Some(file_size)) = remove_file_get_size(&file) {
                exceed_size -= file_size as isize;
            }
            old_value = Some(file);
        }

        while exceed_size >= 0 {
            if self.least_recently_used()?.is_none() {
                return Err(LRUError::InsufficientCapacity);
            }
            if let Some(file_sz) = self.remove_lru_file()? {
                exceed_size -= file_sz as isize;
            }
        }

        self.insert(key, &path)
    }
}

impl<K, V> AsRef<Store<K, V>> for LruCache<K, V>
where
    K: Serialize + DeserializeOwned + Eq,
    V: Serialize + DeserializeOwned,
{
    fn as_ref(&self) -> &Store<K, V> {
        &self.inner
    }
}

impl<K, V> AsMut<Store<K, V>> for LruCache<K, V>
where
    K: Serialize + DeserializeOwned + Eq,
    V: Serialize + DeserializeOwned,
{
    fn as_mut(&mut self) -> &mut Store<K, V> {
        &mut self.inner
    }
}

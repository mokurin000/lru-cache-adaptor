use std::path::PathBuf;

use disklru::Store;
use serde::{Serialize, de::DeserializeOwned};

mod error;
mod utils;

pub use disklru;
pub type LRUResult<K> = std::result::Result<K, LRUError>;

pub use error::LRUError;
use utils::remove_file_get_size;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileInfo<K> {
    pub key: K,
    pub file_path: PathBuf,
    pub file_size: u64,
}

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

    /// Try access by an existing key, without touching LRU order
    ///
    /// If not found, returns `None`.
    pub fn peek(&mut self, key: &K) -> LRUResult<Option<V>> {
        Ok(self.inner.peek(key)?)
    }

    /// Return `Some(V)` if a value was replaced.
    pub fn insert(&mut self, key: &K, value: &V) -> LRUResult<Option<V>> {
        Ok(self.inner.insert(key, value)?)
    }

    pub fn pop(&mut self, key: &K) -> LRUResult<Option<(K, V)>> {
        Ok(self.inner.pop(key)?)
    }

    pub fn pop_least_recently_used(&mut self) -> LRUResult<Option<(K, V)>> {
        Ok(self.inner.pop_lru()?)
    }

    /// most_recently_used family functions all does no effect on LRU order
    pub fn most_recently_used(&self) -> LRUResult<Option<K>> {
        Ok(self.inner.mru()?)
    }
    pub fn most_recently_used_value(&mut self) -> LRUResult<Option<V>> {
        Ok(self.inner.peek_mru()?)
    }
    pub fn most_recently_used_pair(&mut self) -> LRUResult<Option<(K, V)>> {
        let Some(mru_key) = self.most_recently_used()? else {
            return Ok(None);
        };
        Ok(self.inner.peek_key_value(&mru_key)?)
    }

    /// least_recently_used family functions all does no effect on LRU order
    pub fn least_recently_used(&self) -> LRUResult<Option<K>> {
        match self.inner.lru() {
            Ok(v) => Ok(v),
            Err(disklru::Error::ReportBug(_)) => Ok(None),
            Err(e) => Err(e)?,
        }
    }
    pub fn least_recently_used_value(&mut self) -> LRUResult<Option<V>> {
        Ok(self.inner.peek_lru()?)
    }
    pub fn least_recently_used_pair(&mut self) -> LRUResult<Option<(K, V)>> {
        let Some(lru_key) = self.least_recently_used()? else {
            return Ok(None);
        };
        Ok(self.inner.peek_key_value(&lru_key)?)
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
    /// This **does not** pop the key-value pair out.
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
    /// This **does not** pop the key-value pair out.
    pub fn remove_file(&mut self, key: &K) -> LRUResult<Option<u64>> {
        let Some(file) = self.access(key)? else {
            return Ok(None);
        };

        remove_file_get_size(file)
    }

    /// Remove old file pointed by `key` if existing, possibly removing some files.
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
    ) -> LRUResult<Vec<FileInfo<K>>> {
        let mut old_value = None;

        // try to remove old file, on confliction
        if let Some(file) = self.access(key)? {
            if let Ok(Some(file_size)) = remove_file_get_size(&file) {
                exceed_size -= file_size as isize;
            }
            old_value = Some(file);
        }

        let mut removed_files = Vec::new();

        while exceed_size >= 0 {
            let Some((lru_key, file_path)) = self.least_recently_used_pair()? else {
                return Err(LRUError::InsufficientCapacity);
            };

            if let Ok(Some(file_size)) = remove_file_get_size(&file_path) {
                self.pop(&lru_key)?;
                exceed_size -= file_size as isize;
                removed_files.push(FileInfo {
                    key: lru_key,
                    file_path,
                    file_size,
                });
            }
        }

        self.insert(key, &path)?;

        Ok(removed_files)
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

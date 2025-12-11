use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
};

use lru_cache_adaptor::{FileInfo, LRUError, LRUResult, LruCache};
use serde::{Serialize, de::DeserializeOwned};

fn main() -> LRUResult<()> {
    let mut cache = LruCache::new(disklru::Store::open_temporary(1024)?);

    let file_sizes = [512, 512, 768, 512, 32, 1536, 256, 256, 32];
    let total_capacity = 2048_isize;
    let mut used = 0_isize;

    assert!(file_sizes.iter().all(|size| size <= &total_capacity));

    for (i, &size) in file_sizes.iter().enumerate() {
        let path = format!("temp_{i}");

        // file_path not needed here, as insert_new_file
        // handles rotation inside LRU cache.
        for FileInfo { file_size, .. } in place_file(
            &mut cache,
            &i,
            &path,
            size as _,
            size - (total_capacity - used),
        )? {
            used -= file_size as isize;
        }
        used += size;

        println!("after inserting {path} ({size} B), used {used} of {total_capacity} bytes");
        println!(
            "lru content: {:?}",
            cache.as_ref().iter().flatten().collect::<Vec<_>>()
        );
    }

    if cache.as_ref().len() >= 2 {
        cache.access(&cache.least_recently_used()?.unwrap())?;
    }
    println!(
        "lru content after access: {:?}",
        cache.as_ref().iter().flatten().collect::<Vec<_>>()
    );

    let size = total_capacity + 1;
    let exceeded = size - (total_capacity - used);

    // Do NOT try to insert too large file.
    assert!(matches!(
        place_file(
            &mut cache,
            &file_sizes.len(),
            format!("temp_{}", file_sizes.len()),
            size as _,
            exceeded,
        ),
        Err(LRUError::InsufficientCapacity),
    ));

    // On too large file, cache will be flushed by accident.
    assert!(cache.as_ref().iter().count() == 0);
    println!(
        "lru content after bad insertion: {:?}",
        cache.as_ref().iter().flatten().collect::<Vec<_>>()
    );

    Ok(())
}

fn place_file<K>(
    cache: &mut LruCache<K, PathBuf>,
    key: &K,
    path: impl AsRef<Path>,
    size: u64,
    exceed: isize,
) -> LRUResult<Vec<FileInfo<K>>>
where
    K: Serialize + DeserializeOwned + Eq + std::fmt::Debug,
{
    let path = path.as_ref().to_path_buf();

    let file = OpenOptions::new().create(true).write(true).open(&path)?;
    file.set_len(size)?;

    let removed_files = cache.insert_new_file(key, &path, exceed)?;
    if !removed_files.is_empty() {
        println!("removed: {removed_files:#?}");
    }

    Ok(removed_files)
}

use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
};

use lru_cache_adaptor::{FileInfo, LRUError, LRUResult, LruCache};

fn main() -> LRUResult<()> {
    let mut cache = LruCache::new(disklru::Store::open_temporary(1024)?);

    let file_sizes = [512, 512, 768, 512, 1536];
    let total_capacity = 2048_isize;
    let mut used = 0_isize;

    assert!(file_sizes.iter().all(|size| size <= &total_capacity));

    for (i, &size) in file_sizes.iter().enumerate() {
        let path = format!("temp_{i}");

        // file_path not needed here, as insert_new_file
        // handles rotation inside LRU cache.
        for FileInfo { file_size, .. } in
            place_file(&mut cache, &path, size as _, size - (total_capacity - used))?
        {
            used -= file_size as isize;
        }
        used += size;

        println!("after inserting {path}, used {used} of {total_capacity} bytes");
    }

    let size = total_capacity + 1;
    let exceeded = size - (total_capacity - used);
    println!("size: {size} B, exceed: {exceeded} B");

    assert!(matches!(
        place_file(
            &mut cache,
            format!("temp_{}", file_sizes.len()),
            size as _,
            exceeded,
        ),
        Err(LRUError::InsufficientCapacity),
    ));

    Ok(())
}

fn place_file(
    cache: &mut LruCache<PathBuf, PathBuf>,
    path: impl AsRef<Path>,
    size: u64,
    exceed: isize,
) -> LRUResult<Vec<FileInfo>> {
    let path = path.as_ref().to_path_buf();

    let file = OpenOptions::new().create(true).write(true).open(&path)?;
    file.set_len(size)?;

    let removed_files = cache.insert_new_file(&path, &path, exceed)?;
    if !removed_files.is_empty() {
        println!("removed: {removed_files:#?}");
    }

    Ok(removed_files)
}

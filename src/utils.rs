use std::path::Path;

use crate::LRUResult;

pub fn remove_file_get_size(file: impl AsRef<Path>) -> LRUResult<Option<u64>> {
    let path = file.as_ref();
    let filesize = path.metadata()?.len();
    if std::fs::remove_file(path).is_ok() {
        Ok(Some(filesize))
    } else {
        Ok(None)
    }
}

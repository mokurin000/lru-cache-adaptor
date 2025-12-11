use std::{io::ErrorKind, path::Path};

use crate::LRUResult;

pub fn remove_file_get_size(file: impl AsRef<Path>) -> LRUResult<Option<u64>> {
    let path = file.as_ref();
    let filesize = path.metadata()?.len();

    match std::fs::remove_file(path) {
        Ok(_) => Ok(Some(filesize)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e)?,
    }
}

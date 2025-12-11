use std::{io::ErrorKind, path::Path};

use crate::LRUResult;

/// Returns Ok(Some(fil_size)) if:
/// - file exists
/// - removed it successfully
///
/// Returns Ok(None) when:
/// - file not existing
///
/// Returns error when permission errors occured, e.g.
pub fn remove_file_get_size(file: impl AsRef<Path>) -> LRUResult<Option<u64>> {
    let path = file.as_ref();
    let filesize = match path.metadata() {
        Ok(meta) => meta.len(),
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => Err(e)?,
    };

    match std::fs::remove_file(path) {
        Ok(_) => Ok(Some(filesize)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e)?,
    }
}

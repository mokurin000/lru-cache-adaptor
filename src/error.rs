use std::fmt::Display;

#[derive(Debug, thiserror::Error)]
pub enum LRUError {
    #[error("disklru: {0}")]
    DiskLRU(#[from] DiskLRUError),
    #[error("io: {0}")]
    IO(#[from] std::io::Error),
    #[error("the file was too large to place.")]
    InsufficientCapacity,
}

impl From<disklru::Error> for LRUError {
    fn from(value: disklru::Error) -> Self {
        Self::DiskLRU(DiskLRUError(value))
    }
}

#[derive(Debug)]
pub struct DiskLRUError(pub disklru::Error);

impl Display for DiskLRUError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

impl std::error::Error for DiskLRUError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

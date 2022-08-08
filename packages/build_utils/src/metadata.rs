use std::io;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
pub struct FsMetadata {
    pub path: PathBuf,
    pub modified: SystemTime,
}

impl FsMetadata {
    pub async fn from(path: PathBuf) -> io::Result<FsMetadata> {
        let modified = tokio::fs::metadata(&path).await?.modified()?;
        Ok(FsMetadata { path, modified })
    }

    pub async fn from_iter<T>(paths: T) -> io::Result<Vec<FsMetadata>>
    where
        T: IntoIterator<Item = PathBuf>,
    {
        tokio_stream::iter(paths)
            .then(FsMetadata::from)
            .collect()
            .await
    }
}

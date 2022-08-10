use futures::future::join_all;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

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
        let futures = paths.into_iter().map(FsMetadata::from).collect::<Vec<_>>();

        join_all(futures).await.into_iter().collect()
    }
}

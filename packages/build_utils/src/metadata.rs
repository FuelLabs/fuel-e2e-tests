use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs::read_dir;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

pub struct FileMetadata {
    pub path: PathBuf,
    pub modified: SystemTime,
}

pub(crate) async fn paths_in_dir(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let build_dir_entries = ReadDirStream::new(read_dir(dir).await?)
        .collect::<io::Result<Vec<_>>>()
        .await?;

    Ok(build_dir_entries
        .into_iter()
        .map(|entry| entry.path())
        .collect())
}

pub(crate) async fn read_metadata<T>(paths: T) -> Result<Vec<FileMetadata>, io::Error>
where
    T: IntoIterator<Item = PathBuf>,
{
    tokio_stream::iter(paths)
        .then(|path| async move {
            let modified = tokio::fs::metadata(&path).await?.modified()?;
            Ok::<_, io::Error>(FileMetadata { path, modified })
        })
        .collect()
        .await
}

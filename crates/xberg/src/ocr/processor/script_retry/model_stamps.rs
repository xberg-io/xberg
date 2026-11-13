use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MODEL_TIMESTAMP_MARGIN: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ModelStamps(Vec<Option<FileStamp>>);

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileStamp {
    path: PathBuf,
    device: u64,
    inode: u64,
    length: u64,
    modified: SystemTime,
    changed: SystemTime,
}

impl ModelStamps {
    // A recent write can share a filesystem clock tick with a second write. ~keep
    pub(super) fn read<'a>(
        directory: &Path,
        languages: impl IntoIterator<Item = &'a str>,
        implicit_osd: bool,
    ) -> Option<Self> {
        let cutoff = SystemTime::now().checked_sub(MODEL_TIMESTAMP_MARGIN)?;
        languages
            .into_iter()
            .map(|language| (language, false))
            .chain(implicit_osd.then_some(("osd", true)))
            .map(|(language, optional)| {
                let path = directory.join(format!("{language}.traineddata"));
                match FileStamp::read(&path, cutoff) {
                    Ok(stamp) => Some(Some(stamp)),
                    Err(error) if optional && error.kind() == std::io::ErrorKind::NotFound => Some(None),
                    Err(_) => None,
                }
            })
            .collect::<Option<Vec<_>>>()
            .map(Self)
    }
}

impl FileStamp {
    fn read(path: &Path, cutoff: SystemTime) -> std::io::Result<Self> {
        let path = path.canonicalize()?;
        let metadata = path.metadata()?;
        let changed = u64::try_from(metadata.ctime())
            .ok()
            .zip(u32::try_from(metadata.ctime_nsec()).ok())
            .and_then(|(seconds, nanos)| SystemTime::UNIX_EPOCH.checked_add(Duration::new(seconds, nanos)));
        let modified = metadata.modified()?;
        let Some(changed) = changed.filter(|changed| *changed < cutoff) else {
            return Err(std::io::Error::other("recent traineddata change"));
        };
        if !metadata.is_file() || modified >= cutoff {
            return Err(std::io::Error::other("unstable traineddata file"));
        }
        Ok(Self {
            path,
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified,
            changed,
        })
    }
}

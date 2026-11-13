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
        Self::from_metadata(path, metadata, cutoff)
    }

    fn from_metadata(path: PathBuf, metadata: std::fs::Metadata, cutoff: SystemTime) -> std::io::Result<Self> {
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

const MODEL_DIGEST_CAPACITY: usize = 64;

#[derive(Default)]
struct ModelDigests {
    entries: std::sync::Mutex<std::collections::VecDeque<(FileStamp, String)>>,
}

pub(in crate::ocr::processor) fn hash_model_file(path: &Path) -> crate::Result<String> {
    static DIGESTS: std::sync::LazyLock<ModelDigests> = std::sync::LazyLock::new(ModelDigests::default);
    DIGESTS.hash(path, SystemTime::now(), crate::cache::blake3_hash_open_file)
}

impl ModelDigests {
    fn hash(
        &self,
        path: &Path,
        now: SystemTime,
        hash_open_file: impl FnOnce(std::fs::File) -> crate::Result<String>,
    ) -> crate::Result<String> {
        let file = std::fs::File::open(path)
            .map_err(|error| crate::XbergError::cache(format!("Failed to open file for hashing: {error}")))?;
        let Some(cutoff) = now.checked_sub(MODEL_TIMESTAMP_MARGIN) else {
            return hash_open_file(file);
        };
        let before = path
            .canonicalize()
            .and_then(|resolved| FileStamp::from_metadata(resolved, file.metadata()?, cutoff));
        let Ok(before) = before else {
            return hash_open_file(file);
        };
        let cached = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .find(|(stamp, _)| stamp == &before)
            .map(|(_, digest)| digest.clone());
        let digest = match &cached {
            Some(digest) => digest.clone(),
            None => hash_open_file(file)?,
        };
        if FileStamp::read(path, cutoff).as_ref().ok() != Some(&before) {
            return Err(crate::XbergError::cache(format!(
                "Tesseract model '{}' changed while fingerprinting",
                path.display()
            )));
        }

        if cached.is_some() {
            return Ok(digest);
        }
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if !entries.iter().any(|(stamp, _)| stamp == &before) {
            if entries.len() == MODEL_DIGEST_CAPACITY {
                entries.pop_front();
            }
            entries.push_back((before, digest.clone()));
        }
        Ok(digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_model_reuses_digest_and_replacement_gets_a_new_digest() {
        let directory = tempfile::tempdir().unwrap();
        let model = directory.path().join("eng.traineddata");
        std::fs::write(&model, b"one").unwrap();
        let original_mtime = model.metadata().unwrap().modified().unwrap();
        let stable_time = || SystemTime::now() + 2 * MODEL_TIMESTAMP_MARGIN;
        let digests = ModelDigests::default();
        let first = digests
            .hash(&model, stable_time(), crate::cache::blake3_hash_open_file)
            .unwrap();
        let warm = digests
            .hash(&model, stable_time(), |_| panic!("stable model must avoid rehashing"))
            .unwrap();
        assert_eq!(first, warm);

        let replacement = directory.path().join("replacement");
        std::fs::write(&replacement, b"two").unwrap();
        std::fs::File::open(&replacement)
            .unwrap()
            .set_modified(original_mtime)
            .unwrap();
        std::fs::rename(replacement, &model).unwrap();
        let changed = digests
            .hash(&model, stable_time(), crate::cache::blake3_hash_open_file)
            .unwrap();
        assert_ne!(first, changed);
        assert_eq!(changed, crate::cache::blake3_hash_file(&model).unwrap());
    }

    #[test]
    fn digest_is_read_from_the_open_file_across_path_rollback() {
        let directory = tempfile::tempdir().unwrap();
        let model = directory.path().join("eng.traineddata");
        let parked = directory.path().join("parked.traineddata");
        std::fs::write(&model, b"one").unwrap();
        let expected = crate::cache::blake3_hash_file(&model).unwrap();
        let now = SystemTime::now() + 2 * MODEL_TIMESTAMP_MARGIN;
        let digests = ModelDigests::default();
        let result = digests.hash(&model, now, |file| {
            std::fs::rename(&model, &parked).unwrap();
            std::fs::write(&model, b"two").unwrap();
            let digest = crate::cache::blake3_hash_open_file(file).unwrap();
            assert_eq!(digest, expected);
            std::fs::rename(&parked, &model).unwrap();
            Ok(digest)
        });
        // Some filesystems update ctime on rename. Reject that stamp or return the original digest. ~keep
        if let Ok(digest) = result {
            assert_eq!(digest, expected);
        }
        assert_eq!(
            digests.hash(&model, now, crate::cache::blake3_hash_open_file).unwrap(),
            expected
        );
    }

    #[test]
    fn recent_models_are_hashed_each_time() {
        let directory = tempfile::tempdir().unwrap();
        let model = directory.path().join("eng.traineddata");
        std::fs::write(&model, b"one").unwrap();
        let digests = ModelDigests::default();
        let now = SystemTime::now();
        let calls = std::cell::Cell::new(0);
        for _ in 0..2 {
            digests
                .hash(&model, now, |file| {
                    calls.set(calls.get() + 1);
                    crate::cache::blake3_hash_open_file(file)
                })
                .unwrap();
        }
        assert_eq!(calls.get(), 2);
        assert!(digests.entries.lock().unwrap().is_empty());
    }
}

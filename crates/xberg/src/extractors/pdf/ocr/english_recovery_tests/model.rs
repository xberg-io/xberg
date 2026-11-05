use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(super) const MODEL_SHA: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";
const MODEL_SIZE: usize = 4_113_088;

pub(super) struct ModelDirectory {
    path: PathBuf,
    _storage: Option<tempfile::TempDir>,
}

impl std::ops::Deref for ModelDirectory {
    type Target = Path;
    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl ModelDirectory {
    pub(super) fn verify(&self) {
        verify(&std::fs::read(self.join("eng.traineddata")).expect("verified local English model must exist"));
    }
}

fn verify(bytes: &[u8]) {
    assert_eq!(bytes.len(), MODEL_SIZE);
    assert_eq!(hex::encode(Sha256::digest(bytes)), MODEL_SHA);
}

pub(super) fn verified_model_directory() -> ModelDirectory {
    let model = if let Some(path) = std::env::var_os("XBERG_2339_TESSDATA_DIR") {
        ModelDirectory {
            path: path.into(),
            _storage: None,
        }
    } else if let Some(bytes) = xberg_tesseract::bundled_eng_traineddata() {
        verify(bytes);
        let storage = tempfile::tempdir().expect("owned bundled model directory");
        std::fs::write(storage.path().join("eng.traineddata"), bytes).expect("materialize verified bundled English");
        ModelDirectory {
            path: storage.path().to_path_buf(),
            _storage: Some(storage),
        }
    } else {
        ModelDirectory {
            path: std::env::var_os("TESSDATA_PREFIX")
                .expect("provide verified local English or bundle-tessdata-eng")
                .into(),
            _storage: None,
        }
    };
    model.verify();
    model
}

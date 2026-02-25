//! R adapter for Kreuzberg R bindings
//!
//! This adapter benchmarks extraction via the R bindings using a subprocess.

use crate::adapters::subprocess::SubprocessAdapter;
use std::path::PathBuf;

/// R adapter using kreuzberg R package
pub struct RAdapter {
    inner: SubprocessAdapter,
}

impl RAdapter {
    /// Create a new R adapter
    ///
    /// # Arguments
    /// * `rscript_path` - Path to Rscript interpreter (e.g., "Rscript")
    /// * `r_lib_path` - Optional path to kreuzberg R package (for development)
    pub fn new(rscript_path: impl Into<PathBuf>, r_lib_path: Option<PathBuf>) -> Self {
        let mut env = vec![];

        if let Some(path) = r_lib_path {
            env.push(("R_LIBS".to_string(), path.to_string_lossy().to_string()));
        }

        let supported_formats = vec![
            "pdf", "docx", "doc", "xlsx", "xls", "pptx", "ppt", "txt", "md", "html", "xml", "json", "yaml", "toml",
            "eml", "msg", "zip", "tar", "gz", "jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        let inner = SubprocessAdapter::new("kreuzberg-r", rscript_path.into(), vec![], env, supported_formats);

        Self { inner }
    }

    /// Create adapter using default Rscript
    pub fn default_rscript() -> Self {
        Self::new("Rscript", None)
    }
}

impl std::ops::Deref for RAdapter {
    type Target = SubprocessAdapter;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::FrameworkAdapter;

    #[test]
    fn test_r_adapter_creation() {
        let adapter = RAdapter::default_rscript();
        assert_eq!(adapter.name(), "kreuzberg-r");
    }
}

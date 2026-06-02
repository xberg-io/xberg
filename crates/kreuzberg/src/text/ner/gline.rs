//! kreuzberg-gliner-rs ONNX backend for named-entity recognition.
//!
//! Wraps the `kreuzberg-gliner-rs` crate (a Kreuzberg fork of upstream gline-rs,
//! see <https://crates.io/crates/kreuzberg-gliner-rs>) to run
//! [GLiNER](https://github.com/urchade/GLiNER) span-mode models. The default
//! model is `urchade/gliner_multi-v2.1` — a 100 MB multilingual checkpoint that
//! covers PERSON / ORGANIZATION / LOCATION / DATE / EMAIL out of the box.
//!
//! The fork pins `ort = "=2.0.0-rc.12"`, matching the rest of the kreuzberg
//! workspace (paddle-ocr, layout-detection, embeddings, auto-rotate,
//! doc_orientation).
//!
//! Model files download lazily from HuggingFace on [`GlineBackend::new`]. The
//! CLI `kreuzberg warm --ner` pre-fetches the ONNX file via [`download_model`].

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use gliner::model::GLiNER;
use gliner::model::input::text::TextInput;
use gliner::model::params::Parameters;
use gliner::model::pipeline::span::SpanMode;
use orp::params::RuntimeParameters;

use crate::Result;
use crate::types::entity::{Entity, EntityCategory};

use super::backend::NerBackend;

/// HuggingFace repo for the pinned default model.
pub const DEFAULT_MODEL_REPO: &str = "urchade/gliner_multi-v2.1";

/// HuggingFace path to the ONNX weights inside the default repo.
pub const DEFAULT_MODEL_FILE: &str = "onnx/model.onnx";

/// HuggingFace path to the tokenizer inside the default repo.
pub const DEFAULT_TOKENIZER_FILE: &str = "tokenizer.json";

/// Pinned SHA-256 of the default ONNX model. Empty string disables the check.
/// We leave this empty until a canonical pin is taken from a verified release —
/// the engine still streams-verifies any pin that gets added later.
pub const DEFAULT_MODEL_SHA256: &str = "";

/// Pinned SHA-256 of the default tokenizer. Empty string disables the check.
pub const DEFAULT_TOKENIZER_SHA256: &str = "";

/// Names of additional GLiNER variants kreuzberg knows how to download.
///
/// Used by `kreuzberg warm --all-ner-models` to pre-fetch the entire fleet.
pub const KNOWN_MODELS: &[&str] = &[
    "urchade/gliner_multi-v2.1",
    "urchade/gliner_small-v2.1",
    "urchade/gliner_medium-v2.1",
    "urchade/gliner_large-v2.1",
    "knowledgator/gliner-x-large",
];

/// Default entity labels used when the caller supplies an empty `categories` slice.
const DEFAULT_LABELS: &[&str] = &["person", "organization", "location", "date", "email"];

/// Eagerly fetch a GLiNER model (ONNX + tokenizer) into the kreuzberg cache.
///
/// Returns the path to the cached ONNX file. The CLI `warm --ner` /
/// `--ner-model` / `--all-ner-models` flags delegate here.
pub fn download_model(repo_id: &str, _cache_dir: Option<PathBuf>) -> Result<PathBuf> {
    let model_path = crate::model_download::hf_download(repo_id, DEFAULT_MODEL_FILE).map_err(|e| {
        crate::KreuzbergError::Plugin {
            message: format!("Failed to download GLiNER model '{repo_id}': {e}"),
            plugin_name: "ner-gline".to_string(),
        }
    })?;
    if repo_id == DEFAULT_MODEL_REPO && !DEFAULT_MODEL_SHA256.is_empty() {
        crate::model_download::verify_sha256(&model_path, DEFAULT_MODEL_SHA256, "ner-gline-model").map_err(|e| {
            crate::KreuzbergError::validation(format!("GLiNER model SHA256 verification failed: {e}"))
        })?;
    }

    let tokenizer_path = crate::model_download::hf_download(repo_id, DEFAULT_TOKENIZER_FILE).map_err(|e| {
        crate::KreuzbergError::Plugin {
            message: format!("Failed to download GLiNER tokenizer for '{repo_id}': {e}"),
            plugin_name: "ner-gline".to_string(),
        }
    })?;
    if repo_id == DEFAULT_MODEL_REPO && !DEFAULT_TOKENIZER_SHA256.is_empty() {
        crate::model_download::verify_sha256(&tokenizer_path, DEFAULT_TOKENIZER_SHA256, "ner-gline-tokenizer")
            .map_err(|e| {
                crate::KreuzbergError::validation(format!("GLiNER tokenizer SHA256 verification failed: {e}"))
            })?;
    }
    tracing::info!(
        model_path = %model_path.display(),
        tokenizer_path = %tokenizer_path.display(),
        "kreuzberg-gliner-rs model downloaded"
    );
    Ok(model_path)
}

/// Map an [`EntityCategory`] to the label string GLiNER expects.
fn category_to_label(category: &EntityCategory) -> String {
    match category {
        EntityCategory::Person => "person".to_string(),
        EntityCategory::Organization => "organization".to_string(),
        EntityCategory::Location => "location".to_string(),
        EntityCategory::Date => "date".to_string(),
        EntityCategory::Time => "time".to_string(),
        EntityCategory::Money => "money".to_string(),
        EntityCategory::Percent => "percent".to_string(),
        EntityCategory::Email => "email".to_string(),
        EntityCategory::Phone => "phone".to_string(),
        EntityCategory::Url => "url".to_string(),
        EntityCategory::Custom(label) => label.clone(),
    }
}

/// Reverse-map a GLiNER class string to an [`EntityCategory`].
fn label_to_category(class: &str) -> EntityCategory {
    match class {
        "person" => EntityCategory::Person,
        "organization" => EntityCategory::Organization,
        "location" => EntityCategory::Location,
        "date" => EntityCategory::Date,
        "time" => EntityCategory::Time,
        "money" => EntityCategory::Money,
        "percent" => EntityCategory::Percent,
        "email" => EntityCategory::Email,
        "phone" => EntityCategory::Phone,
        "url" => EntityCategory::Url,
        other => EntityCategory::Custom(other.to_string()),
    }
}

/// kreuzberg-gliner-rs ONNX backend wrapper.
///
/// Holds an initialised [`GLiNER<SpanMode>`] behind an `Arc<Mutex<...>>` so the
/// model can be safely shared across async tasks (inference is synchronous and
/// serialised internally by the mutex).
pub struct GlineBackend {
    pub repo_id: String,
    pub model_path: PathBuf,
    pub tokenizer_path: PathBuf,
    // SAFETY: GLiNER<SpanMode> is Send (all sub-fields are Send). The Mutex
    // serialises concurrent inference calls, which is required by the
    // underlying ort::Session API (Session::run requires &mut self).
    model: Arc<Mutex<GLiNER<SpanMode>>>,
}

impl GlineBackend {
    /// Build a backend for `repo_id` (or the default model if `None`).
    ///
    /// Downloads the ONNX weights and tokenizer via `hf-hub` on first call.
    /// After this returns, inference is available without further I/O.
    pub fn new(repo_id: Option<&str>) -> Result<Self> {
        let repo = repo_id.unwrap_or(DEFAULT_MODEL_REPO).to_string();
        let model_path = download_model(&repo, None)?;
        let tokenizer_path = crate::model_download::hf_download(&repo, DEFAULT_TOKENIZER_FILE).map_err(|e| {
            crate::KreuzbergError::Plugin {
                message: format!("Failed to fetch tokenizer for '{repo}': {e}"),
                plugin_name: "ner-gline".to_string(),
            }
        })?;
        let gliner = GLiNER::<SpanMode>::new(
            Parameters::default(),
            RuntimeParameters::default(),
            &tokenizer_path,
            &model_path,
        )
        .map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to initialise GLiNER model for '{repo}': {e}"),
            plugin_name: "ner-gline".to_string(),
        })?;
        Ok(Self {
            repo_id: repo,
            model_path,
            tokenizer_path,
            model: Arc::new(Mutex::new(gliner)),
        })
    }
}

#[async_trait]
impl NerBackend for GlineBackend {
    async fn detect(&self, text: &str, categories: &[EntityCategory]) -> Result<Vec<Entity>> {
        let labels: Vec<String> = if categories.is_empty() {
            DEFAULT_LABELS.iter().map(|s| s.to_string()).collect()
        } else {
            categories.iter().map(category_to_label).collect()
        };
        let text = text.to_string();
        let backend = Arc::clone(&self.model);
        let model_path = self.model_path.clone();
        let tokenizer_path = self.tokenizer_path.clone();

        tokio::task::spawn_blocking(move || {
            let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            let input =
                TextInput::from_str(&[text.as_str()], &label_refs).map_err(|e| crate::KreuzbergError::Plugin {
                    message: format!("Failed to build GLiNER input: {e}"),
                    plugin_name: "ner-gline".to_string(),
                })?;
            let guard = backend.lock().map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("GLiNER inference lock poisoned: {e}"),
                plugin_name: "ner-gline".to_string(),
            })?;
            let output = guard.inference(input).map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("GLiNER inference failed for model '{}' (tokenizer '{}'): {e}", model_path.display(), tokenizer_path.display()),
                plugin_name: "ner-gline".to_string(),
            })?;
            drop(guard);

            let entities: Vec<Entity> = output
                .spans
                .into_iter()
                .next()
                .unwrap_or_default()
                .into_iter()
                .map(|span| {
                    let (start, end) = span.offsets();
                    Entity {
                        category: label_to_category(span.class()),
                        text: span.text().to_string(),
                        start: start as u32,
                        end: end as u32,
                        confidence: Some(span.probability()),
                    }
                })
                .collect();
            Ok(entities)
        })
        .await
        .map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("GLiNER spawn_blocking task panicked: {e}"),
            plugin_name: "ner-gline".to_string(),
        })?
    }

    /// Native zero-shot multi-label inference: passes the union of `categories`
    /// (as label strings) and `custom_labels` to a single GLiNER inference call.
    async fn detect_with_custom(
        &self,
        text: &str,
        categories: &[EntityCategory],
        custom_labels: &[String],
    ) -> Result<Vec<Entity>> {
        // Build a de-duplicated label set from canonical categories + custom labels.
        let mut labels: Vec<String> = if categories.is_empty() && custom_labels.is_empty() {
            DEFAULT_LABELS.iter().map(|s| s.to_string()).collect()
        } else {
            let mut seen = std::collections::HashSet::new();
            let mut result = Vec::new();
            for label in categories.iter().map(category_to_label).chain(custom_labels.iter().cloned()) {
                if seen.insert(label.clone()) {
                    result.push(label);
                }
            }
            result
        };

        if labels.is_empty() {
            labels = DEFAULT_LABELS.iter().map(|s| s.to_string()).collect();
        }

        let text = text.to_string();
        let backend = Arc::clone(&self.model);
        let model_path = self.model_path.clone();

        tokio::task::spawn_blocking(move || {
            let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            let input =
                TextInput::from_str(&[text.as_str()], &label_refs).map_err(|e| crate::KreuzbergError::Plugin {
                    message: format!("Failed to build GLiNER input: {e}"),
                    plugin_name: "ner-gline".to_string(),
                })?;
            let guard = backend.lock().map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("GLiNER inference lock poisoned: {e}"),
                plugin_name: "ner-gline".to_string(),
            })?;
            let output = guard.inference(input).map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("GLiNER inference failed for model '{}': {e}", model_path.display()),
                plugin_name: "ner-gline".to_string(),
            })?;
            drop(guard);

            let entities: Vec<Entity> = output
                .spans
                .into_iter()
                .next()
                .unwrap_or_default()
                .into_iter()
                .map(|span| {
                    let (start, end) = span.offsets();
                    Entity {
                        category: label_to_category(span.class()),
                        text: span.text().to_string(),
                        start: start as u32,
                        end: end as u32,
                        confidence: Some(span.probability()),
                    }
                })
                .collect();
            Ok(entities)
        })
        .await
        .map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("GLiNER spawn_blocking task panicked: {e}"),
            plugin_name: "ner-gline".to_string(),
        })?
    }
}

#[cfg(all(test, feature = "ner-onnx"))]
mod tests {
    use super::*;

    #[test]
    fn category_to_label_maps_all_canonical_variants() {
        assert_eq!(category_to_label(&EntityCategory::Person), "person");
        assert_eq!(category_to_label(&EntityCategory::Organization), "organization");
        assert_eq!(category_to_label(&EntityCategory::Location), "location");
        assert_eq!(category_to_label(&EntityCategory::Date), "date");
        assert_eq!(category_to_label(&EntityCategory::Time), "time");
        assert_eq!(category_to_label(&EntityCategory::Money), "money");
        assert_eq!(category_to_label(&EntityCategory::Percent), "percent");
        assert_eq!(category_to_label(&EntityCategory::Email), "email");
        assert_eq!(category_to_label(&EntityCategory::Phone), "phone");
        assert_eq!(category_to_label(&EntityCategory::Url), "url");
        assert_eq!(category_to_label(&EntityCategory::Custom("vessel".to_string())), "vessel");
    }

    #[test]
    fn label_to_category_maps_all_canonical_labels() {
        assert_eq!(label_to_category("person"), EntityCategory::Person);
        assert_eq!(label_to_category("organization"), EntityCategory::Organization);
        assert_eq!(label_to_category("location"), EntityCategory::Location);
        assert_eq!(label_to_category("date"), EntityCategory::Date);
        assert_eq!(label_to_category("time"), EntityCategory::Time);
        assert_eq!(label_to_category("money"), EntityCategory::Money);
        assert_eq!(label_to_category("percent"), EntityCategory::Percent);
        assert_eq!(label_to_category("email"), EntityCategory::Email);
        assert_eq!(label_to_category("phone"), EntityCategory::Phone);
        assert_eq!(label_to_category("url"), EntityCategory::Url);
        assert_eq!(label_to_category("unknown_label"), EntityCategory::Custom("unknown_label".to_string()));
    }

    #[test]
    fn category_to_label_roundtrips() {
        let categories = [
            EntityCategory::Person,
            EntityCategory::Organization,
            EntityCategory::Location,
            EntityCategory::Date,
            EntityCategory::Time,
            EntityCategory::Money,
            EntityCategory::Percent,
            EntityCategory::Email,
            EntityCategory::Phone,
            EntityCategory::Url,
        ];
        for category in &categories {
            let label = category_to_label(category);
            assert_eq!(label_to_category(&label), *category, "roundtrip failed for {category:?}");
        }
    }

    /// Smoke test — downloads the real model (~100 MB) and runs one inference.
    /// Excluded from normal CI; run with:
    ///   cargo test -p kreuzberg --features ner-onnx,ner --lib ner::gline -- --ignored
    #[ignore]
    #[tokio::test]
    async fn smoke_test_real_inference() {
        let backend = GlineBackend::new(None).expect("GlineBackend::new failed");
        let entities = backend
            .detect("Elon Musk founded SpaceX in Hawthorne, California.", &[])
            .await
            .expect("detect failed");
        assert!(!entities.is_empty(), "expected at least one entity");
        let texts: Vec<&str> = entities.iter().map(|e| e.text.as_str()).collect();
        assert!(texts.contains(&"Elon Musk") || texts.contains(&"SpaceX"), "expected at least one known entity, got: {texts:?}");
    }
}

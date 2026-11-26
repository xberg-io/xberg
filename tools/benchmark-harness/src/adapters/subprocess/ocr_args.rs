//! OCR-argument construction: building/forwarding CLI OCR flags and injecting Tesseract
//! configuration overrides into an adapter's `--config-json` payload.

use crate::types::OcrStatus;
use crate::{Error, Result};
use std::path::Path;

use super::SubprocessAdapter;

pub(super) fn config_json_enables_ocr(args: &[String]) -> bool {
    args.windows(2)
        .rev()
        .find(|pair| pair[0] == "--config-json")
        .and_then(|pair| serde_json::from_str::<serde_json::Value>(&pair[1]).ok())
        .and_then(|config| config.pointer("/ocr/enabled").and_then(serde_json::Value::as_bool))
        == Some(true)
}
pub(super) fn effective_ocr_config_from_args(args: &[String]) -> Option<serde_json::Value> {
    let cli_backend = args
        .windows(2)
        .rev()
        .find(|pair| pair[0] == "--ocr-backend")
        .map(|pair| pair[1].as_str());
    let cli_enabled = args.iter().enumerate().rev().find_map(|(index, arg)| {
        if arg == "--no-ocr" {
            return Some(false);
        }
        (arg == "--ocr").then(|| args.get(index + 1).is_none_or(|value| value != "false"))
    });
    if cli_enabled == Some(false) {
        return None;
    }

    let configured_ocr = args
        .windows(2)
        .rev()
        .find(|pair| pair[0] == "--config-json")
        .and_then(|pair| serde_json::from_str::<serde_json::Value>(&pair[1]).ok())
        .and_then(|config| config.get("ocr").cloned());
    if let Some(mut ocr) = configured_ocr {
        let object = ocr.as_object_mut()?;
        if cli_enabled != Some(true) && object.get("enabled").and_then(serde_json::Value::as_bool) == Some(false) {
            return None;
        }
        if cli_enabled == Some(true) {
            object.insert("enabled".to_string(), serde_json::Value::Bool(true));
        }
        if let Some(cli_backend) = cli_backend {
            object.insert(
                "backend".to_string(),
                serde_json::Value::String(cli_backend.to_string()),
            );
        }
        return Some(ocr);
    }

    (cli_enabled == Some(true)).then(|| match cli_backend {
        Some("tesseract") | None => serde_json::json!({
            "enabled": true,
            "backend": "tesseract"
        }),
        Some(backend) => serde_json::json!({
            "enabled": true,
            "backend": backend
        }),
    })
}
/// True if a JSON `ocr` object's backend — or any stage of a multi-stage `ocr.pipeline` — is
/// `"tesseract"`. Used to gate the Tesseract result-cache override below: only
/// Tesseract has an independent on-disk OCR result cache and PSM auto-selection to preserve.
pub(super) fn ocr_uses_tesseract(ocr_object: &serde_json::Map<String, serde_json::Value>) -> bool {
    if ocr_object.get("backend").and_then(serde_json::Value::as_str) == Some("tesseract") {
        return true;
    }
    ocr_object
        .get("pipeline")
        .and_then(|pipeline| pipeline.get("stages"))
        .and_then(serde_json::Value::as_array)
        .is_some_and(|stages| {
            stages.iter().any(|stage| {
                stage
                    .as_object()
                    .and_then(|stage_object| stage_object.get("backend"))
                    .and_then(serde_json::Value::as_str)
                    == Some("tesseract")
            })
        })
}
/// Sets `languages` on a JSON `ocr` object (and any Tesseract stage of a multi-stage
/// `ocr.pipeline`) without adding OCR fields a default user request lacks.
///
/// A `tesseract_config` that already exists (an explicit PSM preset) has its `language` and
/// `use_cache` refreshed — its `psm` is left untouched. Nothing is materialized otherwise: in
/// xberg the mere presence of `ocr.backend_options` or `ocr.tesseract_config` switches off
/// default-route OCR stages (PDF number recovery, hidden-layer and stored-scan retries, the
/// recognition orientation retry), so the cold OCR result cache comes from the request-level
/// `use_cache: false` instead (see [`inject_ocr_config_into_args`]). ~keep
pub(super) fn materialize_tesseract_ocr(
    ocr_object: &mut serde_json::Map<String, serde_json::Value>,
    languages: &[String],
) {
    ocr_object.insert("language".to_string(), serde_json::json!(languages));

    if ocr_object.get("backend").and_then(serde_json::Value::as_str) == Some("tesseract") {
        apply_tesseract_result_cache_control(ocr_object, languages);
    }

    if let Some(stages) = ocr_object
        .get_mut("pipeline")
        .and_then(|pipeline| pipeline.get_mut("stages"))
        .and_then(serde_json::Value::as_array_mut)
    {
        for stage in stages {
            let Some(stage_object) = stage.as_object_mut() else {
                continue;
            };
            if stage_object.get("backend").and_then(serde_json::Value::as_str) != Some("tesseract") {
                continue;
            }
            stage_object.insert("language".to_string(), serde_json::json!(languages));
            apply_tesseract_result_cache_control(stage_object, languages);
        }
    }
}
pub(super) fn apply_tesseract_result_cache_control(
    object: &mut serde_json::Map<String, serde_json::Value>,
    languages: &[String],
) {
    if let Some(tesseract_config) = object
        .get_mut("tesseract_config")
        .and_then(serde_json::Value::as_object_mut)
    {
        tesseract_config.insert("language".to_string(), serde_json::json!(languages));
        tesseract_config.insert("use_cache".to_string(), serde_json::json!(false));
    }
}
/// Resolves the effective language list for a Tesseract benchmark call: the fixture's
/// canonicalized `ocr_language` when present, else `["eng"]` — xberg's own default (see
/// `default_eng` in `crates/xberg/src/core/config/ocr.rs`) — so every Tesseract call gets a
/// concrete language to compute the matching auto-PSM from, not just fixtures that pin one.
pub(super) fn effective_tesseract_languages(ocr_language: Option<&str>) -> Vec<String> {
    ocr_language
        .map(crate::adapter::canonicalize_ocr_languages)
        .filter(|languages| !languages.is_empty())
        .unwrap_or_else(|| vec!["eng".to_string()])
}
/// Rewrites the request's effective OCR config so a Tesseract benchmark subprocess gets a
/// genuinely cold OCR result cache without leaving xberg's default OCR route (see
/// [`materialize_tesseract_ocr`]).
///
/// This is the final request-args boundary: it starts from [`effective_ocr_config_from_args`],
/// which already merges an `ocr` key that a `--config-json` value may carry with any CLI-only
/// `--ocr`/`--ocr-backend`/`--no-ocr` override — including the case where OCR is enabled purely
/// via a CLI flag (e.g. `request_args_from`'s force-OCR upgrade path) and the base
/// `--config-json` carries no `ocr` key at all. Whatever that merge produces is where the
/// rewritten Tesseract settings get written back via [`inject_ocr_config_into_args`] —
/// creating a `--config-json` flag if none existed — so this always applies, not just when a
/// `tesseract_config`-carrying `--config-json` was already present.
///
/// Applies unconditionally, even when `ocr_language` is `None` (defaults to `"eng"`), because
/// every Tesseract benchmark call needs its result cache disabled, not just fixtures that pin an
/// explicit language. Returns `None` when the args carry no enabled Tesseract OCR config to
/// rewrite (non-tesseract backend or OCR disabled) — callers must fall back to
/// [`xberg_ocr_language_args`] for those. ~keep
pub(super) fn apply_tesseract_ocr_override_to_args(args: &[String], ocr_language: Option<&str>) -> Option<Vec<String>> {
    let mut effective_ocr = effective_ocr_config_from_args(args)?;
    let ocr_object = effective_ocr.as_object_mut()?;
    if ocr_object.get("enabled").and_then(serde_json::Value::as_bool) != Some(true) || !ocr_uses_tesseract(ocr_object) {
        return None;
    }

    let languages = effective_tesseract_languages(ocr_language);
    materialize_tesseract_ocr(ocr_object, &languages);

    inject_ocr_config_into_args(args, effective_ocr)
}
/// Writes `ocr` into the request's `--config-json` value, replacing any `ocr` key it already
/// carries so the language settings win, and sets the request-level `use_cache: false`. Since
/// xberg 3159746f10 that flag disables the OCR result cache for the whole request, whatever a
/// backend option or `tesseract_config` says, so it is the cold-OCR guarantee. Creates a
/// `--config-json` flag when the request has none — the CLI-only OCR-enable path still needs
/// the cache disabled. Returns `None` (leaving the request untouched) if an existing
/// `--config-json` value fails to parse as a JSON object, rather than risk clobbering an
/// unparseable-but-intentional value.
pub(super) fn inject_ocr_config_into_args(args: &[String], ocr: serde_json::Value) -> Option<Vec<String>> {
    let mut new_args = args.to_vec();
    if let Some(config_index) = new_args.iter().rposition(|arg| arg == "--config-json") {
        let raw = new_args.get(config_index + 1)?;
        let mut config: serde_json::Value = serde_json::from_str(raw).ok()?;
        let object = config.as_object_mut()?;
        object.insert("ocr".to_string(), ocr);
        object.insert("use_cache".to_string(), serde_json::json!(false));
        new_args[config_index + 1] = config.to_string();
    } else {
        new_args.push("--config-json".to_string());
        new_args.push(serde_json::json!({ "use_cache": false, "ocr": ocr }).to_string());
    }
    Some(new_args)
}
pub(super) fn xberg_ocr_language_args(args: &[String], ocr_language: Option<&str>) -> Option<[String; 2]> {
    effective_ocr_config_from_args(args)?;
    let language = ocr_language.and_then(crate::adapter::canonical_ocr_language_arg)?;
    Some(["--ocr-language".to_string(), language])
}
pub(super) fn build_batch_file_configs(
    file_paths: &[&Path],
    ocr_languages: &[Option<String>],
    cwd: &Path,
    base_ocr: Option<&serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    let Some(base_ocr) = base_ocr else {
        return serde_json::Map::new();
    };
    let Some(base_ocr_object) = base_ocr.as_object() else {
        return serde_json::Map::new();
    };
    let uses_tesseract = ocr_uses_tesseract(base_ocr_object);

    file_paths
        .iter()
        .zip(ocr_languages)
        .filter_map(|(path, language)| {
            // Every Tesseract fixture gets a per-file language override, defaulting to "eng". The
            // result cache is disabled batch-wide by the base `--config-json`'s `use_cache: false`
            // (a batch-level field). A non-Tesseract fixture without an explicit language needs
            // no override: the base `--config-json` already carries its default language.
            let languages = match language.as_deref().map(crate::adapter::canonicalize_ocr_languages) {
                Some(languages) if !languages.is_empty() => languages,
                _ if uses_tesseract => vec!["eng".to_string()],
                _ => return None,
            };
            let absolute_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd.join(path)
            };
            let mut ocr = base_ocr.clone();
            let ocr_object = ocr.as_object_mut()?;
            if uses_tesseract {
                materialize_tesseract_ocr(ocr_object, &languages);
            } else {
                ocr_object.insert("language".to_string(), serde_json::json!(languages));
            }
            Some((
                absolute_path.to_string_lossy().into_owned(),
                serde_json::json!({ "ocr": ocr }),
            ))
        })
        .collect()
}

impl SubprocessAdapter {
    /// Build request arguments, upgrading an adapter configured with OCR disabled
    /// when the fixture explicitly requires OCR.
    pub(super) fn request_args_from(&self, base_args: &[String], force_ocr: bool) -> Vec<String> {
        let mut args = base_args.to_vec();
        if !force_ocr || self.ignore_fixture_force_ocr {
            return args;
        }

        let is_xberg = self.name.starts_with("xberg-");
        let has_cli_ocr_override = args.iter().any(|arg| matches!(arg.as_str(), "--ocr" | "--no-ocr"));
        let preserve_configured_xberg_ocr = is_xberg && !has_cli_ocr_override && config_json_enables_ocr(base_args);
        if !preserve_configured_xberg_ocr {
            if let Some(index) = args.iter().position(|arg| arg == "--no-ocr") {
                args[index] = "--ocr".to_string();
            } else if let Some(index) = args.iter().position(|arg| arg == "--ocr") {
                if let Some(value) = args.get_mut(index + 1)
                    && matches!(value.as_str(), "true" | "false")
                {
                    *value = "true".to_string();
                }
            } else {
                args.push("--ocr".to_string());
            }
        }

        if is_xberg {
            if let Some(index) = args.iter().position(|arg| arg == "--force-ocr") {
                if let Some(value) = args.get_mut(index + 1) {
                    *value = "true".to_string();
                }
            } else {
                args.extend(["--force-ocr".to_string(), "true".to_string()]);
            }
        }

        args
    }
    pub(super) fn request_args(&self, force_ocr: bool) -> Vec<String> {
        self.request_args_from(&self.args, force_ocr)
    }
    /// Build the single-token `--flag=<language>` argument that forwards a
    /// fixture's OCR language to an external wrapper, or `None` when the adapter
    /// forwards no language (flag unconfigured or fixture pins none). Emitted as
    /// one token to match the wrappers' `--key=value` parsing and to avoid
    /// collision with the positional file path.
    pub(super) fn ocr_language_forward_arg(&self, ocr_language: Option<&str>) -> Option<String> {
        let flag = self.ocr_language_arg.as_deref()?;
        let language = ocr_language.and_then(crate::adapter::canonical_ocr_language_arg)?;
        Some(format!("{flag}={language}"))
    }
    pub(super) fn batch_ocr_language_forward_arg(&self, ocr_languages: &[Option<String>]) -> Result<Option<String>> {
        if !self.ocr_language_policy.requires_homogeneous_batch_language() {
            return Ok(None);
        }
        let Some(first) = ocr_languages.first() else {
            return Ok(None);
        };
        let expected = self.ocr_language_policy.partition_key(first.as_deref());
        if ocr_languages
            .iter()
            .any(|language| self.ocr_language_policy.partition_key(language.as_deref()) != expected)
        {
            return Err(Error::Config(format!(
                "framework '{}' received a native batch with mixed OCR languages",
                self.name
            )));
        }
        Ok(self.ocr_language_forward_arg(first.as_deref()))
    }
    pub(super) fn resolve_ocr_status(&self, value: Option<&serde_json::Value>, force_ocr: bool) -> OcrStatus {
        value
            .and_then(serde_json::Value::as_bool)
            .map(|used| if used { OcrStatus::Used } else { OcrStatus::NotUsed })
            .or_else(|| (force_ocr && !self.ignore_fixture_force_ocr).then_some(OcrStatus::Used))
            .or(self.configured_ocr_status)
            .unwrap_or(OcrStatus::Unknown)
    }
}

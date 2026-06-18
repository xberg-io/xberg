//! Merge a [`Preset`] with caller-supplied overrides (custom schema + context).

use std::collections::BTreeMap;

use crate::presets::types::{CallMode, MergeMode, Preset};
use serde::{Deserialize, Serialize};

/// Errors produced while resolving a preset against caller overrides.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// A custom schema override was supplied but is not a JSON object.
    #[error("custom schema must be a JSON object")]
    SchemaNotObject,
}

/// A preset merged with caller-supplied overrides (custom schema, prompt suffix,
/// context map). Output is what the pipeline orchestrator consumes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPreset {
    /// Source preset identifier.
    pub id: String,
    /// Source preset version.
    pub version: String,
    /// Fingerprint of the source preset file, used as a cache token.
    pub fingerprint: String,
    /// Schema name forwarded to the LLM.
    pub schema_name: String,
    /// Effective JSON Schema (caller override or the preset's own).
    pub schema: serde_json::Value,
    /// System prompt with rendered context appended.
    pub system_prompt: String,
    /// Merge strategy for paginated outputs.
    pub merge_mode: MergeMode,
    /// Preferred call mode.
    pub preferred_call_mode: CallMode,
    /// Whether the prompt asks for per-field citations.
    pub emit_citations: bool,
}

/// Resolve `(preset, custom_schema_override, context)` into a [`ResolvedPreset`].
///
/// - `custom_schema` overrides `preset.schema` when set.
/// - `context` substitutes `{{key}}` tokens in `preset.context_template`; the
///   rendered string is appended to `system_prompt` so the model sees it.
pub fn resolve(
    preset: &Preset,
    custom_schema: Option<serde_json::Value>,
    context: &BTreeMap<String, String>,
) -> Result<ResolvedPreset, ResolveError> {
    let schema = match custom_schema {
        Some(s) if !s.is_object() => return Err(ResolveError::SchemaNotObject),
        Some(s) => s,
        None => preset.schema.clone(),
    };

    let mut system_prompt = preset.system_prompt.clone();
    if let Some(tpl) = preset.context_template.as_deref() {
        let rendered = render_context(tpl, context);
        if !rendered.trim().is_empty() {
            system_prompt.push_str("\n\nContext:\n");
            system_prompt.push_str(&rendered);
        }
    }

    Ok(ResolvedPreset {
        id: preset.id.clone(),
        version: preset.version.clone(),
        fingerprint: preset.fingerprint.clone(),
        schema_name: preset.schema_name.clone(),
        schema,
        system_prompt,
        merge_mode: preset.merge_mode,
        preferred_call_mode: preset.preferred_call_mode,
        emit_citations: preset.emit_citations,
    })
}

/// Lightweight `{{key}}` substitution. Unmatched keys are left in place so the
/// model sees the literal placeholder rather than silently dropping context.
///
/// Walks the input by str slice to preserve multi-byte UTF-8 characters in the
/// surrounding template (em-dashes, accented letters, …).
fn render_context(template: &str, ctx: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 2..];
        match after_open.find("}}") {
            Some(close) => {
                let key = after_open[..close].trim();
                match ctx.get(key) {
                    Some(value) => out.push_str(value),
                    None => {
                        out.push_str("{{");
                        out.push_str(key);
                        out.push_str("}}");
                    }
                }
                rest = &after_open[close + 2..];
            }
            None => {
                // Unterminated `{{` — emit verbatim and stop scanning.
                out.push_str("{{");
                rest = after_open;
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::types::{PresetCategory, PresetSample};

    fn fixture() -> Preset {
        Preset {
            id: "generic_document".into(),
            version: "v1".into(),
            schema_name: "generic_document".into(),
            description: "test".into(),
            category: PresetCategory::Other,
            tags: vec![],
            schema: serde_json::json!({"type": "object", "properties": {"title": {"type": "string"}}}),
            system_prompt: "Extract a title and one-sentence summary from the document.".into(),
            context_template: Some("Vendor: {{vendor}}\nLocale: {{locale}}".into()),
            merge_mode: MergeMode::ObjectMerge,
            preferred_call_mode: CallMode::TextOnly,
            emit_citations: false,
            sample: Some(PresetSample {
                input_path: "samples/doc.pdf".into(),
                output_path: "samples/doc.result.json".into(),
            }),
            fingerprint: "sha256:test".into(),
        }
    }

    #[test]
    fn resolves_without_overrides_uses_preset_schema() {
        let preset = fixture();
        let resolved = resolve(&preset, None, &BTreeMap::new()).unwrap();
        assert_eq!(resolved.schema, preset.schema);
        assert_eq!(resolved.fingerprint, "sha256:test");
    }

    #[test]
    fn custom_schema_overrides_preset_schema() {
        let preset = fixture();
        let custom = serde_json::json!({"type": "object", "properties": {"x": {"type": "string"}}});
        let resolved = resolve(&preset, Some(custom.clone()), &BTreeMap::new()).unwrap();
        assert_eq!(resolved.schema, custom);
    }

    #[test]
    fn rejects_non_object_custom_schema() {
        let preset = fixture();
        let err = resolve(
            &preset,
            Some(serde_json::Value::String("oops".into())),
            &BTreeMap::new(),
        )
        .unwrap_err();
        assert!(matches!(err, ResolveError::SchemaNotObject));
    }

    #[test]
    fn context_substitution_writes_into_prompt() {
        let preset = fixture();
        let mut ctx = BTreeMap::new();
        ctx.insert("vendor".into(), "Acme".into());
        ctx.insert("locale".into(), "en-US".into());
        let resolved = resolve(&preset, None, &ctx).unwrap();
        assert!(resolved.system_prompt.contains("Vendor: Acme"));
        assert!(resolved.system_prompt.contains("Locale: en-US"));
    }

    #[test]
    fn missing_context_keys_remain_as_placeholders() {
        let preset = fixture();
        let ctx = BTreeMap::new();
        let resolved = resolve(&preset, None, &ctx).unwrap();
        assert!(resolved.system_prompt.contains("{{vendor}}"));
        assert!(resolved.system_prompt.contains("{{locale}}"));
    }

    #[test]
    fn template_preserves_multibyte_utf8() {
        let mut preset = fixture();
        preset.context_template = Some("Vendor — {{vendor}}\nNotes: café · α · 中 · א".into());
        let mut ctx = BTreeMap::new();
        ctx.insert("vendor".into(), "Acmé".into());
        let resolved = resolve(&preset, None, &ctx).unwrap();
        assert!(resolved.system_prompt.contains("Vendor — Acmé"));
        assert!(resolved.system_prompt.contains("café · α · 中 · א"));
    }

    #[test]
    fn unterminated_brace_pair_is_emitted_verbatim() {
        let mut preset = fixture();
        preset.context_template = Some("Tail: {{never_closed".into());
        let resolved = resolve(&preset, None, &BTreeMap::new()).unwrap();
        assert!(resolved.system_prompt.contains("Tail: {{never_closed"));
    }
}

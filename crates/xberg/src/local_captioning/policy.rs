use std::collections::BTreeSet;
use std::path::{Component, Path};

use sha2::{Digest, Sha256};

use super::types::{MAX_IMAGE_BYTES, MAX_PIXELS, MAX_PROMPT_BYTES};
use super::{LocalCaptioningConfig, LocalCaptioningFile, LocalCaptioningFormat, LocalCaptioningModel};

const REVISION_HEX_LENGTH: usize = 40;
const MAX_NEW_TOKENS: u32 = 128;
const MAX_REPOSITORY_BYTES: usize = 256;
const MAX_ARTIFACT_PATH_BYTES: usize = 1024;
const MAX_FILES: usize = 64;
const MAX_CACHE_PATH_BYTES: usize = 4096;

const SMOLVLM_COMPANIONS: [&str; 6] = [
    "tokenizer.json",
    "tokenizer_config.json",
    "preprocessor_config.json",
    "processor_config.json",
    "config.json",
    "generation_config.json",
];

/// Florence-2 stores its task tokens outside the tokenizer JSON.
const FLORENCE_COMPANIONS: [&str; 7] = [
    "tokenizer.json",
    "tokenizer_config.json",
    "preprocessor_config.json",
    "processor_config.json",
    "config.json",
    "generation_config.json",
    "added_tokens.json",
];

pub(super) fn invalid(message: impl Into<String>) -> crate::XbergError {
    crate::XbergError::Validation {
        message: message.into(),
        source: None,
    }
}

fn is_repository_id(value: &str) -> bool {
    value.split_once('/').is_some_and(|(owner, name)| {
        [owner, name].into_iter().all(|part| {
            !part.is_empty()
                && !matches!(part, "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        })
    })
}

/// A contained repository-relative file path: no root, no `.`/`..` component, no trailing slash and
/// no alternate separator or scheme.
pub(super) fn is_artifact_path(path: &str) -> bool {
    !path.is_empty()
        && !path.ends_with('/')
        && !path.contains(['\\', ':'])
        && !Path::new(path).is_absolute()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_commit_hash(value: &str) -> bool {
    value.len() == REVISION_HEX_LENGTH && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn validate_config(config: &LocalCaptioningConfig) -> crate::Result<()> {
    validate_model(&config.model)?;
    for (value, ceiling, label) in [
        (config.limits.max_image_bytes, MAX_IMAGE_BYTES, "image byte"),
        (config.limits.max_pixels, MAX_PIXELS, "pixel"),
        (config.limits.max_new_tokens, MAX_NEW_TOKENS, "generated token"),
        (config.limits.max_prompt_bytes, MAX_PROMPT_BYTES, "prompt byte"),
    ] {
        if value == 0 || value > ceiling {
            return Err(invalid(format!(
                "Local captioning {label} limit must be positive and cannot exceed the SDK ceiling"
            )));
        }
    }
    if config.model.format == LocalCaptioningFormat::Florence2 {
        return Err(invalid(
            "Florence-2 captioning is staged in the inventory but not enabled by this adapter: its merged decoder fails on cached decoder steps in the bundled ONNX Runtime. Only SmolVLM is wired; no other family is substituted.",
        ));
    }
    if let Some(prompt) = &config.prompt {
        if prompt.trim().is_empty() || prompt.len() > config.limits.max_prompt_bytes as usize {
            return Err(invalid(
                "Local captioning prompt must be nonblank and within its byte limit",
            ));
        }
        if config.model.format == LocalCaptioningFormat::Florence2 {
            return Err(invalid(
                "Florence-2 captioning exposes a fixed task vocabulary and does not accept a custom prompt",
            ));
        }
    }
    if config
        .cache_directory
        .as_ref()
        .is_some_and(|directory| directory.len() > MAX_CACHE_PATH_BYTES || directory.trim().is_empty())
    {
        return Err(invalid(
            "Local captioning cache directory must be nonblank and within its byte limit",
        ));
    }
    Ok(())
}

fn validate_model(model: &LocalCaptioningModel) -> crate::Result<()> {
    if !is_repository_id(&model.repository)
        || model.repository.len() > MAX_REPOSITORY_BYTES
        || !is_commit_hash(&model.revision)
    {
        return Err(invalid(
            "Local captioning requires an owner/name repository and an immutable forty-digit hexadecimal revision",
        ));
    }
    if model.files.is_empty() || model.files.len() > MAX_FILES {
        return Err(invalid("Local captioning requires a bounded, nonempty artifact set"));
    }
    let mut paths = BTreeSet::new();
    for file in &model.files {
        if !is_artifact_path(&file.path) || file.path.len() > MAX_ARTIFACT_PATH_BYTES {
            return Err(invalid(format!(
                "Local captioning artifact path is not a contained repository path: {}",
                file.path
            )));
        }
        if !is_sha256(&file.sha256) || file.size_bytes == 0 {
            return Err(invalid(format!(
                "Local captioning artifact lacks a SHA-256 or size: {}",
                file.path
            )));
        }
        if !paths.insert(file.path.as_str()) {
            return Err(invalid(format!(
                "Local captioning artifact is listed twice: {}",
                file.path
            )));
        }
    }
    for (label, path) in [
        ("vision-encoder", Some(&model.vision_encoder_file)),
        ("embedding", Some(&model.embed_tokens_file)),
        ("decoder", Some(&model.decoder_file)),
        ("text-encoder", model.text_encoder_file.as_ref()),
    ] {
        let Some(path) = path else { continue };
        if !is_artifact_path(path) || !paths.contains(path.as_str()) {
            return Err(invalid(format!(
                "Local captioning {label} graph {path} is not a listed artifact"
            )));
        }
    }
    match model.format {
        LocalCaptioningFormat::SmolVlm if model.text_encoder_file.is_some() => {
            return Err(invalid("SmolVLM captioning has no separate text encoder"));
        }
        LocalCaptioningFormat::Florence2 if model.text_encoder_file.is_none() => {
            return Err(invalid("Florence-2 captioning requires a text-encoder graph"));
        }
        _ => {}
    }
    let companions = match model.format {
        LocalCaptioningFormat::SmolVlm => &SMOLVLM_COMPANIONS[..],
        LocalCaptioningFormat::Florence2 => &FLORENCE_COMPANIONS[..],
    };
    for companion in companions {
        if !model
            .files
            .iter()
            .any(|file| file.path.rsplit_once('/').is_some_and(|(_, name)| name == *companion))
        {
            return Err(invalid(format!("Local captioning artifact set lacks {companion}")));
        }
    }
    Ok(())
}

/// Stable digest of the descriptor: repository, revision, format and every pinned path, digest and
/// size. Recorded on the model info so a prepared session can be matched to its inventory.
pub(super) fn manifest_digest(model: &LocalCaptioningModel) -> String {
    let mut files: Vec<&LocalCaptioningFile> = model.files.iter().collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut hash = Sha256::new();
    for value in [
        model.repository.as_str(),
        model.revision.as_str(),
        match model.format {
            LocalCaptioningFormat::SmolVlm => "smolvlm",
            LocalCaptioningFormat::Florence2 => "florence2",
        },
        model.vision_encoder_file.as_str(),
        model.embed_tokens_file.as_str(),
        model.decoder_file.as_str(),
        model.text_encoder_file.as_deref().unwrap_or(""),
    ] {
        hash.update((value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    }
    for file in files {
        for value in [file.path.as_str(), file.sha256.as_str()] {
            hash.update((value.len() as u64).to_le_bytes());
            hash.update(value.as_bytes());
        }
        hash.update(file.size_bytes.to_le_bytes());
    }
    hex::encode(hash.finalize())
}

struct SelectedGraphs<'a> {
    vision: &'a str,
    embed: &'a str,
    decoder: &'a str,
    text_encoder: Option<&'a str>,
    format: LocalCaptioningFormat,
}

fn select_graphs<'a>(
    id: &str,
    relatives: &[(&'a str, &crate::candidate_models::CandidateFile)],
    precision: crate::candidate_models::CandidatePrecision,
) -> Result<SelectedGraphs<'a>, String> {
    use crate::candidate_models::CandidatePrecision;

    let graph = |name: &str| -> Result<Option<&'a str>, String> {
        let mut matches = relatives
            .iter()
            .filter(|(relative, _)| relative.starts_with("onnx/") && relative.ends_with(".onnx"))
            .filter(|(relative, _)| {
                let base = relative.rsplit_once('/').map_or(*relative, |(_, base)| base);
                base == format!("{name}.onnx") || base == format!("{name}_int8.onnx")
            })
            .map(|(relative, _)| *relative);
        let first = matches.next();
        if matches.next().is_some() {
            return Err(format!("{id} selects more than one {name} graph"));
        }
        Ok(first)
    };

    let vision = graph("vision_encoder")?.ok_or_else(|| format!("{id} has no vision-encoder graph"))?;
    let embed = graph("embed_tokens")?.ok_or_else(|| format!("{id} has no embedding graph"))?;
    let decoder = graph("decoder_model_merged")?.ok_or_else(|| format!("{id} has no merged decoder graph"))?;
    let text_encoder = graph("encoder_model")?;

    let quantized = decoder.ends_with("_int8.onnx");
    let expected = matches!(precision, CandidatePrecision::Quantized);
    for (name, relative) in [("vision-encoder", vision), ("embedding", embed), ("decoder", decoder)]
        .into_iter()
        .chain(text_encoder.map(|relative| ("text-encoder", relative)))
    {
        if relative.ends_with("_int8.onnx") != quantized {
            return Err(format!(
                "{id} mixes quantization variants: {name} graph {relative} is not {precision:?}"
            ));
        }
    }
    if quantized != expected {
        return Err(format!("{id} has no {precision:?} graph set"));
    }
    Ok(SelectedGraphs {
        vision,
        embed,
        decoder,
        text_encoder,
        format: if text_encoder.is_some() {
            LocalCaptioningFormat::Florence2
        } else {
            LocalCaptioningFormat::SmolVlm
        },
    })
}

/// Assembles the descriptor from an inventory selection. Graph roles come from the export's own
/// file names, so no revision, hash, size or graph path is hard-coded here.
pub(crate) fn descriptor_from_selection(
    repository: &str,
    revision: &str,
    id: &str,
    precision: crate::candidate_models::CandidatePrecision,
    files: &[crate::candidate_models::CandidateFile],
) -> Result<LocalCaptioningModel, String> {
    let prefix = format!("{id}/");
    let mut relatives: Vec<(&str, &crate::candidate_models::CandidateFile)> = files
        .iter()
        .filter_map(|file| file.path.strip_prefix(&prefix).map(|relative| (relative, file)))
        .collect();
    relatives.sort_by(|a, b| a.0.cmp(b.0));
    let graphs = select_graphs(id, &relatives, precision)?;
    let selected = relatives
        .iter()
        .map(|(_, file)| LocalCaptioningFile {
            path: file.path.clone(),
            sha256: file.sha256.clone(),
            size_bytes: file.size_bytes,
        })
        .collect();
    let descriptor = LocalCaptioningModel {
        repository: repository.to_owned(),
        revision: revision.to_owned(),
        format: graphs.format,
        vision_encoder_file: format!("{prefix}{}", graphs.vision),
        embed_tokens_file: format!("{prefix}{}", graphs.embed),
        text_encoder_file: graphs.text_encoder.map(|relative| format!("{prefix}{relative}")),
        decoder_file: format!("{prefix}{}", graphs.decoder),
        files: selected,
    };
    validate_model(&descriptor).map_err(|error| error.to_string())?;
    Ok(descriptor)
}

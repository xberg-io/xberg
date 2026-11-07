use sha2::{Digest, Sha256};

use super::artifacts::{ArtifactInventory, VerifiedArtifacts, read_artifacts};
use super::control::WorkerControl;
use super::metadata::validate_model_metadata;
use super::policy::{invalid, model_identity, validate_cpu_execution};
use super::schema::{TensorSpec, validate_schema};
use super::{LocalClassificationBackend, LocalClassificationConfig, LocalClassificationModelInfo};

pub(super) struct PreparedModel {
    pub(super) session: ort::session::Session,
    pub(super) tokenizer: tokenizers::Tokenizer,
    pub(super) max_tokens: u32,
    pub(super) has_type_ids: bool,
    pub(super) identity: String,
    pub(super) session_options: crate::onnx::OrtSessionOptions,
    pub(super) info: LocalClassificationModelInfo,
}

pub(super) fn validate_cpu_runtime() -> crate::Result<()> {
    let override_provider = crate::ort_discovery::execution_provider_override();
    validate_cpu_execution(override_provider.as_ref(), &[])?;
    let environment = crate::ort_discovery::ort_environment()
        .map_err(|error| invalid(format!("Local NLI CPU environment: {error}")))?;
    let providers = environment
        .execution_providers()
        .iter()
        .map(|provider| provider.name())
        .collect::<Vec<_>>();
    validate_cpu_execution(override_provider.as_ref(), &providers)
}

fn read_metadata(bytes: &[u8], name: &str) -> crate::Result<serde_json::Value> {
    serde_json::from_slice(bytes).map_err(|error| invalid(format!("Local NLI {name} JSON: {error}")))
}

fn effective_token_limit(
    config: &LocalClassificationConfig,
    metadata: &serde_json::Value,
    tokenizer_metadata: &serde_json::Value,
) -> crate::Result<u32> {
    let positions = metadata["max_position_embeddings"]
        .as_u64()
        .filter(|value| *value > 0)
        .ok_or_else(|| invalid("Local NLI model must declare a positive position limit"))?;
    let tokenizer_limit = tokenizer_metadata["model_max_length"].as_u64().unwrap_or(positions);
    if tokenizer_limit == 0 {
        return Err(invalid("Local NLI tokenizer position limit must be positive"));
    }
    Ok(u64::from(config.limits.max_tokens).min(positions).min(tokenizer_limit) as u32)
}

fn tensor_specs(outlets: &[ort::value::Outlet]) -> crate::Result<Vec<TensorSpec>> {
    outlets
        .iter()
        .map(|outlet| {
            let dtype = outlet.dtype();
            Ok(TensorSpec {
                name: outlet.name().to_owned(),
                element_type: dtype
                    .tensor_type()
                    .ok_or_else(|| invalid("Local NLI supports tensor inputs and outputs only"))?,
                dimensions: dtype
                    .tensor_shape()
                    .ok_or_else(|| invalid("Local NLI tensor shape is missing"))?
                    .to_vec(),
            })
        })
        .collect()
}

fn model_info(config: &LocalClassificationConfig, inventory: &ArtifactInventory) -> LocalClassificationModelInfo {
    let model = &config.model;
    LocalClassificationModelInfo {
        selected_backend: LocalClassificationBackend::OnnxCpu,
        effective_backend: LocalClassificationBackend::OnnxCpu,
        repository: model.repository.clone(),
        revision: model.revision.clone(),
        model_file: model.model_file.clone(),
        model_sha256: inventory.model_sha256().to_owned(),
        manifest_sha256: hex::encode(Sha256::digest(model.sha256_manifest.as_bytes())),
        experimental: true,
    }
}

fn load_tokenizer_assets(
    config: &LocalClassificationConfig,
    tokenizer_bytes: Vec<u8>,
    metadata_bytes: Vec<u8>,
    tokenizer_metadata_bytes: Vec<u8>,
    special_bytes: Vec<u8>,
) -> crate::Result<(tokenizers::Tokenizer, u32)> {
    use tokenizers::{PaddingParams, PaddingStrategy};
    let metadata = read_metadata(&metadata_bytes, "config.json")?;
    validate_model_metadata(&metadata, &config.model)?;
    let tokenizer_metadata = read_metadata(&tokenizer_metadata_bytes, "tokenizer_config.json")?;
    let max_tokens = effective_token_limit(config, &metadata, &tokenizer_metadata)?;
    let mut tokenizer = tokenizers::Tokenizer::from_bytes(tokenizer_bytes)
        .map_err(|error| invalid(format!("Local NLI tokenizer: {error}")))?;
    let pad_token = tokenizer_metadata["pad_token"].as_str().unwrap_or("[PAD]").to_owned();
    let pad_id = u32::try_from(metadata["pad_token_id"].as_u64().unwrap_or(0))
        .map_err(|error| invalid(format!("Local NLI padding token id: {error}")))?;
    tokenizer
        .with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            pad_token,
            pad_id,
            ..Default::default()
        }))
        .with_truncation(None)
        .map_err(|error| invalid(format!("Local NLI tokenizer truncation: {error}")))?;
    let special = read_metadata(&special_bytes, "special_tokens_map.json")?;
    let special = special
        .as_object()
        .ok_or_else(|| invalid("Local NLI special token map must be an object"))?;
    for value in special.values() {
        if let Some(token) = added_token(value) {
            tokenizer
                .add_special_tokens([token])
                .map_err(|error| invalid(format!("Local NLI special token: {error}")))?;
        }
    }
    Ok((tokenizer, max_tokens))
}

fn added_token(value: &serde_json::Value) -> Option<tokenizers::AddedToken> {
    let token = if let Some(content) = value.as_str() {
        tokenizers::AddedToken {
            content: content.to_owned(),
            special: true,
            ..Default::default()
        }
    } else {
        tokenizers::AddedToken {
            content: value.get("content")?.as_str()?.to_owned(),
            special: true,
            single_word: value.get("single_word")?.as_bool()?,
            lstrip: value.get("lstrip")?.as_bool()?,
            rstrip: value.get("rstrip")?.as_bool()?,
            normalized: value.get("normalized")?.as_bool()?,
        }
    };
    Some(token)
}

pub(super) fn load_model(config: &LocalClassificationConfig, control: &WorkerControl) -> crate::Result<PreparedModel> {
    control.check()?;
    validate_cpu_runtime()?;
    let inventory = ArtifactInventory::new(&config.model)?;
    let VerifiedArtifacts {
        graph,
        external,
        tokenizer,
        config: metadata,
        tokenizer_config,
        special_tokens,
    } = read_artifacts(config, &inventory, control)?;
    let (tokenizer, max_tokens) = load_tokenizer_assets(config, tokenizer, metadata, tokenizer_config, special_tokens)?;
    control.check()?;
    let session_options = crate::onnx::ort_session_options();
    let session = build_memory_session(graph, external, &session_options)?;
    control.check()?;
    validate_cpu_runtime()?;
    if session_options != crate::onnx::ort_session_options() {
        return Err(invalid(
            "ORT options changed during local NLI preparation; explicitly prepare again",
        ));
    }
    validate_schema(
        &tensor_specs(session.inputs())?,
        &tensor_specs(session.outputs())?,
        &config.model.output_name,
    )?;
    Ok(PreparedModel {
        has_type_ids: session.inputs().iter().any(|input| input.name() == "token_type_ids"),
        session,
        tokenizer,
        max_tokens,
        identity: model_identity(config),
        session_options,
        info: model_info(config, &inventory),
    })
}

fn build_memory_session(
    graph: Vec<u8>,
    external: Vec<(String, Vec<u8>)>,
    session_options: &crate::onnx::OrtSessionOptions,
) -> crate::Result<ort::session::Session> {
    let acceleration = crate::core::config::acceleration::AccelerationConfig {
        provider: crate::core::config::acceleration::ExecutionProviderType::Cpu,
        ..Default::default()
    };
    crate::onnx::build_session_with(session_options, Some(&acceleration), invalid, |builder| {
        let mut builder = builder.clone();
        for (name, bytes) in external {
            builder = builder
                .with_external_initializer_file_in_memory(name, bytes.into())
                .map_err(|error| ort::Error::new(error.message()))?;
        }
        builder.commit_from_memory(&graph)
    })
    .map_err(|error| crate::XbergError::Validation {
        message: format!("Local NLI external initializer session: {error}"),
        source: Some(Box::new(error)),
    })
}

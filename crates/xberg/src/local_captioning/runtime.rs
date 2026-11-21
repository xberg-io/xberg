use std::sync::{Arc, Mutex};
use std::time::Instant;

use ort::session::Session;

use super::artifacts::{VerifiedArtifacts, read_artifacts, stage};
use super::control::{WorkerControl, check_control, run_controlled, run_controlled_with_factory};
use super::generation::{KvShape, PreparedCaptioner, generate};
use super::policy::{invalid, manifest_digest, validate_config};
use super::preprocess::{PreprocessConfig, decode_png_or_common, preprocess};
use super::types::{
    LocalCaptioningBackend, LocalCaptioningFile, LocalCaptioningFormat, LocalCaptioningGraph, LocalCaptioningGraphRole,
    LocalCaptioningModelInfo, LocalCaptioningPrecision,
};
use super::{LocalCaptioningConfig, LocalCaptioningOptions, LocalCaptioningResult};

const MAX_CONCURRENT_REQUESTS: usize = 1;

struct CaptionerState {
    config: LocalCaptioningConfig,
    pool: crate::model_cache::ModelCache<PreparedModel>,
    prepared_options: Mutex<Option<crate::ort_options::OrtSessionOptions>>,
    semaphore: Arc<tokio::sync::Semaphore>,
}

/// Experimental Rust-only CPU image captioner for trusted, pre-staged ONNX and processor artifacts.
///
/// Graphs are loaded into memory and capped per artifact; the caps do not sandbox the model or
/// bound its parsed or native runtime memory. The async methods require a Tokio runtime with time
/// enabled. Nothing here is qualified or a supported preset.
#[cfg_attr(alef, alef(skip))]
pub struct LocalImageCaptioner {
    state: Arc<CaptionerState>,
}

struct PreparedModel {
    captioner: PreparedCaptioner,
    identity: String,
    session_options: crate::ort_options::OrtSessionOptions,
    info: LocalCaptioningModelInfo,
}

fn validate_cpu_runtime() -> crate::Result<()> {
    let override_provider = crate::ort_discovery::execution_provider_override();
    validate_cpu_execution(override_provider.as_ref(), &[])?;
    let environment = crate::ort_discovery::ort_environment()
        .map_err(|error| invalid(format!("Local captioning CPU environment: {error}")))?;
    let providers = environment
        .execution_providers()
        .iter()
        .map(|provider| provider.name())
        .collect::<Vec<_>>();
    validate_cpu_execution(override_provider.as_ref(), &providers)
}

fn validate_cpu_execution(
    provider: Option<&crate::core::config::acceleration::ExecutionProviderType>,
    environment_providers: &[&str],
) -> crate::Result<()> {
    use crate::core::config::acceleration::ExecutionProviderType;
    if provider.is_some_and(|provider| *provider != ExecutionProviderType::Cpu)
        || environment_providers
            .iter()
            .any(|provider| *provider != "CPUExecutionProvider")
    {
        return Err(invalid(
            "Local captioning requires CPU execution and rejects conflicting provider settings",
        ));
    }
    Ok(())
}

fn build_session(bytes: &[u8], label: &str) -> crate::Result<Session> {
    let session_options = crate::ort_options::ort_session_options();
    let acceleration = crate::core::config::acceleration::AccelerationConfig {
        provider: crate::core::config::acceleration::ExecutionProviderType::Cpu,
        ..Default::default()
    };
    crate::onnx::build_session_with(&session_options, Some(&acceleration), invalid, |builder| {
        builder.clone().commit_from_memory(bytes)
    })
    .map_err(|error| crate::XbergError::Validation {
        message: format!("Local captioning {label} session: {error}"),
        source: Some(Box::new(error)),
    })
}

fn companion<'a>(artifacts: &'a VerifiedArtifacts, name: &str) -> crate::Result<&'a [u8]> {
    artifacts
        .companions
        .get(name)
        .map(Vec::as_slice)
        .ok_or_else(|| invalid(format!("Local captioning artifact set resolved no {name}")))
}

fn json(bytes: &[u8], name: &str) -> crate::Result<serde_json::Value> {
    serde_json::from_slice(bytes).map_err(|error| invalid(format!("Local captioning {name} JSON: {error}")))
}

fn number(value: &serde_json::Value, key: &str) -> Option<u64> {
    value.get(key).and_then(serde_json::Value::as_u64)
}

fn eos_token_ids(generation: &serde_json::Value, tokenizer: &tokenizers::Tokenizer) -> std::collections::BTreeSet<i64> {
    let mut ids = std::collections::BTreeSet::new();
    let value = generation.get("eos_token_id");
    match value {
        Some(serde_json::Value::Number(number)) => {
            if let Some(id) = number.as_i64() {
                ids.insert(id);
            }
        }
        Some(serde_json::Value::Array(values)) => {
            ids.extend(values.iter().filter_map(serde_json::Value::as_i64));
        }
        _ => {}
    }
    for token in ["<|im_end|>", "<end_of_utterance>", "</s>", "<endoftext>"] {
        if let Some(id) = tokenizer.token_to_id(token) {
            ids.insert(i64::from(id));
        }
    }
    ids
}

fn kv_shape(config: &serde_json::Value, format: LocalCaptioningFormat) -> crate::Result<(KvShape, usize)> {
    let text = config
        .get("text_config")
        .filter(|value| value.is_object())
        .unwrap_or(config);
    let hidden = number(text, "hidden_size")
        .or_else(|| number(text, "d_model"))
        .ok_or_else(|| invalid("Local captioning config lacks a text hidden size"))?;
    let heads = number(text, "num_attention_heads")
        .or_else(|| number(text, "decoder_attention_heads"))
        .ok_or_else(|| invalid("Local captioning config lacks attention heads"))?;
    let kv_heads = number(text, "num_key_value_heads").unwrap_or(heads);
    let head_dim = number(text, "head_dim").unwrap_or_else(|| hidden / heads.max(1));
    let layers = number(text, "num_hidden_layers")
        .or_else(|| number(text, "decoder_layers"))
        .or_else(|| match format {
            LocalCaptioningFormat::Florence2 => number(text, "decoder_layers"),
            LocalCaptioningFormat::SmolVlm => number(text, "num_hidden_layers"),
        })
        .ok_or_else(|| invalid("Local captioning config lacks a decoder layer count"))?;
    if heads == 0 || kv_heads == 0 || head_dim == 0 || layers == 0 || hidden == 0 {
        return Err(invalid("Local captioning config has a zero cache dimension"));
    }
    Ok((
        KvShape {
            layers: usize::try_from(layers).map_err(|_| invalid("layer count too large"))?,
            heads: usize::try_from(kv_heads).map_err(|_| invalid("head count too large"))?,
            head_dim: usize::try_from(head_dim).map_err(|_| invalid("head size too large"))?,
        },
        usize::try_from(hidden).map_err(|_| invalid("hidden size too large"))?,
    ))
}

fn graph_files(model: &super::LocalCaptioningModel) -> Vec<(LocalCaptioningGraphRole, &str)> {
    let mut graphs = vec![
        (
            LocalCaptioningGraphRole::VisionEncoder,
            model.vision_encoder_file.as_str(),
        ),
        (LocalCaptioningGraphRole::EmbedTokens, model.embed_tokens_file.as_str()),
        (LocalCaptioningGraphRole::Decoder, model.decoder_file.as_str()),
    ];
    if let Some(text) = model.text_encoder_file.as_deref() {
        graphs.push((LocalCaptioningGraphRole::TextEncoder, text));
    }
    graphs
}

pub(super) fn model_info(model: &super::LocalCaptioningModel) -> crate::Result<LocalCaptioningModelInfo> {
    let graphs = graph_files(model)
        .into_iter()
        .map(|(role, path)| {
            let file: &LocalCaptioningFile = model
                .files
                .iter()
                .find(|file| file.path == path)
                .ok_or_else(|| invalid(format!("Local captioning graph {path} is not declared")))?;
            Ok(LocalCaptioningGraph {
                role,
                path: path.to_owned(),
                sha256: file.sha256.clone(),
            })
        })
        .collect::<crate::Result<Vec<_>>>()?;
    Ok(LocalCaptioningModelInfo {
        selected_backend: LocalCaptioningBackend::OnnxCpu,
        effective_backend: LocalCaptioningBackend::OnnxCpu,
        format: model.format,
        precision: precision_of_model(model),
        repository: model.repository.clone(),
        revision: model.revision.clone(),
        graphs,
        manifest_sha256: manifest_digest(model),
        experimental: true,
    })
}

fn precision_of_model(model: &super::LocalCaptioningModel) -> LocalCaptioningPrecision {
    if model.decoder_file.ends_with("_int8.onnx") {
        LocalCaptioningPrecision::Quantized
    } else {
        LocalCaptioningPrecision::Fp32
    }
}

fn model_identity(model: &super::LocalCaptioningModel) -> String {
    let mut identity = manifest_digest(model);
    identity.push(':');
    identity.push_str(&model.vision_encoder_file);
    identity.push(':');
    identity.push_str(&model.decoder_file);
    identity
}

fn prepare_model(config: &LocalCaptioningConfig, control: &WorkerControl) -> crate::Result<PreparedModel> {
    control.check()?;
    validate_cpu_runtime()?;
    let artifacts = read_artifacts(config, control)?;
    let tokenizer = tokenizers::Tokenizer::from_bytes(companion(&artifacts, "tokenizer.json")?)
        .map_err(|error| invalid(format!("Local captioning tokenizer: {error}")))?;
    let model_config = json(companion(&artifacts, "config.json")?, "config.json")?;
    let generation = json(
        companion(&artifacts, "generation_config.json")?,
        "generation_config.json",
    )?;
    let preprocessor = json(
        companion(&artifacts, "preprocessor_config.json")?,
        "preprocessor_config.json",
    )?;
    let processor = json(companion(&artifacts, "processor_config.json")?, "processor_config.json")?;
    let preprocess = PreprocessConfig::from_json(config.model.format, &preprocessor, &processor)?;
    let (kv, hidden) = kv_shape(&model_config, config.model.format)?;
    let image_token_id = match config.model.format {
        LocalCaptioningFormat::SmolVlm => {
            let id = number(&model_config, "image_token_id")
                .or_else(|| tokenizer.token_to_id("<image>").map(u64::from))
                .ok_or_else(|| invalid("SmolVLM config lacks an image token id"))?;
            i64::try_from(id).map_err(|_| invalid("image token id too large"))?
        }
        LocalCaptioningFormat::Florence2 => -1,
    };
    control.check()?;
    let vision = build_session(&artifacts.vision, "vision encoder")?;
    let embed = build_session(&artifacts.embed, "embedding")?;
    let decoder = build_session(&artifacts.decoder, "decoder")?;
    control.check()?;
    let past_suffixes: Vec<String> = decoder
        .inputs()
        .iter()
        .filter_map(|input| input.name().strip_prefix("past_key_values.").map(str::to_owned))
        .collect();
    let per_layer = past_suffixes.len() / kv.layers.max(1);
    if !past_suffixes.len().is_multiple_of(kv.layers.max(1))
        || !matches!(per_layer, 2 | 4)
        || past_suffixes.len() != kv.layers * per_layer
    {
        return Err(invalid(format!(
            "Local captioning decoder exposes {} cache inputs for {} layers; expected two or four per layer",
            past_suffixes.len(),
            kv.layers
        )));
    }
    if past_suffixes.iter().collect::<std::collections::BTreeSet<_>>().len() != past_suffixes.len() {
        return Err(invalid("Local captioning decoder repeats a cache input"));
    }
    let info = model_info(&config.model)?;
    let captioner = PreparedCaptioner {
        format: config.model.format,
        preprocess,
        tokenizer: tokenizer.clone(),
        vision,
        embed,
        decoder,
        kv,
        past_suffixes,
        image_token_id,
        eos_token_ids: eos_token_ids(&generation, &tokenizer),
        hidden,
    };
    Ok(PreparedModel {
        captioner,
        identity: model_identity(&config.model),
        session_options: crate::ort_options::ort_session_options(),
        info,
    })
}

impl LocalImageCaptioner {
    /// Validate an experimental captioner configuration without resolving artifacts or loading a model.
    pub fn new(config: LocalCaptioningConfig) -> crate::Result<Self> {
        validate_config(&config)?;
        Ok(Self {
            state: Arc::new(CaptionerState {
                config,
                pool: crate::model_cache::ModelCache::with_capacity(MAX_CONCURRENT_REQUESTS),
                prepared_options: Mutex::new(None),
                semaphore: Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_REQUESTS)),
            }),
        })
    }

    /// Resolves and verifies every pinned artifact through the shared download/cache path.
    ///
    /// A cache hit performs no network request. A miss downloads only when Hugging Face offline
    /// mode is off. This is the only method that can reach the network.
    pub async fn stage(&self, options: &LocalCaptioningOptions) -> crate::Result<()> {
        let started = Instant::now();
        check_control(options, started)?;
        let state = Arc::clone(&self.state);
        let worker_options = options.clone();
        run_controlled(Arc::clone(&state.semaphore), options.clone(), started, move |request| {
            let control = WorkerControl {
                state: request,
                options: worker_options,
                started,
            };
            control.check()?;
            stage(&state.config, &control)
        })
        .await
    }

    /// Prepare the experimental CPU sessions from a pre-staged, checksum-pinned immutable cache.
    ///
    /// Neither preparation nor captioning fetches missing assets from the network. Cancellation
    /// and deadlines are checked before and between load stages; an already running filesystem
    /// read or session build may finish before this method returns cancellation or timeout.
    pub async fn prepare(&self, options: &LocalCaptioningOptions) -> crate::Result<LocalCaptioningModelInfo> {
        let started = Instant::now();
        check_control(options, started)?;
        let state = Arc::clone(&self.state);
        let worker_options = options.clone();
        run_controlled(Arc::clone(&state.semaphore), options.clone(), started, move |request| {
            let control = WorkerControl {
                state: request,
                options: worker_options,
                started,
            };
            control.check()?;
            validate_cpu_runtime()?;
            let session_options = crate::ort_options::ort_session_options();
            let identity = model_identity(&state.config.model);
            let model = state.pool.take_or_create_matching(
                |model| model.identity == identity && model.session_options == session_options,
                || prepare_model(&state.config, &control),
            )?;
            control.check()?;
            let info = model.info.clone();
            *state
                .prepared_options
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(model.session_options.clone());
            Ok(info)
        })
        .await
    }

    /// Caption an image with an explicitly prepared local model, without provider fallback.
    ///
    /// Cancellation asks ONNX Runtime to terminate active inference and waits for the blocking
    /// worker. A deadline does not guarantee a maximum wall-clock return time.
    pub async fn caption(
        &self,
        image_bytes: &[u8],
        options: &LocalCaptioningOptions,
    ) -> crate::Result<LocalCaptioningResult> {
        let started = Instant::now();
        check_control(options, started)?;
        if image_bytes.is_empty() || image_bytes.len() > self.state.config.limits.max_image_bytes as usize {
            return Err(invalid(
                "Local captioning image must be nonempty and within its byte limit",
            ));
        }
        if self
            .state
            .prepared_options
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_none()
        {
            return Err(invalid("Local captioning requires explicit prepare before inference"));
        }
        let state = Arc::clone(&self.state);
        let worker_options = options.clone();
        run_controlled_with_factory(Arc::clone(&state.semaphore), options.clone(), started, || {
            let image = image_bytes.to_vec();
            let options = worker_options;
            move |request| {
                let control = WorkerControl {
                    state: request,
                    options,
                    started,
                };
                caption_admitted(&state, &image, &control)
            }
        })
        .await
    }
}

fn caption_admitted(
    state: &CaptionerState,
    image_bytes: &[u8],
    control: &WorkerControl,
) -> crate::Result<LocalCaptioningResult> {
    control.check()?;
    validate_cpu_runtime()?;
    let session_options = crate::ort_options::ort_session_options();
    let prepared = state
        .prepared_options
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if prepared.as_ref() != Some(&session_options) {
        return Err(invalid(
            "ORT options changed; explicitly prepare the local captioner again",
        ));
    }
    let identity = model_identity(&state.config.model);
    let mut model = state.pool.take_or_create_matching(
        |model| model.identity == identity && model.session_options == session_options,
        || {
            Err(invalid(
                "Local captioning session is not cached; explicitly prepare again",
            ))
        },
    )?;
    control.check()?;
    let image = decode_png_or_common(image_bytes, state.config.limits.max_pixels)?;
    let batch = preprocess(
        &state.config,
        model.captioner.format,
        &model.captioner.preprocess,
        &image,
    )?;
    let caption = generate(
        &mut model.captioner,
        &batch,
        state.config.prompt.as_deref(),
        state.config.limits.max_new_tokens,
        control,
    )?;
    Ok(LocalCaptioningResult {
        caption,
        model: model.info.clone(),
    })
}

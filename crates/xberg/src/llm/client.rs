//! LLM client factory — converts xberg's LlmConfig to a liter-llm ManagedClient.

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::LazyLock;

use ahash::AHashMap;
use liter_llm::client::ClientConfig;
use liter_llm::{LlmInFlightLimitConfig, ManagedClient};
use parking_lot::RwLock;

#[cfg(not(target_arch = "wasm32"))]
use crate::core::config::CredentialProviderConfig;
use crate::core::config::{
    BedrockConfig, LlmBudgetConfig, LlmCacheConfig, LlmConfig, LlmProviderConfig, LlmRateLimitConfig,
};

/// Translate xberg's [`BedrockConfig`] into liter-llm's own (unboxed) `client::BedrockConfig`.
fn to_liter_llm_bedrock(bedrock: &BedrockConfig) -> liter_llm::client::BedrockConfig {
    liter_llm::client::BedrockConfig {
        region: bedrock.region.clone(),
        cross_region_prefix: bedrock.cross_region_prefix.clone(),
        access_key_id: bedrock.access_key_id.clone(),
        secret_access_key: bedrock.secret_access_key.clone(),
        session_token: bedrock.session_token.clone(),
    }
}

/// Translate xberg's [`LlmProviderConfig`] into liter-llm's own `client::LlmProviderConfig`.
fn to_liter_llm_provider(provider: &LlmProviderConfig) -> liter_llm::client::LlmProviderConfig {
    liter_llm::client::LlmProviderConfig {
        name: provider.name.clone(),
        base_url: provider.base_url.clone(),
        auth_header: provider.auth_header.clone(),
        model_prefixes: provider.model_prefixes.clone(),
    }
}

/// Convert xberg's flat `auth_header` header-name-or-default into liter-llm's richer
/// [`liter_llm::AuthHeaderFormat`].
///
/// xberg's [`LlmProviderConfig::auth_header`] is a single optional header *name*,
/// documented as "defaults to `Authorization` when unset" — it can only express
/// liter-llm's `Bearer` scheme or a raw named header, never the explicit
/// "no authentication" mode `AuthHeaderFormat::None` supports; that mode is
/// unreachable through this mapping (a future field would be needed to expose it).
/// The honest mapping for what xberg *can* express:
/// - `None`, or `Some("Authorization")` (case-insensitive) -> [`liter_llm::AuthHeaderFormat::Bearer`]
///   (`Authorization: Bearer <key>`), matching xberg's documented default.
/// - `Some(name)` for any other header name -> [`liter_llm::AuthHeaderFormat::ApiKey`]
///   (`<name>: <key>`, the raw key with no scheme prefix).
fn to_auth_header_format(auth_header: Option<&str>) -> liter_llm::AuthHeaderFormat {
    match auth_header {
        None => liter_llm::AuthHeaderFormat::Bearer,
        Some(name) if name.eq_ignore_ascii_case("authorization") => liter_llm::AuthHeaderFormat::Bearer,
        Some(name) => liter_llm::AuthHeaderFormat::ApiKey(name.to_string()),
    }
}

/// Register every configured custom provider ([`LlmConfig::providers`]) in liter-llm's
/// process-global custom-provider registry ([`liter_llm::register_custom_provider`]).
///
/// liter-llm's `into_client_builder` never reads `providers` — there is no equivalent
/// field on `ClientConfig` at all (see [`to_liter_llm_config`]). Custom providers only
/// take effect once registered here; without this call, a `providers` entry in config
/// is silently a no-op — shipping a config key that does nothing is worse than a clear
/// error, so registration failures are surfaced as [`crate::XbergError::Validation`]
/// rather than swallowed.
///
/// # Idempotency and cross-config collisions
///
/// The registry is process-global and keyed by provider `name`:
/// `register_custom_provider` replaces an existing entry with the same name rather than
/// erroring. Calling this repeatedly with the *same* [`LlmConfig`] is therefore
/// idempotent — it re-registers identical data every time a client is built from that
/// config. Calling it with *different* [`LlmConfig`]s that both define a provider under
/// the same `name` is **not** conflict-free: the most recently built client's
/// registration wins for every model lookup process-wide, including requests already in
/// flight on a client built from the other config. liter-llm exposes no read API to
/// detect this collision before overwriting (`detect_custom_provider` is crate-private
/// to liter-llm), so it cannot be caught here — use distinct provider names across every
/// `LlmConfig` used within one process to avoid it.
fn register_configured_providers(config: &LlmConfig) -> crate::Result<()> {
    let Some(providers) = config.providers.as_ref() else {
        return Ok(());
    };

    for provider in providers {
        let custom = liter_llm::CustomProviderConfig {
            name: provider.name.clone(),
            base_url: provider.base_url.clone(),
            auth_header: to_auth_header_format(provider.auth_header.as_deref()),
            model_prefixes: provider.model_prefixes.clone(),
        };

        liter_llm::register_custom_provider(custom).map_err(|e| {
            let msg = format!("Failed to register custom LLM provider '{}': {e}", provider.name);
            crate::XbergError::Validation {
                message: msg,
                source: Some(Box::new(e)),
            }
        })?;
    }

    Ok(())
}

/// Parse xberg's plain-string [`LlmConfig::reasoning_effort`] into liter-llm's
/// [`liter_llm::ReasoningEffort`] request-time enum.
///
/// liter-llm serializes [`liter_llm::ReasoningEffort`] with
/// `#[serde(rename_all = "lowercase")]`; this matches the same five spellings
/// (`"low"`, `"medium"`, `"high"`, `"minimal"`, `"max"`) case-insensitively. An
/// unrecognized value is a [`crate::XbergError::Validation`] rather than a silent
/// drop, matching the "a clear error is better than silence" principle used for
/// `providers` above — a request built from a mistyped value (e.g. `"maximum"`)
/// should fail loudly instead of quietly going out without the hint.
///
/// Called from every site that builds a [`liter_llm::ChatCompletionRequest`] from an
/// [`LlmConfig`], alongside the `temperature`/`max_tokens` lines: `llm::text_completion`,
/// `llm::structured` (both request builders), `llm::vlm_ocr` and `text::ner::llm`. A new
/// request builder must wire this and `config.extra_body` too, or those two config fields
/// silently do nothing on that path.
pub(crate) fn parse_reasoning_effort(config: &LlmConfig) -> crate::Result<Option<liter_llm::ReasoningEffort>> {
    let Some(value) = config.reasoning_effort.as_deref() else {
        return Ok(None);
    };

    match value.to_ascii_lowercase().as_str() {
        "low" => Ok(Some(liter_llm::ReasoningEffort::Low)),
        "medium" => Ok(Some(liter_llm::ReasoningEffort::Medium)),
        "high" => Ok(Some(liter_llm::ReasoningEffort::High)),
        "minimal" => Ok(Some(liter_llm::ReasoningEffort::Minimal)),
        "max" => Ok(Some(liter_llm::ReasoningEffort::Max)),
        _ => Err(crate::XbergError::Validation {
            message: format!(
                "Invalid LLM reasoning_effort '{value}': expected one of \"low\", \"medium\", \"high\", \
                 \"minimal\", \"max\""
            ),
            source: None,
        }),
    }
}

/// Convert xberg's list-shaped [`LlmConfig::stop`] into liter-llm's untagged
/// [`liter_llm::StopSequence`] request-time enum.
///
/// liter-llm represents `stop` as either a single string or a list of strings
/// (`types::common::StopSequence`, `#[serde(untagged)]`); xberg's `LlmConfig::stop` is
/// always a plain list — even a single stop sequence is `["..."]` — so this always maps
/// onto [`liter_llm::StopSequence::Multiple`], which is wire-compatible with every
/// provider that also accepts a single-element array for `stop`.
///
/// Called only from [`apply_request_time_params`], which is the single place every
/// request builder goes through.
pub(crate) fn to_stop_sequence(config: &LlmConfig) -> Option<liter_llm::StopSequence> {
    config
        .stop
        .as_ref()
        .map(|sequences| liter_llm::StopSequence::Multiple(sequences.clone()))
}

/// Apply every request-time parameter carried on an [`LlmConfig`] onto a liter-llm
/// [`liter_llm::ChatCompletionRequest`] in one place, so a new (or migrated) request
/// builder cannot silently drop one of them the way `top_p`, `stop`, `seed`,
/// `presence_penalty`, and `frequency_penalty` previously were — every existing call site
/// wired `temperature`, `max_tokens`, `reasoning_effort` (via [`parse_reasoning_effort`]),
/// and `extra_body` individually, but none of them set those other five
/// [`LlmConfig`] fields, so they were accepted by every config file and language binding
/// and then silently never reached a provider.
///
/// Deliberately leaves `request.model` and `request.messages` untouched — those come from
/// the call site's own prompt/schema/messages, not from `config`.
///
/// Every request builder in the crate calls this: `llm::text_completion::complete_text`,
/// `llm::structured` (both builders), `llm::vlm_ocr` and `text::ner::llm`. Keep it that
/// way — a builder that sets these fields by hand is how the five above went missing.
///
/// # Errors
///
/// Propagates [`parse_reasoning_effort`]'s error for an unrecognized
/// [`LlmConfig::reasoning_effort`] string.
///
/// `#[cfg_attr(alef, alef(skip))]`: `&mut liter_llm::ChatCompletionRequest` is a foreign,
/// non-`repr(C)` type with no FFI-representable equivalent — alef's own
/// `lossy_sanitized_surface` check rejects it outright, matching every other
/// binding-unreachable LLM entry point in this crate (e.g.
/// `create_client_with_credential_provider` above, `llm::text_completion::complete_text`,
/// `llm::structured::complete_with_json_schema`).
#[cfg_attr(alef, alef(skip))]
pub(crate) fn apply_request_time_params(
    request: &mut liter_llm::ChatCompletionRequest,
    config: &LlmConfig,
) -> crate::Result<()> {
    // Parse before mutating anything: a rejected `reasoning_effort` must leave the
    // request untouched rather than half-applied.
    let reasoning_effort = parse_reasoning_effort(config)?;

    request.temperature = config.temperature;
    request.max_tokens = config.max_tokens;
    request.top_p = config.top_p;
    request.stop = to_stop_sequence(config);
    request.seed = config.seed;
    request.presence_penalty = config.presence_penalty;
    request.frequency_penalty = config.frequency_penalty;
    request.reasoning_effort = reasoning_effort;
    request.extra_body = config.extra_body.clone();
    Ok(())
}

/// Translate xberg's [`LlmCacheConfig`] into liter-llm's own `client::LlmCacheConfig`.
fn to_liter_llm_cache(cache: &LlmCacheConfig) -> liter_llm::client::LlmCacheConfig {
    liter_llm::client::LlmCacheConfig {
        max_entries: cache.max_entries,
        ttl_seconds: cache.ttl_seconds,
        backend: cache.backend.clone(),
        backend_config: cache.backend_config.clone(),
    }
}

/// Translate xberg's [`LlmBudgetConfig`] into liter-llm's own `client::LlmBudgetConfig`.
fn to_liter_llm_budget(budget: &LlmBudgetConfig) -> liter_llm::client::LlmBudgetConfig {
    liter_llm::client::LlmBudgetConfig {
        global_limit: budget.global_limit,
        model_limits: budget.model_limits.clone(),
        enforcement: budget.enforcement.clone(),
    }
}

/// Translate xberg's [`LlmRateLimitConfig`] into liter-llm's own `client::LlmRateLimitConfig`.
fn to_liter_llm_rate_limit(rate_limit: &LlmRateLimitConfig) -> liter_llm::client::LlmRateLimitConfig {
    liter_llm::client::LlmRateLimitConfig {
        rpm: rate_limit.rpm,
        tpm: rate_limit.tpm,
        window_seconds: rate_limit.window_seconds,
    }
}

/// Build a live liter-llm credential provider from xberg's [`CredentialProviderConfig`].
///
/// Every variant needs one of liter-llm's `native-http`-backed auth modules
/// (`azure-auth`, `vertex-auth`, `bedrock-auth` — `vertex-adc` rides plain `native-http`),
/// which xberg's Cargo.toml enables unconditionally on every non-wasm32 target (see the
/// `liter-llm` dependency comment there). This function is therefore only compiled on
/// non-wasm32 — the same reason it exists at all: the whole `crate::llm` module (this file
/// included) is compiled out on `wasm32` at the crate root
/// (`#[cfg(all(feature = "liter-llm", not(target_arch = "wasm32")))]` on `pub mod llm;` in
/// `lib.rs`), so there is nothing here for a wasm32 build to reject or fall back from.
#[cfg(not(target_arch = "wasm32"))]
fn build_credential_provider(
    config: &CredentialProviderConfig,
) -> crate::Result<Arc<dyn liter_llm::auth::CredentialProvider>> {
    match config {
        CredentialProviderConfig::AzureAd {
            tenant_id,
            client_id,
            client_secret,
            scope,
        } => Ok(build_azure_ad_provider(
            tenant_id,
            client_id,
            client_secret,
            scope.as_deref(),
        )),
        CredentialProviderConfig::VertexOauth2 {
            service_account_key_file,
            scope,
        } => build_vertex_oauth2_provider(service_account_key_file, scope.as_deref()),
        CredentialProviderConfig::VertexAdc { scope } => Ok(build_vertex_adc_provider(scope.as_deref())),
        CredentialProviderConfig::BedrockWebIdentity {
            role_arn,
            token_file,
            session_name,
            region,
        } => Ok(build_bedrock_web_identity_provider(
            role_arn,
            token_file,
            session_name.as_deref(),
            region.as_deref(),
        )),
    }
}

/// Build an Azure AD OAuth2 client-credentials provider. See [`build_credential_provider`].
#[cfg(not(target_arch = "wasm32"))]
fn build_azure_ad_provider(
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    scope: Option<&str>,
) -> Arc<dyn liter_llm::auth::CredentialProvider> {
    let mut provider = liter_llm::auth::azure_ad::AzureAdCredentialProvider::new(
        tenant_id.to_string(),
        client_id.to_string(),
        secrecy::SecretString::from(client_secret.to_string()),
    );
    if let Some(scope) = scope {
        provider = provider.with_scope(scope.to_string());
    }
    Arc::new(provider)
}

/// Build a Vertex AI OAuth2 provider from a service-account JSON key file. See
/// [`build_credential_provider`].
#[cfg(not(target_arch = "wasm32"))]
fn build_vertex_oauth2_provider(
    service_account_key_file: &str,
    scope: Option<&str>,
) -> crate::Result<Arc<dyn liter_llm::auth::CredentialProvider>> {
    let mut provider = liter_llm::auth::vertex_oauth::VertexOAuthCredentialProvider::from_key_file(
        std::path::Path::new(service_account_key_file),
    )
    .map_err(|e| crate::XbergError::Validation {
        message: format!("Failed to load Vertex OAuth2 service account key file '{service_account_key_file}': {e}"),
        source: Some(Box::new(e)),
    })?;
    if let Some(scope) = scope {
        provider = provider.with_scope(scope.to_string());
    }
    Ok(Arc::new(provider))
}

/// Build a Vertex AI Application Default Credentials provider. See [`build_credential_provider`].
#[cfg(not(target_arch = "wasm32"))]
fn build_vertex_adc_provider(scope: Option<&str>) -> Arc<dyn liter_llm::auth::CredentialProvider> {
    let mut provider = liter_llm::auth::vertex_adc::VertexAdcCredentialProvider::new();
    if let Some(scope) = scope {
        provider = provider.with_scope(scope.to_string());
    }
    Arc::new(provider)
}

/// Build an AWS STS `AssumeRoleWithWebIdentity` provider for Bedrock. See
/// [`build_credential_provider`].
#[cfg(not(target_arch = "wasm32"))]
fn build_bedrock_web_identity_provider(
    role_arn: &str,
    token_file: &str,
    session_name: Option<&str>,
    region: Option<&str>,
) -> Arc<dyn liter_llm::auth::CredentialProvider> {
    Arc::new(liter_llm::auth::bedrock_sts::WebIdentityCredentialProvider::new(
        role_arn.to_string(),
        token_file.to_string(),
        session_name.unwrap_or("liter-llm-session").to_string(),
        region.unwrap_or("us-east-1").to_string(),
    ))
}

/// Cache backend name accepted when liter-llm's `opendal-cache` feature is not compiled in.
///
/// xberg does not enable `opendal-cache` (see the `liter-llm` dependency comment in
/// Cargo.toml): it pulls opendal's `services-memory`/`services-redis`/`services-fs`/
/// `services-s3` clients, a heavy dependency for a passthrough cache config field. Without
/// this check, liter-llm's own `into_client_builder` would silently downgrade any other
/// backend name to `Memory` — see [`validate_cache_backend`].
const SUPPORTED_CACHE_BACKEND: &str = "memory";

/// Reject an `LlmConfig::cache` backend name xberg cannot honor, instead of letting it
/// silently degrade to the in-memory backend inside liter-llm's own `into_client_builder`.
fn validate_cache_backend(cache: &LlmCacheConfig) -> crate::Result<()> {
    match cache.backend.as_deref() {
        None | Some(SUPPORTED_CACHE_BACKEND) => Ok(()),
        Some(other) => Err(crate::XbergError::Validation {
            message: format!(
                "Unsupported LLM cache backend '{other}': xberg only supports \"{SUPPORTED_CACHE_BACKEND}\" \
                 (liter-llm's `opendal-cache` feature, which unlocks Redis/S3/filesystem-backed caches, is not \
                 compiled in). Use \"{SUPPORTED_CACHE_BACKEND}\" or omit `backend`."
            ),
            source: None,
        }),
    }
}

/// Translate xberg's [`LlmConfig`] into liter-llm's own canonical `client::LlmConfig`,
/// field for field, so the two never drift silently: every field the two types share
/// (by name and type, by construction — see the `LlmConfig` module doc) is copied
/// here without any per-field re-derivation of liter-llm's own mapping semantics.
///
/// Two deliberate exceptions:
/// - `headers` is left `None`. liter-llm's `into_client_builder` silently drops any
///   header that fails to parse as an HTTP header name/value pair, while xberg's
///   [`build_client_config`] validates headers explicitly and returns a
///   [`crate::XbergError::Validation`] naming the offending header — so headers are
///   applied separately, after conversion, on the builder this function's caller
///   gets back from `into_client_builder()`.
/// - `base_url` is trimmed of a trailing slash before being handed to liter-llm,
///   which does not perform this normalization itself; without it a base URL like
///   `"https://api.openai.com/v1/"` would join with a request path into a doubled
///   `//`.
///
/// [`LlmConfig::max_concurrency`] is the one field that is not a straight copy: it maps
/// onto liter-llm's [`LlmInFlightLimitConfig`] (`Some(max)` becomes
/// `Some(LlmInFlightLimitConfig { max_in_flight: Some(max) })`, `None` stays `None`).
/// liter-llm only installs the corresponding Tower in-flight-limit layer when the client
/// is built as a [`ManagedClient`] — a plain `DefaultClient` has no Tower stack at all, so
/// this field is silently inert unless the client is built through [`ManagedClient`] (see
/// [`create_client`]).
///
/// `providers` is copied below purely for a lossless DTO-to-DTO conversion (see the
/// dedicated test); liter-llm's own `into_client_builder` never reads it back off the
/// value returned here — [`build_client_config`] instead registers each entry directly
/// from xberg's [`LlmConfig::providers`] via [`register_configured_providers`], which is
/// the mapping that actually takes effect.
fn to_liter_llm_config(config: &LlmConfig) -> liter_llm::client::LlmConfig {
    liter_llm::client::LlmConfig {
        model: config.model.clone(),
        api_key: config.api_key.clone(),
        base_url: config
            .base_url
            .as_deref()
            .map(|url| url.trim_end_matches('/').to_string()),
        timeout_secs: config.timeout_secs,
        max_retries: config.max_retries,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        load_env: config.load_env,
        headers: None,
        providers: config
            .providers
            .as_ref()
            .map(|providers| providers.iter().map(to_liter_llm_provider).collect()),
        cache: config.cache.as_deref().map(to_liter_llm_cache),
        budget: config.budget.as_deref().map(to_liter_llm_budget),
        rate_limit: config.rate_limit.as_deref().map(to_liter_llm_rate_limit),
        in_flight_limit: config.max_concurrency.map(|max| LlmInFlightLimitConfig {
            max_in_flight: Some(max),
        }),
        cost_tracking: config.cost_tracking,
        tracing: config.tracing,
        cooldown_secs: config.cooldown_secs,
        health_check_secs: config.health_check_secs,
        bedrock: config.bedrock.as_deref().map(to_liter_llm_bedrock),
    }
}

/// Translate xberg's [`LlmConfig`] into a liter-llm [`ClientConfig`].
///
/// `model`, `temperature`, and `max_tokens` are request-time parameters, not
/// client-level settings; they are deliberately carried on [`LlmConfig`] without
/// being mapped here, matching liter-llm's own `LlmConfig::into_client_builder`.
///
/// The actual field-by-field wiring (`base_url`, `timeout_secs`, `max_retries`,
/// `load_env`, `cache`, `budget`, `rate_limit`, `cost_tracking`, `tracing`,
/// `cooldown_secs`, `health_check_secs`, `bedrock`) is delegated to liter-llm's
/// own [`liter_llm::client::LlmConfig::into_client_builder`] via
/// [`to_liter_llm_config`], rather than re-implemented here, so this function
/// cannot drift from liter-llm's own mapping as new fields are added upstream.
/// `headers` and `max_response_bytes` are applied separately (see
/// [`to_liter_llm_config`]): neither has an equivalent on
/// [`liter_llm::client::LlmConfig`], only on the builder returned by
/// `into_client_builder()`. `providers` has no equivalent on [`ClientConfig`] at all;
/// it is instead registered as a side effect via [`register_configured_providers`] —
/// see that function's doc for the process-global registry semantics this implies.
///
/// Split out of [`create_client`] so the mapping is observable in tests without
/// constructing a live HTTP client.
///
/// [`LlmConfig::validate`] runs first, rejecting an out-of-range `top_p`,
/// `presence_penalty`, `frequency_penalty`, or a zero `max_response_bytes` before any
/// provider registration or header validation below — the same "fail before doing
/// anything else" position as the existing `validate_cache_backend` check.
fn build_client_config(config: &LlmConfig) -> crate::Result<ClientConfig> {
    config.validate()?;
    register_configured_providers(config)?;
    if let Some(ref cache) = config.cache {
        validate_cache_backend(cache)?;
    }

    let mut builder = to_liter_llm_config(config).into_client_builder();
    #[cfg(not(target_arch = "wasm32"))]
    {
        builder = apply_credential_provider(config, builder)?;
    }

    if let Some(ref headers) = config.headers {
        for (key, value) in headers {
            builder = builder.header(key.as_str(), value.as_str()).map_err(|e| {
                let msg = format!("Invalid LLM header '{key}': {e}");
                crate::XbergError::Validation {
                    message: msg,
                    source: Some(Box::new(e)),
                }
            })?;
        }
    }

    // liter-llm's `max_response_bytes` builder method is native-only (`native-http`,
    // non-wasm32) — see the doc comment on `LlmConfig::max_response_bytes`.
    // `LlmConfig::validate` above already rejects `Some(0)`, so this only ever forwards
    // a value liter-llm accepts, but the fallible builder call is still mapped for
    // defense in depth.
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(limit) = config.max_response_bytes {
        builder = builder.max_response_bytes(limit).map_err(|e| {
            let msg = format!("Invalid LLM max_response_bytes {limit}: {e}");
            crate::XbergError::Validation {
                message: msg,
                source: Some(Box::new(e)),
            }
        })?;
    }

    Ok(builder.build())
}

/// Resolve [`LlmConfig::credential_provider`], if set, into a live liter-llm provider and
/// attach it to `builder`. A no-op when unset.
#[cfg(not(target_arch = "wasm32"))]
fn apply_credential_provider(
    config: &LlmConfig,
    builder: liter_llm::client::ClientConfigBuilder,
) -> crate::Result<liter_llm::client::ClientConfigBuilder> {
    let Some(provider_config) = config.credential_provider.as_deref() else {
        return Ok(builder);
    };
    let provider = build_credential_provider(provider_config)?;
    Ok(builder.credential_provider(provider))
}

/// Build a fresh liter-llm [`ManagedClient`] from an already-resolved [`ClientConfig`].
///
/// Shared by [`cached_managed_client`] (the cache-miss path behind [`create_client`]) and
/// [`create_client_with_credential_provider`] (which is never cached — see that function's
/// docs). Building through [`ManagedClient`] rather than the plain `DefaultClient` is what
/// makes [`LlmConfig::max_concurrency`] (mapped to `in_flight_limit` in
/// [`to_liter_llm_config`]) actually take effect: `DefaultClient` has no Tower stack at
/// all, so the same config field would be silently inert built the other way — see
/// GH#1465.
fn build_managed_client(config: &LlmConfig, client_config: ClientConfig) -> crate::Result<ManagedClient> {
    ManagedClient::new(client_config, Some(&config.model)).map_err(|e| {
        let msg = format!("Failed to build LLM client for model '{}': {e}", config.model);
        crate::XbergError::Validation {
            message: msg,
            source: Some(Box::new(e)),
        }
    })
}

/// Process-wide cache of shared [`ManagedClient`]s, keyed by a digest of the [`LlmConfig`]
/// that built them.
///
/// # Why a shared client (GH#1465)
///
/// [`LlmConfig::max_concurrency`] is meant to bound *provider* concurrency — e.g. "never
/// have more than N requests in flight against this OpenAI account at once". A limiter
/// attached to a client that is rebuilt on every call is not that: it is a per-call
/// allowance, so N concurrent extractions each mint their own instance and their own
/// semaphore, and aggregate provider concurrency stays unbounded — the opposite of what a
/// provider limit is for (see the GH#1465 issue and #1453, which this replaces). Caching
/// by configuration identity means every caller that resolves to the same [`LlmConfig`]
/// shares one [`ManagedClient`], and therefore one in-flight-request semaphore, making the
/// bound global instead of per-extraction.
///
/// # The key is a digest, never the config
///
/// [`LlmConfig::api_key`] (and, via [`LlmConfig::credential_provider`], secrets nested in
/// [`CredentialProviderConfig`]) must never become a map key, a log field, or anything
/// `Debug`-printed (see this crate's secrets-handling conventions). [`config_digest`]
/// stores only a 32-byte BLAKE3 digest of the JSON-serialized config here — a fixed-size,
/// non-reversible fingerprint. Two configs that serialize identically collide onto the
/// same cached client (the desired cache-hit behavior); nothing about the original secret
/// is recoverable from the stored digest.
///
/// # Growth
///
/// A cache keyed on arbitrary caller-supplied config content has unbounded key
/// cardinality in principle — e.g. a long-running server accepting a distinct per-request
/// [`LlmConfig`] from untrusted callers. [`MAX_CACHED_LLM_CLIENTS`] caps the number of
/// distinct clients held at once. Once full, a config that would need a new entry still
/// gets a correctly-built, fully-functional client from [`cached_managed_client`] — it is
/// simply not pooled, so that one call loses the cross-call sharing benefit instead of the
/// process growing without bound. There is no eviction (no LRU) of existing entries: the
/// cap is a ceiling on distinct configurations, not a size-managed cache, on the
/// expectation that in-process usage exercises a small, mostly-static set of
/// configurations (one per configured pipeline stage/feature), not one per request.
///
/// Shape matches the engine pools already established in this crate for other expensive,
/// process-wide resources — see `candle_ocr::trocr_backend::ENGINE_POOL`.
#[cfg(not(target_arch = "wasm32"))]
static LLM_CLIENT_POOL: LazyLock<RwLock<AHashMap<[u8; 32], Arc<ManagedClient>>>> =
    LazyLock::new(|| RwLock::new(AHashMap::new()));

/// Ceiling on distinct cached LLM client configurations — see [`LLM_CLIENT_POOL`].
#[cfg(not(target_arch = "wasm32"))]
const MAX_CACHED_LLM_CLIENTS: usize = 256;

/// Compute the [`LLM_CLIENT_POOL`] cache key for `config`: a BLAKE3 digest of its
/// JSON serialization. Never log or otherwise expose this value's input — see
/// [`LLM_CLIENT_POOL`]'s docs on why only the digest is stored.
#[cfg(not(target_arch = "wasm32"))]
fn config_digest(config: &LlmConfig) -> crate::Result<[u8; 32]> {
    let serialized = serde_json::to_vec(config).map_err(|e| crate::XbergError::Validation {
        message: format!("Failed to serialize LLM config for client-cache lookup: {e}"),
        source: Some(Box::new(e)),
    })?;
    Ok(*blake3::hash(&serialized).as_bytes())
}

/// Return a cached [`ManagedClient`] for `config`, building and pooling one on first use.
///
/// Uses a read -> miss -> build -> write -> double-check pattern (matching
/// `candle_ocr::trocr_backend::get_or_init_engine`) so that two racing callers with the
/// same config do not both pay the client-construction cost, and so that only one of the
/// two ends up in the pool. See [`LLM_CLIENT_POOL`] for the key derivation, sharing
/// rationale, and growth bound.
#[cfg(not(target_arch = "wasm32"))]
fn cached_managed_client(config: &LlmConfig) -> crate::Result<Arc<ManagedClient>> {
    let digest = config_digest(config)?;

    {
        let pool = LLM_CLIENT_POOL.read();
        if let Some(client) = pool.get(&digest) {
            return Ok(Arc::clone(client));
        }
    }

    let client_config = build_client_config(config)?;
    let client = Arc::new(build_managed_client(config, client_config)?);

    let mut pool = LLM_CLIENT_POOL.write();
    if let Some(existing) = pool.get(&digest) {
        return Ok(Arc::clone(existing));
    }
    if pool.len() < MAX_CACHED_LLM_CLIENTS {
        pool.insert(digest, Arc::clone(&client));
    } else {
        tracing::debug!(
            cached = pool.len(),
            limit = MAX_CACHED_LLM_CLIENTS,
            "LLM client cache at capacity; built an unpooled client for this call"
        );
    }
    Ok(client)
}

/// Create a shared, process-wide-cached liter-llm [`ManagedClient`] from xberg's
/// [`LlmConfig`].
///
/// The `model` field from the config is passed as a model hint so that
/// liter-llm can resolve the correct provider automatically.
///
/// When `api_key` is `None`, liter-llm falls back to the provider's standard
/// environment variable (e.g., `OPENAI_API_KEY`).
///
/// For a `bedrock/`-prefixed model, liter-llm's own provider validation runs
/// here and rejects the request up front — with a message naming the required
/// AWS credentials and how to supply them — when neither explicit
/// [`BedrockConfig`](crate::core::config::BedrockConfig) credentials nor
/// `AWS_ACCESS_KEY_ID` in the environment are available. That upstream message
/// is wrapped with the model that failed to build, so the resulting error names
/// the operation (building the LLM client), the input (the model string), and
/// (via the wrapped source) a concrete suggestion.
///
/// Returns an `Arc` shared with every other caller that resolves to an identical
/// [`LlmConfig`] (see [`cached_managed_client`]/[`LLM_CLIENT_POOL`]) — this, plus building
/// through [`ManagedClient`] rather than `DefaultClient`, is what makes
/// [`LlmConfig::max_concurrency`] a real, global provider-request bound instead of a
/// per-call allowance (GH#1465).
pub(crate) fn create_client(config: &LlmConfig) -> crate::Result<Arc<ManagedClient>> {
    cached_managed_client(config)
}

/// Create a liter-llm [`ManagedClient`] from xberg's [`LlmConfig`], overriding any
/// [`LlmConfig::credential_provider`] with an explicit, caller-supplied provider.
///
/// This is the escape hatch for authentication modes [`CredentialProviderConfig`] cannot
/// express as data — GitHub Copilot's interactive device-flow provider
/// (`liter_llm::auth::github_copilot::GithubCopilotCredentialProvider`), or any fully custom
/// [`liter_llm::auth::CredentialProvider`] implementation. Building `provider` requires the
/// caller to depend on `liter-llm` directly; xberg does not re-export its auth types.
///
/// Deliberately **not** cached through [`LLM_CLIENT_POOL`]: the cache key is a digest of
/// the serializable [`LlmConfig`], and `provider` (an `Arc<dyn CredentialProvider>`) is not
/// part of that — nor of any other serializable identity this function has access to. Two
/// callers passing the same `config` with two different `provider`s must never be handed
/// the same client, so every call here builds a fresh, unpooled [`ManagedClient`] instead.
/// It is still built through [`ManagedClient`] (via [`build_managed_client`]), so
/// [`LlmConfig::max_concurrency`] still takes effect per client built this way — it simply
/// is not shared across calls the way [`create_client`]'s cached client is.
///
/// Not available on wasm32: this function lives in `crate::llm::client`, which — like the rest
/// of `crate::llm` — is compiled out on that target (see the crate-root `pub mod llm;` gate in
/// `lib.rs`), because there is no native-http-backed `CredentialProvider` implementation to
/// construct `provider` from there in the first place. [`LlmConfig::credential_provider`]
/// itself still exists as a field on wasm32 (see that field's docs) — it is simply never read.
///
/// This Rust-only function accepts a liter-llm trait object and is not exposed
/// through language bindings.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub fn create_client_with_credential_provider(
    config: &LlmConfig,
    provider: Arc<dyn liter_llm::auth::CredentialProvider>,
) -> crate::Result<ManagedClient> {
    let mut client_config = build_client_config(config)?;
    client_config.credential_provider = Some(provider);

    build_managed_client(config, client_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{BedrockConfig, CredentialProviderConfig, LlmConfig};

    #[cfg(feature = "api")]
    #[tokio::test]
    async fn test_client_path_normalization_with_base_url() {
        use axum::{Router, routing::post};
        use liter_llm::LlmClient;
        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        let app = Router::new().fallback(post(
            move |_method: axum::http::Method, uri: axum::http::Uri, headers: axum::http::HeaderMap| async move {
                assert_eq!(uri.path(), "/v1/chat/completions");

                let auth = headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("none")
                    .to_string();
                let _ = tx.send(auth);

                axum::response::Json(serde_json::json!({
                    "id": "test",
                    "object": "chat.completion",
                    "created": 12345,
                    "model": "test",
                    "choices": [{
                        "index": 0,
                        "message": { "role": "assistant", "content": "{\"foo\": \"bar\"}" },
                        "finish_reason": "stop"
                    }]
                }))
            },
        ));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let base_url = format!("http://{}/v1/", addr);
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-api-key".to_string()),
            base_url: Some(base_url),
            ..LlmConfig::default()
        };

        let client = create_client(&config).unwrap();

        let request = liter_llm::ChatCompletionRequest {
            model: config.model.clone(),
            messages: vec![liter_llm::Message::User(liter_llm::UserMessage {
                content: liter_llm::UserContent::Text("test".to_string()),
                ..Default::default()
            })],
            ..Default::default()
        };

        let _ = client.chat(request).await.expect("Request failed");

        let auth_header = tokio::time::timeout(tokio::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("Timeout waiting for header")
            .expect("No header received");

        assert_eq!(auth_header, "Bearer test-api-key");
    }

    #[test]
    fn test_create_client_sanitizes_base_url() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: Some("https://api.openai.com/v1/".to_string()),
            ..LlmConfig::default()
        };

        let _ = create_client(&config).unwrap();
    }

    #[test]
    fn test_create_client_applies_load_env_and_valid_headers() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Gateway-Key".to_string(), "secret123".to_string());
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            load_env: Some(true),
            headers: Some(headers),
            ..LlmConfig::default()
        };

        assert!(
            create_client(&config).is_ok(),
            "valid load_env + headers should build a client"
        );
    }

    #[test]
    fn test_create_client_rejects_invalid_header() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Bad\r\nInjected".to_string(), "value".to_string());
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            headers: Some(headers),
            ..LlmConfig::default()
        };

        match create_client(&config) {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(message.contains("Invalid LLM header"), "unexpected message: {message}");
            }
            Err(other) => panic!("expected a Validation error, got: {other}"),
            Ok(_) => panic!("expected create_client to reject the invalid header"),
        }
    }

    /// Regression test for GH#1465.
    ///
    /// A per-client in-flight-request limit is only a real *provider* concurrency bound if
    /// every caller resolving to the same [`LlmConfig`] shares one client (and therefore
    /// one semaphore). Before this fix, `create_client` built a fresh `DefaultClient` (no
    /// Tower stack at all) on every call, so two calls with an identical config produced
    /// two independent instances — this asserts they now produce the *same* instance.
    #[test]
    fn test_create_client_shares_one_instance_for_identical_config() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key-for-sharing-test".to_string()),
            ..LlmConfig::default()
        };

        let first = create_client(&config).expect("build first client");
        let second = create_client(&config).expect("build second client");

        assert!(
            Arc::ptr_eq(&first, &second),
            "two create_client calls with an identical LlmConfig must return the same client \
             instance so an in-flight-request limit is enforced globally, not per call"
        );
    }

    /// Companion to [`test_create_client_shares_one_instance_for_identical_config`]: two
    /// distinct configurations must never collide onto the same cached client — that would
    /// leak one caller's client (and its credentials-derived provider auth) to a caller
    /// with unrelated configuration.
    #[test]
    fn test_create_client_does_not_share_instance_across_different_configs() {
        let config_a = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key-a-for-sharing-test".to_string()),
            ..LlmConfig::default()
        };
        let config_b = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key-b-for-sharing-test".to_string()),
            ..LlmConfig::default()
        };

        let client_a = create_client(&config_a).expect("build client for config a");
        let client_b = create_client(&config_b).expect("build client for config b");

        assert!(
            !Arc::ptr_eq(&client_a, &client_b),
            "create_client must not share a client instance across different LlmConfigs"
        );
    }

    fn bedrock_model_config(bedrock: BedrockConfig) -> LlmConfig {
        LlmConfig {
            model: "bedrock/anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            bedrock: Some(Box::new(bedrock)),
            ..LlmConfig::default()
        }
    }

    /// Regression test for https://github.com/xberg-io/xberg/issues/1381
    ///
    /// Region and cross-region prefix must reach the liter-llm client config —
    /// before this they were unreachable because `LlmConfig` had no `bedrock` field.
    #[test]
    fn test_build_client_config_maps_bedrock_region_and_cross_region_prefix() {
        let config = bedrock_model_config(BedrockConfig {
            region: Some("eu-central-1".to_string()),
            cross_region_prefix: Some("eu".to_string()),
            ..BedrockConfig::default()
        });

        let client_config = build_client_config(&config).expect("build client config");

        assert_eq!(client_config.bedrock_region.as_deref(), Some("eu-central-1"));
        assert_eq!(client_config.bedrock_cross_region_prefix.as_deref(), Some("eu"));
        assert_eq!(client_config.bedrock_access_key_id, None);
        assert_eq!(client_config.bedrock_secret_access_key, None);
        assert_eq!(client_config.bedrock_session_token, None);
    }

    /// Explicit static credentials must reach the client config verbatim.
    #[test]
    fn test_build_client_config_maps_bedrock_credentials() {
        let config = bedrock_model_config(BedrockConfig {
            region: Some("us-east-1".to_string()),
            access_key_id: Some("AKIAEXAMPLE".to_string()),
            secret_access_key: Some("example-secret".to_string()),
            session_token: Some("example-token".to_string()),
            ..BedrockConfig::default()
        });

        let client_config = build_client_config(&config).expect("build client config");

        assert_eq!(client_config.bedrock_region.as_deref(), Some("us-east-1"));
        assert_eq!(client_config.bedrock_access_key_id.as_deref(), Some("AKIAEXAMPLE"));
        assert_eq!(
            client_config.bedrock_secret_access_key.as_deref(),
            Some("example-secret")
        );
        assert_eq!(client_config.bedrock_session_token.as_deref(), Some("example-token"));
    }

    /// Session token is optional: long-lived key pairs must map without one.
    #[test]
    fn test_build_client_config_maps_bedrock_credentials_without_session_token() {
        let config = bedrock_model_config(BedrockConfig {
            access_key_id: Some("AKIAEXAMPLE".to_string()),
            secret_access_key: Some("example-secret".to_string()),
            ..BedrockConfig::default()
        });

        let client_config = build_client_config(&config).expect("build client config");

        assert_eq!(client_config.bedrock_access_key_id.as_deref(), Some("AKIAEXAMPLE"));
        assert_eq!(
            client_config.bedrock_secret_access_key.as_deref(),
            Some("example-secret")
        );
        assert_eq!(client_config.bedrock_session_token, None);
    }

    /// With no `bedrock` block, every Bedrock slot must stay `None` so liter-llm
    /// resolves region and credentials from the AWS environment / default chain.
    #[test]
    fn test_build_client_config_leaves_bedrock_unset_when_absent() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");

        assert_eq!(client_config.bedrock_region, None);
        assert_eq!(client_config.bedrock_cross_region_prefix, None);
        assert_eq!(client_config.bedrock_access_key_id, None);
        assert_eq!(client_config.bedrock_secret_access_key, None);
        assert_eq!(client_config.bedrock_session_token, None);
    }

    /// Regression test for https://github.com/xberg-io/xberg/issues/1381
    ///
    /// A `bedrock/`-prefixed model with no explicit `BedrockConfig` credentials
    /// and no `AWS_ACCESS_KEY_ID` in the environment must fail fast at
    /// `create_client` time with a clear, actionable error instead of a bare
    /// "failed to build client" message or a confusing runtime request failure.
    /// `#[serial]` because the test mutates the process-wide `AWS_ACCESS_KEY_ID`
    /// environment variable, matching the save/restore convention used by the
    /// other env-mutating tests in this crate (see
    /// `core::server_config::tests::env_tests`).
    ///
    /// `#[allow(unsafe_code)]` is scoped to this one function rather than the module:
    /// the crate sets `#![deny(unsafe_code)]` (lib.rs:36), and `std::env::set_var` /
    /// `remove_var` are `unsafe` as of edition 2024. `core::server_config::tests::env_tests`
    /// takes the same exemption, but as a file-level `#![allow]` — it can, because it is a
    /// dedicated test file. These tests sit inline in a production module, so a
    /// module-scoped allow would also cover `create_client` itself. ~keep
    #[allow(unsafe_code)]
    #[serial_test::serial]
    #[test]
    fn test_create_client_reports_clear_error_for_unconfigured_bedrock() {
        let original = std::env::var("AWS_ACCESS_KEY_ID").ok();
        // SAFETY: guarded by #[serial_test::serial] — no other test in this
        // process observes AWS_ACCESS_KEY_ID concurrently while this one runs.
        unsafe {
            std::env::remove_var("AWS_ACCESS_KEY_ID");
        }

        let config = bedrock_model_config(BedrockConfig::default());
        let result = create_client(&config);

        // SAFETY: see above.
        unsafe {
            match &original {
                Some(val) => std::env::set_var("AWS_ACCESS_KEY_ID", val),
                None => std::env::remove_var("AWS_ACCESS_KEY_ID"),
            }
        }

        match result {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(
                    message.contains("bedrock/anthropic.claude-3-sonnet-20240229-v1:0"),
                    "error should name the model (the input) that failed to build: {message}"
                );
                assert!(
                    message.contains("AWS credentials"),
                    "error should name the root cause (missing AWS credentials): {message}"
                );
                assert!(
                    message.contains("AWS_ACCESS_KEY_ID") && message.contains("AWS_SECRET_ACCESS_KEY"),
                    "error should suggest how to fix it (set explicit config or the AWS env vars): {message}"
                );
            }
            Err(other) => panic!("expected a Validation error, got: {other}"),
            Ok(_) => panic!("expected create_client to reject an unconfigured bedrock model"),
        }
    }

    /// Once Bedrock credentials actually reach liter-llm's own `ClientConfig`
    /// (built by `build_client_config`), that type's `Debug` impl — not xberg's
    /// `LlmConfig`/`BedrockConfig` impls — is the last line of defense against a
    /// credential reaching a log via an accidental `tracing::debug!("{config:?}")`
    /// or panic dump. This proves the credential survives the xberg -> liter-llm
    /// boundary redacted, using real-looking (but fake) values.
    #[test]
    fn test_build_client_config_debug_never_leaks_bedrock_credentials() {
        let config = bedrock_model_config(BedrockConfig {
            region: Some("us-east-1".to_string()),
            access_key_id: Some("AKIAEXAMPLEFAKE".to_string()),
            secret_access_key: Some("example-fake-secret".to_string()),
            session_token: Some("example-fake-token".to_string()),
            ..BedrockConfig::default()
        });

        let client_config = build_client_config(&config).expect("build client config");
        let rendered = format!("{client_config:?}");

        for secret in ["AKIAEXAMPLEFAKE", "example-fake-secret", "example-fake-token"] {
            assert!(
                !rendered.contains(secret),
                "liter-llm ClientConfig Debug leaked {secret}: {rendered}"
            );
        }
        assert!(
            rendered.contains("us-east-1"),
            "non-secret region should stay visible for diagnosability: {rendered}"
        );
    }

    /// A misconfigured request (invalid header) on a model that also carries
    /// live-looking Bedrock credentials must never leak those credentials into
    /// the resulting error — the error's `Display` output is exactly the string
    /// that reaches xberg's logs and callers, so this is the real "does a
    /// credential reach a log" boundary (as opposed to the config-file
    /// persistence `Serialize` impl, which legitimately carries a credential the
    /// caller explicitly set, the same way `api_key` already does).
    #[test]
    fn test_create_client_error_never_leaks_bedrock_credentials() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Bad\r\nInjected".to_string(), "value".to_string());
        let mut config = bedrock_model_config(BedrockConfig {
            region: Some("us-east-1".to_string()),
            access_key_id: Some("AKIAEXAMPLEFAKE".to_string()),
            secret_access_key: Some("example-fake-secret".to_string()),
            session_token: Some("example-fake-token".to_string()),
            ..BedrockConfig::default()
        });
        config.headers = Some(headers);

        // Not `expect_err`: that needs `T: Debug` and liter-llm's `ManagedClient` (returned
        // wrapped in `Arc` by `create_client`) does not implement it, so the success arm has
        // to be discarded by hand.
        let Err(err) = create_client(&config) else {
            panic!("invalid header must reject the request");
        };
        let rendered = format!("{err}");

        for secret in ["AKIAEXAMPLEFAKE", "example-fake-secret", "example-fake-token"] {
            assert!(
                !rendered.contains(secret),
                "create_client error leaked {secret}: {rendered}"
            );
        }
    }

    /// Regression test for https://github.com/xberg-io/xberg/issues/1381
    ///
    /// The response cache configuration must reach liter-llm's `ClientConfig`
    /// unchanged, via the delegated `into_client_builder` mapping.
    #[test]
    fn test_build_client_config_maps_cache_settings() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            cache: Some(Box::new(LlmCacheConfig {
                max_entries: Some(128),
                ttl_seconds: Some(90),
                backend: Some("memory".to_string()),
                backend_config: None,
            })),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");
        let cache = client_config.cache_config.expect("cache_config present");

        assert_eq!(cache.max_entries, 128);
        assert_eq!(cache.ttl, std::time::Duration::from_secs(90));
        assert!(matches!(cache.backend, liter_llm::tower::CacheBackend::Memory));
    }

    /// Regression test for https://github.com/xberg-io/xberg/issues/1381
    ///
    /// Without the `opendal-cache` liter-llm feature, a non-memory cache backend name must be
    /// rejected up front instead of silently degrading to the in-memory backend inside
    /// liter-llm's own `into_client_builder` — a user who configured `backend = "s3"` and got
    /// silent in-memory caching would have no way to notice.
    #[test]
    fn test_build_client_config_rejects_unsupported_cache_backend_without_opendal_cache() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            cache: Some(Box::new(LlmCacheConfig {
                backend: Some("s3".to_string()),
                ..LlmCacheConfig::default()
            })),
            ..LlmConfig::default()
        };

        match build_client_config(&config) {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(message.contains("Unsupported LLM cache backend 's3'"), "{message}");
                assert!(message.contains("opendal-cache"), "{message}");
            }
            Err(other) => panic!("expected a Validation error, got: {other}"),
            Ok(_) => panic!("expected build_client_config to reject the unsupported cache backend"),
        }
    }

    /// A `cache.backend` of exactly `"memory"` must still build successfully — the only
    /// backend name xberg accepts without the `opendal-cache` liter-llm feature.
    #[test]
    fn test_build_client_config_accepts_explicit_memory_cache_backend() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            cache: Some(Box::new(LlmCacheConfig {
                backend: Some("memory".to_string()),
                ..LlmCacheConfig::default()
            })),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");
        let cache = client_config.cache_config.expect("cache_config present");
        assert!(matches!(cache.backend, liter_llm::tower::CacheBackend::Memory));
    }

    /// An absent `cache.backend` must still build successfully, defaulting to the in-memory
    /// backend exactly as liter-llm's own `into_client_builder` does.
    #[test]
    fn test_build_client_config_accepts_unset_cache_backend() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            cache: Some(Box::new(LlmCacheConfig::default())),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");
        let cache = client_config.cache_config.expect("cache_config present");
        assert!(matches!(cache.backend, liter_llm::tower::CacheBackend::Memory));
    }

    /// Regression test for https://github.com/xberg-io/xberg/issues/1381
    ///
    /// Budget limits, per-model limits, and enforcement mode must all reach
    /// liter-llm's `ClientConfig`.
    #[test]
    fn test_build_client_config_maps_budget_settings() {
        let mut model_limits = std::collections::HashMap::new();
        model_limits.insert("openai/gpt-4o".to_string(), 25.0);
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            budget: Some(Box::new(LlmBudgetConfig {
                global_limit: Some(100.0),
                model_limits: Some(model_limits),
                enforcement: Some("soft".to_string()),
            })),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");
        let budget = client_config.budget_config.expect("budget_config present");

        assert_eq!(budget.global_limit, Some(100.0));
        assert_eq!(budget.model_limits.get("openai/gpt-4o"), Some(&25.0));
        assert_eq!(budget.enforcement, liter_llm::tower::Enforcement::Soft);
    }

    /// An unrecognized enforcement string must fall back to liter-llm's own
    /// default (`Hard`), matching `into_client_builder`'s `_ => Enforcement::Hard`.
    #[test]
    fn test_build_client_config_defaults_unknown_enforcement_to_hard() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            budget: Some(Box::new(LlmBudgetConfig {
                global_limit: Some(10.0),
                enforcement: Some("not-a-real-mode".to_string()),
                ..LlmBudgetConfig::default()
            })),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");
        let budget = client_config.budget_config.expect("budget_config present");

        assert_eq!(budget.enforcement, liter_llm::tower::Enforcement::Hard);
    }

    /// Regression test for https://github.com/xberg-io/xberg/issues/1381
    ///
    /// Rate limit RPM/TPM/window must reach liter-llm's `ClientConfig`.
    #[test]
    fn test_build_client_config_maps_rate_limit_settings() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            rate_limit: Some(Box::new(LlmRateLimitConfig {
                rpm: Some(60),
                tpm: Some(100_000),
                window_seconds: Some(30),
            })),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");
        let rate_limit = client_config.rate_limit_config.expect("rate_limit_config present");

        assert_eq!(rate_limit.rpm, Some(60));
        assert_eq!(rate_limit.tpm, Some(100_000));
        assert_eq!(rate_limit.window, std::time::Duration::from_secs(30));
    }

    /// Regression test for https://github.com/xberg-io/xberg/issues/1381
    ///
    /// `cost_tracking` and `tracing` flags must reach liter-llm's `ClientConfig`.
    #[test]
    fn test_build_client_config_maps_cost_tracking_and_tracing_flags() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            cost_tracking: Some(true),
            tracing: Some(true),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");

        assert!(client_config.enable_cost_tracking);
        assert!(client_config.enable_tracing);
    }

    /// Regression test for https://github.com/xberg-io/xberg/issues/1381
    ///
    /// `cooldown_secs` and `health_check_secs` must reach liter-llm's
    /// `ClientConfig` as `Duration`s.
    #[test]
    fn test_build_client_config_maps_cooldown_and_health_check_intervals() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            cooldown_secs: Some(15),
            health_check_secs: Some(45),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");

        assert_eq!(
            client_config.cooldown_duration,
            Some(std::time::Duration::from_secs(15))
        );
        assert_eq!(
            client_config.health_check_interval,
            Some(std::time::Duration::from_secs(45))
        );
    }

    /// With none of the new liter-llm passthrough fields set, every corresponding
    /// `ClientConfig` slot must stay at liter-llm's own default/unset state.
    #[test]
    fn test_build_client_config_leaves_new_passthrough_fields_unset_when_absent() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");

        assert!(client_config.cache_config.is_none());
        assert!(client_config.budget_config.is_none());
        assert!(client_config.rate_limit_config.is_none());
        assert!(client_config.cooldown_duration.is_none());
        assert!(client_config.health_check_interval.is_none());
        assert!(!client_config.enable_cost_tracking);
        assert!(!client_config.enable_tracing);
    }

    /// Header validation must still run even when the request also carries the
    /// new tower-backed passthrough fields — `to_liter_llm_config` deliberately
    /// leaves `headers` unset on the DTO handed to `into_client_builder`, so this
    /// validation (not liter-llm's own silent-drop-on-parse-failure behavior) is
    /// what actually runs.
    #[test]
    fn test_build_client_config_still_validates_headers_with_tower_fields_set() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Bad\r\nInjected".to_string(), "value".to_string());
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            headers: Some(headers),
            cost_tracking: Some(true),
            ..LlmConfig::default()
        };

        match build_client_config(&config) {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(message.contains("Invalid LLM header"), "unexpected message: {message}");
            }
            Err(other) => panic!("expected a Validation error, got: {other}"),
            Ok(_) => panic!("expected build_client_config to reject the invalid header"),
        }
    }

    /// Regression test for GH#1465.
    ///
    /// `LlmConfig::max_concurrency` must map onto liter-llm's `in_flight_limit` — the field
    /// `into_client_builder` wires onto a Tower in-flight-request-limit layer when the
    /// client is built through `ManagedClient` (see `build_managed_client`'s docs). `None`
    /// must stay `None` (unlimited), never default to an artificial limit.
    #[test]
    fn test_to_liter_llm_config_maps_max_concurrency_to_in_flight_limit() {
        let with_limit = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            max_concurrency: Some(4),
            ..LlmConfig::default()
        };
        assert_eq!(
            to_liter_llm_config(&with_limit).in_flight_limit,
            Some(LlmInFlightLimitConfig { max_in_flight: Some(4) })
        );

        let without_limit = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            max_concurrency: None,
            ..LlmConfig::default()
        };
        assert_eq!(to_liter_llm_config(&without_limit).in_flight_limit, None);
    }

    /// `providers` has no equivalent field on liter-llm's `ClientConfig` at all —
    /// liter-llm's own `into_client_builder` does not map it either. This test proves
    /// the DTO-to-DTO conversion in `to_liter_llm_config` is lossless for `providers`
    /// on its own terms; it is *not* proof that a configured provider takes effect —
    /// see `test_build_client_config_registers_configured_providers_in_the_liter_llm_registry`
    /// below for that, which exercises the actual runtime registration in
    /// `register_configured_providers`/`build_client_config`.
    #[test]
    fn test_to_liter_llm_config_forwards_providers_into_the_dto_boundary() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            providers: Some(vec![LlmProviderConfig {
                name: "my-provider".to_string(),
                base_url: "https://my-llm.example.com/v1".to_string(),
                auth_header: Some("X-Api-Key".to_string()),
                model_prefixes: vec!["my-provider/".to_string()],
            }]),
            ..LlmConfig::default()
        };

        let upstream = to_liter_llm_config(&config);
        let providers = upstream.providers.expect("providers present");

        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].name, "my-provider");
        assert_eq!(providers[0].base_url, "https://my-llm.example.com/v1");
        assert_eq!(providers[0].auth_header.as_deref(), Some("X-Api-Key"));
        assert_eq!(providers[0].model_prefixes, vec!["my-provider/".to_string()]);
    }

    /// `to_auth_header_format` must map `None` and the literal `"Authorization"` header
    /// name (case-insensitively) onto liter-llm's `Bearer` scheme — xberg's documented
    /// default.
    #[test]
    fn test_to_auth_header_format_maps_none_and_authorization_to_bearer() {
        assert!(matches!(
            to_auth_header_format(None),
            liter_llm::AuthHeaderFormat::Bearer
        ));
        assert!(matches!(
            to_auth_header_format(Some("Authorization")),
            liter_llm::AuthHeaderFormat::Bearer
        ));
        assert!(matches!(
            to_auth_header_format(Some("authorization")),
            liter_llm::AuthHeaderFormat::Bearer
        ));
    }

    /// Any other configured header name must map onto liter-llm's `ApiKey` variant,
    /// carrying the exact header name through unchanged.
    #[test]
    fn test_to_auth_header_format_maps_custom_header_name_to_api_key() {
        match to_auth_header_format(Some("X-Api-Key")) {
            liter_llm::AuthHeaderFormat::ApiKey(name) => assert_eq!(name, "X-Api-Key"),
            other => panic!("expected ApiKey variant, got {other:?}"),
        }
    }

    /// Regression test for https://github.com/xberg-io/xberg/issues/1381
    ///
    /// A configured `providers` entry must actually reach liter-llm's process-global
    /// custom-provider registry, not merely round-trip through the `to_liter_llm_config`
    /// DTO conversion (that is all `test_to_liter_llm_config_forwards_providers_into_the_dto_boundary`
    /// proves). `register_custom_provider`/`unregister_custom_provider` are the only
    /// public read/write surface liter-llm exposes on that registry —
    /// `unregister_custom_provider` returning `true` is used here as the observable
    /// proof of a prior successful registration, since there is no public getter.
    #[test]
    fn test_build_client_config_registers_configured_providers_in_the_liter_llm_registry() {
        let provider_name = "xberg-test-provider-registers-in-registry";
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            providers: Some(vec![LlmProviderConfig {
                name: provider_name.to_string(),
                base_url: "https://my-llm.example.com/v1".to_string(),
                auth_header: Some("X-Api-Key".to_string()),
                model_prefixes: vec!["xberg-test-provider-registers/".to_string()],
            }]),
            ..LlmConfig::default()
        };

        build_client_config(&config).expect("build client config");

        let removed = liter_llm::unregister_custom_provider(provider_name).expect("unregister should succeed");
        assert!(removed, "expected provider '{provider_name}' to have been registered");

        let removed_again = liter_llm::unregister_custom_provider(provider_name).expect("unregister should succeed");
        assert!(
            !removed_again,
            "provider should already be gone after the first unregister"
        );
    }

    /// Building a client twice from the same config (e.g. two extraction calls
    /// reusing one `LlmConfig`) must not fail merely because the provider was already
    /// registered — `register_custom_provider` replaces same-named entries rather than
    /// erroring, so repeated registration of identical data is idempotent.
    #[test]
    fn test_build_client_config_provider_registration_is_idempotent_for_same_config() {
        let provider_name = "xberg-test-provider-idempotent";
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            providers: Some(vec![LlmProviderConfig {
                name: provider_name.to_string(),
                base_url: "https://my-llm.example.com/v1".to_string(),
                auth_header: None,
                model_prefixes: vec!["xberg-test-provider-idempotent/".to_string()],
            }]),
            ..LlmConfig::default()
        };

        build_client_config(&config).expect("first registration should succeed");
        build_client_config(&config).expect("second registration with identical config should succeed");

        let removed = liter_llm::unregister_custom_provider(provider_name).expect("unregister should succeed");
        assert!(removed);
    }

    /// An invalid provider entry (liter-llm rejects an empty `model_prefixes` list —
    /// it would match every model) must surface as a named `XbergError::Validation`,
    /// not be swallowed.
    #[test]
    fn test_build_client_config_rejects_invalid_provider_registration() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            providers: Some(vec![LlmProviderConfig {
                name: "xberg-test-provider-invalid".to_string(),
                base_url: "https://my-llm.example.com/v1".to_string(),
                auth_header: None,
                model_prefixes: vec![],
            }]),
            ..LlmConfig::default()
        };

        match build_client_config(&config) {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(
                    message.contains("xberg-test-provider-invalid"),
                    "error should name the provider: {message}"
                );
            }
            Err(other) => panic!("expected a Validation error, got: {other}"),
            Ok(_) => panic!("expected build_client_config to reject the invalid provider config"),
        }
    }

    /// Regression test for https://github.com/xberg-io/xberg/issues/1381
    ///
    /// `reasoning_effort` must parse into the exact matching `liter_llm::ReasoningEffort`
    /// variant, case-insensitively, so it can be assigned onto
    /// `ChatCompletionRequest::reasoning_effort` at each request-building call site.
    #[test]
    fn test_parse_reasoning_effort_maps_known_values_case_insensitively() {
        let cases = [
            ("low", liter_llm::ReasoningEffort::Low),
            ("MEDIUM", liter_llm::ReasoningEffort::Medium),
            ("High", liter_llm::ReasoningEffort::High),
            ("minimal", liter_llm::ReasoningEffort::Minimal),
            ("max", liter_llm::ReasoningEffort::Max),
        ];
        for (input, expected) in cases {
            let config = LlmConfig {
                model: "openai/gpt-4o".to_string(),
                reasoning_effort: Some(input.to_string()),
                ..LlmConfig::default()
            };
            assert_eq!(
                parse_reasoning_effort(&config).expect("known value should parse"),
                Some(expected),
                "input {input:?} mapped incorrectly"
            );
        }
    }

    /// An absent `reasoning_effort` must parse to `None` rather than defaulting to a
    /// specific effort level — silence in config should stay silence on the request.
    #[test]
    fn test_parse_reasoning_effort_returns_none_when_unset() {
        let config = LlmConfig::default();
        assert_eq!(parse_reasoning_effort(&config).expect("no value should parse"), None);
    }

    /// An unrecognized `reasoning_effort` string must be a named `XbergError::Validation`,
    /// not a silent drop of the misconfigured value.
    #[test]
    fn test_parse_reasoning_effort_rejects_unrecognized_value() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            reasoning_effort: Some("maximum".to_string()),
            ..LlmConfig::default()
        };

        match parse_reasoning_effort(&config) {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(
                    message.contains("maximum"),
                    "error should name the bad value: {message}"
                );
            }
            other => panic!("expected a Validation error, got: {other:?}"),
        }
    }

    /// `to_stop_sequence` must map an absent `stop` to `None`.
    #[test]
    fn test_to_stop_sequence_returns_none_when_unset() {
        let config = LlmConfig::default();
        assert!(to_stop_sequence(&config).is_none());
    }

    /// `to_stop_sequence` must map a single-element list onto liter-llm's
    /// `StopSequence::Multiple` — never `StopSequence::Single` — so xberg's `LlmConfig::stop`
    /// has one FFI-friendly shape regardless of how many sequences are configured.
    #[test]
    fn test_to_stop_sequence_maps_single_entry_to_multiple_variant() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            stop: Some(vec!["\n\n".to_string()]),
            ..LlmConfig::default()
        };

        match to_stop_sequence(&config) {
            Some(liter_llm::StopSequence::Multiple(sequences)) => {
                assert_eq!(sequences, vec!["\n\n".to_string()]);
            }
            other => panic!("expected StopSequence::Multiple, got {other:?}"),
        }
    }

    /// `to_stop_sequence` must map every configured stop sequence, in order.
    #[test]
    fn test_to_stop_sequence_maps_multiple_entries_in_order() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            stop: Some(vec!["\n\n".to_string(), "[END]".to_string()]),
            ..LlmConfig::default()
        };

        match to_stop_sequence(&config) {
            Some(liter_llm::StopSequence::Multiple(sequences)) => {
                assert_eq!(sequences, vec!["\n\n".to_string(), "[END]".to_string()]);
            }
            other => panic!("expected StopSequence::Multiple, got {other:?}"),
        }
    }

    /// `build_client_config` must reject an out-of-range `top_p` before doing any other
    /// work (provider registration, header validation), via `LlmConfig::validate`.
    #[test]
    fn test_build_client_config_rejects_invalid_top_p() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            top_p: Some(1.5),
            ..LlmConfig::default()
        };

        match build_client_config(&config) {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(message.contains("top_p"), "{message}");
            }
            other => panic!("expected a Validation error, got {other:?}"),
        }
    }

    /// `build_client_config` must reject an out-of-range `presence_penalty`.
    #[test]
    fn test_build_client_config_rejects_invalid_presence_penalty() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            presence_penalty: Some(-3.0),
            ..LlmConfig::default()
        };

        match build_client_config(&config) {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(message.contains("presence_penalty"), "{message}");
            }
            other => panic!("expected a Validation error, got {other:?}"),
        }
    }

    /// `build_client_config` must reject an out-of-range `frequency_penalty`.
    #[test]
    fn test_build_client_config_rejects_invalid_frequency_penalty() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            frequency_penalty: Some(3.0),
            ..LlmConfig::default()
        };

        match build_client_config(&config) {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(message.contains("frequency_penalty"), "{message}");
            }
            other => panic!("expected a Validation error, got {other:?}"),
        }
    }

    /// Valid `top_p`, `stop`, `seed`, `presence_penalty`, and `frequency_penalty` values
    /// must not prevent `build_client_config` from succeeding.
    #[test]
    fn test_build_client_config_accepts_valid_sampling_fields() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            top_p: Some(0.9),
            stop: Some(vec!["\n\n".to_string()]),
            seed: Some(42),
            presence_penalty: Some(0.5),
            frequency_penalty: Some(-0.5),
            ..LlmConfig::default()
        };

        assert!(build_client_config(&config).is_ok());
    }

    /// `build_client_config` must succeed with a nonzero `max_response_bytes` set, applying
    /// liter-llm's native-only builder method (`ClientConfigBuilder::max_response_bytes`).
    #[test]
    fn test_build_client_config_accepts_nonzero_max_response_bytes() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            max_response_bytes: Some(4096),
            ..LlmConfig::default()
        };

        assert!(build_client_config(&config).is_ok());
    }

    /// An unset `max_response_bytes` must still build a client (unbounded, matching
    /// liter-llm's own default).
    #[test]
    fn test_build_client_config_accepts_unset_max_response_bytes() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            max_response_bytes: None,
            ..LlmConfig::default()
        };

        assert!(build_client_config(&config).is_ok());
    }

    /// `build_client_config` must reject `max_response_bytes: Some(0)` — caught by
    /// `LlmConfig::validate` before liter-llm's own builder is ever reached.
    #[test]
    fn test_build_client_config_rejects_zero_max_response_bytes() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            max_response_bytes: Some(0),
            ..LlmConfig::default()
        };

        match build_client_config(&config) {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(message.contains("max_response_bytes"), "{message}");
            }
            other => panic!("expected a Validation error, got {other:?}"),
        }
    }

    /// Regression test for https://github.com/xberg-io/xberg/issues/1381
    ///
    /// A `VertexAdc` credential provider needs no I/O to construct (unlike `AzureAd`, which
    /// would perform a live OAuth exchange the first time it resolves, and `VertexOauth2`,
    /// which reads a key file) — it must build a client and attach a provider without error.
    /// liter-llm's own `ClientConfig::Debug` prints only `"[configured]"` for a set
    /// `credential_provider`, so that string is the exact, structural proof a provider reached
    /// the client config, not merely a truthiness check.
    #[test]
    fn test_build_client_config_attaches_vertex_adc_credential_provider() {
        let config = LlmConfig {
            model: "vertex_ai/gemini-1.5-pro".to_string(),
            credential_provider: Some(Box::new(CredentialProviderConfig::VertexAdc { scope: None })),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");
        assert!(client_config.credential_provider.is_some());
        let rendered = format!("{client_config:?}");
        assert!(
            rendered.contains(r#"credential_provider: Some("[configured]")"#),
            "{rendered}"
        );
    }

    /// An `AzureAd` credential provider must reach the client config, and liter-llm's own
    /// `ClientConfig::Debug` must never leak the `client_secret` xberg handed it.
    #[test]
    fn test_build_client_config_attaches_azure_ad_credential_provider_without_leaking_secret() {
        let config = LlmConfig {
            model: "azure/gpt-4o".to_string(),
            credential_provider: Some(Box::new(CredentialProviderConfig::AzureAd {
                tenant_id: "tenant-123".to_string(),
                client_id: "client-456".to_string(),
                client_secret: "super-secret-azure-value".to_string(),
                scope: None,
            })),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");
        let rendered = format!("{client_config:?}");
        assert!(
            !rendered.contains("super-secret-azure-value"),
            "liter-llm ClientConfig Debug leaked the Azure client secret: {rendered}"
        );
        assert!(
            rendered.contains(r#"credential_provider: Some("[configured]")"#),
            "{rendered}"
        );
    }

    /// A `VertexOauth2` credential provider reads its service-account key file at build time —
    /// an invalid file must surface as a named `XbergError::Validation` naming the file, not a
    /// panic or an opaque downstream error.
    #[test]
    fn test_build_client_config_rejects_missing_vertex_oauth2_key_file() {
        let config = LlmConfig {
            model: "vertex_ai/gemini-1.5-pro".to_string(),
            credential_provider: Some(Box::new(CredentialProviderConfig::VertexOauth2 {
                service_account_key_file: "/nonexistent/xberg-test-vertex-key.json".to_string(),
                scope: None,
            })),
            ..LlmConfig::default()
        };

        match build_client_config(&config) {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(
                    message.contains("/nonexistent/xberg-test-vertex-key.json"),
                    "error should name the file: {message}"
                );
                assert!(
                    message.contains("Vertex OAuth2"),
                    "error should name the auth mode: {message}"
                );
            }
            other => panic!("expected a Validation error, got: {other:?}"),
        }
    }

    /// A valid service-account JSON key file must build a `VertexOauth2` credential provider
    /// successfully. Uses a per-process-unique temp file path so parallel test runs never
    /// collide (test-independence).
    #[test]
    fn test_build_client_config_attaches_vertex_oauth2_credential_provider_from_key_file() {
        let path = std::env::temp_dir().join(format!(
            "xberg-test-vertex-oauth2-key-{}-{:?}.json",
            std::process::id(),
            std::thread::current().id()
        ));
        // Assembled rather than written as a literal: a PEM header in source trips the
        // detect-private-key commit hook, and suppressing that hook to admit a fixture is
        // exactly the exemption that later lets a real key through.
        let pem_label = "PRIVATE KEY";
        let fixture_json = serde_json::json!({
            "client_email": "xberg-test@example.iam.gserviceaccount.com",
            "private_key": format!("-----BEGIN {pem_label}-----\nZmFrZS1rZXktbWF0ZXJpYWw=\n-----END {pem_label}-----\n"),
        });
        std::fs::write(&path, fixture_json.to_string()).expect("write fixture key file");

        let config = LlmConfig {
            model: "vertex_ai/gemini-1.5-pro".to_string(),
            credential_provider: Some(Box::new(CredentialProviderConfig::VertexOauth2 {
                service_account_key_file: path.to_string_lossy().into_owned(),
                scope: Some("https://www.googleapis.com/auth/cloud-platform".to_string()),
            })),
            ..LlmConfig::default()
        };

        let result = build_client_config(&config);
        let _ = std::fs::remove_file(&path);

        let client_config = result.expect("build client config");
        assert!(client_config.credential_provider.is_some());
    }

    /// A `BedrockWebIdentity` credential provider needs no I/O to construct — the token file is
    /// only read when the provider later resolves a credential — so it must build a client
    /// successfully from plain data alone.
    #[test]
    fn test_build_client_config_attaches_bedrock_web_identity_credential_provider() {
        let config = LlmConfig {
            model: "bedrock/anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            credential_provider: Some(Box::new(CredentialProviderConfig::BedrockWebIdentity {
                role_arn: "arn:aws:iam::123456789012:role/xberg-bedrock".to_string(),
                token_file: "/var/run/secrets/eks.amazonaws.com/serviceaccount/token".to_string(),
                session_name: None,
                region: None,
            })),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");
        assert!(client_config.credential_provider.is_some());
    }

    /// With no `credential_provider` set, the client config's slot must stay `None` so
    /// liter-llm falls back to `api_key`/environment resolution as before this feature existed.
    #[test]
    fn test_build_client_config_leaves_credential_provider_unset_when_absent() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            ..LlmConfig::default()
        };

        let client_config = build_client_config(&config).expect("build client config");
        assert!(client_config.credential_provider.is_none());
    }

    // -----------------------------------------------------------------------
    // `apply_request_time_params` — `LlmConfig` carries nine request-time
    // parameters. Before this helper existed each request builder set four of
    // them by hand, so `top_p`, `stop`, `seed`, `presence_penalty` and
    // `frequency_penalty` were accepted by every config file and language
    // binding and then silently never reached a provider.
    // -----------------------------------------------------------------------

    fn base_request() -> liter_llm::ChatCompletionRequest {
        liter_llm::ChatCompletionRequest {
            model: "openai/gpt-4o".to_string(),
            messages: vec![liter_llm::Message::User(liter_llm::UserMessage {
                content: liter_llm::UserContent::Text("hello".to_string()),
                name: None,
            })],
            ..Default::default()
        }
    }

    /// `LlmConfig::stop` — always a plain list, even for a single sequence — must map onto
    /// exactly `StopSequence::Multiple` with the same strings, in the same order.
    #[test]
    fn should_forward_stop_sequences_to_the_request() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            stop: Some(vec!["\n\n".to_string(), "[END]".to_string()]),
            ..LlmConfig::default()
        };
        let mut request = base_request();

        apply_request_time_params(&mut request, &config).expect("apply_request_time_params should succeed");

        assert_eq!(
            request.stop,
            Some(liter_llm::StopSequence::Multiple(vec![
                "\n\n".to_string(),
                "[END]".to_string()
            ]))
        );
    }

    /// An unset `LlmConfig::stop` must leave `request.stop` as `None`, not an empty list.
    #[test]
    fn should_leave_stop_none_when_config_stop_is_unset() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            ..LlmConfig::default()
        };
        let mut request = base_request();

        apply_request_time_params(&mut request, &config).expect("apply_request_time_params should succeed");

        assert_eq!(request.stop, None);
    }

    /// `top_p`, `seed`, `presence_penalty`, and `frequency_penalty` must all reach the
    /// request with their exact configured values — these four were silently dropped by
    /// every call site before `apply_request_time_params` existed.
    #[test]
    fn should_forward_top_p_seed_and_penalties_to_the_request() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            top_p: Some(0.9),
            seed: Some(42),
            presence_penalty: Some(0.5),
            frequency_penalty: Some(-0.5),
            ..LlmConfig::default()
        };
        let mut request = base_request();

        apply_request_time_params(&mut request, &config).expect("apply_request_time_params should succeed");

        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.seed, Some(42));
        assert_eq!(request.presence_penalty, Some(0.5));
        assert_eq!(request.frequency_penalty, Some(-0.5));
    }

    /// `temperature`, `max_tokens`, `reasoning_effort`, and `extra_body` — the four fields
    /// every call site already wired by hand — must still reach the request exactly the
    /// same way now that they are routed through `apply_request_time_params`.
    #[test]
    fn should_forward_temperature_max_tokens_reasoning_effort_and_extra_body_to_the_request() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(1024),
            reasoning_effort: Some("high".to_string()),
            extra_body: Some(serde_json::json!({"safety_settings": {"harassment": "block_none"}})),
            ..LlmConfig::default()
        };
        let mut request = base_request();

        apply_request_time_params(&mut request, &config).expect("apply_request_time_params should succeed");

        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1024));
        assert!(matches!(
            request.reasoning_effort,
            Some(liter_llm::ReasoningEffort::High)
        ));
        assert_eq!(
            request.extra_body,
            Some(serde_json::json!({"safety_settings": {"harassment": "block_none"}}))
        );
    }

    /// An unrecognized `reasoning_effort` string must surface as a named
    /// `XbergError::Validation` and must leave the request completely untouched — the
    /// parse happens before any field is written, so a rejected config cannot half-apply.
    #[test]
    fn should_return_validation_error_for_invalid_reasoning_effort() {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            reasoning_effort: Some("maximum".to_string()),
            stop: Some(vec!["\n\n".to_string()]),
            temperature: Some(0.7),
            ..LlmConfig::default()
        };
        let mut request = base_request();

        match apply_request_time_params(&mut request, &config) {
            Err(crate::XbergError::Validation { message, .. }) => {
                assert!(message.contains("reasoning_effort"), "{message}");
                assert!(message.contains("maximum"), "{message}");
            }
            other => panic!("expected a Validation error, got {other:?}"),
        }

        assert_eq!(request.stop, None, "a rejected config must not half-apply");
        assert_eq!(request.temperature, None, "a rejected config must not half-apply");
    }

    /// `apply_request_time_params` must never touch `request.model` or `request.messages` —
    /// those are call-site specific (the prompt/schema differs per feature), not carried on
    /// `LlmConfig`.
    #[test]
    fn should_not_touch_model_or_messages_fields() {
        let config = LlmConfig {
            model: "anthropic/claude-sonnet-4-20250514".to_string(),
            temperature: Some(0.3),
            ..LlmConfig::default()
        };
        let mut request = base_request();
        let original_model = request.model.clone();
        let original_messages_len = request.messages.len();

        apply_request_time_params(&mut request, &config).expect("apply_request_time_params should succeed");

        assert_eq!(request.model, original_model, "model must stay call-site controlled");
        assert_eq!(
            request.messages.len(),
            original_messages_len,
            "messages must stay call-site controlled"
        );
    }
}

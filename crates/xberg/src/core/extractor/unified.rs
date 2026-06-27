//! Unified public extraction API.

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

#[cfg(feature = "url-ingestion")]
use std::borrow::Cow;

#[cfg(feature = "url-ingestion")]
use crawlberg::{CrawlConfig, CrawlEngine, CrawlPageResult, DownloadedDocument, ScrapeResult};

#[cfg(feature = "url-ingestion")]
use crate::core::config::UrlExtractionMode;
use crate::core::config::{
    ExtractInput, ExtractInputKind, ExtractionConfig, ExtractionErrorItem, ExtractionOutput, ExtractionSummary,
};
#[cfg(feature = "url-ingestion")]
use crate::types::{ExtractedUri, Metadata};
use crate::types::{ExtractionResult, UriKind};
use crate::{Result, XbergError};

use super::bytes::extract_bytes;
use super::file::extract_file;

const HTTP_SCHEME: &str = "http://";
const HTTPS_SCHEME: &str = "https://";
const FILE_SCHEME: &str = "file://";

/// Extract content from a single bytes or URI input.
pub async fn extract(input: ExtractInput, config: &ExtractionConfig) -> Result<ExtractionOutput> {
    let mut seen = initial_seen_urls(std::slice::from_ref(&input));
    let seed_hosts = initial_seed_hosts(std::slice::from_ref(&input));
    let mut output = extract_one(input, config, 0).await?;
    follow_recursive_document_urls(&mut output, config, &mut seen, &seed_hosts).await?;
    Ok(output)
}

/// Extract content from multiple bytes or URI inputs.
pub async fn extract_batch(inputs: Vec<ExtractInput>, config: &ExtractionConfig) -> Result<ExtractionOutput> {
    let mut seen = initial_seen_urls(&inputs);
    let seed_hosts = initial_seed_hosts(&inputs);
    let mut output = ExtractionOutput {
        summary: ExtractionSummary {
            inputs: inputs.len(),
            ..Default::default()
        },
        ..Default::default()
    };

    for (index, input) in inputs.into_iter().enumerate() {
        let source = input_source(&input);
        match extract_one(input, config, index).await {
            Ok(mut item_output) => {
                output.summary.remote_urls += item_output.summary.remote_urls;
                output.summary.pages_crawled += item_output.summary.pages_crawled;
                output.summary.documents_downloaded += item_output.summary.documents_downloaded;
                output.results.append(&mut item_output.results);
                output.errors.append(&mut item_output.errors);
                merge_crawl_summary(
                    &mut output,
                    item_output.crawl_final_urls,
                    item_output.crawl_redirect_count,
                    item_output.crawl_unique_normalized_urls,
                );
            }
            Err(error) => output.errors.push(error_item(index, source, &error)),
        }
    }

    output.refresh_counts();
    follow_recursive_document_urls(&mut output, config, &mut seen, &seed_hosts).await?;
    Ok(output)
}

async fn extract_one(input: ExtractInput, base_config: &ExtractionConfig, index: usize) -> Result<ExtractionOutput> {
    let config = input
        .config
        .as_ref()
        .map(|overrides| base_config.with_file_overrides(overrides))
        .unwrap_or_else(|| base_config.clone());

    match input.kind {
        ExtractInputKind::Bytes => extract_bytes_input(input, &config, index).await,
        ExtractInputKind::Uri => extract_uri_input(input, &config, index).await,
    }
}

async fn extract_bytes_input(input: ExtractInput, config: &ExtractionConfig, index: usize) -> Result<ExtractionOutput> {
    let bytes = input
        .bytes
        .ok_or_else(|| XbergError::validation("extract input kind 'bytes' requires the 'bytes' field".to_string()))?;
    let mime_type = resolve_bytes_mime_type(input.mime_type.as_deref(), input.filename.as_deref(), &bytes)?;
    let mut result = extract_bytes(&bytes, &mime_type, config).await?;
    annotate_source(
        &mut result,
        "bytes",
        input.filename.as_deref().unwrap_or("<bytes>"),
        input.filename.as_deref().unwrap_or("<bytes>"),
        index,
    );
    Ok(ExtractionOutput::single(result))
}

async fn extract_uri_input(input: ExtractInput, config: &ExtractionConfig, index: usize) -> Result<ExtractionOutput> {
    let uri = input
        .uri
        .ok_or_else(|| XbergError::validation("extract input kind 'uri' requires the 'uri' field".to_string()))?;

    if uri.starts_with(HTTP_SCHEME) || uri.starts_with(HTTPS_SCHEME) {
        return extract_remote_uri(&uri, config, index).await;
    }

    if uri.contains("://") && !uri.starts_with(FILE_SCHEME) {
        return Err(XbergError::UnsupportedFormat(format!(
            "unsupported URI scheme for extraction input: {uri}"
        )));
    }

    let path = if uri.starts_with(FILE_SCHEME) {
        if !config.url.allow_local_file_inputs || !config.url.allow_file_uris {
            return Err(XbergError::validation(
                "file:// URI inputs are disabled by configuration".to_string(),
            ));
        }
        file_uri_to_path(&uri)?
    } else {
        if !config.url.allow_local_file_inputs {
            return Err(XbergError::validation(
                "local filesystem path inputs are disabled by configuration".to_string(),
            ));
        }
        PathBuf::from(&uri)
    };

    let mut result = extract_file(&path, input.mime_type.as_deref(), config).await?;
    annotate_source(&mut result, "uri", &uri, path.to_string_lossy().as_ref(), index);
    Ok(ExtractionOutput::single(result))
}

fn resolve_bytes_mime_type(mime_type: Option<&str>, filename: Option<&str>, bytes: &[u8]) -> Result<String> {
    if let Some(mime_type) = mime_type {
        return Ok(mime_type.to_string());
    }

    if let Some(filename) = filename
        && let Ok(detected) = crate::core::mime::detect_mime_type(filename, false)
    {
        return Ok(detected);
    }

    if let Some(kind) = infer::get(bytes) {
        return Ok(kind.mime_type().to_string());
    }

    Ok("application/octet-stream".to_string())
}

fn file_uri_to_path(uri: &str) -> Result<PathBuf> {
    let parsed = url::Url::parse(uri)
        .map_err(|error| XbergError::validation(format!("invalid file URI for extraction input: {error}")))?;
    if parsed.scheme() != "file" {
        return Err(XbergError::UnsupportedFormat(format!(
            "unsupported URI scheme for local file extraction: {}",
            parsed.scheme()
        )));
    }

    if let Some(host) = parsed.host_str().filter(|host| !host.is_empty())
        && !host.eq_ignore_ascii_case("localhost")
    {
        return Err(XbergError::UnsupportedFormat(format!(
            "unsupported non-local file URI host: {host}"
        )));
    }

    parsed
        .to_file_path()
        .map_err(|()| XbergError::UnsupportedFormat(format!("unsupported file URI path: {uri}")))
}

fn annotate_source(result: &mut ExtractionResult, source_kind: &str, source_uri: &str, final_uri: &str, index: usize) {
    result
        .metadata
        .additional
        .insert("source_kind".into(), serde_json::json!(source_kind));
    result
        .metadata
        .additional
        .insert("source_uri".into(), serde_json::json!(source_uri));
    result
        .metadata
        .additional
        .insert("final_uri".into(), serde_json::json!(final_uri));
    result
        .metadata
        .additional
        .insert("source_index".into(), serde_json::json!(index));
}

fn input_source(input: &ExtractInput) -> String {
    match input.kind {
        ExtractInputKind::Bytes => input.filename.clone().unwrap_or_else(|| "<bytes>".to_string()),
        ExtractInputKind::Uri => input.uri.clone().unwrap_or_else(|| "<uri>".to_string()),
    }
}

#[cfg(feature = "url-ingestion")]
async fn extract_remote_uri(uri: &str, config: &ExtractionConfig, index: usize) -> Result<ExtractionOutput> {
    let crawl_config = crawlberg_config(config)?;
    let engine = CrawlEngine::builder()
        .config(crawl_config)
        .build()
        .map_err(map_crawl_error)?;

    match config.url.mode {
        UrlExtractionMode::Auto | UrlExtractionMode::Document => {
            let scrape = engine.scrape(uri).await.map_err(map_crawl_error)?;
            output_from_scrape(scrape, config, index).await
        }
        UrlExtractionMode::Crawl => {
            let crawl = engine.crawl(uri).await.map_err(map_crawl_error)?;
            output_from_crawl(crawl, config, index).await
        }
    }
}

#[cfg(not(feature = "url-ingestion"))]
async fn extract_remote_uri(uri: &str, _config: &ExtractionConfig, _index: usize) -> Result<ExtractionOutput> {
    Err(XbergError::UnsupportedFormat(format!(
        "HTTP(S) URI extraction requires the 'url-ingestion' feature: {uri}"
    )))
}

#[cfg(feature = "url-ingestion")]
fn crawlberg_config(config: &ExtractionConfig) -> Result<CrawlConfig> {
    let crawl_config = config.url.crawl.clone();
    crawl_config.validate().map_err(map_crawl_error)?;
    Ok(crawl_config)
}

#[cfg(feature = "url-ingestion")]
async fn output_from_scrape(scrape: ScrapeResult, config: &ExtractionConfig, index: usize) -> Result<ExtractionOutput> {
    let mut output = ExtractionOutput {
        summary: ExtractionSummary {
            inputs: 1,
            remote_urls: 1,
            ..Default::default()
        },
        ..Default::default()
    };

    if let Some(document) = scrape.downloaded_document {
        let result = extract_downloaded_document(document, config, index).await?;
        output.results.push(result);
        output.summary.documents_downloaded = 1;
    } else {
        output.results.push(result_from_scrape_page(&scrape, index));
        output.summary.pages_crawled = 1;
    }

    merge_crawl_summary(&mut output, vec![scrape.final_url], 0, Vec::new());
    output.refresh_counts();
    Ok(output)
}

#[cfg(feature = "url-ingestion")]
async fn output_from_crawl(
    crawl: crawlberg::CrawlResult,
    config: &ExtractionConfig,
    index: usize,
) -> Result<ExtractionOutput> {
    let final_url = crawl.final_url.clone();
    let redirect_count = crawl.redirect_count;
    let unique_normalized_urls = crawl.normalized_urls.clone();
    let crawl_error = crawl.error.clone();
    let mut output = ExtractionOutput {
        summary: ExtractionSummary {
            inputs: 1,
            remote_urls: 1,
            pages_crawled: crawl.pages.len(),
            ..Default::default()
        },
        ..Default::default()
    };

    for page in crawl.pages {
        if let Some(document) = page.downloaded_document.clone() {
            match extract_downloaded_document(document, config, index).await {
                Ok(result) => {
                    output.results.push(result);
                    output.summary.documents_downloaded += 1;
                }
                Err(error) => output.errors.push(error_item(index, page.url.clone(), &error)),
            }
        } else {
            output.results.push(result_from_crawl_page(&page, index));
        }
    }

    if let Some(error) = crawl_error {
        output.errors.push(ExtractionErrorItem {
            index,
            code: 1099,
            error_type: "crawl".into(),
            source: final_url.clone(),
            message: error,
        });
    }

    merge_crawl_summary(&mut output, vec![final_url], redirect_count, unique_normalized_urls);
    output.refresh_counts();
    Ok(output)
}

fn merge_crawl_summary(
    output: &mut ExtractionOutput,
    final_urls: Vec<String>,
    redirect_count: usize,
    unique_normalized_urls: Vec<String>,
) {
    if final_urls.is_empty() && redirect_count == 0 && unique_normalized_urls.is_empty() {
        return;
    }

    output.crawl_redirect_count += redirect_count;
    for url in final_urls {
        if !output.crawl_final_urls.contains(&url) {
            output.crawl_final_urls.push(url);
        }
    }
    for url in unique_normalized_urls {
        if !output.crawl_unique_normalized_urls.contains(&url) {
            output.crawl_unique_normalized_urls.push(url);
        }
    }
}

#[cfg(feature = "url-ingestion")]
async fn extract_downloaded_document(
    document: DownloadedDocument,
    config: &ExtractionConfig,
    index: usize,
) -> Result<ExtractionResult> {
    let mime_type = document.mime_type.to_string();
    let mut result = extract_bytes(&document.content, &mime_type, config).await?;
    annotate_source(&mut result, "url_document", &document.url, &document.url, index);
    result
        .metadata
        .additional
        .insert("downloaded_size".into(), serde_json::json!(document.size));
    result
        .metadata
        .additional
        .insert("content_hash".into(), serde_json::json!(document.content_hash));
    if let Some(filename) = document.filename {
        result
            .metadata
            .additional
            .insert("filename".into(), serde_json::json!(filename));
    }
    Ok(result)
}

#[cfg(feature = "url-ingestion")]
fn result_from_scrape_page(scrape: &ScrapeResult, index: usize) -> ExtractionResult {
    let content = scrape
        .markdown
        .as_ref()
        .map(|markdown| markdown.content.clone())
        .filter(|content| !content.is_empty())
        .unwrap_or_else(|| scrape.html.clone());
    let mut result = ExtractionResult {
        content,
        mime_type: Cow::Owned(normalized_content_type(&scrape.content_type)),
        metadata: Metadata::default(),
        uris: links_to_uris(scrape.links.iter().map(|link| (&link.url, &link.text))),
        ..Default::default()
    };
    annotate_source(&mut result, "url_page", &scrape.final_url, &scrape.final_url, index);
    result
        .metadata
        .additional
        .insert("status_code".into(), serde_json::json!(scrape.status_code));
    result
        .metadata
        .additional
        .insert("browser_used".into(), serde_json::json!(scrape.browser_used));
    result
}

#[cfg(feature = "url-ingestion")]
fn result_from_crawl_page(page: &CrawlPageResult, index: usize) -> ExtractionResult {
    let content = page
        .markdown
        .as_ref()
        .map(|markdown| markdown.content.clone())
        .filter(|content| !content.is_empty())
        .unwrap_or_else(|| page.html.clone());
    let mut result = ExtractionResult {
        content,
        mime_type: Cow::Owned(normalized_content_type(&page.content_type)),
        metadata: Metadata::default(),
        uris: links_to_uris(page.links.iter().map(|link| (&link.url, &link.text))),
        ..Default::default()
    };
    annotate_source(&mut result, "url_page", &page.url, &page.normalized_url, index);
    result
        .metadata
        .additional
        .insert("status_code".into(), serde_json::json!(page.status_code));
    result
        .metadata
        .additional
        .insert("crawl_depth".into(), serde_json::json!(page.depth));
    result
        .metadata
        .additional
        .insert("browser_used".into(), serde_json::json!(page.browser_used));
    result
}

#[cfg(feature = "url-ingestion")]
fn normalized_content_type(content_type: &str) -> String {
    let mime = content_type.split(';').next().unwrap_or(content_type).trim();
    if mime.is_empty() {
        "text/html".to_string()
    } else {
        mime.to_string()
    }
}

#[cfg(feature = "url-ingestion")]
fn links_to_uris<'a>(links: impl Iterator<Item = (&'a String, &'a String)>) -> Option<Vec<ExtractedUri>> {
    let uris = links
        .map(|(url, text)| ExtractedUri {
            url: url.clone(),
            label: if text.is_empty() { None } else { Some(text.clone()) },
            page: None,
            kind: UriKind::Hyperlink,
        })
        .collect::<Vec<_>>();
    if uris.is_empty() { None } else { Some(uris) }
}

#[cfg(feature = "url-ingestion")]
fn map_crawl_error(error: crawlberg::CrawlError) -> XbergError {
    XbergError::validation(format!("crawlberg URL extraction failed: {error}"))
}

async fn follow_recursive_document_urls(
    output: &mut ExtractionOutput,
    config: &ExtractionConfig,
    seen: &mut HashSet<String>,
    seed_hosts: &HashSet<String>,
) -> Result<()> {
    if !follow_document_urls(config) {
        return Ok(());
    }

    let max_depth = document_url_depth(config);
    if max_depth == 0 {
        return Ok(());
    }

    let pattern = config
        .url
        .document_url_pattern
        .as_deref()
        .map(regex::Regex::new)
        .transpose()
        .map_err(|error| XbergError::validation(format!("invalid document_url_pattern regex: {error}")))?;

    let mut queue = VecDeque::new();
    enqueue_discovered_urls(output, config, &pattern, seed_hosts, seen, &mut queue, 1);

    while let Some((uri, depth)) = queue.pop_front() {
        let index = output.summary.inputs;
        output.summary.inputs += 1;

        match extract_one(ExtractInput::uri(uri.clone()), config, index).await {
            Ok(mut item_output) => {
                if depth < max_depth {
                    enqueue_discovered_urls(&item_output, config, &pattern, seed_hosts, seen, &mut queue, depth + 1);
                }
                output.summary.remote_urls += item_output.summary.remote_urls;
                output.summary.pages_crawled += item_output.summary.pages_crawled;
                output.summary.documents_downloaded += item_output.summary.documents_downloaded;
                output.results.append(&mut item_output.results);
                output.errors.append(&mut item_output.errors);
            }
            Err(error) => output.errors.push(error_item(index, uri, &error)),
        }
    }

    output.refresh_counts();
    Ok(())
}

fn enqueue_discovered_urls(
    output: &ExtractionOutput,
    config: &ExtractionConfig,
    pattern: &Option<regex::Regex>,
    seed_hosts: &HashSet<String>,
    seen: &mut HashSet<String>,
    queue: &mut VecDeque<(String, u32)>,
    depth: u32,
) {
    let max_total = config.url.max_total_urls.unwrap_or(1_000) as usize;
    if seen.len() >= max_total {
        return;
    }

    for result in &output.results {
        let max_per_result = config.url.max_document_urls_per_result.unwrap_or(100) as usize;
        for url in urls_from_result(result).into_iter().take(max_per_result) {
            if seen.len() >= max_total {
                return;
            }
            if should_follow_discovered_url(&url, config, pattern, seed_hosts) && seen.insert(url.clone()) {
                queue.push_back((url, depth));
            }
        }
    }
}

fn urls_from_result(result: &ExtractionResult) -> Vec<String> {
    let mut urls = Vec::new();
    if let Some(uris) = &result.uris {
        urls.extend(
            uris.iter()
                .filter(|uri| matches!(uri.kind, UriKind::Hyperlink | UriKind::Reference | UriKind::Citation))
                .map(|uri| uri.url.clone()),
        );
    }

    let text_regex = regex::Regex::new(r#"https?://[^\s<>"')\]]+"#).expect("static URL regex is valid");
    urls.extend(text_regex.find_iter(&result.content).map(|matched| {
        matched
            .as_str()
            .trim_end_matches(|ch: char| matches!(ch, '.' | ',' | ';' | ':' | '!' | '?'))
            .to_string()
    }));
    urls
}

fn should_follow_discovered_url(
    url: &str,
    config: &ExtractionConfig,
    pattern: &Option<regex::Regex>,
    seed_hosts: &HashSet<String>,
) -> bool {
    if !(url.starts_with(HTTP_SCHEME) || url.starts_with(HTTPS_SCHEME)) {
        return false;
    }
    if let Some(pattern) = pattern
        && !pattern.is_match(url)
    {
        return false;
    }
    if stay_on_domain(config)
        && !seed_hosts.is_empty()
        && let Some(host) = http_host(url)
    {
        return seed_hosts.contains(&host)
            || (allow_subdomains(config) && seed_hosts.iter().any(|seed| host.ends_with(&format!(".{seed}"))));
    }
    true
}

fn follow_document_urls(config: &ExtractionConfig) -> bool {
    #[cfg(feature = "url-ingestion")]
    {
        config.url.crawl.follow_document_urls
    }
    #[cfg(not(feature = "url-ingestion"))]
    {
        let _ = config;
        false
    }
}

fn document_url_depth(config: &ExtractionConfig) -> u32 {
    #[cfg(feature = "url-ingestion")]
    {
        config
            .url
            .crawl
            .document_url_depth
            .or_else(|| config.url.crawl.max_depth.map(|depth| depth as u32))
            .unwrap_or(1)
    }
    #[cfg(not(feature = "url-ingestion"))]
    {
        let _ = config;
        0
    }
}

fn stay_on_domain(config: &ExtractionConfig) -> bool {
    #[cfg(feature = "url-ingestion")]
    {
        config.url.crawl.stay_on_domain
    }
    #[cfg(not(feature = "url-ingestion"))]
    {
        let _ = config;
        false
    }
}

fn allow_subdomains(config: &ExtractionConfig) -> bool {
    #[cfg(feature = "url-ingestion")]
    {
        config.url.crawl.allow_subdomains
    }
    #[cfg(not(feature = "url-ingestion"))]
    {
        let _ = config;
        false
    }
}

fn initial_seen_urls(inputs: &[ExtractInput]) -> HashSet<String> {
    inputs
        .iter()
        .filter_map(|input| input.uri.clone())
        .filter(|uri| uri.starts_with(HTTP_SCHEME) || uri.starts_with(HTTPS_SCHEME))
        .collect()
}

fn initial_seed_hosts(inputs: &[ExtractInput]) -> HashSet<String> {
    inputs
        .iter()
        .filter_map(|input| input.uri.as_deref().and_then(http_host))
        .collect()
}

fn http_host(uri: &str) -> Option<String> {
    let parsed = url::Url::parse(uri).ok()?;
    match parsed.scheme() {
        "http" | "https" => parsed.host_str().map(|host| host.to_ascii_lowercase()),
        _ => None,
    }
}

fn error_item(index: usize, source: String, error: &XbergError) -> ExtractionErrorItem {
    ExtractionErrorItem {
        index,
        code: error_code(error),
        error_type: error_type(error).to_string(),
        source,
        message: error.to_string(),
    }
}

fn error_code(error: &XbergError) -> u32 {
    match error {
        XbergError::Io(_) => 1001,
        XbergError::Validation { .. } => 1002,
        XbergError::UnsupportedFormat(_) => 1003,
        XbergError::Timeout { .. } => 1004,
        XbergError::Cancelled => 1005,
        XbergError::Security { .. } => 1006,
        _ => 1099,
    }
}

fn error_type(error: &XbergError) -> &'static str {
    match error {
        XbergError::Io(_) => "io",
        XbergError::Validation { .. } => "validation",
        XbergError::UnsupportedFormat(_) => "unsupported_format",
        XbergError::Timeout { .. } => "timeout",
        XbergError::Cancelled => "cancelled",
        XbergError::Security { .. } => "security",
        _ => "other",
    }
}

#[cfg(all(test, feature = "tokio-runtime"))]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn extract_bytes_input_returns_envelope() {
        let config = ExtractionConfig::default();
        let output = extract(ExtractInput::bytes(b"hello".to_vec(), "text/plain", None), &config)
            .await
            .unwrap();

        assert_eq!(output.results.len(), 1);
        assert_eq!(output.summary.inputs, 1);
        assert_eq!(output.summary.results, 1);
        assert_eq!(output.results[0].content.trim(), "hello");
    }

    #[tokio::test]
    async fn extract_local_uri_returns_envelope() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        File::create(&path).unwrap().write_all(b"hello path").unwrap();

        let config = ExtractionConfig::default();
        let output = extract(ExtractInput::uri(path.to_string_lossy()), &config)
            .await
            .unwrap();

        assert_eq!(output.results.len(), 1);
        assert_eq!(output.results[0].content.trim(), "hello path");
    }

    #[tokio::test]
    async fn extract_file_uri_returns_envelope() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        File::create(&path).unwrap().write_all(b"hello file uri").unwrap();

        let config = ExtractionConfig::default();
        let output = extract(ExtractInput::uri(format!("file://{}", path.display())), &config)
            .await
            .unwrap();

        assert_eq!(output.results.len(), 1);
        assert_eq!(output.results[0].content.trim(), "hello file uri");
    }

    #[tokio::test]
    async fn extract_rejects_local_path_when_policy_disallows_it() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        File::create(&path).unwrap().write_all(b"hello local policy").unwrap();

        let mut config = ExtractionConfig::default();
        config.url.allow_local_file_inputs = false;
        let error = extract(ExtractInput::uri(path.to_string_lossy()), &config)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("local filesystem path inputs are disabled"));
    }

    #[tokio::test]
    async fn extract_rejects_non_local_file_uri_host() {
        let config = ExtractionConfig::default();
        let error = extract(ExtractInput::uri("file://evilhost/tmp/doc.txt"), &config)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("unsupported non-local file URI host"));
    }

    #[tokio::test]
    async fn extract_file_uri_accepts_localhost_host() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        File::create(&path)
            .unwrap()
            .write_all(b"hello localhost file uri")
            .unwrap();

        let config = ExtractionConfig::default();
        let output = extract(
            ExtractInput::uri(format!("file://localhost{}", path.display())),
            &config,
        )
        .await
        .unwrap();

        assert_eq!(output.results.len(), 1);
        assert_eq!(output.results[0].content.trim(), "hello localhost file uri");
    }

    #[tokio::test]
    async fn extract_rejects_unsupported_scheme() {
        let config = ExtractionConfig::default();
        let error = extract(ExtractInput::uri("s3://bucket/file.txt"), &config)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("unsupported URI scheme"));
    }

    #[tokio::test]
    async fn extract_batch_collects_mixed_inputs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        File::create(&path).unwrap().write_all(b"hello batch path").unwrap();

        let config = ExtractionConfig::default();
        let output = extract_batch(
            vec![
                ExtractInput::bytes(b"hello batch bytes".to_vec(), "text/plain", None),
                ExtractInput::uri(path.to_string_lossy()),
            ],
            &config,
        )
        .await
        .unwrap();

        assert_eq!(output.results.len(), 2);
        assert_eq!(output.summary.inputs, 2);
        assert!(output.errors.is_empty());
    }

    #[tokio::test]
    async fn extract_batch_collects_unsupported_scheme_error() {
        let config = ExtractionConfig::default();
        let output = extract_batch(
            vec![
                ExtractInput::bytes(b"hello batch bytes".to_vec(), "text/plain", None),
                ExtractInput::uri("s3://bucket/doc.txt"),
            ],
            &config,
        )
        .await
        .unwrap();

        assert_eq!(output.results.len(), 1);
        assert_eq!(output.errors.len(), 1);
        assert_eq!(output.summary.inputs, 2);
        assert_eq!(output.summary.results, 1);
        assert_eq!(output.summary.errors, 1);
        assert_eq!(output.errors[0].index, 1);
        assert_eq!(output.errors[0].code, 1003);
        assert_eq!(output.errors[0].error_type, "unsupported_format");
    }
}

//! #2132: isolate image transport and transcript preservation from provider transcription quality.
#![cfg(all(feature = "liter-llm", feature = "tokio-runtime", not(target_arch = "wasm32")))]

use base64::Engine;
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use xberg::core::config::{LlmConfig, OcrConfig};
use xberg::llm::vlm_ocr::VlmOcrBackend;
use xberg::plugins::OcrBackend;

const GROUP_COUNT: usize = 8;
const SENTENCE: &str = "The committee reviewed the quarterly figures. \
    Each department submitted its report on time. \
    No further action was required from the board.";

fn page_image() -> Vec<u8> {
    let image = image::RgbImage::from_fn(31, 47, |x, y| {
        image::Rgb([(x * 7) as u8, (y * 5) as u8, ((x + y) * 3) as u8])
    });
    let mut encoded = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut encoded, image::ImageFormat::Png)
        .expect("patterned RGB fixture encodes");
    encoded.into_inner()
}

fn ocr_config(server: &MockServer) -> OcrConfig {
    OcrConfig {
        backend: "vlm".to_owned(),
        language: vec!["eng".to_owned()],
        vlm_config: Some(LlmConfig {
            model: "openai/transcript-boundary".to_owned(),
            api_key: Some("test-key".to_owned()),
            base_url: Some(server.uri()),
            load_env: Some(false),
            timeout_secs: Some(10),
            max_retries: Some(0),
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn serve_transcript(server: &MockServer, transcript: &str) {
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-transcript-boundary",
            "object": "chat.completion",
            "created": 0,
            "model": "transcript-boundary",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": transcript},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 100, "completion_tokens": 500, "total_tokens": 600}
        })))
        .expect(1)
        .mount(server)
        .await;
}

async fn assert_image_transported_unchanged(server: &MockServer, image: &[u8]) {
    let requests = server.received_requests().await.expect("request capture available");
    assert_eq!(requests.len(), 1, "exactly one provider call");
    let body: Value = requests[0].body_json().expect("provider request is JSON");
    let parts = body["messages"][0]["content"].as_array().expect("multimodal parts");
    assert_eq!(parts.len(), 2, "prompt and one page image");
    assert_eq!(parts[0]["type"], "text");
    assert_eq!(parts[1]["type"], "image_url");
    let url = parts[1]["image_url"]["url"].as_str().expect("image data URL");
    let payload = url.strip_prefix("data:image/png;base64,").expect("PNG data URL");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .expect("valid base64 image");
    assert_eq!(decoded, image, "provider receives the complete encoded page");
}

#[tokio::test]
async fn should_preserve_every_repeated_group_and_the_exact_provider_image() {
    let server = MockServer::start().await;
    let transcript = format!(
        "Page header\n\n{}\n\nPage footer\n",
        [SENTENCE; GROUP_COUNT].join("\n\n")
    );
    serve_transcript(&server, &transcript).await;
    let image = page_image();
    let document = VlmOcrBackend
        .process_image(&image, &ocr_config(&server))
        .await
        .expect("VLM backend accepts the provider response");
    assert_image_transported_unchanged(&server, &image).await;
    assert_eq!(
        document.content, transcript,
        "adapter preserves the raw transcript byte for byte"
    );
    assert_eq!(document.content.matches(SENTENCE).count(), GROUP_COUNT);
}

#[tokio::test]
async fn should_keep_a_provider_omission_visible_instead_of_inventing_a_repeated_group() {
    let server = MockServer::start().await;
    let transcript = [SENTENCE; GROUP_COUNT - 1].join("\n\n");
    serve_transcript(&server, &transcript).await;
    let document = VlmOcrBackend
        .process_image(&page_image(), &ocr_config(&server))
        .await
        .expect("VLM backend accepts the shorter response");
    assert_eq!(document.content, transcript);
    assert_eq!(document.content.matches(SENTENCE).count(), GROUP_COUNT - 1);
}

//! Gated smoke test: requires a real GLiNER2 PyTorch safetensors snapshot
//! and a real PEFT LoRA adapter on disk. Run explicitly with:
//!
//! ```text
//! GLINER2_CANDLE_MODEL_DIR=/path/to/gliner2-multi-v1 \
//! GLINER2_TEST_ADAPTER_DIR=/path/to/adapter \
//! cargo test -p xberg-gliner --features candle --test candle_smoke -- --ignored
//! ```
#![cfg(feature = "candle")]

#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "requires real GLiNER2 safetensors model + PEFT adapter on disk"]
fn base_model_extracts_entities_and_adapter_changes_output() {
    let Ok(model_dir) = std::env::var("GLINER2_CANDLE_MODEL_DIR") else {
        eprintln!("skipping: GLINER2_CANDLE_MODEL_DIR not set");
        return;
    };
    let Ok(adapter_dir) = std::env::var("GLINER2_TEST_ADAPTER_DIR") else {
        eprintln!("skipping: GLINER2_TEST_ADAPTER_DIR not set");
        return;
    };

    let mut model =
        xberg_gliner::candle::Gliner2Candle::from_local(std::path::Path::new(&model_dir)).expect("load base model");
    let text = "Steve Jobs founded Apple in Cupertino.";
    let labels = ["person", "organization", "location"];

    let base_spans = model
        .extract_ner(text, &labels, 0.3)
        .expect("base extraction must succeed");
    assert!(!base_spans.is_empty(), "base model must find at least one entity");

    model
        .load_adapter("test-adapter", std::path::Path::new(&adapter_dir))
        .expect("adapter load must succeed");
    assert_eq!(model.active_adapter(), Some("test-adapter"));

    let adapter_spans = model
        .extract_ner(text, &labels, 0.3)
        .expect("adapted extraction must succeed");

    assert_ne!(
        base_spans, adapter_spans,
        "loading a real adapter must change inference output"
    );

    model.unload_adapter().expect("unload must succeed");
    assert_eq!(model.active_adapter(), None);
    let unloaded_spans = model.extract_ner(text, &labels, 0.3).expect("post-unload extraction");
    assert_eq!(
        base_spans, unloaded_spans,
        "unload_adapter must restore exact base-model behavior"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "requires real fastino/gliner2-privacy-filter-PII-multi safetensors on disk"]
fn pii_model_loads_from_bytes_and_extracts_entities() {
    let Ok(model_dir) = std::env::var("GLINER2_PII_MODEL_DIR") else {
        eprintln!("skipping: GLINER2_PII_MODEL_DIR not set");
        return;
    };
    let dir = std::path::Path::new(&model_dir);

    let safetensors = std::fs::read(dir.join("model.safetensors")).expect("read model.safetensors");
    let tokenizer_json = std::fs::read(dir.join("tokenizer.json")).expect("read tokenizer.json");
    let encoder_config_json =
        std::fs::read(dir.join("encoder_config").join("config.json")).expect("read encoder_config/config.json");

    // Exercises the exact wasm32-relevant code path (from_bytes, no filesystem
    // reads inside the constructor itself) even though this test runs
    // natively; Candle's tensor ops are portable, and this is the real
    // check the design spec's config-schema inspection could not perform by
    // reading alone: does model.safetensors's tensor naming actually carry
    // the `encoder.` prefix encoder.rs strips via vb.pp("encoder")? ~keep
    let model = xberg_gliner::candle::Gliner2Candle::from_bytes(&safetensors, &tokenizer_json, &encoder_config_json)
        .expect("from_bytes must load the real pinned PII model without a tensor mismatch");

    let text = "Email john.smith@acme.com or call +1 415 555 0199. Signed, Jane Doe.";
    let labels = ["email", "phone_number", "person"];
    let spans = model
        .extract_ner(text, &labels, 0.3)
        .expect("extraction against the real PII model must succeed");
    assert!(
        !spans.is_empty(),
        "the real PII model must find at least one entity in a PII-laden sentence"
    );
}

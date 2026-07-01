//! Gated smoke test: requires a real GLiNER2 PyTorch safetensors snapshot
//! and a real PEFT LoRA adapter on disk. Run explicitly with:
//!
//! ```text
//! GLINER2_CANDLE_MODEL_DIR=/path/to/gliner2-multi-v1 \
//! GLINER2_TEST_ADAPTER_DIR=/path/to/adapter \
//! cargo test -p xberg-gliner-candle --test smoke -- --ignored
//! ```

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

    let mut model = xberg_gliner_candle::Gliner2Candle::from_local(std::path::Path::new(&model_dir))
        .expect("load base model");
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

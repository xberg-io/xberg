use std::collections::BTreeSet;

use super::*;
use crate::candidate_models::{CANDIDATE_MODELS, CandidateFile, CandidateModel, CandidatePrecision, CandidateTask};

fn staged(id: &str, precision: LocalCaptioningPrecision) -> LocalCaptioningModel {
    LocalCaptioningModel::staged(id, precision).unwrap()
}

fn config(model: LocalCaptioningModel) -> LocalCaptioningConfig {
    LocalCaptioningConfig {
        model,
        limits: LocalCaptioningLimits::default(),
        prompt: None,
        cache_directory: None,
    }
}

#[test]
fn staged_captioning_candidates_expose_the_files_the_adapter_loads() {
    let candidates = LocalCaptioningModel::staged_candidates();
    assert_eq!(
        candidates,
        ["smolvlm-256m-instruct", "smolvlm-500m-instruct", "florence-2-base"]
    );
    for (id, expected_format) in [
        ("smolvlm-256m-instruct", LocalCaptioningFormat::SmolVlm),
        ("smolvlm-500m-instruct", LocalCaptioningFormat::SmolVlm),
        ("florence-2-base", LocalCaptioningFormat::Florence2),
    ] {
        let model = staged(id, LocalCaptioningPrecision::Quantized);
        assert_eq!(model.format, expected_format, "{id}");
        assert_eq!(model.repository, "xberg-io/captioning-models");
        for graph in [
            Some(model.vision_encoder_file.as_str()),
            Some(model.embed_tokens_file.as_str()),
            model.text_encoder_file.as_deref(),
            Some(model.decoder_file.as_str()),
        ]
        .into_iter()
        .flatten()
        {
            assert!(graph.starts_with(&format!("{id}/onnx/")), "{graph}");
            assert!(model.files.iter().any(|file| file.path == graph), "{graph}");
        }
        assert_eq!(
            model.text_encoder_file.is_some(),
            expected_format == LocalCaptioningFormat::Florence2
        );
        for companion in ["tokenizer.json", "preprocessor_config.json", "config.json"] {
            assert!(
                model.files.iter().any(|file| file.path.ends_with(companion)),
                "{id} lacks {companion}"
            );
        }
        if model.format == LocalCaptioningFormat::SmolVlm {
            super::policy::validate_config(&config(model)).unwrap();
        } else {
            let error = super::policy::validate_config(&config(model)).unwrap_err().to_string();
            assert!(error.contains("not enabled by this adapter"), "{error}");
        }
    }
}

#[test]
fn precision_selects_the_int8_or_fp32_graph_set_never_a_mixture() {
    for id in ["smolvlm-256m-instruct", "florence-2-base"] {
        let quantized = staged(id, LocalCaptioningPrecision::Quantized);
        assert!(quantized.decoder_file.ends_with("_int8.onnx"), "{id}");
        let fp32 = staged(id, LocalCaptioningPrecision::Fp32);
        assert!(!fp32.decoder_file.ends_with("_int8.onnx"), "{id}");
        let quantized_paths: BTreeSet<&str> = quantized
            .files
            .iter()
            .map(|file| file.path.as_str())
            .filter(|path| path.contains("/onnx/"))
            .collect();
        let fp32_paths: BTreeSet<&str> = fp32
            .files
            .iter()
            .map(|file| file.path.as_str())
            .filter(|path| path.contains("/onnx/"))
            .collect();
        let expected_graphs = if quantized.format == LocalCaptioningFormat::Florence2 {
            4
        } else {
            3
        };
        assert_eq!(quantized_paths.len(), expected_graphs, "{id}");
        assert!(quantized_paths.is_disjoint(&fp32_paths), "{id}");
    }
}

#[test]
fn an_unknown_id_a_wrong_task_and_a_missing_graph_are_rejected_explicitly() {
    let error = LocalCaptioningModel::staged("not-a-candidate", LocalCaptioningPrecision::Quantized)
        .unwrap_err()
        .to_string();
    assert!(error.contains("not a candidate in the model inventory"), "{error}");

    let error = LocalCaptioningModel::staged("nli-minilm2-l6-h768", LocalCaptioningPrecision::Quantized)
        .unwrap_err()
        .to_string();
    assert!(error.contains("not an image-captioning candidate"), "{error}");
}

#[test]
fn a_selection_without_a_complete_graph_family_is_rejected() {
    let mut files: Vec<CandidateFile> = CANDIDATE_MODELS[1].files_for(CandidatePrecision::Quantized).unwrap();
    files.retain(|file| !file.path.ends_with("decoder_model_merged_int8.onnx"));
    let error = crate::candidate_models::captioning_model_with_files(
        &CANDIDATE_MODELS[1],
        CandidatePrecision::Quantized,
        &files,
    )
    .unwrap_err();
    assert!(error.contains("no merged decoder graph"), "{error}");

    let mut mixed: Vec<CandidateFile> = CANDIDATE_MODELS[1].files_for(CandidatePrecision::Quantized).unwrap();
    for file in &mut mixed {
        if file.path.ends_with("vision_encoder_int8.onnx") {
            file.path = file.path.replace("_int8.onnx", ".onnx");
        }
    }
    let error = crate::candidate_models::captioning_model_with_files(
        &CANDIDATE_MODELS[1],
        CandidatePrecision::Quantized,
        &mixed,
    )
    .unwrap_err();
    assert!(error.contains("mixes quantization variants"), "{error}");
}

fn fabricated(model: &CandidateModel, precision: CandidatePrecision) -> Vec<CandidateFile> {
    model
        .files_for(precision)
        .unwrap()
        .into_iter()
        .map(|mut file| {
            file.sha256 = "a".repeat(64);
            file.size_bytes = 1;
            file
        })
        .collect()
}

#[test]
fn descriptor_roles_come_from_the_pinned_graph_names() {
    for (index, expected_format, has_encoder) in [
        (1, LocalCaptioningFormat::SmolVlm, false),
        (3, LocalCaptioningFormat::Florence2, true),
    ] {
        let model = &CANDIDATE_MODELS[index];
        let descriptor = crate::candidate_models::captioning_model_with_files(
            model,
            CandidatePrecision::Quantized,
            &fabricated(model, CandidatePrecision::Quantized),
        )
        .unwrap();
        assert_eq!(descriptor.format, expected_format);
        assert_eq!(descriptor.text_encoder_file.is_some(), has_encoder);
        assert!(descriptor.decoder_file.contains("decoder_model_merged"));
        assert!(descriptor.vision_encoder_file.contains("vision_encoder"));
        assert!(descriptor.embed_tokens_file.contains("embed_tokens"));
    }
}

#[test]
fn captioning_config_rejects_florence_and_loose_bounds() {
    let florence = staged("florence-2-base", LocalCaptioningPrecision::Quantized);
    let error = super::policy::validate_config(&config(florence))
        .unwrap_err()
        .to_string();
    assert!(error.contains("not enabled by this adapter"), "{error}");
    assert!(error.contains("fails on cached decoder steps"), "{error}");

    let smolvlm = staged("smolvlm-256m-instruct", LocalCaptioningPrecision::Quantized);
    let mut limits = config(smolvlm.clone());
    limits.limits.max_new_tokens = 0;
    let error = super::policy::validate_config(&limits).unwrap_err().to_string();
    assert!(error.contains("generated token"), "{error}");

    let mut prompt = config(smolvlm);
    prompt.prompt = Some("describe the chart".to_owned());
    super::policy::validate_config(&prompt).unwrap();
}

#[test]
fn a_corrupt_descriptor_is_rejected_before_any_session_is_built() {
    let model = staged("smolvlm-256m-instruct", LocalCaptioningPrecision::Quantized);

    let mut digest = config(model.clone());
    digest.model.files[0].sha256 = "not-a-digest".to_owned();
    assert!(super::policy::validate_config(&digest).is_err());

    let mut revision = config(model.clone());
    revision.model.revision = "main".to_owned();
    assert!(super::policy::validate_config(&revision).is_err());

    let mut traversal = config(model);
    traversal.model.files[0].path = "../escape".to_owned();
    assert!(super::policy::validate_config(&traversal).is_err());
}

#[test]
fn the_model_info_records_the_selected_quantization_and_experimental_state() {
    for (id, precision) in [
        ("smolvlm-256m-instruct", LocalCaptioningPrecision::Quantized),
        ("florence-2-base", LocalCaptioningPrecision::Fp32),
    ] {
        let model = staged(id, precision);
        let info = super::runtime::model_info(&model).unwrap();
        assert_eq!(info.precision, precision);
        assert!(info.experimental);
        assert_eq!(info.format, model.format);
        assert!(!info.manifest_sha256.is_empty());
        assert!(
            info.graphs
                .iter()
                .any(|graph| { graph.role == LocalCaptioningGraphRole::Decoder && graph.path == model.decoder_file })
        );
    }
}

#[test]
fn every_captioning_candidate_is_registered_under_the_captioning_task() {
    let models = CandidateTask::ImageCaptioning.models();
    assert_eq!(models.len(), 3);
    assert!(models.iter().all(|model| model.task == CandidateTask::ImageCaptioning));
}

/// A 1x1 PNG; the smoke test only checks that the staged pipeline produces a caption.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49,
    0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xdd, 0x8d, 0xb0,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

/// Real inference is opt-in: it requires the pinned int8 artifacts to be staged. The test is
/// ignored by default so CI compiles it without downloading hundreds of megabytes; running it with
/// `--ignored` fails loudly when the artifacts are missing instead of skipping.
#[tokio::test]
#[ignore = "requires staged pinned artifacts"]
async fn staged_smolvlm_produces_a_caption_when_its_artifacts_are_present() {
    let model = LocalCaptioningModel::staged("smolvlm-256m-instruct", LocalCaptioningPrecision::Quantized)
        .expect("the SmolVLM candidate must stay in the inventory");
    let captioner = LocalImageCaptioner::new(config(model)).expect("the SmolVLM config must be valid");
    captioner
        .prepare(&Default::default())
        .await
        .expect("stage the pinned xberg-io/captioning-models artifacts first (LocalImageCaptioner::stage)");
    // A real image can be supplied for manual verification; the default stays offline and tiny.
    let image = match std::env::var_os("XBERG_CAPTION_SMOKE_IMAGE") {
        Some(path) => std::fs::read(path).expect("XBERG_CAPTION_SMOKE_IMAGE should be readable"),
        None => TINY_PNG.to_vec(),
    };
    let result = captioner
        .caption(&image, &Default::default())
        .await
        .expect("staged SmolVLM inference should produce a caption");
    assert!(!result.caption.is_empty(), "caption must not be empty");
    assert!(result.model.experimental);
    assert_eq!(result.model.precision, LocalCaptioningPrecision::Quantized);
}

/// Florence-2 pins and descriptor validation are covered, but the adapter refuses the family
/// explicitly instead of silently substituting SmolVLM.
#[test]
fn staged_florence_is_refused_explicitly() {
    let model = LocalCaptioningModel::staged("florence-2-base", LocalCaptioningPrecision::Quantized)
        .expect("Florence-2 pins must stay in the inventory");
    assert_eq!(model.format, LocalCaptioningFormat::Florence2);
    let error = match LocalImageCaptioner::new(config(model)) {
        Ok(_) => panic!("Florence-2 must be refused by this adapter"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("not enabled by this adapter"), "{error}");
    assert!(error.contains("Only SmolVLM is wired"), "{error}");
}

#[test]
fn smolvlm_prompt_closes_the_image_block_before_the_question() {
    let text = super::generation::smolvlm_prompt_text(1, None, 64).expect("one global frame");
    let opening = text.find("<global-img>").expect("global image tag");
    let closing = text.rfind("<fake_token_around_image>").expect("closing fake image tag");
    assert!(opening < closing, "{text}");
    assert_eq!(text.matches("<image>").count(), 64);
    assert_eq!(text.matches("<fake_token_around_image>").count(), 2, "{text}");
    assert!(
        text[closing..].starts_with("<fake_token_around_image>Can you describe this image?"),
        "{text}"
    );
    assert!(text.ends_with("<end_of_utterance>\nAssistant:"), "{text}");
}

/// Compares the built prompt against the pinned processor's single-frame expansion. It needs the
/// pinned tokenizer from the Hugging Face cache, so it is ignored by default and fails loudly when
/// run without the staged artifact.
#[test]
#[ignore = "requires the pinned SmolVLM tokenizer in the Hugging Face cache"]
fn smolvlm_prompt_matches_the_pinned_processor_token_expansion() {
    let model = LocalCaptioningModel::staged("smolvlm-256m-instruct", LocalCaptioningPrecision::Quantized)
        .expect("the SmolVLM candidate must stay in the inventory");
    let tokenizer_file = model
        .files
        .iter()
        .find(|file| file.path.ends_with("/tokenizer.json"))
        .expect("the descriptor must pin tokenizer.json");
    let client = crate::model_download::hf_client(None).expect("Hugging Face client");
    let path = crate::model_download::hf_cached_revision_with_client(
        &client,
        &model.repository,
        &tokenizer_file.path,
        Some(&model.revision),
    )
    .expect("cache lookup")
    .expect("stage the pinned tokenizer first");
    let tokenizer = tokenizers::Tokenizer::from_file(path).expect("load the pinned tokenizer");

    let text = super::generation::smolvlm_prompt_text(1, None, 64).expect("one global frame");
    let ids = tokenizer
        .encode(text.as_str(), false)
        .expect("prompt")
        .get_ids()
        .to_vec();

    let fake = tokenizer
        .token_to_id("<fake_token_around_image>")
        .expect("fake image token");
    let global = tokenizer.token_to_id("<global-img>").expect("global image token");
    let image = tokenizer.token_to_id("<image>").expect("image token");
    let mut expected = tokenizer
        .encode("<|im_start|>User:", false)
        .expect("prefix")
        .get_ids()
        .to_vec();
    expected.extend([fake, global]);
    expected.extend(std::iter::repeat_n(image, 64));
    expected.push(fake);
    expected.extend(
        tokenizer
            .encode("Can you describe this image?<end_of_utterance>\nAssistant:", false)
            .expect("suffix")
            .get_ids(),
    );
    assert_eq!(ids, expected);
}

#[test]
fn preprocessor_dimensions_are_bounded() {
    use super::LocalCaptioningFormat;
    use super::preprocess::PreprocessConfig;

    let preprocessor = |edge: u32| {
        serde_json::json!({
            "max_image_size": {"longest_edge": edge},
            "image_mean": [0.5, 0.5, 0.5],
            "image_std": [0.5, 0.5, 0.5],
            "rescale_factor": 0.00392156862745098
        })
    };
    let processor = |tokens: u32| serde_json::json!({"image_seq_len": tokens});

    let error = PreprocessConfig::from_json(
        LocalCaptioningFormat::SmolVlm,
        &preprocessor(super::preprocess::MAX_EDGE + 1),
        &processor(64),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("image edge must be in"), "{error}");
    PreprocessConfig::from_json(
        LocalCaptioningFormat::SmolVlm,
        &preprocessor(super::preprocess::MAX_EDGE),
        &processor(64),
    )
    .expect("the ceiling itself is accepted");

    let error = PreprocessConfig::from_json(
        LocalCaptioningFormat::SmolVlm,
        &preprocessor(512),
        &processor(super::preprocess::MAX_IMAGE_SEQ_LEN + 1),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("image token count must be in"), "{error}");
}

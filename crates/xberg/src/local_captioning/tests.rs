use std::collections::BTreeSet;

use super::*;
use crate::candidate_models::{CANDIDATE_MODELS, CandidateFile, CandidateModel, CandidatePrecision, CandidateTask};

fn staged(id: &str, precision: LocalCaptioningPrecision) -> LocalCaptioningModel {
    LocalCaptioningModel::staged(id, precision).unwrap()
}

pub(super) fn config(model: LocalCaptioningModel) -> LocalCaptioningConfig {
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
    assert!(error.contains("pipeline is not implemented"), "{error}");

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
    assert!(error.contains("only SmolVLM is wired"), "{error}");
}

#[test]
fn smolvlm_prompt_closes_the_image_block_before_the_question() {
    let text = super::generation::smolvlm_prompt_text(0, 0, None, 64).expect("one global frame");
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

#[test]
fn split_prompt_matches_the_pinned_processor_row_and_column_expansion() {
    let text = super::generation::smolvlm_prompt_text(2, 2, Some("What is shown?"), 4).expect("2x2 tiles");
    let tiles = "<fake_token_around_image><row_1_col_1><image><image><image><image>\
<fake_token_around_image><row_1_col_2><image><image><image><image>\n\
<fake_token_around_image><row_2_col_1><image><image><image><image>\
<fake_token_around_image><row_2_col_2><image><image><image><image>\n";
    let global = "\n<fake_token_around_image><global-img><image><image><image><image>\
<fake_token_around_image>";
    assert_eq!(
        text,
        format!("<|im_start|>User:{tiles}{global}What is shown?<end_of_utterance>\nAssistant:")
    );
    // The processor emits the tiles in row-major order and the global frame last.
    let rows: Vec<usize> = ["<row_1_col_1>", "<row_1_col_2>", "<row_2_col_1>", "<row_2_col_2>"]
        .iter()
        .map(|tag| text.find(tag).expect("row tag"))
        .collect();
    assert!(rows.windows(2).all(|pair| pair[0] < pair[1]), "{text}");
    assert!(rows[3] < text.find("<global-img>").unwrap(), "{text}");
    assert_eq!(text.matches("<image>").count(), 20);
}

#[test]
fn tile_geometry_follows_the_pinned_processor_resize_ladder() {
    use super::preprocess::tile_geometry;

    // The pinned SmolVLM processor resizes the longest edge to 2048 and tiles at 512, so a 4:3
    // image rounds to 2048x1536 and splits into three rows by four columns.
    let pinned = tile_geometry(800, 600, 512, 2048);
    assert_eq!(pinned.resize, (1536, 2048));
    assert_eq!(pinned.tiled, (1536, 2048));
    assert_eq!((pinned.rows, pinned.cols), (3, 4));
    assert_eq!(pinned.frames(), 13);

    // The same image with a 2x2 geometry (896 longest edge, 448 tiles) needs exactly four tiles.
    let square = tile_geometry(800, 600, 448, 896);
    assert_eq!(square.resize, (672, 896));
    assert_eq!(square.tiled, (896, 896));
    assert_eq!((square.rows, square.cols), (2, 2));
    assert_eq!(square.frames(), 5);

    // A portrait image swaps the roles without losing the aspect ratio.
    let portrait = tile_geometry(600, 800, 448, 896);
    assert_eq!((portrait.rows, portrait.cols), (2, 2));
    assert_eq!(portrait.frames(), 5);
}

#[test]
fn preprocessing_emits_tiles_in_row_major_order_then_the_global_frame() {
    use super::LocalCaptioningFormat;
    use super::preprocess::{PreprocessConfig, preprocess};

    let preprocessor = serde_json::json!({
        "do_image_splitting": true,
        "max_image_size": {"longest_edge": 448},
        "size": {"longest_edge": 896},
        "image_mean": [0.5, 0.5, 0.5],
        "image_std": [0.5, 0.5, 0.5],
        "rescale_factor": 0.00392156862745098,
        "resample": 1
    });
    let config = PreprocessConfig::from_json(
        LocalCaptioningFormat::SmolVlm,
        &preprocessor,
        &serde_json::json!({"image_seq_len": 64}),
    )
    .expect("bounded 2x2 geometry");
    assert_eq!(config.edge, 448);
    assert_eq!(config.resize_edge, 896);
    assert!(config.do_image_splitting);

    // Four quadrants with distinct values prove the row-major tile order, not just the count.
    let mut source = image::RgbImage::from_pixel(800, 600, image::Rgb([0, 0, 0]));
    for y in 0..600 {
        for x in 0..800 {
            let quadrant = u8::from(x >= 400) + 2 * u8::from(y >= 300);
            source.put_pixel(x, y, image::Rgb([quadrant * 60, 0, 0]));
        }
    }
    let captioning = LocalCaptioningConfig {
        model: staged("smolvlm-256m-instruct", LocalCaptioningPrecision::Quantized),
        limits: LocalCaptioningLimits::default(),
        prompt: None,
        cache_directory: None,
    };
    let batch = preprocess(&captioning, LocalCaptioningFormat::SmolVlm, &config, &source).unwrap();
    assert_eq!((batch.rows, batch.cols), (2, 2));
    assert_eq!(batch.num_images, 5);
    assert_eq!(batch.edge, 448);
    assert_eq!(
        batch.pixels.len(),
        5 * 3 * 448usize * 448usize,
        "every frame must be a normalized 448x448 CHW tensor"
    );
    // Tiles keep the row-major quadrant order (top-left, top-right, bottom-left, bottom-right)
    // and the global frame is last.
    let plane = 448usize * 448;
    let tile_center = |tile: usize| batch.pixels[tile * 3 * plane + 224 * 448 + 224];
    let centers: Vec<f32> = (0..4).map(tile_center).collect();
    assert!(
        centers.windows(2).all(|pair| pair[0] < pair[1]),
        "tiles must follow the source quadrants in row-major order: {centers:?}"
    );
    let global = 4 * 3 * plane;
    assert!(batch.pixels[global + 224 * 448 + 224] < batch.pixels[global + 224 * 448 + 672]);
    assert!(
        batch.pixels[global] < batch.pixels[global + 447 * 448],
        "the global frame keeps the whole image, not one tile"
    );
}

#[test]
fn a_processor_config_whose_grid_exceeds_the_tile_budget_is_rejected() {
    use super::LocalCaptioningFormat;
    use super::preprocess::{MAX_TILES, PreprocessConfig};

    let preprocessor = |max_edge: u32, resize_edge: u32| {
        serde_json::json!({
            "do_image_splitting": true,
            "max_image_size": {"longest_edge": max_edge},
            "size": {"longest_edge": resize_edge},
            "image_mean": [0.5, 0.5, 0.5],
            "image_std": [0.5, 0.5, 0.5]
        })
    };
    // 4096 / 256 = 16 per side, far beyond the pinned 4x4 grid.
    let error = PreprocessConfig::from_json(
        LocalCaptioningFormat::SmolVlm,
        &preprocessor(256, 4096),
        &serde_json::json!({"image_seq_len": 64}),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("tiles"), "{error}");
    assert!(MAX_TILES >= 16);

    // A tile grid that fits but whose prompt expansion would exceed the token ceiling.
    let error = PreprocessConfig::from_json(
        LocalCaptioningFormat::SmolVlm,
        &preprocessor(448, 896),
        &serde_json::json!({"image_seq_len": 1024}),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("image tokens"), "{error}");
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

    let text = super::generation::smolvlm_prompt_text(0, 0, None, 64).expect("one global frame");
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

/// Golden `input_ids` from `Idefics3Processor` (transformers 4.57.6, the pinned export's processor
/// class and the pinned tokenizer) for the 2x2 geometry: an 800x600 image with
/// `size.longest_edge = 896` and `max_image_size.longest_edge = 448`, text
/// `<|im_start|>User:<image>Can you describe this image?<end_of_utterance>\nAssistant:`.
/// The pinned 2048/512 geometry always rounds any input up to a 4x4 grid, so a 2x2 expansion is
/// exercised through the processor's supported geometry override rather than a different class.
const GOLDEN_2X2_INPUT_IDS: [i64; 347] = [
    1, 11126, 42, 49189, 49153, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49189, 49154, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 198, 49189, 49159, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49189, 49160, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 1116, 49189, 49152, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190,
    49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49190, 49189, 7306, 346,
    5125, 451, 2443, 47, 49279, 198, 9519, 9531, 42,
];

/// Compares a 2x2 tiled prompt's token ids against the pinned processor. It needs the pinned
/// tokenizer from the Hugging Face cache, so it is ignored by default and fails loudly when run
/// without the staged artifact.
#[test]
#[ignore = "requires the pinned SmolVLM tokenizer in the Hugging Face cache"]
fn smolvlm_tiled_prompt_matches_the_pinned_processor_token_expansion() {
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

    let text = super::generation::smolvlm_prompt_text(2, 2, None, 64).expect("2x2 tiles");
    let ids: Vec<i64> = tokenizer
        .encode(text.as_str(), false)
        .expect("prompt")
        .get_ids()
        .iter()
        .map(|id| i64::from(*id))
        .collect();
    assert_eq!(ids, GOLDEN_2X2_INPUT_IDS);
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

fn artifact_fixture() -> (tempfile::TempDir, LocalCaptioningModel) {
    use sha2::{Digest, Sha256};

    let directory = tempfile::tempdir().unwrap();
    let revision = "a".repeat(40);
    let snapshot = directory
        .path()
        .join("models--owner--captioner/snapshots")
        .join(&revision);
    std::fs::create_dir_all(snapshot.join("smolvlm-256m-instruct/onnx")).unwrap();
    let mut files = Vec::new();
    for (path, bytes) in [
        ("smolvlm-256m-instruct/onnx/vision_encoder.onnx", b"vision".as_slice()),
        ("smolvlm-256m-instruct/onnx/embed_tokens.onnx", b"embed".as_slice()),
        (
            "smolvlm-256m-instruct/onnx/decoder_model_merged.onnx",
            b"decoder".as_slice(),
        ),
        ("smolvlm-256m-instruct/tokenizer.json", b"tokenizer".as_slice()),
        (
            "smolvlm-256m-instruct/tokenizer_config.json",
            b"tokenizer-config".as_slice(),
        ),
        (
            "smolvlm-256m-instruct/preprocessor_config.json",
            b"preprocessor".as_slice(),
        ),
        ("smolvlm-256m-instruct/processor_config.json", b"processor".as_slice()),
        ("smolvlm-256m-instruct/config.json", b"config".as_slice()),
        ("smolvlm-256m-instruct/generation_config.json", b"generation".as_slice()),
    ] {
        std::fs::write(snapshot.join(path), bytes).unwrap();
        files.push(LocalCaptioningFile {
            path: path.to_owned(),
            sha256: hex::encode(Sha256::digest(bytes)),
            size_bytes: bytes.len() as u64,
        });
    }
    let model = LocalCaptioningModel {
        repository: "owner/captioner".to_owned(),
        revision,
        format: LocalCaptioningFormat::SmolVlm,
        vision_encoder_file: "smolvlm-256m-instruct/onnx/vision_encoder.onnx".to_owned(),
        embed_tokens_file: "smolvlm-256m-instruct/onnx/embed_tokens.onnx".to_owned(),
        text_encoder_file: None,
        decoder_file: "smolvlm-256m-instruct/onnx/decoder_model_merged.onnx".to_owned(),
        files,
    };
    (directory, model)
}

#[test]
fn the_offline_artifact_report_distinguishes_staged_missing_and_mismatched_files() {
    let (directory, model) = artifact_fixture();
    let cache = directory.path().to_str().unwrap();

    let report = LocalCaptioningArtifactReport::for_model(&model, Some(cache)).unwrap();
    assert!(report.complete());
    assert_eq!(report.staged(), model.files.len());
    assert_eq!(
        report.total_bytes,
        model.files.iter().map(|file| file.size_bytes).sum::<u64>()
    );
    assert_eq!(report.staged_bytes, report.total_bytes);
    assert_eq!(report.selected_backend, LocalCaptioningBackend::OnnxCpu);
    assert_eq!(report.effective_backend, LocalCaptioningBackend::OnnxCpu);
    let decoder = report
        .artifacts
        .iter()
        .find(|artifact| artifact.path == model.decoder_file)
        .unwrap();
    assert_eq!(decoder.sha256, model.files[2].sha256);
    assert_eq!(decoder.state, LocalCaptioningArtifactState::Staged);

    // A present file of the wrong size is corrupt; a deleted one is missing.
    let decoder_path = directory
        .path()
        .join("models--owner--captioner/snapshots")
        .join(&model.revision)
        .join(&model.decoder_file);
    std::fs::write(&decoder_path, b"decoder-but-longer").unwrap();
    let embed_path = directory
        .path()
        .join("models--owner--captioner/snapshots")
        .join(&model.revision)
        .join(&model.embed_tokens_file);
    std::fs::remove_file(embed_path).unwrap();
    let report = LocalCaptioningArtifactReport::for_model(&model, Some(cache)).unwrap();
    assert!(!report.complete());
    assert_eq!(report.staged(), model.files.len() - 2);
    let states: Vec<_> = report
        .artifacts
        .iter()
        .map(|artifact| (artifact.path.as_str(), artifact.state))
        .collect();
    assert!(states.contains(&(model.decoder_file.as_str(), LocalCaptioningArtifactState::SizeMismatch)));
    assert!(states.contains(&(model.embed_tokens_file.as_str(), LocalCaptioningArtifactState::Missing)));
}

#[test]
fn a_candidate_report_from_an_empty_cache_is_offline_and_fully_missing() {
    let directory = tempfile::tempdir().unwrap();
    let report = LocalCaptioningArtifactReport::for_candidate(
        "smolvlm-256m-instruct",
        LocalCaptioningPrecision::Quantized,
        Some(directory.path().to_str().unwrap()),
    )
    .unwrap();
    assert_eq!(report.candidate.as_deref(), Some("smolvlm-256m-instruct"));
    assert_eq!(report.repository, "xberg-io/captioning-models");
    assert_eq!(report.revision, crate::candidate_models::CAPTIONING_MODELS_REVISION);
    assert!(report.empty());
    assert_eq!(report.staged(), 0);
    assert!(report.total_bytes > 0);
    assert!(report.artifacts.iter().all(|artifact| artifact.sha256.len() == 64));
    // The inventory adds the notice and model-card files, so the report is the full artifact set.
    assert!(
        report
            .artifacts
            .iter()
            .any(|artifact| artifact.path.ends_with("/LICENSE"))
    );
}

#[tokio::test]
async fn preparing_without_staged_artifacts_fails_offline_and_names_the_pinned_file() {
    let directory = tempfile::tempdir().unwrap();
    let mut config = config(staged("smolvlm-256m-instruct", LocalCaptioningPrecision::Quantized));
    config.cache_directory = Some(directory.path().to_str().unwrap().to_owned());
    let captioner = LocalImageCaptioner::new(config).unwrap();
    let error = captioner.prepare(&Default::default()).await.unwrap_err().to_string();
    assert!(error.contains("missing from the pre-staged cache"), "{error}");
    assert!(error.contains("xberg-io/captioning-models@"));

    let image = TINY_PNG.to_vec();
    let error = captioner
        .caption(&image, &Default::default())
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("requires explicit prepare"), "{error}");
}

/// Alternating quantizations of the pinned candidate prepare independent sessions and keep their
/// own identity, and a prepared captioner serves concurrent requests without re-reading artifacts.
#[tokio::test]
#[ignore = "requires staged pinned artifacts"]
async fn alternating_captioning_models_and_concurrent_requests_stay_independent() {
    use std::sync::Arc;

    let quantized = LocalImageCaptioner::new(config(staged(
        "smolvlm-256m-instruct",
        LocalCaptioningPrecision::Quantized,
    )))
    .unwrap();
    let fp32 =
        LocalImageCaptioner::new(config(staged("smolvlm-256m-instruct", LocalCaptioningPrecision::Fp32))).unwrap();
    let quantized_info = quantized.prepare(&Default::default()).await.unwrap();
    let fp32_info = fp32.prepare(&Default::default()).await.unwrap();
    assert_eq!(quantized_info.precision, LocalCaptioningPrecision::Quantized);
    assert_eq!(fp32_info.precision, LocalCaptioningPrecision::Fp32);
    assert_ne!(quantized_info.manifest_sha256, fp32_info.manifest_sha256);

    let captioner = Arc::new(quantized);
    let image = TINY_PNG.to_vec();
    let mut workers = Vec::new();
    for _ in 0..2 {
        let captioner = Arc::clone(&captioner);
        let image = image.clone();
        workers.push(tokio::spawn(async move {
            captioner.caption(&image, &Default::default()).await
        }));
    }
    for worker in workers {
        let result = worker.await.unwrap().expect("concurrent captions");
        assert!(!result.caption.is_empty());
        assert_eq!(result.model.precision, LocalCaptioningPrecision::Quantized);
    }
}

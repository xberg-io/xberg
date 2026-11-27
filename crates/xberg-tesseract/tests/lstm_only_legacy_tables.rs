//! An LSTM-only engine skips the legacy engine's unichar ambiguity tables (`build.rs` patches
//! `init_tesseract_lang_data`). Loading the universal ambigs inserts their correct n-grams, such
//! as `rn` from `m rn 0`, into the engine's character set, so `rn` being a unichar tells that
//! they were loaded. The engine loads the bundled English model, so `required-features` in
//! `Cargo.toml` builds this file only with `bundle-tessdata-eng`. A `dynamic-linking` build uses
//! the host's unpatched Tesseract, so the file is empty there.

#![cfg(not(feature = "dynamic-linking"))]

use xberg_tesseract::TesseractAPI;

const OEM_LSTM_ONLY: i32 = 1;

#[test]
fn lstm_only_engine_does_not_load_the_legacy_ambiguity_tables() {
    let models = tempfile::tempdir().expect("temporary model directory");
    std::fs::write(
        models.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().expect("the bundled English model"),
    )
    .expect("write the English model");
    let api = TesseractAPI::new().expect("create engine");
    api.init_2(models.path().to_str().expect("UTF-8 model path"), "eng", OEM_LSTM_ONLY)
        .expect("initialize an LSTM-only engine");

    assert!(
        api.is_valid_character("m").expect("query a character of the LSTM set"),
        "the check reads the engine's character set"
    );
    assert!(
        !api.is_valid_character("rn").expect("query the ambiguity n-gram"),
        "the universal ambiguity n-gram `rn` must not be in an LSTM-only engine's character set"
    );
}

//! Integration tests for `TesseractAPI` against the host's installed tessdata: OCR on
//! a "hello world" image and a simple table image, resolving the tessdata directory
//! from `TESSDATA_PREFIX` or platform-specific defaults.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
use std::path::{Path, PathBuf};
use xberg_tesseract::{TesseractAPI, WordData};

#[test]
fn word_data_remains_constructible_with_the_original_public_fields() {
    let word = WordData {
        text: "hello".to_string(),
        left: 1,
        top: 2,
        right: 3,
        bottom: 4,
        confidence: 95.0,
        font_attrs: None,
        language: Some("eng".to_string()),
    };

    assert_eq!(word.text, "hello");
}

fn get_default_tessdata_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home_dir)
            .join("Library")
            .join("Application Support")
            .join("xberg-tesseract")
            .join("tessdata")
    } else if cfg!(target_os = "linux") {
        let system_paths = [
            PathBuf::from("/usr/share/tesseract-ocr/5/tessdata"),
            PathBuf::from("/usr/share/tesseract-ocr/tessdata"),
        ];
        for path in &system_paths {
            if path.exists() {
                return path.clone();
            }
        }
        let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home_dir).join(".xberg-tesseract").join("tessdata")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").expect("APPDATA environment variable not set"))
            .join("xberg-tesseract")
            .join("tessdata")
    } else {
        panic!("Unsupported operating system");
    }
}

fn get_tessdata_dir() -> PathBuf {
    match std::env::var("TESSDATA_PREFIX") {
        Ok(dir) => {
            let prefix_path = PathBuf::from(dir);
            let tessdata_path = if prefix_path.ends_with("tessdata") {
                prefix_path
            } else {
                prefix_path.join("tessdata")
            };
            println!("Using TESSDATA_PREFIX directory: {:?}", tessdata_path);
            tessdata_path
        }
        Err(_) => {
            let default_dir = get_default_tessdata_dir();
            println!("TESSDATA_PREFIX not set, using default directory: {:?}", default_dir);
            default_dir
        }
    }
}

fn ensure_eng_traineddata_exists(tessdata_dir: &Path) {
    let eng_traineddata = tessdata_dir.join("eng.traineddata");
    assert!(
        eng_traineddata.exists(),
        "eng.traineddata not found in {}. Set TESSDATA_PREFIX or install English tessdata.",
        tessdata_dir.display()
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn load_test_image(relative: &str) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
    let mut path = repo_root();
    path.push("test_documents");
    path.push(relative);

    let img = image::open(&path)
        .map_err(|e| format!("Failed to open test image {}: {}", path.display(), e))?
        .to_rgb8();
    let (width, height) = img.dimensions();
    Ok((img.into_raw(), width, height))
}

#[test]
fn test_ocr_on_hello_world_image() {
    let tessdata_dir = get_tessdata_dir();
    ensure_eng_traineddata_exists(&tessdata_dir);

    let api = TesseractAPI::new().expect("Failed to create TesseractAPI");
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) =
        load_test_image("images/test_hello_world.png").expect("Failed to load test image");
    api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to set image");

    let text = api.get_utf8_text().expect("Failed to perform OCR");
    assert!(
        text.to_lowercase().contains("hello"),
        "Text does not contain expected word. Found: {}",
        text
    );
}

#[test]
fn test_ocr_on_table_image() {
    let tessdata_dir = get_tessdata_dir();
    ensure_eng_traineddata_exists(&tessdata_dir);

    let api = TesseractAPI::new().expect("Failed to create TesseractAPI");
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");
    api.set_variable("tessedit_pageseg_mode", "1")
        .expect("Failed to set PSM");

    let (image_data, width, height) = load_test_image("images/simple_table.png").expect("Failed to load test image");
    api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to set image");

    let text = api.get_utf8_text().expect("Failed to perform OCR");
    let lowercase = text.to_lowercase();
    assert!(
        lowercase.contains("product") && lowercase.contains("price"),
        "Table text missing expected words. Found: {}",
        text
    );
}

#[test]
fn test_word_symbols_spell_each_accepted_word_inside_its_box() {
    let tessdata_dir = get_tessdata_dir();
    ensure_eng_traineddata_exists(&tessdata_dir);

    let api = TesseractAPI::new().expect("Failed to create TesseractAPI");
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");
    let (image_data, width, height) = load_test_image("images/simple_table.png").expect("Failed to load test image");
    api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to set image");
    api.recognize().expect("Failed to recognize");

    let iterator = api.get_iterator().expect("Failed to get result iterator");
    let all_words = iterator.extract_all_words().expect("Failed to read words").words;
    let words = iterator
        .extract_word_symbols(|_| true)
        .expect("Failed to read word symbols");
    assert!(
        words.len() >= 2,
        "the table image must give several words, got {}",
        words.len()
    );
    assert_eq!(
        words.len(),
        all_words.len(),
        "every word must come back with its symbols"
    );
    for word in &words {
        let spelled: String = word.symbols.iter().map(|symbol| symbol.text.as_str()).collect();
        assert_eq!(spelled, word.text, "the symbols must spell their own word");
        for symbol in &word.symbols {
            assert!(
                symbol.left >= word.left && symbol.right <= word.right,
                "symbol {:?} must lie inside word {:?} ({}..{})",
                symbol.text,
                word.text,
                word.left,
                word.right
            );
        }
    }
    let wanted = words[0].text.clone();
    let expected = words.iter().filter(|word| word.text == wanted).count();
    let filtered = iterator
        .extract_word_symbols(|text| text == wanted)
        .expect("Failed to read filtered word symbols");
    assert!(
        expected < words.len(),
        "the image must give more than one distinct word"
    );
    assert_eq!(
        filtered.len(),
        expected,
        "the filter must keep every word it accepts and no other"
    );
    assert!(
        filtered.iter().all(|word| word.text == wanted),
        "the filter must keep only the words it accepts"
    );
}

#[test]
fn test_invalid_language_code() {
    let tessdata_dir = get_tessdata_dir();
    ensure_eng_traineddata_exists(&tessdata_dir);

    let api = TesseractAPI::new().expect("Failed to create TesseractAPI");

    let result = api.init(tessdata_dir.to_str().unwrap(), "invalid_lang");
    assert!(result.is_err());
}

#[test]
fn test_empty_image_data() {
    let tessdata_dir = get_tessdata_dir();
    ensure_eng_traineddata_exists(&tessdata_dir);

    let api = TesseractAPI::new().expect("Failed to create TesseractAPI");
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let empty_data: Vec<u8> = Vec::new();
    let res = api.set_image(&empty_data, 100, 100, 3, 300);
    assert!(res.is_err());
}

#[test]
fn test_invalid_image_parameters() {
    let tessdata_dir = get_tessdata_dir();
    ensure_eng_traineddata_exists(&tessdata_dir);

    let api = TesseractAPI::new().expect("Failed to create TesseractAPI");
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) =
        load_test_image("images/test_hello_world.png").expect("Failed to load test image");

    let res = api.set_image(&image_data, -1, height as i32, 3, 3 * width as i32);
    assert!(res.is_err());

    let res = api.set_image(&image_data, width as i32, 0, 3, 3 * width as i32);
    assert!(res.is_err());

    let res = api.set_image(&image_data, width as i32, height as i32, 0, 3 * width as i32);
    assert!(res.is_err());

    let res = api.set_image(&image_data, width as i32, height as i32, 3, width as i32);
    assert!(res.is_err());
}

#[test]
fn test_variable_setting() {
    let tessdata_dir = get_tessdata_dir();
    ensure_eng_traineddata_exists(&tessdata_dir);

    let api = TesseractAPI::new().expect("Failed to create TesseractAPI");
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let res = api.set_variable("invalid_variable_name", "1");
    assert!(res.is_err());

    let res = api.set_variable("tessedit_char_whitelist", "");
    assert!(res.is_ok());

    assert!(api.set_variable("tessedit_pageseg_mode", "1").is_ok());
    assert!(api.set_variable("tessedit_ocr_engine_mode", "1").is_ok());
}

#[test]
fn test_multiple_operations() {
    let tessdata_dir = get_tessdata_dir();
    ensure_eng_traineddata_exists(&tessdata_dir);

    let api = TesseractAPI::new().expect("Failed to create TesseractAPI");
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) =
        load_test_image("images/test_hello_world.png").expect("Failed to load test image");

    for _ in 0..3 {
        let res = api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32);
        assert!(res.is_ok());
        let text = api.get_utf8_text().expect("Failed to perform OCR");
        assert!(!text.is_empty());
    }
}

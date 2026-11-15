use super::{Bounds, crop, proposal, read_candidate, source_ink};
use image::RgbImage;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use xberg_paddle_ocr::{InferenceBackend, TextLine, base_net::BaseNet, crnn_net::CrnnNet};

fn checked_file(path: PathBuf, sha256: &str) -> PathBuf {
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert_eq!(hex::encode(Sha256::digest(bytes)), sha256, "{}", path.display());
    path
}

fn recognize(recognizer: &CrnnNet, crop: &RgbImage) -> String {
    let height = 48;
    let width = (crop.width() * height / crop.height()).max(1);
    let resized = image::imageops::resize(crop, width, height, image::imageops::FilterType::Triangle);
    let [line]: [TextLine; 1] = recognizer
        .get_text_lines(&[resized], &Default::default(), 0.0, 1)
        .unwrap()
        .try_into()
        .unwrap_or_else(|lines: Vec<_>| panic!("expected one crop read, got {}", lines.len()));
    line.text.trim().to_owned()
}

fn replay(recognizer: &CrnnNet, cell: &RgbImage, detected: Bounds, stock: &str) -> (Option<String>, Vec<String>) {
    let mut votes = Vec::new();
    let selected = read_candidate(
        cell,
        &(0..cell.height()),
        &(0..cell.width()),
        detected,
        stock,
        &mut |crop| {
            let text = recognize(recognizer, crop);
            votes.push(text.clone());
            Some(text)
        },
    );
    (selected, votes)
}

#[test]
#[ignore = "requires pinned offline Paddle recognizer and the #2503 ruled and interior fixture directories"]
fn interior_span_alias_does_not_change_a_real_decimal_vote() {
    let cache = std::env::var_os("XBERG_2503_PADDLE_CACHE").expect("pinned Paddle model cache");
    let model = Path::new(&cache).join(
        "models--xberg-io--paddleocr-onnx-models/snapshots/bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8/v6/rec/small",
    );
    let network = checked_file(
        model.join("model.onnx"),
        "0cc734c53747ea66968d286f6e6597fc1c91df121964ab4be5085c156fcac892",
    );
    let dictionary = checked_file(
        model.join("dict.txt"),
        "b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d",
    );
    let mut recognizer = CrnnNet::new();
    recognizer
        .init_model_dict_file_on(
            InferenceBackend::Ort,
            network.to_str().unwrap(),
            1,
            dictionary.to_str().unwrap(),
        )
        .unwrap();

    let fixtures = std::env::var_os("XBERG_2503_RULED_FIXTURES").expect("pinned synthetic JPEG directory");
    let source = checked_file(
        Path::new(&fixtures).join("fresh_1_9px.jpg"),
        "7046863cfab54bffd783a1b64ddda73bbe72af32c30b42cf014adca5bcd84f1e",
    );
    let image = image::open(source).unwrap().to_rgb8();
    let positive = crop(&image, (169, 58, 225, 74)).unwrap();
    let (selected, votes) = replay(&recognizer, &positive, (7, 0, 51, 15), "90.302");
    assert_eq!(selected.as_deref(), Some("90,302"), "positive control votes: {votes:?}");

    let variants = std::env::var_os("XBERG_2503_INTERIOR_MUTANT_FIXTURES").expect("variant 99 fixture directory");
    let variants = Path::new(&variants);
    let unaltered = checked_file(
        variants.join("stock-jpg_9_grid_lead-5-3.png"),
        "25cbf5cf8fdac0e77e3ee0c3327bc929013809bcd737a91f5142c62e40c9ae71",
    );
    let mutant = checked_file(
        variants.join("mutant-099.png"),
        "3b15e05c306196f5d38709d6ab84d37ecab40370ce2159b7a2f938093099d075",
    );
    let stock = "948.98";
    let cell = image::open(unaltered).unwrap().to_rgb8();
    let detection = (5, 0, 50, 16);
    assert_eq!(recognize(&recognizer, &crop(&cell, detection).unwrap()), stock);
    assert!(proposal(stock, &source_ink(&cell), detection).is_none());

    let altered = image::open(mutant).unwrap().to_rgb8();
    assert_eq!(altered.dimensions(), cell.dimensions());
    let ink = source_ink(&altered);
    assert_eq!(ink.len(), stock.len());
    assert!(ink.iter().all(|glyph| glyph.is_inside(detection)));
    assert_eq!(proposal(stock, &ink, detection).as_deref(), Some("948,98"));

    let (selected, votes) = replay(&recognizer, &altered, detection, stock);
    assert_eq!(votes.len(), 3, "the mutated cell must reach every crop read");
    assert_eq!(
        votes[0], stock,
        "mutant changed the stock-side read, replay inconclusive: {votes:?}"
    );
    assert_eq!(selected, None, "wrong-position punctuation accepted: {votes:?}");
}

use super::super::{ImageEncoding, ImageSource, PdfDocument, StreamFilters, image_source};
use lopdf::{Object, dictionary};
use sha2::{Digest, Sha256};
use std::io::Write;

enum ParameterForm {
    Dictionary,
    DictionaryArray,
    NullArray,
}

fn flate() -> Object {
    Object::Name(b"FlateDecode".to_vec())
}

fn encode_up_rows(pixels: &[u8], width: usize) -> Vec<u8> {
    let mut tagged = Vec::with_capacity(pixels.len() + pixels.len() / width);
    for (y, row) in pixels.chunks_exact(width).enumerate() {
        tagged.push(2);
        tagged.extend(
            row.iter()
                .enumerate()
                .map(|(x, sample)| sample.wrapping_sub(if y == 0 { 0 } else { pixels[(y - 1) * width + x] })),
        );
    }
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&tagged).unwrap();
    encoder.finish().unwrap()
}

fn predictor_source(filter: Object, form: ParameterForm) -> Option<ImageSource> {
    let mut pdf = lopdf::Document::load_mem(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ocr/hidden_layer_ccitt_2425/flate8-partial.pdf"
    )))
    .unwrap();
    let image = pdf
        .objects
        .values_mut()
        .filter_map(|object| object.as_stream_mut().ok())
        .find(|stream| stream.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Image"))
        .unwrap();
    let width = usize::try_from(image.dict.get(b"Width").unwrap().as_i64().unwrap()).unwrap();
    let pixels = image.decompressed_content().unwrap();
    assert_eq!(width, 1275);
    assert_eq!(pixels.len(), 2_103_750);
    assert_eq!(
        hex::encode(Sha256::digest(&pixels)),
        "96b76deb96146ead1a486b9f6ba115424a6bee7183467eb7347930f20f35d885"
    );
    image.set_content(encode_up_rows(&pixels, width));
    image.dict.set("Filter", filter);
    let params = dictionary! {"Predictor" => 12, "Columns" => 1275, "Colors" => 1, "BitsPerComponent" => 8};
    let parameters = match form {
        ParameterForm::Dictionary => Object::Dictionary(params),
        ParameterForm::DictionaryArray => Object::Array(vec![Object::Dictionary(params)]),
        ParameterForm::NullArray => Object::Array(vec![Object::Null]),
    };
    image.dict.set("DecodeParms", parameters);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    image_source(&PdfDocument::from_bytes(bytes).unwrap(), 0, "Scan")
}

#[test]
fn image_source_keeps_predictor_for_scalar_and_singleton_flate_with_dictionary_parameters() {
    for filter in [flate(), Object::Array(vec![flate()])] {
        let source = predictor_source(filter, ParameterForm::Dictionary).expect("valid predictor image");
        let ImageEncoding::Stream {
            filters: StreamFilters::Flate,
            predictor: Some(params),
        } = &source.encoding
        else {
            panic!("image_source must retain the Flate predictor");
        };
        assert_eq!(
            (
                params.predictor,
                params.columns,
                params.colors,
                params.bits_per_component
            ),
            (12, 1275, 1, 8)
        );
        assert_eq!((source.width, source.height, source.channels), (1275, 1650, 1));
        let samples = source
            .decode_samples(8 * 1024 * 1024)
            .expect("decode actual predictor rows");
        assert_eq!(samples.len(), 2_103_750);
        assert_eq!(
            hex::encode(Sha256::digest(samples)),
            "96b76deb96146ead1a486b9f6ba115424a6bee7183467eb7347930f20f35d885"
        );
    }
}

#[test]
fn image_source_rejects_ascii85_flate_chain_with_dictionary_parameters() {
    let filter = Object::Array(vec![Object::Name(b"ASCII85Decode".to_vec()), flate()]);
    assert_eq!(predictor_source(filter, ParameterForm::Dictionary).map(|_| ()), None);
}

#[test]
fn image_source_rejects_array_decode_parameters_for_singleton_flate() {
    let sources = [ParameterForm::DictionaryArray, ParameterForm::NullArray]
        .map(|form| predictor_source(Object::Array(vec![flate()]), form).map(|_| ()));
    assert_eq!(sources, [None, None]);
}

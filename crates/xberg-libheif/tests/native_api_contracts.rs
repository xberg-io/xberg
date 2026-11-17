use xberg_libheif::{
    Channel, ColorSpace, CompressionFormat, HeifContext, HeifErrorCode, HeifErrorSubCode, Image, LibHeif, RgbChroma,
};

fn rgb_image() -> Image {
    let mut image = Image::new(64, 32, ColorSpace::Rgb(RgbChroma::Rgb)).expect("allocate synthetic RGB image");
    image
        .create_plane(Channel::Interleaved, 64, 32, 8)
        .expect("allocate RGB plane");
    image
        .planes_mut()
        .interleaved
        .expect("interleaved RGB plane")
        .data
        .fill(128);
    image
}

#[test]
#[ignore = "requires an installed libheif AV1 encoder"]
fn encoder_descriptors_find_a_registered_encoder_by_id() {
    let library = LibHeif::new();
    let format = Some(CompressionFormat::Av1);
    let available = library.encoder_descriptors(1, format, None);
    let id = available.first().expect("installed AV1 encoder").id();
    assert!(!id.is_empty());
    let selected = library.encoder_descriptors(1, format, Some(id));
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id(), id);
    assert!(
        library
            .encoder_descriptors(1, format, Some("missing-xberg-test-codec"))
            .is_empty()
    );
}

#[test]
#[ignore = "requires an installed libheif AV1 encoder"]
fn generic_mime_metadata_preserves_its_content_type_and_bytes() {
    let library = LibHeif::new();
    let mut encoder = library
        .encoder_for_format(CompressionFormat::Av1)
        .expect("installed AV1 encoder");
    let mut context = HeifContext::new().expect("allocate HEIF context");
    let handle = context
        .encode_image(&rgb_image(), &mut encoder, None)
        .expect("encode RGB image");
    let content_type = "application/x-xberg-test";
    let payload = b"synthetic metadata";
    context
        .add_generic_metadata(&handle, payload, b"mime", Some(content_type))
        .expect("add MIME metadata");
    let bytes = context.write_to_bytes().expect("serialize encoded image");
    let decoded_context = HeifContext::read_from_bytes(&bytes).expect("read encoded image");
    let decoded_handle = decoded_context.primary_image_handle().expect("primary image");
    let mut ids = [0; 1];
    assert_eq!(decoded_handle.metadata_block_ids(&mut ids, b"mime"), 1);
    assert_eq!(decoded_handle.metadata_content_type(ids[0]), Some(content_type));
    assert_eq!(decoded_handle.metadata(ids[0]).expect("read MIME metadata"), payload);
}

#[test]
#[ignore = "requires an installed libheif AV1 encoder"]
fn a_scaled_thumbnail_returns_an_owned_image_handle() {
    let library = LibHeif::new();
    let mut encoder = library
        .encoder_for_format(CompressionFormat::Av1)
        .expect("installed AV1 encoder");
    let mut context = HeifContext::new().expect("allocate HEIF context");
    let image = rgb_image();
    let master = context
        .encode_image(&image, &mut encoder, None)
        .expect("encode RGB image");
    let thumbnail = context
        .encode_thumbnail(&image, &master, 32, &mut encoder, None)
        .expect("encode a scaled thumbnail")
        .expect("a smaller bounding box requires a thumbnail");
    assert_eq!((thumbnail.width(), thumbnail.height()), (32, 16));

    match context.encode_thumbnail(&image, &master, 128, &mut encoder, None) {
        Ok(None) => {}
        Ok(Some(_)) => panic!("an image already within the bounding box should yield no thumbnail"),
        Err(error) => assert!(
            matches!(
                (error.code, error.sub_code),
                (HeifErrorCode::UsageError, HeifErrorSubCode::InvalidParameterValue)
            ),
            "{error}"
        ),
    }
}

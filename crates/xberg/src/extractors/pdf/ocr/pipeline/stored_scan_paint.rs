use crate::extractors::security::SecurityLimits;
use std::borrow::Cow;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::object::{Object, ObjectRef};

const MAX_STORED_SCAN_PAINT_BYTES: usize = 4096;
const CERTIFIED_STEPS: usize = 5; // Four paint operators and one named XObject.

pub(super) fn certify(doc: &PdfDocument, page: usize, limits: &SecurityLimits) -> Option<(Rect, ObjectRef)> {
    if limits.max_iterations < CERTIFIED_STEPS {
        return None;
    }
    let (frame, content) = super::super::hidden_layer::page_frame_and_contents(doc, page)?;
    if frame.x != 0.0 || frame.y != 0.0 {
        return None;
    }
    let Object::Stream { dict, data } = content else {
        return None;
    };
    let limit = MAX_STORED_SCAN_PAINT_BYTES.min(limits.max_content_size / 4);
    if data.len() > limit || dict.contains_key("DecodeParms") {
        return None;
    }
    let paint = match dict.get("Filter") {
        None => Cow::Borrowed(data.as_ref()),
        Some(filter) if filter.as_name() == Some("FlateDecode") => Cow::Owned(decode_complete_flate(&data, limit)?),
        Some(_) => return None,
    };
    let name = super::super::tall_source::image_name(&paint, frame)?;
    let resources = doc.get_page_resources(page).ok()?;
    let resources = resources.as_dict()?;
    if resources.contains_key("ColorSpace") {
        return None;
    }
    let xobjects = doc.resolve_object(resources.get("XObject")?).ok()?;
    let named = xobjects.as_dict()?.get(&name)?;
    let identity = named.as_reference()?;
    let image = doc.resolve_object(named).ok()?;
    let image = image.as_dict()?;
    (image.get("Subtype")?.as_name()? == "Image" && !image.contains_key("OC")).then_some((frame, identity))
}

fn decode_complete_flate(data: &[u8], limit: usize) -> Option<Vec<u8>> {
    use flate2::{Decompress, FlushDecompress, Status};

    let mut bytes = vec![0; limit.checked_add(1)?];
    let mut decoder = Decompress::new(true);
    let status = decoder.decompress(data, &mut bytes, FlushDecompress::Finish).ok()?;
    let length = usize::try_from(decoder.total_out()).ok()?;
    if status != Status::StreamEnd || decoder.total_in() != data.len() as u64 || length > limit {
        return None;
    }
    bytes.truncate(length);
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn stored_scan_paint_requires_one_complete_bounded_flate_stream() {
        let plain = b"q 32 0 0 32 0 0 cm /Im0 Do Q";
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(plain).unwrap();
        let encoded = encoder.finish().unwrap();
        assert_eq!(
            decode_complete_flate(&encoded, plain.len()).as_deref(),
            Some(plain.as_slice())
        );
        assert!(decode_complete_flate(&encoded, plain.len() - 1).is_none());
        assert!(decode_complete_flate(&encoded[..encoded.len() - 1], plain.len()).is_none());
        let trailing = [encoded.as_slice(), b" q Q"].concat();
        assert!(decode_complete_flate(&trailing, plain.len()).is_none());
    }
}

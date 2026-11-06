//! Exact, budgeted decoding of grayscale-equivalent source samples.

use super::super::hidden_layer::image_predictor;
use super::{MAX_SOURCE_PIXELS, MAX_STRIPS, MAX_TILE_SIDE};
use xberg_native_pdf::decoders::{DecodeParams, decode_predictor};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::object::Object;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum GrayPacking {
    Byte,
    Bilevel,
    NeutralRgb,
    PngPredictor(i64),
    Indexed([u8; 2]),
}

impl GrayPacking {
    pub(super) fn requires_line_certificate(self) -> bool {
        matches!(self, Self::Bilevel | Self::Indexed(_) | Self::PngPredictor(_))
    }
}

pub(super) fn flate_packing(
    dict: &std::collections::HashMap<String, Object>,
    width: u32,
    unfiltered_paint: bool,
) -> Option<GrayPacking> {
    let color = dict.get("ColorSpace")?;
    let depth = dict.get("BitsPerComponent")?.as_integer()?;
    let parameters = dict.get("DecodeParms");
    match (color.as_name(), depth, parameters) {
        (Some("DeviceGray"), 8, None) => Some(GrayPacking::Byte),
        (Some("DeviceGray"), 1, None) => Some(GrayPacking::Bilevel),
        (Some("DeviceRGB"), 8, None) if unfiltered_paint => Some(GrayPacking::NeutralRgb),
        (Some("DeviceGray"), 8, Some(Object::Dictionary(parameters))) => {
            image_predictor(parameters, width, 1).map(|params| GrayPacking::PngPredictor(params.predictor))
        }
        (None, 1, None) => indexed_palette(color).map(GrayPacking::Indexed),
        _ => None,
    }
}

fn indexed_palette(color: &Object) -> Option<[u8; 2]> {
    let [kind, base, hival, Object::String(lookup)] = color.as_array()?.as_slice() else {
        return None;
    };
    if kind.as_name()? != "Indexed" || base.as_name()? != "DeviceGray" || hival.as_integer()? != 1 {
        return None;
    }
    match *lookup.as_slice() {
        [0, 255] => Some([0, 255]),
        [255, 0] => Some([255, 0]),
        _ => None,
    }
}

pub(super) fn decode_source_gray(
    compressed: &[u8],
    width: usize,
    height: usize,
    packing: GrayPacking,
    byte_limit: usize,
) -> Option<(Vec<u8>, usize)> {
    let pixels = width.checked_mul(height)?;
    let retained_bytes = compressed.len().checked_add(pixels)?.checked_add(1)?;
    if retained_bytes > byte_limit {
        return None;
    }
    if packing == GrayPacking::Byte {
        return Some((decode_exact_gray(compressed, pixels)?, retained_bytes));
    }
    let stride = match packing {
        GrayPacking::Byte => return None,
        GrayPacking::Bilevel | GrayPacking::Indexed(_) => width.div_ceil(8),
        GrayPacking::PngPredictor(_) => width.checked_add(1)?,
        GrayPacking::NeutralRgb => width.checked_mul(3)?,
    };
    let packed_size = stride.checked_mul(height)?;
    if stride == 0 || retained_bytes.checked_add(packed_size)? > byte_limit {
        return None;
    }
    let packed = decode_exact_gray(compressed, packed_size)?;
    if let GrayPacking::PngPredictor(predictor) = packing {
        let samples = decode_predictor(
            &packed,
            &DecodeParams {
                predictor,
                columns: width,
                colors: 1,
                bits_per_component: 8,
            },
        )
        .ok()?;
        return (samples.len() == pixels).then_some((samples, retained_bytes));
    }
    let mut samples = Vec::new();
    samples.try_reserve_exact(pixels).ok()?;
    match packing {
        GrayPacking::Byte | GrayPacking::PngPredictor(_) => return None,
        GrayPacking::Bilevel | GrayPacking::Indexed(_) => {
            // PDF starts each packed row on a byte boundary, with the first sample in the high bit. ~keep
            let palette = match packing {
                GrayPacking::Indexed(palette) => palette,
                _ => [0, 255],
            };
            for row in packed.chunks_exact(stride) {
                samples.extend((0..width).map(|x| palette[usize::from((row[x / 8] >> (7 - x % 8)) & 1)]));
            }
        }
        GrayPacking::NeutralRgb => {
            for &[red, green, blue] in packed.as_chunks::<3>().0 {
                if red != green || green != blue {
                    return None;
                }
                samples.push(red);
            }
        }
    }
    Some((samples, retained_bytes))
}

pub(super) fn decode_exact_gray(compressed: &[u8], pixels: usize) -> Option<Vec<u8>> {
    use flate2::{Decompress, FlushDecompress, Status};

    let capacity = pixels.checked_add(1)?;
    let mut samples = Vec::new();
    samples.try_reserve_exact(capacity).ok()?;
    samples.resize(capacity, 0);
    let mut decoder = Decompress::new(true);
    let status = decoder
        .decompress(compressed, &mut samples, FlushDecompress::Finish)
        .ok()?;
    if status != Status::StreamEnd
        || decoder.total_out() != pixels as u64
        || decoder.total_in() != compressed.len() as u64
    {
        return None;
    }
    samples.truncate(pixels);
    Some(samples)
}

pub(super) fn source_frame(
    dict: &std::collections::HashMap<String, Object>,
    frame: Rect,
    rendered_width: Option<u32>,
) -> Option<(u32, u32, f64)> {
    let width = u32::try_from(dict.get("Width")?.as_integer()?).ok()?;
    let height = u32::try_from(dict.get("Height")?.as_integer()?).ok()?;
    let pixels = (width as usize).checked_mul(height as usize)?;
    if rendered_width.is_some_and(|rendered| u64::from(rendered) * 100 >= u64::from(width) * 98)
        || width > MAX_TILE_SIDE
        || height <= MAX_TILE_SIDE
        || height > MAX_TILE_SIDE * MAX_STRIPS as u32
        || pixels > MAX_SOURCE_PIXELS
    {
        return None;
    }
    let dpi = 72.0 * f64::from(width) / f64::from(frame.width);
    let dpi_y = 72.0 * f64::from(height) / f64::from(frame.height);
    if !(150.0..=300.0).contains(&dpi) || (dpi - dpi_y).abs() > 0.01 {
        return None;
    }
    Some((width, height, dpi))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::ZlibEncoder};
    use std::collections::HashMap;
    use std::io::Write;

    fn flate(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn png_predictor_requires_complete_rows_tags_and_live_decode_budget() {
        let input = flate(&[0, 10, 20, 30, 2, 1, 2, 3]);
        let packing = GrayPacking::PngPredictor(12);
        let retained = input.len() + 6 + 1;
        let peak = retained + 8;
        assert_eq!(
            decode_source_gray(&input, 3, 2, packing, peak),
            Some((vec![10, 20, 30, 11, 22, 33], retained))
        );
        assert!(decode_source_gray(&input, 3, 2, packing, peak - 1).is_none());
        for invalid in [
            flate(&[0, 10, 20, 30]),
            flate(&[0, 10, 20, 30, 5, 1, 2, 3]),
            [input, vec![0]].concat(),
        ] {
            assert!(decode_source_gray(&invalid, 3, 2, packing, usize::MAX).is_none());
        }
    }

    #[test]
    fn flate_predictor_parameters_must_describe_the_same_gray_frame() {
        let parameters = HashMap::from([
            ("Predictor".into(), Object::Integer(12)),
            ("Columns".into(), Object::Integer(3)),
            ("Colors".into(), Object::Integer(1)),
            ("BitsPerComponent".into(), Object::Integer(8)),
        ]);
        let mut image = HashMap::from([
            ("ColorSpace".into(), Object::Name("DeviceGray".into())),
            ("BitsPerComponent".into(), Object::Integer(8)),
            ("DecodeParms".into(), Object::Dictionary(parameters.clone())),
        ]);
        assert!(matches!(
            flate_packing(&image, 3, true),
            Some(GrayPacking::PngPredictor(12))
        ));
        let mut defaults = parameters.clone();
        defaults.remove("Colors");
        defaults.remove("BitsPerComponent");
        image.insert("DecodeParms".into(), Object::Dictionary(defaults.clone()));
        assert!(matches!(
            flate_packing(&image, 3, true),
            Some(GrayPacking::PngPredictor(12))
        ));
        defaults.remove("Columns");
        image.insert("DecodeParms".into(), Object::Dictionary(defaults));
        assert!(flate_packing(&image, 3, true).is_none());
        assert!(matches!(
            flate_packing(&image, 1, true),
            Some(GrayPacking::PngPredictor(12))
        ));
        for (key, invalid) in [("Predictor", 2), ("Columns", 4), ("Colors", 3), ("BitsPerComponent", 1)] {
            let mut malformed = parameters.clone();
            malformed.insert(key.into(), Object::Integer(invalid));
            image.insert("DecodeParms".into(), Object::Dictionary(malformed));
            assert!(flate_packing(&image, 3, true).is_none(), "{key}");
        }
        for key in ["Predictor", "Columns", "Colors", "BitsPerComponent", "Unknown"] {
            let mut malformed = parameters.clone();
            malformed.insert(key.into(), Object::Name("invalid".into()));
            image.insert("DecodeParms".into(), Object::Dictionary(malformed));
            assert!(flate_packing(&image, 3, true).is_none(), "{key}");
        }
    }

    #[test]
    fn indexed_rows_use_the_gray_lookup_and_ignore_padding() {
        let input = flate(&[0b0101_1111, 0b1011_1111]);
        let packing = GrayPacking::Indexed([255, 0]);
        let retained = input.len() + 6 + 1;
        let peak = retained + 2;
        assert_eq!(
            decode_source_gray(&input, 3, 2, packing, peak),
            Some((vec![255, 0, 255, 0, 255, 0], retained))
        );
        assert!(decode_source_gray(&input, 3, 2, packing, peak - 1).is_none());
        assert!(decode_source_gray(&flate(&[0, 0, 0]), 3, 2, packing, usize::MAX).is_none());
        let palette = |highest, bytes| {
            Object::Array(vec![
                Object::Name("Indexed".into()),
                Object::Name("DeviceGray".into()),
                Object::Integer(highest),
                Object::String(bytes),
            ])
        };
        assert_eq!(indexed_palette(&palette(1, vec![255, 0])), Some([255, 0]));
        assert_eq!(indexed_palette(&palette(1, vec![0, 255])), Some([0, 255]));
        assert_eq!(indexed_palette(&palette(0, vec![255, 0])), None);
        assert_eq!(indexed_palette(&palette(1, vec![255, 64])), None);
        assert_eq!(indexed_palette(&palette(1, vec![255, 0, 0])), None);
    }
}

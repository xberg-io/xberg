use super::Number;
#[path = "font_profile.rs"]
mod font_profile;
#[path = "source_profile.rs"]
mod source_profile;
use font_profile::*;
use image::{DynamicImage, GrayImage};
use lopdf::{Dictionary, Object as Output, ObjectId, Stream, dictionary};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg_native_pdf::fonts::FontInfo;
use xberg_native_pdf::layout::TextChar;
use xberg_native_pdf::object::{Object, ObjectRef};
use xberg_native_pdf::{PdfDocument, geometry::Rect, rendering};

const MAX_FONT_BYTES: usize = 4 * 1024 * 1024;
const MAX_OBJECTS: usize = 32;
const MAX_DEPTH: usize = 8;
const MAX_VALUES: usize = 4096;
const MAX_FONT_TABLES: usize = 64;
const MAX_GLYPHS: usize = 256;
const MAX_POINTS: usize = 1024;
const MAX_FONT_POINTS: usize = 4096;
const MAX_CONTOURS: usize = 64;
const MAX_INSTRUCTIONS: usize = 4096;
// Covers source/proof font caches, byte maps and cached paths for <=4096 total simple points. ~keep
const NATIVE_FONT_CACHE_BYTES: usize = 4 * 1024 * 1024;
// Covers bounded hinting VM, outline and path scratch beyond native canvas/PNG admission. ~keep
const FONT_WORKING_BYTES: usize = 2 * 1024 * 1024;
const MAX_CMAP_RECORDS: usize = 32;
const MAX_UNICODE_BYTES: usize = 16 * 1024;
const MAX_UNICODE_HEX_TOKENS: usize = 520;
const MAX_DPI: u32 = 600;
const MIN_GLYPH_PIXELS: f32 = 16.0;
const MIN_INK_PIXELS: usize = 16;
const INK_LUMINANCE: u8 = 128;
const DILATION_PIXELS: i64 = 1;
const REQUIRED_INK_COVERAGE: f64 = 1.0;
const MEMORY_FACTOR: usize = 8;
const RENDER_FIXED_BYTES: usize = 256 * 1024;
const RENDER_BYTES_PER_PIXEL: usize = 32;
const GEOMETRY_EPSILON: f32 = 0.001;

// Shape agreement is additional evidence: an unchanged outline with a false ToUnicode still agrees. ~keep
pub(super) struct ShapeProof {
    document: PdfDocument,
    candidates: Vec<Candidate>,
    media: Rect,
    retained_bytes: usize,
    pixels_left: AtomicUsize,
}
struct Candidate {
    text: String,
    bbox: Rect,
    baseline: f32,
    frame: Rect,
    smallest_size: f32,
}

#[derive(Debug, thiserror::Error)]
enum Failure {
    #[error("glyph shape proof declined: {0}")]
    Declined(&'static str),
    #[error("native PDF glyph shape operation failed: {0}")]
    Native(#[from] xberg_native_pdf::Error),
    #[error("encoding isolated glyph content failed: {0}")]
    Content(#[from] lopdf::Error),
    #[error("bounded glyph PDF serialization or stream validation failed: {0}")]
    Io(#[from] std::io::Error),
}
type ProofResult<T> = std::result::Result<T, Failure>;

struct Audited<'a> {
    admitted: source_profile::Admitted,
    fonts: HashMap<String, Object>,
    builder: Builder<'a>,
}
struct Source<'a> {
    chars: Vec<TextChar>,
    fonts: HashMap<String, Object>,
    builder: Builder<'a>,
    working_bytes: usize,
}

#[cfg(test)]
fn preflight(document: &PdfDocument, page: usize, max_bytes: usize) -> crate::Result<bool> {
    Ok(optional(audit_page(document, page, max_bytes))?.is_some())
}
fn audit_page(document: &PdfDocument, page: usize, max_bytes: usize) -> ProofResult<Audited<'_>> {
    page_media(document, page)?;
    let admitted = source_profile::admit(document, page, max_bytes)?;
    let resources = document.get_page_resources(page)?;
    let fonts = page_fonts(document, &resources)?;
    let mut builder = Builder::new(document, max_bytes - admitted.working_bytes);
    builder.audit_fonts(&fonts)?;
    Ok(Audited {
        admitted,
        fonts,
        builder,
    })
}
fn bounded_source(document: &PdfDocument, page: usize, max_bytes: usize) -> ProofResult<Source<'_>> {
    let Audited {
        admitted,
        fonts,
        builder,
    } = audit_page(document, page, max_bytes)?;
    let mut extractor = xberg_native_pdf::extractors::TextExtractor::new();
    extractor.set_document(document);
    // XObjects are audited Image-only. They emit no text; omit them from the private extractor ~keep
    // to avoid source image/resource cache writes and copies. Fonts are supplied only after audit. ~keep
    extractor.set_resources(Object::Dictionary(HashMap::new()));
    for (name, object) in &fonts {
        let font = document.get_or_load_font_for_rendering(object)?;
        extractor.add_font(name.clone(), font.as_ref().clone());
    }
    let chars = extractor.extract_owned(&admitted.bytes)?;
    require(chars.len() <= admitted.max_chars, "native character emission invariant")?;
    Ok(Source {
        chars,
        fonts,
        builder,
        working_bytes: admitted.working_bytes,
    })
}

pub(super) enum Preparation<T> {
    Ready(T),
    UnsupportedBeforeWork,
    UnmeasuredDecline,
}

pub(super) fn prepare_source_accounted(
    document: &PdfDocument,
    page: usize,
    max_bytes: usize,
    max_iterations: usize,
) -> crate::Result<Preparation<(Vec<Number>, ShapeProof)>> {
    if optional(preflight_source(document, page))?.is_none() {
        return Ok(Preparation::UnsupportedBeforeWork);
    }
    Ok(
        match optional(prepare_source_inner(document, page, max_bytes, max_iterations))? {
            Some(proof) => Preparation::Ready(proof),
            None => Preparation::UnmeasuredDecline,
        },
    )
}

fn preflight_source(document: &PdfDocument, page: usize) -> ProofResult<()> {
    page_media(document, page)?;
    source_profile::metadata(document, page)?;
    // The render document already visited these dictionaries. Do not resolve XObjects, font ~keep
    // descriptors or programs here: decoding/copying them makes a decline unmeasured work. ~keep
    let resources = document.get_page_resources(page)?;
    let fonts = resources
        .as_dict()
        .and_then(|dict| dict.get("Font"))
        .ok_or(Failure::Declined("missing page fonts"))?;
    let fonts = document.resolve_object(fonts)?;
    let fonts = fonts.as_dict().ok_or(Failure::Declined("page fonts dictionary"))?;
    require(!fonts.is_empty() && fonts.len() <= MAX_OBJECTS, "page font count")?;
    for (name, font) in fonts {
        require(
            name.len() <= source_profile::MAX_NAME_BYTES,
            "font resource name length",
        )?;
        let font = document.resolve_object(font)?;
        let dict = font.as_dict().ok_or(Failure::Declined("font dictionary"))?;
        require(
            match dict.get("Subtype").and_then(Object::as_name) {
                Some("TrueType") => true,
                Some("Type1") => !dict.contains_key("FontDescriptor") && !dict.contains_key("ToUnicode"),
                _ => false,
            },
            "unsupported page font subtype",
        )?;
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn prepare_source(
    document: &PdfDocument,
    page: usize,
    max_bytes: usize,
    max_iterations: usize,
) -> crate::Result<Option<(Vec<Number>, ShapeProof)>> {
    Ok(
        match prepare_source_accounted(document, page, max_bytes, max_iterations)? {
            Preparation::Ready(proof) => Some(proof),
            Preparation::UnsupportedBeforeWork | Preparation::UnmeasuredDecline => None,
        },
    )
}
fn prepare_source_inner(
    document: &PdfDocument,
    page: usize,
    max_bytes: usize,
    max_iterations: usize,
) -> ProofResult<(Vec<Number>, ShapeProof)> {
    require(max_iterations > 0, "source candidate work budget")?;
    let source = bounded_source(document, page, max_bytes)?;
    let numbers = super::candidates(&source.chars);
    require(numbers.len() <= max_iterations, "source candidate work budget")?;
    let shape = prepare_glyphs(document, page, &numbers, max_bytes, source)?;
    Ok((numbers, shape))
}
#[cfg(test)]
pub(super) fn prepare(
    document: &PdfDocument,
    page: usize,
    numbers: &[Number],
    max_bytes: usize,
) -> crate::Result<Option<ShapeProof>> {
    optional(prepare_inner(document, page, numbers, max_bytes))
}
#[cfg(test)]
fn prepare_inner(document: &PdfDocument, page: usize, numbers: &[Number], max_bytes: usize) -> ProofResult<ShapeProof> {
    let source = bounded_source(document, page, max_bytes)?;
    prepare_glyphs(document, page, numbers, max_bytes, source)
}
fn optional<T>(result: ProofResult<T>) -> crate::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(Failure::Declined(reason)) => {
            tracing::debug!(reason, "Cannot certify native glyph shape");
            Ok(None)
        }
        Err(error) => Err(crate::XbergError::Ocr {
            message: "Unable to obtain bounded native glyph shape evidence".into(),
            source: Some(Box::new(error)),
        }),
    }
}
fn require(condition: bool, reason: &'static str) -> ProofResult<()> {
    condition.then_some(()).ok_or(Failure::Declined(reason))
}
fn prepare_glyphs(
    document: &PdfDocument,
    page: usize,
    numbers: &[Number],
    max_bytes: usize,
    source: Source<'_>,
) -> ProofResult<ShapeProof> {
    let Source {
        chars,
        fonts,
        mut builder,
        working_bytes,
    } = source;
    require(
        !numbers.is_empty() && numbers.len() <= super::MAX_NUMBER_GLYPHS,
        "candidate count",
    )?;
    let media = page_media(document, page)?;
    require(
        numbers.iter().map(|number| number.text.chars().count()).sum::<usize>() <= super::MAX_NUMBER_GLYPHS,
        "total numeric glyph count",
    )?;
    let mut candidates = Vec::new();
    for number in numbers {
        candidates.push(builder.add_candidate(number, &chars, &fonts, media)?);
    }
    let cache_bytes = builder.cache_bytes;
    let (bytes, charged) = builder.finish()?;
    let retained_bytes = charged
        .saturating_add(bytes.capacity())
        .saturating_mul(MEMORY_FACTOR)
        .saturating_add(cache_bytes)
        .saturating_add(working_bytes);
    require(retained_bytes <= max_bytes, "retained font proof budget")?;
    Ok(ShapeProof {
        document: PdfDocument::from_bytes(bytes)?,
        candidates,
        media,
        retained_bytes,
        pixels_left: AtomicUsize::new(super::MAX_CROP_PIXELS as usize),
    })
}
fn page_media(document: &PdfDocument, page: usize) -> ProofResult<Rect> {
    let info = document.get_page_info(page)?;
    let media = info.media_box;
    require(valid_rect(media) && media.x == 0.0 && media.y == 0.0, "page frame")?;
    require(
        info.rotation == 0 && info.crop_box.is_none_or(|crop| crop == media),
        "cropped or rotated page",
    )?;
    Ok(media)
}
fn page_fonts(document: &PdfDocument, resources: &Object) -> ProofResult<HashMap<String, Object>> {
    let dict = resources
        .as_dict()
        .ok_or(Failure::Declined("page resources dictionary"))?;
    if let Some(objects) = dict.get("XObject") {
        let objects = document.resolve_object(objects)?;
        let objects = objects.as_dict().ok_or(Failure::Declined("page XObject dictionary"))?;
        require(objects.len() <= MAX_OBJECTS, "page XObject count")?;
        for object in objects.values() {
            let resolved = document.resolve_object(object)?;
            require(
                resolved
                    .as_dict()
                    .and_then(|dict| dict.get("Subtype"))
                    .and_then(Object::as_name)
                    == Some("Image"),
                "unaudited Form fonts",
            )?;
        }
    }
    let fonts = dict.get("Font");
    let fonts = document.resolve_object(fonts.ok_or(Failure::Declined("missing page fonts"))?)?;
    let Object::Dictionary(fonts) = fonts else {
        return Err(Failure::Declined("page fonts dictionary"));
    };
    require(!fonts.is_empty() && fonts.len() <= MAX_OBJECTS, "page font count")?;
    require(
        fonts.keys().all(|name| name.len() <= source_profile::MAX_NAME_BYTES),
        "font resource name length",
    )?;
    Ok(fonts)
}

struct Builder<'a> {
    graph: FontGraph<'a>,
    loaded: HashMap<ObjectRef, (ObjectId, Arc<FontInfo>)>,
    pages: Vec<Output>,
    pages_id: ObjectId,
    cache_bytes: usize,
    max_bytes: usize,
}

impl<'a> Builder<'a> {
    fn new(source: &'a PdfDocument, max_bytes: usize) -> Self {
        let mut graph = FontGraph::new(source, MAX_FONT_BYTES.min(max_bytes / (2 * MEMORY_FACTOR)));
        let pages_id = graph.output.new_object_id();
        Self {
            graph,
            loaded: HashMap::new(),
            pages: Vec::new(),
            pages_id,
            cache_bytes: 0,
            max_bytes,
        }
    }
    fn audit_fonts(&mut self, fonts: &HashMap<String, Object>) -> ProofResult<()> {
        self.cache_bytes = fonts.len().saturating_mul(NATIVE_FONT_CACHE_BYTES);
        require(
            self.cache_bytes.saturating_add(FONT_WORKING_BYTES) <= self.max_bytes,
            "font cache byte reservation",
        )?;
        let available = self.max_bytes - self.cache_bytes - FONT_WORKING_BYTES;
        self.graph.limit = self.graph.limit.min(available / (2 * MEMORY_FACTOR));
        for object in fonts.values() {
            self.graph.copy(object, 0)?;
            let resolved = self.graph.source.resolve_object(object)?;
            let dict = resolved.as_dict().ok_or(Failure::Declined("font dictionary"))?;
            match dict.get("Subtype").and_then(Object::as_name) {
                Some("TrueType") => audit_truetype(self.graph.source, dict)?,
                Some("Type1") if !dict.contains_key("FontDescriptor") && !dict.contains_key("ToUnicode") => {}
                _ => return Err(Failure::Declined("unsupported page font subtype")),
            }
        }
        Ok(())
    }
    fn add_candidate(
        &mut self,
        number: &Number,
        chars: &[TextChar],
        fonts: &HashMap<String, Object>,
        media: Rect,
    ) -> ProofResult<Candidate> {
        let glyphs = candidate_chars(number, chars)?;
        let frame = glyph_frame(number, &glyphs);
        require(inside(frame, media), "glyph frame outside page")?;
        let mut operations = Vec::new();
        let mut resources = Dictionary::new();
        let mut smallest_size = f32::MAX;
        for glyph in glyphs {
            let (id, info) = self.font(glyph, fonts)?;
            let code = physical_code(&info, glyph)?;
            let name = format!("F{}", id.0);
            resources.set(name.as_bytes(), id);
            operations.extend(glyph_operations(glyph, code, &name, frame));
            smallest_size = smallest_size.min(glyph.font_size);
        }
        self.add_page(frame, resources, operations)?;
        Ok(Candidate {
            text: number.text.clone(),
            bbox: number.bbox,
            baseline: number.baseline,
            frame,
            smallest_size,
        })
    }
    fn font(&mut self, glyph: &TextChar, fonts: &HashMap<String, Object>) -> ProofResult<(ObjectId, Arc<FontInfo>)> {
        let mut matches = HashSet::new();
        for object in fonts.values() {
            let resolved = self.graph.source.resolve_object(object)?;
            if resolved
                .as_dict()
                .and_then(|dict| dict.get("BaseFont"))
                .and_then(Object::as_name)
                == Some(glyph.font_name.as_str())
            {
                matches.insert(object.as_reference().ok_or(Failure::Declined("direct font identity"))?);
            }
        }
        require(matches.len() == 1, "ambiguous font identity")?;
        let reference = *matches
            .iter()
            .next()
            .ok_or(Failure::Declined("missing font identity"))?;
        if let Some(loaded) = self.loaded.get(&reference) {
            return Ok(loaded.clone());
        }
        let object = Object::Reference(reference);
        let resolved = self.graph.source.resolve_object(&object)?;
        let subtype = resolved
            .as_dict()
            .and_then(|dict| dict.get("Subtype"))
            .and_then(Object::as_name);
        require(subtype == Some("TrueType"), "unsupported font subtype")?;
        let copied = self.graph.copy(&object, 0)?;
        let id = copied
            .as_reference()
            .map_err(|_| Failure::Declined("copied font identity"))?;
        let info = self.graph.source.get_or_load_font_for_rendering(&object)?;
        self.loaded.insert(reference, (id, info.clone()));
        Ok((id, info))
    }
    fn add_page(
        &mut self,
        frame: Rect,
        fonts: Dictionary,
        operations: Vec<lopdf::content::Operation>,
    ) -> ProofResult<()> {
        let content = lopdf::content::Content { operations }.encode()?;
        self.graph.charge(content.len())?;
        let contents = self.graph.output.add_object(Stream::new(Dictionary::new(), content));
        let page = self.graph.output.add_object(dictionary! {
            "Type" => "Page", "Parent" => self.pages_id,
            "MediaBox" => vec![0.into(), 0.into(), frame.width.into(), frame.height.into()],
            "Resources" => dictionary! { "Font" => fonts }, "Contents" => contents,
        });
        self.pages.push(page.into());
        Ok(())
    }
    fn finish(mut self) -> ProofResult<(Vec<u8>, usize)> {
        self.graph.output.objects.insert(
            self.pages_id,
            dictionary! {
                "Type" => "Pages", "Count" => self.pages.len() as i64, "Kids" => self.pages,
            }
            .into(),
        );
        let root = self
            .graph
            .output
            .add_object(dictionary! { "Type" => "Catalog", "Pages" => self.pages_id });
        self.graph.output.trailer.set("Root", root);
        let mut sink = BoundedWriter {
            bytes: Vec::new(),
            limit: self.graph.limit,
        };
        self.graph.output.save_to(&mut sink)?;
        Ok((sink.bytes, self.graph.charged))
    }
}
fn candidate_chars<'a>(number: &Number, chars: &'a [TextChar]) -> ProofResult<Vec<&'a TextChar>> {
    require(valid_rect(number.bbox), "candidate geometry")?;
    let glyphs: Vec<_> = chars
        .iter()
        .filter(|glyph| {
            glyph.origin_y == number.baseline && inside(glyph.bbox, number.bbox) && super::is_khmer_digit(glyph.char)
        })
        .collect();
    require(
        !glyphs.is_empty() && glyphs.len() <= super::MAX_NUMBER_GLYPHS,
        "numeric glyph count",
    )?;
    require(
        glyphs.iter().map(|glyph| glyph.char).eq(number.text.chars()),
        "candidate source identity",
    )?;
    require(
        glyphs.iter().map(|glyph| glyph.bbox).reduce(|a, b| a.union(&b)) == Some(number.bbox),
        "candidate source bounds",
    )?;
    for glyph in &glyphs {
        require(
            glyph.font_size.is_finite() && glyph.font_size > 0.0 && glyph.rotation_degrees == 0.0,
            "glyph size or rotation",
        )?;
        let matrix = glyph.matrix.ok_or(Failure::Declined("unknown glyph matrix"))?;
        require(
            matrix == [1.0, 0.0, 0.0, 1.0, glyph.origin_x, glyph.origin_y],
            "transformed glyph",
        )?;
        require(
            glyph.bbox.x == glyph.origin_x && glyph.bbox.y == glyph.origin_y,
            "glyph origin",
        )?;
    }
    Ok(glyphs)
}
fn glyph_frame(number: &Number, glyphs: &[&TextChar]) -> Rect {
    let bottom = glyphs
        .iter()
        .fold(number.bbox.y, |value, glyph| value.min(glyph.origin_y + glyph.descent));
    let top = glyphs.iter().fold(number.bbox.y + number.bbox.height, |value, glyph| {
        value.max(glyph.origin_y + glyph.ascent)
    });
    Rect::new(number.bbox.x, bottom, number.bbox.width, top - bottom)
}
fn physical_code(info: &FontInfo, glyph: &TextChar) -> ProofResult<u8> {
    let claim = glyph.char.to_string();
    let codes: Vec<_> = (0..=255_u16)
        .filter(|code| info.char_to_unicode(u32::from(*code)).as_deref() == Some(claim.as_str()))
        .collect();
    require(codes.len() == 1, "ambiguous physical character code")?;
    let code = codes[0];
    require(
        info.truetype_cmap()
            .and_then(|cmap| cmap.code_to_gid(code))
            .is_some_and(|gid| gid != 0),
        "missing physical glyph",
    )?;
    let advance = info.get_glyph_width(code) * glyph.font_size * info.font_matrix_a;
    require(
        (glyph.advance_width - advance).abs() <= GEOMETRY_EPSILON
            && (glyph.bbox.width - advance).abs() <= GEOMETRY_EPSILON,
        "glyph horizontal scaling",
    )?;
    Ok(code as u8)
}
fn glyph_operations(glyph: &TextChar, code: u8, font: &str, frame: Rect) -> Vec<lopdf::content::Operation> {
    use lopdf::content::Operation;
    let matrix = [1.0, 0.0, 0.0, 1.0, glyph.origin_x - frame.x, glyph.origin_y - frame.y];
    vec![
        Operation::new("BT", vec![]),
        Operation::new("g", vec![0.into()]),
        Operation::new("Tr", vec![0.into()]),
        Operation::new(
            "Tf",
            vec![Output::Name(font.as_bytes().to_vec()), glyph.font_size.into()],
        ),
        Operation::new("Tm", matrix.map(Output::from).to_vec()),
        Operation::new("Tj", vec![Output::String(vec![code], lopdf::StringFormat::Hexadecimal)]),
        Operation::new("ET", vec![]),
    ]
}
fn valid_rect(rect: Rect) -> bool {
    [rect.x, rect.y, rect.width, rect.height]
        .iter()
        .all(|value| value.is_finite())
        && rect.width > 0.0
        && rect.height > 0.0
}
fn inside(inner: Rect, outer: Rect) -> bool {
    valid_rect(inner)
        && valid_rect(outer)
        && inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width <= outer.x + outer.width + GEOMETRY_EPSILON
        && inner.y + inner.height <= outer.y + outer.height + GEOMETRY_EPSILON
}

// Physical byte maps only; every subtable is checked before Native can expand it. ~keep
struct FontGraph<'a> {
    source: &'a PdfDocument,
    output: lopdf::Document,
    references: HashMap<ObjectRef, ObjectId>,
    active: HashSet<ObjectRef>,
    limit: usize,
    charged: usize,
    values: usize,
}

impl<'a> FontGraph<'a> {
    fn new(source: &'a PdfDocument, limit: usize) -> Self {
        Self {
            source,
            output: lopdf::Document::with_version("1.5"),
            references: HashMap::new(),
            active: HashSet::new(),
            limit,
            charged: 0,
            values: 0,
        }
    }
    fn charge(&mut self, bytes: usize) -> ProofResult<()> {
        self.charged = self
            .charged
            .checked_add(bytes)
            .ok_or(Failure::Declined("font byte overflow"))?;
        require(self.charged <= self.limit, "font byte budget")
    }
    fn copy(&mut self, object: &Object, depth: usize) -> ProofResult<Output> {
        require(depth <= MAX_DEPTH, "font dependency depth")?;
        self.values += 1;
        require(self.values <= MAX_VALUES, "font value count")?;
        self.charge(16)?;
        match object {
            Object::Null => Ok(Output::Null),
            Object::Boolean(value) => Ok(Output::Boolean(*value)),
            Object::Integer(value) => Ok(Output::Integer(*value)),
            Object::Real(value) if value.is_finite() && (*value as f32).is_finite() => Ok(Output::Real(*value as f32)),
            Object::Name(value) => {
                require(value.len() <= source_profile::MAX_NAME_BYTES, "font name length")?;
                self.charge(value.len())?;
                Ok(Output::Name(value.as_bytes().to_vec()))
            }
            Object::String(value) => {
                self.charge(value.len())?;
                Ok(Output::String(value.clone(), lopdf::StringFormat::Hexadecimal))
            }
            Object::Array(values) => values
                .iter()
                .map(|value| self.copy(value, depth + 1))
                .collect::<ProofResult<Vec<_>>>()
                .map(Output::Array),
            Object::Dictionary(dict) => self.copy_dict(dict, depth).map(Output::Dictionary),
            Object::Stream { dict, data } => self.copy_stream(dict, data, depth),
            Object::Reference(reference) => self.copy_reference(*reference, depth),
            _ => Err(Failure::Declined("nonfinite font dictionary value")),
        }
    }
    fn copy_dict(&mut self, dict: &HashMap<String, Object>, depth: usize) -> ProofResult<Dictionary> {
        require(dict.len() <= MAX_VALUES, "font dictionary entries")?;
        let mut output = Dictionary::new();
        for (key, value) in dict {
            self.charge(key.len())?;
            output.set(key.as_bytes(), self.copy(value, depth + 1)?);
        }
        Ok(output)
    }
    fn copy_reference(&mut self, reference: ObjectRef, depth: usize) -> ProofResult<Output> {
        require(!self.active.contains(&reference), "cyclic font dependency")?;
        if let Some(id) = self.references.get(&reference) {
            return Ok((*id).into());
        }
        require(self.references.len() < MAX_OBJECTS, "font object count")?;
        let id = self.output.new_object_id();
        self.references.insert(reference, id);
        self.active.insert(reference);
        let resolved = self.source.resolve_object(&Object::Reference(reference))?;
        let copied = self.copy(&resolved, depth + 1)?;
        self.output.objects.insert(id, copied);
        self.active.remove(&reference);
        Ok(id.into())
    }
    fn copy_stream(&mut self, dict: &HashMap<String, Object>, data: &[u8], depth: usize) -> ProofResult<Output> {
        require(
            !["DecodeParms", "F", "FFilter", "FDecodeParms"]
                .iter()
                .any(|key| dict.contains_key(*key)),
            "unsupported font stream parameters",
        )?;
        self.charge(data.len())?;
        match dict.get("Filter") {
            None => {}
            Some(filter) if flate_filter(filter) => {
                let remaining = self.limit.saturating_sub(self.charged);
                let decoder = flate2::read::ZlibDecoder::new(data);
                let decoded = std::io::copy(&mut decoder.take(remaining as u64 + 1), &mut std::io::sink())?;
                self.charge(usize::try_from(decoded).map_err(|_| Failure::Declined("decoded font byte overflow"))?)?;
            }
            _ => return Err(Failure::Declined("unsupported font stream filter")),
        }
        let output = self.copy_dict(dict, depth)?;
        Ok(Output::Stream(Stream::new(output, data.to_vec())))
    }
}
fn flate_filter(filter: &Object) -> bool {
    match filter {
        Object::Name(name) => name == "FlateDecode",
        Object::Array(filters) => matches!(filters.as_slice(), [Object::Name(name)] if name == "FlateDecode"),
        _ => false,
    }
}
struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("glyph proof PDF exceeds its byte budget"));
        }
        self.bytes
            .try_reserve_exact(bytes.len())
            .map_err(std::io::Error::other)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ShapeProof {
    // Charge this once for all candidates, including possible later native cache growth. ~keep
    pub(super) fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
    // Run inside the blocking verifier; remaining_bytes excludes live source but INCLUDES retained_bytes(). ~keep
    pub(super) fn corroborates(
        &self,
        index: usize,
        raster: &DynamicImage,
        media: Rect,
        number: &Number,
        remaining_bytes: usize,
    ) -> crate::Result<bool> {
        Ok(optional(self.compare(index, raster, media, number, remaining_bytes))?.unwrap_or(false))
    }
    fn compare(
        &self,
        index: usize,
        raster: &DynamicImage,
        media: Rect,
        number: &Number,
        remaining_bytes: usize,
    ) -> ProofResult<bool> {
        let candidate = self.candidates.get(index).ok_or(Failure::Declined("candidate index"))?;
        require(
            media == self.media
                && candidate.text == number.text
                && candidate.bbox == number.bbox
                && candidate.baseline == number.baseline,
            "proof candidate binding",
        )?;
        let dpi = raster_dpi(raster, media)?;
        require(
            candidate.smallest_size * dpi as f32 / super::POINTS_PER_INCH >= MIN_GLYPH_PIXELS,
            "glyph raster resolution",
        )?;
        let (x, y, width, height) = crop_bounds(raster, media, candidate.frame, dpi)?;
        let pixels = width as usize * height as usize;
        require(pixels <= super::MAX_CROP_PIXELS as usize, "glyph crop pixel budget")?;
        let required = pixels
            .saturating_mul(RENDER_BYTES_PER_PIXEL)
            .saturating_add(RENDER_FIXED_BYTES);
        let remaining = remaining_bytes
            .checked_sub(self.retained_bytes)
            .ok_or(Failure::Declined("live proof bytes"))?;
        let remaining = remaining
            .checked_sub(FONT_WORKING_BYTES)
            .ok_or(Failure::Declined("glyph font working byte budget"))?;
        require(required <= remaining, "glyph render byte budget")?;
        self.pixels_left
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |left| left.checked_sub(pixels))
            .map_err(|_| Failure::Declined("cumulative glyph pixel budget"))?;
        let painted = painted_mask(raster, (x, y, width, height))?;
        let rendered = rendering::render_png_page_with_budget(&self.document, index, dpi, remaining)?;
        let expected = image::load_from_memory(&rendered.data)
            .map_err(|error| Failure::Io(std::io::Error::other(format!("decoding bounded glyph raster: {error}"))))?
            .into_luma8();
        let phase = crop_phase(media, candidate.frame, dpi, (x, y));
        let coverage = mask_coverage(&expected, &painted, phase);
        tracing::debug!(index, ?coverage, "Native glyph shape ink coverage");
        Ok(coverage
            .is_some_and(|(expected, painted)| expected >= REQUIRED_INK_COVERAGE && painted >= REQUIRED_INK_COVERAGE))
    }
}
fn painted_mask(raster: &DynamicImage, bounds: (u32, u32, u32, u32)) -> ProofResult<GrayImage> {
    let (x, y, width, height) = bounds;
    let crop = raster.crop_imm(x, y, width, height);
    let opaque = crop
        .as_rgba8()
        .is_none_or(|rgba| rgba.pixels().all(|pixel| pixel[3] == u8::MAX));
    require(opaque, "transparent source crop")?;
    Ok(crop.into_luma8())
}
fn raster_dpi(raster: &DynamicImage, media: Rect) -> ProofResult<u32> {
    require(
        matches!(raster.color(), image::ColorType::Rgb8 | image::ColorType::Rgba8),
        "source raster color format",
    )?;
    require(
        valid_rect(media) && raster.width() > 0 && raster.height() > 0,
        "source raster frame",
    )?;
    let mut matching = (1..=MAX_DPI).filter(|dpi| {
        let scale = *dpi as f32 / super::POINTS_PER_INCH;
        let dimensions = (
            (media.width * scale).ceil() as u32,
            (media.height * scale).ceil() as u32,
        );
        dimensions == (raster.width(), raster.height())
    });
    let dpi = matching
        .next()
        .ok_or(Failure::Declined("noncanonical or anisotropic raster DPI"))?;
    require(matching.next().is_none(), "ambiguous raster DPI")?;
    Ok(dpi)
}
fn crop_bounds(raster: &DynamicImage, media: Rect, frame: Rect, dpi: u32) -> ProofResult<(u32, u32, u32, u32)> {
    require(inside(frame, media), "source crop outside page")?;
    let scale = dpi as f32 / super::POINTS_PER_INCH;
    let x = (frame.x * scale).floor() as u32;
    let y = ((media.height - frame.y - frame.height) * scale).floor() as u32;
    let right = ((frame.x + frame.width) * scale).ceil() as u32;
    let bottom = ((media.height - frame.y) * scale).ceil() as u32;
    require(
        right > x && bottom > y && right <= raster.width() && bottom <= raster.height(),
        "source crop extent",
    )?;
    Ok((x, y, right - x, bottom - y))
}
fn crop_phase(media: Rect, frame: Rect, dpi: u32, origin: (u32, u32)) -> (i64, i64) {
    let scale = dpi as f32 / super::POINTS_PER_INCH;
    let x = frame.x * scale - origin.0 as f32;
    let y = (media.height - frame.y - frame.height) * scale - origin.1 as f32;
    (x.round() as i64, y.round() as i64)
}
fn mask_coverage(expected: &GrayImage, painted: &GrayImage, phase: (i64, i64)) -> Option<(f64, f64)> {
    if expected.width().abs_diff(painted.width()) > 1 || expected.height().abs_diff(painted.height()) > 1 {
        return None;
    }
    let (expected_ink, expected_covered) = covered_ink(expected, painted, phase);
    let (painted_ink, painted_covered) = covered_ink(painted, expected, (-phase.0, -phase.1));
    if expected_ink < MIN_INK_PIXELS || painted_ink < MIN_INK_PIXELS || touches_edge(expected) {
        return None;
    }
    Some((
        expected_covered as f64 / expected_ink as f64,
        painted_covered as f64 / painted_ink as f64,
    ))
}
fn covered_ink(source: &GrayImage, other: &GrayImage, phase: (i64, i64)) -> (usize, usize) {
    let mut ink = 0;
    let mut covered = 0;
    for (x, y, pixel) in source.enumerate_pixels() {
        if pixel[0] < INK_LUMINANCE {
            ink += 1;
            covered += usize::from(near_ink(other, i64::from(x) + phase.0, i64::from(y) + phase.1));
        }
    }
    (ink, covered)
}
fn near_ink(image: &GrayImage, x: i64, y: i64) -> bool {
    let rows = (y - DILATION_PIXELS).max(0)..=(y + DILATION_PIXELS).min(i64::from(image.height()) - 1);
    rows.into_iter().any(|row| {
        let columns = (x - DILATION_PIXELS).max(0)..=(x + DILATION_PIXELS).min(i64::from(image.width()) - 1);
        columns
            .into_iter()
            .any(|column| image.get_pixel(column as u32, row as u32)[0] < INK_LUMINANCE)
    })
}
fn touches_edge(image: &GrayImage) -> bool {
    image.enumerate_pixels().any(|(x, y, pixel)| {
        pixel[0] < INK_LUMINANCE && (x == 0 || y == 0 || x + 1 == image.width() || y + 1 == image.height())
    })
}

#[cfg(test)]
#[path = "shape_proof_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "source_profile_tests.rs"]
mod source_tests;

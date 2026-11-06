use crate::PdfDocument;
use crate::content::{Operator, parser::visit_complete_content_stream};
use crate::geometry::Rect;
use crate::object::Object;
use flate2::{Decompress, FlushDecompress, Status};

const MAX_CONTENT_BYTES: usize = 64 * 1024;
const MAX_OPERATORS: usize = 2_048;
const MAX_STACK_DEPTH: usize = 16;
const MAX_FONTS: usize = 32;
const MIN_RULES: usize = 21;

pub(super) fn plain_rule_frame(doc: &PdfDocument, page: usize) -> Option<Rect> {
    let page_object = doc.get_page(page).ok()?;
    let dict = page_object.as_dict()?;
    if doc.is_encrypted()
        || doc.get_page_rotation(page).ok()? != 0
        || ["Annots", "Group", "UserUnit"]
            .iter()
            .any(|key| dict.contains_key(*key))
    {
        return None;
    }
    let media = page_box(dict.get("MediaBox")?)?;
    if let Some(crop) = dict.get("CropBox")
        && page_box(crop)? != media
    {
        return None;
    }
    let Object::Stream { dict, data } = doc.resolve_object(dict.get("Contents")?).ok()? else {
        return None;
    };
    if data.len() > MAX_CONTENT_BYTES || dict.contains_key("DecodeParms") {
        return None;
    }
    let content = doc.cached_page_content(page).ok()?;
    if content.len() > MAX_CONTENT_BYTES || !matches_source_stream(dict.get("Filter"), &data, &content) {
        return None;
    }
    let resources = doc.get_page_resources(page).ok()?;
    let resources = resources.as_dict()?;
    if ["ColorSpace", "ExtGState", "Properties"]
        .iter()
        .any(|key| resources.contains_key(*key))
    {
        return None;
    }
    let fonts = doc.resolve_object(resources.get("Font")?).ok()?;
    let fonts = fonts.as_dict()?;
    if fonts.len() > MAX_FONTS || !fonts.values().all(|font| simple_font(doc, font)) {
        return None;
    }
    audit_operators(&content, |font| fonts.contains_key(font)).then_some(media)
}

fn page_box(object: &Object) -> Option<Rect> {
    let [left, bottom, right, top] = object.as_array()?.as_slice() else {
        return None;
    };
    let coordinate = |value: &Object| match value {
        Object::Integer(n) => Some(*n as f32),
        Object::Real(n) => Some(*n as f32),
        _ => None,
    };
    let (x, y, right, top) = (
        coordinate(left)?,
        coordinate(bottom)?,
        coordinate(right)?,
        coordinate(top)?,
    );
    let bounds = Rect::new(x, y, right - x, top - y);
    (x.is_finite()
        && y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
        && bounds.width > 0.0
        && bounds.height > 0.0)
        .then_some(bounds)
}

fn simple_font(doc: &PdfDocument, font: &Object) -> bool {
    let Ok(font) = doc.resolve_object(font) else {
        return false;
    };
    matches!(
        font.as_dict()
            .and_then(|dict| dict.get("Subtype"))
            .and_then(Object::as_name),
        Some("Type1" | "TrueType")
    )
}

fn matches_source_stream(filter: Option<&Object>, raw: &[u8], content: &[u8]) -> bool {
    match filter {
        None => raw == content,
        Some(filter) if filter.as_name() == Some("FlateDecode") => {
            let mut exact = vec![0; content.len() + 1];
            let mut decoder = Decompress::new(true);
            matches!(
                decoder.decompress(raw, &mut exact, FlushDecompress::Finish),
                Ok(Status::StreamEnd)
            ) && decoder.total_in() == raw.len() as u64
                && decoder.total_out() == content.len() as u64
                && exact[..content.len()] == *content
        }
        _ => false,
    }
}

#[derive(Default)]
struct PaintState {
    saved: usize,
    marked: usize,
    in_text: bool,
    points: usize,
    rules: usize,
}

fn audit_operators(content: &[u8], known_font: impl Fn(&str) -> bool) -> bool {
    let mut state = PaintState::default();
    let complete = visit_complete_content_stream(content, MAX_OPERATORS, |operator| {
        state.path_operator(&operator) || state.text_operator(operator, &known_font)
    });
    matches!(complete, Ok(true))
        && state.saved == 0
        && state.marked == 0
        && !state.in_text
        && state.points == 0
        && state.rules >= MIN_RULES
}

impl PaintState {
    fn path_operator(&mut self, operator: &Operator) -> bool {
        use Operator::*;
        if self.in_text {
            return false;
        }
        match *operator {
            SaveState if self.points == 0 && self.saved < MAX_STACK_DEPTH => self.saved += 1,
            RestoreState if self.points == 0 && self.saved > 0 => self.saved -= 1,
            MoveTo { x, y } if self.points == 0 && x.is_finite() && y.is_finite() => self.points = 1,
            LineTo { x, y } if self.points == 1 && x.is_finite() && y.is_finite() => self.points = 2,
            Stroke if self.points == 2 => {
                self.points = 0;
                self.rules += 1;
            }
            Cm { a, b, c, d, e, f }
                if self.points == 0
                    && b == 0.0
                    && c == 0.0
                    && [a, d, e, f].into_iter().all(f32::is_finite)
                    && a > 0.0
                    && d > 0.0 => {}
            SetLineWidth { width } if width.is_finite() && width > 0.0 => {}
            SetLineCap { cap_style: 0..=2 } | SetLineJoin { join_style: 0..=2 } => {}
            SetStrokeGray { gray } if dark_color([gray; 3]) => {}
            SetStrokeRgb { r, g, b } if dark_color([r, g, b]) => {}
            _ => return false,
        }
        true
    }

    fn text_operator(&mut self, operator: Operator, known_font: impl Fn(&str) -> bool) -> bool {
        use Operator::*;
        match operator {
            BeginMarkedContent { tag } if tag != "OC" && self.marked < MAX_STACK_DEPTH => self.marked += 1,
            BeginMarkedContentDict { tag, properties }
                if tag != "OC"
                    && self.marked < MAX_STACK_DEPTH
                    && properties.as_dict().is_some_and(|dict| !dict.contains_key("OC")) =>
            {
                self.marked += 1;
            }
            EndMarkedContent if self.marked > 0 => self.marked -= 1,
            BeginText if !self.in_text && self.points == 0 => self.in_text = true,
            EndText if self.in_text => self.in_text = false,
            SetFillGray { gray } if dark_color([gray; 3]) => {}
            SetFillRgb { r, g, b } if dark_color([r, g, b]) => {}
            Tf { font, size } if self.in_text && size.is_finite() && size > 0.0 && known_font(&font) => {}
            Td { .. }
            | TD { .. }
            | Tm { .. }
            | TStar
            | Tj { .. }
            | TJ { .. }
            | Tc { .. }
            | Tw { .. }
            | Tz { .. }
            | TL { .. }
            | Ts { .. }
            | Tr { render: 0 }
                if self.in_text => {}
            _ => return false,
        }
        true
    }
}

fn dark_color(channels: [f32; 3]) -> bool {
    channels.iter().all(|channel| (0.0..=1.0).contains(channel)) && channels.iter().any(|channel| *channel <= 0.5)
}

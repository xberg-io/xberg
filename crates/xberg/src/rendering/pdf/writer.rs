//! Laid-out pages to a PDF file.

use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream, StringFormat, dictionary};

use super::document_error;
use super::font::{Face, FontSet};
use super::layout::{Color, Heading, Layout, LinkArea, PAGE_HEIGHT, PAGE_WIDTH, Page, TextRun};
use crate::Result;

/// Horizontal shear of the synthetic italic, about 12 degrees.
const ITALIC_SKEW: f32 = 0.21;
/// Stroke width of the synthetic bold, as a fraction of the font size.
const BOLD_STROKE: f32 = 0.03;
const PRODUCER: &str = "xberg";
/// How close, in points, a run has to start to where the previous one ended to continue it.
const CONTINUATION_TOLERANCE: f32 = 0.01;

pub(super) fn write(layout: &Layout, links: &[String]) -> Result<Vec<u8>> {
    let mut document = Document::with_version("1.7");
    let mut fonts = FontSet::new(&layout.metrics);
    let pages_id = document.new_object_id();
    let resources_id = document.new_object_id();

    let mut page_ids = Vec::with_capacity(layout.pages.len());
    for page in &layout.pages {
        let page_id = document.new_object_id();
        let content = content_stream(page, &mut fonts)?;
        let contents = document.add_object(Stream::new(Dictionary::new(), content));
        let mut dictionary = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => resources_id,
            "Contents" => contents,
        };
        let annotations: Vec<Object> = page
            .links
            .iter()
            .filter_map(|area| link_annotation(area, links, page_id))
            .map(|annotation| document.add_object(annotation).into())
            .collect();
        if !annotations.is_empty() {
            dictionary.set("Annots", annotations);
        }
        document.objects.insert(page_id, Object::Dictionary(dictionary));
        page_ids.push(page_id);
    }

    let font_resources = fonts.into_resources(&mut document)?;
    document.objects.insert(
        resources_id,
        Object::Dictionary(dictionary! { "Font" => font_resources }),
    );
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>(),
            "Count" => page_ids.len() as i64,
            "MediaBox" => vec![0.into(), 0.into(), real(PAGE_WIDTH), real(PAGE_HEIGHT)],
        }),
    );

    let mut catalog = dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    };
    if let Some(outlines) = outline(&mut document, &layout.headings, &page_ids) {
        catalog.set("Outlines", outlines);
    }
    let catalog_id = document.add_object(catalog);
    let info_id = document.add_object(dictionary! { "Producer" => Object::string_literal(PRODUCER) });
    document.trailer.set("Root", catalog_id);
    document.trailer.set("Info", info_id);

    document.compress();
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).map_err(document_error)?;
    Ok(bytes)
}

fn content_stream(page: &Page, fonts: &mut FontSet) -> Result<Vec<u8>> {
    let mut operations = Vec::new();
    for fill in &page.fills {
        let rect = fill.rect;
        operations.push(Operation::new("q", vec![]));
        operations.push(color_operation("rg", fill.color));
        operations.push(Operation::new(
            "re",
            vec![real(rect.x), real(rect.y), real(rect.width), real(rect.height)],
        ));
        operations.push(Operation::new("f", vec![]));
        operations.push(Operation::new("Q", vec![]));
    }
    text_operations(&page.texts, fonts, &mut operations);
    for stroke in &page.strokes {
        operations.push(Operation::new("q", vec![]));
        operations.push(color_operation("RG", stroke.color));
        operations.push(Operation::new("w", vec![real(stroke.width)]));
        operations.push(Operation::new("m", vec![real(stroke.from.0), real(stroke.from.1)]));
        operations.push(Operation::new("l", vec![real(stroke.to.0), real(stroke.to.1)]));
        operations.push(Operation::new("S", vec![]));
        operations.push(Operation::new("Q", vec![]));
    }
    Content { operations }.encode().map_err(document_error)
}

/// The page's text, inside one graphics state of its own, with one text object per stretch
/// of runs that continue each other on a line. A line or a table cell starting elsewhere
/// opens a new one, which readers that join text in stream order treat as a break.
///
/// Font, colour, rendering mode and rise are text and graphics state, which carries over
/// from one run to the next, so each is set only where it changes.
fn text_operations(runs: &[TextRun], fonts: &mut FontSet, operations: &mut Vec<Operation>) {
    let mut state = TextState::default();
    // Where the previous run ended: its baseline and right edge.
    let mut end: Option<(f32, f32)> = None;
    operations.push(Operation::new("q", vec![]));
    for run in runs.iter().filter(|run| !run.text.is_empty()) {
        let continues = end.is_some_and(|(y, x)| y == run.y && (run.x - x).abs() < CONTINUATION_TOLERANCE);
        if !continues {
            if end.is_some() {
                operations.push(Operation::new("ET", vec![]));
            }
            operations.push(Operation::new("BT", vec![]));
        }
        end = Some((run.y, run.x + run.width));
        if state.font != Some((run.face, run.size)) {
            operations.push(Operation::new(
                "Tf",
                vec![Object::Name(run.face.resource_name().into()), real(run.size)],
            ));
            state.font = Some((run.face, run.size));
        }
        if state.color != Some(run.color) {
            operations.push(color_operation("rg", run.color));
            operations.push(color_operation("RG", run.color));
            state.color = Some(run.color);
        }
        let mode = if run.synthetic_bold { FILL_THEN_STROKE } else { FILL };
        if mode == FILL_THEN_STROKE && state.stroke_width != Some(run.size) {
            operations.push(Operation::new("w", vec![real(run.size * BOLD_STROKE)]));
            state.stroke_width = Some(run.size);
        }
        if state.mode != mode {
            operations.push(Operation::new("Tr", vec![mode.into()]));
            state.mode = mode;
        }
        if state.rise != run.rise {
            operations.push(Operation::new("Ts", vec![real(run.rise)]));
            state.rise = run.rise;
        }
        let skew = if run.synthetic_italic { ITALIC_SKEW } else { 0.0 };
        operations.push(Operation::new(
            "Tm",
            vec![1.into(), 0.into(), real(skew), 1.into(), real(run.x), real(run.y)],
        ));
        let format = match run.face {
            Face::Sans => StringFormat::Hexadecimal,
            Face::Courier { .. } => StringFormat::Literal,
        };
        operations.push(Operation::new(
            "Tj",
            vec![Object::String(fonts.encode(run.face, &run.text), format)],
        ));
    }
    if end.is_some() {
        operations.push(Operation::new("ET", vec![]));
    }
    operations.push(Operation::new("Q", vec![]));
}

/// The text rendering modes in use (ISO 32000-1 §9.3.6).
const FILL: i64 = 0;
const FILL_THEN_STROKE: i64 = 2;

/// What the content stream has set so far, starting from the defaults.
#[derive(Default)]
struct TextState {
    font: Option<(Face, f32)>,
    color: Option<Color>,
    /// The font size the stroke width was last set for.
    stroke_width: Option<f32>,
    mode: i64,
    rise: f32,
}

fn link_annotation(area: &LinkArea, links: &[String], page_id: ObjectId) -> Option<Dictionary> {
    let target = links.get(area.target)?;
    let rect = area.rect;
    Some(dictionary! {
        "Type" => "Annot",
        "Subtype" => "Link",
        "P" => page_id,
        "Rect" => vec![real(rect.x), real(rect.y), real(rect.x + rect.width), real(rect.y + rect.height)],
        "Border" => vec![0.into(), 0.into(), 0.into()],
        "A" => dictionary! {
            "S" => "URI",
            "URI" => Object::string_literal(target.as_str()),
        },
    })
}

/// The document outline, one entry per heading, nested by heading level.
fn outline(document: &mut Document, headings: &[Heading], page_ids: &[ObjectId]) -> Option<ObjectId> {
    if headings.is_empty() {
        return None;
    }
    let root = document.new_object_id();
    let ids: Vec<ObjectId> = headings.iter().map(|_| document.new_object_id()).collect();

    // A heading's parent is the nearest earlier heading of a higher level.
    let mut parents: Vec<Option<usize>> = Vec::with_capacity(headings.len());
    let mut top_level = Vec::new();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); headings.len()];
    let mut positions = Vec::with_capacity(headings.len());
    let mut open: Vec<usize> = Vec::new();
    for (index, heading) in headings.iter().enumerate() {
        while open.last().is_some_and(|&last| headings[last].level >= heading.level) {
            open.pop();
        }
        let parent = open.last().copied();
        let siblings = match parent {
            Some(parent) => &mut children[parent],
            None => &mut top_level,
        };
        positions.push(siblings.len());
        siblings.push(index);
        parents.push(parent);
        open.push(index);
    }
    // Every entry is open, so each counts all of its descendants. Children follow their
    // parent, so one backward pass sees every child's count before its parent's.
    let mut descendants = vec![0usize; headings.len()];
    for index in (0..headings.len()).rev() {
        if let Some(parent) = parents[index] {
            descendants[parent] += 1 + descendants[index];
        }
    }

    for (index, heading) in headings.iter().enumerate() {
        let siblings = parents[index].map_or(&top_level, |parent| &children[parent]);
        let position = positions[index];
        let mut entry = dictionary! {
            "Title" => text_string(&heading.title),
            "Parent" => parents[index].map_or(root, |parent| ids[parent]),
            "Dest" => vec![
                Object::Reference(page_ids[heading.page.min(page_ids.len() - 1)]),
                "XYZ".into(),
                Object::Null,
                real(heading.top),
                Object::Null,
            ],
        };
        if position > 0 {
            entry.set("Prev", ids[siblings[position - 1]]);
        }
        if let Some(&next) = siblings.get(position + 1) {
            entry.set("Next", ids[next]);
        }
        if let (Some(&first), Some(&last)) = (children[index].first(), children[index].last()) {
            entry.set("First", ids[first]);
            entry.set("Last", ids[last]);
            entry.set("Count", descendants[index] as i64);
        }
        document.objects.insert(ids[index], Object::Dictionary(entry));
    }

    let mut root_entry = dictionary! {
        "Type" => "Outlines",
        "Count" => headings.len() as i64,
    };
    if let (Some(&first), Some(&last)) = (top_level.first(), top_level.last()) {
        root_entry.set("First", ids[first]);
        root_entry.set("Last", ids[last]);
    }
    document.objects.insert(root, Object::Dictionary(root_entry));
    Some(root)
}

/// A PDF text string: PDFDocEncoding when the text is ASCII, UTF-16BE with a byte order
/// mark otherwise.
fn text_string(text: &str) -> Object {
    if text.is_ascii() {
        return Object::string_literal(text);
    }
    let mut bytes = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    Object::String(bytes, StringFormat::Hexadecimal)
}

fn color_operation(operator: &str, color: Color) -> Operation {
    Operation::new(operator, color.iter().map(|component| real(*component)).collect())
}

/// Two decimal places is a hundredth of a point, well below anything visible, and keeps
/// the content streams short.
fn real(value: f32) -> Object {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 {
        Object::Integer(rounded as i64)
    } else {
        Object::Real(rounded)
    }
}

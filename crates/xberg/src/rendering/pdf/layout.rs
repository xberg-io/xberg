//! Blocks to positioned drawing operations on Letter pages.
//!
//! Coordinates are PDF user space: points, origin at the bottom-left of the page.

use comrak::nodes::TableAlignment;

use super::blocks::{Block, BlockKind, Document, Role, Row, Script, Span, Style, Table};
use super::font::{Face, Metrics};
use crate::Result;

pub(super) const PAGE_WIDTH: f32 = 612.0;
pub(super) const PAGE_HEIGHT: f32 = 792.0;
const MARGIN: f32 = 72.0;
const CONTENT_RIGHT: f32 = PAGE_WIDTH - MARGIN;
const CONTENT_TOP: f32 = PAGE_HEIGHT - MARGIN;

const BODY_SIZE: f32 = 11.0;
const CODE_SIZE: f32 = 10.0;
const HEADING_SIZES: [f32; 6] = [18.0, 16.0, 14.0, 12.0, 11.0, 11.0];
const HEADING_SPACE_BEFORE: [f32; 6] = [18.0, 12.0, 12.0, 12.0, 12.0, 12.0];
const LINE_SPACING: f32 = 1.3;
/// DejaVu Sans' ascent and descent, in ems.
const ASCENT: f32 = 0.928;
const DESCENT: f32 = 0.236;
const SCRIPT_SCALE: f32 = 0.7;
const SUPERSCRIPT_RISE: f32 = 0.33;
const SUBSCRIPT_DROP: f32 = 0.15;

const SPACE_AFTER: f32 = 6.0;
const LIST_SPACE_AFTER: f32 = 3.0;
/// The DOCX writer's indent step, half an inch.
const INDENT_STEP: f32 = 36.0;
/// Deeper nesting is laid out at this depth, which still leaves two inches for text.
const MAX_DEPTH: usize = 9;
const TAB_WIDTH: usize = 4;
const CODE_PADDING: f32 = 4.0;
const CELL_PADDING_X: f32 = 5.0;
const CELL_PADDING_Y: f32 = 3.0;
const RULE_GAP: f32 = 6.0;
const QUOTE_RAIL_OFFSET: f32 = 12.0;
const QUOTE_RAIL_WIDTH: f32 = 2.0;
const GRID_WIDTH: f32 = 0.5;
/// Space a heading keeps below it on its page, so it is not left alone at the bottom.
const HEADING_KEEP: f32 = 2.0 * BODY_SIZE * LINE_SPACING;

pub(super) const BLACK: Color = [0.0, 0.0, 0.0];
const LINK_COLOR: Color = [0.02, 0.39, 0.76];
const HIGHLIGHT_COLOR: Color = [1.0, 0.95, 0.4];
const CODE_BACKGROUND: Color = [0.95, 0.95, 0.95];
const HEADER_BACKGROUND: Color = [0.93, 0.93, 0.93];
const GRID_COLOR: Color = [0.55, 0.55, 0.55];
const RULE_COLOR: Color = [0.6, 0.6, 0.6];
const QUOTE_RAIL_COLOR: Color = [0.8, 0.8, 0.8];

pub(super) type Color = [f32; 3];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Rect {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

#[derive(Debug, PartialEq)]
pub(super) struct TextRun {
    pub(super) x: f32,
    /// The baseline of the line the run is on.
    pub(super) y: f32,
    /// How far a superscript or subscript sits above (below, negative) the baseline. It is
    /// drawn as text rise, so readers still see the run as part of its line.
    pub(super) rise: f32,
    pub(super) size: f32,
    pub(super) width: f32,
    pub(super) face: Face,
    /// DejaVu Sans is embedded in one weight and style; the others are drawn from it.
    pub(super) synthetic_bold: bool,
    pub(super) synthetic_italic: bool,
    pub(super) color: Color,
    pub(super) text: String,
}

#[derive(Debug, PartialEq)]
pub(super) struct Fill {
    pub(super) rect: Rect,
    pub(super) color: Color,
}

#[derive(Debug, PartialEq)]
pub(super) struct Stroke {
    pub(super) from: (f32, f32),
    pub(super) to: (f32, f32),
    pub(super) width: f32,
    pub(super) color: Color,
}

#[derive(Debug, PartialEq)]
pub(super) struct LinkArea {
    pub(super) rect: Rect,
    /// Index into [`Document::links`].
    pub(super) target: usize,
}

/// A page's drawing, kept apart by kind. Backgrounds are drawn first and rules last, so
/// the text runs form one uninterrupted sequence in reading order: readers that group text
/// by content-stream order otherwise split a line off at a background drawn before it.
#[derive(Debug, Default)]
pub(super) struct Page {
    pub(super) fills: Vec<Fill>,
    pub(super) texts: Vec<TextRun>,
    pub(super) strokes: Vec<Stroke>,
    pub(super) links: Vec<LinkArea>,
}

/// A heading, for the document outline.
#[derive(Debug)]
pub(super) struct Heading {
    pub(super) level: u8,
    pub(super) title: String,
    pub(super) page: usize,
    pub(super) top: f32,
}

pub(super) struct Layout {
    pub(super) pages: Vec<Page>,
    pub(super) headings: Vec<Heading>,
    pub(super) metrics: Metrics,
}

pub(super) fn lay_out(document: &Document) -> Result<Layout> {
    let mut layouter = Layouter {
        metrics: Metrics::load()?,
        pages: vec![Page::default()],
        headings: Vec::new(),
        y: CONTENT_TOP,
        page_empty: true,
        pending_space: 0.0,
        previous_quotes: Vec::new(),
    };
    for block in &document.blocks {
        layouter.block(block);
    }
    Ok(Layout {
        pages: layouter.pages,
        headings: layouter.headings,
        metrics: layouter.metrics,
    })
}

/// How a run of text is set before its own style applies.
#[derive(Clone, Copy)]
struct TextBase {
    size: f32,
    bold: bool,
    italic: bool,
}

/// A stretch of one line in one style and face.
#[derive(Debug)]
struct Piece {
    text: String,
    style: Style,
    face: Face,
    size: f32,
    width: f32,
}

#[derive(Debug, Default)]
struct Line {
    pieces: Vec<Piece>,
    width: f32,
}

impl Line {
    fn push(&mut self, piece: Piece) {
        self.width += piece.width;
        if let Some(last) = self.pieces.last_mut()
            && last.style == piece.style
            && last.face == piece.face
            && last.size == piece.size
        {
            last.text.push_str(&piece.text);
            last.width += piece.width;
            return;
        }
        self.pieces.push(piece);
    }

    fn extend(&mut self, pieces: Vec<Piece>) {
        for piece in pieces {
            self.push(piece);
        }
    }
}

struct Layouter {
    metrics: Metrics,
    pages: Vec<Page>,
    headings: Vec<Heading>,
    /// Top of the free space on the current page.
    y: f32,
    page_empty: bool,
    /// Space still owed below the previous block.
    pending_space: f32,
    previous_quotes: Vec<usize>,
}

impl Layouter {
    fn block(&mut self, block: &Block) {
        let depth = block.depth.min(MAX_DEPTH);
        let space_before = match &block.kind {
            BlockKind::Text {
                role: Role::Heading(level),
                ..
            } => HEADING_SPACE_BEFORE[usize::from(level - 1)],
            _ => 0.0,
        };
        self.gap(self.pending_space.max(space_before), &block.quotes);
        self.previous_quotes.clone_from(&block.quotes);

        match &block.kind {
            BlockKind::Text {
                role,
                spans,
                marker,
                in_list,
            } => {
                self.text_block(*role, spans, marker.as_deref(), depth, &block.quotes);
                self.pending_space = if *in_list && *role == Role::Body {
                    LIST_SPACE_AFTER
                } else {
                    SPACE_AFTER
                };
            }
            BlockKind::Code { lines } => {
                self.code_block(lines, depth, &block.quotes);
                self.pending_space = SPACE_AFTER;
            }
            BlockKind::Rule => {
                self.rule(depth, &block.quotes);
                self.pending_space = 0.0;
            }
            BlockKind::Table(table) => {
                self.table(table, depth, &block.quotes);
                self.pending_space = SPACE_AFTER;
            }
        }
    }

    fn page(&mut self) -> &mut Page {
        self.pages.last_mut().expect("the layout always holds a page")
    }

    fn new_page(&mut self) {
        self.pages.push(Page::default());
        self.y = CONTENT_TOP;
        self.page_empty = true;
    }

    fn fits(&self, height: f32) -> bool {
        self.y - height >= MARGIN
    }

    /// Claim `height` of vertical space, on a new page when this one cannot hold it, and
    /// return the top of the claimed space.
    fn reserve(&mut self, height: f32) -> f32 {
        if !self.fits(height) && !self.page_empty {
            self.new_page();
        }
        let top = self.y;
        self.y -= height;
        self.page_empty = false;
        top
    }

    /// Space between blocks, dropped at the top of a page. The quote rails the two blocks
    /// share run through it.
    fn gap(&mut self, height: f32, quotes: &[usize]) {
        if self.page_empty || height <= 0.0 {
            return;
        }
        if !self.fits(height) {
            self.new_page();
            return;
        }
        let shared = self
            .previous_quotes
            .iter()
            .zip(quotes)
            .take_while(|(previous, next)| previous == next)
            .count();
        let top = self.y;
        self.y -= height;
        self.quote_rails(&quotes[..shared], top, height);
    }

    fn quote_rails(&mut self, quotes: &[usize], top: f32, height: f32) {
        for depth in quotes {
            let x = MARGIN + depth_offset(*depth) + QUOTE_RAIL_OFFSET;
            self.page().strokes.push(Stroke {
                from: (x, top),
                to: (x, top - height),
                width: QUOTE_RAIL_WIDTH,
                color: QUOTE_RAIL_COLOR,
            });
        }
    }

    fn text_block(&mut self, role: Role, spans: &[Span], marker: Option<&str>, depth: usize, quotes: &[usize]) {
        let base = match role {
            Role::Body => TextBase {
                size: BODY_SIZE,
                bold: false,
                italic: false,
            },
            Role::Heading(level) => TextBase {
                size: HEADING_SIZES[usize::from(level - 1)],
                bold: true,
                italic: level == 6,
            },
        };
        let x = MARGIN + depth_offset(depth);
        let lines = self.break_lines(spans, base, CONTENT_RIGHT - x);
        let line_height = base.size * LINE_SPACING;

        if let Role::Heading(level) = role {
            let height = line_height * lines.len() as f32;
            if !self.fits(height + HEADING_KEEP) && !self.page_empty {
                self.new_page();
            }
            self.headings.push(Heading {
                level,
                title: plain_text(spans),
                page: self.pages.len() - 1,
                top: self.y,
            });
        }

        for (index, line) in lines.iter().enumerate() {
            let top = self.reserve(line_height);
            let baseline = baseline(top, base.size);
            if index == 0
                && let Some(marker) = marker
            {
                // The space after the marker is the gap before the text, drawn as a real
                // space so that readers joining runs in stream order still separate them.
                let base = TextBase {
                    size: BODY_SIZE,
                    bold: false,
                    italic: false,
                };
                let mut marker_line = Line::default();
                for c in marker.chars().chain([' ']) {
                    marker_line.push(self.piece(c, Style::default(), base));
                }
                self.draw_line(&marker_line, x - marker_line.width, baseline);
            }
            self.draw_line(line, x, baseline);
            self.quote_rails(quotes, top, line_height);
        }
    }

    fn code_block(&mut self, lines: &[String], depth: usize, quotes: &[usize]) {
        let x = MARGIN + depth_offset(depth);
        let width = CONTENT_RIGHT - x;
        let base = TextBase {
            size: CODE_SIZE,
            bold: false,
            italic: false,
        };
        let style = Style {
            code: true,
            ..Style::default()
        };
        let line_height = CODE_SIZE * LINE_SPACING;

        let mut visual_lines = Vec::new();
        for line in lines {
            let expanded = expand_tabs(line);
            visual_lines.extend(self.wrap_characters(&expanded, style, base, width - 2.0 * CODE_PADDING));
        }

        self.code_background(x, width, CODE_PADDING, quotes);
        for line in &visual_lines {
            let top = self.reserve(line_height);
            self.page().fills.push(Fill {
                rect: Rect {
                    x,
                    y: top - line_height,
                    width,
                    height: line_height,
                },
                color: CODE_BACKGROUND,
            });
            self.draw_line(line, x + CODE_PADDING, baseline(top, CODE_SIZE));
            self.quote_rails(quotes, top, line_height);
        }
        self.code_background(x, width, CODE_PADDING, quotes);
    }

    fn code_background(&mut self, x: f32, width: f32, height: f32, quotes: &[usize]) {
        let top = self.reserve(height);
        self.page().fills.push(Fill {
            rect: Rect {
                x,
                y: top - height,
                width,
                height,
            },
            color: CODE_BACKGROUND,
        });
        self.quote_rails(quotes, top, height);
    }

    fn rule(&mut self, depth: usize, quotes: &[usize]) {
        let x = MARGIN + depth_offset(depth);
        let top = self.reserve(2.0 * RULE_GAP);
        self.page().strokes.push(Stroke {
            from: (x, top - RULE_GAP),
            to: (CONTENT_RIGHT, top - RULE_GAP),
            width: 1.0,
            color: RULE_COLOR,
        });
        self.quote_rails(quotes, top, 2.0 * RULE_GAP);
    }

    fn table(&mut self, table: &Table, depth: usize, quotes: &[usize]) {
        let x = MARGIN + depth_offset(depth);
        let column_width = (CONTENT_RIGHT - x) / table.columns as f32;
        let rows: Vec<RowLines> = table.rows.iter().map(|row| self.row_lines(row, column_width)).collect();
        let header_rows: Vec<&RowLines> = rows.iter().take_while(|row| row.header).collect();
        let geometry = TableGeometry {
            x,
            column_width,
            alignments: &table.alignments,
            quotes,
        };

        for row in &rows {
            if !self.fits(row.height()) && !self.page_empty {
                self.continue_table(&geometry, row, &header_rows);
            }
            // A row taller than a page is split between lines, and continues on the next.
            let mut start = 0;
            loop {
                let available = ((self.y - MARGIN - 2.0 * CELL_PADDING_Y) / (BODY_SIZE * LINE_SPACING)).floor();
                let count = (available.max(1.0) as usize).min(row.line_count() - start);
                self.draw_row(&geometry, row, start..start + count);
                start += count;
                if start >= row.line_count() {
                    break;
                }
                self.continue_table(&geometry, row, &header_rows);
            }
        }
    }

    /// Move the table to a new page, repeating its header rows above a body row.
    fn continue_table(&mut self, geometry: &TableGeometry<'_>, row: &RowLines, header_rows: &[&RowLines]) {
        self.new_page();
        if !row.header {
            for header in header_rows {
                self.draw_row(geometry, header, 0..header.line_count());
            }
        }
    }

    fn row_lines(&self, row: &Row, column_width: f32) -> RowLines {
        let base = TextBase {
            size: BODY_SIZE,
            bold: row.header,
            italic: false,
        };
        let cells = row
            .cells
            .iter()
            .map(|spans| self.break_lines(spans, base, column_width - 2.0 * CELL_PADDING_X))
            .collect();
        RowLines {
            header: row.header,
            cells,
        }
    }

    /// Draw the lines `range` of every cell in `row` as one band of the table.
    fn draw_row(&mut self, geometry: &TableGeometry<'_>, row: &RowLines, range: std::ops::Range<usize>) {
        let line_height = BODY_SIZE * LINE_SPACING;
        let height = range.len() as f32 * line_height + 2.0 * CELL_PADDING_Y;
        let top = self.reserve(height);
        let width = geometry.column_width * row.cells.len() as f32;
        if row.header {
            self.page().fills.push(Fill {
                rect: Rect {
                    x: geometry.x,
                    y: top - height,
                    width,
                    height,
                },
                color: HEADER_BACKGROUND,
            });
        }

        for (column, lines) in row.cells.iter().enumerate() {
            let left = geometry.x + column as f32 * geometry.column_width;
            for (offset, line) in lines.iter().skip(range.start).take(range.len()).enumerate() {
                let free = geometry.column_width - 2.0 * CELL_PADDING_X - line.width;
                let indent = match geometry.alignments.get(column) {
                    Some(TableAlignment::Center) => free / 2.0,
                    Some(TableAlignment::Right) => free,
                    _ => 0.0,
                }
                .max(0.0);
                let line_top = top - CELL_PADDING_Y - offset as f32 * line_height;
                self.draw_line(line, left + CELL_PADDING_X + indent, baseline(line_top, BODY_SIZE));
            }
        }

        let right = geometry.x + width;
        let mut grid = vec![
            ((geometry.x, top), (right, top)),
            ((geometry.x, top - height), (right, top - height)),
        ];
        for column in 0..=row.cells.len() {
            let x = geometry.x + column as f32 * geometry.column_width;
            grid.push(((x, top), (x, top - height)));
        }
        for (from, to) in grid {
            self.page().strokes.push(Stroke {
                from,
                to,
                width: GRID_WIDTH,
                color: GRID_COLOR,
            });
        }
        self.quote_rails(geometry.quotes, top, height);
    }

    /// Break `spans` into lines no wider than `width`, at spaces and between CJK
    /// characters, and inside a word only when the word alone is wider than a line.
    fn break_lines(&self, spans: &[Span], base: TextBase, width: f32) -> Vec<Line> {
        let mut breaker = LineBreaker {
            width,
            lines: Vec::new(),
            line: Line::default(),
            spaces: Vec::new(),
            word: Vec::new(),
            word_width: 0.0,
        };
        for span in spans {
            for c in span.text.chars() {
                match c {
                    '\n' => breaker.hard_break(),
                    ' ' | '\t' => {
                        breaker.end_word();
                        breaker.spaces.push(self.piece(' ', span.style, base));
                    }
                    c if c.is_control() => {}
                    c if breaks_around(c) => {
                        breaker.end_word();
                        breaker.add_to_word(self.piece(c, span.style, base));
                        breaker.end_word();
                    }
                    c => breaker.add_to_word(self.piece(c, span.style, base)),
                }
            }
        }
        breaker.finish()
    }

    /// Break one line of code into lines no wider than `width`, keeping every space. A line
    /// breaks after its last space that follows some text, so words stay whole, and between
    /// characters only where no such space fits.
    fn wrap_characters(&self, text: &str, style: Style, base: TextBase, width: f32) -> Vec<Line> {
        let mut lines = Vec::new();
        let mut current: Vec<Piece> = Vec::new();
        for c in text.chars().filter(|c| !c.is_control()) {
            let piece = self.piece(c, style, base);
            while !current.is_empty() && pieces_width(&current) + piece.width > width {
                let rest = current
                    .iter()
                    .rposition(|piece| piece.text == " ")
                    .filter(|&space| current[..space].iter().any(|piece| piece.text != " "))
                    .map_or_else(Vec::new, |space| current.split_off(space + 1));
                lines.push(line_of(std::mem::replace(&mut current, rest)));
            }
            current.push(piece);
        }
        lines.push(line_of(current));
        lines
    }

    fn piece(&self, c: char, style: Style, base: TextBase) -> Piece {
        let style = Style {
            bold: style.bold || base.bold,
            italic: style.italic || base.italic,
            ..style
        };
        let face = Metrics::face_for(c, style.code, style.bold, style.italic);
        let size = match style.script {
            Script::Normal => base.size,
            Script::Super | Script::Sub => base.size * SCRIPT_SCALE,
        };
        Piece {
            text: c.to_string(),
            style,
            face,
            size,
            width: self.metrics.advance(face, c) * size / 1000.0,
        }
    }

    fn draw_line(&mut self, line: &Line, x: f32, baseline: f32) {
        let mut fills = Vec::new();
        let mut texts = Vec::new();
        let mut strokes = Vec::new();
        let mut links: Vec<LinkArea> = Vec::new();
        let mut cursor = x;

        for piece in &line.pieces {
            let full_size = match piece.style.script {
                Script::Normal => piece.size,
                Script::Super | Script::Sub => piece.size / SCRIPT_SCALE,
            };
            let rise = match piece.style.script {
                Script::Normal => 0.0,
                Script::Super => full_size * SUPERSCRIPT_RISE,
                Script::Sub => -full_size * SUBSCRIPT_DROP,
            };
            let band = Rect {
                x: cursor,
                y: baseline - DESCENT * full_size,
                width: piece.width,
                height: (ASCENT + DESCENT) * full_size,
            };
            if piece.style.highlight {
                fills.push(Fill {
                    rect: band,
                    color: HIGHLIGHT_COLOR,
                });
            }
            let color = if piece.style.link.is_some() { LINK_COLOR } else { BLACK };
            let synthetic = piece.face == Face::Sans;
            texts.push(TextRun {
                x: cursor,
                y: baseline,
                rise,
                size: piece.size,
                width: piece.width,
                face: piece.face,
                synthetic_bold: synthetic && piece.style.bold,
                synthetic_italic: synthetic && piece.style.italic,
                color,
                text: piece.text.clone(),
            });
            let stroke = full_size * 0.05;
            if piece.style.underline || piece.style.link.is_some() {
                let y = baseline - full_size * 0.12;
                strokes.push(Stroke {
                    from: (cursor, y),
                    to: (cursor + piece.width, y),
                    width: stroke,
                    color,
                });
            }
            if piece.style.strike {
                let y = baseline + rise + piece.size * 0.3;
                strokes.push(Stroke {
                    from: (cursor, y),
                    to: (cursor + piece.width, y),
                    width: stroke,
                    color,
                });
            }
            if let Some(target) = piece.style.link {
                match links.last_mut() {
                    Some(last) if last.target == target && (last.rect.x + last.rect.width - cursor).abs() < 0.01 => {
                        last.rect.width += piece.width;
                    }
                    _ => links.push(LinkArea { rect: band, target }),
                }
            }
            cursor += piece.width;
        }

        let page = self.page();
        page.fills.extend(fills);
        page.texts.extend(texts);
        page.strokes.extend(strokes);
        page.links.extend(links);
    }
}

struct TableGeometry<'a> {
    x: f32,
    column_width: f32,
    alignments: &'a [TableAlignment],
    quotes: &'a [usize],
}

struct RowLines {
    header: bool,
    cells: Vec<Vec<Line>>,
}

impl RowLines {
    fn line_count(&self) -> usize {
        self.cells.iter().map(Vec::len).max().unwrap_or(0).max(1)
    }

    fn height(&self) -> f32 {
        self.line_count() as f32 * BODY_SIZE * LINE_SPACING + 2.0 * CELL_PADDING_Y
    }
}

struct LineBreaker {
    width: f32,
    lines: Vec<Line>,
    line: Line,
    /// Spaces seen since the last word, dropped if the line breaks there.
    spaces: Vec<Piece>,
    word: Vec<Piece>,
    word_width: f32,
}

impl LineBreaker {
    fn add_to_word(&mut self, piece: Piece) {
        self.word_width += piece.width;
        self.word.push(piece);
    }

    fn end_word(&mut self) {
        if self.word.is_empty() {
            return;
        }
        let word = std::mem::take(&mut self.word);
        let word_width = std::mem::take(&mut self.word_width);
        let spaces = std::mem::take(&mut self.spaces);

        if !self.line.pieces.is_empty() {
            let spaces_width: f32 = spaces.iter().map(|space| space.width).sum();
            if self.line.width + spaces_width + word_width <= self.width {
                self.line.extend(spaces);
                self.line.extend(word);
                return;
            }
            self.lines.push(std::mem::take(&mut self.line));
        }

        if word_width <= self.width {
            self.line.extend(word);
            return;
        }
        for piece in word {
            if !self.line.pieces.is_empty() && self.line.width + piece.width > self.width {
                self.lines.push(std::mem::take(&mut self.line));
            }
            self.line.push(piece);
        }
    }

    fn hard_break(&mut self) {
        self.end_word();
        self.spaces.clear();
        self.lines.push(std::mem::take(&mut self.line));
    }

    fn finish(mut self) -> Vec<Line> {
        self.end_word();
        if !self.line.pieces.is_empty() || self.lines.is_empty() {
            self.lines.push(self.line);
        }
        self.lines
    }
}

fn pieces_width(pieces: &[Piece]) -> f32 {
    pieces.iter().map(|piece| piece.width).sum()
}

fn line_of(pieces: Vec<Piece>) -> Line {
    let mut line = Line::default();
    line.extend(pieces);
    line
}

fn depth_offset(depth: usize) -> f32 {
    depth.min(MAX_DEPTH) as f32 * INDENT_STEP
}

/// The baseline of a line whose box starts at `top`, with the text centred in the box.
fn baseline(top: f32, size: f32) -> f32 {
    let line_height = size * LINE_SPACING;
    top - (line_height - (ASCENT + DESCENT) * size) / 2.0 - ASCENT * size
}

fn expand_tabs(line: &str) -> String {
    let mut expanded = String::with_capacity(line.len());
    let mut column = 0;
    for c in line.chars() {
        if c == '\t' {
            let spaces = TAB_WIDTH - column % TAB_WIDTH;
            expanded.extend(std::iter::repeat_n(' ', spaces));
            column += spaces;
        } else {
            expanded.push(c);
            column += 1;
        }
    }
    expanded
}

/// Scripts written without spaces between words, where a line may break between any
/// two characters.
fn breaks_around(c: char) -> bool {
    matches!(c,
        '\u{2E80}'..='\u{303F}'
        | '\u{3040}'..='\u{30FF}'
        | '\u{3100}'..='\u{31FF}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}'
        | '\u{AC00}'..='\u{D7AF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{FF00}'..='\u{FFEF}'
        | '\u{20000}'..='\u{2FA1F}')
}

fn plain_text(spans: &[Span]) -> String {
    let text: String = spans.iter().map(|span| span.text.replace('\n', " ")).collect();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

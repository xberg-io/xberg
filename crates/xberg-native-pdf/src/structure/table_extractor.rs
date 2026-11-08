//! Table extraction from PDF structure tree.
//!
//! Implements table detection and reconstruction according to ISO 32000-1:2008 Section 14.8.4.3.4
//! (Table Elements).
//!
//! Table structure hierarchy:
//! - Table: Top-level container
//!   - THead: Optional header row group
//!   - TBody: One or more body row groups
//!   - TFoot: Optional footer row group
//! - TR: Table row (contains TH and/or TD cells)
//!   - TH: Table header cell
//!   - TD: Table data cell

use crate::error::Error;
use crate::geometry::Rect;
use crate::layout::{Color, FontWeight, TextBlock, TextSpan};
use crate::structure::types::{McidScope, StructChild, StructElem, StructType};

/// A complete extracted table with rows and optional header information.
#[derive(Debug, Clone)]
pub struct Table {
    /// Rows of the table (alternating between header and body rows)
    pub rows: Vec<TableRow>,

    /// Whether the table has an explicit header section
    pub has_header: bool,

    /// Number of columns (inferred from first row)
    pub col_count: usize,

    /// Bounding box of the table region (used to exclude table spans from normal rendering)
    pub bbox: Option<Rect>,
}

/// A single row in a table.
#[derive(Debug, Clone)]
pub struct TableRow {
    /// Cells in this row
    pub cells: Vec<TableCell>,

    /// Whether this is a header row
    pub is_header: bool,
}

/// A single cell in a table.
#[derive(Debug, Clone)]
pub struct TableCell {
    /// Text content of the cell
    pub text: String,

    /// Original text spans that make up this cell's content, with
    /// font/style metadata preserved for format-aware rendering.
    pub spans: Vec<crate::layout::TextSpan>,

    /// Number of columns this cell spans (default 1)
    pub colspan: u32,

    /// Number of rows this cell spans (default 1)
    pub rowspan: u32,

    /// MCID values that make up this cell's content
    pub mcids: Vec<u32>,

    /// Bounding box of the cell
    pub bbox: Option<Rect>,

    /// Whether this is a header cell
    pub is_header: bool,
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

impl Table {
    /// Create a new extracted table
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            has_header: false,
            col_count: 0,
            bbox: None,
        }
    }

    /// Check whether this table looks like a real data grid as opposed to
    /// spurious spatial output (form layouts, label-colon-value pairs).
    ///
    /// A real grid has:
    /// - ≥2 rows
    /// - ≥2 columns
    /// - Consistent column population: at least 50% of rows must have
    ///   at least 2 non-empty cells. Filters out the common
    ///   form-as-table false positive where rows look like
    ///   `| Single label, all other slots empty | | | |`.
    pub fn is_real_grid(&self) -> bool {
        if self.col_count < 2 || self.rows.len() < 2 {
            return false;
        }
        let rows_with_two_or_more_filled_cells = self
            .rows
            .iter()
            .filter(|r| r.cells.iter().filter(|c| !c.text.trim().is_empty()).count() >= 2)
            .count();
        let ratio = rows_with_two_or_more_filled_cells as f32 / self.rows.len() as f32;

        // Wide tables (≥ 8 columns) are high-risk false positives: prose sentences
        // can be split into many single-phrase cells by decorative rule lines.
        // Real wide data tables have most rows densely filled (≥ 60% of columns);
        // prose-split false tables have highly variable row fill counts (some rows
        // have 1-2 filled cells, others have 10+), so the fraction of "dense" rows
        // is well below 70%.
        //
        // Exception: a consolidated multi-row table can contain a mix
        // of dense data rows and sparse header / multi-row-label rows.  The sparse
        // rows are legitimate table content (column headers split across multiple
        // visual rows, lane-count labels that only appear on the first row of a
        // sub-group), so a strict dense-row ratio rejects real tables.  Accept the
        // table if it has BOTH enough dense rows in absolute terms (≥ half the
        // column count) AND a meaningful dense-row ratio (≥ 40 %). ~keep
        if self.col_count >= 8 {
            let min_dense = ((self.col_count as f32 * 0.6) as usize).max(2);
            let dense_rows = self
                .rows
                .iter()
                .filter(|r| r.cells.iter().filter(|c| !c.text.trim().is_empty()).count() >= min_dense)
                .count();
            let dense_row_ratio = dense_rows as f32 / self.rows.len() as f32;
            if self.rows.len() >= 3 && ratio >= 0.7 && dense_row_ratio >= 0.70 {
                return true;
            }
            let min_absolute_dense = (self.col_count / 2).max(3);
            return dense_rows >= min_absolute_dense && dense_row_ratio >= 0.40;
        }

        ratio >= 0.5
    }

    /// Render the table as clean, space-padded plain text.
    pub fn render_text(&self) -> String {
        let col_count = self.col_count;
        if col_count == 0 || self.rows.is_empty() {
            return String::new();
        }

        let mut col_widths = vec![0usize; col_count];
        for row in &self.rows {
            let mut col_idx = 0;
            for cell in &row.cells {
                if cell.colspan == 1 && col_idx < col_count {
                    let w = cell.text.trim().chars().count();
                    col_widths[col_idx] = col_widths[col_idx].max(w);
                }
                col_idx += cell.colspan as usize;
            }
        }

        for w in &mut col_widths {
            if *w < 2 {
                *w = 2;
            }
        }

        // Trim trailing empty columns (no non-empty cell contributes content
        // to that column, including cells with colspan > 1 that cover it). ~keep
        let effective_cols = {
            let mut eff = col_widths.len();
            while eff > 0 {
                let col = eff - 1;
                let all_empty = self.rows.iter().all(|row| {
                    let mut ci = 0;
                    for cell in &row.cells {
                        let span = cell.colspan as usize;
                        let covers_col = ci <= col && col < ci + span;
                        if covers_col {
                            return cell.text.trim().is_empty();
                        }
                        ci += span;
                    }
                    true
                });
                if all_empty {
                    eff -= 1;
                } else {
                    break;
                }
            }
            eff
        };
        if effective_cols == 0 {
            return String::new();
        }
        let col_widths = &col_widths[..effective_cols];

        let is_right_aligned: Vec<bool> = (0..effective_cols)
            .map(|c| {
                let mut has_content = false;
                for row in &self.rows {
                    let mut ci = 0;
                    for cell in &row.cells {
                        if ci == c && cell.colspan == 1 {
                            let t = cell.text.trim();
                            if !t.is_empty() {
                                has_content = true;
                                let stripped: String = t
                                    .chars()
                                    .filter(|ch| {
                                        !matches!(ch, '$' | '€' | '£' | ',' | ' ' | '%' | '+' | '-' | '(' | ')')
                                    })
                                    .collect();
                                if stripped.is_empty() || stripped.parse::<f64>().is_err() {
                                    return false;
                                }
                            }
                        }
                        ci += cell.colspan as usize;
                    }
                }
                has_content
            })
            .collect();

        let mut output = String::new();

        for row in &self.rows {
            let mut col_idx = 0;
            let mut cells_text = Vec::new();
            for cell in &row.cells {
                let text = cell.text.trim();
                if col_idx < effective_cols {
                    let width = if cell.colspan > 1 {
                        let end = (col_idx + cell.colspan as usize).min(effective_cols);
                        let base: usize = col_widths[col_idx..end].iter().sum();
                        base + (end - col_idx).saturating_sub(1) * 2
                    } else {
                        col_widths[col_idx]
                    };
                    let formatted = if cell.colspan == 1 && is_right_aligned[col_idx] {
                        format!("{:>width$}", text, width = width)
                    } else {
                        format!("{:<width$}", text, width = width)
                    };
                    cells_text.push(formatted);
                } else {
                    cells_text.push(text.to_string());
                }
                col_idx += cell.colspan as usize;
            }
            output.push_str(cells_text.join("  ").trim_end());
            output.push('\n');
        }

        output
    }

    /// Add a row to the table
    pub fn add_row(&mut self, row: TableRow) {
        if self.col_count == 0 && !row.cells.is_empty() {
            self.col_count = row.cells.len();
        }
        self.rows.push(row);
    }

    /// Check if table is empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

impl TableRow {
    /// Create a new table row
    pub fn new(is_header: bool) -> Self {
        Self {
            cells: Vec::new(),
            is_header,
        }
    }

    /// Check if any cell in this row has a colspan > 1.
    pub fn has_colspan(&self) -> bool {
        self.cells.iter().any(|c| c.colspan > 1)
    }

    /// Add a cell to the row
    pub fn add_cell(&mut self, cell: TableCell) {
        self.cells.push(cell);
    }
}

impl TableCell {
    /// Create a new table cell
    pub fn new(text: String, is_header: bool) -> Self {
        Self {
            text,
            spans: Vec::new(),
            colspan: 1,
            rowspan: 1,
            mcids: Vec::new(),
            bbox: None,
            is_header,
        }
    }

    /// Set colspan
    pub fn with_colspan(mut self, colspan: u32) -> Self {
        self.colspan = colspan;
        self
    }

    /// Set rowspan
    pub fn with_rowspan(mut self, rowspan: u32) -> Self {
        self.rowspan = rowspan;
        self
    }

    /// Add an MCID
    pub fn add_mcid(&mut self, mcid: u32) {
        self.mcids.push(mcid);
    }
}

/// Find all Table structure elements in the structure tree for a given page.
///
/// Recursively walks the structure tree to collect StructElem nodes where
/// `struct_type == StructType::Table` and the element (or any descendant)
/// has marked content on the specified page.
///
/// # Arguments
/// * `struct_tree` - The structure tree root
/// * `page_num` - Page number to match (0-based)
///
/// # Returns
/// * `Vec<&StructElem>` - Table elements found for the page
pub fn find_table_elements(struct_tree: &crate::structure::types::StructTreeRoot, page_num: u32) -> Vec<&StructElem> {
    let mut tables = Vec::new();
    for elem in &struct_tree.root_elements {
        collect_table_elements(elem, page_num, &mut tables);
    }
    tables
}

/// Recursively collect Table elements that have content on the given page.
fn collect_table_elements<'a>(elem: &'a StructElem, page_num: u32, tables: &mut Vec<&'a StructElem>) {
    if elem.struct_type == StructType::Table {
        if element_has_page_content(elem, page_num) {
            tables.push(elem);
        }
        return; // Don't recurse into table children looking for nested tables ~keep
    }

    for child in &elem.children {
        if let StructChild::StructElem(child_elem) = child {
            collect_table_elements(child_elem, page_num, tables);
        }
    }
}

/// Walk the structure tree once and bucket every `Table` element by each page
/// it has content on (owned clones), so the converter table path can do an
/// O(1) per-page lookup instead of `find_table_elements`'s per-page walk
/// (≈ O(pages²) on a tagged document). For a given page the result matches
/// `find_table_elements(tree, page)`: same DFS pre-order, and like
/// `collect_table_elements` it does not recurse into a Table's children.
pub fn find_table_elements_all_pages(
    struct_tree: &crate::structure::types::StructTreeRoot,
) -> std::collections::HashMap<u32, Vec<StructElem>> {
    let mut by_page: std::collections::HashMap<u32, Vec<StructElem>> = std::collections::HashMap::new();
    for elem in &struct_tree.root_elements {
        collect_table_elements_all_pages(elem, &mut by_page);
    }
    by_page
}

fn collect_table_elements_all_pages(elem: &StructElem, by_page: &mut std::collections::HashMap<u32, Vec<StructElem>>) {
    if elem.struct_type == StructType::Table {
        // Pages this table (or any descendant) has content on — the exact set
        // for which `element_has_page_content(elem, page)` is true. ~keep
        let mut pages = std::collections::BTreeSet::new();
        collect_content_pages(elem, &mut pages);
        for p in pages {
            by_page.entry(p).or_default().push(elem.clone());
        }
        return; // mirror collect_table_elements: don't recurse into table children ~keep
    }

    for child in &elem.children {
        if let StructChild::StructElem(child_elem) = child {
            collect_table_elements_all_pages(child_elem, by_page);
        }
    }
}

/// Collect the set of pages on which `elem` (or any descendant) has content.
/// Mirrors the truth set of [`element_has_page_content`] across all pages.
fn collect_content_pages(elem: &StructElem, pages: &mut std::collections::BTreeSet<u32>) {
    if let Some(p) = elem.page {
        pages.insert(p);
    }
    for child in &elem.children {
        match child {
            StructChild::MarkedContentRef { page, .. } => {
                pages.insert(*page);
            }
            StructChild::StructElem(child_elem) => {
                collect_content_pages(child_elem, pages);
            }
            StructChild::ObjectRef(_, _) => {}
        }
    }
}

/// Check if a structure element or any descendant has marked content on the given page.
fn element_has_page_content(elem: &StructElem, page_num: u32) -> bool {
    if elem.page == Some(page_num) {
        return true;
    }

    for child in &elem.children {
        match child {
            StructChild::MarkedContentRef { page, .. } => {
                if *page == page_num {
                    return true;
                }
            }
            StructChild::StructElem(child_elem) => {
                if element_has_page_content(child_elem, page_num) {
                    return true;
                }
            }
            StructChild::ObjectRef(_, _) => {}
        }
    }

    false
}

/// Extract a table from a structure element tree using TextSpans (MCID matching).
///
/// Converts TextSpans to a format suitable for MCID-based cell text extraction,
/// then delegates to the standard `extract_table` function.
///
/// # Arguments
/// * `table_elem` - The Table structure element
/// * `spans` - Text spans from the page (with MCID values)
///
/// # Returns
/// * `Table` containing all rows and cells
pub fn extract_table_from_spans(table_elem: &StructElem, spans: &[crate::layout::TextSpan]) -> Result<Table, Error> {
    // Convert spans to TextBlocks for MCID matching, applying column-spanning
    // decimal split so that "12.11" (sailing score columns) becomes "12 11". ~keep
    let text_blocks: Vec<TextBlock> = spans
        .iter()
        .filter(|s| s.mcid.is_some())
        .map(|s| {
            let text = span_text_for_cell(s);
            TextBlock {
                chars: Vec::new(),
                bbox: s.bbox,
                text,
                avg_font_size: s.font_size,
                dominant_font: s.font_name.clone(),
                is_bold: s.font_weight.is_bold(),
                is_italic: s.is_italic,
                mcid: s.mcid,
                mcid_scope: s.mcid_scope.clone(),
                sequence: s.sequence,
                rotation_degrees: s.rotation_degrees,
            }
        })
        .collect();
    extract_table(table_elem, &text_blocks)
}

/// Return the display text for a span when used as a table cell token.
/// Mirrors `PdfDocument::push_span_text`: splits column-spanning decimals
/// (e.g. "12.11" across adjacent score columns) at the decimal point.
pub(super) fn span_text_for_cell(span: &crate::layout::TextSpan) -> String {
    let text = &span.text;
    let dot_pos = match text.find('.') {
        Some(p) if p > 0 && p < text.len() - 1 => p,
        _ => return text.clone(),
    };
    if text[dot_pos + 1..].contains('.') {
        return text.clone();
    }
    if !text[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
        return text.clone();
    }
    if !text[dot_pos + 1..].chars().all(|c| c.is_ascii_digit()) {
        return text.clone();
    }
    let char_count = text.chars().count();
    // Signal 1: sparse char_widths array (cw.len < char_count) means the
    // span was assembled from two concatenated Tj runs — see the matching
    // `is_column_spanning_decimal` rule in document.rs.  Catches sailing-
    // score cells emitted as a single Tj like "1.10" (cw=[w]) where the
    // PDF actually means "1" followed by "10" in adjacent score columns
    // bbox.width can still be tight here, so the
    // bbox-inflation check below isn't sufficient. ~keep
    if !span.char_widths.is_empty() && span.char_widths.len() < char_count {
        return format!("{} {}", &text[..dot_pos], &text[dot_pos + 1..]);
    }
    let expected_width = if !span.char_widths.is_empty() {
        let cw_sum: f32 = span.char_widths.iter().sum();
        cw_sum * (char_count as f32 / span.char_widths.len() as f32)
    } else if span.font_size > 0.0 {
        // 0.50em per char: digits are narrower than average; keeps the
        // fallback from producing false negatives on word_spans (char_widths=[]). ~keep
        span.font_size * 0.50 * char_count as f32
    } else {
        return text.clone();
    };
    let gap = span.bbox.width - expected_width;
    if span.font_size > 0.0 && gap > span.font_size * 1.0 {
        format!("{} {}", &text[..dot_pos], &text[dot_pos + 1..])
    } else {
        text.clone()
    }
}

/// Extract a table from a structure element tree.
///
/// According to PDF spec Section 14.8.4.3.4, a Table element may contain:
/// - Direct TR (table row) children, OR
/// - THead (optional) + TBody (one or more) + TFoot (optional)
///
/// # Arguments
/// * `table_elem` - The Table structure element
/// * `text_blocks` - All text blocks in the document (for MCID matching)
///
/// # Returns
/// * `Table` containing all rows and cells
pub fn extract_table(table_elem: &StructElem, text_blocks: &[TextBlock]) -> Result<Table, Error> {
    let mut table = Table::new();

    let has_thead = table_elem
        .children
        .iter()
        .any(|child| matches!(child, StructChild::StructElem(elem) if elem.struct_type == StructType::THead));

    if has_thead {
        table.has_header = true;
    }

    for child in &table_elem.children {
        match child {
            StructChild::StructElem(elem) => match elem.struct_type {
                StructType::TR => {
                    let row = extract_row(elem, text_blocks, false)?;
                    table.add_row(row);
                }
                StructType::THead => {
                    extract_row_group(elem, text_blocks, true, &mut table)?;
                }
                StructType::TBody => {
                    extract_row_group(elem, text_blocks, false, &mut table)?;
                }
                StructType::TFoot => {
                    extract_row_group(elem, text_blocks, false, &mut table)?;
                }
                _ => {}
            },
            StructChild::MarkedContentRef { .. } => {}
            StructChild::ObjectRef(_, _) => {}
        }
    }

    Ok(table)
}

/// Extract rows from a row group (THead, TBody, TFoot).
fn extract_row_group(
    group_elem: &StructElem,
    text_blocks: &[TextBlock],
    is_header: bool,
    table: &mut Table,
) -> Result<(), Error> {
    for child in &group_elem.children {
        match child {
            StructChild::StructElem(elem) if elem.struct_type == StructType::TR => {
                let row = extract_row(elem, text_blocks, is_header)?;
                table.add_row(row);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Extract a single row (TR element).
fn extract_row(tr_elem: &StructElem, text_blocks: &[TextBlock], force_header: bool) -> Result<TableRow, Error> {
    let mut row = TableRow::new(force_header);

    for child in &tr_elem.children {
        match child {
            StructChild::StructElem(elem) => match elem.struct_type {
                StructType::TH => {
                    let cell = extract_cell(elem, text_blocks, true)?;
                    row.add_cell(cell);
                }
                StructType::TD => {
                    let cell = extract_cell(elem, text_blocks, false)?;
                    row.add_cell(cell);
                }
                _ => {}
            },
            StructChild::MarkedContentRef { .. } => {}
            StructChild::ObjectRef(_, _) => {}
        }
    }

    Ok(row)
}

/// Extract a single cell (TH or TD element).
fn extract_cell(cell_elem: &StructElem, text_blocks: &[TextBlock], is_header: bool) -> Result<TableCell, Error> {
    let mut mcids = Vec::new();
    collect_mcids(cell_elem, &mut mcids);
    let mut scoped_mcids = Vec::new();
    collect_scoped_mcids(cell_elem, &mut scoped_mcids);

    // Find all text blocks that match these MCIDs, joining them with position-aware
    // spacing: insert a space only when there is a genuine horizontal gap between
    // adjacent spans on the same line, or when spans are on different lines.
    // This prevents spurious spaces inside CJK expressions like "Q（peu/d）" whose
    // glyphs are stored as separate marked-content runs that abut each other. ~keep
    let mut cell_text = String::new();
    let mut cell_spans: Vec<TextSpan> = Vec::new();

    // Collect every block the cell's marked-content sequences own, then join
    // them in reading order: content-stream order routinely interleaves a
    // wrapped title's lines.
    //
    // Reading order follows the run's OWN writing axis, not the page's. ISO
    // 32000-1 §9.4.4 puts glyph displacement along the text matrix's writing
    // direction, so a line rotated 90° advances along y while successive
    // lines step along x, and right-to-left script is stored in logical order
    // while advancing toward decreasing x. Projecting onto those axes
    // recovers reading order for upright, rotated and RTL cells alike, where
    // a fixed y-then-x sort reverses the last two. ~keep
    let mut blocks: Vec<&TextBlock> = text_blocks.iter().filter(|b| block_belongs(b, &scoped_mcids)).collect();

    let (sin, cos) = cell_rotation_degrees(&blocks).to_radians().sin_cos();
    let direction = if blocks.iter().any(|b| crate::text::bidi::looks_rtl(&b.text)) {
        -1.0
    } else {
        1.0
    };
    // Along the writing direction, and perpendicular to it in the direction
    // successive lines advance (unrotated: +x and "down the page"). ~keep
    let along = |b: &TextBlock| direction * (b.bbox.x * cos + b.bbox.y * sin);
    let perp = |b: &TextBlock| b.bbox.x * sin - b.bbox.y * cos;
    // How far a run extends along its own writing direction. A span's bbox is
    // run-local — `width` is the accumulated advance and `height` the font
    // size, whatever the rotation (see the TextSpan construction in
    // extractors/text.rs) — so the writing-axis extent is always `width`. ~keep
    let advance_extent = |b: &TextBlock| b.bbox.width;

    // `perp` multiplies a bbox coordinate by a sine that is exactly 0.0 at
    // rotation 0, so a non-finite coordinate yields NaN on the axis we are not
    // even sorting along. Comparing NaN as Equal breaks transitivity and
    // `sort_by` then panics, so use the crate's NaN ordering. ~keep
    blocks.sort_by(|a, b| crate::utils::safe_float_cmp(perp(a), perp(b)));
    // Lines are formed by walking along the perpendicular axis and opening a
    // new line when the step from the current line's first block exceeds half
    // the SMALLER of the two heights — a per-pair threshold, so one tall
    // multi-line block cannot swallow line spacing tighter than its own
    // height. ~keep
    let mut keyed: Vec<(usize, &TextBlock)> = Vec::with_capacity(blocks.len());
    let mut line = 0usize;
    let mut anchor: Option<&TextBlock> = None;
    for b in blocks {
        if let Some(a) = anchor {
            let threshold = a.bbox.height.min(b.bbox.height).max(1.0) * 0.5;
            if perp(b) - perp(a) > threshold {
                line += 1;
                anchor = Some(b);
            }
        } else {
            anchor = Some(b);
        }
        keyed.push((line, b));
    }
    keyed.sort_by(|(line_a, a), (line_b, b)| {
        line_a
            .cmp(line_b)
            .then_with(|| crate::utils::safe_float_cmp(along(a), along(b)))
    });
    // `direction` reversed the whole cell, but UAX #9 reverses only the
    // surrounding script: numerals (class EN/AN) keep left-to-right order
    // inside a right-to-left paragraph. Producers routinely split one figure
    // across marked-content runs ("4" + "7.500"), so without this the sort
    // reads it back as "7.5004" — a silently altered figure, not a spacing
    // artefact. ~keep
    if direction < 0.0 {
        restore_numeric_run_order(&mut keyed);
    }

    let mut prev_block: Option<(usize, &TextBlock)> = None;
    for (line_index, block) in keyed {
        let mut leading_space = false;
        if !cell_text.is_empty() {
            let need_space = if let Some((prev_line, prev)) = prev_block {
                // Line membership was already decided above, in the writing
                // frame. Re-deriving it here from page-frame y would call
                // every run of a rotated line a new line (they differ in y by
                // their own advance) and inject a space into every word. ~keep
                let gap = along(block) - (along(prev) + advance_extent(prev));
                let font_size = prev.avg_font_size.max(block.avg_font_size).max(1.0);
                if line_index != prev_line {
                    true
                } else if gap <= font_size * 0.15 {
                    false
                } else {
                    // Suppress space insertion when one side is CJK and the
                    // other is CJK or a fullwidth/math operator (e.g. ≤, ＜, μ).
                    // This mirrors the CJK-pair suppression in document.rs and
                    // converters/mod.rs. ~keep
                    #[inline(always)]
                    fn is_cjk(c: char) -> bool {
                        matches!(c,
                            '\u{3040}'..='\u{309F}' |   // Hiragana ~keep
                            '\u{30A0}'..='\u{30FF}' |   // Katakana ~keep
                            '\u{4E00}'..='\u{9FFF}' |   // CJK Unified Ideographs ~keep
                            '\u{AC00}'..='\u{D7AF}' |   // Hangul ~keep
                            '\u{3400}'..='\u{4DBF}' |   // CJK Extension A ~keep
                            '\u{20000}'..='\u{2A6DF}'   // CJK Extension B ~keep
                        )
                    }
                    #[inline(always)]
                    fn is_fw_math(c: char) -> bool {
                        matches!(c,
                            '\u{FF0B}' | '\u{FF0D}' |
                            '\u{FF1A}' | '\u{FF1B}' |
                            '\u{FF1C}'..='\u{FF1E}' |
                            '\u{2260}' | '\u{2248}' |
                            '\u{2264}'..='\u{2265}' |
                            '\u{00B5}' | '\u{03BC}' |
                            '\u{00B1}' | '\u{00D7}' | '\u{00F7}'
                        )
                    }
                    let p_last = prev.text.chars().next_back();
                    let b_first = block.text.chars().next();
                    let suppress = if let (Some(p), Some(b)) = (p_last, b_first) {
                        let p_cjk = is_cjk(p);
                        let b_cjk = is_cjk(b);
                        (p_cjk || is_fw_math(p)) && (b_cjk || is_fw_math(b)) && (p_cjk || b_cjk)
                    } else {
                        false
                    };
                    !suppress
                }
            } else {
                !cell_text.ends_with(' ')
            };
            if need_space {
                cell_text.push(' ');
                leading_space = true;
            }
        }
        cell_text.push_str(&block.text);
        // Synthesize a minimal TextSpan capturing the block's
        // style. Only the fields the markdown converter
        // consults (text, font_weight, is_italic, font_size,
        // bbox) need real values — everything else is filled
        // from sensible defaults. Carry the inter-block space
        // into the span text as well: the markdown/HTML table
        // renderers reconstruct spacing from the spans (not from
        // cell_text), and their horizontal-gap heuristic cannot
        // see a line wrap, so without this they glue tokens
        // across wrapped lines. Both renderers already treat a
        // leading space in the span text as authoritative
        // (their `already_has_space` guard), so this never
        // double-spaces. ~keep
        let span_text = if leading_space {
            let mut s = String::with_capacity(block.text.len() + 1);
            s.push(' ');
            s.push_str(&block.text);
            s
        } else {
            block.text.clone()
        };
        cell_spans.push(TextSpan {
            provenance: None,
            artifact_type: None,
            text: span_text,
            bbox: block.bbox,
            font_name: block.dominant_font.clone(),
            font_size: block.avg_font_size,
            font_weight: if block.is_bold {
                FontWeight::Bold
            } else {
                FontWeight::Normal
            },
            is_italic: block.is_italic,
            is_monospace: false,
            color: Color::black(),
            mcid: block.mcid,
            mcid_scope: None,
            sequence: 0,
            offset_semantic: false,
            split_boundary_before: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            // Carry the source block's rotation rather than resetting to 0.0: this push
            // re-synthesises a span from an already-rotated block, so hardcoding zero
            // silently flattens rotated table cells. Upstream fixed this on the older
            // per-block push; the fix belongs here now that the push is line-aware. ~keep
            rotation_degrees: block.rotation_degrees,
            wmode: 0,
            text_rise: 0.0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        });
        prev_block = Some((line_index, block));
    }

    let mut cell = TableCell::new(cell_text.trim().to_string(), is_header);
    cell.mcids = mcids;
    cell.spans = cell_spans;

    Ok(cell)
}

/// Whether a numeral keeps left-to-right order inside right-to-left text —
/// UAX #9 class EN (European) or AN (Arabic-Indic).
fn is_uax9_numeral(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '\u{0660}'..='\u{0669}' | '\u{06F0}'..='\u{06F9}')
}

/// Whether a cell block is a numeric run that must not share the surrounding
/// script's reversal.
///
/// At least one numeral is required deliberately. Absorbing bare separators
/// (a lone "." or "%") into the run measures worse: it pulls a trailing
/// separator into a run the sort already placed correctly, turning one wrong
/// cell into two. Full UAX #9 resolution in the cell sorter is a larger design
/// question than this defect.
fn is_numeric_run(text: &str) -> bool {
    let mut saw_numeral = false;
    for c in text.trim().chars() {
        if is_uax9_numeral(c) {
            saw_numeral = true;
        } else if !matches!(c, '.' | ',' | '\u{066B}' | '\u{066C}') {
            return false;
        }
    }
    saw_numeral
}

/// Undo the right-to-left sort reversal within each maximal run of numeric
/// blocks, leaving the surrounding script reversed. A run never spans a line
/// boundary, so line membership bounds it.
fn restore_numeric_run_order(keyed: &mut [(usize, &TextBlock)]) {
    let mut start = 0usize;
    while start < keyed.len() {
        if !is_numeric_run(&keyed[start].1.text) {
            start += 1;
            continue;
        }
        let line = keyed[start].0;
        let mut end = start + 1;
        while end < keyed.len() && keyed[end].0 == line && is_numeric_run(&keyed[end].1.text) {
            end += 1;
        }
        keyed[start..end].reverse();
        start = end;
    }
}

/// The quadrant the cell's glyph runs are drawn at, as the modal rotation of
/// its blocks. A cell mixing rotations (a rotated header beside an upright
/// note) reads in the majority frame; ties resolve to the smaller angle so
/// the choice is deterministic.
fn cell_rotation_degrees(blocks: &[&TextBlock]) -> f32 {
    let mut counts: [usize; 4] = [0; 4];
    for b in blocks {
        let normalized = b.rotation_degrees.rem_euclid(360.0);
        // Snap to a quadrant; a free angle (skew) reads in the upright frame,
        // which is what the unrotated sort did for it before. ~keep
        let quadrant = ((normalized / 90.0).round() as usize) % 4;
        if (normalized - (quadrant as f32) * 90.0).abs() < 0.5 {
            counts[quadrant] += 1;
        } else {
            counts[0] += 1;
        }
    }
    let (index, _) = counts
        .iter()
        .enumerate()
        .max_by_key(|&(ref index, &count)| (count, std::cmp::Reverse(*index)))
        .expect("counts is non-empty");
    (index as f32) * 90.0
}

/// Recursively collect all MCIDs from a structure element and its children.
fn collect_mcids(elem: &StructElem, mcids: &mut Vec<u32>) {
    let mut scoped = Vec::new();
    collect_scoped_mcids(elem, &mut scoped);
    mcids.extend(scoped.into_iter().map(|(_, mcid)| mcid));
}

/// Collect a cell's marked-content references with the content stream each one
/// names.
///
/// An MCID is unique only inside its own content stream (ISO 32000-1 §14.7.4.3),
/// so two Form XObjects on one page routinely both number a run 0. Carrying the
/// scope lets a cell tell its own run from an identically numbered one drawn by
/// a different stream.
fn collect_scoped_mcids(elem: &StructElem, out: &mut Vec<(McidScope, u32)>) {
    for child in &elem.children {
        match child {
            StructChild::MarkedContentRef { mcid, scope, .. } => {
                out.push((scope.clone(), *mcid));
            }
            StructChild::StructElem(child_elem) => {
                collect_scoped_mcids(child_elem, out);
            }
            StructChild::ObjectRef(_, _) => {}
        }
    }
}

/// Whether a text block is one of the runs this cell's references name.
///
/// The scope decides only where it can: a block whose stream matches a
/// reference naming that stream belongs to the cell. Where no block carries the
/// stream a reference names — the producer omitted `/Stm`, which is legal and
/// common — the MCID number alone decides, so those cells keep their text
/// instead of emptying.
fn block_belongs(block: &TextBlock, scoped: &[(McidScope, u32)]) -> bool {
    let Some(mcid) = block.mcid else { return false };
    let named: Vec<&McidScope> = scoped.iter().filter(|(_, m)| *m == mcid).map(|(s, _)| s).collect();
    if named.is_empty() {
        return false;
    }
    match &block.mcid_scope {
        // The block names the stream a reference names: it is this cell's run.
        Some(scope) if named.contains(&scope) => true,
        // The block names a stream, and it is not one this cell references:
        // a different stream's run that happens to share the number.
        Some(_) => false,
        // The block names no stream, so the producer omitted `/Stm`. The
        // number alone decides, which is what kept this cell's text before
        // scopes existed.
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::types::StructTreeRoot;

    /// A block whose bbox carries a non-finite coordinate, which a malformed
    /// file can produce. `perp()` multiplies x by a sine that is exactly 0.0 at
    /// rotation 0, so `inf * 0.0` is NaN on an axis the sort does not even use.
    fn block_at(x: f32, y: f32, mcid: u32, text: &str) -> TextBlock {
        let ch = crate::layout::TextChar {
            char: text.chars().next().unwrap_or('x'),
            bbox: Rect::new(x, y, 10.0, 10.0),
            font_name: "Test".to_string(),
            font_size: 10.0,
            font_weight: FontWeight::default(),
            is_italic: false,
            is_monospace: false,
            color: Color::default(),
            mcid: Some(mcid),
            origin_x: x,
            origin_y: y,
            rotation_degrees: 0.0,
            advance_width: 10.0,
            rendered_advance: 10.0,
            ascent: 8.0,
            descent: -2.0,
            matrix: None,
        };
        let mut block = TextBlock::from_chars(vec![ch]);
        block.bbox = Rect::new(x, y, 10.0, 10.0);
        block.text = text.to_string();
        block.mcid = Some(mcid);
        block
    }

    /// `sort_by` panics on a comparator that is not a total order, and NaN
    /// compares equal to everything. This class of panic has been fixed twice
    /// already; the cell sorts must go through `utils::safe_float_cmp`.
    #[test]
    fn cell_blocks_with_non_finite_bbox_do_not_panic() {
        // Enough blocks that `sort_by` takes its merge path: Rust's total-order
        // violation check is best-effort, and a three-element slice sorts by
        // insertion without ever comparing the offending pair. ~keep
        const N: u32 = 24;
        let children: Vec<StructChild> = (0..N)
            .map(|mcid| StructChild::MarkedContentRef {
                mcid,
                page: 0,
                scope: crate::structure::types::McidScope::Page(0),
            })
            .collect();
        let cell = StructElem {
            struct_type: StructType::TD,
            children,
            page: Some(0),
            attributes: std::collections::HashMap::new(),
            alt_text: None,
            expansion: None,
            actual_text: None,
            source_role: None,
        };
        // Two non-finite x coordinates scattered through the run. `perp`
        // multiplies x by a sine that is exactly 0.0 at rotation 0, so each
        // yields NaN on the axis the sort does not even use. ~keep
        let blocks: Vec<TextBlock> = (0..N)
            .map(|i| {
                // Four lines of six: y selects the line, x the position within
                // it. The perp sort decides line membership, so a NaN there
                // changes which line a block lands on. ~keep
                let x = if i == 7 || i == 17 {
                    f32::INFINITY
                } else {
                    10.0 * (i % 6) as f32
                };
                let text = ((b'a' + i as u8) as char).to_string();
                block_at(x, 20.0 + 40.0 * (i / 6) as f32, i, &text)
            })
            .collect();

        // The assertion is that this returns at all: with `partial_cmp(..)
        // .unwrap_or(Equal)` the NaN breaks transitivity and sort_by panics. ~keep
        let cell = extract_cell(&cell, &blocks, false).expect("cell extraction");
        assert!(
            cell.text.contains('a') && cell.text.contains('x'),
            "finite blocks must survive a non-finite sibling, got {:?}",
            cell.text
        );
    }

    #[test]
    fn test_table_new() {
        let table = Table::new();
        assert!(table.is_empty());
        assert_eq!(table.col_count, 0);
        assert!(!table.has_header);
        assert!(table.bbox.is_none());
    }

    #[test]
    fn test_table_bbox() {
        let mut table = Table::new();
        assert!(table.bbox.is_none());

        table.bbox = Some(Rect::new(10.0, 20.0, 100.0, 50.0));
        assert!(table.bbox.is_some());
        let bbox = table.bbox.unwrap();
        assert_eq!(bbox.x, 10.0);
        assert_eq!(bbox.y, 20.0);
        assert_eq!(bbox.width, 100.0);
        assert_eq!(bbox.height, 50.0);
    }

    #[test]
    fn test_table_row_new() {
        let header_row = TableRow::new(true);
        assert!(header_row.is_header);
        assert!(header_row.cells.is_empty());

        let body_row = TableRow::new(false);
        assert!(!body_row.is_header);
    }

    #[test]
    fn test_table_cell_new() {
        let cell = TableCell::new("Hello".to_string(), false);
        assert_eq!(cell.text, "Hello");
        assert!(!cell.is_header);
        assert_eq!(cell.colspan, 1);
        assert_eq!(cell.rowspan, 1);
        assert!(cell.mcids.is_empty());
    }

    #[test]
    fn test_table_cell_with_spans() {
        let cell = TableCell::new("Data".to_string(), false)
            .with_colspan(2)
            .with_rowspan(3);

        assert_eq!(cell.colspan, 2);
        assert_eq!(cell.rowspan, 3);
    }

    #[test]
    fn test_table_cell_header() {
        let cell = TableCell::new("Header".to_string(), true);
        assert!(cell.is_header);
    }

    #[test]
    fn test_table_row_add_cells() {
        let mut row = TableRow::new(false);
        row.add_cell(TableCell::new("Cell1".to_string(), false));
        row.add_cell(TableCell::new("Cell2".to_string(), false));

        assert_eq!(row.cells.len(), 2);
        assert_eq!(row.cells[0].text, "Cell1");
        assert_eq!(row.cells[1].text, "Cell2");
    }

    #[test]
    fn test_table_add_rows() {
        let mut table = Table::new();
        let mut row1 = TableRow::new(false);
        row1.add_cell(TableCell::new("A".to_string(), false));
        row1.add_cell(TableCell::new("B".to_string(), false));

        table.add_row(row1);
        assert_eq!(table.col_count, 2);
        assert_eq!(table.rows.len(), 1);
    }

    #[test]
    fn test_table_has_header() {
        let mut table = Table::new();
        assert!(!table.has_header);

        table.has_header = true;
        assert!(table.has_header);
    }

    /// Helper: create a minimal Table StructElem with MarkedContentRefs on a given page
    fn make_table_elem(page: u32, mcids: &[u32]) -> StructElem {
        let mut table = StructElem::new(StructType::Table);
        let mut tr = StructElem::new(StructType::TR);
        for &mcid in mcids {
            let mut td = StructElem::new(StructType::TD);
            td.add_child(StructChild::MarkedContentRef {
                mcid,
                page,
                scope: crate::structure::McidScope::Page(page),
            });
            tr.add_child(StructChild::StructElem(Box::new(td)));
        }
        table.add_child(StructChild::StructElem(Box::new(tr)));
        table
    }

    #[test]
    fn test_find_table_elements_finds_table_on_matching_page() {
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(make_table_elem(0, &[1, 2]));

        let tables = find_table_elements(&tree, 0);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].struct_type, StructType::Table);
    }

    #[test]
    fn test_find_table_elements_skips_table_on_different_page() {
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(make_table_elem(1, &[1, 2]));

        let tables = find_table_elements(&tree, 0);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_find_table_elements_empty_tree() {
        let tree = StructTreeRoot::new();
        let tables = find_table_elements(&tree, 0);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_find_table_elements_multiple_tables() {
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(make_table_elem(0, &[1, 2]));
        tree.add_root_element(make_table_elem(0, &[3, 4]));

        let tables = find_table_elements(&tree, 0);
        assert_eq!(tables.len(), 2);
    }

    #[test]
    fn test_find_table_elements_nested_in_section() {
        let mut tree = StructTreeRoot::new();
        let mut sect = StructElem::new(StructType::Sect);
        sect.add_child(StructChild::StructElem(Box::new(make_table_elem(0, &[1]))));
        tree.add_root_element(sect);

        let tables = find_table_elements(&tree, 0);
        assert_eq!(tables.len(), 1);
    }

    #[test]
    fn test_find_table_elements_table_with_page_attribute() {
        let mut tree = StructTreeRoot::new();
        let mut table = StructElem::new(StructType::Table);
        table.page = Some(2);
        tree.add_root_element(table);

        let tables = find_table_elements(&tree, 2);
        assert_eq!(tables.len(), 1);
    }

    #[test]
    fn test_find_table_elements_mixed_pages() {
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(make_table_elem(0, &[1]));
        tree.add_root_element(make_table_elem(1, &[2]));
        tree.add_root_element(make_table_elem(0, &[3]));

        let page0_tables = find_table_elements(&tree, 0);
        assert_eq!(page0_tables.len(), 2);

        let page1_tables = find_table_elements(&tree, 1);
        assert_eq!(page1_tables.len(), 1);
    }

    /// The single-walk all-pages bucketing must match, per page, the per-page
    /// `find_table_elements` walk it replaces.
    #[test]
    fn test_find_table_elements_all_pages_matches_per_page() {
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(make_table_elem(0, &[1, 2]));
        let mut sect = StructElem::new(StructType::Sect);
        sect.add_child(StructChild::StructElem(Box::new(make_table_elem(0, &[3]))));
        tree.add_root_element(sect);
        tree.add_root_element(make_table_elem(1, &[4]));
        let mut page_attr_table = StructElem::new(StructType::Table);
        page_attr_table.page = Some(2);
        tree.add_root_element(page_attr_table);

        let all = find_table_elements_all_pages(&tree);
        for page in 0..4u32 {
            let per_page = find_table_elements(&tree, page);
            let bucket = all.get(&page).cloned().unwrap_or_default();
            assert_eq!(
                bucket.len(),
                per_page.len(),
                "page {page}: bucket count must match per-page walk"
            );
            for (b, p) in bucket.iter().zip(per_page.iter()) {
                // same DFS pre-order ⇒ structurally identical elements ~keep
                assert_eq!(b.struct_type, p.struct_type);
                assert_eq!(b.page, p.page);
                assert_eq!(b.children.len(), p.children.len());
            }
        }
    }

    #[test]
    fn test_find_table_elements_all_pages_empty_tree() {
        let tree = StructTreeRoot::new();
        assert!(find_table_elements_all_pages(&tree).is_empty());
    }

    #[test]
    fn test_element_has_page_content_via_mcid() {
        let mut elem = StructElem::new(StructType::P);
        elem.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 3,
            scope: crate::structure::McidScope::Page(3),
        });

        assert!(element_has_page_content(&elem, 3));
        assert!(!element_has_page_content(&elem, 0));
    }

    #[test]
    fn test_element_has_page_content_via_page_attribute() {
        let mut elem = StructElem::new(StructType::P);
        elem.page = Some(5);

        assert!(element_has_page_content(&elem, 5));
        assert!(!element_has_page_content(&elem, 0));
    }

    #[test]
    fn test_element_has_page_content_recursive() {
        let mut parent = StructElem::new(StructType::Sect);
        let mut child = StructElem::new(StructType::P);
        child.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 2,
            scope: crate::structure::McidScope::Page(2),
        });
        parent.add_child(StructChild::StructElem(Box::new(child)));

        assert!(element_has_page_content(&parent, 2));
        assert!(!element_has_page_content(&parent, 0));
    }

    #[test]
    fn test_element_has_page_content_empty() {
        let elem = StructElem::new(StructType::P);
        assert!(!element_has_page_content(&elem, 0));
    }

    #[test]
    fn test_element_has_page_content_object_ref_ignored() {
        let mut elem = StructElem::new(StructType::P);
        elem.add_child(StructChild::ObjectRef(1, 0));
        assert!(!element_has_page_content(&elem, 0));
    }

    fn make_text_span(text: &str, mcid: Option<u32>) -> crate::layout::TextSpan {
        use crate::layout::text_block::{Color, FontWeight};

        crate::layout::TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: Rect::new(0.0, 0.0, 50.0, 12.0),
            font_name: "Test".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::black(),
            mcid,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        }
    }

    #[test]
    fn test_extract_table_from_spans_basic() {
        let mut table_elem = StructElem::new(StructType::Table);
        let mut tr = StructElem::new(StructType::TR);
        let mut td1 = StructElem::new(StructType::TD);
        td1.add_child(StructChild::MarkedContentRef {
            mcid: 10,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut td2 = StructElem::new(StructType::TD);
        td2.add_child(StructChild::MarkedContentRef {
            mcid: 11,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        tr.add_child(StructChild::StructElem(Box::new(td1)));
        tr.add_child(StructChild::StructElem(Box::new(td2)));
        table_elem.add_child(StructChild::StructElem(Box::new(tr)));

        let spans = vec![
            make_text_span("Hello", Some(10)),
            make_text_span("World", Some(11)),
            make_text_span("Unrelated", Some(99)),
        ];

        let result = extract_table_from_spans(&table_elem, &spans).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].cells.len(), 2);
        assert_eq!(result.rows[0].cells[0].text, "Hello");
        assert_eq!(result.rows[0].cells[1].text, "World");
    }

    #[test]
    fn test_extract_table_from_spans_no_matching_mcids() {
        let mut table_elem = StructElem::new(StructType::Table);
        let mut tr = StructElem::new(StructType::TR);
        let mut td = StructElem::new(StructType::TD);
        td.add_child(StructChild::MarkedContentRef {
            mcid: 10,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        tr.add_child(StructChild::StructElem(Box::new(td)));
        table_elem.add_child(StructChild::StructElem(Box::new(tr)));

        let spans = vec![make_text_span("Other", Some(99))];

        let result = extract_table_from_spans(&table_elem, &spans).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].cells[0].text, "");
    }

    #[test]
    fn test_extract_table_from_spans_filters_no_mcid_spans() {
        let mut table_elem = StructElem::new(StructType::Table);
        let mut tr = StructElem::new(StructType::TR);
        let mut td = StructElem::new(StructType::TD);
        td.add_child(StructChild::MarkedContentRef {
            mcid: 5,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        tr.add_child(StructChild::StructElem(Box::new(td)));
        table_elem.add_child(StructChild::StructElem(Box::new(tr)));

        let spans = vec![make_text_span("No MCID", None), make_text_span("Has MCID", Some(5))];

        let result = extract_table_from_spans(&table_elem, &spans).unwrap();
        assert_eq!(result.rows[0].cells[0].text, "Has MCID");
    }

    #[test]
    fn test_extract_table_from_spans_with_thead() {
        let mut table_elem = StructElem::new(StructType::Table);

        let mut thead = StructElem::new(StructType::THead);
        let mut hdr_tr = StructElem::new(StructType::TR);
        let mut th = StructElem::new(StructType::TH);
        th.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        hdr_tr.add_child(StructChild::StructElem(Box::new(th)));
        thead.add_child(StructChild::StructElem(Box::new(hdr_tr)));
        table_elem.add_child(StructChild::StructElem(Box::new(thead)));

        let mut tbody = StructElem::new(StructType::TBody);
        let mut body_tr = StructElem::new(StructType::TR);
        let mut td = StructElem::new(StructType::TD);
        td.add_child(StructChild::MarkedContentRef {
            mcid: 2,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        body_tr.add_child(StructChild::StructElem(Box::new(td)));
        tbody.add_child(StructChild::StructElem(Box::new(body_tr)));
        table_elem.add_child(StructChild::StructElem(Box::new(tbody)));

        let spans = vec![make_text_span("Header", Some(1)), make_text_span("Data", Some(2))];

        let result = extract_table_from_spans(&table_elem, &spans).unwrap();
        assert!(result.has_header);
        assert_eq!(result.rows.len(), 2);
        assert!(result.rows[0].is_header);
        assert!(!result.rows[1].is_header);
        assert_eq!(result.rows[0].cells[0].text, "Header");
        assert_eq!(result.rows[1].cells[0].text, "Data");
    }

    #[test]
    fn test_extract_table_from_spans_empty_table() {
        let table_elem = StructElem::new(StructType::Table);
        let spans: Vec<crate::layout::TextSpan> = vec![];

        let result = extract_table_from_spans(&table_elem, &spans).unwrap();
        assert!(result.is_empty());
    }

    /// Adjacent MCID spans (gap ≤ 0) must NOT
    /// have a space inserted between them.  The PDF stores e.g. "Q" (MCID 1) and "（"
    /// (MCID 2) as separate marked-content runs that abut each other on the same line.
    /// Before the fix, extract_cell always inserted a space between any two MCID blocks,
    /// producing "Q （peu/d）" instead of the correct "Q（peu/d）".
    #[test]
    fn test_extract_cell_adjacent_mcid_spans_no_space() {
        use crate::layout::text_block::{Color, FontWeight};

        let mut td = StructElem::new(StructType::TD);
        td.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        td.add_child(StructChild::MarkedContentRef {
            mcid: 2,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        td.add_child(StructChild::MarkedContentRef {
            mcid: 3,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut tr = StructElem::new(StructType::TR);
        tr.add_child(StructChild::StructElem(Box::new(td)));
        let mut table_elem = StructElem::new(StructType::Table);
        table_elem.add_child(StructChild::StructElem(Box::new(tr)));

        // Exact coordinates for the Q（peu/d） column header case:
        //   "Q"     x=345.79 w=8.22  end=354.01
        //   "（"    x=353.83 w=10.56 end=364.39   gap=-0.18 (overlap → no space)
        //   "peu/d" x=364.39 w=25.24             gap=0.00  (touching → no space) ~keep
        let base = crate::layout::TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: String::new(),
            bbox: Rect::new(0.0, 678.0, 0.0, 10.56),
            font_name: "Test".to_string(),
            font_size: 10.56,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        };
        let spans = vec![
            crate::layout::TextSpan {
                text: "Q".into(),
                bbox: Rect::new(345.79, 678.0, 8.22, 10.56),
                mcid: Some(1),
                mcid_scope: None,
                ..base.clone()
            },
            crate::layout::TextSpan {
                text: "（".into(),
                bbox: Rect::new(353.83, 678.0, 10.56, 10.56),
                mcid: Some(2),
                mcid_scope: None,
                ..base.clone()
            },
            crate::layout::TextSpan {
                text: "peu/d".into(),
                bbox: Rect::new(364.39, 678.0, 25.24, 10.56),
                mcid: Some(3),
                mcid_scope: None,
                ..base.clone()
            },
        ];

        let result = extract_table_from_spans(&table_elem, &spans).unwrap();
        assert_eq!(
            result.rows[0].cells[0].text, "Q（peu/d",
            "adjacent MCID spans must not get a space inserted between them"
        );
    }

    /// Companion test: MCID spans on different lines (multi-line cell) DO get a space.
    #[test]
    fn test_extract_cell_multiline_mcid_spans_have_space() {
        let mut td = StructElem::new(StructType::TD);
        td.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        td.add_child(StructChild::MarkedContentRef {
            mcid: 2,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut tr = StructElem::new(StructType::TR);
        tr.add_child(StructChild::StructElem(Box::new(td)));
        let mut table_elem = StructElem::new(StructType::Table);
        table_elem.add_child(StructChild::StructElem(Box::new(tr)));

        let base = crate::layout::TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: String::new(),
            bbox: Rect::new(0.0, 0.0, 0.0, 12.0),
            font_name: "Test".to_string(),
            font_size: 12.0,
            font_weight: crate::layout::text_block::FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: crate::layout::text_block::Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        };
        // Line 1: "Hello" ends at x=100, y=200.  Line 2: "World" starts at x=10, y=188.
        // y_diff = 12 > line_h * 0.5 = 6 → different lines → space inserted. ~keep
        let spans = vec![
            crate::layout::TextSpan {
                text: "Hello".into(),
                bbox: Rect::new(10.0, 200.0, 90.0, 12.0),
                mcid: Some(1),
                mcid_scope: None,
                ..base.clone()
            },
            crate::layout::TextSpan {
                text: "World".into(),
                bbox: Rect::new(10.0, 188.0, 90.0, 12.0),
                mcid: Some(2),
                mcid_scope: None,
                ..base.clone()
            },
        ];

        let result = extract_table_from_spans(&table_elem, &spans).unwrap();
        assert_eq!(result.rows[0].cells[0].text, "Hello World");
    }

    /// The synthesized `cell.spans` on the tagged-PDF (MCID→TextBlock) path must
    /// carry per-block `font_weight`/`is_italic`, otherwise the markdown/HTML
    /// table renderers can't emit bold/italic markers and silently fall back to
    /// plain text. Also asserts the inter-line space is carried into the span
    /// text so renderers reconstructing from spans don't glue tokens across a
    /// wrapped line.
    #[test]
    fn test_extract_cell_spans_carry_bold_italic_and_spacing() {
        use crate::layout::text_block::{Color, FontWeight};

        let mut td = StructElem::new(StructType::TD);
        td.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        td.add_child(StructChild::MarkedContentRef {
            mcid: 2,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut tr = StructElem::new(StructType::TR);
        tr.add_child(StructChild::StructElem(Box::new(td)));
        let mut table_elem = StructElem::new(StructType::Table);
        table_elem.add_child(StructChild::StructElem(Box::new(tr)));

        let base = crate::layout::TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: String::new(),
            bbox: Rect::new(0.0, 0.0, 0.0, 12.0),
            font_name: "Test".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        };
        let spans = vec![
            crate::layout::TextSpan {
                text: "Bold".into(),
                bbox: Rect::new(10.0, 200.0, 40.0, 12.0),
                font_weight: FontWeight::Bold,
                mcid: Some(1),
                mcid_scope: None,
                ..base.clone()
            },
            crate::layout::TextSpan {
                text: "Italic".into(),
                bbox: Rect::new(10.0, 188.0, 40.0, 12.0),
                is_italic: true,
                mcid: Some(2),
                mcid_scope: None,
                ..base.clone()
            },
        ];

        let result = extract_table_from_spans(&table_elem, &spans).unwrap();
        let cell = &result.rows[0].cells[0];
        assert_eq!(cell.spans.len(), 2, "both MCID blocks must yield a span");
        assert_eq!(cell.spans[0].text, "Bold");
        assert!(
            matches!(cell.spans[0].font_weight, FontWeight::Bold),
            "bold block must propagate FontWeight::Bold into the synthesized span"
        );
        assert!(!cell.spans[0].is_italic, "non-italic block must not be italic");
        assert!(
            matches!(cell.spans[1].font_weight, FontWeight::Normal),
            "non-bold block must stay FontWeight::Normal"
        );
        assert!(
            cell.spans[1].is_italic,
            "italic block must propagate is_italic into the synthesized span"
        );
        assert_eq!(
            cell.spans[1].text, " Italic",
            "wrapped-line span must carry the leading inter-block space (review #533)"
        );
    }

    /// A figure a producer split across two marked-content runs must keep its
    /// own left-to-right order inside a right-to-left cell (UAX #9 class
    /// EN/AN); only the surrounding Arabic reverses. Blanket-reversing the
    /// whole cell reads "47.500" back as "7.5004", which changes the value
    /// rather than merely re-spacing it.
    #[test]
    fn rtl_table_cell_numeric_run_keeps_ltr_order() {
        // Visual layout, left to right on one line:
        //   تن   |  "4" "7.500"  |   حداقل
        //   x=5     x=30  x=37       x=75
        // Reading order is right to left, so حداقل comes first and تن last,
        // but the two numeral runs read "4" then "7.500" between them. ~keep
        let cell = rtl_cell_from(&[
            ("تن", 5.0, 20.0),
            ("4", 30.0, 7.0),
            ("7.500", 37.0, 33.0),
            ("حداقل", 75.0, 35.0),
        ]);

        assert!(
            cell.text.contains("47.500"),
            "the split figure must survive intact; got {:?}",
            cell.text
        );
        assert!(
            !cell.text.contains("7.5004"),
            "numerals must not inherit the surrounding script's reversal; got {:?}",
            cell.text
        );
        let arabic_first = cell.text.find("حداقل").expect("Arabic run present");
        let arabic_last = cell.text.find("تن").expect("Arabic run present");
        assert!(
            arabic_first < arabic_last,
            "the surrounding Arabic must still read right to left; got {:?}",
            cell.text
        );
    }

    /// The un-reversal is scoped to numerals: an all-Arabic cell keeps the
    /// plain right-to-left order the blanket reversal already produced.
    #[test]
    fn rtl_table_cell_without_numerals_is_unchanged() {
        let cell = rtl_cell_from(&[("تن", 5.0, 20.0), ("حداقل", 75.0, 35.0)]);
        assert_eq!(
            cell.text, "حداقل تن",
            "a cell with no numerals must read purely right to left"
        );
    }

    /// Build a single-cell tagged table from `(text, x, width)` runs laid out
    /// on one line, and return the extracted cell.
    fn rtl_cell_from(runs: &[(&str, f32, f32)]) -> crate::structure::TableCell {
        use crate::layout::text_block::{Color, FontWeight};

        let mut td = StructElem::new(StructType::TD);
        let mut spans = Vec::new();
        for (index, (text, x, width)) in runs.iter().enumerate() {
            let mcid = index as u32 + 1;
            td.add_child(StructChild::MarkedContentRef {
                mcid,
                page: 0,
                scope: crate::structure::McidScope::Page(0),
            });
            spans.push(crate::layout::TextSpan {
                provenance: None,
                text_rise: 0.0,
                artifact_type: None,
                text: (*text).into(),
                bbox: Rect::new(*x, 200.0, *width, 12.0),
                font_name: "Test".to_string(),
                font_size: 12.0,
                font_weight: FontWeight::Normal,
                is_italic: false,
                is_monospace: false,
                color: Color::black(),
                mcid: Some(mcid),
                mcid_scope: None,
                sequence: index,
                split_boundary_before: false,
                offset_semantic: false,
                char_spacing: 0.0,
                word_spacing: 0.0,
                horizontal_scaling: 1.0,
                primary_detected: false,
                char_widths: vec![],
                char_x_offsets: Vec::new(),
                heading_level: None,
                rotation_degrees: 0.0,
                wmode: 0,
                rtl_draw_logical: false,
                ligature_glyph: false,
                mirrored: false,
                page_rotation_applied: 0,
            });
        }
        let mut tr = StructElem::new(StructType::TR);
        tr.add_child(StructChild::StructElem(Box::new(td)));
        let mut table_elem = StructElem::new(StructType::Table);
        table_elem.add_child(StructChild::StructElem(Box::new(tr)));

        let result = extract_table_from_spans(&table_elem, &spans).unwrap();
        result.rows[0].cells[0].clone()
    }

    #[test]
    fn test_extract_cell_spans_carry_rotation_degrees() {
        use crate::layout::text_block::{Color, FontWeight};

        let mut td = StructElem::new(StructType::TD);
        td.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut tr = StructElem::new(StructType::TR);
        tr.add_child(StructChild::StructElem(Box::new(td)));
        let mut table_elem = StructElem::new(StructType::Table);
        table_elem.add_child(StructChild::StructElem(Box::new(tr)));

        let span = crate::layout::TextSpan {
            provenance: None,
            text_rise: 0.0,
            mirrored: false,
            page_rotation_applied: 0,
            artifact_type: None,
            text: "Rotated".into(),
            bbox: Rect::new(10.0, 200.0, 40.0, 12.0),
            font_name: "Test".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::black(),
            mcid: Some(1),
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 90.0,
            wmode: 0,
            rtl_draw_logical: false,
            ligature_glyph: false,
        };

        let result = extract_table_from_spans(&table_elem, &[span]).unwrap();
        let cell = &result.rows[0].cells[0];
        assert_eq!(cell.spans.len(), 1);
        assert_eq!(
            cell.spans[0].rotation_degrees, 90.0,
            "cell span re-synthesis must carry the source block's rotation through, \
             not silently reset it to 0.0"
        );
    }

    /// CJK + fullwidth operator with a gap that *exceeds* the 0.15em threshold must
    /// still suppress space insertion — this exercises the new CJK-suppression branch
    /// (the `test_extract_cell_adjacent_mcid_spans_no_space` test above only covers
    /// the gap ≤ threshold path, which never reaches this branch).
    #[test]
    fn test_extract_cell_cjk_fullwidth_gap_suppresses_space() {
        use crate::layout::text_block::{Color, FontWeight};

        // Build: TD with three MCIDs: "数" (CJK), "≤" (math op), "量" (CJK)
        // Place them with a gap of 3.0 pt (> font_size * 0.15 = 1.5 for 10 pt font)
        // so the gap branch fires, then the CJK suppression should prevent a space. ~keep
        let mut td = StructElem::new(StructType::TD);
        td.add_child(StructChild::MarkedContentRef {
            mcid: 10,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        td.add_child(StructChild::MarkedContentRef {
            mcid: 11,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        td.add_child(StructChild::MarkedContentRef {
            mcid: 12,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut tr = StructElem::new(StructType::TR);
        tr.add_child(StructChild::StructElem(Box::new(td)));
        let mut table_elem = StructElem::new(StructType::Table);
        table_elem.add_child(StructChild::StructElem(Box::new(tr)));

        let base = crate::layout::TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: String::new(),
            bbox: Rect::new(0.0, 100.0, 0.0, 10.0),
            font_name: "Test".to_string(),
            font_size: 10.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        };
        // "数" ends at x=10+10=20; "≤" starts at x=23 → gap=3.0 > 1.5 → gap branch fires
        // CJK("数")→math_op("≤") with at least one CJK side → suppress space
        // "≤" ends at x=23+10=33; "量" starts at x=36 → gap=3.0 → suppress space ~keep
        let spans = vec![
            crate::layout::TextSpan {
                text: "数".into(),
                bbox: Rect::new(10.0, 100.0, 10.0, 10.0),
                mcid: Some(10),
                mcid_scope: None,
                ..base.clone()
            },
            crate::layout::TextSpan {
                text: "≤".into(),
                bbox: Rect::new(23.0, 100.0, 10.0, 10.0),
                mcid: Some(11),
                mcid_scope: None,
                ..base.clone()
            },
            crate::layout::TextSpan {
                text: "量".into(),
                bbox: Rect::new(36.0, 100.0, 10.0, 10.0),
                mcid: Some(12),
                mcid_scope: None,
                ..base.clone()
            },
        ];

        let result = extract_table_from_spans(&table_elem, &spans).unwrap();
        assert_eq!(
            result.rows[0].cells[0].text, "数≤量",
            "CJK + math-op + CJK with gap > 0.15em should not have spaces inserted: \
             got '{}'",
            result.rows[0].cells[0].text
        );
    }

    /// Counterpart: Latin + Latin with a gap exceeding the threshold MUST insert a space.
    /// This guards that the CJK-suppression branch does not affect non-CJK pairs.
    #[test]
    fn test_extract_cell_latin_gap_inserts_space() {
        use crate::layout::text_block::{Color, FontWeight};

        let mut td = StructElem::new(StructType::TD);
        td.add_child(StructChild::MarkedContentRef {
            mcid: 20,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        td.add_child(StructChild::MarkedContentRef {
            mcid: 21,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut tr = StructElem::new(StructType::TR);
        tr.add_child(StructChild::StructElem(Box::new(td)));
        let mut table_elem = StructElem::new(StructType::Table);
        table_elem.add_child(StructChild::StructElem(Box::new(tr)));

        let base = crate::layout::TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: String::new(),
            bbox: Rect::new(0.0, 100.0, 0.0, 10.0),
            font_name: "Test".to_string(),
            font_size: 10.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        };
        // "Hello" ends at 50; "world" starts at 53 → gap=3.0 > 1.5 → space inserted
        // Neither side is CJK, so the CJK suppression must NOT fire. ~keep
        let spans = vec![
            crate::layout::TextSpan {
                text: "Hello".into(),
                bbox: Rect::new(0.0, 100.0, 50.0, 10.0),
                mcid: Some(20),
                mcid_scope: None,
                ..base.clone()
            },
            crate::layout::TextSpan {
                text: "world".into(),
                bbox: Rect::new(53.0, 100.0, 30.0, 10.0),
                mcid: Some(21),
                mcid_scope: None,
                ..base.clone()
            },
        ];

        let result = extract_table_from_spans(&table_elem, &spans).unwrap();
        assert_eq!(
            result.rows[0].cells[0].text, "Hello world",
            "Latin→Latin with gap > 0.15em should insert a space: got '{}'",
            result.rows[0].cells[0].text
        );
    }
}

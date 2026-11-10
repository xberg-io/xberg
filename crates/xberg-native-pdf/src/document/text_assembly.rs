//! Plain-text assembly from ordered spans.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Circular references and recursion limit errors are handled gracefully
    /// with warning messages in the output.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// let text = doc.extract_text(0)?;
    /// println!("Page 1 text: {}", text);
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    ///
    /// # Extract text from a page
    ///
    /// xberg-native-pdf exposes three plain-text surfaces with different strengths.
    /// Pick by document shape:
    ///
    /// - `extract_text(page)` (this method) — glyph-walk assembly with
    ///   row-aware ordering, inline table rendering, and artifact filtering.
    ///   The most discoverable default; strongest on single-column prose.
    /// - `to_plain_text(page, opts)` / `to_plain_text_all(opts)` — runs the
    ///   full pipeline (reading-order strategy incl. XY-cut). Best on
    ///   multi-column / complex layouts where reading order matters.
    /// - `to_markdown_all(opts)` then strip markup — preserves structure
    ///   (headings, lists, tables) and often scores highest on heavily
    ///   structured documents; lossiest for pure prose.
    ///
    /// No single mode wins on every PDF; when extraction quality is critical
    /// and the layout is unknown, compare `to_plain_text_all` and
    /// markdown-stripped output and keep whichever is better for your corpus.
    #[tracing::instrument(name = "pdf.extract_text", skip_all, fields(page = page_index))]
    pub fn extract_text(&self, page_index: usize) -> Result<String> {
        let options = crate::converters::ConversionOptions {
            extract_tables: true,
            ..Default::default()
        };
        let result = self.extract_text_with_options(page_index, &options);
        if let Err(error) = &result {
            crate::error::trace_failure("extract_text", error);
        }
        result
    }

    /// Extract text from a page with specific options.
    pub fn extract_text_with_options(
        &self,
        page_index: usize,
        options: &crate::converters::ConversionOptions,
    ) -> Result<String> {
        let base_spans = self.extract_spans(page_index)?;
        // Vertical CJK (tategaki, ISO 32000-1 §9.7.4.3 vertical writing mode):
        // glyphs run top-to-bottom in columns that progress right-to-left, so
        // the horizontal row-major assembler shreds the reading order. When the
        // page is geometrically vertical, read it column-major instead. ~keep
        if let Some(vertical) = Self::try_assemble_vertical_cjk(&base_spans) {
            return Ok(vertical);
        }
        let text = self.assemble_text_from_spans(page_index, base_spans, options)?;
        Ok(Self::apply_mixed_rtl_line_pass(text))
    }

    /// [`Self::map_dominant_rotation_into_reading_frame`] with the
    /// mapped/unchanged distinction collapsed, for the call sites that only
    /// want "spans as a reader sees them".
    ///
    /// Every library text surface goes through this — via
    /// [`Self::assemble_text_from_spans`] or the converter pipelines. Applying
    /// the frame at one call site made the same page read correctly through
    /// `extract_text` and incorrectly through `to_markdown` / `to_html` /
    /// `to_plain_text`. Callers must apply `ConversionOptions` region filters
    /// BEFORE this: region rects are page-space coordinates, and the map
    /// rewrites the geometry they select against.
    fn spans_in_reading_frame(
        &self,
        page_index: usize,
        spans: Vec<crate::layout::TextSpan>,
    ) -> Vec<crate::layout::TextSpan> {
        match self.map_dominant_rotation_into_reading_frame(page_index, spans) {
            Ok(mapped) => mapped,
            Err(original) => original,
        }
    }

    /// Map a dominant-rotation page's spans into their rotated reading
    /// frame so the standard horizontal assembler applies.
    ///
    /// Returns `Ok(mapped)` when the page is unrotated (`/Rotate 0` — on
    /// rotated pages `postprocess_spans` already handles content rotation,
    ///) and at least half its non-whitespace spans share one
    /// quadrant text rotation; the mapped spans are horizontal in the
    /// frame a reader turns the page into, with `rotation_degrees` cleared
    /// so downstream passes treat them as the upright text they now are.
    /// Returns `Err(spans)` — the input unchanged — on every other page,
    /// keeping output byte-identical there.
    ///
    /// Only used for plain-text assembly, where no coordinates leak to the
    /// caller (region rects leak IN, so they are filtered out before this
    /// runs); coordinate-bearing APIs (`extract_words`) reorder in the
    /// rotated frame but report true page-space bboxes instead (see
    /// `crate::pipeline::page_reading_order`).
    fn map_dominant_rotation_into_reading_frame(
        &self,
        page_index: usize,
        spans: Vec<crate::layout::TextSpan>,
    ) -> std::result::Result<Vec<crate::layout::TextSpan>, Vec<crate::layout::TextSpan>> {
        if self.get_page_rotation(page_index).unwrap_or(0) != 0 {
            return Err(spans);
        }
        let Some(deg) = crate::utils::dominant_rotation(&spans) else {
            return Err(spans);
        };
        // Same quadrant mapping as the word path: 90° text reads upright
        // under a /Rotate-90-style display transform, -90° under 270,
        // 180° under 180. Mirrored / free-angle runs have no frame. ~keep
        let rot = if (deg - 90.0).abs() < 0.5 {
            90
        } else if (deg - 180.0).abs() < 0.5 {
            180
        } else if (deg + 90.0).abs() < 0.5 {
            270
        } else {
            return Err(spans);
        };
        tracing::debug!(target: LOG_TARGET, "page {page_index}: dominant text rotation {deg}° — assembling text in rotated frame");
        let (llx, lly, urx, ury) = self.get_page_media_box(page_index).unwrap_or((0.0, 0.0, 612.0, 792.0));
        let (w, h) = (urx - llx, ury - lly);
        let mut spans = spans;
        // Rotated spans store TEXT-LOCAL extents (origin + advance-along-
        // the-run as `width` + font size as `height`): rotate the ORIGIN
        // as a point and keep the extents, which already describe the run
        // in its own upright frame (same convention as
        // `order_rotated_blocks`). ~keep
        for s in &mut spans {
            let (rx, ry) = (s.bbox.x - llx, s.bbox.y - lly);
            let (mx, my) = match rot {
                90 => (ry, w - rx),
                180 => (w - rx, h - ry),
                270 => (h - ry, rx),
                _ => (rx, ry),
            };
            s.bbox.x = llx + mx;
            s.bbox.y = lly + my;
            s.rotation_degrees = 0.0;
        }
        Ok(spans)
    }

    /// Assemble page text from the page's native spans **plus** caller-supplied
    /// extra spans, positioned together in a single reading-order pass.
    ///
    /// The Auto extractor uses this to drop text recovered from an image region
    /// (via OCR) into the page at the image's spatial location — so a chart
    /// caption embedded in a figure reads in its correct place rather than being
    /// appended after the whole page. The native spans are extracted and
    /// assembled exactly as [`extract_text_with_options`](Self::extract_text_with_options)
    /// would, so the native text is byte-for-byte preserved; the extra spans
    /// only add content, sorted in by their bounding box.
    // Only the unit test that pins span placement calls this, so it is
    // compiled only under `cfg(test)` (no dead code). ~keep
    #[cfg(test)]
    pub(crate) fn extract_text_with_extra_spans(
        &self,
        page_index: usize,
        extra: Vec<crate::layout::TextSpan>,
        options: &crate::converters::ConversionOptions,
    ) -> Result<String> {
        let mut base_spans = self.extract_spans(page_index)?;
        base_spans.extend(extra);
        let text = self.assemble_text_from_spans(page_index, base_spans, options)?;
        Ok(Self::apply_mixed_rtl_line_pass(text))
    }

    /// Returns column-major text when the page is a vertical-CJK (tategaki)
    /// layout, or `None` for every other page (so horizontal documents are
    /// byte-for-byte unchanged).
    ///
    /// Detection is purely geometric: among CJK glyph spans, count how many
    /// neighbour pairs are stacked *vertically* (same column, one glyph-height
    /// apart) versus *horizontally* (same row, one glyph-width apart). Vertical
    /// writing is declared only when CJK is the clear majority of the page and
    /// vertical adjacencies dominate horizontal ones — so horizontal CJK
    /// (Chinese/Japanese prose set left-to-right) never triggers it. Assembly
    /// then orders spans by column right-to-left (X descending, banded to the
    /// glyph width) and top-to-bottom within a column (Y descending), matching
    /// how the script is read.
    pub(super) fn try_assemble_vertical_cjk(spans: &[TextSpan]) -> Option<String> {
        fn is_cjk(c: char) -> bool {
            matches!(
                c as u32,
                0x3040..=0x30FF
                | 0x3400..=0x4DBF
                | 0x4E00..=0x9FFF
                | 0xF900..=0xFAFF
                | 0xFF66..=0xFF9F
            )
        }
        let cjk: Vec<&TextSpan> = spans.iter().filter(|s| s.text.chars().any(is_cjk)).collect();
        if cjk.len() < 8 {
            return None;
        }
        let single_glyph_cjk = cjk
            .iter()
            .filter(|s| {
                let t = s.text.trim();
                t.chars().count() == 1 && t.chars().all(is_cjk)
            })
            .count();
        if single_glyph_cjk * 2 < cjk.len() {
            return None;
        }
        let total_chars: usize = spans
            .iter()
            .map(|s| s.text.chars().filter(|c| !c.is_whitespace()).count())
            .sum();
        let cjk_chars: usize = cjk.iter().map(|s| s.text.chars().filter(|c| is_cjk(*c)).count()).sum();
        if total_chars == 0 || cjk_chars * 2 < total_chars {
            return None;
        }

        let mut widths: Vec<f32> = cjk.iter().map(|s| s.bbox.width).filter(|w| *w > 0.0).collect();
        if widths.is_empty() {
            return None;
        }
        widths.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
        let gw = widths[widths.len() / 2];

        let sample = &cjk[..cjk.len().min(250)];
        let (mut vert, mut horiz) = (0usize, 0usize);
        for (i, a) in sample.iter().enumerate() {
            let (mut best, mut bdx, mut bdy) = (f32::MAX, 0.0f32, 0.0f32);
            for (j, b) in sample.iter().enumerate() {
                if i == j {
                    continue;
                }
                let dx = a.bbox.x - b.bbox.x;
                let dy = a.bbox.y - b.bbox.y;
                let d2 = dx * dx + dy * dy;
                if d2 < best {
                    best = d2;
                    bdx = dx.abs();
                    bdy = dy.abs();
                }
            }
            if bdy > bdx {
                vert += 1;
            } else if bdx > bdy {
                horiz += 1;
            }
        }
        if vert == 0 || vert <= horiz * 2 {
            return None;
        }

        let band = (gw * 0.5).max(1.0);
        let mut ordered: Vec<&TextSpan> = spans.iter().collect();
        ordered.sort_by(|a, b| {
            let ca = (a.bbox.x / band).round() as i32;
            let cb = (b.bbox.x / band).round() as i32;
            cb.cmp(&ca).then(crate::utils::safe_float_cmp(b.bbox.y, a.bbox.y))
        });
        Some(ordered.iter().map(|s| s.text.as_str()).collect())
    }

    /// Per-line UAX #9 pass for mixed-direction lines (bidi item 4): for each
    /// output line that is confidently RTL and mixes Arabic/Hebrew with
    /// European/Arabic-Indic numerals or Latin words (e.g. a date
    /// `14 april 1434 ٤٣٤١`), give the embedded LTR sub-runs their left-to-right
    /// sublevel (UAX #9 §3.3.4) while leaving the already-logical RTL runs fixed.
    /// Gated inside `reorder_mixed_rtl_line`, so pure-RTL, pure-LTR, and
    /// non-RTL lines are returned byte-for-byte unchanged; the ASCII fast path
    /// keeps all Latin-only extraction identical.
    fn apply_mixed_rtl_line_pass(text: String) -> String {
        if text.is_ascii() {
            return text;
        }
        text.split('\n')
            .map(crate::text::bidi::reorder_mixed_rtl_line)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Apply caller-specified region filters to a span set: drop spans matching
    /// any `exclude_regions` (under `exclude_regions_mode`), then keep only spans
    /// inside `include_region` if one is set. Exclusion runs first so it takes
    /// precedence. Shared by the plain-text, markdown, and HTML conversion paths
    /// so `ConversionOptions` region filtering behaves identically across every
    /// text surface. A no-op when neither field is set.
    fn apply_region_filters(
        base_spans: Vec<crate::layout::TextSpan>,
        options: &crate::converters::ConversionOptions,
    ) -> Vec<crate::layout::TextSpan> {
        use crate::layout::SpatialCollectionFiltering;
        let mut spans = base_spans;
        if !options.exclude_regions.is_empty() {
            spans = spans.exclude_rects(&options.exclude_regions, options.exclude_regions_mode);
        }
        if let Some((ref region, mode)) = options.include_region {
            spans = spans.filter_by_rect(region, mode);
        }
        spans
    }

    pub(super) fn assemble_text_from_spans(
        &self,
        page_index: usize,
        base_spans: Vec<crate::layout::TextSpan>,
        options: &crate::converters::ConversionOptions,
    ) -> Result<String> {
        if self.is_encrypted_unreadable() {
            tracing::warn!(target: LOG_TARGET, "PDF is encrypted and could not be decrypted; returning empty text");
            return Ok(String::new());
        }

        let base_spans = Self::apply_region_filters(base_spans, options);
        // Dominant text-matrix rotation (a landscape table typeset on a
        // portrait page): the row-major assembler groups lines in the
        // portrait frame and interleaves every rotated row. Assemble such
        // pages in their rotated reading frame instead — after the region
        // filters, whose rects select in page space, and before table
        // detection, which consumes the geometry this maps. ~keep
        let base_spans = self.spans_in_reading_frame(page_index, base_spans);
        // Struct-tree-scope `/ActualText` is applied per branch below
        // — the structure-order assembler handles it natively via the
        // per-page action map, and the geometric branch applies the
        // raw-span applier on its own input. Pre-applying here would
        // double-process: the structure-order path would see already-
        // mutated spans and lose run-position information, dropping
        // sibling MCIDs of a nested scope (CRITICAL-1 shape). ~keep

        // Structure tree: use it for reading order only when it is trustworthy
        // per the shared predicate (§14.8.2.3.1) — the document is /Marked or
        // the catalog references a /StructTreeRoot (PDF 1.4 documents such as
        // hello_structure.pdf predate /MarkInfo but are still tagged, §14.7.1),
        // AND /MarkInfo /Suspects is not true. Suspect documents fall through to
        // the geometric `else` arm below, the spec-correct behaviour. ~keep
        let cached_tree = self.struct_tree_trustworthy();
        let widget_spans = self.extract_widget_spans(page_index);

        let tables = if options.extract_tables {
            // text_fallback=false: extract_text preserves the prior behaviour
            // where line-less pages return no tables. Only the structured-output
            // converters (to_markdown, to_html) opt in to text-only spatial fallback. ~keep
            self.extract_page_tables(page_index, &base_spans, options, false)
        } else {
            Vec::new()
        };

        let mut all_spans = base_spans;
        all_spans.extend(widget_spans);

        if all_spans.is_empty() {
            let page = self.get_page(page_index)?;
            let page_dict = page.as_dict().ok_or_else(|| Error::ParseError {
                offset: 0,
                reason: "Page is not a dictionary".to_string(),
            })?;
            let no_content_text = if self.page_cannot_have_text(page_dict) {
                true
            } else {
                match self.get_page_content_data(page_index) {
                    Ok(ref content_data) => !Self::may_contain_text(content_data),
                    Err(_) => false, // Can't read content stream — be conservative ~keep
                }
            };
            if no_content_text {
                let mut text = String::new();
                self.append_non_widget_annotation_text(page_index, &mut text);
                return Ok(text);
            }
        }

        let text = if let Some(ref struct_tree) = cached_tree {
            // Build per-page traversal cache once, then O(1) lookup per page. ~keep
            if self.structure_content_cache.lock_or_recover().is_none() {
                let all_content = crate::structure::traverse_structure_tree_all_pages(struct_tree);
                *self.structure_content_cache.lock_or_recover() = Some(all_content);
            }
            self.extract_text_structure_order_cached_with_spans(page_index, all_spans, options.include_artifacts)?
        } else {
            // Untagged or Suspects=true PDF: use page content
            // (geometric) order. Apply struct-tree-scope `/ActualText`
            // here — the structure-order assembler above handles it
            // natively for the trustworthy branch. Suspects=true
            // documents still get their producer-supplied replacement
            // because `actualtext_index()` is decoupled from
            // `struct_tree_marked` (§14.9.4 is content replacement,
            // not a reading-order signal). ~keep
            let mut spans = all_spans;
            self.apply_actualtext_to_spans(page_index, &mut spans);

            // Exclude spans that are inside detected tables, BUT
            // preserve multi-row-spanning label columns.
            // The spatial table extractor clusters data cells into
            // table cells but does NOT emit the sparse label column
            // that sits vertically centred within each multi-row data
            // block (common on CJK lab-report reference tables like
            // WS/T 779). Those labels would otherwise be dropped
            // entirely from the output: the retain below would remove
            // them because their bbox is inside the table,
            // `table.render_text()` would not re-emit them because the
            // extractor never captured them as cells. Before running
            // the retain filter we identify these rowspan labels (same
            // heuristic `reorder_rowspan_labels` uses) and keep them in
            // the span list so `reorder_rowspan_labels` below can
            // promote them to the top of their row block. ~keep
            if !tables.is_empty() {
                // Absorb floating-point accumulation error in the
                // difference between a span's directly-computed
                // bbox.right (origin + width, small accumulation)
                // and a table bbox.right (min/max reduction across
                // many cell edges, larger accumulation). Without
                // this slack, a span whose real geometry is inside
                // the table by construction but whose f32 right-edge
                // exceeds the table's f32 right-edge by ~0.01–0.05pt
                // gets wrongly kept in the flow stream, producing
                // duplicated output. 0.1pt is well below any
                // visually meaningful PDF layout distance. ~keep
                const RETAIN_TOLERANCE: f32 = 0.1;

                // Build the set of cell text strings that every detected
                // table will render via `table.render_text()`. Labels
                // whose exact text already appears as a cell in some
                // table are already covered by the inline-table flush
                // below, so we must NOT also preserve them in the flow
                // span list (it would produce duplicate output). ~keep
                let mut table_cell_texts: std::collections::HashSet<&str> = std::collections::HashSet::new();
                for t in &tables {
                    for row in &t.rows {
                        for cell in &row.cells {
                            let trimmed = cell.text.trim();
                            if !trimmed.is_empty() {
                                table_cell_texts.insert(trimmed);
                            }
                        }
                    }
                }

                // For tagged PDFs, collect the MCIDs that are actually owned by
                // table cells. When a span's MCID is NOT in this set, the span is
                // NOT part of the table even if it lies inside the table's bbox
                // (e.g. a paragraph physically adjacent to a table that was tagged
                // as a sibling <P> element, not as a <TD>). Filtering such spans
                // by bbox alone would silently drop real content.
                // Falls back to bbox-only filtering when no MCIDs are present
                // (untagged PDFs or spatial-detection tables).
                // Only tables with a bbox reach the inline flush below; a
                // cell of an unflushed table renders nowhere and must not
                // absorb anything — whether ownership is decided by MCID or
                // by geometry. ~keep
                let table_cell_mcids: HashSet<u32> = tables
                    .iter()
                    .filter(|t| t.bbox.is_some())
                    .flat_map(|t| {
                        t.rows
                            .iter()
                            .flat_map(|r| r.cells.iter().flat_map(|c| c.mcids.iter().copied()))
                    })
                    .collect();
                // Flatten every flushed cell once (bbox plus the cell, whose
                // text and member spans drive the ownership test below) and
                // index it into coarse y-bands, so the per-span containment
                // test below scans only the cells in the span's y-band
                // instead of every cell on the page (was O(spans x cells) on
                // untagged table pages). A cell that contains a span
                // necessarily shares the span's y-band, so this is
                // byte-identical to the full scan. ~keep
                let flushed_cells: Vec<(crate::geometry::Rect, &crate::structure::table_extractor::TableCell)> = tables
                    .iter()
                    .filter(|t| t.bbox.is_some())
                    .flat_map(|t| {
                        t.rows
                            .iter()
                            .flat_map(|r| r.cells.iter().filter_map(|c| c.bbox.map(|b| (b, c))))
                    })
                    .collect();
                // Same-text cell lookup for the fallback path: a span that no
                // cell bbox contains can still be absorbed by a cell whose
                // exact text it carries, as long as that cell's budget is
                // untouched. ~keep
                let mut cell_text_index: std::collections::HashMap<&str, Vec<usize>> = std::collections::HashMap::new();
                for (ci, (_, c)) in flushed_cells.iter().enumerate() {
                    let text = c.text.trim();
                    if !text.is_empty() {
                        cell_text_index.entry(text).or_default().push(ci);
                    }
                }
                const CELL_Y_BIN: f32 = 18.0;
                let cell_bin = |y: f32| (y / CELL_Y_BIN).floor() as i32;
                let mut cell_y_index: std::collections::HashMap<i32, Vec<usize>> = std::collections::HashMap::new();
                for (ci, (b, _)) in flushed_cells.iter().enumerate() {
                    for bin in cell_bin(b.y)..=cell_bin(b.y + b.height) {
                        cell_y_index.entry(bin).or_default().push(ci);
                    }
                }
                // Per-cell token budgets. `render_text()` emits each cell's
                // content exactly once, so a cell can absorb dropped spans
                // totalling at most the material it was built from. Bbox
                // containment alone cannot decide a drop: a rowspan-merged
                // cell's bbox can cover far more spans than its text was
                // built from (a schedule column's tall cell covering
                // hundreds of repeated marks while holding six lines of
                // them), and an empty cell re-emits nothing at all (a value
                // column without row rules becomes one tall empty cell whose
                // bbox swallows every figure in the column). A span leaves
                // the flow only by consuming its tokens from a covering
                // cell's budget — the fragment-level form of the same rule
                // the text fallback needs: membership cannot see
                // multiplicity.
                //
                // Budgets are denominated in the cell's member-span tokens,
                // not its rendered text: the cell builders rewrite text
                // while joining spans (sub-em gaps and CJK/fullwidth pairs
                // glue two spans with no separator, column-spanning decimals
                // split "12.11" into "12 11"), so the rendered text is not a
                // token superset of the spans the cell was built from. The
                // member spans are, by construction — they are clones of the
                // same flow spans this filter walks. A cell recorded without
                // member spans falls back to its rendered text. ~keep
                let mut cell_remaining: Vec<std::collections::HashMap<&str, usize>> = flushed_cells
                    .iter()
                    .map(|(_, c)| {
                        let mut m = std::collections::HashMap::new();
                        if c.spans.is_empty() {
                            for tok in c.text.split_whitespace() {
                                *m.entry(tok).or_insert(0) += 1;
                            }
                        } else {
                            for sp in &c.spans {
                                for tok in sp.text.split_whitespace() {
                                    *m.entry(tok).or_insert(0) += 1;
                                }
                            }
                        }
                        m
                    })
                    .collect();
                // The detector runs on `extract_words` output, so a budget
                // token can itself be a glued compound of several flow spans
                // drawn with no gap ("12,3" + "45" -> "12,345"). A contained
                // span may therefore consume a substring of one budget
                // token; the unmatched remainder returns to the budget so
                // the sibling fragments can follow. Absorption stays bounded
                // by the material the cell was built from. The text-fallback
                // path keeps whole-token matching: it has no containment
                // evidence, so fragments would let distant same-substring
                // text vanish. ~keep
                fn take_one(
                    work: &mut std::collections::HashMap<&str, usize>,
                    tok: &str,
                    allow_fragment: bool,
                ) -> bool {
                    if let Some(r) = work.get_mut(tok)
                        && *r > 0
                    {
                        *r -= 1;
                        return true;
                    }
                    if !allow_fragment {
                        return false;
                    }
                    // Shortest host wins: splits are deterministic (HashMap
                    // iteration order is not) and minimal. ~keep
                    let Some(host) = work
                        .iter()
                        .filter(|(k, n)| **n > 0 && k.len() > tok.len() && k.contains(tok))
                        .map(|(k, _)| *k)
                        .min_by_key(|k| (k.len(), *k))
                    else {
                        return false;
                    };
                    *work.get_mut(host).unwrap() -= 1;
                    let start = host.find(tok).unwrap();
                    let before = &host[..start];
                    let after = &host[start + tok.len()..];
                    if !before.is_empty() {
                        *work.entry(before).or_insert(0) += 1;
                    }
                    if !after.is_empty() {
                        *work.entry(after).or_insert(0) += 1;
                    }
                    true
                }
                // Test plus decrement, in one place so the containment and
                // text-fallback paths cannot drift apart — they must draw on
                // the SAME accounting. All-or-nothing: a span either fits
                // the cell's remaining budget whole or leaves it untouched. ~keep
                fn try_consume(
                    remaining: &mut std::collections::HashMap<&str, usize>,
                    span_tokens: &[(&str, usize)],
                    allow_fragment: bool,
                ) -> bool {
                    let mut work = remaining.clone();
                    for (tok, n) in span_tokens {
                        for _ in 0..*n {
                            if !take_one(&mut work, tok, allow_fragment) {
                                return false;
                            }
                        }
                    }
                    *remaining = work;
                    true
                }

                let preserved_label_indices: std::collections::HashSet<usize> = Self::identify_multi_row_labels(&spans)
                    .into_iter()
                    .filter(|&idx| {
                        // Only preserve labels whose text is NOT
                        // already emitted by any table's
                        // `render_text()`. This is what makes the
                        // fix safe on pages where the spatial
                        // extractor captured the sparse label
                        // column as cells — we let the table
                        // render them and drop them from flow.
                        // On pages like WS/T 779 where the label
                        // column is a genuine multi-row-spanning
                        // column that the extractor did NOT
                        // capture, the set is empty and every
                        // identified label stays in flow where
                        // `reorder_rowspan_labels` below can
                        // promote it. ~keep
                        let t = spans[idx].text.trim();
                        !t.is_empty() && !table_cell_texts.contains(t)
                    })
                    .collect();

                // One pass covers both cases: with no preserved labels the
                // index test is simply never true.
                //
                // A cell-covered span leaves the flow by consuming its tokens
                // from the covering cell's budget. A text-fallback span
                // (matching a cell's exact text without lying inside any cell
                // — the ascent-overshoot geometry the fallback exists for)
                // claims a same-text cell whose budget is still whole. Both
                // paths draw on the SAME per-cell budgets, so a cell that
                // absorbed its own spans cannot also be claimed from a
                // distance: with split accounting, a page carrying one
                // repeated label per row dropped three spans against two
                // rendering cells.
                //
                // Once every cell that could absorb a text is spoken for,
                // further spans matching it stay in the flow, because
                // `render_text()` will not emit them again. Membership cannot
                // see multiplicity. Spans no cell can absorb stay in the
                // flow: better to duplicate than to silently drop. ~keep
                let mut kept: Vec<crate::layout::TextSpan> = Vec::with_capacity(spans.len());
                for (i, s) in spans.drain(..).enumerate() {
                    if preserved_label_indices.contains(&i) {
                        kept.push(s);
                        continue;
                    }
                    if !table_cell_mcids.is_empty() {
                        // Tagged PDF: MCID decides ownership precisely. A span
                        // with no MCID (widget/annotation) stays in flow —
                        // better to duplicate than to silently drop. ~keep
                        if s.mcid.is_some_and(|m| table_cell_mcids.contains(&m)) {
                            continue;
                        }
                        kept.push(s);
                        continue;
                    }
                    // Inner scope: every borrow of `s.text` ends before the
                    // span is moved into `kept`. ~keep
                    let dropped = {
                        let trimmed = s.text.trim();
                        // A span rarely carries more than a few tokens; a
                        // linear-scan Vec beats allocating a HashMap per span. ~keep
                        let mut span_tokens: Vec<(&str, usize)> = Vec::new();
                        for tok in trimmed.split_whitespace() {
                            if let Some(entry) = span_tokens.iter_mut().find(|(t, _)| *t == tok) {
                                entry.1 += 1;
                            } else {
                                span_tokens.push((tok, 1));
                            }
                        }
                        // Probe only the cells in the span's y-band (±1 bin
                        // guards the containment tolerance). Equivalent to
                        // scanning every cell. ~keep
                        let slo = cell_bin(s.bbox.y) - 1;
                        let shi = cell_bin(s.bbox.y + s.bbox.height) + 1;
                        let mut decided = false;
                        'cells: for bin in slo..=shi {
                            let Some(cands) = cell_y_index.get(&bin) else {
                                continue;
                            };
                            for &ci in cands {
                                let (cell_bbox, _) = &flushed_cells[ci];
                                if !Self::contains_rect_with_tolerance(cell_bbox, &s.bbox, RETAIN_TOLERANCE) {
                                    continue;
                                }
                                if try_consume(&mut cell_remaining[ci], &span_tokens, true) {
                                    decided = true;
                                    break 'cells;
                                }
                            }
                        }
                        // Fallback: text-based match. The bbox check above
                        // uses a tight 0.1pt tolerance and rejects spans whose
                        // font ascent extends slightly above the cell's ink
                        // box ("FY 15 1st Q TTL" labels in a regional airline's
                        // traffic table). Require spatial proximity so body
                        // text that coincidentally matches a cell's text
                        // elsewhere on the page is not dropped. ~keep
                        if !decided && !trimmed.is_empty() && cell_text_index.contains_key(trimmed) {
                            let near_table = tables.iter().any(|t| {
                                t.bbox.is_some_and(|tb| {
                                    let cx = s.bbox.x + s.bbox.width / 2.0;
                                    let cy = s.bbox.y + s.bbox.height / 2.0;
                                    cx >= tb.x - RETAIN_TOLERANCE
                                        && cx <= tb.x + tb.width + RETAIN_TOLERANCE
                                        && cy >= tb.y - RETAIN_TOLERANCE
                                        && cy <= tb.y + tb.height + RETAIN_TOLERANCE
                                })
                            });
                            if near_table {
                                for &ci in &cell_text_index[trimmed] {
                                    if try_consume(&mut cell_remaining[ci], &span_tokens, false) {
                                        decided = true;
                                        break;
                                    }
                                }
                            }
                        }
                        decided
                    };
                    if !dropped {
                        kept.push(s);
                    }
                }
                spans = kept;
            }

            // Row-aware ordering: quantize Y into bands and sort band-
            // descending, then X ascending within a band. Strict Y sorting
            // would interleave cells from the same tabular row whose Y
            // values differ by typographic jitter (common in CJK layouts,
            // superscripts, and centered multi-line labels).
            //
            // Skip for multi-column pages: extract_spans() already applied
            // XY-cut column ordering. Re-sorting with row-aware would interleave
            // left/right columns line-by-line, splicing words from adjacent
            // columns into each other. A topological block order (a precede
            // relation over text blocks) handles genuine multi-region pages (a
            // two-column body/footer, a sidebar beside the body) that a flat
            // row-aware (y,x) sort interleaves. The gate (substantial, text-dense,
            // dominant side-by-side blocks) rejects single-column pages, tables,
            // TOCs and forms; it de-interleaves real two-column bodies and
            // sidebar+body layouts. topological_block_order runs unconditionally;
            // it self-gates to None unless the page has dominant side-by-side
            // text-dense blocks (see its side_by_side gate), so single-column,
            // table, and TOC pages are byte-identical.
            //
            // Item 2 (M2): first lift a narrow, sparse, body-aligned marginalia
            // rail (manuscript line numbers / a folio rail) OUT of the body, so
            // the column dispatch below sees a clean body. A rail otherwise
            // injects a spurious second corridor (disqualifying prose detection)
            // and a sparse block (defeating the topological side-by-side gate),
            // and a flat (y,x) sort then weaves its numerals into the prose. The
            // rail is re-appended at the end of the reading order after the
            // ladder, before the artifact retain. No-op (None) on ordinary pages
            // → byte-identical. ~keep
            let marginalia_trailing: Vec<crate::layout::TextSpan> =
                if let Some(idx) = Self::lift_marginalia_column(&spans) {
                    let idxset: std::collections::HashSet<usize> = idx.into_iter().collect();
                    let mut keep = Vec::with_capacity(spans.len());
                    let mut marg = Vec::new();
                    for (i, s) in std::mem::take(&mut spans).into_iter().enumerate() {
                        if idxset.contains(&i) {
                            marg.push(s);
                        } else {
                            keep.push(s);
                        }
                    }
                    marg.sort_by(|a, b| crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x));
                    spans = keep;
                    marg
                } else {
                    Vec::new()
                };

            let mut topo_applied = false;
            if let Some(reordered) = Self::topological_block_order(&spans) {
                spans = reordered;
                topo_applied = true;
            }
            if topo_applied {
            } else if let Some(ordered) = Self::sidebar_body_reading_order(&spans) {
                // Narrow metadata SIDEBAR + wide body (e.g. an MDPI first page
                // whose left rail carries Citation:/Received:/Accepted:/Copyright
                // furniture). The row-aware (y,x) sort otherwise threads that rail
                // INTO the body paragraphs at matching Y-bands; segregating it so
                // each region reads contiguously matches a block-based extractor
                // and stops the rail-into-body interleave. Already used by the
                // md/html/structured paths; this wires the SAME ordering into the
                // plain-text path. Tightly gated (≥30 spans, narrow sidebar with
                // ≥2 furniture labels), so it is a no-op (None) on ordinary pages. ~keep
                spans = ordered;
            } else if let Some(gutter_x) =
                Self::prose_two_column_gutter(&spans).or_else(|| Self::classifier_column_gutter(&spans))
            {
                // Genuine two-column prose (content-balance gated — forms /
                // TOC / tables / figures are rejected), OR a ragged
                // reference list / dense results body that the clean corridor
                // sweep and `is_multi_column_page` MISS but the per-column region
                // classifier confirms (`classifier_column_gutter`). Both read
                // column-major with band separation: full-width rows (titles,
                // mid-body section headings, footers — spans crossing the gutter)
                // are emitted at their vertical position, between the column runs
                // around them, so they are never split across the gutter
                // (§14.8.3). This branch is tried BEFORE the single-column
                // row-aware path so a 2-column reference page (which fails
                // `is_multi_column_page`) is reordered instead of interleaved;
                // both gutter detectors return None on single-column pages, which
                // then fall through to the row-aware branch unchanged. ~keep
                Self::reorder_column_major_with_bands(&mut spans, gutter_x);
                // NB: do NOT run reorder_same_line_runs here. The column emit
                // already orders each column by (y desc, x asc); a same-line
                // X-sort would re-merge vertically-adjacent lines whenever the
                // body leading (e.g. ~9pt) is tighter than same_line_threshold
                // (min_fs·1.2 ≈ 10.8pt), pulling a new left-margin reference
                // ahead of the previous reference's indented continuation
                // (bibliography interleave) or shattering wrapped hyphenated
                // lines in dense two-column bodies. ~keep
            } else if !Self::is_multi_column_page(&spans)
                || (!tables.is_empty() && Self::multicol_signal_is_tabular(&spans, &tables))
            {
                // Either a genuine single-column page, OR a single-column page
                // whose only multi-column geometric signal comes from a TABLE
                // (a data grid whose column-aligned cells trip
                // `is_multi_column_page`). In the latter case the genuine
                // two-column branches (topological / prose-gutter / classifier)
                // above all declined, so the page is NOT a two-column body; the
                // multi-column false positive is purely tabular. The correct
                // reading order is then the row-aware (y desc, x asc) band sort
                // — it linearises both the surrounding prose AND the table rows.
                // Without it the page keeps raw content-stream order, which on
                // these journal pages interleaves the table's column-major cell
                // stream INTO the prose paragraph (PMC8078162 §3.1). Gated on a
                // detected table whose region accounts for the multi-column
                // signal (`multicol_signal_is_tabular`), so genuine two-column
                // pages — which the column branches catch first, and which carry
                // no page-dominating table — are unaffected. ~keep
                spans.sort_by(|a, b| {
                    let cmp = crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x);
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                    a.sequence.cmp(&b.sequence)
                });

                Self::reorder_rowspan_labels(&mut spans);

                // Restore intra-line reading order after the row-aware band sort.
                // Off-baseline glyphs (e.g. superscripts/subscripts) can land in
                // adjacent bands and be emitted out of X order; fix that per line. ~keep
                Self::reorder_same_line_runs(&mut spans);
            }

            // Re-append the lifted marginalia rail at the end of the body
            // reading order (Item 2 / M2). Done before the artifact retain so
            // any artifact-marked rail spans are still dropped. ~keep
            spans.extend(marginalia_trailing);

            // Drop content marked /Artifact (PDF Spec ISO 32000-1:2008
            // §14.8.2.2 — headers, footers, page numbers, decorations) —
            // unless the caller opted in via `options.include_artifacts`
            // (default true). Untagged-PDF running-header detection
            // runs at document level and feeds the same artifact_type flag. ~keep
            if !options.include_artifacts {
                spans.retain(|s| s.artifact_type.is_none());
            }

            Self::reverse_rtl_visual_order_runs(&mut spans);

            spans.retain(|s| {
                s.bbox.x.is_finite()
                    && s.bbox.y.is_finite()
                    && s.bbox.width.is_finite()
                    && s.bbox.height.is_finite()
                    && s.font_size.is_finite()
            });

            // as isolated fragments interleaved with other spans (pdfa_004).
            Self::merge_sub_superscript_spans(&mut spans);

            // Inline table insertion.
            //
            // Tables were previously rendered in a single block appended
            // at the end of the page text, after all flow spans. That
            // matches how `extract_text` historically worked but it means
            // tabular content appears far away from the prose that
            // surrounds it in reading order — on product data sheets
            // like ORAFOL 5900 the "Physical and Chemical Properties"
            // label/value rows showed up 20+ lines below the section
            // they belong to, which was perceived as
            // the content being dropped entirely.
            //
            // Instead, maintain a sorted queue of tables keyed by their
            // top-Y (the larger Y coordinate of the table's bbox, per PDF
            // user-space conventions where Y grows upward). As we walk
            // the flow spans in row-aware reading order, whenever the
            // next span's top-Y falls below the top-Y of the queue's
            // leading table, we flush that table's rendered text at
            // that point, then continue. A final pass at the end emits
            // any tables whose top-Y is below all remaining spans (or
            // that have no flow spans at all).
            //
            // Tables are emitted at most once regardless of how many
            // spans sit above them, preserving existing behaviour
            // semantics while inlining the rendering at its spatial
            // reading-order position. ~keep
            let mut pending_tables: Vec<(f32, &crate::structure::table_extractor::Table)> = tables
                .iter()
                .filter_map(|t| t.bbox.map(|b| (b.y + b.height, t)))
                .collect();
            // Sort descending by top-Y so `pop()` returns the next table
            // to emit in reading order (larger Y first). ~keep
            pending_tables.sort_by(|(a, _), (b, _)| crate::utils::safe_float_cmp(*b, *a));

            let flush_table = |text: &mut String, table: &crate::structure::table_extractor::Table| {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push('\n');
                text.push_str(&table.render_text());
                if !text.ends_with('\n') {
                    text.push('\n');
                }
            };

            let mut text = String::with_capacity(spans.len() * 20);
            let mut prev_span: Option<TextSpan> = None;

            for span in &spans {
                // Flush any tables that sit above this span in PDF
                // reading order (their top-Y is greater than or equal
                // to the span's top-Y, meaning they should appear first). ~keep
                while let Some(&(table_top_y, table)) = pending_tables.last() {
                    let span_top_y = span.bbox.y + span.bbox.height;
                    if table_top_y >= span_top_y {
                        flush_table(&mut text, table);
                        pending_tables.pop();
                        // Reset prev_span so the flow-text glue logic
                        // doesn't try to stitch the table's rendered
                        // block together with the next flow span. ~keep
                        prev_span = None;
                    } else {
                        break;
                    }
                }

                if let Some(prev) = &prev_span {
                    let prev_end_x = prev.bbox.x + prev.bbox.width;
                    let span_end_x = span.bbox.x + span.bbox.width;
                    // Containment check: skip a span only if it is geometrically
                    // contained within the previous span AND has identical text.
                    // Without the text comparison, distinct lines that happen to
                    // overlap spatially (e.g., due to small Tm-scaled offsets)
                    // would be silently dropped. ~keep
                    let y_same = (prev.bbox.y - span.bbox.y).abs() < 2.0;
                    if y_same
                        && span.bbox.x >= prev.bbox.x - 0.5
                        && span_end_x <= prev_end_x + 0.5
                        && span.text == prev.text
                    {
                        continue;
                    }

                    let y_diff = (prev.bbox.y - span.bbox.y).abs();
                    let gap = span.bbox.x - prev_end_x;
                    let delta_x = span.bbox.x - prev.bbox.x;

                    // Korean mid-eojeol soft wrap (SEG-KO): keep a Hangul word whole
                    // when it wrapped mid-syllable. The wrap surfaces either as a
                    // y-line break OR (when the two halves share a baseline band) as a
                    // large backward X jump, so it is gated at each break site below. ~keep
                    let hangul_midword_wrap = Self::hangul_midword_line_wrap(&text, prev, span);
                    if y_diff > Self::same_line_threshold(prev, span) {
                        let font_size = prev.font_size.max(span.font_size).max(10.0);
                        let line_height = font_size * 1.2;
                        let num_breaks = (y_diff / line_height).round() as usize;
                        if !(hangul_midword_wrap && num_breaks == 1) {
                            for _ in 0..num_breaks.clamp(1, 3) {
                                text.push('\n');
                            }
                        }
                    } else if gap < -1.0 {
                        let fs = span.font_size.max(prev.font_size).max(6.0);
                        if gap < -(fs * 20.0) {
                            if !hangul_midword_wrap && !text.ends_with('\n') {
                                text.push('\n');
                            }
                        } else if delta_x < -fs * 3.0 {
                            // Same visual line, but the current span starts well to the LEFT of the
                            // previous span's start — i.e., the upstream ordering is non-monotonic in X.
                            // This commonly occurs with multi-column layouts or XY-cut artifacts where
                            // spans from different visual rows fall within the same Y tolerance band
                            // (see `same_line_threshold`).
                            //
                            // Without inserting a separator, these spans would be concatenated
                            // (e.g. `instancesinstancesinstances` from adjacent table headers).
                            // Treat this backward X jump as a logical break and emit a newline.
                            // ~keep
                            if !text.ends_with('\n') {
                                text.push('\n');
                            }
                        } else if gap < -fs * 3.0 && y_diff > fs * 0.5 && delta_x <= fs * 0.5 {
                            // Soft-wrapped next line (carriage return). Three
                            // signals must coincide so inline math/super-scripts
                            // and chart labels are NOT mistaken for a wrap:
                            //   • `gap < -fs*3` — the previous span ended far to
                            //     the RIGHT of where this one starts, i.e. the
                            //     line filled and wrapped (adjacent math glyphs
                            //     have a near-zero gap, so they are excluded);
                            //   • `y_diff > fs*0.5` — a real baseline drop (a
                            //     super/sub-script shift is smaller);
                            //   • `delta_x <= fs*0.5` — it returned to ~the line's
                            //     left margin.
                            // Its leading sits UNDER `same_line_threshold`
                            // (single-spaced body at ~1.0 em vs the 1.2 em
                            // threshold) so the y-newline branch above never
                            // fired, leaving the line-final and line-initial words
                            // glued together. A line break encodes no space (ISO
                            // 32000 §9.4.2 — positioning is geometric); synthesize
                            // one as a newline, which the downstream join /
                            // `normalize_text` collapses to a space and
                            // de-hyphenates. ~keep
                            if !hangul_midword_wrap && !text.ends_with('\n') {
                                text.push('\n');
                            }
                        } else if y_diff > 1.0
                            && delta_x <= 0.5
                            && gap < -fs
                            && !prev.rtl_draw_logical
                            && !span.rtl_draw_logical
                            && !crate::text::bidi::looks_rtl(&prev.text)
                            && !crate::text::bidi::looks_rtl(&span.text)
                        {
                            // Backtracking span with a real baseline offset,
                            // under the soft-wrap thresholds above: displayed
                            // math draws a fraction's denominator AFTER the
                            // relation sign that follows the numerator, so
                            // the next span starts at-or-left-of the previous
                            // span's ORIGIN with an overlap far beyond
                            // kerning (a denominator sits ~2 em back at a
                            // ~0.3 em baseline offset; stacked column cells
                            // land at delta_x ≈ 0 a row-pitch down). Bare
                            // concatenation fused these into tokens like
                            // "=dt" — break the line instead. Same-baseline
                            // kerned runs (y_diff ≈ 0) and forward-advancing
                            // subscripts (gap > -1 em) never reach here. ~keep
                            if !hangul_midword_wrap && !text.ends_with('\n') {
                                text.push('\n');
                            }
                        } else if prev.font_name != span.font_name
                            && span_end_x > prev_end_x + 0.5
                            && !text.ends_with(' ')
                            && !text.ends_with('\n')
                        {
                            text.push(' ');
                        } else if delta_x > fs * 1.5
                            && !text.ends_with(' ')
                            && !text.ends_with('\n')
                            && !Self::is_reliable_kerning_overlap(prev, span, gap)
                        {
                            // Inflated-width overlap recovery.
                            // A negative raw gap here usually comes from a
                            // font whose `/Widths` array is missing
                            // `FontInfo::new` fell back to the 550/1000-em
                            // constant, which over-reports each glyph's
                            // advance and drags `prev_end_x` past the real
                            // start of the next span. When the two spans'
                            // actual origins (`delta_x`) are separated by
                            // more than 1.5 em, they cannot both belong to
                            // the same word — the overlap is a width-table
                            // artifact, not real kerning — so insert a
                            // space to preserve the word boundary. This
                            // rescues cases like "STATION" + "FREEDOM"
                            // "UTILIZATION" + "CONFERENCE" in the NASA
                            // Apollo report header where raw gaps of
                            // -1.75 pt and -12.75 pt sit alongside
                            // delta_x values of 56 pt and 78 pt. ~keep
                            text.push(' ');
                        } else if y_diff > 1.0
                            && gap < -fs
                            && (span_end_x - prev_end_x).abs() <= fs / 12.0
                            && !prev.rtl_draw_logical
                            && !span.rtl_draw_logical
                            && !crate::text::bidi::looks_rtl(&prev.text)
                            && !crate::text::bidi::looks_rtl(&span.text)
                        {
                            // Right-aligned column stack. The backtracking arm
                            // above only reaches stacks that share a LEFT edge
                            // (delta_x ≈ 0); a right-aligned numeric column
                            // shares its RIGHT edge instead, so a shorter
                            // successor starts further right by the width
                            // difference — the one band no arm above tests. A
                            // row pitch under `same_line_threshold` then fused
                            // figures from consecutive rows ("22,796" + "3,052"
                            // → "22,7963,052"). Coincident right edges plus a
                            // full-em backward gap at a real baseline offset is
                            // the stack signature. The right-edge tolerance
                            // is a twelfth of the font size: the calibration
                            // column right-aligns its negative rows 0.371 pt
                            // left of its positive ones at 6 pt (0.062 em),
                            // and fs/12 (0.083 em) covers that with margin
                            // while reproducing the validated 0.5 pt band at
                            // the calibration size. Wider em-fractions sweep
                            // in coincidental ragged-edge alignments (a
                            // half-advance band separated twenty times the
                            // measured column population). Placed last, so it
                            // can only add a separator where none is emitted
                            // today. ~keep
                            if !hangul_midword_wrap && !text.ends_with('\n') {
                                text.push('\n');
                            }
                        }
                    } else if y_diff > 2.0 && gap > FORWARD_GAP_K * prev.font_size.max(span.font_size).max(1.0) {
                        // Forward-gap guard: pairs newly admitted to same-line
                        // handling by the widened threshold get a column/field-
                        // boundary check against FORWARD_GAP_K * max(fs).
                        // the constant's doc comment for calibration notes. ~keep
                        if !text.ends_with('\n') {
                            text.push('\n');
                        }
                    } else if prev.font_name != span.font_name
                        && gap > 0.5
                        && gap < prev.font_size.max(span.font_size).max(6.0) * 3.0
                        && !text.ends_with(' ')
                        && !text.ends_with('\n')
                    {
                        // Same-line font transition with a meaningful
                        // positive gap. Cross-font runs that survive the
                        // upstream `cross_font_word_glue` merge (i.e.
                        // both sides are multi-char) are word boundaries
                        // even when the gap is too small for the generic
                        // `should_insert_space` threshold (0.15 × fs) —
                        // e.g. roman → italic transitions in academic
                        // paper headers sit at ~2.7 pt at 10.9 pt body. ~keep
                        text.push(' ');
                    } else if Self::should_insert_space(prev, span) {
                        text.push(' ');
                    } else {
                        let fs = span.font_size.max(prev.font_size).max(6.0);
                        if gap > fs * 3.0 {
                            text.push('\n');
                        }
                    }
                }

                Self::push_span_text(&mut text, span);
                prev_span = Some(span.clone());
            }

            // Drain any tables that sit below all flow spans (or the
            // page had no flow spans at all). Without this final
            // pass they would be silently dropped now that the
            // end-of-page `for table in tables` block has been
            // removed. ~keep
            while let Some((_, table)) = pending_tables.pop() {
                flush_table(&mut text, table);
            }
            text
        };

        // Annotation text is already included via annotation_content_spans() in
        // extract_spans() — do NOT call append_non_widget_annotation_text() here,
        // as that would emit every annotation a second time. ~keep

        let final_text = Self::filter_leaked_metadata(&text);

        let final_text = Self::normalize_kangxi_radicals(&final_text);

        let final_text = Self::normalize_arabic_presentation_forms(&final_text);

        let cleaned_text = crate::converters::whitespace::cleanup_plain_text(&final_text);

        // For tagged PDFs, the structure-tree traversal at line 4306 already
        // captures all table-cell content via MCIDs. Appending tables here
        // would double-emit that content (structure-tree text + table render),
        // dropping precision. For untagged PDFs, tables are inlined via
        // pending_tables above, so this block is never reached (cached_tree
        // is None → condition would be false). The block is removed. ~keep

        // UTF-8 mojibake repair: a run of Latin-1 Supplement chars
        // whose raw bytes form valid UTF-8 decoding to non-Latin-1 code
        // points is almost certainly a double-encoded non-Latin string
        // (Cyrillic, Greek, CJK, Arabic, Hebrew, …) that surfaced
        // because the producing font had no ToUnicode CMap and the
        // /Differences / AGL lookup returned the UTF-8 byte sequence
        // re-interpreted as Latin-1. Re-decode those runs in place. ~keep
        let cleaned_text = Self::repair_utf8_mojibake(&cleaned_text);

        // Drop stray spaces a producer inserted between a CJK ideograph and an
        // embedded ASCII number (e.g. "公元前 1000 年" → "公元前1000年"). ~keep
        let cleaned_text = crate::extractors::text::strip_cjk_digit_boundary_spaces(&cleaned_text);

        // Drop a space the word-break heuristic injected inside a prime-notation
        // number (e.g. "0′′ .28" / "0′′. 28" → "0′′.28"). ~keep
        let cleaned_text = crate::extractors::text::strip_prime_decimal_boundary_spaces(&cleaned_text);

        Ok(cleaned_text)
    }

    /// Walk `input` and repair runs of Latin-1 Supplement characters
    /// whose raw byte values form a valid UTF-8 sequence whose decoded
    /// codepoints include at least one non-Latin-1 character.
    ///
    /// This undoes the most common shape of "Cyrillic served as
    /// Latin-1" mojibake that surfaces on PDFs whose fonts have no
    /// ToUnicode CMap. The decoded-codepoint gate (≥ U+0100 somewhere
    /// in the decoded run) ensures genuine Latin-1 content like
    /// "Résumé" — which also decodes as valid UTF-8 but stays entirely
    /// within U+0000..U+00FF — is left alone.
    fn repair_utf8_mojibake(input: &str) -> String {
        // Fast-path: if the string contains no Latin-1 Supplement codepoints
        // (U+0080..=U+00FF), there is nothing to repair. This avoids the
        // O(n) `Vec<char>` allocation on every ASCII-only page. ~keep
        if !input.chars().any(|c| matches!(c as u32, 0x80..=0xFF)) {
            return input.to_string();
        }
        let mut out = String::with_capacity(input.len());
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let mut j = i;
            while j < chars.len() {
                let cc = chars[j] as u32;
                if (0x80..=0xFF).contains(&cc) {
                    j += 1;
                } else {
                    break;
                }
            }
            if j - i >= 2 {
                let bytes: Vec<u8> = chars[i..j].iter().map(|&c| c as u8).collect();
                if let Ok(decoded) = std::str::from_utf8(&bytes)
                    && decoded.chars().any(|c| c as u32 > 0xFF)
                {
                    out.push_str(decoded);
                    i = j;
                    continue;
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        out
    }

    /// Extract text from all pages of the document.
    ///
    /// Concatenates text from every page, separated by form feed characters (`\x0c`).
    /// This is a convenience method equivalent to calling `extract_text()` for each page.
    ///
    /// # Returns
    ///
    /// The combined text from all pages.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("paper.pdf")?;
    /// let all_text = doc.extract_all_text()?;
    /// println!("Full document: {} chars", all_text.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_all_text(&self) -> Result<String> {
        let num_pages = self.page_count()?;
        let mut result = String::new();

        for i in 0..num_pages {
            if i > 0 {
                result.push('\x0c');
            }
            match self.extract_text(i) {
                Ok(text) => result.push_str(&text),
                Err(error) => {
                    tracing::warn!(target: LOG_TARGET,
                        page_index = i,
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "failed to extract text from page"
                    );
                }
            }
        }

        Ok(result)
    }
}

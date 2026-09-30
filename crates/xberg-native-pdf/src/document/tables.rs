//! Table detection and row-label reconstruction.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

/// Gap, in em, within which two source-adjacent word fragments are re-glued into
/// one table word. One em rather than the original half em: a font can kern the
/// two halves of one word apart by more than a normal inter-word space, while a
/// genuine column jump is many ems wide. See `extract_table_word_spans`. ~keep
const TABLE_WORD_FUSE_EM: f32 = 1.0;

fn preserve_source_space(text: &mut String, preceded_by_space: bool) {
    if preceded_by_space {
        text.insert(0, ' ');
    }
}

impl PdfDocument {
    /// Promote labels in rowspan-sparse columns so they sort at the top
    /// of their data-row block instead of landing mid-group.
    ///
    /// A "label" here is a span in an X-cluster that contains far fewer
    /// spans than the most populous X-cluster (i.e., it spans multiple
    /// rows of the adjacent data column). Labels are typically vertically
    /// centred in their block, so a strict Y sort places them between
    /// the rows they describe. This post-processor detects the pattern
    /// and rewrites each label's effective sort Y to sit just above the
    /// topmost data row it visually covers.
    ///
    /// Data rows are partitioned between adjacent labels at the midpoint
    /// of their Y coordinates (nearest-label assignment). The topmost
    /// data row in a label's partition becomes the anchor for promotion.
    ///
    /// Nothing is mutated if there are no sparse columns or not enough
    /// data rows to confidently infer row-grouping (min 6 rows in the
    /// dense reference column).
    /// Identify span indices that look like multi-row-spanning labels —
    /// sparse-X-column spans whose Y values sit inside the data Y range
    /// of the dense columns on the page. These are the same spans that
    /// `reorder_rowspan_labels` would promote to the top of their row
    /// block, except this function returns them **before** the spatial
    /// table detector's retain filter has a chance to drop them from
    /// the flow span list.
    ///
    /// The retain filter in `extract_text_with_options` removes every
    /// span whose bbox is contained in a detected table's bbox. On CJK
    /// reference-data PDFs the test-name label column is
    /// narrow and vertically centred within each multi-row data block,
    /// so its spans are inside the table bbox and would be dropped
    /// without replacement — the spatial table extractor does not emit
    /// these labels as `TableCell`s either. Preserving the identified
    /// labels through the retain filter lets `reorder_rowspan_labels`
    /// promote them to their proper reading-order position alongside
    /// the surviving flow spans.
    ///
    /// Returns a `HashSet` of indices into the provided `spans` slice.
    /// Callers must use the returned indices **before** any reordering
    /// or retention mutates the slice.
    pub(crate) fn identify_multi_row_labels(spans: &[crate::layout::TextSpan]) -> std::collections::HashSet<usize> {
        use std::collections::{BTreeSet, HashMap as StdHashMap, HashSet};

        let mut out: HashSet<usize> = HashSet::new();
        if spans.len() < 10 {
            return out;
        }

        // Cluster by X proximity (15pt gap threshold) — same heuristic
        // as `reorder_rowspan_labels`. ~keep
        let mut by_x: Vec<usize> = (0..spans.len()).collect();
        by_x.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[a].bbox.x, spans[b].bbox.x));
        const X_GAP: f32 = 15.0;
        let mut columns: Vec<Vec<usize>> = Vec::new();
        let mut cur: Vec<usize> = Vec::new();
        let mut last_x = f32::NEG_INFINITY;
        for &idx in &by_x {
            let x = spans[idx].bbox.x;
            if !cur.is_empty() && x - last_x > X_GAP {
                columns.push(std::mem::take(&mut cur));
            }
            cur.push(idx);
            last_x = x;
        }
        if !cur.is_empty() {
            columns.push(cur);
        }
        if columns.len() < 2 {
            return out;
        }

        let max_count = columns.iter().map(|c| c.len()).max().unwrap_or(0);
        if max_count < 6 {
            return out;
        }

        let mut col_order: Vec<usize> = (0..columns.len()).collect();
        col_order.sort_by(|&a, &b| columns[b].len().cmp(&columns[a].len()));
        let dense_cols_count = columns.iter().filter(|c| c.len() * 2 > max_count).count();

        let band_of = |y: f32| (y / crate::utils::ROW_BAND_TOLERANCE_PT).round() as i32;
        let data_bands: BTreeSet<i32> = if dense_cols_count >= 3 {
            let top: Vec<&Vec<usize>> = col_order.iter().take(3).map(|&i| &columns[i]).collect();
            let mut support: StdHashMap<i32, usize> = StdHashMap::new();
            for col in &top {
                let bands: HashSet<i32> = col.iter().map(|&i| band_of(spans[i].bbox.y)).collect();
                for b in bands {
                    *support.entry(b).or_insert(0) += 1;
                }
            }
            support.into_iter().filter(|(_, c)| *c >= 3).map(|(b, _)| b).collect()
        } else if dense_cols_count == 2 {
            let a: HashSet<i32> = columns[col_order[0]]
                .iter()
                .map(|&i| band_of(spans[i].bbox.y))
                .collect();
            let b: HashSet<i32> = columns[col_order[1]]
                .iter()
                .map(|&i| band_of(spans[i].bbox.y))
                .collect();
            a.intersection(&b).copied().collect()
        } else {
            columns[col_order[0]]
                .iter()
                .map(|&i| band_of(spans[i].bbox.y))
                .collect()
        };

        if data_bands.len() < 4 {
            return out;
        }

        let band_pt = crate::utils::ROW_BAND_TOLERANCE_PT;
        let data_top = (*data_bands.iter().next_back().unwrap() as f32) * band_pt + band_pt / 2.0;
        let data_bot = (*data_bands.iter().next().unwrap() as f32) * band_pt - band_pt / 2.0;

        for col in &columns {
            if col.len() < 2 || col.len() * 2 >= max_count {
                continue;
            }
            let in_data: Vec<usize> = col
                .iter()
                .copied()
                .filter(|&i| {
                    let y = spans[i].bbox.y;
                    y > data_bot && y < data_top
                })
                .collect();
            if in_data.len() >= 2 {
                out.extend(in_data);
            }
        }

        out
    }

    pub(crate) fn reorder_rowspan_labels(spans: &mut Vec<crate::layout::TextSpan>) {
        use std::collections::HashMap;

        if spans.len() < 10 {
            return;
        }

        // Cluster by X proximity (15pt gap threshold). Walk spans ordered
        // by left edge; start a new cluster whenever the gap exceeds the
        // threshold. ~keep
        let mut by_x: Vec<usize> = (0..spans.len()).collect();
        by_x.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[a].bbox.x, spans[b].bbox.x));
        const X_GAP: f32 = 15.0;
        let mut columns: Vec<Vec<usize>> = Vec::new();
        let mut cur: Vec<usize> = Vec::new();
        let mut last_x = f32::NEG_INFINITY;
        for &idx in &by_x {
            let x = spans[idx].bbox.x;
            if !cur.is_empty() && x - last_x > X_GAP {
                columns.push(std::mem::take(&mut cur));
            }
            cur.push(idx);
            last_x = x;
        }
        if !cur.is_empty() {
            columns.push(cur);
        }
        if columns.len() < 2 {
            return;
        }

        let max_count = columns.iter().map(|c| c.len()).max().unwrap_or(0);
        if max_count < 6 {
            return;
        }

        let mut col_order: Vec<usize> = (0..columns.len()).collect();
        col_order.sort_by(|&a, &b| columns[b].len().cmp(&columns[a].len()));

        // A column is "dense" when it holds a strict majority of the
        // most populous column's spans. Pages with multiple dense data
        // columns (three or more) let us derive the data-row range by
        // intersecting their Y bands — headers and sub-headers populate
        // only a subset of columns at their Y and fall out. ~keep
        let dense_cols_count = columns.iter().filter(|c| c.len() * 2 > max_count).count();

        let dense_col = &columns[col_order[0]];
        let mut dense_ys: Vec<f32> = dense_col.iter().map(|&i| spans[i].bbox.y).collect();
        dense_ys.sort_by(|a, b| crate::utils::safe_float_cmp(*b, *a));

        // Compute the set of Y bands that count as "data". When several
        // dense columns are available, require a band to have support in
        // the top three; otherwise fall back to the single dense column's
        // own Y values. ~keep
        let band_of = |y: f32| (y / crate::utils::ROW_BAND_TOLERANCE_PT).round() as i32;
        use std::collections::{BTreeSet, HashMap as StdHashMap, HashSet};

        let data_bands: BTreeSet<i32> = if dense_cols_count >= 3 {
            let top: Vec<&Vec<usize>> = col_order.iter().take(3).map(|&i| &columns[i]).collect();
            let mut support: StdHashMap<i32, usize> = StdHashMap::new();
            for col in &top {
                let bands: HashSet<i32> = col.iter().map(|&i| band_of(spans[i].bbox.y)).collect();
                for b in bands {
                    *support.entry(b).or_insert(0) += 1;
                }
            }
            support.into_iter().filter(|(_, c)| *c >= 3).map(|(b, _)| b).collect()
        } else if dense_cols_count == 2 {
            let a: HashSet<i32> = columns[col_order[0]]
                .iter()
                .map(|&i| band_of(spans[i].bbox.y))
                .collect();
            let b: HashSet<i32> = columns[col_order[1]]
                .iter()
                .map(|&i| band_of(spans[i].bbox.y))
                .collect();
            a.intersection(&b).copied().collect()
        } else {
            dense_col.iter().map(|&i| band_of(spans[i].bbox.y)).collect()
        };

        if data_bands.len() < 4 {
            return;
        }
        let band_pt = crate::utils::ROW_BAND_TOLERANCE_PT;
        let data_top = (*data_bands.iter().next_back().unwrap() as f32) * band_pt + band_pt / 2.0;
        let data_bot = (*data_bands.iter().next().unwrap() as f32) * band_pt - band_pt / 2.0;

        // Y-bands occupied by the dense column. Genuine rowspan labels are
        // vertically centred *between* data rows, so their Y-band must NOT
        // appear in this set. Spans whose Y aligns with the dense column are
        // line-continuation text on the same logical line, not labels. ~keep
        let dense_bands: HashSet<i32> = dense_col.iter().map(|&i| band_of(spans[i].bbox.y)).collect();

        // Numbered reference/bibliography lists render the leading marker
        // ("1.", "2.", "3.") in a narrow column to the left of the body
        // text. That marker column is sparse (one per entry) and its markers
        // sit between body rows, so they look exactly like rowspan labels to
        // the heuristic below — but they are NOT: each number belongs to its
        // own entry and promoting them scrambles the reference order. Detect
        // the pattern (>=3 numbered markers sharing a tight left-edge cluster
        // and spread down >=3 distinct rows = a vertical numbered list) and
        // exclude those markers from label promotion. ~keep
        let is_numbered_marker = |i: usize| -> bool {
            let t = spans[i].text.trim_start();
            let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
            (1..=3).contains(&digits) && t[digits..].starts_with(['.', ')'])
        };
        let numbered_excluded: HashSet<usize> = {
            let markers: Vec<usize> = (0..spans.len()).filter(|&i| is_numbered_marker(i)).collect();
            if markers.len() >= 3 {
                let mut xs: Vec<f32> = markers.iter().map(|&i| spans[i].bbox.x).collect();
                xs.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
                let median_x = xs[xs.len() / 2];
                let cluster: Vec<usize> = markers
                    .iter()
                    .copied()
                    .filter(|&i| (spans[i].bbox.x - median_x).abs() <= 6.0)
                    .collect();
                let rows: HashSet<i32> = cluster.iter().map(|&i| band_of(spans[i].bbox.y)).collect();
                if cluster.len() >= 3 && rows.len() >= 3 {
                    cluster.into_iter().collect()
                } else {
                    HashSet::new()
                }
            } else {
                HashSet::new()
            }
        };

        // Collect "label" candidates: spans that sit in a "sparse"
        // column — one that holds meaningfully fewer spans than the
        // most populous column. A candidate only qualifies when it
        // sits strictly inside the data Y range AND the sparse column
        // it belongs to has at least two entries inside that range —
        // single-span sparse cells are almost always stray annotations,
        // not labels. ~keep
        let mut labels: Vec<usize> = Vec::new();
        for col in &columns {
            if col.len() < 2 || col.len() * 2 >= max_count {
                continue;
            }
            let in_data: Vec<usize> = col
                .iter()
                .copied()
                .filter(|&i| {
                    let y = spans[i].bbox.y;
                    // Exclude spans on the same Y-band as the dense column:
                    // those are line-continuation text, not rowspan labels.
                    // Also exclude numbered-list markers (reference numbers),
                    // which would otherwise be hoisted out of reading order. ~keep
                    y > data_bot
                        && y < data_top
                        && !dense_bands.contains(&band_of(y))
                        && !numbered_excluded.contains(&i)
                })
                .collect();
            if in_data.len() >= 2 {
                labels.extend(in_data);
            }
        }
        if labels.is_empty() {
            return;
        }
        labels.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[b].bbox.y, spans[a].bbox.y));

        // Labels that sit at near-identical Y values almost always
        // annotate the same logical row block (e.g. a test-name in the
        // "name" column alongside a unit "×10⁹/L" in the "unit" column,
        // both vertically centred in the same 6-row group). Cluster
        // labels by Y proximity so each logical block is promoted as a
        // unit. ~keep
        const CLUSTER_GAP: f32 = 10.0;
        let mut clusters: Vec<Vec<usize>> = Vec::new();
        let mut cur: Vec<usize> = Vec::new();
        let mut last_y = f32::NAN;
        for &idx in &labels {
            let y = spans[idx].bbox.y;
            if !cur.is_empty() && (last_y - y).abs() > CLUSTER_GAP {
                clusters.push(std::mem::take(&mut cur));
            }
            cur.push(idx);
            last_y = y;
        }
        if !cur.is_empty() {
            clusters.push(cur);
        }
        let cluster_ys: Vec<f32> = clusters
            .iter()
            .map(|c| c.iter().map(|&i| spans[i].bbox.y).sum::<f32>() / c.len() as f32)
            .collect();

        let mut promoted: HashMap<usize, f32> = HashMap::new();
        for (k, cluster) in clusters.iter().enumerate() {
            let c_y = cluster_ys[k];
            let upper = if k > 0 {
                (cluster_ys[k - 1] + c_y) / 2.0
            } else {
                f32::INFINITY
            };
            let lower = if k + 1 < clusters.len() {
                (c_y + cluster_ys[k + 1]) / 2.0
            } else {
                f32::NEG_INFINITY
            };
            let upper_clamped = upper.min(data_top);
            let lower_clamped = lower.max(data_bot - 1.0);
            let mut anchor = f32::NEG_INFINITY;
            for &y in &dense_ys {
                if y <= upper_clamped && y > lower_clamped && y > anchor {
                    anchor = y;
                }
            }
            if anchor.is_finite() {
                for &i in cluster {
                    promoted.insert(i, anchor + 1.0);
                }
            }
        }
        if promoted.is_empty() {
            return;
        }

        let mut order: Vec<usize> = (0..spans.len()).collect();
        order.sort_by(|&a, &b| {
            let ya = promoted.get(&a).copied().unwrap_or(spans[a].bbox.y);
            let yb = promoted.get(&b).copied().unwrap_or(spans[b].bbox.y);
            crate::utils::row_aware_span_cmp(ya, spans[a].bbox.x, yb, spans[b].bbox.x)
        });
        let reordered: Vec<crate::layout::TextSpan> = order.into_iter().map(|i| spans[i].clone()).collect();
        *spans = reordered;
    }

    /// Extract tables from a page.
    ///
    /// Uses a hybrid spatial algorithm that combines text alignment and vector lines
    /// for robust table detection without explicit structure markup.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let tables = doc.extract_tables(0)?;
    /// for table in tables {
    ///     println!("Table with {} rows and {} columns", table.rows.len(), table.col_count);
    /// }
    /// ```
    pub fn extract_tables(&self, page_index: usize) -> Result<Vec<crate::structure::table_extractor::Table>> {
        self.extract_tables_with_config(
            page_index,
            crate::structure::spatial_table_detector::TableDetectionConfig::default(),
        )
    }

    /// Word-level span source for the spatial table detector.
    ///
    /// Words give the detector cell granularity (a spaced string splits into
    /// separate columns), but the geometric word clustering re-decides every
    /// join from raw bbox gaps — and a sub-em kerned split defeats any gap
    /// threshold, so a word the span-level merger had correctly assembled
    /// from per-glyph advance evidence can come back in fragments and
    /// surface as spaces inside a table cell. Re-glue the fragments the
    /// merger marked as boundary-free before handing spans to the detector.
    fn extract_table_word_spans(&self, page_index: usize) -> Result<Vec<crate::layout::TextSpan>> {
        let (words, continues_prev, space_before) = self.extract_words_inner(page_index, None, None, true)?;
        let mut fused: Vec<crate::layout::Word> = Vec::with_capacity(words.len());
        for ((word, continues), preceded_by_space) in words.into_iter().zip(continues_prev).zip(space_before) {
            let mut word = word;
            // The cell assembler re-decides joins from bbox gaps alone, and on a
            // tight face the space's glyphs abut or overlap their neighbours, so
            // that gap reads as zero or negative and the two words fuse
            // (GH#1948: `corrispettivisuperiori`). Carry the source space in the
            // word's own text so `cell_span_separator` emits it. Use the exact
            // source-character marker: a non-continuation can instead mean that
            // clustering visited glyphs out of source order or skipped a
            // non-whitespace glyph. ~keep
            preserve_source_space(&mut word.text, preceded_by_space);
            // Source adjacency says the producer drew these glyphs consecutively;
            // it does not say they are typographically adjacent, so geometry still
            // has a veto on both bounds:
            //   above  — the merger also concatenates runs across a column jump
            //            when neither side carries a space glyph, and honouring
            //            that here would dissolve the grid;
            //   below  — a large NEGATIVE gap is the signature of a backtrack
            //            (displayed-math denominators) or a line-wrap reset, the
            //            two cases the word merge loop guards with
            //            `gap < -font_size` and `delta_x < -5 * font_size`.
            // The band is one em, not the original half em: a font can kern two
            // halves of one word apart by more than a normal space (GH#1948:
            // `d’impresa` seams at 0.68 em), and a genuine column jump is many
            // ems wide. ~keep
            match fused.last_mut() {
                Some(prev)
                    if continues
                        && (word.bbox.x - (prev.bbox.x + prev.bbox.width)).abs()
                            < prev.avg_font_size.max(word.avg_font_size).max(1.0) * TABLE_WORD_FUSE_EM =>
                {
                    prev.absorb(word)
                }
                _ => fused.push(word),
            }
        }
        Ok(fused
            .into_iter()
            .map(|w| crate::layout::TextSpan {
                provenance: None,
                artifact_type: None,
                text: w.text,
                bbox: w.bbox,
                font_name: w.dominant_font,
                font_size: w.avg_font_size,
                font_weight: if w.is_bold {
                    crate::layout::FontWeight::Bold
                } else {
                    crate::layout::FontWeight::Normal
                },
                is_italic: w.is_italic,
                is_monospace: false,
                color: crate::layout::Color::black(),
                mcid: w.mcid,
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
                text_rise: 0.0,
                rtl_draw_logical: false,
                mirrored: false,
                page_rotation_applied: 0,
            })
            .collect())
    }

    /// Extract tables from a page using a custom configuration.
    pub fn extract_tables_with_config(
        &self,
        page_index: usize,
        config: crate::structure::spatial_table_detector::TableDetectionConfig,
    ) -> Result<Vec<crate::structure::table_extractor::Table>> {
        use crate::structure::spatial_table_detector::detect_tables_with_lines;

        // Use words instead of spans for better granularity.
        // This ensures that strings with spaces are split into separate columns
        // for the spatial detector. ~keep
        let spans = self.extract_table_word_spans(page_index)?;
        // GH#1656: a page-sized background rectangle passes is_table_primitive
        // (its <1000pt bound doesn't exclude A4/Letter) and then unions every
        // other primitive on the page into one cluster. Drop it here, where the
        // real MediaBox is available; a failed MediaBox lookup leaves every
        // primitive alone rather than filtering on garbage. ~keep
        let media_box = self.get_page_media_box(page_index).ok();
        let lines: Vec<_> = self
            .extract_paths(page_index)?
            .into_iter()
            .filter(|p| p.is_table_primitive())
            .filter(|p| match media_box {
                Some(mb) => !p.is_page_frame_rectangle(mb),
                None => true,
            })
            .collect();

        // Same prose-rejection filter `extract_page_tables` applies to the
        // extract_text/to_markdown/to_html path — this public API called
        // `detect_tables_with_lines` directly with no post-filter at all, so
        // it was already able to fabricate/garble tables on any prose-shaped
        // spatial candidate, independent of anything else in this function. ~keep
        Ok(detect_tables_with_lines(&spans, &lines, &config)
            .into_iter()
            .filter(|t| t.is_real_grid() && !looks_like_prose_table(t))
            .collect())
    }

    /// Extract tables from a specific rectangular region of a page.
    pub fn extract_tables_in_rect(
        &self,
        page_index: usize,
        region: crate::geometry::Rect,
    ) -> Result<Vec<crate::structure::table_extractor::Table>> {
        self.extract_tables_in_rect_with_config(
            page_index,
            region,
            crate::structure::spatial_table_detector::TableDetectionConfig::relaxed(),
        )
    }

    /// Extract tables from a specific region using custom configuration.
    pub fn extract_tables_in_rect_with_config(
        &self,
        page_index: usize,
        region: crate::geometry::Rect,
        config: crate::structure::spatial_table_detector::TableDetectionConfig,
    ) -> Result<Vec<crate::structure::table_extractor::Table>> {
        let tables = self.extract_tables_with_config(page_index, config)?;
        Ok(tables
            .into_iter()
            .filter(|table| {
                if let Some(bbox) = table.bbox {
                    bbox.intersects(&region)
                } else {
                    false
                }
            })
            .collect())
    }

    /// Extract tables from a page using structure tree and spatial detection.
    ///
    /// Tries two strategies in order:
    /// 1. **Structure tree** (tagged PDFs): Finds Table elements in the structure
    ///    tree and extracts cell content via MCID matching.
    /// 2. **Spatial detection** (untagged PDFs): Uses X/Y coordinate clustering
    ///    to detect grid-aligned text as tables.
    ///
    /// Returns early with structure tree tables if found (high confidence).
    pub(super) fn extract_page_tables(
        &self,
        page_index: usize,
        spans: &[TextSpan],
        options: &crate::converters::ConversionOptions,
        text_fallback: bool,
    ) -> Vec<crate::structure::Table> {
        let struct_tree_opt = {
            let cached = self.structure_tree_cache.lock_or_recover().clone();
            match cached {
                Some(tree) => tree,
                None => {
                    let is_marked = self.mark_info().map(|m| m.marked).unwrap_or(false);
                    let has_struct_tree_root = !is_marked
                        && self
                            .catalog()
                            .ok()
                            .and_then(|cat| cat.as_dict().map(|d| d.contains_key("StructTreeRoot")))
                            .unwrap_or(false);
                    let tree = if is_marked || has_struct_tree_root {
                        self.structure_tree().ok().flatten().map(Arc::new)
                    } else {
                        None
                    };
                    *self.structure_tree_cache.lock_or_recover() = Some(tree.clone());
                    tree
                }
            }
        };
        if let Some(ref struct_tree) = struct_tree_opt {
            if self.table_elements_cache.lock_or_recover().is_none() {
                let all = crate::structure::find_table_elements_all_pages(struct_tree);
                *self.table_elements_cache.lock_or_recover() = Some(all);
            }
            let table_elems: Vec<crate::structure::StructElem> = self
                .table_elements_cache
                .lock_or_recover()
                .as_ref()
                .and_then(|c| c.get(&(page_index as u32)))
                .cloned()
                .unwrap_or_default();
            if !table_elems.is_empty() {
                let mut tables = Vec::new();
                for table_elem in &table_elems {
                    match crate::structure::extract_table_from_spans(table_elem, spans) {
                        Ok(mut table) if !table.is_empty() => {
                            if table.bbox.is_none() {
                                let all_mcids: HashSet<u32> = table
                                    .rows
                                    .iter()
                                    .flat_map(|r| r.cells.iter().flat_map(|c| c.mcids.iter().copied()))
                                    .collect();
                                if !all_mcids.is_empty() {
                                    let mut min_x = f32::INFINITY;
                                    let mut min_y = f32::INFINITY;
                                    let mut max_x = f32::NEG_INFINITY;
                                    let mut max_y = f32::NEG_INFINITY;
                                    for span in spans {
                                        if let Some(mcid) = span.mcid
                                            && all_mcids.contains(&mcid)
                                        {
                                            min_x = min_x.min(span.bbox.x);
                                            min_y = min_y.min(span.bbox.y);
                                            max_x = max_x.max(span.bbox.x + span.bbox.width);
                                            max_y = max_y.max(span.bbox.y + span.bbox.height);
                                        }
                                    }
                                    if min_x < max_x && min_y < max_y {
                                        table.bbox = Some(crate::geometry::Rect::new(
                                            min_x,
                                            min_y,
                                            max_x - min_x,
                                            max_y - min_y,
                                        ));
                                    }
                                }
                            }
                            tables.push(table);
                        }
                        _ => {}
                    }
                }
                if !tables.is_empty() {
                    tracing::debug!(target: LOG_TARGET,
                        "Found {} table(s) via structure tree for page {}",
                        tables.len(),
                        page_index
                    );
                    return tables;
                }
            }
        }

        let mut config = options.table_detection_config.clone().unwrap_or_default();
        // Honour the caller's text_fallback choice regardless of the default
        // on `TableDetectionConfig` — `extract_text` / `to_plain_text` pass
        // `text_fallback=false` to opt out of text-only spatial fallback even
        // though the type-level default is `true`. ~keep
        config.text_fallback = text_fallback;

        let paths = self.extract_paths(page_index).unwrap_or_default();

        // Filter to table-relevant paths (lines and rectangles only).
        // Chart/plot pages often have hundreds of curves and fills that
        // extract_edges ignores anyway — passing them through the full
        // detection pipeline wastes O(n²) time. ~keep
        const LINE_TOL: f32 = 2.0;
        // GH#1656: same page-frame furniture filter as extract_tables_with_config
        // above — required here too, since this entry point builds its own
        // `table_paths` and feeds them straight into detect_tables_with_lines
        // without going through that method. ~keep
        let media_box = self.get_page_media_box(page_index).ok();
        let table_paths: Vec<_> = paths
            .into_iter()
            .filter(|p| p.is_horizontal_line(LINE_TOL) || p.is_vertical_line(LINE_TOL) || p.is_rectangle())
            .filter(|p| match media_box {
                Some(mb) => !p.is_page_frame_rectangle(mb),
                None => true,
            })
            .collect();

        // A page with thousands of line/rect paths is a drawing or chart, not a
        // ruled table; skip the O(E²) collinear-join + intersection sweep. Real
        // ruled tables have at most a few hundred edges. (Tagged tables already
        // returned above via the structure tree.) ~keep
        const MAX_TABLE_EDGES: usize = 1500;
        if table_paths.len() > MAX_TABLE_EDGES {
            tracing::debug!(target: LOG_TARGET,
                "Page {} has {} line/rect paths (> {}) — skipping spatial table sweep",
                page_index,
                table_paths.len(),
                MAX_TABLE_EDGES
            );
            return Vec::new();
        }

        if table_paths.is_empty() {
            use crate::structure::spatial_table_detector::TableStrategy;
            let is_text_only = matches!(
                (config.horizontal_strategy, config.vertical_strategy),
                (TableStrategy::Text, TableStrategy::Text)
            );
            if !is_text_only && !config.text_fallback {
                return Vec::new();
            }
            if !is_text_only && config.text_fallback {
                tracing::debug!(target: LOG_TARGET,
                    "No ruling lines on page {} — using text-only spatial fallback",
                    page_index
                );
            }
        }
        let paths = table_paths;

        let word_spans = self.extract_table_word_spans(page_index).unwrap_or_default();

        let input_spans = if !word_spans.is_empty() { &word_spans } else { spans };

        let raw_tables =
            crate::structure::spatial_table_detector::detect_tables_with_lines(input_spans, &paths, &config);

        // Issue 484/486/487: when a logical multi-row table is drawn with a
        // horizontal ruling line between every pair of rows, the line-based
        // detector emits one Table per row strip. Each fragment is a 1- or
        // 2-row table that fails is_real_grid below and gets dropped, after
        // which the cells fall through to paragraph flow with column-based
        // reading order — producing orphan `<p>40000≤Q</p>` /
        // `<p>＜55000</p>` pairs. Consolidate vertically-adjacent fragments
        // that share an identical column structure BEFORE applying
        // is_real_grid so the merged multi-row table survives the filter. ~keep
        let raw_tables = crate::structure::spatial_table_detector::consolidate_adjacent_table_fragments(raw_tables);

        // Step 4: spatial detection without struct-tree backing
        // is prone to false positives on form-style layouts (label-colon-
        // value pairs that align horizontally, form fillable boxes drawn
        // with thin lines). Drop tables that don't look like real grids. ~keep
        let raw_count = raw_tables.len();
        let mut tables: Vec<crate::structure::Table> = raw_tables
            .into_iter()
            .filter(|t| t.is_real_grid())
            // Prose-shape filter — applies to line-based detection too: a
            // PDF with decorative horizontal rules (newsletter mastheads,
            // press-release banners) can hand `is_real_grid` a "wide data
            // table" that is actually wrapped paragraphs partitioned by
            // word x-alignment. Reject those before they reach the
            // converter. See `looks_like_prose_table` for the heuristic. ~keep
            .filter(|t| !looks_like_prose_table(t))
            .collect();

        if raw_count != tables.len() {
            tracing::debug!(target: LOG_TARGET,
                "Spatial table detection: filtered {} non-real-grid candidates on page {} ({} kept)",
                raw_count - tables.len(),
                page_index,
                tables.len(),
            );
        } else if !tables.is_empty() {
            tracing::debug!(target: LOG_TARGET,
                "Found {} table(s) via hybrid spatial detection for page {}",
                tables.len(),
                page_index
            );
        }

        // Text-only spatial fallback for converter paths (to_markdown / to_html).
        //
        // Wide data tables (e.g. sailing-score grids with 16-18 columns) exceed the default
        // `max_table_columns: 15` limit and are rejected by the main pipeline. When the
        // caller explicitly opted in to text-only detection (text_fallback=true), retry with
        // a relaxed config that raises the column ceiling and adjusts tolerances so that
        // genuinely wide data tables are captured.
        //
        // Safety guards:
        // - Only fires when the main pipeline returned no tables (avoids double-counting).
        // - Only fires when the caller is a converter (text_fallback=true).
        // - Skipped for tagged PDFs: the structure tree already provides the authoritative
        //   layout; spatial heuristics produce false-positive tables from structure elements
        //   (e.g. headings detected as single-row tables).
        // - Skipped for predominantly-RTL pages: Arabic/Hebrew text alignment patterns
        //   mimic table columns in spatial heuristics.
        // - When ruling lines exist, spans are filtered to the line-bounded region to
        //   prevent page headers/footers from being erroneously included in the table.
        // - Results must pass is_real_grid() just like main-pipeline tables. ~keep

        // Guard 1 — Tagged PDFs: presence of a structure tree means the document has an
        // explicit semantic layout. Spatial text-only detection would misfire on
        // structure elements (headings, paragraphs) that happen to share a Y band. ~keep
        if config.text_fallback && struct_tree_opt.is_some() {
            tracing::debug!(target: LOG_TARGET,
                "Text-only spatial fallback skipped for page {} — document has a structure tree (tagged PDF)",
                page_index
            );
            return tables;
        }

        // Guard 2 — RTL pages: Arabic and Hebrew text naturally aligns horizontally in
        // patterns that the column-clustering algorithm mistakes for table columns.
        // Skip spatial detection when more than 30 % of the input spans are RTL. ~keep
        if config.text_fallback {
            let rtl_count = input_spans
                .iter()
                .filter(|s| crate::text::bidi::looks_rtl(&s.text))
                .count();
            let rtl_fraction = rtl_count as f32 / input_spans.len().max(1) as f32;
            if rtl_fraction > 0.30 {
                tracing::debug!(target: LOG_TARGET,
                    "Text-only spatial fallback skipped for page {} — {:.0}% RTL spans (threshold 30%)",
                    page_index,
                    rtl_fraction * 100.0
                );
                return tables;
            }
        }

        if config.text_fallback && tables.is_empty() {
            use crate::structure::spatial_table_detector::detect_tables_from_spans_column_aware;
            // Build a relaxed config derived from the caller's config.
            // We only raise the limits known to block wide data tables (e.g. sailing
            // score grids with 16-18 columns that exceed the default max_table_columns=15). ~keep
            let relaxed_config = crate::structure::spatial_table_detector::TableDetectionConfig {
                // Allow up to 25 columns — covers 17-column sailing score tables. ~keep
                max_table_columns: config.max_table_columns.max(25),
                // Tighter column grouping than the default 15 pt so that nearby
                // score columns are not merged into each other. ~keep
                column_tolerance: config.column_tolerance.min(10.0),
                // Looser merge threshold so that columns with slight X scatter
                // (e.g. centred numeric cells) are aggregated correctly. ~keep
                column_merge_threshold: config.column_merge_threshold.max(30.0),
                ..config.clone()
            };

            // When ruling lines are present on the page, restrict text detection to
            // spans that fall within the VERTICAL-LINE Y bounds. Vertical lines
            // define the table's column structure and their Y extent precisely
            // delineates the table rows, excluding page headers and footers which
            // sit above/below the table frame.
            //
            // Note: we use V-line Y bounds specifically (not total path bbox) because
            // H-lines in these PDFs often span the full page height (outer frame),
            // while V-lines are confined to the interior table region. ~keep
            let candidate_spans: Vec<crate::layout::TextSpan>;
            let fallback_spans: &[crate::layout::TextSpan] = {
                let v_lines: Vec<_> = paths.iter().filter(|p| p.is_vertical_line(2.0)).collect();
                if !v_lines.is_empty() {
                    // Rendered extents: a stroke-width-encoded column rule's
                    // drawn bar spans the table height while its geometric
                    // bbox is a ~0pt speck at the midline —
                    // banding on the speck would filter out the table's own
                    // spans. ~keep
                    let vline_y_min = v_lines
                        .iter()
                        .map(|p| p.rendered_bbox().y)
                        .fold(f32::INFINITY, f32::min);
                    let vline_y_max = v_lines
                        .iter()
                        .map(|p| {
                            let r = p.rendered_bbox();
                            r.y + r.height
                        })
                        .fold(f32::NEG_INFINITY, f32::max);
                    // Small margin to include spans whose centres just touch the frame. ~keep
                    const V_MARGIN: f32 = 5.0;
                    candidate_spans = input_spans
                        .iter()
                        .filter(|s| {
                            let cy = s.bbox.y + s.bbox.height * 0.5;
                            cy >= vline_y_min - V_MARGIN && cy <= vline_y_max + V_MARGIN
                        })
                        .cloned()
                        .collect();
                    tracing::debug!(target: LOG_TARGET,
                        "Text fallback (page {}): V-lines Y=[{:.1},{:.1}] — filtered {} spans to {}",
                        page_index,
                        vline_y_min,
                        vline_y_max,
                        input_spans.len(),
                        candidate_spans.len()
                    );
                    &candidate_spans
                } else {
                    input_spans
                }
            };

            let text_candidates = detect_tables_from_spans_column_aware(fallback_spans, &relaxed_config);
            let pre_filter = text_candidates.len();
            let text_tables: Vec<_> = text_candidates
                .into_iter()
                // Text-only detection infers columns from word x-alignment
                // alone; a title + a wrapped body line (two rows) is the
                // signature of ordinary prose, not a table. Require ≥3
                // rows of evidence before promoting to a table. ~keep
                .filter(|t| t.rows.len() >= 3 && t.is_real_grid())
                // Prose split across many "columns" is the dominant
                // false-positive shape for text-only detection on
                // line-less pages: a paragraph wraps to N lines, words
                // cluster into N×K cells, and `is_real_grid` accepts the
                // shape. Real data-table cells almost never end with a
                // comma or semicolon (those punctuation marks belong to
                // running sentences), so a high comma-tail ratio is the
                // most discriminating prose signal we have. ~keep
                .filter(|t| !looks_like_prose_table(t))
                .collect();
            if !text_tables.is_empty() {
                tracing::debug!(target: LOG_TARGET,
                    "Text-only relaxed fallback found {} table(s) on page {} ({} filtered by is_real_grid)",
                    text_tables.len(),
                    page_index,
                    pre_filter - text_tables.len(),
                );
                tables = text_tables;
            }
        }

        tables
    }
}

#[cfg(test)]
mod source_space_tests {
    use super::preserve_source_space;

    #[test]
    fn non_contiguous_source_glyphs_without_whitespace_do_not_gain_a_space() {
        let mut text = "fragment".to_string();

        preserve_source_space(&mut text, false);

        assert_eq!(text, "fragment");
    }

    #[test]
    fn source_whitespace_is_preserved_when_geometry_cannot_show_it() {
        let mut text = "fragment".to_string();

        preserve_source_space(&mut text, true);

        assert_eq!(text, " fragment");
    }
}

use super::*;
use crate::structure::spatial_table_detector::{
    MAX_SOURCE_SPANNED_COLUMNS, PaintedCellOwnership, TableDetectionConfig, TableStrategy,
    certify_source_spanned_wide_table, detect_tables_with_line_ownership, detect_tables_without_painted_recovery,
    source_spanned_rules_present, source_spanned_work_fits,
};
use crate::structure::table_extractor::Table;

type RuledSourceTables = (Vec<Table>, Option<PaintedCellOwnership>);

impl PdfDocument {
    /// Extract tables with independent configurations using one page's words and ruling paths.
    /// Results follow the order of the supplied configurations.
    pub fn extract_tables_with_configs<const N: usize>(
        &self,
        page_index: usize,
        configs: [&crate::structure::spatial_table_detector::TableDetectionConfig; N],
    ) -> Result<[Vec<crate::structure::table_extractor::Table>; N]> {
        self.extract_tables_with_source_ownership(page_index, configs)
            .map(|tables| tables.map(|(tables, _)| tables))
    }

    /// Extract Native tables and retain certificates from accepted painted tilings.
    #[doc(hidden)]
    pub fn extract_tables_with_source_ownership<const N: usize>(
        &self,
        page_index: usize,
        configs: [&crate::structure::spatial_table_detector::TableDetectionConfig; N],
    ) -> Result<[RuledSourceTables; N]> {
        self.extract_ruled_source_tables(page_index, configs, false)
    }

    /// Retry an empty default ruled route for a fully source-spanned wide table.
    #[doc(hidden)]
    pub fn extract_tables_with_wide_source_recovery(
        &self,
        page_index: usize,
        configs: [&TableDetectionConfig; 2],
    ) -> Result<[RuledSourceTables; 2]> {
        self.extract_ruled_source_tables(page_index, configs, true)
    }

    fn extract_ruled_source_tables<const N: usize>(
        &self,
        page_index: usize,
        configs: [&TableDetectionConfig; N],
        recover_wide: bool,
    ) -> Result<[RuledSourceTables; N]> {
        // Use words instead of spans for better granularity.
        // This ensures that strings with spaces are split into separate columns
        // for the spatial detector. ~keep
        let spans = self.extract_table_word_spans(page_index)?;
        #[cfg(test)]
        crate::structure::spatial_table_detector::header_source_diagnostic::event("table_word_spans", || {
            serde_json::json!(
                spans
                    .iter()
                    .enumerate()
                    .map(|(id, span)| serde_json::json!({"id": id, "span": span}))
                    .collect::<Vec<_>>()
            )
        });
        // GH#1656: a page-sized background rectangle passes is_table_primitive
        // (its <1000pt bound doesn't exclude A4/Letter) and then unions every
        // other primitive on the page into one cluster. Drop it here, where the
        // real MediaBox is available; a failed MediaBox lookup leaves every
        // primitive alone rather than filtering on garbage. ~keep
        let media_box = self.get_page_media_box(page_index).ok();
        let lines: Vec<_> = self
            .extract_paths(page_index)?
            .into_iter()
            .filter(|p| {
                #[cfg(test)]
                crate::structure::spatial_table_detector::header_source_diagnostic::event("raw_path", || {
                    serde_json::json!(p)
                });
                p.is_table_primitive()
            })
            .filter(|p| match media_box {
                Some(mb) => !p.is_page_frame_rectangle(mb),
                None => true,
            })
            .collect();

        #[cfg(test)]
        crate::structure::spatial_table_detector::header_source_diagnostic::event("filtered_paths", || {
            serde_json::json!(lines)
        });

        // Same prose-rejection filter `extract_page_tables` applies to the
        // extract_text/to_markdown/to_html path — this public API called
        // `detect_tables_with_lines` directly with no post-filter at all, so
        // it was already able to fabricate/garble tables on any prose-shaped
        // spatial candidate, independent of anything else in this function. ~keep
        let mut results = configs.map(|config| {
            let (mut tables, ownership) = detect_tables_with_line_ownership(&spans, &lines, config);
            tables.retain(|table| {
                is_public_ruled_table(table)
                    || (is_line_only(config)
                        && table.col_count <= config.max_table_columns
                        && certify_source_spanned_wide_table(table, &spans, &lines).is_some()
                        && !looks_like_prose_table(table))
            });
            let ownership = if tables.len() == 1 { ownership } else { None };
            (tables, ownership)
        });
        certify_empty_legacy_routes(&mut results, configs, &spans, &lines);
        if recover_wide
            && results.iter().all(|(tables, _)| tables.is_empty())
            && let Some(config) = configs.last()
            && let Some(last) = results.last_mut()
            && let Some(recovered) = recover_wide_source_table(config, &spans, &lines)
        {
            *last = recovered;
        }
        Ok(results)
    }
}

fn is_line_only(config: &TableDetectionConfig) -> bool {
    config.horizontal_strategy == TableStrategy::Lines && config.vertical_strategy == TableStrategy::Lines
}

fn recover_wide_source_table(
    config: &TableDetectionConfig,
    spans: &[crate::layout::TextSpan],
    lines: &[crate::elements::PathContent],
) -> Option<RuledSourceTables> {
    if !is_line_only(config)
        || config.max_table_columns >= MAX_SOURCE_SPANNED_COLUMNS
        || !source_spanned_work_fits(spans, lines)
        || !source_spanned_rules_present(lines)
    {
        return None;
    }
    let mut retry = config.clone();
    retry.max_table_columns = MAX_SOURCE_SPANNED_COLUMNS;
    let tables = detect_tables_without_painted_recovery(spans, lines, &retry);
    let [table] = <[Table; 1]>::try_from(tables).ok()?;
    if table.col_count <= config.max_table_columns || looks_like_prose_table(&table) {
        return None;
    }
    let mut ownership = certify_source_spanned_wide_table(&table, spans, lines)?;
    ownership.replaces_empty_ruled_route = true;
    Some((vec![table], Some(ownership)))
}

fn is_public_ruled_table(table: &Table) -> bool {
    table.is_real_grid() && !looks_like_prose_table(table)
}

fn certify_empty_legacy_routes<const N: usize>(
    results: &mut [RuledSourceTables; N],
    configs: [&TableDetectionConfig; N],
    spans: &[crate::layout::TextSpan],
    lines: &[crate::elements::PathContent],
) {
    let candidates = results
        .iter_mut()
        .filter_map(|(_, evidence)| evidence.as_mut())
        .filter(|evidence| evidence.double_rule);
    // An empty current tier alone cannot prove the same tier did not cover the page before recovery.
    let mut candidates = candidates.peekable();
    if candidates.peek().is_some()
        && configs.iter().all(|config| {
            !detect_tables_without_painted_recovery(spans, lines, config)
                .iter()
                .any(is_public_ruled_table)
        })
    {
        for evidence in candidates {
            evidence.replaces_empty_ruled_route = true;
        }
    }
}

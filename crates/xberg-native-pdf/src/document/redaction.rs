//! Region erasure and running-header/footer removal.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Mark a specific rectangular region on a page for erasure.
    ///
    /// Content in this region will be excluded from all subsequent text extractions.
    pub fn erase_region(&self, page_index: usize, rect: crate::geometry::Rect) -> Result<()> {
        self.erase_regions
            .lock_or_recover()
            .entry(page_index)
            .or_default()
            .push(rect);
        // Redaction changes a page's spans; drop the span cache. ~keep
        self.page_spans_cache.lock_or_recover().clear();
        self.search_index.lock_or_recover().clear();
        Ok(())
    }

    /// Clear all erase regions for a page.
    pub fn clear_erase_regions(&self, page_index: usize) -> Result<()> {
        self.erase_regions.lock_or_recover().remove(&page_index);
        self.page_spans_cache.lock_or_recover().clear();
        self.search_index.lock_or_recover().clear();
        Ok(())
    }

    /// Identify and remove headers.
    ///
    /// Uses spec-compliant /Artifact tags when available (100% accuracy), or
    /// falls back to heuristic analysis of the top 15% of pages.
    pub fn remove_headers(&self, threshold: f32) -> Result<usize> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err(crate::error::Error::InvalidOperation(
                "Threshold must be between 0.0 and 1.0".to_string(),
            ));
        }
        self.remove_repeated_text(PageArea::Header, threshold)
    }

    /// Identify and remove footers.
    ///
    /// Uses spec-compliant /Artifact tags when available (100% accuracy), or
    /// falls back to heuristic analysis of the bottom 15% of pages.
    pub fn remove_footers(&self, threshold: f32) -> Result<usize> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err(crate::error::Error::InvalidOperation(
                "Threshold must be between 0.0 and 1.0".to_string(),
            ));
        }
        self.remove_repeated_text(PageArea::Footer, threshold)
    }

    /// Identify and remove both headers and footers.
    ///
    /// Prioritizes ISO 32000 spec-compliant /Artifact tags, with a heuristic
    /// fallback for untagged PDFs.
    ///
    /// # Arguments
    /// * `threshold` - Fraction of pages (0.0-1.0) where text must repeat to be removed (heuristic mode only).
    pub fn remove_artifacts(&self, threshold: f32) -> Result<usize> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err(crate::error::Error::InvalidOperation(
                "Threshold must be between 0.0 and 1.0".to_string(),
            ));
        }
        let h = self.remove_headers(threshold)?;
        let f = self.remove_footers(threshold)?;
        Ok(h + f)
    }

    /// Erase every span carrying a spec-compliant `/Artifact` pagination tag
    /// matching `area` (Tagged PDF, 100% accurate). Split out of
    /// [`Self::remove_repeated_text`] to keep it short. ~keep
    fn remove_spec_compliant_pagination_artifacts(&self, area: PageArea, page_count: usize) -> Result<usize> {
        use crate::extractors::text::{ArtifactType, PaginationSubtype};

        let mut removed_count = 0;
        for page_idx in 0..page_count {
            let spans = self.extract_spans(page_idx)?;
            for span in spans {
                if let Some(ArtifactType::Pagination(subtype)) = span.artifact_type {
                    let is_match = match (area, subtype) {
                        (PageArea::Header, PaginationSubtype::Header) => true,
                        (PageArea::Footer, PaginationSubtype::Footer) => true,
                        _ => false,
                    };

                    if is_match {
                        self.erase_region(page_idx, span.bbox)?;
                        removed_count += 1;
                    }
                }
            }
        }
        Ok(removed_count)
    }

    /// Collect every span's text that falls in `area`'s zone (top/bottom 15%
    /// of the page), keyed by its trimmed text, with the `(page, bbox)` of
    /// each occurrence. Split out of [`Self::remove_repeated_text`] to keep
    /// it short. A zone span already tagged `Pagination(Other)` (by the
    /// cross-page running-artifact pass, which verified its position
    /// consistency) is erased here and counted, since a span whose digits vary
    /// per page (`1 com`, `2 com`) never repeats verbatim. ~keep
    fn collect_zone_text_occurrences(
        &self,
        area: PageArea,
        page_count: usize,
    ) -> Result<(
        std::collections::HashMap<String, Vec<(usize, crate::geometry::Rect)>>,
        usize,
    )> {
        use crate::extractors::text::{ArtifactType, PaginationSubtype};
        use std::collections::HashMap;

        let mut tagged_removed = 0;
        let mut occurrences: HashMap<String, Vec<(usize, crate::geometry::Rect)>> = HashMap::new();
        for page_idx in 0..page_count {
            let height = self.get_page_media_box(page_idx)?.3;
            let zone = match area {
                PageArea::Header => height * 0.85,
                PageArea::Footer => height * 0.15,
            };

            let spans = self.extract_spans(page_idx)?;
            for span in spans.iter() {
                let is_in_zone = match area {
                    PageArea::Header => span.bbox.y > zone,
                    PageArea::Footer => (span.bbox.y + span.bbox.height) < zone,
                };

                if !is_in_zone {
                    continue;
                }
                if matches!(
                    span.artifact_type,
                    Some(ArtifactType::Pagination(PaginationSubtype::Other))
                ) {
                    self.erase_region(page_idx, span.bbox)?;
                    tagged_removed += 1;
                    continue;
                }
                let text = span.text.trim().to_string();
                if text.len() > 3 && !text.chars().all(|c| c.is_numeric()) {
                    occurrences.entry(text).or_default().push((page_idx, span.bbox));
                }
            }
        }
        Ok((occurrences, tagged_removed))
    }

    /// Erase every occurrence group that repeats on at least `min_occurrences`
    /// distinct pages AND stays position-locked within tolerance (a genuine
    /// running header/footer, not a recurring form label). Split out of
    /// [`Self::remove_repeated_text`] to keep it short. ~keep
    fn erase_position_locked_occurrences(
        &self,
        occurrences: std::collections::HashMap<String, Vec<(usize, crate::geometry::Rect)>>,
        min_occurrences: usize,
    ) -> Result<usize> {
        use std::collections::HashSet;

        // Genuine running headers/footers are position-locked: the same string
        // lands at the same x/y on every page. A form label or instruction that
        // merely happens to recur — e.g. "Check if:", "(see instructions)",
        // "Name(s) shown on return" on a multi-page tax form — drifts in x or y
        // between pages. Requiring positional consistency separates "recurs
        // because it is chrome" from "recurs because it is the same real label"
        // without deleting content. ~keep
        const POS_TOL_X: f32 = 40.0;
        const POS_TOL_Y: f32 = 24.0;

        let mut removed_count = 0;
        for (_text, occs) in occurrences {
            let distinct_pages: HashSet<usize> = occs.iter().map(|(p, _)| *p).collect();
            if distinct_pages.len() < min_occurrences {
                continue;
            }

            let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
            let (mut min_y, mut max_y) = (f32::MAX, f32::MIN);
            for (_, bbox) in &occs {
                min_x = min_x.min(bbox.x);
                max_x = max_x.max(bbox.x);
                min_y = min_y.min(bbox.y);
                max_y = max_y.max(bbox.y);
            }
            if (max_x - min_x) > POS_TOL_X || (max_y - min_y) > POS_TOL_Y {
                continue;
            }

            for (page_idx, bbox) in occs {
                self.erase_region(page_idx, bbox)?;
                removed_count += 1;
            }
        }

        Ok(removed_count)
    }

    /// Helper to remove repeated text in a specific page area.
    fn remove_repeated_text(&self, area: PageArea, threshold: f32) -> Result<usize> {
        let page_count = self.page_count()?;
        if page_count < 1 {
            return Ok(0);
        }

        // 1. Spec-Compliant Removal (Priority)
        // If the PDF uses /Artifact tags (Tagged PDF), we use those directly as they are 100% accurate.
        // ~keep
        let removed_count = self.remove_spec_compliant_pagination_artifacts(area, page_count)?;
        if removed_count > 0 {
            tracing::info!(target: LOG_TARGET,
                count = removed_count,
                area = if area == PageArea::Header { "headers" } else { "footers" },
                "removed spec-compliant artifacts"
            );
            return Ok(removed_count);
        }

        if page_count < 2 {
            return Ok(0);
        }

        // Sanitize threshold to avoid min_occurrences becoming 0 for invalid inputs. ~keep
        let clamped_threshold = if threshold.is_finite() {
            threshold.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let raw_min = (page_count as f32 * clamped_threshold).ceil();
        let min_occurrences = if raw_min < 1.0 { 1 } else { raw_min as usize };

        // Each entry records the IN-ZONE occurrences of a repeated string as
        // (page, bbox). Keeping the bbox (not just the page set) lets us both
        // (a) erase only the header/footer occurrence and never an identically
        // worded span elsewhere on the page, and (b) require the occurrences to
        // share a position before treating the string as chrome. ~keep
        let (occurrences, tagged_removed) = self.collect_zone_text_occurrences(area, page_count)?;

        Ok(tagged_removed + self.erase_position_locked_occurrences(occurrences, min_occurrences)?)
    }

    /// Erase existing header content.
    ///
    /// Identifies existing text in the header area (top 15%) and marks it for erasure.
    pub fn erase_header(&self, page_index: usize) -> Result<()> {
        self.erase_page_area_content(page_index, PageArea::Header)
    }

    /// Deprecated: Use `erase_header` instead.
    #[deprecated(note = "use erase_header instead")]
    pub fn edit_header(&self, page_index: usize) -> Result<()> {
        self.erase_header(page_index)
    }

    /// Erase existing footer content.
    ///
    /// Identifies existing text in the footer area (bottom 15%) and marks it for erasure.
    pub fn erase_footer(&self, page_index: usize) -> Result<()> {
        self.erase_page_area_content(page_index, PageArea::Footer)
    }

    /// Deprecated: Use `erase_footer` instead.
    #[deprecated(note = "use erase_footer instead")]
    pub fn edit_footer(&self, page_index: usize) -> Result<()> {
        self.erase_footer(page_index)
    }

    /// Erase both header and footer content.
    ///
    /// This is a convenience method that calls both erase_header and erase_footer.
    pub fn erase_artifacts(&self, page_index: usize) -> Result<()> {
        self.erase_header(page_index)?;
        self.erase_footer(page_index)?;
        Ok(())
    }

    /// Helper to erase content in a specific page area.
    fn erase_page_area_content(&self, page_index: usize, area: PageArea) -> Result<()> {
        let height = self.get_page_media_box(page_index)?.3;
        let zone = match area {
            PageArea::Header => height * 0.85,
            PageArea::Footer => height * 0.15,
        };

        let spans = self.extract_spans(page_index)?;
        for span in spans {
            let is_in_zone = match area {
                PageArea::Header => span.bbox.y > zone,
                PageArea::Footer => (span.bbox.y + span.bbox.height) < zone,
            };

            if is_in_zone {
                self.erase_region(page_index, span.bbox)?;
            }
        }
        Ok(())
    }

    /// Determine if a space should be inserted between two text spans.
    ///
    /// According to PDF spec (ISO 32000-1:2008 Section 9.3.3), word spacing
    /// only applies to actual space characters (0x20). Many PDFs (especially
    /// academic papers) use precise positioning instead of space characters.
    /// This function detects such gaps and inserts spaces heuristically.
    ///
    /// # Algorithm
    /// 1. Check if spans are on the same line (Y positions similar)
    /// 2. Calculate horizontal gap between end of prev span and start of current span
    /// 3. Insert space if gap exceeds threshold (0.25 × font size)
    ///
    /// # Arguments
    /// * `prev` - Previous text span
    /// * `current` - Current text span
    ///
    /// Filter leaked PDF internal metadata from extracted text.
    ///
    /// Some PDFs embed inline ColorSpace definitions (CalRGB, CalGray, Lab) that
    /// get parsed as text content. This removes known metadata patterns like
    /// "WhitePoint [ ... ]", "BlackPoint [ ... ]", "Gamma [ ... ]", "Matrix [ ... ]".
    pub(super) fn filter_leaked_metadata(text: &str) -> String {
        // Known PDF metadata keys that should never appear in extracted text.
        // These come from CalRGB/CalGray/Lab color space dictionaries. ~keep
        const METADATA_PATTERNS: &[&str] = &["WhitePoint", "BlackPoint", "Gamma", "Matrix", "CalRGB", "CalGray"];

        if !METADATA_PATTERNS.iter().any(|p| text.contains(p)) {
            return text.to_string();
        }

        let mut result = String::with_capacity(text.len());
        for line in text.lines() {
            let trimmed = line.trim();
            let is_metadata = METADATA_PATTERNS.iter().any(|pattern| {
                if let Some(rest) = trimmed.strip_prefix(pattern) {
                    let rest = rest.trim_start();
                    rest.is_empty() || rest.starts_with('[') || rest.starts_with('/') || rest.starts_with('<')
                } else {
                    false
                }
            });

            if !is_metadata {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(line);
            }
        }

        result
    }
}

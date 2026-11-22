//! PDF-specific configuration.
//!
//! Defines PDF extraction options including metadata handling, image extraction,
//! password management, and hierarchy extraction for document structure analysis.

use std::fmt;

use serde::{Deserialize, Serialize};

/// PDF extraction backend selection.
///
/// Controls which engine parses and renders PDF documents. Wire format is
/// snake_case in all serializers (JSON, TOML, YAML). Defaults to
/// [`PdfBackend::Native`] -- selecting anything else never changes behavior
/// for a caller who does not opt in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfBackend {
    /// xberg's own pure-Rust PDF engine (default), `crates/xberg-native-pdf`.
    #[default]
    Native,
    /// pdfium -- Google's PDFium engine, gated behind the `pdf-pdfium` Cargo
    /// feature (#700 added the selection level; #702 added the extraction
    /// engine, `extractors::pdf::pdfium_engine`, deliberately smaller in scope
    /// than `Native` -- see that module's doc comment for exactly what it
    /// extracts). A build without the `pdf-pdfium` feature rejects this at the
    /// CLI validation layer rather than silently falling back to `Native`.
    /// A build *with* the feature still enforces the selection at extraction
    /// time (`extractors::pdf::PdfExtractor::extract_core`), because that is
    /// the one dispatch point every caller -- CLI, library use, API/MCP
    /// servers, language bindings -- passes through; see that function's doc
    /// comment for why the enforcement lives there and not only in CLI
    /// validation.
    Pdfium,
}

impl std::str::FromStr for PdfBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().replace('-', "_").as_str() {
            // No alias for the pre-1.1.0 spelling: the rename is deliberate and a silently
            // accepted old name would keep it alive in configs indefinitely. ~keep
            "native" => Ok(Self::Native),
            "pdfium" => Ok(Self::Pdfium),
            other => Err(format!("unknown PDF backend '{other}'; expected: native, pdfium")),
        }
    }
}

impl fmt::Display for PdfBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfBackend::Native => write!(f, "native"),
            PdfBackend::Pdfium => write!(f, "pdfium"),
        }
    }
}

/// PDF-specific configuration.
#[cfg(feature = "pdf")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfConfig {
    /// Extract images from PDF
    #[serde(default)]
    pub extract_images: bool,

    /// Extract tables from PDF.
    ///
    /// When `true` (default), runs the native engine's grid detector and, if it
    /// finds nothing, falls back to the heuristic text-layer reconstruction in
    /// `pdf::native::table::extract_tables_heuristic`. Set to `false` to skip
    /// both passes — `tables` will then be empty in the result.
    #[serde(default = "default_true")]
    pub extract_tables: bool,

    /// List of passwords to try when opening encrypted PDFs
    #[serde(default)]
    pub passwords: Option<Vec<String>>,

    /// Extract PDF metadata
    #[serde(default = "default_true")]
    pub extract_metadata: bool,

    /// Hierarchy extraction configuration (None = hierarchy extraction disabled)
    #[serde(default)]
    pub hierarchy: Option<HierarchyConfig>,

    /// Extract PDF annotations (text notes, highlights, links, stamps).
    /// Default: false
    #[serde(default)]
    pub extract_annotations: bool,

    /// Top margin fraction (0.0–1.0) of page height to exclude headers/running heads.
    /// Ignored when `ContentFilterConfig.include_headers` is `true`.
    /// Effective nonzero margins require per-page OCR so geometry can be filtered;
    /// document-capable OCR backends use their image-processing path in that case.
    /// Default: 0.0 (disabled; set explicitly to filter header content)
    #[serde(default)]
    pub top_margin_fraction: Option<f32>,

    /// Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers.
    /// Ignored when `ContentFilterConfig.include_footers` is `true`.
    /// Effective nonzero margins require per-page OCR so geometry can be filtered;
    /// document-capable OCR backends use their image-processing path in that case.
    /// Default: 0.0 (disabled; set explicitly to filter footer content)
    #[serde(default)]
    pub bottom_margin_fraction: Option<f32>,

    /// Allow single-column pseudo tables in extraction results.
    ///
    /// By default, tables with fewer than 2 columns (layout-guided) or 3 columns
    /// (heuristic) are rejected. When `true`, the minimum column count is relaxed
    /// to 1, allowing single-column structured data (glossaries, itemized lists)
    /// to be emitted as tables. Other quality filters (density, sparsity, prose
    /// detection) still apply.
    #[serde(default)]
    pub allow_single_column_tables: bool,

    /// Perform OCR on inline images extracted from PDF pages and attach the
    /// recognized text to each `ExtractedImage.ocr_result`. Uses the backend
    /// selected by `ExtractionConfig.ocr`, or the default OCR backend when no
    /// OCR configuration is supplied. Requires the `ocr` or `ocr-pipeline`
    /// feature. Per-image failures degrade gracefully (the image is returned
    /// without OCR text rather than failing the whole extraction). Default:
    /// `false`.
    #[serde(default)]
    pub ocr_inline_images: bool,

    /// Extract AcroForm and XFA form fields into `ExtractedDocument.form_fields`.
    ///
    /// When `true` (default), reads the document's interactive form structure
    /// (field names, types, values, widget geometry). Cheap and strictly
    /// additive — non-form PDFs simply yield an empty list. Set to `false` to
    /// skip the form pass entirely.
    #[serde(default = "default_true")]
    pub extract_form_fields: bool,

    /// Reorder extracted text by layout-detected reading order.
    ///
    /// When `true`, projects text spans onto layout-detected regions, performs
    /// column detection, and emits spans in natural reading order (important
    /// for multi-column academic PDFs). It also repairs 90/180/270-degree
    /// rotated text runs — sideways tables and captions — that otherwise read
    /// word-reversed and glued (GH#1358); see
    /// `crate::extractors::pdf::reading_order` for the rotation-handling
    /// details and its limits. Requires the `layout-detection` feature and a
    /// page for which layout detection actually produces hints: a page with
    /// no detected regions falls back to the original, unrepaired extraction
    /// order even with this enabled. Independent of
    /// [`LayoutStrategy`](crate::core::config::LayoutStrategy), which only
    /// controls whether layout detection runs at all — enabling `Always` or
    /// `Auto` alone does not turn reordering on. Defaults to `false`.
    #[serde(default)]
    pub reading_order: bool,

    /// Which engine parses and renders this PDF.
    ///
    /// Defaults to [`PdfBackend::Native`]. Selecting [`PdfBackend::Pdfium`]
    /// requires the `pdf-pdfium` feature and is rejected otherwise; the
    /// pdfium engine is also deliberately narrower in scope than `Native` --
    /// see [`PdfBackend`] and `extractors::pdf::pdfium_engine` for details.
    #[serde(default)]
    pub backend: PdfBackend,

    /// Return each page's vector geometry (lines, curves and filled shapes).
    ///
    /// When `true`, every `PageContent` of a PDF carries a `geometry` value: each
    /// painted path's subpaths, fill colour, stroke colour/width/dash and Optional
    /// Content layer, in page coordinates with the content-stream and Form XObject
    /// transforms already applied. Text bounds use the same coordinate frame, so a
    /// label's box and the shape it annotates are directly comparable.
    ///
    /// Off by default because a dense drawing sheet can hold more than ten thousand
    /// paths and this output reports every one of them. Asking for it also turns on
    /// per-page tracking (`pages.extract_pages`) because that is where the geometry is
    /// returned.
    ///
    /// PDF only, and only on the native backend. Geometry is `None` on non-PDF
    /// documents, on a page with no drawn path, and when the page's content stream cannot
    /// be parsed.
    #[serde(default)]
    pub extract_vector_geometry: bool,

    /// Closed-region recovery over each page's vector paths.
    ///
    /// When enabled (default off), line segments are joined into a planar graph,
    /// door-sized gaps are closed and every closed face is returned on
    /// `pages[].geometry.regions` with its area in square page points and the
    /// page text inside it. `snap_tolerance` and `gap_tolerance` control how
    /// aggressively endpoints are merged and gaps are bridged.
    ///
    /// Requires the `plan-regions` Cargo feature; enabling it without that
    /// feature is rejected during validation. Asking for regions also turns on
    /// vector-geometry extraction internally, but the raw paths are only
    /// reported when [`extract_vector_geometry`](Self::extract_vector_geometry)
    /// is set.
    #[serde(default)]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.5.0"))]
    pub plan_regions: Option<PlanRegionConfig>,
}

/// Settings for closed-region recovery on a PDF page's vector paths.
#[cfg(feature = "pdf")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanRegionConfig {
    /// Run the closed-region pass. Off by default per page/document.
    #[serde(default)]
    pub enabled: bool,

    /// Endpoints within this many page points of each other are treated as one
    /// node. Default `0.5`, half a point.
    #[serde(default = "default_snap_tolerance")]
    pub snap_tolerance: f32,

    /// Open wall ends up to this many page points apart are joined (door
    /// openings). Default `24.0` (two feet at 12 points per foot); set `0.0` to
    /// disable gap closing.
    #[serde(default = "default_gap_tolerance")]
    pub gap_tolerance: f32,

    /// Maximum line segments considered on one page. A denser sheet is skipped
    /// rather than allowed to dominate the extraction. Default `20_000`.
    #[serde(default = "default_max_segments")]
    pub max_segments: usize,
}

#[cfg(feature = "pdf")]
impl Default for PlanRegionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            snap_tolerance: default_snap_tolerance(),
            gap_tolerance: default_gap_tolerance(),
            max_segments: default_max_segments(),
        }
    }
}

#[cfg(feature = "pdf")]
fn default_snap_tolerance() -> f32 {
    0.5
}

#[cfg(feature = "pdf")]
fn default_gap_tolerance() -> f32 {
    24.0
}

#[cfg(feature = "pdf")]
fn default_max_segments() -> usize {
    20_000
}

/// Hierarchy extraction configuration for PDF text structure analysis.
///
/// Enables extraction of document hierarchy levels (H1-H6) based on font size
/// clustering and semantic analysis. When enabled, hierarchical blocks are
/// included in page content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HierarchyConfig {
    /// Enable hierarchy extraction
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Number of font size clusters to use for hierarchy levels (1-7)
    ///
    /// Default: 3, which provides two heading levels plus body text.
    /// Larger values create more fine-grained hierarchy levels.
    #[serde(default = "default_k_clusters")]
    pub k_clusters: usize,

    /// Include bounding box information in hierarchy blocks
    #[serde(default = "default_true")]
    pub include_bbox: bool,
}

#[cfg(feature = "pdf")]
impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            extract_images: false,
            extract_tables: true,
            passwords: None,
            extract_metadata: true,
            hierarchy: None,
            extract_annotations: false,
            top_margin_fraction: None,
            bottom_margin_fraction: None,
            allow_single_column_tables: false,
            ocr_inline_images: false,
            extract_form_fields: true,
            reading_order: false,
            backend: PdfBackend::default(),
            extract_vector_geometry: false,
            plan_regions: None,
        }
    }
}

#[cfg(feature = "pdf")]
impl PdfConfig {
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    pub(crate) fn has_nonzero_raw_margins(&self) -> bool {
        self.top_margin_fraction.unwrap_or_default() != 0.0 || self.bottom_margin_fraction.unwrap_or_default() != 0.0
    }

    /// Validate PDF-specific extraction settings.
    ///
    /// # Errors
    ///
    /// Returns [`crate::XbergError::Validation`] when a configured page margin is
    /// non-finite or outside `[0.0, 1.0]`, when hierarchy clustering requests
    /// fewer than one or more than seven clusters, or when an enabled option is
    /// unavailable in the current feature set.
    pub fn validate(&self) -> crate::Result<()> {
        validate_margin_fraction("pdf_options.top_margin_fraction", self.top_margin_fraction)?;
        validate_margin_fraction("pdf_options.bottom_margin_fraction", self.bottom_margin_fraction)?;

        if let Some(regions) = &self.plan_regions {
            validate_tolerance("pdf_options.plan_regions.snap_tolerance", regions.snap_tolerance)?;
            validate_tolerance("pdf_options.plan_regions.gap_tolerance", regions.gap_tolerance)?;
            if regions.max_segments == 0 {
                return Err(crate::XbergError::validation(
                    "pdf_options.plan_regions.max_segments must be greater than zero",
                ));
            }
            #[cfg(not(feature = "plan-regions"))]
            if regions.enabled {
                return Err(crate::XbergError::validation(
                    "pdf_options.plan_regions requires the plan-regions feature",
                ));
            }
        }

        if let Some(hierarchy) = &self.hierarchy
            && !(1..=7).contains(&hierarchy.k_clusters)
        {
            return Err(crate::XbergError::validation(format!(
                "pdf_options.hierarchy.k_clusters must be between 1 and 7 inclusive, got {}",
                hierarchy.k_clusters
            )));
        }

        #[cfg(not(feature = "layout-detection"))]
        if self.reading_order {
            return Err(crate::XbergError::validation(
                "pdf_options.reading_order requires the layout-detection feature",
            ));
        }

        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        if self.ocr_inline_images {
            return Err(crate::XbergError::validation(
                "pdf_options.ocr_inline_images requires the ocr or ocr-pipeline feature",
            ));
        }

        Ok(())
    }
}

#[cfg(feature = "pdf")]
fn validate_margin_fraction(field: &str, value: Option<f32>) -> crate::Result<()> {
    if let Some(value) = value
        && (!value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Err(crate::XbergError::validation(format!(
            "{field} must be finite and between 0.0 and 1.0 inclusive, got {value}"
        )));
    }

    Ok(())
}

#[cfg(feature = "pdf")]
fn validate_tolerance(field: &str, value: f32) -> crate::Result<()> {
    if !value.is_finite() || value < 0.0 {
        return Err(crate::XbergError::validation(format!(
            "{field} must be finite and non-negative, got {value}"
        )));
    }
    Ok(())
}

impl Default for HierarchyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            k_clusters: 3,
            include_bbox: true,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_k_clusters() -> usize {
    3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    fn only_nonzero_raw_margins_require_projection() {
        for (top, bottom, expected) in [
            (None, None, false),
            (Some(0.0), None, false),
            (None, Some(0.0), false),
            (Some(-0.0), Some(0.0), false),
            (Some(0.05), Some(0.0), true),
            (Some(0.0), Some(0.10), true),
            (None, Some(0.10), true),
            (Some(f32::EPSILON), None, true),
        ] {
            let pdf = PdfConfig {
                top_margin_fraction: top,
                bottom_margin_fraction: bottom,
                ..Default::default()
            };
            assert_eq!(
                pdf.has_nonzero_raw_margins(),
                expected,
                "raw margins: {top:?}, {bottom:?}"
            );
        }
    }

    #[test]
    #[cfg(all(feature = "pdf", not(feature = "layout-detection")))]
    fn should_reject_reading_order_without_layout_detection() {
        let config = PdfConfig {
            reading_order: true,
            ..Default::default()
        };

        let error = config.validate().expect_err("unsupported reading order must fail");
        assert!(error.to_string().contains("layout-detection"));
    }

    #[test]
    #[cfg(all(feature = "pdf", not(any(feature = "ocr", feature = "ocr-pipeline"))))]
    fn should_reject_inline_image_ocr_without_ocr_support() {
        let config = PdfConfig {
            ocr_inline_images: true,
            ..Default::default()
        };

        let error = config.validate().expect_err("unsupported inline image OCR must fail");
        assert!(error.to_string().contains("ocr or ocr-pipeline"));
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn should_reject_non_finite_or_out_of_range_pdf_margins() {
        for (field, top_margin_fraction, bottom_margin_fraction) in [
            ("top_margin_fraction", Some(f32::NAN), None),
            ("top_margin_fraction", Some(f32::INFINITY), None),
            ("top_margin_fraction", Some(-0.01), None),
            ("top_margin_fraction", Some(1.01), None),
            ("bottom_margin_fraction", None, Some(f32::NEG_INFINITY)),
            ("bottom_margin_fraction", None, Some(-0.01)),
            ("bottom_margin_fraction", None, Some(1.01)),
        ] {
            let config = PdfConfig {
                top_margin_fraction,
                bottom_margin_fraction,
                ..PdfConfig::default()
            };

            let error = config.validate().expect_err("invalid PDF margin must be rejected");
            assert!(
                error.to_string().contains(field),
                "error must identify {field}, got: {error}"
            );
        }
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn should_accept_inclusive_pdf_margin_bounds() {
        for value in [0.0, 1.0] {
            let config = PdfConfig {
                top_margin_fraction: Some(value),
                bottom_margin_fraction: Some(value),
                ..PdfConfig::default()
            };

            config.validate().expect("inclusive PDF margin bound must be valid");
        }
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn should_reject_hierarchy_cluster_count_outside_supported_range() {
        for k_clusters in [0, 8] {
            let config = PdfConfig {
                hierarchy: Some(HierarchyConfig {
                    k_clusters,
                    ..HierarchyConfig::default()
                }),
                ..PdfConfig::default()
            };

            let error = config
                .validate()
                .expect_err("unsupported hierarchy cluster count must be rejected");
            assert!(
                error.to_string().contains("k_clusters"),
                "error must identify k_clusters, got: {error}"
            );
        }
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_hierarchy_config_default() {
        let config = HierarchyConfig::default();
        assert!(config.enabled);
        assert_eq!(config.k_clusters, 3);
        assert!(config.include_bbox);
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_hierarchy_config_disabled() {
        let config = HierarchyConfig {
            enabled: false,
            k_clusters: 3,
            include_bbox: false,
        };
        assert!(!config.enabled);
        assert_eq!(config.k_clusters, 3);
        assert!(!config.include_bbox);
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn extract_vector_geometry_is_off_by_default_and_read_from_its_key() {
        assert!(
            !PdfConfig::default().extract_vector_geometry,
            "a dense sheet's paths are an opt-in output"
        );

        let with_key: PdfConfig =
            serde_json::from_str(r#"{"extract_vector_geometry": true}"#).expect("a config with the key parses");
        assert!(with_key.extract_vector_geometry);

        let without_key: PdfConfig =
            serde_json::from_str(r#"{"extract_tables": true}"#).expect("a config without the key parses");
        assert!(!without_key.extract_vector_geometry);
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn plan_regions_is_off_by_default_and_reads_its_key() {
        assert!(
            PdfConfig::default().plan_regions.is_none(),
            "closed regions are an opt-in output"
        );

        let with_key: PdfConfig = serde_json::from_str(
            r#"{"plan_regions": {"enabled": true, "snap_tolerance": 0.25, "gap_tolerance": 12.0}}"#,
        )
        .expect("a config with the key parses");
        let regions = with_key.plan_regions.expect("the plan-regions config is present");
        assert!(regions.enabled);
        assert_eq!(regions.snap_tolerance, 0.25);
        assert_eq!(regions.gap_tolerance, 12.0);
        assert_eq!(regions.max_segments, 20_000, "the cap keeps its documented default");
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn should_reject_invalid_plan_region_settings() {
        for (snap, gap, max_segments) in [
            (f32::NAN, 24.0, 20_000),
            (-0.5, 24.0, 20_000),
            (0.5, f32::INFINITY, 20_000),
            (0.5, -1.0, 20_000),
            (0.5, 24.0, 0),
        ] {
            let config = PdfConfig {
                plan_regions: Some(PlanRegionConfig {
                    enabled: true,
                    snap_tolerance: snap,
                    gap_tolerance: gap,
                    max_segments,
                }),
                ..PdfConfig::default()
            };
            assert!(
                config.validate().is_err(),
                "snap={snap}, gap={gap}, max_segments={max_segments} must be rejected"
            );
        }
    }

    #[test]
    #[cfg(all(feature = "pdf", not(feature = "plan-regions")))]
    fn should_reject_plan_regions_without_the_feature() {
        let config = PdfConfig {
            plan_regions: Some(PlanRegionConfig::default()),
            ..PdfConfig::default()
        };
        let config = PdfConfig {
            plan_regions: config.plan_regions.map(|regions| PlanRegionConfig {
                enabled: true,
                ..regions
            }),
            ..config
        };
        let error = config.validate().expect_err("the feature is required");
        assert!(error.to_string().contains("plan-regions"), "got: {error}");
    }

    #[test]
    #[cfg(all(feature = "pdf", feature = "plan-regions"))]
    fn should_accept_plan_regions_with_the_feature() {
        let config = PdfConfig {
            plan_regions: Some(PlanRegionConfig {
                enabled: true,
                ..PlanRegionConfig::default()
            }),
            ..PdfConfig::default()
        };
        config.validate().expect("the feature is available");
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_pdf_config_custom_margins() {
        let config = PdfConfig {
            extract_images: false,
            extract_tables: true,
            passwords: None,
            extract_metadata: true,
            hierarchy: None,
            extract_annotations: false,
            top_margin_fraction: Some(0.10),
            bottom_margin_fraction: Some(0.08),
            allow_single_column_tables: false,
            ocr_inline_images: false,
            extract_form_fields: true,
            reading_order: false,
            backend: PdfBackend::Native,
            extract_vector_geometry: false,
            plan_regions: None,
        };
        assert_eq!(config.top_margin_fraction, Some(0.10));
        assert_eq!(config.bottom_margin_fraction, Some(0.08));
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn pdf_config_omitting_extract_form_fields_defaults_to_true() {
        let json = r#"{"extract_tables": true, "extract_metadata": true}"#;
        let config: PdfConfig = serde_json::from_str(json).unwrap();
        assert!(
            config.extract_form_fields,
            "omitted extract_form_fields must default to true (default-on)"
        );
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn pdf_config_omitting_reading_order_defaults_to_false() {
        // reading_order uses `#[serde(default)]` (bool default = false).
        let json = r#"{"extract_tables": true, "extract_metadata": true}"#;
        let config: PdfConfig = serde_json::from_str(json).unwrap();
        assert!(!config.reading_order, "omitted reading_order must default to false");
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn pdf_config_new_fields_round_trip() {
        let config = PdfConfig {
            extract_form_fields: false,
            reading_order: true,
            ..PdfConfig::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: PdfConfig = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.extract_form_fields);
        assert!(deserialized.reading_order);
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn pdf_config_omitting_backend_defaults_to_native() {
        let json = r#"{"extract_tables": true, "extract_metadata": true}"#;
        let config: PdfConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.backend,
            PdfBackend::Native,
            "omitted backend must default to native -- no build's behavior may change unless a caller opts in"
        );
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn pdf_backend_round_trips_through_json_as_snake_case() {
        let config = PdfConfig {
            backend: PdfBackend::Pdfium,
            ..PdfConfig::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(
            json.get("backend").and_then(|v| v.as_str()),
            Some("pdfium"),
            "wire format must be snake_case, got {json}"
        );
        let deserialized: PdfConfig = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.backend, PdfBackend::Pdfium);
    }

    #[test]
    fn pdf_backend_from_str_is_case_insensitive() {
        assert_eq!("native".parse::<PdfBackend>().unwrap(), PdfBackend::Native);
        assert_eq!("NATIVE".parse::<PdfBackend>().unwrap(), PdfBackend::Native);
        assert_eq!("pdfium".parse::<PdfBackend>().unwrap(), PdfBackend::Pdfium);
        assert_eq!("PDFium".parse::<PdfBackend>().unwrap(), PdfBackend::Pdfium);
    }

    /// The pre-1.1.0 spelling must be REJECTED, not silently accepted.
    ///
    /// This is a breaking change made on purpose: an alias would keep the old name alive in
    /// user configs forever, which is the opposite of the rename's point. A caller who set
    /// `backend = "pdf_oxide"` gets an error naming the valid values rather than a silent
    /// behaviour they did not ask for.
    #[test]
    fn pdf_backend_from_str_rejects_the_pre_rename_spelling() {
        for old in ["pdf_oxide", "pdf-oxide", "PDF-OXIDE"] {
            let error = old.parse::<PdfBackend>().unwrap_err();
            assert!(
                error.contains("native"),
                "rejecting {old} must point at the new name, got: {error}"
            );
        }
    }

    #[test]
    fn pdf_backend_from_str_rejects_unknown_value_and_lists_valid_ones() {
        let error = "xyz".parse::<PdfBackend>().unwrap_err();
        assert!(error.contains("native"), "error must list native, got: {error}");
        assert!(error.contains("pdfium"), "error must list pdfium, got: {error}");
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn pdf_config_rejects_typod_backend_key() {
        let json = serde_json::json!({"extract_tables": true, "pdf_backend": "pdfium"});
        let error = serde_json::from_value::<PdfConfig>(json).expect_err("unknown keys must fail deserialization");
        assert!(
            error.to_string().contains("pdf_backend"),
            "the error must name the unknown field: {error}"
        );
    }
}

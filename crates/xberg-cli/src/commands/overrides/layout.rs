//! `--layout*` overrides and the plain-output-wastes-layout warning.

#[cfg(feature = "layout-detection")]
use anyhow::{Result, bail};
use xberg::ExtractionConfig;

use super::ExtractionOverrides;

impl ExtractionOverrides {
    /// Reject invalid or contradictory `--layout*` flag combinations.
    #[cfg(feature = "layout-detection")]
    pub(super) fn validate_layout(&self) -> Result<()> {
        if let Some(conf) = self.layout_confidence
            && !(0.0..=1.0).contains(&conf)
        {
            bail!("Invalid layout confidence: {conf}. Value must be between 0.0 and 1.0.");
        }
        if self.layout == Some(false) && (self.layout_confidence.is_some() || self.layout_table_model.is_some()) {
            bail!("--layout false cannot be combined with --layout-confidence or --layout-table-model");
        }
        if self.layout == Some(false) && self.layout_strategy.is_some() {
            bail!("--layout false cannot be combined with --layout-strategy");
        }
        if self.layout == Some(false) && self.layout_pages.is_some() {
            bail!("--layout false cannot be combined with --layout-pages");
        }
        if let Some(ref strategy) = self.layout_strategy
            && strategy.parse::<xberg::LayoutStrategy>().is_err()
        {
            bail!("Invalid layout strategy: '{strategy}'. Valid: always, auto.");
        }
        Ok(())
    }

    #[allow(unused_variables)]
    pub(super) fn apply_layout(&self, config: &mut ExtractionConfig) {
        #[cfg(feature = "layout-detection")]
        {
            if self.layout == Some(false) {
                config.layout = None;
                return;
            }

            #[cfg(feature = "formula-recognition")]
            let has_formula_flag = self.layout_formula_model.is_some();
            #[cfg(not(feature = "formula-recognition"))]
            let has_formula_flag = false;
            let has_layout_flag = self.layout == Some(true)
                || self.layout_confidence.is_some()
                || self.layout_table_model.is_some()
                || self.layout_strategy.is_some()
                || self.layout_pages.is_some()
                || self.use_layout_for_markdown
                || has_formula_flag;
            if has_layout_flag {
                let mut layout = config.layout.clone().unwrap_or_default();
                if let Some(confidence) = self.layout_confidence {
                    layout.confidence_threshold = Some(confidence);
                }
                if let Some(ref table_model) = self.layout_table_model {
                    layout.table_model = table_model.parse().unwrap_or_default();
                }
                if let Some(ref strategy) = self.layout_strategy {
                    layout.strategy = strategy.parse().unwrap_or_default();
                }
                if let Some(ref pages) = self.layout_pages {
                    layout.pages = Some(pages.clone());
                }
                #[cfg(feature = "formula-recognition")]
                if let Some(ref formula_model) = self.layout_formula_model {
                    match formula_model.parse() {
                        Ok(model) => layout.formula_model = Some(model),
                        Err(error) => tracing::warn!("{error}; ignoring --layout-formula-model"),
                    }
                }
                config.layout = Some(layout);
            }
            if self.use_layout_for_markdown {
                config.use_layout_for_markdown = true;
            }
        }
    }

    /// Warn when the final resolved configuration enables layout detection while
    /// `output_format` stays `Plain` (contract point 4 of the OCR/layout structure
    /// contract — see `xberg::core::config_validation::layout_wastes_plain_output`).
    ///
    /// This is a warning, not a validation error and not a coercion: `Plain` remains the
    /// default output format and layout remains off by default. A caller who set both
    /// deliberately still gets exactly what they asked for; they are only told that the
    /// layout pass (20s-202s depending on backend, per the WP-E measurements) will run and
    /// its output will be discarded, since no renderer at `Plain` consumes structure.
    #[cfg(feature = "layout-detection")]
    pub(super) fn warn_layout_wastes_plain_output(&self, config: &ExtractionConfig) {
        if xberg::core::config_validation::layout_wastes_plain_output(config.layout.is_some(), &config.output_format) {
            tracing::warn!(
                "layout detection is enabled but the output format is 'plain'; the layout pass \
                 will run and the headings/lists/tables it detects will be discarded because \
                 plain output never renders structure. Pass --content-format markdown (or \
                 html/djot/json) to use the detected structure, or omit --layout to skip the \
                 extra work."
            );
        }
    }
}

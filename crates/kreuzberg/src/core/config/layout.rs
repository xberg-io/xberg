//! Layout detection configuration.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Which table structure recognition model to use.
///
/// Controls the model used for table cell detection within layout-detected
/// table regions. Wire format is snake_case in all serializers (JSON, TOML,
/// YAML).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableModel {
    /// TATR (Table Transformer) -- default, 30MB, DETR-based row/column detection.
    #[default]
    Tatr,
    /// SLANeXT wired variant -- 365MB, optimized for bordered tables.
    SlanetWired,
    /// SLANeXT wireless variant -- 365MB, optimized for borderless tables.
    SlanetWireless,
    /// SLANet-plus -- 7.78MB, lightweight general-purpose.
    SlanetPlus,
    /// Classifier-routed SLANeXT: auto-select wired/wireless per table.
    /// Uses PP-LCNet classifier (6.78MB) + both SLANeXT variants (730MB total).
    SlanetAuto,
    /// Disable table structure model inference entirely; use heuristic path only.
    Disabled,
}

impl std::str::FromStr for TableModel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tatr" => Ok(Self::Tatr),
            "slanet_wired" => Ok(Self::SlanetWired),
            "slanet_wireless" => Ok(Self::SlanetWireless),
            "slanet_plus" => Ok(Self::SlanetPlus),
            "slanet_auto" => Ok(Self::SlanetAuto),
            "disabled" => Ok(Self::Disabled),
            other => Err(format!(
                "unknown table model: '{other}'. Valid: tatr, slanet_wired, slanet_wireless, slanet_plus, slanet_auto, disabled"
            )),
        }
    }
}

impl fmt::Display for TableModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TableModel::Tatr => write!(f, "tatr"),
            TableModel::SlanetWired => write!(f, "slanet_wired"),
            TableModel::SlanetWireless => write!(f, "slanet_wireless"),
            TableModel::SlanetPlus => write!(f, "slanet_plus"),
            TableModel::SlanetAuto => write!(f, "slanet_auto"),
            TableModel::Disabled => write!(f, "disabled"),
        }
    }
}

/// Layout detection configuration.
///
/// Controls layout detection behavior in the extraction pipeline.
/// When set on [`ExtractionConfig`](super::ExtractionConfig), layout detection
/// is enabled for PDF extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutDetectionConfig {
    /// Confidence threshold override (None = use model default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_threshold: Option<f32>,

    /// Whether to apply postprocessing heuristics (default: true).
    #[serde(default = "default_true")]
    pub apply_heuristics: bool,

    /// Table structure recognition model.
    ///
    /// Controls which model is used for table cell detection within layout-detected
    /// table regions. Defaults to [`TableModel::Tatr`].
    #[serde(default)]
    pub table_model: TableModel,

    /// Hardware acceleration for ONNX models (layout detection + table structure).
    ///
    /// When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT)
    /// is used for inference. Defaults to `None` (auto-select per platform).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceleration: Option<super::acceleration::AccelerationConfig>,

    /// Maximum number of table cells allowed before a table is deemed too complex
    /// and rendered as plain text. Default is 5000.
    #[serde(default = "default_max_table_cells")]
    pub max_table_cells: Option<usize>,
}

fn default_max_table_cells() -> Option<usize> {
    Some(5000)
}

impl Default for LayoutDetectionConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: None,
            apply_heuristics: true,
            table_model: TableModel::default(),
            acceleration: None,
            max_table_cells: default_max_table_cells(),
        }
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LayoutDetectionConfig::default();
        assert_eq!(config.table_model, TableModel::Tatr);
        assert!(config.apply_heuristics);
        assert!(config.confidence_threshold.is_none());
        assert_eq!(config.max_table_cells, Some(5000));
    }

    #[test]
    fn test_table_model_deserialize() {
        let json = r#""tatr""#;
        let model: TableModel = serde_json::from_str(json).unwrap();
        assert_eq!(model, TableModel::Tatr);

        let json = r#""slanet_auto""#;
        let model: TableModel = serde_json::from_str(json).unwrap();
        assert_eq!(model, TableModel::SlanetAuto);

        let json = r#""disabled""#;
        let model: TableModel = serde_json::from_str(json).unwrap();
        assert_eq!(model, TableModel::Disabled);
    }

    #[test]
    fn test_table_model_serialize() {
        let json = serde_json::to_string(&TableModel::SlanetWired).unwrap();
        assert_eq!(json, r#""slanet_wired""#);
    }

    #[test]
    fn test_table_model_round_trip() {
        for model in [
            TableModel::Tatr,
            TableModel::SlanetWired,
            TableModel::SlanetWireless,
            TableModel::SlanetPlus,
            TableModel::SlanetAuto,
            TableModel::Disabled,
        ] {
            let serialized = serde_json::to_string(&model).unwrap();
            let parsed: TableModel = serde_json::from_str(&serialized).unwrap();
            assert_eq!(parsed, model, "round-trip failed for {model:?}");
        }
    }

    #[test]
    fn test_backward_compat_unknown_fields_ignored() {
        // Old configs with "preset" field should still deserialize because
        // serde ignores unknown fields by default.
        let json = r#"{"preset": "accurate", "apply_heuristics": true}"#;
        let config: LayoutDetectionConfig = serde_json::from_str(json).unwrap();
        assert!(config.apply_heuristics);
        assert_eq!(config.table_model, TableModel::Tatr);
    }

    #[test]
    fn test_backward_compat_old_table_model_field() {
        // Old configs with table_model as a string should still work
        let json = r#"{"table_model": "slanet_wired"}"#;
        let config: LayoutDetectionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.table_model, TableModel::SlanetWired);
    }

    #[test]
    fn test_table_model_display() {
        assert_eq!(TableModel::Tatr.to_string(), "tatr");
        assert_eq!(TableModel::SlanetWired.to_string(), "slanet_wired");
        assert_eq!(TableModel::Disabled.to_string(), "disabled");
    }
}

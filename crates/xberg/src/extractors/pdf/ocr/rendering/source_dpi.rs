use crate::core::config::OcrConfig;
use crate::core::config::ocr::SOURCE_DPI_BACKEND_OPTION;
use std::borrow::Cow;

/// `config` carrying `dpi` as its `source_dpi`, unless the caller already set one or no
/// resolution is proven, in which case `config` is returned as is.
pub(super) fn with_proven_source_dpi(config: &OcrConfig, dpi: Option<f64>) -> Cow<'_, OcrConfig> {
    let explicit = config
        .backend_options
        .as_ref()
        .and_then(|options| options.get(SOURCE_DPI_BACKEND_OPTION))
        .is_some();
    let Some(dpi) = dpi.filter(|_| !explicit) else {
        return Cow::Borrowed(config);
    };
    let mut config = config.clone();
    let mut options = match config.backend_options.take() {
        Some(serde_json::Value::Object(options)) => options,
        _ => serde_json::Map::new(),
    };
    options.insert(SOURCE_DPI_BACKEND_OPTION.to_owned(), serde_json::json!(dpi));
    config.backend_options = Some(serde_json::Value::Object(options));
    Cow::Owned(config)
}

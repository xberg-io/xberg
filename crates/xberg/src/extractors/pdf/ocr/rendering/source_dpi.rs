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
        None => serde_json::Map::new(),
        Some(serde_json::Value::Object(options)) => options,
        // Options in a shape this cannot extend are the caller's to keep.
        Some(other) => {
            config.backend_options = Some(other);
            return Cow::Owned(config);
        }
    };
    options.insert(SOURCE_DPI_BACKEND_OPTION.to_owned(), serde_json::json!(dpi));
    config.backend_options = Some(serde_json::Value::Object(options));
    Cow::Owned(config)
}

/// A fixed description of why the backend rejected an image. The raw error stays in the debug
/// log: backend errors can carry paths and library detail that a result must not expose.
pub(super) fn failure_cause(error: &crate::XbergError) -> &'static str {
    if error.to_string().contains("max_content_size") {
        "exceeds a configured size limit"
    } else {
        match error {
            crate::XbergError::Ocr { .. } => "the OCR backend failed",
            crate::XbergError::Parsing { .. } | crate::XbergError::ImageProcessing { .. } => {
                "the image could not be decoded"
            }
            _ => "the image was rejected",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::XbergError;

    fn config(options: Option<serde_json::Value>) -> OcrConfig {
        OcrConfig {
            backend_options: options,
            ..Default::default()
        }
    }

    #[test]
    fn a_proven_resolution_is_merged_into_existing_options() {
        let base = config(Some(serde_json::json!({ "use_cache": false })));
        let stamped = with_proven_source_dpi(&base, Some(150.0));
        assert_eq!(
            stamped.backend_options,
            Some(serde_json::json!({ "use_cache": false, "source_dpi": 150.0 }))
        );
        let bare = config(None);
        let from_none = with_proven_source_dpi(&bare, Some(150.0));
        assert_eq!(
            from_none.backend_options,
            Some(serde_json::json!({ "source_dpi": 150.0 }))
        );
    }

    #[test]
    fn a_caller_set_resolution_or_no_proof_leaves_the_config_alone() {
        let explicit = config(Some(serde_json::json!({ "source_dpi": 96.0 })));
        assert!(matches!(
            with_proven_source_dpi(&explicit, Some(150.0)),
            Cow::Borrowed(_)
        ));
        assert!(matches!(with_proven_source_dpi(&config(None), None), Cow::Borrowed(_)));
    }

    #[test]
    fn options_that_are_not_an_object_are_kept() {
        let base = config(Some(serde_json::json!("opaque")));
        let stamped = with_proven_source_dpi(&base, Some(150.0));
        assert_eq!(stamped.backend_options, Some(serde_json::json!("opaque")));
    }

    #[test]
    fn failure_causes_name_a_class_and_never_the_backend_text() {
        let limit = XbergError::Ocr {
            message: "Tesseract failed at /home/user/tessdata: exceeds security_limits.max_content_size".into(),
            source: None,
        };
        let engine = XbergError::Ocr {
            message: "cannot open /home/user/tessdata/eng.traineddata".into(),
            source: None,
        };
        assert_eq!(failure_cause(&limit), "exceeds a configured size limit");
        assert_eq!(failure_cause(&engine), "the OCR backend failed");
        for cause in [failure_cause(&limit), failure_cause(&engine)] {
            assert!(!cause.contains('/'));
        }
    }
}

use std::sync::Arc;

use crate::plugins::{InternalDocumentExtractor, Plugin, register_document_extractor, unregister_document_extractor};
use crate::types::{internal::InternalDocument, internal_builder::InternalDocumentBuilder};
use crate::{ExtractionConfig, Result};

pub(crate) struct BatchLayoutProbe(&'static str);

struct LayoutProbeExtractor(&'static [&'static str]);

impl BatchLayoutProbe {
    pub(crate) fn register(mime: &'static [&'static str]) -> Self {
        register_document_extractor(Arc::new(LayoutProbeExtractor(mime))).unwrap();
        Self(mime[0])
    }

    pub(crate) fn assert_output(output: &crate::ExtractionResult, base_layout: bool) {
        assert!(output.errors.is_empty(), "{:?}", output.errors);
        let actual: Vec<_> = output.results.iter().map(|result| result.content.as_str()).collect();
        let inherited = if base_layout {
            "layout enabled"
        } else {
            "layout disabled"
        };
        assert_eq!(actual, ["layout enabled", inherited, "layout disabled"]);
    }
}

impl Drop for BatchLayoutProbe {
    fn drop(&mut self) {
        unregister_document_extractor(self.0).unwrap();
    }
}

impl Plugin for LayoutProbeExtractor {
    fn name(&self) -> &str {
        self.0[0]
    }
}

#[async_trait::async_trait]
impl InternalDocumentExtractor for LayoutProbeExtractor {
    async fn extract_content(
        &self,
        _bytes: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let content = if config.use_layout_for_markdown {
            "layout enabled"
        } else {
            "layout disabled"
        };
        let mut builder = InternalDocumentBuilder::new("batch-layout-probe");
        builder.push_paragraph(content, vec![], None, None);
        let mut document = builder.build();
        document.mime_type = mime_type.to_string();
        Ok(document)
    }

    fn supported_mime_types(&self) -> &[&str] {
        self.0
    }

    fn priority(&self) -> i32 {
        100
    }
}

use super::*;
use crate::plugins::{Plugin, PostProcessor};
use async_trait::async_trait;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

struct Probe(Arc<AtomicUsize>);

impl Plugin for Probe {
    fn name(&self) -> &str {
        "no-runtime-probe"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PostProcessor for Probe {
    async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        self.0.fetch_add(1, Ordering::SeqCst);
        result.content.push_str(" processed");
        Ok(())
    }
    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Late
    }
}

fn finish_ready<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("the pure processor path must not depend on an async runtime"),
    }
}

#[test]
fn ordinary_async_processor_path_works_without_tokio() {
    assert!(!cfg!(feature = "tokio-runtime"));
    let calls = Arc::new(AtomicUsize::new(0));
    let processor: PostProcessorHandle = Arc::new(Probe(Arc::clone(&calls)));
    let stages = [(ProcessingStage::Late, Arc::new(vec![processor]))];
    let mut result = ExtractedDocument {
        content: "source".into(),
        ..Default::default()
    };
    finish_ready(execute_processor_stages(
        &mut result,
        &ExtractionConfig::default(),
        &None,
        &stages,
        None,
    ))
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(result.content, "source processed");
}

#[cfg(feature = "redaction")]
#[test]
fn source_bound_redaction_path_works_without_tokio() {
    use crate::core::config::OutputFormat;
    use crate::core::config::redaction::{RedactionConfig, RedactionTerm};
    use crate::types::builder::superscript;
    use crate::types::internal_builder::InternalDocumentBuilder;

    assert!(!cfg!(feature = "tokio-runtime"));
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("secret", vec![superscript(0, 6)], Some(1), None);
    let mut result =
        crate::extraction::derive::derive_with_surfaces(builder.build(), false, OutputFormat::Markdown, true);
    assert_eq!(result.formatted_content.as_deref(), Some("<sup>secret</sup>\n"));
    let mut registry = crate::plugins::registry::PostProcessorRegistry::new();
    assert!(registry.register_builtin_redaction().unwrap());
    let builtin = registry.builtin_redaction().unwrap();
    let stages = [(
        ProcessingStage::Late,
        Arc::new(registry.get_for_stage(ProcessingStage::Late)),
    )];
    assert_eq!(stages[0].1.len(), 1);
    assert!(Arc::ptr_eq(&stages[0].1[0], &builtin));
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        redaction: Some(RedactionConfig {
            custom_terms: vec![RedactionTerm::labeled("private", "secret")],
            ..Default::default()
        }),
        ..Default::default()
    };
    finish_ready(execute_processor_stages(
        &mut result,
        &config,
        &None,
        &stages,
        Some(&builtin),
    ))
    .unwrap();
    assert_eq!(result.content, "[REDACTED]");
    assert_eq!(result.formatted_content.as_deref(), Some("<sup>[REDACTED]</sup>\n"));
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 3);
}

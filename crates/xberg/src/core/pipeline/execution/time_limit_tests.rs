use super::*;
use crate::cancellation::CancellationToken;
use crate::plugins::{Plugin, PostProcessor};
use async_trait::async_trait;
use parking_lot::Mutex;

type Ran = Arc<Mutex<Vec<&'static str>>>;

/// A processor that records its run and, when `stops`, asks for the time-limit stop the way the
/// timeout wrapper does while a long processor is still running.
struct Recording {
    name: &'static str,
    stops: Option<CancellationToken>,
    ran: Ran,
}

impl Plugin for Recording {
    fn name(&self) -> &str {
        self.name
    }
}

#[async_trait]
impl PostProcessor for Recording {
    async fn process(&self, _result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        self.ran.lock().push(self.name);
        if let Some(token) = &self.stops {
            token.request_stop();
        }
        Ok(())
    }
    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }
}

fn stage(stops: Option<&CancellationToken>, ran: &Ran, names: &[&'static str]) -> Vec<ProcessorStage> {
    let processors: Vec<PostProcessorHandle> = names
        .iter()
        .enumerate()
        .map(|(index, &name)| {
            Arc::new(Recording {
                name,
                stops: stops.filter(|_| index == 0).cloned(),
                ran: Arc::clone(ran),
            }) as PostProcessorHandle
        })
        .collect();
    vec![(ProcessingStage::Middle, Arc::new(processors))]
}

fn config_with(token: &CancellationToken) -> ExtractionConfig {
    ExtractionConfig {
        cancel_token: Some(token.clone()),
        ..ExtractionConfig::default()
    }
}

#[tokio::test]
async fn processors_not_yet_started_when_the_limit_is_requested_are_skipped_and_named() {
    let token = CancellationToken::new();
    let ran: Ran = Arc::default();
    let stages = stage(Some(&token), &ran, &["first", "second", "third"]);
    let mut result = ExtractedDocument::default();

    execute_processor_stages(&mut result, &config_with(&token), &None, &stages, None)
        .await
        .expect("a skipped processor is not an error");

    assert_eq!(
        *ran.lock(),
        ["first"],
        "the processor in flight finishes, the rest do not start"
    );
    let warning = result
        .processing_warnings
        .iter()
        .find(|warning| warning.source == "time_limit")
        .expect("the skipped processors must be reported");
    assert!(
        warning.message.contains("post-processors second, third"),
        "the warning names the skipped stage: {}",
        warning.message
    );
    assert!(token.result_is_partial(), "the document must not be cached");
}

#[tokio::test]
async fn processors_all_run_when_the_limit_is_not_reached() {
    let token = CancellationToken::new();
    let ran: Ran = Arc::default();
    let stages = stage(None, &ran, &["first", "second"]);
    let mut result = ExtractedDocument::default();

    execute_processor_stages(&mut result, &config_with(&token), &None, &stages, None)
        .await
        .expect("processors run");

    assert_eq!(*ran.lock(), ["first", "second"]);
    assert!(result.processing_warnings.is_empty());
    assert!(!token.result_is_partial());
}

#[tokio::test]
async fn the_builtin_redaction_processor_still_runs_after_the_limit_is_requested() {
    let token = CancellationToken::new();
    let ran: Ran = Arc::default();
    let handle = |name: &'static str, stops: Option<&CancellationToken>| -> PostProcessorHandle {
        Arc::new(Recording {
            name,
            stops: stops.cloned(),
            ran: Arc::clone(&ran),
        })
    };
    let redaction = handle("redaction", None);
    let stages = vec![(
        ProcessingStage::Middle,
        Arc::new(vec![
            handle("stopper", Some(&token)),
            handle("skipped", None),
            Arc::clone(&redaction),
        ]),
    )];
    let mut result = ExtractedDocument::default();

    execute_processor_stages(&mut result, &config_with(&token), &None, &stages, Some(&redaction))
        .await
        .expect("processors run");

    assert_eq!(
        *ran.lock(),
        ["stopper", "redaction"],
        "dropping redaction would return the text it removes"
    );
}

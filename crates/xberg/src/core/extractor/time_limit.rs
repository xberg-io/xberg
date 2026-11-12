//! The `extraction_timeout_secs` handling shared by the file and bytes routes.
//!
//! These are plain functions, not one generic async wrapper: the callers' future types are
//! already deep enough that adding another async layer to them overflows rustc's auto-trait
//! recursion limit (E0275) when `extract` is type-erased to `dyn Future + Send`. ~keep

use crate::cancellation::CancellationToken;
use crate::types::ExtractedDocument;
use crate::{Result, XbergError};
use std::time::{Duration, Instant};

/// How long extraction may keep running after the limit, to finish its page in hand and return
/// the pages it holds. A stuck backend cannot hold the caller past it.
///
/// Ten seconds covers one OCR batch (a page costs 1 to 2 s, and a batch is at most the thread
/// budget wide) plus the assembly of the finished pages, while keeping the overrun of the
/// caller's limit short. An extraction that needs longer gets the `Timeout` error as before.
pub(super) const PARTIAL_RESULT_GRACE: Duration = Duration::from_secs(10);

const WARNING_SOURCE: &str = "timeout";

/// The limit passed. Asks an extraction whose page loop is running on `token` to stop at the
/// next page boundary and returns the token to await the extraction for [`PARTIAL_RESULT_GRACE`]
/// more. Any other extraction is cancelled and gets the timeout error at once.
pub(super) fn begin_grace(
    token: Option<&CancellationToken>,
    secs: u64,
    started: Instant,
) -> std::result::Result<&CancellationToken, XbergError> {
    match token.filter(|token| token.page_loop_running()) {
        Some(token) => {
            token.request_stop();
            Ok(token)
        }
        None => {
            if let Some(token) = token {
                token.cancel();
            }
            Err(timed_out(secs, started))
        }
    }
}

/// The outcome of awaiting an extraction through its grace period. A stopped extraction returns
/// the pages it finished with a warning naming the first page it did not read; one that did not
/// return in time is cancelled and gets the timeout error.
pub(super) fn finish_grace(
    outcome: std::result::Result<Result<ExtractedDocument>, tokio::time::error::Elapsed>,
    token: &CancellationToken,
    secs: u64,
    started: Instant,
) -> Result<ExtractedDocument> {
    match outcome {
        Ok(Ok(mut document)) => {
            if let Some(page) = token.first_unread_page() {
                // The operation-wide caches must not keep a partial document.
                super::cacheability::invalidate_current();
                crate::core::diagnostics::push_warning(
                    &mut document.processing_warnings,
                    WARNING_SOURCE,
                    format!(
                        "Extraction reached extraction_timeout_secs ({secs}s) and returned the pages it finished: \
                         page {page} is the first page that was not read."
                    ),
                );
            }
            Ok(document)
        }
        // A stopped run that read nothing reports the limit with the caller's numbers.
        Ok(Err(XbergError::Timeout { .. })) => Err(timed_out(secs, started)),
        Ok(Err(error)) => Err(error),
        Err(_elapsed) => {
            token.cancel();
            Err(timed_out(secs, started))
        }
    }
}

fn timed_out(secs: u64, started: Instant) -> XbergError {
    XbergError::Timeout {
        elapsed_ms: started.elapsed().as_millis() as u64,
        limit_ms: secs * 1000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn elapsed() -> tokio::time::error::Elapsed {
        tokio::time::timeout(Duration::ZERO, std::future::pending::<()>())
            .await
            .expect_err("a pending future must time out")
    }

    #[test]
    fn begin_grace_cancels_an_extraction_without_a_page_loop() {
        let token = CancellationToken::new();

        let outcome = begin_grace(Some(&token), 3, Instant::now());

        assert!(matches!(outcome, Err(XbergError::Timeout { limit_ms: 3000, .. })));
        assert!(token.is_cancelled());
        assert!(!token.stop_requested());
    }

    #[test]
    fn begin_grace_asks_a_page_loop_to_stop_without_cancelling() {
        let token = CancellationToken::new();
        let _guard = token.enter_page_loop();

        let outcome = begin_grace(Some(&token), 3, Instant::now());

        assert!(outcome.is_ok());
        assert!(token.stop_requested());
        assert!(!token.is_cancelled());
    }

    #[test]
    fn begin_grace_skips_the_grace_once_the_page_loop_is_over() {
        let token = CancellationToken::new();
        drop(token.enter_page_loop());

        let outcome = begin_grace(Some(&token), 3, Instant::now());

        assert!(matches!(outcome, Err(XbergError::Timeout { limit_ms: 3000, .. })));
        assert!(token.is_cancelled(), "a timeout after the loops is a hard timeout");
        assert!(!token.stop_requested());
    }

    #[test]
    fn finish_grace_names_the_first_unread_page_of_a_partial_result() {
        let token = CancellationToken::new();
        token.note_unread_page(7);
        token.note_unread_page(4);

        let document = finish_grace(Ok(Ok(ExtractedDocument::default())), &token, 3, Instant::now())
            .expect("a stopped page loop must return its pages");

        let warning = document
            .processing_warnings
            .iter()
            .find(|warning| warning.source == WARNING_SOURCE)
            .expect("a partial result must carry the timeout warning");
        assert!(warning.message.contains("page 4 is the first page that was not read"));
        assert!(!token.is_cancelled());
    }

    #[test]
    fn finish_grace_adds_no_warning_when_the_run_read_every_page() {
        let token = CancellationToken::new();

        let document = finish_grace(Ok(Ok(ExtractedDocument::default())), &token, 3, Instant::now())
            .expect("a finished run is returned as it is");

        assert!(document.processing_warnings.is_empty());
    }

    #[test]
    fn finish_grace_reports_the_callers_limit_when_nothing_was_read() {
        let token = CancellationToken::new();
        let nothing_read = Err(XbergError::Timeout {
            elapsed_ms: 0,
            limit_ms: 0,
        });

        let outcome = finish_grace(Ok(nothing_read), &token, 3, Instant::now());

        assert!(matches!(outcome, Err(XbergError::Timeout { limit_ms: 3000, .. })));
    }

    #[tokio::test]
    async fn finish_grace_cancels_an_extraction_that_outlives_the_grace() {
        let token = CancellationToken::new();
        let outcome = elapsed().await;

        let result = finish_grace(Err(outcome), &token, 3, Instant::now());

        assert!(matches!(result, Err(XbergError::Timeout { limit_ms: 3000, .. })));
        assert!(token.is_cancelled(), "the hard bound must cancel the token");
    }
}

//! Structured warning surface.
//!
//! `PdfDocument::structured_warnings()` returns the warnings raised since
//! the document was opened, as a list of structured `Warning` records.
//! Callers who want diagnostics as data (rather than stderr text from
//! `tracing::warn!`) opt in to this surface. The existing `tracing::warn!`
//! calls continue to fire so the `setup_logging(level="WARNING")`
//! shape keeps working.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// A single structured warning raised during PDF processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    /// The category — used by callers to filter.
    pub category: WarningCategory,
    /// The page index the warning was raised on, if any. `None` means
    /// the warning is document-scoped (xref recovery, trailer parse,
    /// etc.).
    pub page: Option<usize>,
    /// Free-form message. Matches the `tracing::warn!` strings to
    /// preserve grep-ability for users transitioning off the stderr
    /// noise.
    pub message: String,
    /// PDF spec section the warning references, when applicable.
    /// E.g. "7.3.8.1" for the stream-keyword newline violation.
    pub spec_section: Option<&'static str>,
}

/// Coarse-grained category for filtering. Each maps to a target in
/// `tracing::warn!` calls — `xberg_native_pdf::parser`, `xberg_native_pdf::fonts`,
/// `xberg_native_pdf::content`, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WarningCategory {
    /// PDF spec violations during xref / stream / content-stream parsing.
    /// E.g. "SPEC VIOLATION: No newline after stream keyword".
    SpecViolation,
    /// Font has no `ToUnicode` entry; falling back to AGL / CID-as-
    /// Unicode chain.
    ToUnicodeMissing,
    /// Xref table corrupt; reconstructing from `obj`/`endobj` scan.
    XrefRecovery,
    /// Content stream exceeded `MAX_OPERATORS` cap; truncating.
    OperatorCapExceeded,
    /// Type 3 font detected — may require special glyph name mapping.
    Type3Font,
    /// Unexpected EOF while reading an object header / body.
    EofPremature,
    /// Encryption / decryption related warning.
    Encryption,
    /// Other font warnings (DescendantFonts inline-dict fallback, etc.).
    Font,
    /// Layout / reading-order warnings.
    Layout,
    /// A glyph produced no rendered output while the cursor still advanced,
    /// so the page renders with an invisible gap.
    GlyphDropped,
    /// A `/JPXDecode` image dictionary's `/Width` or `/Height` disagreed with its codestream's
    /// `SIZ` marker segment; the codestream's size was used (GH#1900).
    JpxSizeMismatch,
}

impl WarningCategory {
    /// Stable kebab-case string for cross-binding consumption.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SpecViolation => "spec_violation",
            Self::ToUnicodeMissing => "to_unicode_missing",
            Self::XrefRecovery => "xref_recovery",
            Self::OperatorCapExceeded => "operator_cap_exceeded",
            Self::Type3Font => "type3_font",
            Self::EofPremature => "eof_premature",
            Self::Encryption => "encryption",
            Self::Font => "font",
            Self::Layout => "layout",
            Self::GlyphDropped => "glyph_dropped",
            Self::JpxSizeMismatch => "jpx_size_mismatch",
        }
    }
}

/// Thread-safe sink for warnings raised during a single PDF document's lifetime.
#[derive(Debug, Default, Clone)]
pub struct WarningSink {
    warnings: Arc<Mutex<Vec<Warning>>>,
    dropped: Arc<AtomicUsize>,
}

/// A sink that is never drained must not grow without bound: past this many
/// entries a single `diagnostics_truncated` sentinel counts what was dropped.
const MAX_SINK_ENTRIES: usize = 1000;
/// How far back a repeat of an earlier warning is collapsed into it. One
/// malformed font can warn once per glyph. ~keep
const REPEAT_WINDOW: usize = 16;

impl WarningSink {
    fn insert(&self, v: &mut Vec<Warning>, warning: Warning) {
        let recent = &v[..v.len().min(MAX_SINK_ENTRIES)];
        if recent
            .iter()
            .rev()
            .take(REPEAT_WINDOW)
            .any(|w| w.category == warning.category && w.page == warning.page && w.message == warning.message)
        {
            return;
        }
        if v.len() < MAX_SINK_ENTRIES {
            v.push(warning);
            return;
        }
        let dropped = self.dropped.fetch_add(1, Ordering::Relaxed) + 1;
        let sentinel = Warning {
            category: WarningCategory::SpecViolation,
            page: None,
            message: format!(
                "diagnostics_truncated: {dropped} further diagnostics were dropped after the {MAX_SINK_ENTRIES}-entry cap"
            ),
            spec_section: None,
        };
        if v.len() == MAX_SINK_ENTRIES {
            v.push(sentinel);
        } else {
            v[MAX_SINK_ENTRIES] = sentinel;
        }
    }
}

// The document owns the sink. Each synchronous parser or renderer entry point
// binds it on the thread performing the work. Outside a scope, the tracing
// event still fires and the warning remains unattributed. ~keep
thread_local! {
    static ACTIVE_SINK: RefCell<Option<WarningSink>> = const { RefCell::new(None) };
}

#[must_use = "the warning scope ends when the guard is dropped"]
pub(crate) struct WarningScope {
    previous: Option<WarningSink>,
    _thread_bound: PhantomData<Rc<()>>,
}

impl Drop for WarningScope {
    fn drop(&mut self) {
        ACTIVE_SINK.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}

pub(crate) fn record_warning(warning: Warning) {
    let sink = ACTIVE_SINK.with(|slot| slot.borrow().clone());
    if let Some(sink) = sink {
        sink.push(warning);
    }
}

const UNSCOPED_WARNING_LIMIT: usize = 256;
static UNSCOPED_WARNINGS: Mutex<VecDeque<Warning>> = Mutex::new(VecDeque::new());

/// Explicitly push a warning into the legacy process-wide sink.
/// It retains the 256 most recent entries, independently of document scopes.
pub fn push_global_warning(warning: Warning) {
    if let Ok(mut warnings) = UNSCOPED_WARNINGS.lock() {
        if warnings.len() == UNSCOPED_WARNING_LIMIT {
            warnings.pop_front();
        }
        warnings.push_back(warning);
    }
}

/// Drain the legacy process-wide sink used by standalone operations.
pub fn drain_global_warnings() -> Vec<Warning> {
    UNSCOPED_WARNINGS
        .lock()
        .map(|mut warnings| warnings.drain(..).collect())
        .unwrap_or_default()
}

/// Snapshot the legacy process-wide sink used by standalone operations.
pub fn snapshot_global_warnings() -> Vec<Warning> {
    UNSCOPED_WARNINGS
        .lock()
        .map(|warnings| warnings.iter().cloned().collect())
        .unwrap_or_default()
}

impl WarningSink {
    /// Create an empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn enter(&self) -> WarningScope {
        let previous = ACTIVE_SINK.with(|slot| slot.borrow_mut().replace(self.clone()));
        WarningScope {
            previous,
            _thread_bound: PhantomData,
        }
    }

    /// Record a warning without emitting a tracing event.
    pub fn push(&self, warning: Warning) {
        if let Ok(mut v) = self.warnings.lock() {
            self.insert(&mut v, warning);
        }
        // If the mutex was poisoned, silently drop — better than panic. ~keep
    }

    /// Snapshot of all warnings raised so far. Returns owned clones so
    /// the caller can keep them past the document's lifetime.
    pub fn snapshot(&self) -> Vec<Warning> {
        self.warnings.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Total warning count.
    pub fn len(&self) -> usize {
        self.warnings.lock().map(|v| v.len()).unwrap_or(0)
    }

    /// True if no warnings have been raised.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all warnings. Used by `PdfDocument::reset_warnings()` for
    /// callers who want to track per-operation warnings.
    pub fn clear(&self) {
        if let Ok(mut v) = self.warnings.lock() {
            v.clear();
            self.dropped.store(0, Ordering::Relaxed);
        }
    }

    /// Push multiple warnings under a single lock acquisition.
    pub fn extend(&self, warnings: impl IntoIterator<Item = Warning>) {
        if let Ok(mut v) = self.warnings.lock() {
            for warning in warnings {
                self.insert(&mut v, warning);
            }
        }
    }

    /// Drain and return all accumulated warnings.
    pub fn take(&self) -> Vec<Warning> {
        match self.warnings.lock() {
            Ok(mut v) => {
                self.dropped.store(0, Ordering::Relaxed);
                std::mem::take(&mut *v)
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sink_starts_empty() {
        let sink = WarningSink::new();
        assert!(sink.is_empty());
        assert_eq!(sink.len(), 0);
        assert_eq!(sink.snapshot().len(), 0);
    }

    #[test]
    fn push_and_snapshot() {
        let sink = WarningSink::new();
        sink.push(Warning {
            category: WarningCategory::ToUnicodeMissing,
            page: Some(0),
            message: "Type0 font 'X' has no ToUnicode entry!".into(),
            spec_section: Some("9.10.2"),
        });
        assert_eq!(sink.len(), 1);
        let snap = sink.snapshot();
        assert_eq!(snap[0].category, WarningCategory::ToUnicodeMissing);
        assert_eq!(snap[0].page, Some(0));
        assert!(snap[0].message.contains("ToUnicode"));
    }

    #[test]
    fn category_as_str_stable() {
        assert_eq!(WarningCategory::SpecViolation.as_str(), "spec_violation");
        assert_eq!(WarningCategory::ToUnicodeMissing.as_str(), "to_unicode_missing");
        assert_eq!(WarningCategory::OperatorCapExceeded.as_str(), "operator_cap_exceeded");
    }

    #[test]
    fn clear_resets() {
        let sink = WarningSink::new();
        sink.push(Warning {
            category: WarningCategory::SpecViolation,
            page: None,
            message: "x".into(),
            spec_section: None,
        });
        assert_eq!(sink.len(), 1);
        sink.clear();
        assert!(sink.is_empty());
    }

    fn scope_warning(message: &str) -> Warning {
        Warning {
            category: WarningCategory::SpecViolation,
            page: None,
            message: message.into(),
            spec_section: None,
        }
    }

    #[test]
    fn nested_warning_scope_restores_the_outer_sink() {
        let outer = WarningSink::new();
        let inner = WarningSink::new();
        let _outer = outer.enter();
        record_warning(scope_warning("before"));
        {
            let _inner = inner.enter();
            record_warning(scope_warning("inner"));
        }
        record_warning(scope_warning("after"));
        let messages: Vec<_> = outer.take().into_iter().map(|warning| warning.message).collect();
        assert_eq!(messages, ["before", "after"]);
        assert_eq!(inner.take()[0].message, "inner");
        assert!(inner.is_empty());
    }

    #[test]
    fn unwinding_restores_the_outer_warning_scope() {
        let outer = WarningSink::new();
        let inner = WarningSink::new();
        let _outer = outer.enter();
        let result = std::panic::catch_unwind(|| {
            let _inner = inner.enter();
            record_warning(scope_warning("during unwind"));
            panic!("exercise warning scope cleanup");
        });
        assert!(result.is_err());
        record_warning(scope_warning("after unwind"));
        assert_eq!(
            outer
                .snapshot()
                .iter()
                .map(|warning| warning.message.as_str())
                .collect::<Vec<_>>(),
            ["after unwind"]
        );
        assert_eq!(
            inner
                .snapshot()
                .iter()
                .map(|warning| warning.message.as_str())
                .collect::<Vec<_>>(),
            ["during unwind"]
        );
    }

    #[test]
    fn overlapping_warning_scopes_keep_each_threads_warnings() {
        let barrier = std::sync::Barrier::new(2);
        let workers = ["first document", "second document"];
        let sinks = std::thread::scope(|scope| {
            let threads = workers.map(|message| {
                let barrier = &barrier;
                scope.spawn(move || {
                    let sink = WarningSink::new();
                    let guard = sink.enter();
                    barrier.wait();
                    record_warning(scope_warning(message));
                    barrier.wait();
                    drop(guard);
                    sink
                })
            });
            threads.map(|worker| worker.join().expect("warning worker"))
        });

        for (sink, expected) in sinks.into_iter().zip(workers) {
            let messages: Vec<_> = sink.take().into_iter().map(|warning| warning.message).collect();
            assert_eq!(messages, [expected]);
        }
    }

    #[test]
    fn automatic_warning_outside_a_scope_is_not_retained() {
        let sink = WarningSink::new();
        drop(sink.enter());
        let message = "automatic warning without a document scope";
        record_warning(scope_warning(message));
        assert!(sink.is_empty());
        assert!(
            !snapshot_global_warnings()
                .iter()
                .any(|warning| warning.message == message)
        );
    }

    #[test]
    fn warning_serializes_to_json() {
        let w = Warning {
            category: WarningCategory::SpecViolation,
            page: Some(0),
            message: "No newline after stream keyword".into(),
            spec_section: Some("7.3.8.1"),
        };
        let json = serde_json::to_string(&w).unwrap();
        assert!(json.contains("\"category\":\"spec_violation\""));
        assert!(json.contains("\"page\":0"));
        assert!(json.contains("\"spec_section\":\"7.3.8.1\""));
    }

    #[test]
    fn sink_thread_safe() {
        use std::sync::Arc;
        use std::thread;

        let sink = Arc::new(WarningSink::new());
        let mut handles = Vec::new();
        for i in 0..10 {
            let s = sink.clone();
            handles.push(thread::spawn(move || {
                s.push(Warning {
                    category: WarningCategory::Font,
                    page: Some(i),
                    message: format!("font warning {}", i),
                    spec_section: None,
                });
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(sink.len(), 10);
    }
}

#[cfg(test)]
mod sink_bound_tests {
    use super::*;

    fn w(msg: &str) -> Warning {
        Warning {
            category: WarningCategory::SpecViolation,
            page: None,
            message: msg.to_string(),
            spec_section: None,
        }
    }

    #[test]
    fn test_identical_warning_is_not_recorded_repeatedly() {
        let sink = WarningSink::new();
        for _ in 0..50 {
            sink.push(w("one malformed font, warned per glyph"));
        }
        assert_eq!(sink.len(), 1);
    }

    #[test]
    fn test_sink_is_bounded_and_reports_what_it_dropped() {
        let sink = WarningSink::new();
        for i in 0..MAX_SINK_ENTRIES + 25 {
            sink.push(w(&format!("distinct {i}")));
        }
        let out = sink.take();
        assert_eq!(out.len(), MAX_SINK_ENTRIES + 1, "capped, plus one sentinel");
        let sentinel = out.last().expect("sentinel");
        assert!(
            sentinel.message.starts_with("diagnostics_truncated: 25 further"),
            "{sentinel:?}"
        );
    }

    #[test]
    fn test_repeat_outside_the_window_is_kept() {
        let sink = WarningSink::new();
        sink.push(w("first"));
        for i in 0..REPEAT_WINDOW {
            sink.push(w(&format!("other {i}")));
        }
        sink.push(w("first"));
        assert_eq!(sink.len(), REPEAT_WINDOW + 2);
    }

    #[test]
    fn test_take_resets_the_drop_count_under_the_vector_lock() {
        let sink = WarningSink::new();
        for i in 0..MAX_SINK_ENTRIES + 25 {
            sink.push(w(&format!("distinct {i}")));
        }
        let mut held = sink.warnings.lock().unwrap();
        let drainer = {
            let sink = sink.clone();
            std::thread::spawn(move || sink.take())
        };
        std::thread::sleep(std::time::Duration::from_millis(200));
        sink.insert(&mut held, w("overflow while draining"));
        drop(held);
        let out = drainer.join().unwrap();
        let sentinel = out.last().expect("sentinel");
        assert!(
            sentinel.message.starts_with("diagnostics_truncated: 26 further"),
            "{sentinel:?}"
        );
    }
}

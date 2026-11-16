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
use std::marker::PhantomData;
use std::rc::Rc;
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
}

// The document owns the sink. Each synchronous parser or renderer entry point
// binds it on the thread actually performing the work, including worker threads. ~keep
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
    } else {
        push_global_warning(warning);
    }
}

static UNSCOPED_WARNINGS: Mutex<Vec<Warning>> = Mutex::new(Vec::new());

/// Push a warning into the legacy process-wide sink for standalone operations.
/// Warnings in this sink are never attached to a document.
pub fn push_global_warning(warning: Warning) {
    if let Ok(mut warnings) = UNSCOPED_WARNINGS.lock() {
        warnings.push(warning);
    }
}

/// Drain the legacy process-wide sink used by standalone operations.
pub fn drain_global_warnings() -> Vec<Warning> {
    UNSCOPED_WARNINGS
        .lock()
        .map(|mut warnings| std::mem::take(&mut *warnings))
        .unwrap_or_default()
}

/// Snapshot the legacy process-wide sink used by standalone operations.
pub fn snapshot_global_warnings() -> Vec<Warning> {
    UNSCOPED_WARNINGS
        .lock()
        .map(|warnings| warnings.clone())
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
            v.push(warning);
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
        }
    }

    /// Push multiple warnings under a single lock acquisition.
    pub fn extend(&self, warnings: impl IntoIterator<Item = Warning>) {
        if let Ok(mut v) = self.warnings.lock() {
            v.extend(warnings);
        }
    }

    /// Drain and return all accumulated warnings.
    pub fn take(&self) -> Vec<Warning> {
        match self.warnings.lock() {
            Ok(mut v) => std::mem::take(&mut *v),
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

//! Per-document tally of recoverable input handling.
//!
//! Real PDFs routinely carry a missing embedded font, an object absent from the
//! xref table, or an unsupported CFF version. The parser handles all of these,
//! so none of them is a condition an operator can act on. Reporting each
//! occurrence at WARN made WARN unusable: measured over a 4,000-document corpus,
//! `xberg_native_pdf` emitted 4,012,488 WARN events against 44 real failures —
//! 99.96% of the run, roughly 863 events per document, one genuine failure per
//! 91,000 warnings (GH#1547).
//!
//! Two things are wrong there and both are fixed here. The *level* is wrong:
//! recoverable input handling is flow detail, so the per-occurrence events moved
//! to TRACE per this repo's level contract ("TRACE — per-item detail"). The
//! *cardinality* is wrong independently of level: one event per byte run or per
//! object is the wrong granularity for a corpus even at DEBUG. Each site now
//! increments a counter, and the document emits one DEBUG summary carrying the
//! totals when it is dropped. Someone debugging a single document still learns
//! what happened; someone running a corpus is not buried.
//!
//! ERROR behaviour is deliberately untouched — it already corresponded one to
//! one with documents that genuinely failed to parse.

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Counters for input the parser recovered from, accumulated over one document.
///
/// Held in an `Arc` on `PdfDocument` and shared with the thread-local pointer
/// below, so the free functions in `fonts::` can record without threading a
/// parameter through every decode signature. Atomics rather than a `Cell` because
/// a document's pages may be rendered from several threads.
#[derive(Debug, Default)]
pub struct RecoveryCounts {
    /// Text decoded with no font available, falling back to Latin-1.
    pub missing_font_decodes: AtomicU64,
    /// Bytes covered by those Latin-1 fallbacks.
    pub missing_font_bytes: AtomicU64,
    /// Objects absent from the xref table, triggering a file scan.
    pub xref_misses: AtomicU64,
    /// Objects the file scan then located successfully.
    pub file_scan_recoveries: AtomicU64,
    /// Objects neither the xref nor the file scan found, treated as Null
    /// per PDF spec 7.3.10.
    pub objects_treated_as_null: AtomicU64,
    /// Fonts whose CFF table declared a version the parser does not read.
    pub unsupported_cff_versions: AtomicU64,
}

/// A snapshot of [`RecoveryCounts`], taken once per document for reporting.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RecoverySnapshot {
    /// Text decoded with no font available, falling back to Latin-1.
    pub missing_font_decodes: u64,
    /// Bytes covered by those Latin-1 fallbacks.
    pub missing_font_bytes: u64,
    /// Objects absent from the xref table, triggering a file scan.
    pub xref_misses: u64,
    /// Objects the file scan then located successfully.
    pub file_scan_recoveries: u64,
    /// Objects neither the xref nor the file scan found, treated as Null.
    pub objects_treated_as_null: u64,
    /// Fonts whose CFF table declared a version the parser does not read.
    pub unsupported_cff_versions: u64,
}

impl RecoverySnapshot {
    /// True when nothing was recovered, so the document emits no summary at all.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Both tallies together.
    fn plus(&self, other: &Self) -> Self {
        Self {
            missing_font_decodes: self.missing_font_decodes.saturating_add(other.missing_font_decodes),
            missing_font_bytes: self.missing_font_bytes.saturating_add(other.missing_font_bytes),
            xref_misses: self.xref_misses.saturating_add(other.xref_misses),
            file_scan_recoveries: self.file_scan_recoveries.saturating_add(other.file_scan_recoveries),
            objects_treated_as_null: self
                .objects_treated_as_null
                .saturating_add(other.objects_treated_as_null),
            unsupported_cff_versions: self
                .unsupported_cff_versions
                .saturating_add(other.unsupported_cff_versions),
        }
    }

    /// What was recovered between `earlier` and this snapshot.
    pub(crate) fn since(&self, earlier: &Self) -> Self {
        Self {
            missing_font_decodes: self.missing_font_decodes.saturating_sub(earlier.missing_font_decodes),
            missing_font_bytes: self.missing_font_bytes.saturating_sub(earlier.missing_font_bytes),
            xref_misses: self.xref_misses.saturating_sub(earlier.xref_misses),
            file_scan_recoveries: self.file_scan_recoveries.saturating_sub(earlier.file_scan_recoveries),
            objects_treated_as_null: self
                .objects_treated_as_null
                .saturating_sub(earlier.objects_treated_as_null),
            unsupported_cff_versions: self
                .unsupported_cff_versions
                .saturating_sub(earlier.unsupported_cff_versions),
        }
    }
}

impl RecoveryCounts {
    /// Charge `delta` to the document, and to every [`capture_recoveries`] frame open on this
    /// thread for it.
    pub(crate) fn charge(&self, delta: &RecoverySnapshot) {
        self.add(delta);
        let identity = std::ptr::from_ref(self) as usize;
        CAPTURES.with(|captures| {
            for (watched, captured) in captures.borrow_mut().iter_mut() {
                if *watched == identity {
                    *captured = captured.plus(delta);
                }
            }
        });
    }

    fn add(&self, delta: &RecoverySnapshot) {
        self.missing_font_decodes
            .fetch_add(delta.missing_font_decodes, Ordering::Relaxed);
        self.missing_font_bytes
            .fetch_add(delta.missing_font_bytes, Ordering::Relaxed);
        self.xref_misses.fetch_add(delta.xref_misses, Ordering::Relaxed);
        self.file_scan_recoveries
            .fetch_add(delta.file_scan_recoveries, Ordering::Relaxed);
        self.objects_treated_as_null
            .fetch_add(delta.objects_treated_as_null, Ordering::Relaxed);
        self.unsupported_cff_versions
            .fetch_add(delta.unsupported_cff_versions, Ordering::Relaxed);
    }

    pub(crate) fn snapshot(&self) -> RecoverySnapshot {
        RecoverySnapshot {
            missing_font_decodes: self.missing_font_decodes.load(Ordering::Relaxed),
            missing_font_bytes: self.missing_font_bytes.load(Ordering::Relaxed),
            xref_misses: self.xref_misses.load(Ordering::Relaxed),
            file_scan_recoveries: self.file_scan_recoveries.load(Ordering::Relaxed),
            objects_treated_as_null: self.objects_treated_as_null.load(Ordering::Relaxed),
            unsupported_cff_versions: self.unsupported_cff_versions.load(Ordering::Relaxed),
        }
    }
}

thread_local! {
    /// The document currently being operated on by this thread, if any.
    ///
    /// A pointer rather than the counters themselves: the counters belong to the
    /// document so the totals stay correct across threads, and this only tells
    /// free functions which document to charge. `None` outside any scope, which
    /// is why every `record` below is a no-op rather than an error when unset —
    /// a decode helper called directly from a unit test must not panic.
    static ACTIVE: RefCell<Option<Arc<RecoveryCounts>>> = const { RefCell::new(None) };
    /// Open [`capture_recoveries`] frames, innermost last: the counters each watches and what
    /// this thread charged them meanwhile.
    static CAPTURES: RefCell<Vec<(usize, RecoverySnapshot)>> = const { RefCell::new(Vec::new()) };
}

struct CaptureFrame;

impl Drop for CaptureFrame {
    fn drop(&mut self) {
        CAPTURES.with(|captures| captures.borrow_mut().pop());
    }
}

/// Run `operation` and also return what it charged `counts` on this thread. A delta of the shared
/// counters would also hold what other threads recovered meanwhile, and a cached result would
/// charge those again on every replay.
pub(crate) fn capture_recoveries<T>(counts: &RecoveryCounts, operation: impl FnOnce() -> T) -> (T, RecoverySnapshot) {
    let identity = std::ptr::from_ref(counts) as usize;
    CAPTURES.with(|captures| captures.borrow_mut().push((identity, RecoverySnapshot::default())));
    let frame = CaptureFrame;
    let value = operation();
    let captured = CAPTURES.with(|captures| {
        captures
            .borrow()
            .last()
            .map(|(_, captured)| *captured)
            .unwrap_or_default()
    });
    drop(frame);
    (value, captured)
}

/// Installs `counts` as the active document for the current thread, restoring
/// whatever was active before when dropped.
///
/// Re-entrant by construction: it saves and restores rather than clearing, so a
/// public entry point that calls another one does not detach the inner scope.
#[must_use = "the scope is only active while the guard is alive"]
pub(crate) struct RecoveryScope {
    previous: Option<Arc<RecoveryCounts>>,
}

impl RecoveryScope {
    pub(crate) fn enter(counts: &Arc<RecoveryCounts>) -> Self {
        let previous = ACTIVE.with(|slot| slot.borrow_mut().replace(Arc::clone(counts)));
        Self { previous }
    }
}

impl Drop for RecoveryScope {
    fn drop(&mut self) {
        ACTIVE.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}

/// Charges one recovery against the active document, if there is one.
pub(crate) fn record(bump: impl FnOnce(&RecoveryCounts)) {
    ACTIVE.with(|slot| {
        if let Some(counts) = slot.borrow().as_ref() {
            bump(counts);
        }
    });
}

/// Emits the single per-document DEBUG summary described in GH#1547.
///
/// Silent when nothing was recovered, so a clean document adds no log line.
pub(crate) fn report(snapshot: &RecoverySnapshot) {
    if snapshot.is_empty() {
        return;
    }
    tracing::debug!(
        target: "xberg_native_pdf::recovery",
        missing_font_decodes = snapshot.missing_font_decodes,
        missing_font_bytes = snapshot.missing_font_bytes,
        xref_misses = snapshot.xref_misses,
        file_scan_recoveries = snapshot.file_scan_recoveries,
        objects_treated_as_null = snapshot.objects_treated_as_null,
        unsupported_cff_versions = snapshot.unsupported_cff_versions,
        "recovered from malformed PDF input"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts() -> Arc<RecoveryCounts> {
        Arc::new(RecoveryCounts::default())
    }

    #[test]
    fn record_outside_any_scope_is_a_noop_rather_than_a_panic() {
        // Decode helpers are called directly from unit tests with no document
        // in scope; that must stay silent rather than abort.
        record(|c| {
            c.xref_misses.fetch_add(1, Ordering::Relaxed);
        });
    }

    #[test]
    fn record_inside_a_scope_charges_that_documents_counters() {
        let document = counts();
        {
            let _scope = RecoveryScope::enter(&document);
            record(|c| {
                c.missing_font_decodes.fetch_add(1, Ordering::Relaxed);
            });
            record(|c| {
                c.missing_font_bytes.fetch_add(7, Ordering::Relaxed);
            });
        }
        let snapshot = document.snapshot();
        assert_eq!(snapshot.missing_font_decodes, 1);
        assert_eq!(snapshot.missing_font_bytes, 7);
    }

    #[test]
    fn record_after_the_scope_ends_charges_nothing() {
        let document = counts();
        drop(RecoveryScope::enter(&document));
        record(|c| {
            c.xref_misses.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(document.snapshot().xref_misses, 0);
    }

    #[test]
    fn a_nested_scope_restores_the_outer_document_rather_than_detaching_it() {
        let outer = counts();
        let inner = counts();
        let _outer_scope = RecoveryScope::enter(&outer);
        {
            let _inner_scope = RecoveryScope::enter(&inner);
            record(|c| {
                c.xref_misses.fetch_add(1, Ordering::Relaxed);
            });
        }
        record(|c| {
            c.xref_misses.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(inner.snapshot().xref_misses, 1, "inner scope took its own recovery");
        assert_eq!(
            outer.snapshot().xref_misses,
            1,
            "outer scope resumed after the inner one ended"
        );
    }

    /// `scan_detect` interprets pages on several threads at once. The shared counters' delta over
    /// one interpretation held the other threads' recoveries, which a cached page then charged
    /// again on every replay.
    #[test]
    fn a_capture_holds_only_what_its_own_thread_charged() {
        let document = counts();
        let xref_misses = |count| RecoverySnapshot {
            xref_misses: count,
            ..RecoverySnapshot::default()
        };

        let ((), captured) = capture_recoveries(&document, || {
            std::thread::scope(|scope| {
                scope.spawn(|| document.charge(&xref_misses(5)));
            });
            document.charge(&xref_misses(1));
        });

        assert_eq!(captured, xref_misses(1));
        assert_eq!(document.snapshot(), xref_misses(6));
    }

    #[test]
    fn a_nested_capture_charges_the_outer_one_too() {
        let document = counts();
        let other = counts();
        let one = RecoverySnapshot {
            missing_font_decodes: 1,
            ..RecoverySnapshot::default()
        };

        let (inner, outer) = capture_recoveries(&document, || {
            let ((), inner) = capture_recoveries(&document, || {
                document.charge(&one);
                other.charge(&one);
            });
            inner
        });

        assert_eq!(inner, one);
        assert_eq!(outer, one);
    }

    #[test]
    fn a_clean_document_snapshot_reports_nothing() {
        assert!(counts().snapshot().is_empty());
    }

    #[test]
    fn decoding_without_a_font_charges_the_document_instead_of_warning() {
        // The 3,701,388-event site from GH#1547: one WARN per byte run became one
        // counter bump, so this asserts the wiring reaches the real decode path.
        let document = counts();
        let bytes = b"hello";
        {
            let _scope = RecoveryScope::enter(&document);
            let decoded = crate::fonts::unicode_decode::decode_text_to_unicode(
                bytes,
                None,
                crate::fonts::unicode_decode::DecodePolicy::default(),
                None,
            );
            assert_eq!(decoded, "hello", "Latin-1 fallback still decodes the bytes");
        }
        let snapshot = document.snapshot();
        assert_eq!(snapshot.missing_font_decodes, 1);
        assert_eq!(snapshot.missing_font_bytes, bytes.len() as u64);
    }
}

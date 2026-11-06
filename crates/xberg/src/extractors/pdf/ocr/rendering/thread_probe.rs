/// #1690/#1747: `RENDER_CALL_THREAD_NAMES` below is test-only instrumentation (compiled
/// under `cfg(test)` plus the gates of its only users, so it never reaches a release build
/// and is never dead under a feature leg that lacks those users) that lets a test observe
/// which OS threads actually executed page renders -- the mechanism a regression here
/// breaks -- rather than inferring parallelism from wall-clock duration, which flakes under
/// shared-box load. Records thread NAMES rather than raw `ThreadId`s: the set is
/// process-global, so a guard must be able to tell its own pool's threads apart from any
/// other test's, e.g. via a caller-chosen name prefix -- otherwise a concurrently running
/// extraction can inflate the set and let a sequential regression here read as parallel
/// (the same defect class `pdf::native::images::PAGE_CALL_THREAD_NAMES` fixed against
/// #1732, and `core/config/concurrency.rs` against #215). See
/// `parallel_render_dispatches_across_more_than_one_thread` in `ocr/tests.rs`. ~keep
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(in super::super) static RENDER_CALL_THREAD_NAMES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
    std::sync::OnceLock::new();
#[cfg(all(test, feature = "ocr", feature = "pdf"))]
pub(in super::super) fn clear_render_call_thread_ids() {
    RENDER_CALL_THREAD_NAMES
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
        .lock()
        .unwrap()
        .clear();
}
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn record_render_thread() {
    let current = std::thread::current();
    let name = current.name().unwrap_or("<unnamed>").to_owned();
    RENDER_CALL_THREAD_NAMES
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
        .lock()
        .unwrap()
        .insert(name);
}

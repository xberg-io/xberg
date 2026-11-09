use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::{Layer, Registry};

#[derive(Default)]
struct LoadedObject {
    reference: Option<u64>,
    loading: bool,
}

impl Visit for LoadedObject {
    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "object_id" {
            self.reference = Some(value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.loading = format!("{value:?}") == "loading object";
        }
    }
}

struct ObjectLoads {
    reference: u64,
    calls: Arc<AtomicUsize>,
}

impl Layer<Registry> for ObjectLoads {
    fn on_event(&self, event: &tracing::Event<'_>, _context: Context<'_, Registry>) {
        let mut loaded = LoadedObject::default();
        event.record(&mut loaded);
        if loaded.loading && loaded.reference == Some(self.reference) {
            self.calls.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub(super) fn count<T>(reference: u64, action: impl FnOnce() -> T) -> (T, usize) {
    // A lone dispatcher lets another thread register a callsite against its empty subscriber.
    // Keep two live so tracing consults this counter when computing shared callsite interest. ~keep
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let calls = Arc::new(AtomicUsize::new(0));
    let subscriber = Registry::default().with(ObjectLoads {
        reference,
        calls: Arc::clone(&calls),
    });
    let output = tracing::subscriber::with_default(subscriber, action);
    (output, calls.load(Ordering::Relaxed))
}

#[test]
fn counts_loads_when_an_unsubscribed_thread_registers_the_callsite_first() {
    fn emit_load() {
        tracing::trace!(object_id = 2784_u64, "loading object");
    }

    let (_, calls) = count(2784, || {
        std::thread::spawn(emit_load)
            .join()
            .expect("register the load callsite without a subscriber");
        emit_load();
    });
    assert_eq!(calls, 1, "another thread must not disable the load counter");
}

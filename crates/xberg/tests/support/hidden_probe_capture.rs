use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use tracing::instrument::WithSubscriber as _;
use tracing_subscriber::layer::SubscriberExt as _;

pub(super) type Decision = (String, String, String);

#[derive(Default)]
struct Fields(BTreeMap<String, String>);

impl tracing::field::Visit for Fields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().to_owned(), value.to_owned());
    }
}

struct ProbeDecisions {
    field: &'static str,
    decisions: Arc<Mutex<Vec<Decision>>>,
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for ProbeDecisions {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        if fields.0.get("message").map(String::as_str) == Some("Hidden PDF text probe decision") {
            let decision = (
                fields.0["page"].clone(),
                fields.0[self.field].clone(),
                fields.0["probed"].clone(),
            );
            self.decisions.lock().expect("decision lock").push(decision);
        }
    }
}

pub(super) async fn capture<F: Future>(extraction: F, field: &'static str) -> (F::Output, Vec<Decision>) {
    let decisions = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(ProbeDecisions {
        field,
        decisions: Arc::clone(&decisions),
    });
    // ~keep Set the dispatcher for each poll so unrelated tasks on this thread do not inherit it.
    let result = Box::pin(extraction).with_subscriber(subscriber).await;
    let decisions = std::mem::take(&mut *decisions.lock().expect("decision lock"));
    (result, decisions)
}

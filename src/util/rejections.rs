//! Counts rejected records as they are logged, for a final summary.
//!
//! A log event is a rejection if it carries the field `rejected = true`. It
//! is counted under its `stage` field, as 1 or, for events that report
//! several records at once, as its `count` field.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

/// A `tracing` layer that counts rejection events per stage. Clones share
/// the same counts, so one clone can be installed and another queried.
#[derive(Clone, Default)]
pub(crate) struct RejectionCounter {
    counts: Arc<Mutex<BTreeMap<String, u64>>>,
}

impl RejectionCounter {
    pub(crate) fn total(&self) -> u64 {
        self.counts.lock().unwrap().values().sum()
    }

    /// Per-stage counts, e.g. `SAMPLE VARIANT PARSING 3, VARIANT RESOLUTION 2`.
    pub(crate) fn breakdown(&self) -> String {
        self.counts
            .lock()
            .unwrap()
            .iter()
            .map(|(stage, n)| format!("{stage} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl<S: Subscriber> Layer<S> for RejectionCounter {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut fields = RejectionFields::default();
        event.record(&mut fields);
        if fields.rejected {
            let stage = fields.stage.unwrap_or_else(|| "OTHER".to_string());
            *self.counts.lock().unwrap().entry(stage).or_default() += fields.count.unwrap_or(1);
        }
    }
}

/// The fields of an event that matter for counting.
#[derive(Default)]
struct RejectionFields {
    rejected: bool,
    stage: Option<String>,
    count: Option<u64>,
}

impl Visit for RejectionFields {
    fn record_bool(&mut self, field: &Field, value: bool) {
        if field.name() == "rejected" {
            self.rejected = value;
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "stage" {
            self.stage = Some(value.to_string());
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "count" {
            self.count = Some(value);
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if field.name() == "count" {
            self.count = u64::try_from(value).ok();
        }
    }

    fn record_debug(&mut self, _field: &Field, _value: &dyn fmt::Debug) {}
}

#[cfg(test)]
mod tests {
    use tracing::{info, warn};
    use tracing_subscriber::layer::SubscriberExt;

    use super::*;

    fn count(log: impl FnOnce()) -> RejectionCounter {
        let counter = RejectionCounter::default();
        let subscriber = tracing_subscriber::registry().with(counter.clone());
        tracing::subscriber::with_default(subscriber, log);
        counter
    }

    #[test]
    fn counts_only_events_marked_as_rejected_per_stage() {
        let counter = count(|| {
            info!(stage = "PARSING", rejected = true, "bad line");
            info!(stage = "PARSING", rejected = true, "bad line");
            warn!(stage = "LOOKUP", rejected = true, "missing");
            info!(stage = "PARSING", "merged, not a rejection");
            info!("unrelated");
        });
        assert_eq!(counter.total(), 3);
        assert_eq!(counter.breakdown(), "LOOKUP 1, PARSING 2");
    }

    #[test]
    fn count_field_reports_several_records_at_once() {
        let counter = count(|| {
            warn!(
                stage = "LOOKUP",
                rejected = true,
                count = 5usize,
                "5 missing"
            );
            info!(rejected = true, "no stage");
        });
        assert_eq!(counter.total(), 6);
        assert_eq!(counter.breakdown(), "LOOKUP 5, OTHER 1");
    }
}

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

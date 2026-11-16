use tracing::info;

use super::{
    Variant,
    log::{Stage, log_variant_event},
};

pub(crate) trait JoinVariants {
    fn join(&mut self, variants_b: Vec<Variant>);
}

impl JoinVariants for Vec<Variant> {
    fn join(&mut self, variants_b: Vec<Variant>) {
        let incoming = variants_b.len();
        let mut merged = 0usize;

        for variant_b in variants_b {
            if add_or_merge(self, variant_b) {
                merged += 1;
            }
        }

        // List-level totals have no single variant, so they stay a plain event.
        info!(
            incoming,
            merged,
            added = incoming - merged,
            total = self.len(),
            "LIST RECONCILIATION: joined variant lists"
        );
    }
}

/// Adds `var` to `collection`, merging it into an identical variant already
/// there. Returns whether it was merged.
pub(super) fn add_or_merge(collection: &mut Vec<Variant>, var: Variant) -> bool {
    match collection.iter_mut().find(|v| **v == var) {
        Some(existing) => {
            existing.merge(&var);
            true
        }
        None => {
            collection.push(var);
            false
        }
    }
}

impl Variant {
    /// Merge two identical variants by joining their IDs with `_`.
    pub(crate) fn merge(&mut self, other: &Variant) {
        debug_assert!(self == other, "merge called on non-identical variants");
        self.id = join_ids(std::mem::take(&mut self.id), &other.id);
        log_variant_event(
            Stage::DuplicateReconciliation,
            self,
            &format!("merged id from {}", other.id),
        );
    }
}

/// Appends `incoming` to `current` with `_`, reusing the existing buffer.
fn join_ids(mut current: String, incoming: &str) -> String {
    if !current.is_empty() {
        current.push('_');
    }
    current.push_str(incoming);
    current
}
#[cfg(test)]
#[path = "tests/test_join.rs"]
mod tests;

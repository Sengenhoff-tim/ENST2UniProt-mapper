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
            match self.iter().position(|v| *v == variant_b) {
                Some(idx) => {
                    self[idx].merge(&variant_b);
                    merged += 1;
                }
                None => self.push(variant_b),
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

impl Variant {
    /// Merge two identical variants by joining their IDs with `_`.
    pub(super) fn merge(&mut self, other: &Variant) {
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

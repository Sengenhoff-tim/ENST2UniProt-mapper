use tracing::info;

use super::{
    log::{log_variant_event, Stage},
    Variant,
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
mod tests {
    use super::*;
    use crate::test_support::variant;

    #[test]
    fn identical_variants_are_merged_with_joined_ids() {
        let mut list = vec![variant("a", 5, 5, Some("F"), Some("L"))];
        list.join(vec![variant("b", 5, 5, Some("F"), Some("L"))]);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "a_b");
    }

    #[test]
    fn different_variants_are_appended() {
        let mut list = vec![variant("a", 5, 5, Some("F"), Some("L"))];
        list.join(vec![variant("b", 6, 6, Some("G"), Some("A"))]);
        let ids: Vec<&str> = list.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
    }

    #[test]
    fn join_ids_skips_the_separator_for_an_empty_id() {
        assert_eq!(join_ids(String::new(), "b"), "b");
        assert_eq!(join_ids("a".to_string(), "b"), "a_b");
    }
}

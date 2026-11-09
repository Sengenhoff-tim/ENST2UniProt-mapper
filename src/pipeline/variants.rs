use crate::variant::Variant;

use super::log::{log_pipeline_rejection, Stage};

/// Attempts to resolve a premature-stop / out-of-bounds condition for `var`
/// against `seq`. On success, pushes `var` into `collection`; on failure,
/// logs the variant id and error and drops the variant.
pub(super) fn try_add_variant(collection: &mut Vec<Variant>, mut var: Variant, seq: &str) {
    match var.resolve_stop(seq) {
        Ok(_) => collection.push(var),
        Err(e) => log_pipeline_rejection(Stage::VariantResolution, &var.id, &e.to_string()),
    }
}

//TODO move upstream
/// Keeps only variants that pass `Variant::is_valid`, logging each dropped
/// one against `owner` (the accession or ENST the variants belong to).
pub(super) fn retain_valid(variants: Vec<Variant>, owner: &str) -> Vec<Variant> {
    variants
        .into_iter()
        .filter(|v| {
            let ok = v.is_valid();
            if !ok {
                log_pipeline_rejection(
                    Stage::VariantValidation,
                    owner,
                    &format!("dropped variant {} after failed validation", v),
                );
            }
            ok
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::variant;

    #[test]
    fn try_add_variant_keeps_resolvable_variants() {
        let mut list = Vec::new();
        try_add_variant(&mut list, variant("v", 2, 2, Some("C"), Some("*")), "ACDE");
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].begin, list[0].end), (2, 4));
    }

    #[test]
    fn try_add_variant_drops_out_of_range_variants() {
        let mut list = Vec::new();
        try_add_variant(&mut list, variant("v", 9, 9, Some("K"), Some("E")), "ACDE");
        assert!(list.is_empty());
    }

    #[test]
    fn retain_valid_drops_invalid_residues() {
        let kept = retain_valid(
            vec![variant("ok", 1, 1, Some("A"), Some("C")), variant("bad", 1, 1, Some("A"), Some("?"))],
            "P1",
        );
        let ids: Vec<&str> = kept.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, ["ok"]);
    }
}

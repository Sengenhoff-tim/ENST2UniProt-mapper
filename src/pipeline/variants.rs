use crate::variant::Variant;

use super::log::{Stage, log_pipeline_rejection};

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
#[path = "tests/test_variants.rs"]
mod tests;

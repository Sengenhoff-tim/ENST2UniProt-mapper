use crate::variant::Variant;

use super::log::{Stage, log_pipeline_rejection};

/// Resolves `var` against `seq` (see `Variant::resolve`). On success, adds it
/// to `collection`, merging it into an identical variant already there; on
/// failure, logs the variant id and error and drops the variant.
pub(super) fn try_add_variant(collection: &mut Vec<Variant>, mut var: Variant, seq: &str) {
    if let Err(e) = var.resolve(seq) {
        log_pipeline_rejection(Stage::VariantResolution, &var.id, &e.to_string());
        return;
    }
    match collection.iter_mut().find(|v| **v == var) {
        Some(existing) => existing.merge(&var),
        None => collection.push(var),
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
                    Stage::ValidationSafetyNet,
                    owner,
                    &format!(
                        "dropped variant {v} of {owner} with invalid residues (converters should not produce these)"
                    ),
                );
            }
            ok
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/test_variants.rs"]
mod tests;

use super::{
    Variant,
    join::add_or_merge,
    log::{Stage, log_variant_rejection},
};

/// Resolves `var` against `seq` (see `Variant::resolve`). On success, adds it
/// to `collection`, merging it into an identical variant already there; on
/// failure, logs the variant and error and drops the variant.
pub(crate) fn try_add_variant(collection: &mut Vec<Variant>, mut var: Variant, seq: &str) {
    if let Err(e) = var.resolve(seq) {
        log_variant_rejection(Stage::VariantResolution, &var, &e.to_string());
        return;
    }
    add_or_merge(collection, var);
}

//TODO move upstream
/// Keeps only variants that pass `Variant::is_valid`, logging each dropped
/// one against `owner` (the accession or ENST the variants belong to).
pub(crate) fn retain_valid(variants: Vec<Variant>, owner: &str) -> Vec<Variant> {
    variants
        .into_iter()
        .filter(|v| {
            let ok = v.is_valid();
            if !ok {
                log_variant_rejection(
                    Stage::ValidationSafetyNet,
                    v,
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
#[path = "tests/test_collection.rs"]
mod tests;

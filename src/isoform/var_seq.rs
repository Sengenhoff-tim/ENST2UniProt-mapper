use crate::uniprot::Feature;
use anyhow::{Result, anyhow};

/// Apply a set of VAR_SEQ features to the canonical sequence to produce an
/// isoform sequence. Features must be applied in descending position order
/// to avoid index drift from earlier substitutions/deletions.
pub(super) fn apply_var_seq_features(canonical: &str, features: &[&Feature]) -> Result<String> {
    let mut sorted: Vec<&Feature> = features.to_vec();
    sorted.sort_by_key(|f| std::cmp::Reverse(f.begin));

    let mut seq = canonical.to_string();

    for feature in sorted {
        let start = feature
            .begin
            .ok_or_else(|| anyhow!("VAR_SEQ begin position is unknown"))?;
        let end = feature
            .end
            .ok_or_else(|| anyhow!("VAR_SEQ end position is unknown"))?;

        if start == 0 || start > end || end > seq.len() {
            return Err(anyhow!(
                "VAR_SEQ position {}..{} out of range for sequence length {}",
                start,
                end,
                seq.len()
            ));
        }

        let range_start = start - 1;
        let range_end = end;

        if let Some(original) = feature.original.as_deref() {
            let actual = &seq[range_start..range_end];
            if actual != original {
                return Err(anyhow!(
                    "VAR_SEQ original sequence mismatch at {}..{}: expected '{}', found '{}'",
                    start,
                    end,
                    original,
                    actual
                ));
            }
        }

        // no <variation> = "Missing" → deletion
        let replacement = feature.variations.first().map(|s| s.as_str()).unwrap_or("");

        seq.replace_range(range_start..range_end, replacement);
    }

    Ok(seq)
}

#[cfg(test)]
#[path = "tests/test_var_seq.rs"]
mod tests;

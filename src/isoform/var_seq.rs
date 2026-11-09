use anyhow::{anyhow, Result};
use crate::uniprot::Feature;

/// Apply a set of VAR_SEQ features to the canonical sequence to produce an
/// isoform sequence. Features must be applied in descending position order
/// to avoid index drift from earlier substitutions/deletions.
pub(super) fn apply_var_seq_features(canonical: &str, features: &[&Feature]) -> Result<String> {
    let mut sorted: Vec<&Feature> = features.to_vec();
    sorted.sort_by_key(|f| std::cmp::Reverse(f.begin));

    let mut seq = canonical.to_string();

    for feature in sorted {
        let start = feature.begin.ok_or_else(|| anyhow!("VAR_SEQ begin position is unknown"))?;
        let end = feature.end.ok_or_else(|| anyhow!("VAR_SEQ end position is unknown"))?;

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
mod tests {
    use super::*;
    use crate::test_support::{feature, SEQ};
    use crate::uniprot::FeatureKind::SpliceVariant;

    fn apply(features: &[Feature]) -> Result<String> {
        apply_var_seq_features(SEQ, &features.iter().collect::<Vec<_>>())
    }

    #[test]
    fn substitutes_a_range() {
        let seq = apply(&[feature(SpliceVariant, "VSP_1", 2, 3, Some("CD"), &["W"])]).unwrap();
        assert_eq!(seq, "AWEFGHIKLMNPQRSTVWY");
    }

    #[test]
    fn deletes_a_range_without_variation() {
        let seq = apply(&[feature(SpliceVariant, "VSP_1", 18, 20, None, &[])]).unwrap();
        assert_eq!(seq, "ACDEFGHIKLMNPQRST");
    }

    #[test]
    fn applies_several_features_regardless_of_input_order() {
        // An earlier feature changing the length must not shift a later one.
        let seq = apply(&[
            feature(SpliceVariant, "VSP_1", 2, 3, Some("CD"), &["WWWW"]),
            feature(SpliceVariant, "VSP_2", 18, 20, None, &[]),
        ])
        .unwrap();
        assert_eq!(seq, "AWWWWEFGHIKLMNPQRST");
    }

    #[test]
    fn rejects_an_original_that_does_not_match() {
        let err = apply(&[feature(SpliceVariant, "VSP_1", 2, 3, Some("XX"), &["W"])]).unwrap_err();
        assert!(err.to_string().contains("mismatch"), "{err}");
    }

    #[test]
    fn rejects_out_of_range_and_unknown_positions() {
        assert!(apply(&[feature(SpliceVariant, "VSP_1", 0, 1, None, &[])]).is_err());
        assert!(apply(&[feature(SpliceVariant, "VSP_1", 19, 21, None, &[])]).is_err());
        assert!(apply(&[feature(SpliceVariant, "VSP_1", 5, 4, None, &[])]).is_err());

        let mut unknown = feature(SpliceVariant, "VSP_1", 1, 1, None, &[]);
        unknown.begin = None;
        assert!(apply(&[unknown]).is_err());
    }
}

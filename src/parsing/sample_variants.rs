use std::collections::{BTreeSet, HashMap};
use std::fs;

use anyhow::{Context, Result};

use crate::types::EnsemblId;
use crate::variant::Variant;

use super::{
    log::{Stage, log_line_event, log_line_rejection},
    notation::{positions_match, to_variant},
};

/// parse a ensembl transcript mapped variant input file, returning a map of Ensembl-ID → list of variants.
pub(crate) fn parse_sample_variants(path: &str) -> Result<HashMap<EnsemblId, Vec<Variant>>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read variants file '{}'", path))?;
    let mut variants: HashMap<EnsemblId, Vec<Variant>> = HashMap::new();
    // Transcripts with at least one rejected line (sorted for stable logs).
    let mut rejected: BTreeSet<EnsemblId> = BTreeSet::new();
    for (line_no, line) in content.lines().enumerate() {
        // e.g. "  ENST00000367770.5    726G>726S  " -> "ENST00000367770.5    726G>726S"
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let location = format!("{}:{}", path, line_no + 1);
        let skip =
            |outcome: String| log_line_rejection(Stage::SampleVariantParsing, &location, &outcome);

        let mut parts = line.split_whitespace();

        // e.g. "ENST00000367770.5    726G>726S" -> "ENST00000367770.5"
        let Some(enst) = parts.next() else {
            continue;
        };
        // e.g. "ENST00000367770.5" starts with "ENST" -> ok; "NM_001256789.1" -> not ok, logged and skipped
        if !enst.starts_with("ENST") {
            skip(format!("skipped, '{}' does not look like an ENST id", enst));
            continue;
        }
        let enst_id = EnsemblId::from_versioned(enst);

        // e.g. "ENST00000367770.5    726G>726S" -> "726G>726S"
        let Some(variant_string) = parts.next() else {
            // e.g. "ENST00000367770.5" alone on the line, no second field
            skip(format!("skipped, missing variant field for {}", enst));
            rejected.insert(enst_id);
            continue;
        };

        // e.g. "726G>726S" -> Some(true); "726G>731S" -> Some(false), logged and skipped
        if positions_match(variant_string) == Some(false) {
            skip(format!(
                "skipped, position mismatch in variant '{}' for {}",
                variant_string, enst
            ));
            rejected.insert(enst_id);
            continue;
        }

        // variant id is "{enst}_{idx}", idx counting this transcript's parsed variants
        let idx = variants.get(&enst_id).map_or(0, Vec::len);

        // e.g. "726G>726S" -> Some(Variant { .. }); "garbage!!" -> None, logged and skipped
        let Some(variant) = to_variant(&format!("{}_{}", enst, idx), variant_string) else {
            skip(format!(
                "skipped, could not parse variant '{}' for {}",
                variant_string, enst
            ));
            rejected.insert(enst_id);
            continue;
        };

        // Only now create the transcript's list: transcripts without a single
        // parsable variant must not appear at all.
        variants.entry(enst_id).or_default().push(variant);
    }

    for enst_id in rejected.iter().filter(|id| !variants.contains_key(*id)) {
        log_line_event(
            Stage::SampleVariantParsing,
            path,
            &format!(
                "left out {}: none of its variant lines could be parsed",
                enst_id
            ),
        );
    }
    Ok(variants)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_file;

    fn parse(content: &str) -> HashMap<EnsemblId, Vec<Variant>> {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(&dir, "variants.txt", content);
        parse_sample_variants(path.to_str().unwrap()).unwrap()
    }

    #[test]
    fn groups_variants_by_unversioned_transcript_with_indexed_ids() {
        let variants = parse("ENST1.5\t726G>726S\n  ENST1.5    10A>10C  \nENST2.1\t1M>1V\n");
        let ids: Vec<&str> = variants["ENST1"].iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, ["ENST1.5_0", "ENST1.5_1"]);
        assert_eq!(variants["ENST1"][0].begin, 726);
        assert_eq!(variants["ENST2"].len(), 1);
    }

    #[test]
    fn transcripts_without_parsable_variants_are_left_out() {
        let variants = parse("ENST1\tgarbage\nENST1\t726G>731S\nENST2\t1M>1V\n");
        assert!(!variants.contains_key("ENST1"));
        assert_eq!(variants.len(), 1);
    }

    #[test]
    fn ids_count_only_parsed_variants() {
        let variants = parse("ENST1\tgarbage\nENST1\t1M>1V\nENST1\t2A>2C\n");
        let ids: Vec<&str> = variants["ENST1"].iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, ["ENST1_0", "ENST1_1"]);
    }

    #[test]
    fn skips_invalid_lines() {
        let variants = parse(
            "NM_0001\t1A>1C\n\
             ENST1\n\
             ENST1\t726G>731S\n\
             ENST1\tgarbage\n\
             \n\
             ENST1\t5K>5E\n",
        );
        assert_eq!(variants.len(), 1);
        assert_eq!(variants["ENST1"].len(), 1);
        assert_eq!(variants["ENST1"][0].begin, 5);
    }
}

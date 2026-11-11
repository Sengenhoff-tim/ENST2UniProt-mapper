use super::*;
use crate::test_support::{iso_variant, rich_entry, variant};
use crate::uniprot::Evidence;

#[test]
fn var_seq_substitution_with_evidence() {
    let entry = rich_entry();
    let lines = var_seq_ft_lines(&entry.features[0], &entry).unwrap();
    assert_eq!(
        lines,
        [
            "FT   VAR_SEQ         2..3",
            "FT                   /note=\"CD -> W (in isoform 2)\"",
            "FT                   /evidence=\"ECO:0000303|PubMed:12345\"",
            "FT                   /id=\"VSP_000001\"",
        ]
    );
}

#[test]
fn var_seq_deletion_is_missing() {
    let entry = rich_entry();
    let lines = var_seq_ft_lines(&entry.features[1], &entry).unwrap();
    assert_eq!(
        lines,
        [
            "FT   VAR_SEQ         18..20",
            "FT                   /note=\"Missing (in isoform 2)\"",
            "FT                   /id=\"VSP_000002\"",
        ]
    );
}

#[test]
fn var_seq_without_id_falls_back_to_evidence() {
    let mut entry = rich_entry();
    entry.features[0].id = None;
    let feature = entry.features[0].clone();
    let lines = var_seq_ft_lines(&feature, &entry).unwrap();
    assert_eq!(
        lines.last().unwrap(),
        "FT                   /id=\"ECO:0000303|PubMed:12345\""
    );
}

#[test]
fn var_seq_without_position_is_skipped() {
    let mut entry = rich_entry();
    entry.features[0].begin = None;
    let feature = entry.features[0].clone();
    assert!(var_seq_ft_lines(&feature, &entry).is_none());
}

#[test]
fn evidence_string_resolves_each_source_kind() {
    let mut entry = rich_entry();
    entry.evidences.insert(
        "2".into(),
        Evidence {
            code: "ECO:0000305".into(),
            source: Some(EvidenceSource::Ref("3".into())),
        },
    );
    entry.evidences.insert(
        "3".into(),
        Evidence {
            code: "ECO:0000250".into(),
            source: None,
        },
    );
    let mut feature = entry.features[0].clone();
    feature.evidence_keys = vec!["1".into(), "2".into(), "3".into(), "unknown".into()];
    assert_eq!(
        format_evidence_string(&feature, &entry),
        "ECO:0000303|PubMed:12345, ECO:0000305|Ref.3, ECO:0000250"
    );
}

#[test]
fn variant_lines_for_substitution_and_missing() {
    assert_eq!(
        variant("VAR_1", 9, 9, Some("K"), Some("E")).ft_lines(),
        [
            "FT   VARIANT         9",
            "FT                   /note=\"K -> E\"",
            "FT                   /id=\"VAR_1\"",
        ]
    );
    let missing = variant("VAR_2", 3, 5, None, None).ft_lines();
    assert_eq!(missing[0], "FT   VARIANT         3..5");
    assert_eq!(missing[1], "FT                   /note=\"Missing\"");
}

#[test]
fn variant_on_isoform_prefixes_the_position() {
    let lines = iso_variant("P1-2", "VAR_1", 5, 6, "GH", "A").ft_lines();
    assert_eq!(lines[0], "FT   VARIANT         P1-2:5..6");
}

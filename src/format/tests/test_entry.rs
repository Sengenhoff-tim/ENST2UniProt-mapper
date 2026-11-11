use super::*;
use crate::test_support::{SEQ, rich_entry, variant};

#[test]
fn uniprot_backed_entry_has_names_isoforms_and_features() {
    let entry = rich_entry();
    let variants = [variant("VAR_000001", 9, 9, Some("K"), Some("E"))];
    let out = format_entry("P99999", SEQ, &variants, Some(&entry));

    for expected in [
        "ID   P99999                  Reviewed;         20 AA.\nAC   P99999;",
        "DE   RecName: Full=Test protein;",
        "GN   Name=TST; Synonyms=TST; ORFNames=TST;",
        "CC   -!- ALTERNATIVE PRODUCTS:",
        "DR   UTA; P99999; Universal Transcript Archive",
        "FT   VAR_SEQ         2..3",
        "FT   VAR_SEQ         18..20",
        "FT   VARIANT         9",
        "SQ   SEQUENCE   20 AA;",
        "     ACDEFGHIKL MNPQRSTVWY\n//",
    ] {
        assert!(out.contains(expected), "missing {expected:?} in:\n{out}");
    }
}

#[test]
fn var_seq_lines_precede_variant_lines() {
    let entry = rich_entry();
    let out = format_entry(
        "P99999",
        SEQ,
        &[variant("V", 1, 1, Some("A"), Some("C"))],
        Some(&entry),
    );
    assert!(out.find("FT   VAR_SEQ").unwrap() < out.find("FT   VARIANT").unwrap());
}

#[test]
fn synthetic_entry_is_named_after_its_accession() {
    let out = format_entry("ENST00000000009", "MACDE", &[], None);
    assert!(out.starts_with("ID   ENST00000000009         Reviewed;         5 AA."));
    assert!(out.contains("DE   RecName: Full=ENST00000000009;"));
    assert!(!out.contains("ALTERNATIVE PRODUCTS"));
    assert!(!out.contains("\nFT   "));
}

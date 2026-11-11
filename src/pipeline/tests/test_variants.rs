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
        vec![
            variant("ok", 1, 1, Some("A"), Some("C")),
            variant("bad", 1, 1, Some("A"), Some("?")),
        ],
        "P1",
    );
    let ids: Vec<&str> = kept.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(ids, ["ok"]);
}

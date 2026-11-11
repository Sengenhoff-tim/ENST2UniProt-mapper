use crate::test_support::variant;

#[test]
fn accepts_amino_acid_codes_and_missing_fields() {
    assert!(variant("v", 1, 1, Some("ACDEFGHIKLMNPQRSTVWY"), Some("UOXBZJ")).is_valid());
    assert!(variant("v", 1, 1, Some("A"), None).is_valid());
    assert!(variant("v", 1, 1, None, None).is_valid());
}

#[test]
fn rejects_non_amino_acid_characters() {
    assert!(!variant("v", 1, 1, Some("A"), Some("*")).is_valid());
    assert!(!variant("v", 1, 1, Some("a"), Some("C")).is_valid());
    assert!(!variant("v", 1, 1, Some("A"), Some("del")).is_valid());
}

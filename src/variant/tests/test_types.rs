use crate::test_support::{iso_variant, variant};

#[test]
fn equality_ignores_the_id() {
    let a = variant("a", 5, 5, Some("F"), Some("L"));
    let b = variant("b", 5, 5, Some("F"), Some("L"));
    assert_eq!(a, b);
    assert_ne!(a, variant("a", 5, 5, Some("F"), Some("W")));
    assert_ne!(a, iso_variant("P1-2", "a", 5, 5, "F", "L"));
}

#[test]
fn display_is_compact() {
    assert_eq!(
        variant("v1", 3, 4, Some("DE"), None).to_string(),
        "v1 (canonical:3-4 DE>-)"
    );
    assert_eq!(
        iso_variant("P1-2", "v2", 5, 5, "F", "L").to_string(),
        "v2 (P1-2:5-5 F>L)"
    );
}

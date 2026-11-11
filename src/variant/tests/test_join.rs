use super::*;
use crate::test_support::variant;

#[test]
fn identical_variants_are_merged_with_joined_ids() {
    let mut list = vec![variant("a", 5, 5, Some("F"), Some("L"))];
    list.join(vec![variant("b", 5, 5, Some("F"), Some("L"))]);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "a_b");
}

#[test]
fn different_variants_are_appended() {
    let mut list = vec![variant("a", 5, 5, Some("F"), Some("L"))];
    list.join(vec![variant("b", 6, 6, Some("G"), Some("A"))]);
    let ids: Vec<&str> = list.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);
}

#[test]
fn join_ids_skips_the_separator_for_an_empty_id() {
    assert_eq!(join_ids(String::new(), "b"), "b");
    assert_eq!(join_ids("a".to_string(), "b"), "a_b");
}

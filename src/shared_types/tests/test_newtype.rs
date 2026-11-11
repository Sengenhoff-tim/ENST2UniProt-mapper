use std::collections::HashSet;

use crate::shared_types::{EnsemblId, UniProtCanonId};

#[test]
fn ids_can_be_looked_up_by_str() {
    let set: HashSet<UniProtCanonId> = [UniProtCanonId::new("P04637")].into();
    assert!(set.contains("P04637"));
    assert!(!set.contains("P38398"));
}

#[test]
fn ids_display_and_sort_like_their_string() {
    assert_eq!(EnsemblId::new("ENST1").to_string(), "ENST1");
    let mut ids = [
        EnsemblId::new("ENST2"),
        EnsemblId::new("ENST10"),
        EnsemblId::new("ENST1"),
    ];
    ids.sort();
    let sorted: Vec<&str> = ids.iter().map(EnsemblId::as_str).collect();
    assert_eq!(sorted, ["ENST1", "ENST10", "ENST2"]);
}

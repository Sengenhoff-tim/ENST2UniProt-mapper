use super::*;

#[test]
fn splits_position_from_residues() {
    assert_eq!(split_position_and_residues("726G"), Some((726, "G")));
    assert_eq!(split_position_and_residues("12AB*"), Some((12, "AB*")));
    assert_eq!(split_position_and_residues("G726"), None);
}

#[test]
fn positions_match_compares_both_sides() {
    assert_eq!(positions_match("726G>726S"), Some(true));
    assert_eq!(positions_match("726G>731S"), Some(false));
    assert_eq!(positions_match("garbage!!"), None);
}

#[test]
fn single_residue_substitution() {
    let v = to_variant("ENST1_0", "726G>726S").unwrap();
    assert_eq!((v.id.as_str(), v.begin, v.end), ("ENST1_0", 726, 726));
    assert_eq!(
        (v.aa_ref.as_deref(), v.aa_new.as_deref()),
        (Some("G"), Some("S"))
    );
    assert_eq!(v.isoform, None);
}

#[test]
fn multi_residue_reference_spans_its_length() {
    let v = to_variant("x", "2AA>2B").unwrap();
    assert_eq!((v.begin, v.end), (2, 3));
    assert_eq!(
        (v.aa_ref.as_deref(), v.aa_new.as_deref()),
        (Some("AA"), Some("B"))
    );
}

#[test]
fn stop_markers_are_kept_verbatim() {
    let v = to_variant("x", "3C>3*").unwrap();
    assert_eq!(
        (v.aa_ref.as_deref(), v.aa_new.as_deref()),
        (Some("C"), Some("*"))
    );
}

#[test]
fn rejects_malformed_notation() {
    assert!(to_variant("x", "726G").is_none(), "no '>'");
    assert!(to_variant("x", "G>S").is_none(), "no position");
    assert!(to_variant("x", "726>726S").is_none(), "empty reference");
}

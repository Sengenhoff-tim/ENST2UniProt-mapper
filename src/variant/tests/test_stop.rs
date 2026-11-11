use crate::test_support::variant;
use crate::variant::Variant;

/// `ABCDE`, as in the cases below.
const SEQ: &str = "ABCDE";

fn resolved(mut v: Variant) -> Variant {
    v.resolve_stop(SEQ).expect("resolves");
    v
}

fn fields(v: &Variant) -> (usize, usize, Option<&str>, Option<&str>) {
    (v.begin, v.end, v.aa_ref.as_deref(), v.aa_new.as_deref())
}

#[test]
fn plain_substitution_is_unchanged() {
    let v = resolved(variant("v", 2, 2, Some("B"), Some("X")));
    assert_eq!(fields(&v), (2, 2, Some("B"), Some("X")));
}

#[test]
fn missing_sequence_is_unchanged() {
    let v = resolved(variant("v", 2, 3, None, None));
    assert_eq!(fields(&v), (2, 3, None, None));
}

#[test]
fn ebi_del_becomes_missing_sequence() {
    let v = resolved(variant("v", 2, 2, Some("B"), Some("del")));
    assert_eq!(fields(&v), (2, 2, Some("B"), None));
}

#[test]
fn gained_stop_truncates_to_the_sequence_end() {
    // "3C>3*": everything from position 3 on is lost
    let v = resolved(variant("v", 3, 3, Some("C"), Some("*")));
    assert_eq!(fields(&v), (3, 5, Some("CDE"), None));
}

#[test]
fn gained_stop_keeps_residues_before_the_marker() {
    // "1A>1BB*": BB replaces the rest of the sequence
    let v = resolved(variant("v", 1, 1, Some("A"), Some("BB*")));
    assert_eq!(fields(&v), (1, 5, Some("ABCDE"), Some("BB")));
}

#[test]
fn lost_stop_extends_from_the_last_residue() {
    // "1*>1BB*": C-terminal extension anchored on the last residue E
    let v = resolved(variant("v", 1, 1, Some("*"), Some("BB*")));
    assert_eq!(fields(&v), (5, 5, Some("E"), Some("EBB")));
}

#[test]
fn stop_inside_the_reference_extends_the_span_to_the_end() {
    // "2B*>2X": the reference ends at the stop, the span runs to the end
    let v = resolved(variant("v", 2, 2, Some("B*"), Some("X")));
    assert_eq!(fields(&v), (2, 5, Some("BCDE"), Some("X")));
}

#[test]
fn resolved_reference_always_spans_begin_to_end() {
    for (aa_ref, aa_new) in [
        ("A", "*"),
        ("A", "BB*"),
        ("*", "BB*"),
        ("B*", "X"),
        ("BC", "X*Y"),
    ] {
        let v = resolved(variant("v", 2, 2, Some(aa_ref), Some(aa_new)));
        let span = v.end - v.begin + 1;
        assert_eq!(
            v.aa_ref.as_deref().map(str::len),
            Some(span),
            "{aa_ref}>{aa_new}: {v}"
        );
    }
}

#[test]
fn rejects_new_residues_without_reference() {
    assert!(
        variant("v", 2, 2, None, Some("X"))
            .resolve_stop(SEQ)
            .is_err()
    );
}

#[test]
fn rejects_positions_outside_the_sequence() {
    assert!(
        variant("v", 0, 0, Some("A"), Some("X"))
            .resolve_stop(SEQ)
            .is_err()
    );
    assert!(
        variant("v", 6, 6, Some("A"), Some("X"))
            .resolve_stop(SEQ)
            .is_err()
    );
    assert!(
        variant("v", 1, 1, Some("A"), Some("X"))
            .resolve_stop("")
            .is_err()
    );
}

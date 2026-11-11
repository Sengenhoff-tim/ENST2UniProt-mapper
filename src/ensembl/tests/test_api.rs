use super::*;

#[test]
fn strip_stop_codon_truncates_at_the_first_marker() {
    assert_eq!(strip_stop_codon("ENST1", "MAC*DE*".to_string()), "MAC");
    assert_eq!(strip_stop_codon("ENST1", "MACDE".to_string()), "MACDE");
}

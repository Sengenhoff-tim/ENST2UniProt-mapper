use super::*;

#[test]
fn groups_residues_by_ten_and_lines_by_sixty() {
    let seq = "A".repeat(65);
    let body = format_seq_body(&seq);
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], format!("     {}", ["AAAAAAAAAA"; 6].join(" ")));
    assert_eq!(lines[1], "     AAAAA");
}

#[test]
fn short_sequence_is_one_line() {
    assert_eq!(format_seq_body("ACDEFGHIKLMN"), "     ACDEFGHIKL MN");
}

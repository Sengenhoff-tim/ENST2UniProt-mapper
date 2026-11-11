use super::*;
use crate::test_support::write_file;

#[test]
fn reads_one_accession_per_line_trimmed_skipping_blanks() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(&dir, "acc.txt", "P04637\n  P38398 \n\nQ9Y6K9\n");
    let ids = parse_accession_list(path.to_str().unwrap()).unwrap();
    let ids: Vec<&str> = ids.iter().map(UniProtCanonId::as_str).collect();
    assert_eq!(ids, ["P04637", "P38398", "Q9Y6K9"]);
}

#[test]
fn missing_file_is_an_error() {
    assert!(parse_accession_list("/nonexistent/acc.txt").is_err());
}

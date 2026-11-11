use super::*;
use crate::test_support::write_file;

fn parse(content: &str) -> HashMap<EnsemblId, Vec<UniProtCanonId>> {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(&dir, "idmapping.tsv", content);
    parse_idmapping(&path).unwrap()
}

fn accessions_of<'a>(
    mapping: &'a HashMap<EnsemblId, Vec<UniProtCanonId>>,
    enst: &str,
) -> Vec<&'a str> {
    mapping[enst].iter().map(UniProtCanonId::as_str).collect()
}

#[test]
fn maps_unversioned_transcripts_to_canonical_accessions() {
    let mapping = parse("P62258-2\tENST00000571732.5\n");
    assert_eq!(accessions_of(&mapping, "ENST00000571732"), ["P62258"]);
}

#[test]
fn collects_several_accessions_per_transcript_without_duplicates() {
    let mapping = parse("P1\tENST1.1\nP2\tENST1.1\nP1-2\tENST1.2\n");
    assert_eq!(accessions_of(&mapping, "ENST1"), ["P1", "P2"]);
}

#[test]
fn skips_non_transcript_and_incomplete_lines() {
    let mapping = parse("P1\tENSP00000000001\nP2\n\nP3\tENST3\n");
    assert_eq!(mapping.len(), 1);
    assert_eq!(accessions_of(&mapping, "ENST3"), ["P3"]);
}

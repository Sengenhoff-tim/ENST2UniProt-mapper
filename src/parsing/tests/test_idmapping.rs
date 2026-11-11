use std::io::Write;

use flate2::{Compression, write::GzEncoder};

use super::*;
use crate::test_support::write_file;

fn parse(content: &str) -> HashMap<EnsemblId, Vec<UniProtCanonId>> {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(&dir, "idmapping.dat", content);
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
    let mapping = parse("P62258-2\tEnsembl_TRS\tENST00000571732.5\n");
    assert_eq!(accessions_of(&mapping, "ENST00000571732"), ["P62258"]);
}

#[test]
fn collects_several_accessions_per_transcript_without_duplicates() {
    let mapping = parse(
        "P1\tEnsembl_TRS\tENST1.1\n\
         P2\tEnsembl_TRS\tENST1.1\n\
         P1-2\tEnsembl_TRS\tENST1.2\n",
    );
    assert_eq!(accessions_of(&mapping, "ENST1"), ["P1", "P2"]);
}

#[test]
fn uses_only_ensembl_transcript_lines() {
    // As in HUMAN_9606_idmapping.dat: gene, protein and other ids are ignored.
    let mapping = parse(
        "P62258\tUniProtKB-ID\t1433E_HUMAN\n\
         P62258\tEnsembl\tENSG00000108953.18\n\
         P62258-1\tEnsembl_PRO\tENSP00000264335.8\n\
         P62258-1\tEnsembl_TRS\tENST00000264335.13\n\
         P62258\tGI\t5803225\n",
    );
    assert_eq!(mapping.len(), 1);
    assert_eq!(accessions_of(&mapping, "ENST00000264335"), ["P62258"]);
}

#[test]
fn skips_non_transcript_and_incomplete_lines() {
    let mapping = parse(
        "P1\tEnsembl_TRS\tENSP00000000001\n\
         P2\tENST2\n\
         P2\n\
         \n\
         P3\tEnsembl_TRS\tENST3\n",
    );
    assert_eq!(mapping.len(), 1);
    assert_eq!(accessions_of(&mapping, "ENST3"), ["P3"]);
}

#[test]
fn reads_gzipped_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("idmapping.dat.gz");
    let mut gz = GzEncoder::new(
        std::fs::File::create(&path).unwrap(),
        Compression::default(),
    );
    gz.write_all(b"P62258-2\tEnsembl_TRS\tENST00000571732.5\n")
        .unwrap();
    gz.finish().unwrap();

    let mapping = parse_idmapping(&path).unwrap();
    assert_eq!(accessions_of(&mapping, "ENST00000571732"), ["P62258"]);
}

#[test]
fn missing_file_is_an_error() {
    assert!(parse_idmapping(Path::new("/nonexistent/idmapping.dat")).is_err());
}

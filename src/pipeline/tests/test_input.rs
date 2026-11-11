use super::*;
use crate::test_support::{strs, write_file};

fn enst(id: &str) -> EnsemblId {
    EnsemblId::new(id)
}

fn acc(id: &str) -> UniProtCanonId {
    UniProtCanonId::new(id)
}

fn mapping(pairs: &[(&str, &[&str])]) -> HashMap<EnsemblId, Vec<UniProtCanonId>> {
    pairs
        .iter()
        .map(|(e, accs)| (enst(e), accs.iter().map(|a| acc(a)).collect()))
        .collect()
}

#[test]
fn extends_accessions_in_transcript_order_without_duplicates() {
    let id_mapping = mapping(&[
        ("ENST2", &["P2", "P1"]),
        ("ENST1", &["P3"]),
        ("ENST9", &["P9"]),
    ]);
    let mut accessions = vec![acc("P1")];
    let ensts = [enst("ENST2"), enst("ENST1"), enst("ENST404")];

    extend_with_mapped_accessions(&mut accessions, ensts.iter(), &id_mapping);

    // ENST1 is visited before ENST2; P1 was already present; ENST9 isn't a sample transcript.
    assert_eq!(strs(&accessions), ["P1", "P3", "P2"]);
}

#[test]
fn omit_list_is_empty_without_a_file() {
    assert!(read_omit_list(None).unwrap().is_empty());
}

#[test]
fn omit_list_reads_trimmed_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(&dir, "omit.txt", "P1\n  P2 \n\n");
    let omit = read_omit_list(Some(&path)).unwrap();
    assert_eq!(omit, [acc("P1"), acc("P2")].into());
}

#[test]
fn fallback_candidates_are_unconsumed_and_not_omitted() {
    let id_mapping = mapping(&[
        ("ENST1", &["P1"]),
        ("ENST2", &["P2"]),
        ("ENST3", &["P3", "P4"]),
    ]);
    let variant_ensts = [enst("ENST4"), enst("ENST3"), enst("ENST2"), enst("ENST1")];
    let consumed: HashSet<EnsemblId> = [enst("ENST1")].into();
    let omit: HashSet<UniProtCanonId> = [acc("P4")].into();

    let candidates =
        ensembl_fallback_candidates(variant_ensts.iter(), &id_mapping, &consumed, &omit);

    // ENST1 consumed, ENST3 maps to omitted P4, ENST4 has no mapping and is kept.
    assert_eq!(strs(&candidates), ["ENST2", "ENST4"]);
}

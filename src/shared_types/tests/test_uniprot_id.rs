use super::*;

#[test]
fn isoform_id_knows_its_canonical_accession() {
    assert_eq!(
        UniProtIsoId::new("P04637-2").canonical(),
        UniProtCanonId::new("P04637")
    );
}

#[test]
fn uniprot_id_classifies_accessions() {
    assert_eq!(
        UniProtId::from_accession("P04637"),
        UniProtId::Canonical(UniProtCanonId::new("P04637"))
    );
    assert_eq!(
        UniProtId::from_accession("P04637-2"),
        UniProtId::Isoform(UniProtIsoId::new("P04637-2"))
    );
}

#[test]
fn uniprot_id_canonical_for_both_kinds() {
    let expected = UniProtCanonId::new("P04637");
    assert_eq!(UniProtId::from_accession("P04637").canonical(), expected);
    assert_eq!(UniProtId::from_accession("P04637-2").canonical(), expected);
}

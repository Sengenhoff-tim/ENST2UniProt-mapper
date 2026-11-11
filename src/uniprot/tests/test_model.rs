use super::*;
use crate::shared_types::UniProtIsoId;
use crate::test_support::{minimal_entry_xml, parse_entry, rich_entry};

#[test]
fn names_default_to_unknown() {
    let entry = parse_entry(&minimal_entry_xml("P1", ""));
    assert_eq!(
        (entry.protein_name(), entry.gene_name()),
        ("Unknown", "Unknown")
    );
    assert_eq!(entry.primary_accession(), "P1");
}

#[test]
fn feature_filters_split_by_kind() {
    let entry = rich_entry();
    let ids = |it: Vec<&Feature>| it.iter().filter_map(|f| f.id.clone()).collect::<Vec<_>>();
    assert_eq!(
        ids(entry.var_seq_features().collect()),
        ["VSP_000001", "VSP_000002"]
    );
    assert_eq!(
        ids(entry.natural_variant_features().collect()),
        ["VAR_000001", "VAR_000002"]
    );
}

#[test]
fn ensembl_mapping_points_to_isoforms_or_canonical() {
    let expected = [
        (
            UniProtId::Isoform(UniProtIsoId::new("P99999-1")),
            EnsemblId::new("ENST00000000001"),
        ),
        (
            UniProtId::Isoform(UniProtIsoId::new("P99999-2")),
            EnsemblId::new("ENST00000000002"),
        ),
        (
            UniProtId::Canonical(UniProtCanonId::new("P99999")),
            EnsemblId::new("ENST00000000003"),
        ),
    ];
    assert_eq!(rich_entry().ensembl_mapping(), expected);
}

#[test]
fn isoform_sequence_type_parses_xml_values() {
    assert_eq!(
        IsoformSequenceType::parse("displayed"),
        IsoformSequenceType::Displayed
    );
    assert_eq!(
        IsoformSequenceType::parse("described"),
        IsoformSequenceType::Described
    );
    assert_eq!(
        IsoformSequenceType::parse("not described"),
        IsoformSequenceType::NotDescribed
    );
    assert_eq!(
        IsoformSequenceType::parse("external"),
        IsoformSequenceType::External
    );
    assert_eq!(
        IsoformSequenceType::parse("new"),
        IsoformSequenceType::Other("new".into())
    );
}

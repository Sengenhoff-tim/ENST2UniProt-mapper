use super::*;
use crate::test_support::{
    SEQ, minimal_entry_xml, parse_entries, parse_entry, rich_entry, rich_entry_xml, uniprot_xml,
};
use crate::uniprot::FeatureKind;

fn parse_with_omit(entries: &[String], omit: &[&str]) -> Vec<String> {
    let omit: HashSet<UniProtCanonId> = omit.iter().map(|a| UniProtCanonId::new(*a)).collect();
    parse_uniprot_xml(uniprot_xml(entries).as_bytes(), &omit, "test")
        .unwrap()
        .into_vec()
        .into_iter()
        .map(|(id, _)| id.to_string())
        .collect()
}

#[test]
fn reads_accessions_names_and_sequence() {
    let entry = rich_entry();
    assert_eq!(entry.accessions, ["P99999", "Q00000"]);
    assert_eq!(entry.protein_name.as_deref(), Some("Test protein"));
    assert_eq!(entry.gene_name.as_deref(), Some("TST"));
    assert_eq!(entry.sequence, SEQ);
}

#[test]
fn keeps_only_splice_and_sequence_variant_features() {
    let entry = rich_entry();
    let ids: Vec<&str> = entry
        .features
        .iter()
        .filter_map(|f| f.id.as_deref())
        .collect();
    assert_eq!(
        ids,
        ["VSP_000001", "VSP_000002", "VAR_000001", "VAR_000002"]
    );
}

#[test]
fn reads_feature_details() {
    let entry = rich_entry();
    let vsp1 = &entry.features[0];
    assert_eq!(vsp1.kind, FeatureKind::SpliceVariant);
    assert_eq!((vsp1.begin, vsp1.end), (Some(2), Some(3)));
    assert_eq!(vsp1.original.as_deref(), Some("CD"));
    assert_eq!(vsp1.variations, ["W"]);
    assert_eq!(vsp1.description.as_deref(), Some("in isoform 2"));
    assert_eq!(vsp1.evidence_keys, ["1"]);

    let vsp2 = &entry.features[1];
    assert_eq!((vsp2.original.as_ref(), vsp2.variations.len()), (None, 0));

    let var2 = &entry.features[3];
    assert_eq!(var2.kind, FeatureKind::SequenceVariant);
    assert_eq!((var2.begin, var2.end), (Some(5), Some(5)));
    assert_eq!(var2.location_sequence.as_deref(), Some("P99999-2"));
}

#[test]
fn unknown_positions_are_none() {
    let entry = parse_entry(&minimal_entry_xml(
        "P1",
        r#"<feature type="sequence variant" id="VAR_1"><location><begin status="unknown"/><end position="4"/></location></feature>"#,
    ));
    assert_eq!(
        (entry.features[0].begin, entry.features[0].end),
        (None, Some(4))
    );
}

#[test]
fn reads_the_alternative_products_comment_only() {
    let ap = rich_entry().alternative_products.unwrap();
    assert_eq!(ap.events, ["alternative splicing"]);
    assert_eq!(ap.notes, ["Additional isoforms seem to exist."]);
    assert_eq!(ap.isoforms.len(), 3);

    let iso2 = &ap.isoforms[1];
    assert_eq!(iso2.ids, ["P99999-2"]);
    assert_eq!(iso2.names, ["2", "Beta"]);
    assert_eq!(iso2.sequence_type, IsoformSequenceType::Described);
    assert_eq!(iso2.vsp_ids, ["VSP_000001", "VSP_000002"]);
    assert_eq!(
        ap.isoforms[2].sequence_type,
        IsoformSequenceType::NotDescribed
    );
}

#[test]
fn reads_ensembl_cross_references_with_molecules() {
    let refs = rich_entry().ensembl_refs;
    let refs: Vec<(&str, Option<&str>)> = refs
        .iter()
        .map(|r| (r.transcript_id.as_str(), r.molecule.as_deref()))
        .collect();
    assert_eq!(
        refs,
        [
            ("ENST00000000001.3", Some("P99999-1")),
            ("ENST00000000002.1", Some("P99999-2")),
            ("ENST00000000003.1", None),
        ]
    );
}

#[test]
fn reads_evidence_with_database_and_reference_sources() {
    let entry = parse_entry(&minimal_entry_xml(
        "P1",
        r#"<evidence type="ECO:0000303" key="1"><source><dbReference type="PubMed" id="123"/></source></evidence>
           <evidence type="ECO:0000305" key="2"><source ref="3"/></evidence>
           <evidence type="ECO:0000250" key="3"/>"#,
    ));
    let source = |key: &str| entry.evidences[key].source.clone();
    assert!(
        matches!(source("1"), Some(EvidenceSource::DbRef { db, id }) if db == "PubMed" && id == "123")
    );
    assert!(matches!(source("2"), Some(EvidenceSource::Ref(r)) if r == "3"));
    assert!(source("3").is_none());
    assert_eq!(entry.evidences["3"].code, "ECO:0000250");
}

#[test]
fn unescapes_text_and_attributes() {
    let entry = parse_entry(&minimal_entry_xml(
        "P1",
        r#"<protein><recommendedName><fullName>A &amp; B</fullName></recommendedName></protein>
           <feature type="splice variant" id="VSP_1" description="in &quot;x&quot;"><location><position position="1"/></location></feature>"#,
    ));
    assert_eq!(entry.protein_name.as_deref(), Some("A & B"));
    assert_eq!(entry.features[0].description.as_deref(), Some("in \"x\""));
}

#[test]
fn keeps_document_order_and_skips_omitted_entries() {
    let entries = ["P3", "P1", "P2"].map(|a| minimal_entry_xml(a, ""));
    assert_eq!(parse_with_omit(&entries, &["P1"]), ["P3", "P2"]);
}

#[test]
fn omit_matches_primary_accessions_only() {
    let entries = [rich_entry_xml("P1")];
    assert_eq!(parse_with_omit(&entries, &["Q00000"]), ["P1"]);
}

#[test]
fn keeps_the_first_of_duplicate_entries() {
    let entries = [
        minimal_entry_xml(
            "P1",
            "<protein><recommendedName><fullName>first</fullName></recommendedName></protein>",
        ),
        minimal_entry_xml(
            "P1",
            "<protein><recommendedName><fullName>second</fullName></recommendedName></protein>",
        ),
    ];
    let parsed = parse_entries(&entries);
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].1.protein_name.as_deref(), Some("first"));
}

#[test]
fn skips_entries_without_sequence() {
    let entries = [
        r#"<entry><accession>P1</accession></entry>"#.to_string(),
        minimal_entry_xml("P2", ""),
    ];
    assert_eq!(parse_with_omit(&entries, &[]), ["P2"]);
}

#[test]
fn ignores_unknown_elements_and_values() {
    let entry = parse_entry(&minimal_entry_xml(
        "P1",
        r#"<brandNewElement a="b"><x>y</x></brandNewElement>
           <feature type="brand new type"><location><position position="1"/></location></feature>"#,
    ));
    assert!(entry.features.is_empty());
}

#[test]
fn malformed_xml_is_an_error() {
    let xml = "<uniprot><entry><accession>P1</accession><sequence>AC</entry></uniprot>";
    assert!(parse_uniprot_xml(xml.as_bytes(), &HashSet::new(), "test").is_err());
}

#[test]
fn document_cut_between_entries_is_an_error() {
    let complete = uniprot_xml(&[minimal_entry_xml("P1", ""), minimal_entry_xml("P2", "")]);
    let cut = &complete[..complete.find("</entry>").unwrap() + "</entry>".len()];
    let err = parse_uniprot_xml(cut.as_bytes(), &HashSet::new(), "test")
        .err()
        .unwrap();
    assert!(err.to_string().contains("truncated"), "{err}");
}

#[test]
fn truncated_xml_is_an_error() {
    let xml = "<uniprot><entry><accession>P1</accession>";
    assert!(parse_uniprot_xml(xml.as_bytes(), &HashSet::new(), "test").is_err());
}

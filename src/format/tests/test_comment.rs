use super::*;
use crate::test_support::rich_entry;
use crate::uniprot::Isoform;

#[test]
fn renders_the_alternative_products_block() {
    let ap = rich_entry().alternative_products.unwrap();
    assert_eq!(
        alternative_products_cc(&ap),
        "CC   -!- ALTERNATIVE PRODUCTS:\n\
         CC       Event=Alternative splicing; Named isoforms=3;\n\
         CC         Comment=Additional isoforms seem to exist.\n\
         CC       Name=1;\n\
         CC         IsoId=P99999-1; Sequence=Displayed;\n\
         CC       Name=2; Synonyms=Beta;\n\
         CC         IsoId=P99999-2; Sequence=VSP_000001, VSP_000002;\n\
         CC       Name=3;\n\
         CC         IsoId=P99999-3; Sequence=Not described;"
    );
}

#[test]
fn sequence_field_per_type() {
    let line = |sequence_type: IsoformSequenceType, vsp_ids: &[&str]| {
        let ap = AlternativeProducts {
            isoforms: vec![Isoform {
                ids: vec!["P1-2".into()],
                names: Vec::new(),
                sequence_type,
                vsp_ids: vsp_ids.iter().map(|s| s.to_string()).collect(),
            }],
            ..AlternativeProducts::default()
        };
        alternative_products_cc(&ap)
            .lines()
            .last()
            .unwrap()
            .to_string()
    };
    assert!(line(IsoformSequenceType::External, &[]).ends_with("Sequence=External;"));
    assert!(line(IsoformSequenceType::Described, &[]).ends_with("Sequence=Not described;"));
    assert!(line(IsoformSequenceType::Missing, &[]).ends_with("Sequence=Not described;"));
}

#[test]
fn capitalize_uppercases_the_first_letter() {
    assert_eq!(capitalize("alternative promoter"), "Alternative promoter");
    assert_eq!(capitalize(""), "");
}

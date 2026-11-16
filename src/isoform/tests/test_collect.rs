use super::*;
use crate::test_support::{minimal_entry_xml, parse_entry, rich_entry};

#[test]
fn collects_displayed_alias_and_isoforms() {
    let EntryIsoforms {
        displayed_id,
        sequences,
    } = collect_and_reconstruct_isoforms(&rich_entry());

    assert_eq!(displayed_id, Some(UniProtIsoId::new("P99999-1")));
    // -1 is the canonical alias and -3 is not described: only -2 remains.
    assert_eq!(sequences.len(), 1);
    assert_eq!(
        sequences[&UniProtIsoId::new("P99999-2")].as_str(),
        "AWEFGHIKLMNPQRST"
    );
}

#[test]
fn entry_without_alternative_products_has_no_isoforms() {
    let entry = parse_entry(&minimal_entry_xml(
        "P1",
        r#"<feature type="sequence variant" id="VAR_1"><original>K</original><variation>E</variation><location><position position="9"/></location></feature>"#,
    ));
    let collected = collect_and_reconstruct_isoforms(&entry);
    assert_eq!(collected.displayed_id, None);
    assert!(collected.sequences.is_empty());
}

use super::*;

fn id(accession: &str) -> UniProtCanonId {
    UniProtCanonId::new(accession)
}

fn entry(accession: &str) -> UniProtEntry {
    UniProtEntry {
        accessions: vec![accession.to_string()],
        ..UniProtEntry::default()
    }
}

fn accessions(entries: UniProtEntries) -> Vec<String> {
    entries
        .into_vec()
        .into_iter()
        .map(|(id, _)| id.to_string())
        .collect()
}

#[test]
fn insert_rejects_duplicates() {
    let mut entries = UniProtEntries::default();
    assert!(entries.insert(id("P1"), entry("P1")));
    assert!(!entries.insert(id("P1"), entry("P1")));
    assert_eq!(entries.len(), 1);
}

#[test]
fn missing_keeps_required_order_without_duplicates() {
    let mut entries = UniProtEntries::default();
    entries.insert(id("P2"), entry("P2"));
    let required = [id("P3"), id("P2"), id("P1"), id("P3")];
    assert_eq!(entries.missing(&required), [id("P3"), id("P1")]);
}

#[test]
fn add_fetched_appends_in_missing_order_and_reports_the_rest() {
    let mut entries = UniProtEntries::default();
    entries.insert(id("P0"), entry("P0"));
    let fetched: HashMap<_, _> = [(id("P1"), entry("P1")), (id("P3"), entry("P3"))].into();

    let unresolved = entries.add_fetched(vec![id("P3"), id("P2"), id("P1")], fetched);

    assert_eq!(unresolved, [id("P2")]);
    assert_eq!(accessions(entries), ["P0", "P3", "P1"]);
}

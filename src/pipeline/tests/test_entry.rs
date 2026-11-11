use super::*;
use crate::test_support::{minimal_entry_xml, parse_entry, rich_entry, variant};

/// Sample variants for the rich entry's transcripts: ENST..01 (displayed
/// isoform -1), ENST..02 (isoform -2), ENST..03 (canonical) and an
/// unrelated ENST..09.
fn sample_variants() -> HashMap<EnsemblId, Vec<Variant>> {
    [
        ("ENST00000000001", variant("s1", 1, 1, Some("A"), Some("M"))),
        ("ENST00000000002", variant("s2", 2, 2, Some("W"), Some("Y"))),
        ("ENST00000000003", variant("s3", 4, 4, Some("E"), Some("Q"))),
        ("ENST00000000009", variant("s9", 1, 1, Some("A"), Some("C"))),
    ]
    .into_iter()
    .map(|(enst, v)| (EnsemblId::new(enst), vec![v]))
    .collect()
}

fn context(uniprot_variants_enabled: bool) -> EntryContext {
    EntryContext {
        sample_variants: Arc::new(sample_variants()),
        uniprot_variants_enabled,
    }
}

fn sorted<T: Ord + Clone>(set: &HashSet<T>) -> Vec<T> {
    let mut items: Vec<T> = set.iter().cloned().collect();
    items.sort();
    items
}

#[test]
fn sample_variants_are_routed_to_canonical_or_isoform() {
    let entry = rich_entry();
    let isoforms = collect_and_reconstruct_isoforms(&entry);
    let (variants, consumed) = collect_sample_variants(
        &UniProtCanonId::new("P99999"),
        isoforms.displayed_id.as_ref(),
        &entry,
        &isoforms.sequences,
        &sample_variants(),
    );

    let placed: Vec<(&str, Option<&str>)> = variants
        .iter()
        .map(|v| (v.id.as_str(), v.isoform.as_ref().map(UniProtIsoId::as_str)))
        .collect();
    // The displayed isoform -1 is an alias for the canonical sequence.
    assert_eq!(
        placed,
        [("s1", None), ("s2", Some("P99999-2")), ("s3", None)]
    );
    assert_eq!(
        sorted(&consumed),
        ["ENST00000000001", "ENST00000000002", "ENST00000000003"].map(EnsemblId::new)
    );
}

#[test]
fn transcripts_of_unreconstructed_isoforms_are_not_consumed() {
    let entry = rich_entry();
    let (variants, consumed) = collect_sample_variants(
        &UniProtCanonId::new("P99999"),
        None,
        &entry,
        &HashMap::new(),
        &sample_variants(),
    );
    // Without the -1 alias and the -2 sequence only the canonical ENST..03 is used.
    assert_eq!(variants.len(), 1);
    assert_eq!(sorted(&consumed), [EnsemblId::new("ENST00000000003")]);
}

#[tokio::test]
async fn uniprot_variants_are_included_only_when_enabled() {
    let id = UniProtCanonId::new("P99999");
    let (with, _) = process_entry(&id, rich_entry(), context(true))
        .await
        .unwrap();
    let (without, _) = process_entry(&id, rich_entry(), context(false))
        .await
        .unwrap();
    assert!(with.contains("/id=\"VAR_000001\""));
    assert!(!without.contains("/id=\"VAR_000001\""));
    assert!(without.contains("/id=\"s3\""));
}

#[tokio::test]
async fn identical_sample_and_uniprot_variants_are_merged() {
    let id = UniProtCanonId::new("P99999");
    let mut ctx = context(true);
    // Same change as VAR_000001 (9 K -> E), via the canonical transcript.
    ctx.sample_variants = Arc::new(
        [(
            EnsemblId::new("ENST00000000003"),
            vec![variant("s3", 9, 9, Some("K"), Some("E"))],
        )]
        .into(),
    );
    let (line, _) = process_entry(&id, rich_entry(), ctx).await.unwrap();
    assert!(line.contains("/id=\"s3_VAR_000001\""), "{line}");
}

#[tokio::test]
async fn process_entries_writes_in_input_order_and_reports_consumed_transcripts() {
    let entries: Vec<(UniProtCanonId, UniProtEntry)> = ["P3", "P1", "P2"]
        .into_iter()
        .map(|a| {
            (
                UniProtCanonId::new(a),
                parse_entry(&minimal_entry_xml(a, "")),
            )
        })
        .chain([(UniProtCanonId::new("P99999"), rich_entry())])
        .collect();
    let mut out = Vec::new();

    let consumed = process_entries(&mut out, entries, context(true))
        .await
        .unwrap();

    let out = String::from_utf8(out).unwrap();
    let ids: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("ID   "))
        .map(|l| &l[..6])
        .collect();
    assert_eq!(ids, ["P3    ", "P1    ", "P2    ", "P99999"]);
    assert_eq!(consumed.len(), 3);
}

#[test]
fn failed_entries_are_skipped_without_output() {
    let mut out = Vec::new();
    let mut consumed = HashSet::new();
    let failed: EntryResult = Err(anyhow::anyhow!("boom"));
    write_entry_result(&mut out, &mut consumed, &UniProtCanonId::new("P1"), failed).unwrap();
    assert!(out.is_empty());
    assert!(consumed.is_empty());
}

#[tokio::test]
async fn uniprot_variants_are_resolved_against_their_sequence() {
    let id = UniProtCanonId::new("P11111");
    let body = r#"<feature type="sequence variant" id="VAR_OK"><original>C</original><variation>W</variation><location><position position="2"/></location></feature>
  <feature type="sequence variant" id="VAR_REF"><original>W</original><variation>C</variation><location><position position="2"/></location></feature>
  <feature type="sequence variant" id="VAR_END"><original>Y</original><variation>W</variation><location><begin position="20"/><end position="21"/></location></feature>
  <feature type="sequence variant" id="VAR_STOP"><original>R</original><variation>*</variation><location><position position="15"/></location></feature>
  <feature type="sequence variant" id="VAR_ISO"><original>A</original><variation>C</variation><location sequence="P11111-3"><position position="1"/></location></feature>"#;
    let entry = parse_entry(&minimal_entry_xml("P11111", body));
    let (line, _) = process_entry(&id, entry, context(true)).await.unwrap();

    assert!(line.contains("/id=\"VAR_OK\""));
    // reference mismatch, end past the sequence, isoform without a sequence
    for dropped in ["VAR_REF", "VAR_END", "VAR_ISO"] {
        assert!(!line.contains(&format!("/id=\"{dropped}\"")), "{dropped}");
    }
    // the stop gained at 15 now spans to the sequence end
    assert!(line.contains("FT   VARIANT         15..20\nFT                   /note=\"Missing\"\nFT                   /id=\"VAR_STOP\""));
}

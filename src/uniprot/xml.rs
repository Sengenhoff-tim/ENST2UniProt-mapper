//! Streaming reader for UniProtKB XML (as distributed by UniProt and as
//! returned by the REST API with `format=xml`). Only the elements listed on
//! `UniProtEntry` are read; everything else is skipped, and unknown values
//! never cause an entry to be rejected.

use std::collections::HashSet;
use std::io::BufRead;
use std::path::Path;

use anyhow::{Context, Result, bail};
use quick_xml::Reader;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use tracing::{info, warn};

use crate::types::UniProtCanonId;
use crate::util::open_maybe_gz;

use super::entries::UniProtEntries;
use super::model::{
    AlternativeProducts, EnsemblRef, Evidence, EvidenceSource, Feature, FeatureKind, Isoform,
    IsoformSequenceType, UniProtEntry,
};

/// Stage label for log events of this reader.
const STAGE: &str = "UNIPROT XML";

/// Streams UniProt XML from `reader` and returns its entries in document
/// order, except those whose primary accession is in `omit` (these are
/// skipped without being parsed). Duplicate primary accessions keep the
/// first occurrence.
///
/// `source` names the input in log messages.
pub(crate) fn parse_uniprot_xml<B: BufRead>(
    reader: B,
    omit: &HashSet<UniProtCanonId>,
    source: &str,
) -> Result<UniProtEntries> {
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().expand_empty_elements = true;
    xml.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut entries = UniProtEntries::default();
    let mut seen = 0usize;
    // Whether `</uniprot>` was reached. A document cut off between two
    // entries is otherwise well-formed XML, just silently incomplete.
    let mut root_closed = false;

    loop {
        buf.clear();
        let event = xml.read_event_into(&mut buf).with_context(|| {
            format!(
                "malformed UniProt XML in {} at byte {}",
                source,
                xml.buffer_position()
            )
        })?;
        match event {
            Event::Start(e) if e.local_name().as_ref() == b"entry" => {
                seen += 1;
                let end = e.to_end().into_owned();
                let position = xml.buffer_position();
                let entry = parse_entry(&mut xml, &end, omit).with_context(|| {
                    format!(
                        "malformed UniProt XML in {} (entry starting at byte {})",
                        source, position
                    )
                })?;
                if let Some(entry) = entry {
                    let id = UniProtCanonId::new(entry.primary_accession());
                    if !entries.insert(id.clone(), entry) {
                        warn!(
                            stage = STAGE,
                            rejected = true,
                            "{}: duplicate entry for {}, keeping the first",
                            source,
                            id.as_str()
                        );
                    }
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"uniprot" => root_closed = true,
            Event::Eof => break,
            _ => {}
        }
    }
    if !root_closed {
        bail!(
            "UniProt XML in {} ends before </uniprot> after {} entries: the document is truncated",
            source,
            seen
        );
    }

    info!(
        "{}: read {} entries, kept {}, omitted {}",
        source,
        seen,
        entries.len(),
        seen - entries.len()
    );
    Ok(entries)
}

/// Loads all entries of a local UniProt XML file (optionally gzipped),
/// except those in `omit`.
pub(crate) fn load_uniprot_xml(
    path: &Path,
    omit: &HashSet<UniProtCanonId>,
) -> Result<UniProtEntries> {
    let reader = open_maybe_gz(path)?;
    parse_uniprot_xml(reader, omit, &path.display().to_string())
        .with_context(|| format!("failed to read UniProt XML '{}'", path.display()))
}

/// Reads one `<entry>` (whose start tag has just been consumed) up to and
/// including `</entry>`. Returns `None` if its primary accession is in
/// `omit` (the rest of the entry is then skipped unparsed) or if the entry
/// lacks an accession or sequence.
fn parse_entry<B: BufRead>(
    xml: &mut Reader<B>,
    entry_end: &BytesEnd<'static>,
    omit: &HashSet<UniProtCanonId>,
) -> Result<Option<UniProtEntry>> {
    let mut entry = UniProtEntry::default();
    let mut buf = Vec::new();

    // Element names from (excluding) <entry> down to the current element.
    let mut path: Vec<String> = Vec::new();
    // Text content of the innermost element.
    let mut text = String::new();

    let mut gene_count = 0usize;
    let mut capture_gene_name = false;
    let mut in_alternative_products = false;
    let mut feature: Option<Feature> = None;
    let mut isoform: Option<Isoform> = None;
    let mut ensembl: Option<EnsemblRef> = None;
    let mut evidence: Option<(String, Evidence)> = None;

    loop {
        buf.clear();
        match xml.read_event_into(&mut buf)? {
            Event::Start(e) => {
                text.clear();
                path.push(String::from_utf8_lossy(e.local_name().as_ref()).into_owned());
                match path.join("/").as_str() {
                    "gene" => gene_count += 1,
                    "gene/name" => {
                        capture_gene_name = gene_count == 1
                            && entry.gene_name.is_none()
                            && attr(&e, "type")?.as_deref() == Some("primary");
                    }
                    "comment" => {
                        in_alternative_products = entry.alternative_products.is_none()
                            && attr(&e, "type")?.as_deref() == Some("alternative products");
                        if in_alternative_products {
                            entry.alternative_products = Some(AlternativeProducts::default());
                        }
                    }
                    "comment/event" if in_alternative_products => {
                        if let (Some(ap), Some(ty)) =
                            (entry.alternative_products.as_mut(), attr(&e, "type")?)
                        {
                            ap.events.push(ty);
                        }
                    }
                    "comment/isoform" if in_alternative_products => {
                        isoform = Some(Isoform::default())
                    }
                    "comment/isoform/sequence" => {
                        if let Some(iso) = isoform.as_mut() {
                            iso.sequence_type = attr(&e, "type")?
                                .map(|t| IsoformSequenceType::parse(&t))
                                .unwrap_or_default();
                            iso.vsp_ids = attr(&e, "ref")?
                                .map(|r| r.split_whitespace().map(str::to_string).collect())
                                .unwrap_or_default();
                        }
                    }
                    "dbReference" => {
                        ensembl = match (attr(&e, "type")?.as_deref(), attr(&e, "id")?) {
                            (Some("Ensembl"), Some(id)) => Some(EnsemblRef {
                                transcript_id: id,
                                molecule: None,
                            }),
                            _ => None,
                        };
                    }
                    "dbReference/molecule" => {
                        if let Some(r) = ensembl.as_mut() {
                            r.molecule = attr(&e, "id")?;
                        }
                    }
                    "feature" => {
                        let kind = match attr(&e, "type")?.as_deref() {
                            Some("splice variant") => Some(FeatureKind::SpliceVariant),
                            Some("sequence variant") => Some(FeatureKind::SequenceVariant),
                            _ => None,
                        };
                        feature = match kind {
                            Some(kind) => Some(Feature {
                                kind,
                                id: attr(&e, "id")?,
                                description: attr(&e, "description")?,
                                original: None,
                                variations: Vec::new(),
                                begin: None,
                                end: None,
                                location_sequence: None,
                                evidence_keys: attr(&e, "evidence")?
                                    .map(|k| k.split_whitespace().map(str::to_string).collect())
                                    .unwrap_or_default(),
                            }),
                            None => None,
                        };
                    }
                    "feature/location" => {
                        if let Some(f) = feature.as_mut() {
                            f.location_sequence = attr(&e, "sequence")?;
                        }
                    }
                    "feature/location/begin" => {
                        if let Some(f) = feature.as_mut() {
                            f.begin = position(&e)?;
                        }
                    }
                    "feature/location/end" => {
                        if let Some(f) = feature.as_mut() {
                            f.end = position(&e)?;
                        }
                    }
                    "feature/location/position" => {
                        if let Some(f) = feature.as_mut() {
                            f.begin = position(&e)?;
                            f.end = f.begin;
                        }
                    }
                    "evidence" => {
                        if let (Some(key), Some(code)) = (attr(&e, "key")?, attr(&e, "type")?) {
                            evidence = Some((key, Evidence { code, source: None }));
                        }
                    }
                    "evidence/source" => {
                        if let (Some((_, ev)), Some(r)) = (evidence.as_mut(), attr(&e, "ref")?) {
                            ev.source = Some(EvidenceSource::Ref(r));
                        }
                    }
                    "evidence/source/dbReference" => {
                        if let (Some((_, ev)), Some(db), Some(id)) =
                            (evidence.as_mut(), attr(&e, "type")?, attr(&e, "id")?)
                        {
                            ev.source = Some(EvidenceSource::DbRef { db, id });
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(t) => text.push_str(&t.unescape()?),
            Event::End(_) if path.is_empty() => break, // </entry>
            Event::End(_) => {
                match path.join("/").as_str() {
                    "accession" => {
                        entry.accessions.push(std::mem::take(&mut text));
                        if entry.accessions.len() == 1
                            && omit.contains(entry.accessions[0].as_str())
                        {
                            xml.read_to_end_into(entry_end.name(), &mut Vec::new())?;
                            return Ok(None);
                        }
                    }
                    "protein/recommendedName/fullName" => {
                        entry.protein_name = Some(std::mem::take(&mut text));
                    }
                    "gene/name" if capture_gene_name => {
                        entry.gene_name = Some(std::mem::take(&mut text));
                        capture_gene_name = false;
                    }
                    "comment" => in_alternative_products = false,
                    "comment/text" if in_alternative_products => {
                        if let Some(ap) = entry.alternative_products.as_mut() {
                            ap.notes.push(std::mem::take(&mut text));
                        }
                    }
                    "comment/isoform/id" => {
                        if let Some(iso) = isoform.as_mut() {
                            iso.ids.push(std::mem::take(&mut text));
                        }
                    }
                    "comment/isoform/name" => {
                        if let Some(iso) = isoform.as_mut() {
                            iso.names.push(std::mem::take(&mut text));
                        }
                    }
                    "comment/isoform" => {
                        if let (Some(ap), Some(iso)) =
                            (entry.alternative_products.as_mut(), isoform.take())
                        {
                            ap.isoforms.push(iso);
                        }
                    }
                    "dbReference" => entry.ensembl_refs.extend(ensembl.take()),
                    "feature/original" => {
                        if let Some(f) = feature.as_mut() {
                            f.original = Some(std::mem::take(&mut text));
                        }
                    }
                    "feature/variation" => {
                        if let Some(f) = feature.as_mut() {
                            f.variations.push(std::mem::take(&mut text));
                        }
                    }
                    "feature" => entry.features.extend(feature.take()),
                    "evidence" => {
                        if let Some((key, ev)) = evidence.take() {
                            entry.evidences.insert(key, ev);
                        }
                    }
                    "sequence" => {
                        entry.sequence = text.split_whitespace().collect();
                    }
                    _ => {}
                }
                path.pop();
                text.clear();
            }
            Event::Eof => bail!("unexpected end of file inside <entry>"),
            _ => {}
        }
    }

    if entry.accessions.is_empty() {
        warn!(
            stage = STAGE,
            rejected = true,
            "skipped UniProt entry without <accession>"
        );
        return Ok(None);
    }
    if entry.sequence.is_empty() {
        warn!(
            stage = STAGE,
            rejected = true,
            "{}: skipped UniProt entry without <sequence>",
            entry.primary_accession()
        );
        return Ok(None);
    }
    Ok(Some(entry))
}

fn attr(e: &BytesStart, name: &str) -> Result<Option<String>> {
    Ok(match e.try_get_attribute(name)? {
        Some(a) => Some(a.unescape_value()?.into_owned()),
        None => None,
    })
}

/// `<begin position="12"/>`; `None` for e.g. `<begin status="unknown"/>`.
fn position(e: &BytesStart) -> Result<Option<usize>> {
    Ok(attr(e, "position")?.and_then(|p| p.parse().ok()))
}

#[cfg(test)]
mod tests {
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
        let err = parse_uniprot_xml(cut.as_bytes(), &HashSet::new(), "test").err().unwrap();
        assert!(err.to_string().contains("truncated"), "{err}");
    }

    #[test]
    fn truncated_xml_is_an_error() {
        let xml = "<uniprot><entry><accession>P1</accession>";
        assert!(parse_uniprot_xml(xml.as_bytes(), &HashSet::new(), "test").is_err());
    }
}

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

use crate::shared_types::UniProtCanonId;
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
#[path = "tests/test_xml.rs"]
mod tests;

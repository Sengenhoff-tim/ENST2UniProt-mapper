use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::Path;

use anyhow::Result;

use crate::types::{EnsemblId, UniProtCanonId};

use super::log::{log_pipeline_event, Stage};

/// Appends every accession that `id_mapping` maps one of the `ensts` to,
/// unless it is already in `accessions`. The ENSTs are visited sorted by id
/// (callers typically pass `HashMap` keys), so the result is stable across
/// runs.
pub(super) fn extend_with_mapped_accessions<'a>(
    accessions: &mut Vec<UniProtCanonId>,
    ensts: impl Iterator<Item = &'a EnsemblId>,
    id_mapping: &HashMap<EnsemblId, Vec<UniProtCanonId>>,
) {
    let mut existing: HashSet<UniProtCanonId> = accessions.iter().cloned().collect();

    let mut ensts: Vec<&EnsemblId> = ensts.collect();
    ensts.sort();

    for ensembl_id in ensts {
        if let Some(uniprot_ids) = id_mapping.get(ensembl_id) {
            for uniprot_id in uniprot_ids {
                if existing.insert(uniprot_id.clone()) {
                    accessions.push(uniprot_id.clone());
                }
            }
        }
    }
}

/// Reads the accessions to omit, one per line. No path means nothing is omitted.
pub(super) fn read_omit_list(path: Option<&Path>) -> Result<HashSet<UniProtCanonId>> {
    Ok(match path {
        Some(path) => {
            let file = std::fs::File::open(path)?;
            std::io::BufReader::new(file)
                .lines()
                .collect::<std::io::Result<Vec<String>>>()?
                .iter()
                .map(|line| line.trim())
                .filter(|s| !s.is_empty())
                .map(UniProtCanonId::new)
                .collect()
        }
        None => HashSet::new(),
    })
}

/// Sample-variant ENSTs that no processed UniProt entry consumed: no id
/// mapping, a mapped accession found neither in the XML nor via the API, or
/// not cross-referenced by the entry. They get a sequence from Ensembl
/// (FASTA, then API) so their variants are kept.
///
/// The id mapping is assumed to be complete: an ENST is dropped (and logged)
/// only if the mapping links it to an omitted accession. An ENST absent from
/// the mapping has no UniProt identity and always gets its Ensembl entry.
/// Sorted for stable output.
pub(super) fn ensembl_fallback_candidates<'a>(
    variant_ensts: impl Iterator<Item = &'a EnsemblId>,
    id_mapping: &HashMap<EnsemblId, Vec<UniProtCanonId>>,
    assigned: &HashSet<EnsemblId>,
    omit: &HashSet<UniProtCanonId>,
) -> Vec<EnsemblId> {
    let mut result: Vec<EnsemblId> = variant_ensts
        .filter(|enst| !assigned.contains(*enst))
        .filter(|enst| {
            let omitted = id_mapping
                .get(*enst)
                .into_iter()
                .flatten()
                .find(|id| omit.contains(*id))
                .cloned();
            if let Some(id) = &omitted {
                log_pipeline_event(
                    Stage::TranscriptMapping,
                    enst.as_str(),
                    &format!("excluded from Ensembl fallback, mapped to omitted {}", id.as_str()),
                );
            }
            omitted.is_none()
        })
        .cloned()
        .collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{strs, write_file};

    fn enst(id: &str) -> EnsemblId {
        EnsemblId::new(id)
    }

    fn acc(id: &str) -> UniProtCanonId {
        UniProtCanonId::new(id)
    }

    fn mapping(pairs: &[(&str, &[&str])]) -> HashMap<EnsemblId, Vec<UniProtCanonId>> {
        pairs.iter().map(|(e, accs)| (enst(e), accs.iter().map(|a| acc(a)).collect())).collect()
    }

    #[test]
    fn extends_accessions_in_transcript_order_without_duplicates() {
        let id_mapping = mapping(&[("ENST2", &["P2", "P1"]), ("ENST1", &["P3"]), ("ENST9", &["P9"])]);
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
        let id_mapping = mapping(&[("ENST1", &["P1"]), ("ENST2", &["P2"]), ("ENST3", &["P3", "P4"])]);
        let variant_ensts = [enst("ENST4"), enst("ENST3"), enst("ENST2"), enst("ENST1")];
        let consumed: HashSet<EnsemblId> = [enst("ENST1")].into();
        let omit: HashSet<UniProtCanonId> = [acc("P4")].into();

        let candidates = ensembl_fallback_candidates(variant_ensts.iter(), &id_mapping, &consumed, &omit);

        // ENST1 consumed, ENST3 maps to omitted P4, ENST4 has no mapping and is kept.
        assert_eq!(strs(&candidates), ["ENST2", "ENST4"]);
    }
}

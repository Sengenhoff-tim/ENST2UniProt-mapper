use std::collections::{HashMap, HashSet};

use crate::shared_types::{EnsemblId, UniProtCanonId};

use super::log::{Stage, log_pipeline_event};

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
                    &format!(
                        "excluded from Ensembl fallback, mapped to omitted {}",
                        id.as_str()
                    ),
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
#[path = "tests/test_mapping.rs"]
mod tests;

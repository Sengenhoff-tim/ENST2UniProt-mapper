use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;

use anyhow::{Context, Result};

use crate::shared_types::{EnsemblId, UniProtCanonId, UniProtId};
use crate::util::open_maybe_gz;

use super::log::{Stage, log_line_rejection};

/// The id type of the lines that link a UniProt accession to an Ensembl transcript.
const ENSEMBL_TRANSCRIPT_ID_TYPE: &str = "Ensembl_TRS";

/// Parses a UniProt id mapping file as downloaded (e.g.
/// `HUMAN_9606_idmapping.dat.gz`; `.gz` is decompressed transparently),
/// returning a map of Ensembl-ID → list of UniProt-Canon-IDs (an ENST can map
/// to more than one UniProt accession).
///
/// Expected format: tab-separated `<uniprot_id>[-<isoform>]\t<id_type>\t<id>`.
/// Only `Ensembl_TRS` lines are used; all other id types are skipped silently.
/// e.g. "P62258-2\tEnsembl_TRS\tENST00000571732.5" -> (ENST00000571732, [P62258])
pub(crate) fn parse_idmapping(path: &Path) -> Result<HashMap<EnsemblId, Vec<UniProtCanonId>>> {
    let reader = open_maybe_gz(path)
        .with_context(|| format!("failed to open id mapping file '{}'", path.display()))?;
    let mut mapping: HashMap<EnsemblId, Vec<UniProtCanonId>> = HashMap::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line =
            line.with_context(|| format!("failed to read id mapping file '{}'", path.display()))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let location = || format!("{}:{}", path.display(), line_no + 1);

        let mut parts = line.split('\t');
        let (Some(uniprot_raw), Some(id_type), Some(id_raw)) =
            (parts.next(), parts.next(), parts.next())
        else {
            log_line_rejection(
                Stage::IdMappingParsing,
                &location(),
                &format!("skipped, expected 3 tab-separated columns in '{}'", line),
            );
            continue;
        };

        if id_type != ENSEMBL_TRANSCRIPT_ID_TYPE {
            continue;
        }

        // e.g. "P62258-2" -> "P62258"
        let uniprot_id = UniProtId::from_accession(uniprot_raw).canonical();
        // e.g. "ENST00000571732.5" -> "ENST00000571732"
        let enst_id = EnsemblId::from_versioned(id_raw);

        if !enst_id.is_transcript() {
            log_line_rejection(
                Stage::IdMappingParsing,
                &location(),
                &format!("skipped, '{}' does not look like an ENST id", id_raw),
            );
            continue;
        }

        let entry = mapping.entry(enst_id).or_default();
        if !entry.contains(&uniprot_id) {
            entry.push(uniprot_id);
        }
    }
    Ok(mapping)
}

#[cfg(test)]
#[path = "tests/test_idmapping.rs"]
mod tests;

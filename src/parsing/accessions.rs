use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::shared_types::{UniProtCanonId, UniProtId};

use super::log::{Stage, log_line_event};

/// Reads a list of UniProt accessions, one per line (trimmed, blank lines
/// skipped). Isoform accessions such as `P04637-2` are replaced by their
/// canonical accession (`P04637`), since entries are looked up by primary
/// accession; each replacement is logged.
pub(crate) fn parse_accession_list(path: &Path) -> Result<Vec<UniProtCanonId>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read accession list '{}'", path.display()))?;
    let mut accessions = Vec::new();
    for (line_no, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let id = match UniProtId::from_accession(line) {
            UniProtId::Canonical(id) => id,
            UniProtId::Isoform(iso) => {
                let canonical = iso.canonical();
                log_line_event(
                    Stage::AccessionList,
                    &format!("{}:{}", path.display(), line_no + 1),
                    &format!("isoform accession {iso} replaced by its entry {canonical}"),
                );
                canonical
            }
        };
        accessions.push(id);
    }
    Ok(accessions)
}

#[cfg(test)]
#[path = "tests/test_accessions.rs"]
mod tests;

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::shared_types::{UniProtCanonId, UniProtId};

use super::log::{Stage, log_line_event, log_line_rejection};

/// Reads the accessions to ensure, one per line (trimmed, blank lines
/// skipped). Isoform accessions such as `P04637-2` are replaced by their
/// entry's accession (`P04637`): isoforms are written as part of their
/// entry, never as entries of their own. Each replacement is logged.
pub(crate) fn parse_accession_list(path: &Path) -> Result<Vec<UniProtCanonId>> {
    let mut accessions = Vec::new();
    for (location, id) in read_accessions(path)? {
        accessions.push(match id {
            UniProtId::Canonical(id) => id,
            UniProtId::Isoform(iso) => {
                let canonical = iso.canonical();
                log_line_event(
                    Stage::AccessionList,
                    &location,
                    &format!("isoform accession {iso} replaced by its entry {canonical}"),
                );
                canonical
            }
        });
    }
    Ok(accessions)
}

/// Reads the accessions to exclude, one per line (trimmed, blank lines
/// skipped). Only whole entries can be excluded: isoform accessions such as
/// `P04637-2` are logged as rejected and ignored, so that they do not
/// exclude the entire entry `P04637`.
pub(crate) fn parse_exclude_list(path: &Path) -> Result<HashSet<UniProtCanonId>> {
    let mut accessions = HashSet::new();
    for (location, id) in read_accessions(path)? {
        match id {
            UniProtId::Canonical(id) => {
                accessions.insert(id);
            }
            UniProtId::Isoform(iso) => log_line_rejection(
                Stage::AccessionList,
                &location,
                &format!("ignored isoform accession {iso}: only whole entries can be excluded"),
            ),
        }
    }
    Ok(accessions)
}

/// The non-blank lines of `path`, trimmed and classified, each with its
/// `<path>:<line>` location.
fn read_accessions(path: &Path) -> Result<Vec<(String, UniProtId)>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read accession list '{}'", path.display()))?;
    Ok(content
        .lines()
        .enumerate()
        .map(|(line_no, line)| (line_no, line.trim()))
        .filter(|(_, line)| !line.is_empty())
        .map(|(line_no, line)| {
            (
                format!("{}:{}", path.display(), line_no + 1),
                UniProtId::from_accession(line),
            )
        })
        .collect())
}

#[cfg(test)]
#[path = "tests/test_accessions.rs"]
mod tests;

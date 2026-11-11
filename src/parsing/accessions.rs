use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::shared_types::UniProtCanonId;

pub(crate) fn parse_accession_list(path: &Path) -> Result<Vec<UniProtCanonId>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read accession list '{}'", path.display()))?;
    Ok(content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(UniProtCanonId::new)
        .collect())
}

#[cfg(test)]
#[path = "tests/test_accessions.rs"]
mod tests;

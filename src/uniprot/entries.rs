use std::collections::{HashMap, HashSet};

use tracing::warn;

use crate::shared_types::UniProtCanonId;

use super::model::UniProtEntry;

/// The UniProt entries to process: XML entries in file order, then entries
/// fetched from the API in the order they were required.
#[derive(Debug, Default)]
pub(crate) struct UniProtEntries {
    entries: Vec<(UniProtCanonId, UniProtEntry)>,
    /// Primary accessions of `entries`, for lookups.
    ids: HashSet<UniProtCanonId>,
}

impl UniProtEntries {
    /// Adds `entry` unless an entry with the same accession is already
    /// present. Returns whether it was added.
    pub(crate) fn insert(&mut self, id: UniProtCanonId, entry: UniProtEntry) -> bool {
        let added = self.ids.insert(id.clone());
        if added {
            self.entries.push((id, entry));
        }
        added
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Accessions of `required` that are not present, deduplicated, in
    /// `required` order.
    pub(crate) fn missing(&self, required: &[UniProtCanonId]) -> Vec<UniProtCanonId> {
        let mut seen = HashSet::new();
        required
            .iter()
            .filter(|id| !self.ids.contains(*id) && seen.insert(*id))
            .cloned()
            .collect()
    }

    /// Adds the entries `fetched` returned for `missing`, in `missing` order.
    /// Returns the accessions of `missing` that were not fetched.
    pub(crate) fn add_fetched(
        &mut self,
        missing: Vec<UniProtCanonId>,
        mut fetched: HashMap<UniProtCanonId, UniProtEntry>,
    ) -> Vec<UniProtCanonId> {
        let mut unresolved = Vec::new();
        for id in missing {
            match fetched.remove(&id) {
                Some(entry) => {
                    self.insert(id, entry);
                }
                None => unresolved.push(id),
            }
        }
        unresolved
    }

    pub(crate) fn into_vec(self) -> Vec<(UniProtCanonId, UniProtEntry)> {
        self.entries
    }
}

/// Logs the required accessions found neither in the XML nor via the API.
pub(crate) fn log_unresolved_accessions(unresolved: &[UniProtCanonId], api_fallback: bool) {
    if unresolved.is_empty() {
        return;
    }
    warn!(
        stage = "UNIPROT LOOKUP",
        rejected = true,
        count = unresolved.len(),
        "{} required accessions not found as primary accession{}: {}",
        unresolved.len(),
        if api_fallback {
            " in the UniProt XML or API"
        } else {
            " in the UniProt XML (API fallback disabled)"
        },
        unresolved
            .iter()
            .map(UniProtCanonId::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );
}

#[cfg(test)]
#[path = "tests/test_entries.rs"]
mod tests;

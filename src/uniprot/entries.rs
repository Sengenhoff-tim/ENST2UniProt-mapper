use std::collections::{HashMap, HashSet};

use tracing::warn;

use crate::types::UniProtCanonId;

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
mod tests {
    use super::*;

    fn id(accession: &str) -> UniProtCanonId {
        UniProtCanonId::new(accession)
    }

    fn entry(accession: &str) -> UniProtEntry {
        UniProtEntry {
            accessions: vec![accession.to_string()],
            ..UniProtEntry::default()
        }
    }

    fn accessions(entries: UniProtEntries) -> Vec<String> {
        entries
            .into_vec()
            .into_iter()
            .map(|(id, _)| id.to_string())
            .collect()
    }

    #[test]
    fn insert_rejects_duplicates() {
        let mut entries = UniProtEntries::default();
        assert!(entries.insert(id("P1"), entry("P1")));
        assert!(!entries.insert(id("P1"), entry("P1")));
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn missing_keeps_required_order_without_duplicates() {
        let mut entries = UniProtEntries::default();
        entries.insert(id("P2"), entry("P2"));
        let required = [id("P3"), id("P2"), id("P1"), id("P3")];
        assert_eq!(entries.missing(&required), [id("P3"), id("P1")]);
    }

    #[test]
    fn add_fetched_appends_in_missing_order_and_reports_the_rest() {
        let mut entries = UniProtEntries::default();
        entries.insert(id("P0"), entry("P0"));
        let fetched: HashMap<_, _> = [(id("P1"), entry("P1")), (id("P3"), entry("P3"))].into();

        let unresolved = entries.add_fetched(vec![id("P3"), id("P2"), id("P1")], fetched);

        assert_eq!(unresolved, [id("P2")]);
        assert_eq!(accessions(entries), ["P0", "P3", "P1"]);
    }
}

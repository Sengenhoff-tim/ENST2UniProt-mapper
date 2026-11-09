use crate::types::UniProtIsoId;
use crate::uniprot::{EvidenceSource, Feature, UniProtEntry};
use crate::variant::Variant;

/// Column prefix for all FT qualifier continuation lines (21 chars).
const FT_INDENT: &str = "FT                   ";

/// FT lines for one splice variant (VAR_SEQ) feature of `entry`.
pub(super) fn var_seq_ft_lines(feature: &Feature, entry: &UniProtEntry) -> Option<Vec<String>> {
    let begin = feature.begin?;
    let end = feature.end.unwrap_or(begin);

    let mut lines = Vec::new();
    lines.push(format_ft_position_line("VAR_SEQ", &None, begin, end));

    let desc = feature.description.as_deref().unwrap_or("");
    let orig = feature.original.as_deref().unwrap_or("");
    let note = match feature.variations.first() {
        Some(alt_seq) if desc.is_empty() => format!("{} -> {}", orig, alt_seq),
        Some(alt_seq) => format!("{} -> {} ({})", orig, alt_seq, desc),
        None if desc.is_empty() => "Missing".to_string(),
        None => format!("Missing ({})", desc),
    };
    lines.extend(wrap_ft_qualifier("note", &note));

    let evidence = format_evidence_string(feature, entry);
    if !evidence.is_empty() {
        lines.extend(wrap_ft_qualifier("evidence", &evidence));
    }

    // The FT id falls back to the evidence string when the feature has no id.
    let id = feature
        .id
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| (!evidence.is_empty()).then_some(evidence));
    if let Some(id) = id {
        lines.extend(wrap_ft_qualifier("id", &id));
    }

    Some(lines)
}

impl Variant {
    /// Format this variant into VARIANT FT lines.
    pub(super) fn ft_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format_ft_position_line(
            "VARIANT",
            &self.isoform,
            self.begin,
            self.end,
        ));
        let note = if let Some(aa_new) = &self.aa_new
            && let Some(aa_ref) = &self.aa_ref
        {
            format!("{} -> {}", aa_ref, aa_new)
        } else {
            "Missing".to_string()
        };
        lines.extend(wrap_ft_qualifier("note", &note));
        //invalid in uniprot spec
        lines.push(format!("{}/{}=\"{}\"", FT_INDENT, "id", &self.id));
        lines
    }
}

fn format_ft_position_line(
    key: &str,
    isoform: &Option<UniProtIsoId>,
    begin: usize,
    end: usize,
) -> String {
    let range = if begin == end {
        begin.to_string()
    } else {
        format!("{}..{}", begin, end)
    };

    let pos = isoform
        .as_ref()
        .map(|iso| format!("{}:{}", iso.as_str(), range))
        .unwrap_or(range);

    format!("FT   {:<16}{}", key, pos)
}

fn wrap_ft_qualifier(qualifier: &str, value: &str) -> Vec<String> {
    let full = format!("/{}=\"{}\"", qualifier, value);
    vec![format!("{}{}", FT_INDENT, full)]
}

/// Resolves the feature's evidence keys against the entry's `<evidence>`
/// list, e.g. `ECO:0000303|PubMed:14702039, ECO:0000305`.
fn format_evidence_string(feature: &Feature, entry: &UniProtEntry) -> String {
    feature
        .evidence_keys
        .iter()
        .filter_map(|key| entry.evidences.get(key))
        .map(|e| match &e.source {
            Some(EvidenceSource::DbRef { db, id }) => format!("{}|{}:{}", e.code, db, id),
            Some(EvidenceSource::Ref(n)) => format!("{}|Ref.{}", e.code, n),
            None => e.code.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{iso_variant, rich_entry, variant};
    use crate::uniprot::Evidence;

    #[test]
    fn var_seq_substitution_with_evidence() {
        let entry = rich_entry();
        let lines = var_seq_ft_lines(&entry.features[0], &entry).unwrap();
        assert_eq!(
            lines,
            [
                "FT   VAR_SEQ         2..3",
                "FT                   /note=\"CD -> W (in isoform 2)\"",
                "FT                   /evidence=\"ECO:0000303|PubMed:12345\"",
                "FT                   /id=\"VSP_000001\"",
            ]
        );
    }

    #[test]
    fn var_seq_deletion_is_missing() {
        let entry = rich_entry();
        let lines = var_seq_ft_lines(&entry.features[1], &entry).unwrap();
        assert_eq!(
            lines,
            [
                "FT   VAR_SEQ         18..20",
                "FT                   /note=\"Missing (in isoform 2)\"",
                "FT                   /id=\"VSP_000002\"",
            ]
        );
    }

    #[test]
    fn var_seq_without_id_falls_back_to_evidence() {
        let mut entry = rich_entry();
        entry.features[0].id = None;
        let feature = entry.features[0].clone();
        let lines = var_seq_ft_lines(&feature, &entry).unwrap();
        assert_eq!(
            lines.last().unwrap(),
            "FT                   /id=\"ECO:0000303|PubMed:12345\""
        );
    }

    #[test]
    fn var_seq_without_position_is_skipped() {
        let mut entry = rich_entry();
        entry.features[0].begin = None;
        let feature = entry.features[0].clone();
        assert!(var_seq_ft_lines(&feature, &entry).is_none());
    }

    #[test]
    fn evidence_string_resolves_each_source_kind() {
        let mut entry = rich_entry();
        entry.evidences.insert(
            "2".into(),
            Evidence {
                code: "ECO:0000305".into(),
                source: Some(EvidenceSource::Ref("3".into())),
            },
        );
        entry.evidences.insert(
            "3".into(),
            Evidence {
                code: "ECO:0000250".into(),
                source: None,
            },
        );
        let mut feature = entry.features[0].clone();
        feature.evidence_keys = vec!["1".into(), "2".into(), "3".into(), "unknown".into()];
        assert_eq!(
            format_evidence_string(&feature, &entry),
            "ECO:0000303|PubMed:12345, ECO:0000305|Ref.3, ECO:0000250"
        );
    }

    #[test]
    fn variant_lines_for_substitution_and_missing() {
        assert_eq!(
            variant("VAR_1", 9, 9, Some("K"), Some("E")).ft_lines(),
            [
                "FT   VARIANT         9",
                "FT                   /note=\"K -> E\"",
                "FT                   /id=\"VAR_1\"",
            ]
        );
        let missing = variant("VAR_2", 3, 5, None, None).ft_lines();
        assert_eq!(missing[0], "FT   VARIANT         3..5");
        assert_eq!(missing[1], "FT                   /note=\"Missing\"");
    }

    #[test]
    fn variant_on_isoform_prefixes_the_position() {
        let lines = iso_variant("P1-2", "VAR_1", 5, 6, "GH", "A").ft_lines();
        assert_eq!(lines[0], "FT   VARIANT         P1-2:5..6");
    }
}

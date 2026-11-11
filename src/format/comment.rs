use crate::uniprot::{AlternativeProducts, IsoformSequenceType};

/// `alternative splicing` -> `Alternative splicing`, as in the flat file.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Renders the `CC -!- ALTERNATIVE PRODUCTS` block.
pub(super) fn alternative_products_cc(ap: &AlternativeProducts) -> String {
    let mut lines = Vec::new();
    lines.push("CC   -!- ALTERNATIVE PRODUCTS:".to_string());
    let events: Vec<String> = ap.events.iter().map(|e| capitalize(e)).collect();
    lines.push(format!(
        "CC       Event={}; Named isoforms={};",
        events.join(", "),
        ap.isoforms.len()
    ));

    for note in &ap.notes {
        lines.push(format!("CC         Comment={}", note));
    }

    for iso in &ap.isoforms {
        // The first <name> is the isoform name, any further ones are synonyms.
        if let Some((name, synonyms)) = iso.names.split_first() {
            if synonyms.is_empty() {
                lines.push(format!("CC       Name={};", name));
            } else {
                lines.push(format!(
                    "CC       Name={}; Synonyms={};",
                    name,
                    synonyms.join(", ")
                ));
            }
        }

        let iso_id = iso.ids.first().map(String::as_str).unwrap_or("");
        let seq_str = match &iso.sequence_type {
            IsoformSequenceType::Displayed => "Displayed".to_string(),
            IsoformSequenceType::External => "External".to_string(),
            IsoformSequenceType::Described if !iso.vsp_ids.is_empty() => iso.vsp_ids.join(", "),
            _ => "Not described".to_string(),
        };
        lines.push(format!(
            "CC         IsoId={}; Sequence={};",
            iso_id, seq_str
        ));
    }

    lines.join("\n")
}

#[cfg(test)]
#[path = "tests/test_comment.rs"]
mod tests;

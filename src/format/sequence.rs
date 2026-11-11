/// SQ body: 60 residues per line, in groups of 10.
pub(super) fn format_seq_body(seq: &str) -> String {
    seq.as_bytes()
        .chunks(60)
        .map(|chunk| {
            let line = std::str::from_utf8(chunk).unwrap_or("");
            let grouped = line
                .as_bytes()
                .chunks(10)
                .map(|g| std::str::from_utf8(g).unwrap_or(""))
                .collect::<Vec<_>>()
                .join(" ");
            format!("     {}", grouped)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[path = "tests/test_sequence.rs"]
mod tests;

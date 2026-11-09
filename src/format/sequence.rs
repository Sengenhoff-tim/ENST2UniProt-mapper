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
mod tests {
    use super::*;

    #[test]
    fn groups_residues_by_ten_and_lines_by_sixty() {
        let seq = "A".repeat(65);
        let body = format_seq_body(&seq);
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], format!("     {}", ["AAAAAAAAAA"; 6].join(" ")));
        assert_eq!(lines[1], "     AAAAA");
    }

    #[test]
    fn short_sequence_is_one_line() {
        assert_eq!(format_seq_body("ACDEFGHIKLMN"), "     ACDEFGHIKL MN");
    }
}

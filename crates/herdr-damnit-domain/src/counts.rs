pub fn counted(counts: &[(u64, &str)]) -> String {
    counts
        .iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, what)| format!("{count} {what}"))
        .collect::<Vec<_>>()
        .join("  ")
}

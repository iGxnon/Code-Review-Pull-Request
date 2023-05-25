static CHAR_SOFT_LIMIT: usize = 9000;

pub(crate) fn truncate(s: &str) -> &str {
    match s.char_indices().nth(CHAR_SOFT_LIMIT) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

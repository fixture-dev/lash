//! UTF-8-safe text truncation helpers.
//!
//! Byte-index slicing (`&s[..n]`) panics when `n` falls inside a multi-byte
//! character. These helpers clamp indices to char boundaries so display
//! truncation never panics on user text (em dashes, accents, CJK, emoji).

/// Find the largest index at or below `index` that is a char boundary of `s`.
///
/// Returns `s.len()` if `index` is past the end of the string. This mirrors
/// the unstable `str::floor_char_boundary`; swap to the std method once it
/// stabilizes.
///
/// # Examples
///
/// ```
/// use lash_types::text::floor_char_boundary;
///
/// let s = "a—b"; // '—' occupies bytes 1..4
/// assert_eq!(floor_char_boundary(s, 2), 1);
/// assert_eq!(floor_char_boundary(s, 4), 4);
/// assert_eq!(floor_char_boundary(s, 99), s.len());
/// ```
#[must_use]
pub fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Truncate `s` to at most `max_len` bytes, appending `"..."` when truncated.
///
/// The ellipsis counts toward `max_len`, so the result is never longer than
/// `max_len` bytes (except when `max_len < 3`, where `"..."` is returned
/// as-is). Truncation lands on a char boundary, so multi-byte characters at
/// the cut point are dropped rather than split.
///
/// # Examples
///
/// ```
/// use lash_types::text::truncate_with_ellipsis;
///
/// assert_eq!(truncate_with_ellipsis("short", 10), "short");
/// assert_eq!(truncate_with_ellipsis("Hello, World!", 10), "Hello, ...");
/// assert_eq!(truncate_with_ellipsis("Hello", 2), "...");
///
/// // A multi-byte char straddling the cut point is dropped, not split.
/// let s = "aaaa—bbbb"; // '—' occupies bytes 4..7
/// assert_eq!(truncate_with_ellipsis(s, 8), "aaaa...");
/// ```
#[must_use]
pub fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        format!("{}...", &s[..floor_char_boundary(s, max_len - 3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_boundary_on_ascii_is_identity() {
        let s = "abcdef";
        for i in 0..=s.len() {
            assert_eq!(floor_char_boundary(s, i), i);
        }
    }

    #[test]
    fn floor_boundary_walks_back_inside_multibyte_char() {
        let s = "a—b"; // bytes: a=0, — =1..4, b=4
        assert_eq!(floor_char_boundary(s, 1), 1);
        assert_eq!(floor_char_boundary(s, 2), 1);
        assert_eq!(floor_char_boundary(s, 3), 1);
        assert_eq!(floor_char_boundary(s, 4), 4);
    }

    #[test]
    fn floor_boundary_clamps_past_end() {
        assert_eq!(floor_char_boundary("abc", 100), 3);
        assert_eq!(floor_char_boundary("", 5), 0);
    }

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate_with_ellipsis("hi", 10), "hi");
        assert_eq!(truncate_with_ellipsis("exact", 5), "exact");
    }

    #[test]
    fn truncate_tiny_budget_returns_bare_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello", 3), "...");
        assert_eq!(truncate_with_ellipsis("hello", 0), "...");
    }

    #[test]
    fn truncate_never_splits_multibyte_chars() {
        // '—' is 3 bytes; place it so every cut point from 4..=9 lands
        // somewhere interesting and none of them panic.
        let s = "aaaa—bbbb";
        for max_len in 4..=s.len() + 3 {
            let out = truncate_with_ellipsis(s, max_len);
            assert!(out.len() <= max_len.max(3));
            assert!(out.is_char_boundary(out.len()));
        }
    }

    #[test]
    fn truncate_handles_emoji_and_cjk() {
        let s = "日本語のテキストです、長いので切り詰められます";
        for max_len in 0..=s.len() + 3 {
            let _ = truncate_with_ellipsis(s, max_len); // must not panic
        }
        let e = "🦀🦀🦀🦀🦀";
        for max_len in 0..=e.len() + 3 {
            let _ = truncate_with_ellipsis(e, max_len);
        }
    }
}

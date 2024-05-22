use std::borrow::Cow;

fn is_ascii_lowercase_alphanumeric(c: char) -> bool {
    matches!(c, 'a'..='z' | '0'..='9')
}

fn is_ascii_lowercase_alphanumeric_or_dash(c: char) -> bool {
    matches!(c, 'a'..='z' | '0'..='9' | '-')
}

fn is_rfc_1123_label(value: &str) -> bool {
    let mut chars = value.chars().peekable();

    let Some(first) = chars.next() else {
        return false;
    };

    if !is_ascii_lowercase_alphanumeric(first) {
        return false;
    }

    while let Some(c) = chars.next() {
        let is_valid = if chars.peek().is_some() {
            // Not the last character.
            is_ascii_lowercase_alphanumeric_or_dash
        } else {
            // The last character.
            is_ascii_lowercase_alphanumeric
        };

        if !is_valid(c) {
            return false;
        }
    }

    true
}

/// Returns the indices of the first range of alphanumeric characters in the input.
fn alphanumeric_range(input: &str) -> Option<std::ops::RangeInclusive<usize>> {
    // Using `chars().enumerate()` rather than `.char_indices()` because I don't see a safe way to
    // reconstitute an iterator from the byte offsets. We'll just iterate again using char indices.
    let mut chars = input.chars().enumerate();

    let start = loop {
        let (i, c) = chars.next()?;
        if is_ascii_lowercase_alphanumeric(c) {
            break i;
        }
    };

    let mut end = start;
    for (i, c) in chars {
        if is_ascii_lowercase_alphanumeric(c) {
            end = i;
        }
    }

    Some(start..=end)
}

/// Attempts to lossily convert an input string into a string that adheres to the regex
/// `^[a-z0-9]([-a-z0-9]*[a-z0-9])?$`. Returns `None` if there are not enough alphanumeric
/// characters to construct a non-empty string.
pub fn to_rfc_1123_label_lossy(input: &str) -> Option<Cow<str>> {
    if is_rfc_1123_label(input) {
        return Some(Cow::Borrowed(input));
    }

    let range = alphanumeric_range(input)?;

    fn range_len(r: &std::ops::RangeInclusive<usize>) -> usize {
        r.end()
            .checked_add(1)
            .expect("overflow")
            .checked_sub(*r.start())
            .expect("underflow")
    }

    let mut output = String::with_capacity(range_len(&range));

    let chars = input.chars().skip(*range.start()).take(range_len(&range));

    let mut can_append_dash = false;
    for c in chars {
        let to_push = if is_ascii_lowercase_alphanumeric_or_dash(c) {
            Some(c)
        } else if can_append_dash {
            Some('-')
        } else {
            None
        };

        if let Some(c) = to_push {
            can_append_dash = c != '-';
            output.push(c);
        }
    }

    debug_assert!(is_rfc_1123_label(&output));

    Some(Cow::Owned(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_rfc_1123_label_lossy_works() {
        assert_eq!(to_rfc_1123_label_lossy(""), None);
        assert_eq!(to_rfc_1123_label_lossy("-"), None);
        assert_eq!(to_rfc_1123_label_lossy("."), None);
        assert_eq!(to_rfc_1123_label_lossy("X"), None);
        assert_eq!(to_rfc_1123_label_lossy("-.X"), None);
        assert_eq!(to_rfc_1123_label_lossy("a"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1123_label_lossy("a-"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1123_label_lossy("-a"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1123_label_lossy("-a-"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1123_label_lossy("--a"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1123_label_lossy("a--"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1123_label_lossy("--a-"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1123_label_lossy("-a--"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1123_label_lossy("a."), Some(Cow::Borrowed("a")));
        assert_eq!(
            to_rfc_1123_label_lossy("a.c"),
            Some(Cow::Owned("a-c".to_string()))
        );
        assert_eq!(
            to_rfc_1123_label_lossy("-a.c."),
            Some(Cow::Owned("a-c".to_string()))
        );
    }
}

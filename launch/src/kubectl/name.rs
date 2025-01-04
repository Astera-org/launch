use std::borrow::Cow;

fn is_ascii_lowercase_numeric(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'0'..=b'9')
}

fn is_ascii_lowercase_numeric_or_dash(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'-')
}

/// Returns true if the input matches the regex `^[a-z]([-a-z0-9]*[a-z0-9])?$`, see
/// https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#rfc-1035-label-names.
pub fn is_rfc_1035_label(value: &(impl AsRef<[u8]> + ?Sized)) -> bool {
    fn inner(value: &[u8]) -> bool {
        match value.len() {
            0 => false,
            1 => value[0].is_ascii_lowercase(),
            _ => {
                value[0].is_ascii_lowercase()
                    && value[1..value.len() - 1]
                        .iter()
                        .copied()
                        .all(is_ascii_lowercase_numeric_or_dash)
                    && is_ascii_lowercase_numeric(value[value.len() - 1])
            }
        }
    }
    inner(value.as_ref())
}

/// Attempts to lossily convert an input into a string that adheres to the regex
/// `^[a-z]([-a-z0-9]*[a-z0-9])?$`. Returns `None` if there are not enough alphanumeric characters
/// to construct a non-empty string. See
/// https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#rfc-1035-label-names.
pub fn to_rfc_1035_label_lossy(input: &(impl AsRef<[u8]> + ?Sized)) -> Option<Cow<str>> {
    fn inner(input: &[u8]) -> Option<Cow<str>> {
        let start = input.iter().enumerate().find_map(|(index, &byte)| {
            if byte.is_ascii_lowercase() {
                Some(index)
            } else {
                None
            }
        })?;

        // We can use `wrapping_add(1)` since found indices are less than `usize::MAX`.
        let end = input
            .iter()
            .enumerate()
            .skip(start.wrapping_add(1))
            .rev()
            .find_map(|(index, &byte)| {
                if is_ascii_lowercase_numeric(byte) {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(start)
            .wrapping_add(1);

        if is_rfc_1035_label(&input[start..end]) {
            // SAFETY: is_rfc_1035_label guarantees that all bytes are ASCII.
            return Some(Cow::Borrowed(unsafe {
                std::str::from_utf8_unchecked(&input[start..end])
            }));
        }

        // We can use `wrapping_sub` because `start < end`.
        let mut output = Vec::with_capacity(end.wrapping_sub(start));

        output.push(input[start]);

        let mut can_append_dash = true;
        for &byte in &input[start.wrapping_add(1)..end.wrapping_sub(1)] {
            let to_push = if is_ascii_lowercase_numeric_or_dash(byte) {
                Some(byte)
            } else if can_append_dash {
                Some(b'-')
            } else {
                None
            };

            if let Some(c) = to_push {
                can_append_dash = c != b'-';
                output.push(c);
            }
        }

        output.push(input[end.wrapping_sub(1)]);

        debug_assert!(is_rfc_1035_label(&output));

        // SAFETY: All bytes are valid ASCII.
        Some(Cow::Owned(unsafe { String::from_utf8_unchecked(output) }))
    }
    inner(input.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_rfc_1035_label_lossy_works() {
        assert_eq!(to_rfc_1035_label_lossy(""), None);
        assert_eq!(to_rfc_1035_label_lossy("-"), None);
        assert_eq!(to_rfc_1035_label_lossy("."), None);
        assert_eq!(to_rfc_1035_label_lossy("X"), None);
        assert_eq!(to_rfc_1035_label_lossy("1"), None);
        assert_eq!(to_rfc_1035_label_lossy("-.X"), None);
        assert_eq!(to_rfc_1035_label_lossy("a"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1035_label_lossy("a-"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1035_label_lossy("a1"), Some(Cow::Borrowed("a1")));
        assert_eq!(to_rfc_1035_label_lossy("-a"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1035_label_lossy("-a-"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1035_label_lossy("--a"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1035_label_lossy("a--"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1035_label_lossy("--a-"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1035_label_lossy("-aXa-"), Some(Cow::Borrowed("a-a")));
        assert_eq!(to_rfc_1035_label_lossy("-a--"), Some(Cow::Borrowed("a")));
        assert_eq!(to_rfc_1035_label_lossy("a."), Some(Cow::Borrowed("a")));
        assert_eq!(
            to_rfc_1035_label_lossy("a.c"),
            Some(Cow::Owned("a-c".to_string()))
        );
        assert_eq!(
            to_rfc_1035_label_lossy("-a.c."),
            Some(Cow::Owned("a-c".to_string()))
        );
    }
}

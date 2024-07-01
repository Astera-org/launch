//! This module based on the bash module of https://github.com/allenap/shell-quote. This implementation is different in
//! that it has been simplified, and the API has been modified to work with strings and iterators instead of vecs and
//! slices.

use std::borrow::Cow;

/// Quotes each argument and joins them with spaces.
pub fn quote_join<'a, I: IntoIterator<Item = &'a str>>(args: I) -> String {
    let mut out = Default::default();
    quote_join_into(&mut out, args);
    out
}

/// Appends each quoted argument, separated by spaces. If out is non-empty, starts by adding a space before the first arg.
pub fn quote_join_into<'a, I: IntoIterator<Item = &'a str>>(out: &mut String, args: I) {
    for arg in args.into_iter() {
        quote_join_one_into(out, arg);
    }
}

/// Appends a space if `out` is non-empty and the argument which is ANSI-C quoted if necessary.
fn quote_join_one_into(out: &mut String, arg: &str) {
    let stat = arg_encoding_info(arg);

    let additional = if out.is_empty() { 0 } else { 1 } + stat.encoded_len;
    out.reserve(additional);

    let initial_len = out.len();

    // Append a separator if necessary.
    if !out.is_empty() {
        out.push(' ');
    };

    match stat.encoding {
        Encoding::Empty => out.push_str("''"),
        Encoding::Verbatim => out.push_str(arg),
        Encoding::AnsiC => encode_ansi_c(out, arg),
    }

    debug_assert_bytes_written(out.len() - initial_len, additional, arg);
}

/// Encodes a string through [ANSI-C
/// quoting](https://www.gnu.org/software/bash/manual/html_node/ANSI_002dC-Quoting.html) if necessary.
#[allow(unused)] // Useful to have, would be part of public API if this was a crate.
fn quote(arg: &str) -> Cow<str> {
    let stat = arg_encoding_info(arg);

    match stat.encoding {
        Encoding::Empty => Cow::Borrowed("''"),
        Encoding::Verbatim => Cow::Borrowed(arg),
        Encoding::AnsiC => {
            let mut out = String::with_capacity(stat.encoded_len);
            encode_ansi_c(&mut out, arg);
            debug_assert_bytes_written(out.len(), stat.encoded_len, arg);
            Cow::Owned(out)
        }
    }
}

fn debug_assert_bytes_written(actual: usize, expected: usize, arg: &str) {
    debug_assert_eq!(
        actual, expected,
        "wrote {actual} bytes instead of the expected {expected} bytes when encoding {arg:?}",
    );
}

// Returns the `\xHH` encoding
fn encode_x(b: u8) -> [u8; 4] {
    [b'\\', b'x', hex_half(b >> 4), hex_half(b)]
}

fn encode_ansi_c(out: &mut String, arg: &str) {
    // SAFETY: We only write valid UTF-8 into out.
    unsafe {
        let out = out.as_mut_vec();

        let start = out.len();

        out.extend_from_slice(b"$'");

        let mut run = None;

        let finish_run_if_needed =
            |run: &mut Option<usize>, out: &mut Vec<u8>, arg: &str, end: usize| {
                if let Some(start) = *run {
                    out.extend_from_slice(&arg.as_bytes()[start..end]);
                    *run = None;
                }
            };

        let start_run_if_needed = |run: &mut Option<usize>, start| {
            if run.is_none() {
                *run = Some(start);
            }
        };

        for (i, b) in arg.bytes().enumerate() {
            enum Action {
                Push2([u8; 2]),
                Push4([u8; 4]),
                Run,
            }

            let action = match kind(b) {
                Kind::Bell => Action::Push2(*br"\a"),
                Kind::Backspace => Action::Push2(*br"\b"),
                Kind::Escape => Action::Push2(*br"\e"),
                Kind::FormFeed => Action::Push2(*br"\f"),
                Kind::NewLine => Action::Push2(*br"\n"),
                Kind::CarriageReturn => Action::Push2(*br"\r"),
                Kind::HorizontalTab => Action::Push2(*br"\t"),
                Kind::VerticalTab => Action::Push2(*br"\v"),
                Kind::EscapeX => Action::Push4(encode_x(b)),
                Kind::Backslash => Action::Push2(*br"\\"),
                Kind::SingleQuote => Action::Push2(*br"\'"),
                Kind::VerbatimAsciiInert | Kind::VerbatimAscii | Kind::VerbatimUtf8 => Action::Run,
            };

            match match &action {
                Action::Push2(bytes) => Some(&bytes[..]),
                Action::Push4(bytes) => Some(&bytes[..]),
                Action::Run => None,
            } {
                Some(bytes) => {
                    finish_run_if_needed(&mut run, out, arg, i);
                    out.extend_from_slice(bytes);
                }
                None => {
                    start_run_if_needed(&mut run, i);
                }
            }
        }

        finish_run_if_needed(&mut run, out, arg, arg.len());

        out.push(b'\'');

        debug_assert!(std::str::from_utf8(&out[start..]).is_ok());
    }
}

enum Encoding {
    /// The input string was empty.
    Empty,
    /// The input string should be encoded as-is.
    Verbatim,
    /// The input string should be ANSI-C quoted.
    AnsiC,
}

/// Captures encoding information about an &str.
struct EncodingInfo {
    encoded_len: usize,
    encoding: Encoding,
}

impl EncodingInfo {
    fn empty() -> Self {
        Self {
            encoded_len: 2, // For `''`
            encoding: Encoding::Empty,
        }
    }

    fn verbatim(encoded_len: usize) -> Self {
        Self {
            encoded_len,
            encoding: Encoding::Verbatim,
        }
    }

    fn ansi_c(encoded_len: usize) -> Self {
        Self {
            encoded_len,
            encoding: Encoding::AnsiC,
        }
    }
}

fn arg_encoding_info(arg: &str) -> EncodingInfo {
    if arg.is_empty() {
        EncodingInfo::empty()
    } else {
        struct Acc {
            all_inert: bool,
            encoded_len: usize,
        }

        let acc = arg.bytes().fold(
            Acc {
                all_inert: true,
                encoded_len: 0,
            },
            |mut acc, b| {
                let k = kind(b);
                acc.all_inert &= k.is_inert();
                acc.encoded_len += k.escaped_len();
                acc
            },
        );

        if acc.all_inert {
            debug_assert_eq!(acc.encoded_len, arg.len());
            EncodingInfo::verbatim(acc.encoded_len)
        } else {
            EncodingInfo::ansi_c(acc.encoded_len + 3) // + 3 for `$''`.
        }
    }
}

#[derive(Clone, Copy)]
enum Kind {
    /// Escaped as `\a`
    Bell,
    /// Escaped as `\b`
    Backspace,
    /// Escaped as `\e`
    Escape,
    /// Escaped as `\f`
    FormFeed,
    /// Escaped as `\n`
    NewLine,
    /// Escaped as `\r`
    CarriageReturn,
    /// Escaped as `\t`
    HorizontalTab,
    /// Escaped as `\v`
    VerticalTab,
    /// Escaped as `\\`
    Backslash,
    /// Escaped as `\'`
    SingleQuote,
    /// Escaped as `\xHH`
    EscapeX,
    /// Does not require escaping
    VerbatimAsciiInert,
    /// Does not require escaping if present inside an ANSI-C Quoted string `$'...'`
    VerbatimAscii,
    /// UTF-8 sequence that does not require escaping
    VerbatimUtf8,
}

impl Kind {
    fn is_inert(self) -> bool {
        matches!(self, Self::VerbatimAsciiInert)
    }

    fn escaped_len(self) -> usize {
        match self {
            Kind::Bell => 2,
            Kind::Backspace => 2,
            Kind::Escape => 2,
            Kind::FormFeed => 2,
            Kind::NewLine => 2,
            Kind::CarriageReturn => 2,
            Kind::HorizontalTab => 2,
            Kind::VerticalTab => 2,
            Kind::Backslash => 2,
            Kind::SingleQuote => 2,
            Kind::EscapeX => 4,
            Kind::VerbatimAsciiInert => 1,
            Kind::VerbatimAscii => 1,
            Kind::VerbatimUtf8 => 1,
        }
    }
}

fn kind(b: u8) -> Kind {
    match b {
        0x07 => Kind::Bell,
        0x08 => Kind::Backspace,
        b'\t' => Kind::HorizontalTab,
        0x1b => Kind::Escape,
        0x0c => Kind::FormFeed,
        b'\n' => Kind::NewLine,
        b'\r' => Kind::CarriageReturn,
        0x0b => Kind::VerticalTab,
        b'\\' => Kind::Backslash,
        b'\'' => Kind::SingleQuote,

        // ASCII printable letters, numbers, and "safe" punctuation.
        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b',' | b'.' | b'/' | b'_' | b'-' => {
            Kind::VerbatimAsciiInert
        }

        // ASCII punctuation which can have significance in the shell.
        b'|' | b'&' | b';' | b'(' | b')' | b'<' | b'>' | b' ' | b'?' | b'[' | b']' | b'{'
        | b'}' | b'`' | b'~' | b'!' | b'$' | b'@' | b'+' | b'=' | b'*' | b'%' | b'#' | b':'
        | b'^' | b'"' => Kind::VerbatimAscii,

        // ASCII control characters and delete.
        0x00..=0x06 | 0x0e..=0x1a | 0x1c..=0x1f | 0x7f => Kind::EscapeX,

        // ASCII extended characters, or high bytes.
        0x80.. => Kind::VerbatimUtf8,
    }
}

/// Calls `hex_half_unchecked(b & 15)`.
const fn hex_half(b: u8) -> u8 {
    // SAFETY: forces b into the range `0..=15`.
    unsafe { hex_half_unchecked(b & 15) }
}

/// SAFETY: half must be in the range `0..=15`.
const unsafe fn hex_half_unchecked(h: u8) -> u8 {
    debug_assert!(h < 16, "x should always be between 0 and 15");
    match h {
        0..=9 => b'0' + h,
        10..=15 => b'A' + (h - 10),
        _ => std::hint::unreachable_unchecked(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lowercase_ascii() {
        assert_eq!(
            quote_join(["abcdefghijklmnopqrstuvwxyz"]),
            "abcdefghijklmnopqrstuvwxyz"
        );
    }

    #[test]
    fn test_uppercase_ascii() {
        assert_eq!(
            quote_join(["ABCDEFGHIJKLMNOPQRSTUVWXYZ"]),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        );
    }

    #[test]
    fn test_numbers() {
        assert_eq!(quote_join(["0123456789"]), "0123456789");
    }

    #[test]
    fn test_punctuation() {
        assert_eq!(quote_join(["-_=/,.+"]), "$'-_=/,.+'");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(quote_join([""]), "''");
    }

    #[test]
    fn test_basic_escapes() {
        assert_eq!(quote_join([r#"woo"wah""#]), r#"$'woo"wah"'"#);
    }

    #[test]
    fn test_control_characters() {
        assert_eq!(quote_join(["\x00"]), "$'\\x00'");
        assert_eq!(quote_join(["\x07"]), "$'\\a'");
        assert_eq!(quote_join(["\x00"]), "$'\\x00'");
        assert_eq!(quote_join(["\x06"]), "$'\\x06'");
        assert_eq!(quote_join(["\x7F"]), "$'\\x7F'");
    }

    #[test]
    fn test_multiple_args() {
        assert_eq!(quote_join(["echo", "-n", "$PATH"]), "echo -n $'$PATH'")
    }

    #[test]
    #[ignore = "requires the bash command to be available, run manually"]
    fn test_roundtrip() {
        let ascii_bytes = String::from_utf8((0x01..=0x7f).collect()).unwrap();
        let script = quote_join(["echo", "-n", ascii_bytes.as_str()]);
        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(&script)
            .output()
            .unwrap();
        assert_eq!(output.stdout, ascii_bytes.as_bytes());
    }
}

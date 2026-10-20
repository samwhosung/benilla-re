//! Storm's `SStrTokenize` (`0x64ae50`), the tokenizer the console commands, the CVar commands and
//! the file readers split their text with (`systems/storm/strings.md`).

/// What one call leaves: the bytes stored to the output buffer, the cursor, the quote flag.
#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    /// The stored bytes, without the NUL: at most `size − 1` of the token's bytes. The NUL
    /// follows them when `size` is above 0.
    pub text: Vec<u8>,
    /// The cursor after the call, as an offset into the input.
    pub next: usize,
    /// The value stored through the fifth argument: set when a `"` opened or closed in this call.
    pub quoted: bool,
}

/// `SStrTokenize` (`0x64ae50`) on a string cursor, for non-null arguments: the cursor's string
/// `input` from its offset 0 (the bytes before its first NUL), the delimiter string `delims` and
/// the output size `size` (0 for the count-only call that takes a null output).
///
/// Leading delimiters are skipped, and a leading `"` that is one of the delimiters opens a quote
/// and is skipped. The token then runs to the first delimiter, which is consumed, or to the end
/// of the string. When `"` is a delimiter, it opens a quote at the token's start (stored count 0),
/// ends an unquoted token after stored bytes without being consumed, and closes an open quote,
/// ending the token with the quote consumed; inside a quote every byte, delimiters included, is
/// token text. A byte is stored while the stored count plus 1 is below `size`, so the token is cut
/// at `size − 1` bytes and the rest is skipped. Null arguments set error 0x57 and return with
/// nothing written.
pub fn str_tokenize(input: &[u8], delims: &[u8], size: usize) -> Token {
    let input = &input[..input.iter().position(|&b| b == 0).unwrap_or(input.len())];
    let delims = &delims[..delims.iter().position(|&b| b == 0).unwrap_or(delims.len())];
    let at = |i: usize| input.get(i).copied().unwrap_or(0);
    let is_delim = |b: u8| delims.contains(&b);
    let quote_is_delim = is_delim(b'"');

    let mut i = 0;
    let mut in_quote = false;
    let mut quoted = false;
    while at(i) != 0 && is_delim(at(i)) {
        i += 1;
        if quote_is_delim && at(i - 1) == b'"' {
            in_quote = true;
            quoted = true;
            break;
        }
    }

    let mut text = Vec::new();
    while at(i) != 0 {
        let b = at(i);
        if quote_is_delim && b == b'"' {
            if !text.is_empty() && !in_quote {
                break;
            }
            in_quote = !in_quote;
            quoted = true;
            i += 1;
            if !in_quote {
                break;
            }
            continue;
        }
        if !in_quote && is_delim(b) {
            i += 1;
            break;
        }
        if text.len() + 1 < size {
            text.push(b);
        }
        i += 1;
    }
    Token {
        text,
        next: i,
        quoted,
    }
}

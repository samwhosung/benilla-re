//! The add-on URL file's text (`systems/ui/addons.md`, "Reading the URL file"): `0x51d500`.

/// What `0x51d500` stores for the bytes `SFile::Load` returns, a NUL after them: the text after
/// the first `=` of the file read as a C string, without its trailing CR, LF and spaces. `None`
/// leaves the out slot as it was: no `=` before the first NUL.
pub fn url_text(file: &[u8]) -> Option<Vec<u8>> {
    let text = &file[..file.iter().position(|&c| c == 0).unwrap_or(file.len())];
    let eq = text.iter().position(|&c| c == b'=')?;
    let mut url = text[eq + 1..].to_vec();
    while matches!(url.last(), Some(0x0d | 0x0a | 0x20)) {
        url.pop();
    }
    Some(url)
}

//! The shared font methods `SetJustifyH` `0x79fc20`, `SetJustifyV` `0x79fce0` and `GetJustifyV`
//! `0x79fd70` (`systems/ui/fontmethods.md#justify-and-the-followed-font`).

/// The six justify names and their bits, in the order `0x6f1990` and `0x6f1a00` walk them.
pub const JUSTIFY_NAMES: [(u32, &str); 6] = [
    (0x01, "LEFT"),
    (0x02, "CENTER"),
    (0x04, "RIGHT"),
    (0x08, "TOP"),
    (0x10, "MIDDLE"),
    (0x20, "BOTTOM"),
];

/// The horizontal axis of the justify word.
pub const H_MASK: u32 = 0x07;
/// The vertical axis of the justify word.
pub const V_MASK: u32 = 0x38;

/// `0x6f1990`: the bit of a justify name, ignoring ASCII case.
pub fn parse_justify(name: &[u8]) -> Option<u32> {
    JUSTIFY_NAMES
        .iter()
        .find(|(_, n)| n.as_bytes().eq_ignore_ascii_case(name))
        .map(|&(bit, _)| bit)
}

/// `0x6f1a00`: the name of the first bit of `bits` in table order, else `UNKNOWN`.
pub fn justify_name(bits: u32) -> &'static str {
    JUSTIFY_NAMES
        .iter()
        .find(|(bit, _)| bits & bit != 0)
        .map_or("UNKNOWN", |&(_, n)| n)
}

/// A font's justify state: the justify word `+0x54`, the block's set and changed word `+0x38`
/// and the follow mask `+0x2c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JustifyState {
    pub word: u32,
    pub valid: u32,
    pub follow: u32,
}

/// A setter with a parsed `bit` on the axis `mask`: the axis takes the bit's part of it; an
/// unchanged word with the justify group set (0x200) is not stored, else it is stored and 0x202
/// set; the follow mask always loses bit 1. The setter then pushes the changes.
pub fn set_justify(s: JustifyState, bit: u32, mask: u32) -> JustifyState {
    let word = s.word ^ ((s.word ^ bit) & mask);
    let mut out = s;
    if word != s.word || s.valid & 0x200 == 0 {
        out.word = word;
        out.valid |= 0x202;
    }
    out.follow &= !0x2;
    out
}

//! The Mac client's key tables (`systems/core/mackeys.md`), read from the i386 slice of the
//! 1.12.1 (5875) `World of Warcraft.app`. Addresses are the slice's, written `mac:` and six hex
//! digits (`symbols/README.md`, The Mac client).

/// `mac:5bf320`, the fixed key table (`systems/core/mackeys.md#the-fixed-key-table`): the client
/// key code for each Carbon virtual keycode 0 to 0x7f, and −1 for a slot the keyboard layout fills
/// (`#filling-the-table-from-the-layout`). The handler copies it over the live table `mac:991d20`
/// whenever the layout changes.
pub const FIXED_KEY_CODES: [i32; 128] = [
    -1, -1, -1, -1, -1, -1, -1, -1, // 0x00
    -1, -1, -1, -1, -1, -1, -1, -1, // 0x08
    -1, -1, 0x031, 0x032, 0x033, 0x034, 0x036, 0x035, // 0x10
    -1, 0x039, 0x037, -1, 0x038, 0x030, -1, -1, // 0x18
    -1, -1, -1, -1, 0x201, -1, -1, -1, // 0x20
    -1, -1, -1, -1, -1, -1, -1, -1, // 0x28
    0x203, 0x020, -1, 0x202, -1, 0x200, -1, -1, // 0x30
    0x000, 0x20e, 0x002, 0x001, 0x000, 0x002, 0x001, -1, // 0x38
    -1, 0x10e, -1, 0x10c, -1, 0x10a, -1, 0x20f, // 0x40
    -1, -1, -1, 0x10d, 0x201, -1, 0x10b, -1, // 0x48
    -1, 0x30c, 0x100, 0x101, 0x102, 0x103, 0x104, 0x105, // 0x50
    0x106, 0x107, -1, 0x108, 0x109, -1, -1, -1, // 0x58
    0x304, 0x305, 0x306, 0x302, 0x307, 0x308, -1, 0x30a, // 0x60
    -1, 0x212, 0x30d, 0x210, -1, 0x309, -1, 0x30b, // 0x68
    -1, 0x211, 0x208, 0x20a, 0x20c, 0x209, 0x303, 0x20b, // 0x70
    0x301, 0x20d, 0x300, 0x204, 0x206, 0x207, 0x205, -1, // 0x78
];

/// `mac:693d60`, the Edit menu's commands and the letters they press with Control
/// (`systems/core/mackeys.md#edit-menu-commands`): each command ID, four ASCII characters read as a
/// big-endian word, with the key code of its letter.
pub const EDIT_COMMAND_KEYS: [(u32, u32); 5] = [
    (u32::from_be_bytes(*b"undo"), 0x5a),
    (u32::from_be_bytes(*b"cut "), 0x58),
    (u32::from_be_bytes(*b"copy"), 0x43),
    (u32::from_be_bytes(*b"past"), 0x56),
    (u32::from_be_bytes(*b"sall"), 0x41),
];

/// `mac:699fc8`, the chord builder's modifier prefixes (`systems/core/mackeys.md#key-names-on-the-mac`):
/// each modifier code with its prefix, walked from the last entry to the first.
pub const CHORD_PREFIXES: [(u32, &str); 3] = [(0, "SHIFT-"), (1, "CTRL-"), (2, "ALT-")];

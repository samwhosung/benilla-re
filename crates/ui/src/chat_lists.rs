//! Two chat computations (`systems/ui/chatcolors.md`, `systems/ui/chatchannels.md`): the colour
//! word `0x49e990` packs from a chat type's registry entry, and the lines `0x49c7e0` builds from a
//! channel's member list.

/// The live chat colour registry, `0xb4e518`: 94 entries of 0x43 bytes, a 64-byte name and then
/// red, green and blue.
pub const REGISTRY: u32 = 0xb4_e518;
/// The bytes of one registry entry.
pub const ENTRY_SIZE: u32 = 0x43;
/// The first type `0x49e990` answers white for.
pub const TYPE_COUNT: i32 = 94;

/// The address of the red byte `0x49e990` reads for `ty`, in 32 bits, or `None` for a type of 94
/// or more, which gets `0xffffffff`.
pub fn colour_address(ty: i32) -> Option<u32> {
    (ty < TYPE_COUNT).then(|| (REGISTRY + 0x40).wrapping_add((ty as u32).wrapping_mul(ENTRY_SIZE)))
}

/// `0xffRRGGBB` from the three bytes.
pub fn pack_colour(rgb: [u8; 3]) -> u32 {
    0xff00_0000 | u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2])
}

/// One member of a channel list as the printer reads it: the name record's two strings (`+0`
/// and `+0x30`) and the member flag byte.
#[derive(Clone, Debug)]
pub struct Member {
    pub name: Vec<u8>,
    pub second: Vec<u8>,
    pub flags: u8,
}

/// The name buffer of `0x49c7e0`: 0x130 bytes, which the copy, the `-` and the filtered second
/// string fill no further.
const NAME_ROOM: usize = 0x130;
/// `SStrPrintf(entry, 0x33, ...)`: at most 50 characters.
const ENTRY_ROOM: usize = 0x32;
/// A line is shown before an entry that brings it to this many bytes.
const LINE_LIMIT: usize = 0x100;

fn member_name(m: &Member) -> Vec<u8> {
    let mut s: Vec<u8> = m.name.iter().copied().take(NAME_ROOM).collect();
    if !m.second.is_empty() {
        if s.len() < NAME_ROOM {
            s.push(b'-');
        }
        for &c in &m.second {
            if s.len() >= NAME_ROOM {
                break;
            }
            if c != b' ' && c != b'\t' {
                s.push(c);
            }
        }
    }
    s
}

/// The lines `0x49c7e0` shows for a list whose every name is cached, in order.
pub fn channel_list_lines(channel_flags: u8, members: &[Member]) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    let mut line: Vec<u8> = Vec::new();
    let mut entries = 0u32;
    for m in members {
        let prefix: &[u8] = if m.flags & 0x01 != 0 {
            b"*"
        } else if m.flags & 0x02 != 0 {
            b"@"
        } else if channel_flags & 0x02 != 0 && m.flags & 0x04 == 0 {
            b"#"
        } else {
            b""
        };
        let separator: &[u8] = if line.is_empty() { b"" } else { b", " };
        let mut entry = [separator, prefix, &member_name(m)].concat();
        entry.truncate(ENTRY_ROOM);
        if entry.len() + line.len() >= LINE_LIMIT {
            lines.push(std::mem::take(&mut line));
            if entry.first() == Some(&b',') {
                entry.drain(..2.min(entry.len()));
            }
            line = entry;
            entries = 1;
        } else {
            line.extend_from_slice(&entry);
            entries += 1;
        }
    }
    if entries != 0 {
        lines.push(line);
    }
    lines
}

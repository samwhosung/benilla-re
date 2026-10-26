//! The marking half of the client's Lua 5.0 collector (`lgc.c`): a table's traversal with its weak
//! modes, the stack shrink a thread's traversal ends with, and the clearing of weak tables.

/// `valismarked` (`0x6f79f0`) on a value of tag `tag` whose object's mark byte is `marked`: a
/// string (tag 4) is marked first, bit 0 of its byte set; a value of a tag below 4 (nil, boolean,
/// light userdata, number, and `LUA_TNONE`, -1) counts as marked. Returns the answer and the byte.
#[must_use]
pub fn valismarked(tag: i32, marked: u8) -> (bool, u8) {
    let marked = if tag == 4 { marked | 1 } else { marked };
    (tag < 4 || marked & 1 != 0, marked)
}

/// A value as a table holds it: the tag, the context word at `+4` and the collectable object's
/// mark byte (meaningless for a tag below 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub tag: i32,
    pub context: u32,
    pub marked: u8,
}

/// `cleartablekeys` (`0x6f7970`) on one table's nodes, last to first: a key `valismarked` refuses
/// has its node removed (`removekey`, `0x6f79d0`): the value made nil with the current context
/// word, and a collectable key retagged -1. `valismarked` marks each string key on the way.
pub fn clear_table_keys(nodes: &mut [(Slot, Slot)], context: u32) {
    for (key, val) in nodes.iter_mut().rev() {
        let (keep, m) = valismarked(key.tag, key.marked);
        key.marked = m;
        if !keep {
            val.tag = 0;
            val.context = context;
            if key.tag >= 4 {
                key.tag = -1;
            }
        }
    }
}

/// The resizes `checkstacksizes` (`0x6f7900`) asks for: `luaD_reallocCI(L, size_ci / 2)` when the
/// call records in use, `ci_bytes / 24` truncated, times 4 is below `size_ci` (a 16-bit field)
/// and `size_ci` is above 16; then `luaD_reallocstack(L, stacksize / 2)`, by a jump, when the slots
/// up to `max`, `stack_bytes >> 4`, times 4 are below `stacksize` and `stacksize` is above 90. The
/// products and compares are signed 32-bit, the call records' halving unsigned.
#[must_use]
pub fn checkstacksizes(
    ci_bytes: i32,
    size_ci: u16,
    stack_bytes: i32,
    stacksize: i32,
) -> (Option<u32>, Option<i32>) {
    let used_ci = ci_bytes / 24;
    let ci = (used_ci.wrapping_shl(2) < i32::from(size_ci) && size_ci > 16)
        .then_some(u32::from(size_ci) >> 1);
    let used = stack_bytes >> 4;
    let stack = (used.wrapping_shl(2) < stacksize && stacksize > 90).then_some(stacksize / 2);
    (ci, stack)
}

/// What `traversetable` (`0x6f7590`) found the table's weak mode to be and did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Traversal {
    /// Whether the metatable was marked (it was unmarked).
    pub marked_metatable: bool,
    /// Whether `luaT_gettm` was asked for `__mode` (the metatable's flag bit 3 clear).
    pub asked_mode: bool,
    /// The table's mark byte after: bits 1 (weak keys) and 2 (weak values) rewritten when the mode
    /// names either.
    pub marked: u8,
    /// The list the table joined: 1 weak keys (`+4`), 2 weak values (`+8`), 3 both (`+0xc`).
    pub list: Option<u8>,
    /// The array slots marked, by index, in the order marked.
    pub array: Vec<usize>,
    /// The node keys and values marked, `(node, is_value)`, in the order marked.
    pub nodes: Vec<(usize, bool)>,
}

/// `traversetable` on a table with mark byte `marked`, a metatable with mark byte `mt_marked` and
/// flags `mt_flags`, a `__mode` value of tag `mode_tag` and text `mode` (`None` for no value), its
/// array and its node pairs. The metatable is marked unless bit 0 or 4 of its byte is set. With
/// flag bit 3 clear the mode is looked up; a string mode holding `k` or `v` rewrites bits 1 and 2
/// and links the table to its weak list. The array is marked unless values are weak, last slot
/// first; then each node, last first, whose value is not nil: its key unless keys are weak, then
/// its value unless values are weak. Only an unmarked collectable (tag 4 or more, bits 0 and 4 of
/// its byte clear) is marked.
#[must_use]
pub fn traversetable(
    marked: u8,
    mt_marked: u8,
    mt_flags: u8,
    mode: Option<(i32, &[u8])>,
    array: &[Slot],
    nodes: &[(Slot, Slot)],
) -> Traversal {
    let markable = |s: &Slot| s.tag >= 4 && s.marked & 0x11 == 0;
    let mut t = Traversal {
        marked_metatable: mt_marked & 0x11 == 0,
        asked_mode: mt_flags & 8 == 0,
        marked,
        list: None,
        array: Vec::new(),
        nodes: Vec::new(),
    };
    let (mut wk, mut wv) = (false, false);
    if t.asked_mode {
        if let Some((4, text)) = mode {
            let text = text.split(|&b| b == 0).next().unwrap_or_default();
            wk = text.contains(&b'k');
            wv = text.contains(&b'v');
            if wk || wv {
                t.marked = (marked & 0xf9) | (u8::from(wv) << 2) | (u8::from(wk) << 1);
                t.list = Some(u8::from(wk) | (u8::from(wv) << 1));
            }
        }
    }
    if !wv {
        for i in (0..array.len()).rev() {
            if markable(&array[i]) {
                t.array.push(i);
            }
        }
    }
    for i in (0..nodes.len()).rev() {
        let (k, v) = &nodes[i];
        if v.tag == 0 {
            continue;
        }
        if markable(k) && !wk {
            t.nodes.push((i, false));
        }
        if markable(v) && !wv {
            t.nodes.push((i, true));
        }
    }
    t
}

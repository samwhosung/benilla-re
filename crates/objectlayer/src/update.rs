//! The update packet's block readers that need no object behind them
//! (`systems/objectlayer/update.md`): the values skip, the out-of-range and near-object lists,
//! the values block's dispatch, and the GUID split of the two passes.

use crate::datastore::DataStore;

/// The passes' split: the GUID's top four bits are 6 or 8.
pub fn flagged_guid(guid: u64) -> bool {
    matches!(guid >> 60, 6 | 8)
}

/// `SkipValuesBlock` (`0x465bb0`): a count byte `n`, `n` mask words, then a dword for each set
/// bit below `32n`, read and dropped; words past `n` are zero up to 41. The words sit below the
/// value the reads fill, so with 42 words the last is that value: each read replaces it. The
/// model takes counts up to 42; at 43 the next word is the saved store pointer.
pub fn skip_values_block(s: &mut DataStore) {
    let n = usize::from(s.get_u8().unwrap_or(0));
    assert!(
        n <= 42,
        "a count above 42 overwrites the saved store pointer"
    );
    let mut mask = vec![0u32; n.max(41)];
    for w in mask.iter_mut().take(n) {
        if let Some(v) = s.get_u32() {
            *w = v;
        }
    }
    for i in 0..32 * n {
        if mask[i >> 5] & (1 << (i & 31)) != 0 {
            if let Some(v) = s.get_u32() {
                if n == 42 {
                    mask[41] = v;
                }
            }
        }
    }
}

/// `ProcessOutOfRangeObjects` (`0x465ec0`) on a store whose GUIDs are whole: a u32 count and
/// that many packed GUIDs. Returns the GUIDs released (`0x464920`), in order: first every GUID
/// the split does not flag, except `active`; then, rereading from the list's start, the flagged
/// ones, `active` included, until as many have been seen as the first round counted. The cursor
/// ends after the list.
pub fn out_of_range(s: &mut DataStore, active: u64) -> Vec<u64> {
    let mut released = Vec::new();
    let count = s.get_u32().unwrap_or(0);
    let start = s.pos;
    let mut flagged = 0u32;
    for _ in 0..count {
        let g = s.get_pack_guid();
        if flagged_guid(g) {
            flagged += 1;
        } else if g != active {
            released.push(g);
        }
    }
    if flagged != 0 {
        let end = s.pos;
        s.pos = start;
        while flagged != 0 {
            let g = s.get_pack_guid();
            if flagged_guid(g) {
                released.push(g);
                flagged -= 1;
            }
        }
        s.pos = end;
    }
    released
}

/// `ProcessNearObjects` (`0x467230`, `dl` the pass flag) on a store whose GUIDs are whole: a
/// u32 count and that many packed GUIDs; each one that is not `active` and whose split equals
/// `flag` goes to `0x4644f0`, which looks it up and reactivates it. Returns those GUIDs; the
/// function returns their count.
pub fn near_objects(s: &mut DataStore, active: u64, flag: bool) -> Vec<u64> {
    let count = s.get_u32().unwrap_or(0);
    (0..count)
        .map(|_| s.get_pack_guid())
        .filter(|&g| g != active && flagged_guid(g) == flag)
        .collect()
}

/// What `ObjectValuesBlock` (`0x467160`, `dl` the pass flag) does with a values block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValuesBlock {
    /// The GUID's split differs from the pass flag: the block is skipped, with no lookup.
    OtherPass,
    /// `ClntObjMgrFindOrReactivate` (`0x464530`) found nothing: the block is skipped.
    Unknown,
    /// The object's values are written (`0x466590`, with the GUID in the object's own field 0
    /// and two zero flags), then slot 2 runs when the lookup reactivated it.
    Applied { reactivated: bool },
}

/// `ObjectValuesBlock` on a packed GUID and a values block: the split against `flag`, then the
/// lookup, `lookup(guid)` being `None` for no object or whether it was reactivated. A skipped
/// block goes through [`skip_values_block`].
pub fn values_block(
    s: &mut DataStore,
    flag: bool,
    lookup: impl Fn(u64) -> Option<bool>,
) -> (u64, ValuesBlock) {
    let g = s.get_pack_guid();
    if flagged_guid(g) != flag {
        skip_values_block(s);
        return (g, ValuesBlock::OtherPass);
    }
    match lookup(g) {
        None => {
            skip_values_block(s);
            (g, ValuesBlock::Unknown)
        }
        Some(reactivated) => (g, ValuesBlock::Applied { reactivated }),
    }
}

/// The calls `ObjectUpdateHandler` makes for the blocks; each may move the cursor.
pub trait UpdateBlocks {
    /// `ProcessOutOfRangeObjects` (`0x465ec0`), for an out-of-range first block.
    fn out_of_range(&mut self, s: &mut DataStore);
    /// `ObjectCreatePass` (`0x466010`) over blocks `start..count`, with the pass flag.
    fn pass(&mut self, s: &mut DataStore, time: u32, count: u32, start: u32, flag: bool);
    /// The notify pass's handler for a block of type 0 to 5 (`0x465314`): 0 `0x465330`, 1
    /// `0x465ea0`, 2 and 3 `0x465c50` (told whether the type is 3), 4 and 5 `0x465fd0`.
    fn notify(&mut self, s: &mut DataStore, time: u32, ty: u8);
}

/// `ObjectUpdateHandler` (`0x4651a0`, `(time, store)`) on a store that holds its header: a u32
/// block count and a flag byte. The first block's type byte is read; for 4 the out-of-range
/// list is handled and the passes start at block 1, else the cursor goes back. A nonzero flag
/// runs the flagged pass, after which the cursor is put back; the second pass follows. The
/// notify pass then reads each block's type from block 0, calling its handler for types up to
/// 5 and nothing more for others. The reactivated objects' slot 3 follows (not modelled here);
/// the handler returns 1.
pub fn update_handler(s: &mut DataStore, time: u32, blocks: &mut impl UpdateBlocks) -> u32 {
    let count = s.get_u32().unwrap_or(0);
    let flag = s.get_u8().unwrap_or(0);
    let start = s.pos;
    let mut first = 0;
    if s.get_u8().unwrap_or(0) == 4 {
        blocks.out_of_range(s);
        first = 1;
    } else {
        s.pos = start;
    }
    if flag != 0 {
        let at = s.pos;
        blocks.pass(s, time, count, first, true);
        s.pos = at;
    }
    blocks.pass(s, time, count, first, false);
    s.pos = start;
    for _ in 0..count {
        let ty = s.get_u8().unwrap_or(0);
        if ty <= 5 {
            blocks.notify(s, time, ty);
        }
    }
    1
}

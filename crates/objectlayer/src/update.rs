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

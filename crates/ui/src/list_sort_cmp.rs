//! Comparators of the interface's sorted lists that return a verdict or the pair to collate: the
//! mount trainer rows, the quest log groups and rows, the group finder lists, the craft rows and
//! two lists that `0x4fd180` orders. Each runs an ordered key cascade over two records; a tie that
//! reaches the locale collator (`0x64a4c0`, or `0x64a480` for the player name cache) comes back as
//! [`Cmp::Collate`] over the two resolved strings, and the test runs the real collator.
//!
//! The binary builds its result with `(cond) * 2 - 1` (cond gives `+1`), `(-(cond) & 2) - 1` (cond
//! gives `+1`) or `(-(cond) & 0xfffffffe) + 1` (cond gives `-1`); each key reproduces the exact
//! comparison, `<` or `<=`, and the direction. A comparator's arguments are the qsort element
//! pointers: a list of pointers is dereferenced, a list of inline structs is read directly. The
//! caller supplies the record fields, the name-cache resolution (`id < 0 || id > max` gives null,
//! otherwise `table[id * 4]`) and the locale index (`*(obj + off + 0xc0e080 * 4)`); a `0` pointer
//! is absent.
//!
//! Entries: `systems/ui/trainerlist.md#row-order`, `systems/ui/questlog.md#the-order-of-the-rows`,
//! `systems/ui/lfg.md#the-type-lists`, `systems/ui/craft.md#craft-order`.

/// A comparator verdict. `Lt`/`Eq`/`Gt` are decided (sign −1/0/1). `Collate` is a tie deferred to
/// the engine string collator over two already-resolved pointers `a`,`b` (`0` = an empty/absent
/// string). `tie`, when present, is the post-collation fall-back: if the collator returns `0`, the
/// result is `(tie.1 < tie.0) ? +1 : -1`, the binary's `(*(b+k) < *(a+k)) * 2 - 1` tail, the `+0x74` key
/// of `0x4f6920`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmp {
    Lt,
    Eq,
    Gt,
    Collate {
        a: u32,
        b: u32,
        tie: Option<(i32, i32)>,
    },
}

impl Cmp {
    /// `(cond)*2 - 1` and `(-(cond) & 2) - 1`: `cond ⇒ Gt(+1)`, else `Lt(-1)`.
    #[inline]
    fn pos(cond: bool) -> Cmp {
        if cond {
            Cmp::Gt
        } else {
            Cmp::Lt
        }
    }
    /// `(-(cond) & 0xfffffffe) + 1`: `cond ⇒ Lt(-1)`, else `Gt(+1)`.
    #[inline]
    fn neg(cond: bool) -> Cmp {
        if cond {
            Cmp::Lt
        } else {
            Cmp::Gt
        }
    }
}

/// A row of the mount trainer list for [`sortcmp_4d8850`] (`rec = *param`), `CompareMountTrainerRows`
/// (`systems/ui/trainerlist.md#row-order`).
#[derive(Clone, Copy, Debug)]
pub struct MountTrainerRow {
    /// `+0x34` (`rec[0xd]`): the visible flag, the first key.
    pub valid: i32,
    /// `+0` (`*rec`): the spell id, which indexes the cache at `0xc0d788`; a negative value sorts first.
    pub sign_id: i32,
    /// `+0x4` (`rec[1]`): the group key, mapped to a position through `order`.
    pub group: i32,
    /// `+0x30` (byte): a key compared with `(b < a)` giving +1.
    pub byte30: u8,
    /// The record `0xc0d788[sign_id]`; `0` = absent.
    pub name_obj: u32,
    /// The locale name string `*(obj + 0x1e0 + loc * 4)`, for the `0x64a480` collate.
    pub name_str: u32,
}

/// `CompareMountTrainerRows` (`0x4d8850`). Cascade: visible (`+0x34`) -> position of the group in
/// `order` (the header list at `0xb739c0`, empty skips the key) -> a negative `+0` first -> byte
/// `+0x30` (`b < a` gives +1) -> the name `+0x1e0` through the collator `0x64a480`.
pub fn sortcmp_4d8850(a: &MountTrainerRow, b: &MountTrainerRow, order: &[i32]) -> Cmp {
    // `rec[0xd]`: both not visible gives 0, one visible puts that row first.
    if a.valid != 0 && b.valid != 0 {
        // The position of the group in `order`: the last match wins, the default is 0, and an
        // empty `order` skips the key.
        if !order.is_empty() {
            let idx_a = find_last(order, a.group);
            let idx_b = find_last(order, b.group);
            if idx_a != idx_b {
                // `(idx_b < idx_a) * 2 - 1`.
                return Cmp::pos(idx_b < idx_a);
            }
        }
        // Either `+0` negative: `(a.sign >= 0) * 2 - 1`.
        if a.sign_id < 0 || b.sign_id < 0 {
            return Cmp::pos(a.sign_id >= 0);
        }
        // `(b < a)` gives +1.
        if a.byte30 != b.byte30 {
            return Cmp::pos(b.byte30 < a.byte30);
        }
        // Both records must resolve, else 0.
        if a.name_obj == 0 || b.name_obj == 0 {
            return Cmp::Eq;
        }
        return Cmp::Collate {
            a: a.name_str,
            b: b.name_str,
            tie: None,
        };
    }
    if a.valid == 0 && b.valid == 0 {
        return Cmp::Eq;
    }
    // Exactly one visible: `(b.valid != 0) * 2 - 1`.
    Cmp::pos(b.valid != 0)
}

/// `CompareQuestHeaderIds` (`0x4de8f0`): `0x4de751` sorts the array of distinct group ids `0xbb7040`
/// with it. Equal ids give 0, a first id of 0 gives -1 and a second id of 0 gives +1; otherwise each
/// resolved name is staged into a 0x100 buffer (`0x64a5a0`) and collated with `0x64a4c0`, the
/// case-insensitive `strnicmp` with `n = 0x7fffffff`. The id-to-name step (an id above 0 reads
/// `AreaTable` `0xc0e048[id]+0x2c`, below 0 `QuestSort` `0xc0d9b8[-id]+0x4`, both by locale) is the
/// caller's; `name_a` and `name_b` are the resolved source pointers, `0` for an unresolved id,
/// which stages an empty buffer that collates first. The collate runs over the staged buffers.
pub fn sortcmp_4de8f0(id_a: i32, id_b: i32, name_a: u32, name_b: u32) -> Cmp {
    if id_a == id_b {
        return Cmp::Eq;
    }
    if id_a == 0 {
        return Cmp::Lt; // 0xffffffff
    }
    if id_b == 0 {
        return Cmp::Gt;
    }
    Cmp::Collate {
        a: name_a,
        b: name_b,
        tie: None,
    }
}

/// A quest log row (`0xbb71c0`, stride 0x10) for [`sortcmp_4deac0`] (`param` is read directly).
#[derive(Clone, Copy, Debug)]
pub struct RecQuestLogRow {
    /// `+0` (`*param`): the quest id of a quest row, the group id of a header row.
    pub id6: i32,
    /// `+0x8` (`param[2]`): 0 for a quest row (the template resolves through `0x562a40`), nonzero for a
    /// header.
    pub field8: i32,
    /// Quest row only: the quest template, `0` for a cache miss.
    pub obj: u32,
    /// Quest row only: `tpl+0xc`, the group id, looked up in the group id array.
    pub class_id: i32,
    /// Quest row only: `tpl+8`, the quest level.
    pub rank: i32,
    /// Quest row only: `tpl+0x9c`, the inline title string, for the `0x64a4c0` collate.
    pub name_str: u32,
}

/// `CompareQuestLogRows` (`0x4deac0`), the sort at `0x73f727(0xbb71c0, count, 0x10, 0x4deac0)`.
/// Cascade: a quest under a collapsed group last -> the position of the group in `class_table` ->
/// a header before its quests -> the quest level `tpl+8`, the lower first -> the title `tpl+0x9c`
/// through `0x64a4c0`. `class_table` is `0xbb7040` (up to 20 group ids, already name-sorted by
/// `CompareQuestHeaderIds`; the search takes the first match, else 0). `special_mask` is `0xbb748c`,
/// which holds the expanded bits: a quest is hidden when its group's bit is clear. A quest whose
/// template is not cached gives 0. The comparator resolves no group name and reads neither the
/// slot index at `+0x4` nor the expiry latch at `+0xc`.
pub fn sortcmp_4deac0(
    a: &RecQuestLogRow,
    b: &RecQuestLogRow,
    class_table: &[i32],
    special_mask: u32,
) -> Cmp {
    // member resolution presence: a member (field8==0) whose object is null ⇒ unsortable (0).
    if a.field8 == 0 && a.obj == 0 {
        return Cmp::Eq;
    }
    if b.field8 == 0 && b.obj == 0 {
        return Cmp::Eq;
    }
    let class_a = if a.field8 == 0 { a.class_id } else { a.id6 };
    let class_b = if b.field8 == 0 { b.class_id } else { b.id6 };
    let idx_a = find_class(class_table, class_a);
    let idx_b = find_class(class_table, class_b);

    let special_a = a.field8 == 0 && (1u32 << (idx_a & 0x1f) & special_mask) == 0;
    let special_b = b.field8 == 0 && (1u32 << (idx_b & 0x1f) & special_mask) == 0;

    if special_a {
        if special_b {
            return Cmp::Eq; // both special ⇒ tie
        }
        // a special only: +1.
        return Cmp::Gt;
    } else if !special_b {
        // neither special: the inner cascade.
        if idx_a != idx_b {
            // `(-(idx_a < idx_b) & 0xfffffffe) + 1`: idx_a < idx_b gives -1.
            return Cmp::neg(idx_a < idx_b);
        }
        if a.field8 != 0 {
            return Cmp::Lt; // a header ⇒ a first
        }
        if b.field8 != 0 {
            return Cmp::Gt; // b header ⇒ b first
        }
        // Both quests: the level at `+8`, then the title.
        if a.rank != b.rank {
            // `(b.rank <= a.rank) * 2 - 1`; the levels differ, so +1 when b < a.
            return Cmp::pos(b.rank <= a.rank);
        }
        return Cmp::Collate {
            a: a.name_str,
            b: b.name_str,
            tie: None,
        };
    }
    // b special only: -1.
    Cmp::Lt
}

// `CompareLfgEntries` (`0x4e8b20`): `0x4e89c0` sorts the type lists with it (`push 0x4e8b20` follows
// `0x4e8add`); `0xc0db34` is the `LfgDungeons` id map.

/// An `LfgDungeons` row for [`sortcmp_4e8b20`] (`id = *param`, resolved through `0xc0db34`).
#[derive(Clone, Copy, Debug)]
pub struct LfgEntryRow {
    /// The resolved row; `0` = absent.
    pub obj: u32,
    /// `obj+0x28`: the least level.
    pub lvl_lo: i32,
    /// `obj+0x2c`: the greatest level.
    pub lvl_hi: i32,
    /// `obj+0x4`: the locale name string, for the `0x64a4c0` collate.
    pub name_str: u32,
}

/// `CompareLfgEntries` (`0x4e8b20`). Cascade: a row that exists first -> a row whose level range holds
/// the player's level (`obj+0x28 <= 0xbc7624 <= obj+0x2c`) first -> the name `+0x4` through
/// `0x64a4c0`. `level` is the player's level `0xbc7624`.
pub fn sortcmp_4e8b20(a: &LfgEntryRow, b: &LfgEntryRow, level: i32) -> Cmp {
    if a.obj == 0 {
        if b.obj == 0 {
            return Cmp::Eq;
        }
        // a absent, b present: `(a absent) * 2 - 1` is +1.
        return Cmp::Gt;
    }
    if b.obj != 0 {
        // in-range = !(level < lo || hi < level)
        let in_a = !(level < a.lvl_lo || a.lvl_hi < level);
        let in_b = !(level < b.lvl_lo || b.lvl_hi < level);
        if in_a != in_b {
            // `(!in_a) * 2 - 1`.
            return Cmp::pos(!in_a);
        }
        return Cmp::Collate {
            a: a.name_str,
            b: b.name_str,
            tie: None,
        };
    }
    // a present, b absent: `(a absent) * 2 - 1` is -1.
    Cmp::Lt
}

// `CompareLfgQuests` (`0x4e8c80`): `0x4e8be0` sorts the quest ids of the log with it (`0x4e8c52`).

/// `CompareLfgQuests` (`0x4e8c80`). Cascade: a quest whose record is cached first -> the title `+0x9c`
/// through `0x64a4c0`. `obj_a` and `obj_b` are the cached records (`0` = absent); `name_a` and
/// `name_b` are the inline title pointers.
pub fn sortcmp_4e8c80(obj_a: u32, name_a: u32, obj_b: u32, name_b: u32) -> Cmp {
    if obj_a == 0 {
        if obj_b == 0 {
            return Cmp::Eq;
        }
    } else if obj_b != 0 {
        return Cmp::Collate {
            a: name_a,
            b: name_b,
            tie: None,
        };
    }
    // `(a absent) * 2 - 1`: +1 for an absent a, else -1.
    Cmp::pos(obj_a == 0)
}

/// A craft list row for [`sortcmp_4f67a0`] and [`sortcmp_4f6920`] (`rec = *param`),
/// `CompareCraftRows` and `CompareCraftRowsTrain` (`systems/ui/craft.md#craft-order`).
#[derive(Clone, Copy, Debug)]
pub struct CraftRow {
    /// `+0` (`*rec`): a `-1` sentinel and the cache id of the record that `name_obj` stands for; the
    /// group prefix runs only for a value of 0 or more.
    pub sign_id: i32,
    /// `+0x4` (`rec[1]`): the group key, matched against `entry[0]` of the group table.
    pub group_key: i32,
    /// `+0xc` (`rec[3]`): the key after grouping, `(b < a)` giving +1.
    pub key_c: i32,
    /// The record behind `sign_id`; `0` = absent.
    pub name_obj: u32,
    /// `obj+0x1e0`: the locale name string, for the collate.
    pub name_str: u32,
    /// `obj+0x74`: the last key, `(b < a)` giving +1 (`0x4f6920` only).
    pub plus74: i32,
}

/// A group table entry (`0xbdcf88[i]`): `entry[0]` is the group key, `entry[2]` a flag that places
/// the group after the others when nonzero.
#[derive(Clone, Copy, Debug)]
pub struct CraftGroup {
    pub key: i32,
    pub entry2: i32,
}

/// The group prefix shared by the two craft comparators. Returns `Some(verdict)` when the groups
/// decide, `None` to fall through to the name path. An empty `online` table goes to the name path.
fn craft_group_prefix(a: &CraftRow, b: &CraftRow, online: &[CraftGroup]) -> Option<Cmp> {
    if online.is_empty() {
        return None;
    }
    // Both start as true with index 0; a match overwrites them.
    let (mut grp_a, mut grp_b) = (true, true);
    let (mut idx_a, mut idx_b) = (0usize, 0usize);
    for (i, e) in online.iter().enumerate() {
        if e.key == a.group_key && a.sign_id >= 0 {
            grp_a = e.entry2 == 0;
            idx_a = i;
        }
        if e.key == b.group_key && b.sign_id >= 0 {
            grp_b = e.entry2 == 0;
            idx_b = i;
        }
    }
    if grp_a {
        if grp_b {
            if idx_a != idx_b {
                // `(-(idx_b < idx_a) & 2) - 1`.
                return Some(Cmp::pos(idx_b < idx_a));
            }
            return None; // same index: the name path
        }
        // grp_a and not grp_b: -1.
        return Some(Cmp::Lt);
    } else if !grp_b {
        return Some(Cmp::Eq); // neither: 0
    }
    // grp_b and not grp_a: +1.
    Some(Cmp::Gt)
}

/// `CompareCraftRows` (`0x4f67a0`). Cascade: the group prefix -> the `-1` sentinel -> `+0xc` -> the
/// name `+0x1e0` through `0x64a4c0`. The sibling of [`sortcmp_4f6920`] without the `+0x74` key.
pub fn sortcmp_4f67a0(a: &CraftRow, b: &CraftRow, online: &[CraftGroup]) -> Cmp {
    if let Some(v) = craft_group_prefix(a, b, online) {
        return v;
    }
    // name path
    if a.sign_id == -1 || b.sign_id == -1 {
        // `(b == -1) * 2 - 1`.
        return Cmp::pos(b.sign_id == -1);
    }
    if a.name_obj != 0 && b.name_obj != 0 {
        if a.key_c == b.key_c {
            return Cmp::Collate {
                a: a.name_str,
                b: b.name_str,
                tie: None,
            };
        }
        // `(b.key_c < a.key_c) * 2 - 1`.
        return Cmp::pos(b.key_c < a.key_c);
    }
    Cmp::Eq
}

/// `CompareCraftRowsTrain` (`0x4f6920`). Cascade: the group prefix -> the `-1` sentinel -> `+0xc` ->
/// the name `+0x1e0` through `0x64a4c0` -> on a name tie, `+0x74` (carried in [`Cmp::Collate`]'s
/// `tie`). Its presence gate is `obj == 0` giving 0.
pub fn sortcmp_4f6920(a: &CraftRow, b: &CraftRow, online: &[CraftGroup]) -> Cmp {
    if let Some(v) = craft_group_prefix(a, b, online) {
        return v;
    }
    if a.sign_id == -1 || b.sign_id == -1 {
        return Cmp::pos(b.sign_id == -1);
    }
    if a.name_obj == 0 || b.name_obj == 0 {
        return Cmp::Eq;
    }
    if a.key_c != b.key_c {
        return Cmp::pos(b.key_c < a.key_c);
    }
    // Collate the names; a tie falls back to `(b.plus74 < a.plus74) * 2 - 1`.
    Cmp::Collate {
        a: a.name_str,
        b: b.name_str,
        tie: Some((a.plus74, b.plus74)),
    }
}

/// A record of the list that `0x4fd180` orders with `0x4fd380` (`push 0x4fd366`; `rec = *param`).
#[derive(Clone, Copy, Debug)]
pub struct Rec4fd380 {
    /// `+0` (`*rec`): a `-1` sentinel and the cache id behind `name_obj`.
    pub sign_id: i32,
    /// `+0x4` (`rec[1]`): a key of the name path, `(b < a)` giving +1.
    pub plus4: i32,
    /// `+0x8` (`rec[2]`): the key matched against `entry[0]`.
    pub plus8: i32,
    /// `+0xc` (`rec[3]`): the key matched against `entry[1]`.
    pub plus_c: i32,
    /// `+0x14` (`rec[5]`): a key of the name path, `(a < b)` giving +1.
    pub plus14: i32,
    /// `+0x1c` (`rec[7]`): the selector tested with `entry[3]`.
    pub plus1c: i32,
    /// The record behind `sign_id`; `0` = absent.
    pub name_obj: u32,
    /// `obj+0x1e0`: the locale name string, for the collate.
    pub name_str: u32,
}

/// An entry of the table at `0xbde000`: `entry[0]` and `entry[1]` are the match keys, `entry[3]` the
/// modifier tested with `rec+0x1c`.
#[derive(Clone, Copy, Debug)]
pub struct Group4fd380 {
    pub e0: i32,
    pub e1: i32,
    pub e3: i32,
}

/// `0x4fd380`. Cascade: the group of the table `0xbde000` found by `+0x8` and `+0xc` -> the group
/// index -> the `-1` sentinel -> `+0x4` -> `+0x14` -> the name `+0x1e0` through `0x64a4c0`. An empty
/// `groups` goes straight to the name path.
pub fn sortcmp_4fd380(a: &Rec4fd380, b: &Rec4fd380, groups: &[Group4fd380]) -> Cmp {
    if !groups.is_empty() {
        // bVar* init true; idx init 0. Match on (entry[0]==rec+8 && entry[1]==rec+0xc).
        let (mut grp_a, mut grp_b) = (true, true);
        let (mut idx_a, mut idx_b) = (0usize, 0usize);
        for (i, e) in groups.iter().enumerate() {
            if e.e0 == a.plus8 && e.e1 == a.plus_c {
                idx_a = i;
                // `!(rec[7] == 0 || entry[3] == 0)`
                grp_a = !(a.plus1c == 0 || e.e3 == 0);
            }
            if e.e0 == b.plus8 && e.e1 == b.plus_c {
                idx_b = i;
                grp_b = !(b.plus1c == 0 || e.e3 == 0);
            }
        }
        if grp_a {
            if grp_b {
                if idx_a != idx_b {
                    // `(-(idx_b < idx_a) & 2) - 1`.
                    return Cmp::pos(idx_b < idx_a);
                }
                // fall through to name path
            } else {
                return Cmp::Lt; // grp_a and not grp_b
            }
        } else if !grp_b {
            return Cmp::Eq; // neither ⇒ 0
        } else {
            return Cmp::Gt; // grp_b and not grp_a
        }
    }
    // name path
    if a.sign_id == -1 || b.sign_id == -1 {
        return Cmp::pos(b.sign_id == -1);
    }
    if a.name_obj != 0 && b.name_obj != 0 {
        if a.plus4 != b.plus4 {
            return Cmp::pos(b.plus4 < a.plus4);
        }
        if a.plus14 == b.plus14 {
            return Cmp::Collate {
                a: a.name_str,
                b: b.name_str,
                tie: None,
            };
        }
        return Cmp::pos(a.plus14 < b.plus14);
    }
    Cmp::Eq
}

/// A record of the list that `0x4fd180` orders with `0x4fd540` (`push 0x4fd34d`; `rec = *param`).
#[derive(Clone, Copy, Debug)]
pub struct Rec4fd540 {
    /// `+0` (`*rec`): the first key, `(b < a)` giving +1, and the match key for `entry[0]`.
    pub plus0: i32,
    /// `+0x4` (`rec[1]`): the match key for `entry[1]`.
    pub plus4: i32,
}

/// An entry of the table at `0xc0db90` (stride `0x1d` dwords): `e0` and `e1` are the match keys, the
/// locale name is `entry[loc + 0x13]` (`primary_empty` when it is null or empty) and the fallback
/// is `entry[loc + 10]`.
#[derive(Clone, Copy, Debug)]
pub struct Entry4fd540 {
    pub e0: i32,
    pub e1: i32,
    pub name_primary: u32,
    pub primary_empty: bool,
    pub name_fallback: u32,
}

/// `0x4fd540`. Cascade: `+0` (`b < a` gives +1) -> the name `entry[loc + 0x13]`, else
/// `entry[loc + 10]` when that is empty, through `0x64a4c0`. The walk pairs each row to its entry by
/// `(+0, +0x4)`; it is reached only when `a.plus0 == b.plus0`, so both rows share `entry[0]`.
pub fn sortcmp_4fd540(a: &Rec4fd540, b: &Rec4fd540, craft: &[Entry4fd540]) -> Cmp {
    if a.plus0 != b.plus0 {
        // `(b.plus0 < a.plus0) * 2 - 1`.
        return Cmp::pos(b.plus0 < a.plus0);
    }
    // table walk, in the binary's order of matches: `pi9` is A's entry, `pparam1` is B's entry and
    // `pi10` is scratch.
    if craft.is_empty() {
        return Cmp::Eq;
    }
    let mut pi9: Option<usize> = None; // A's entry
    let mut pparam1: Option<usize> = None; // B's entry
    let mut pi10: Option<usize> = None; // scratch
    for (i, e) in craft.iter().enumerate() {
        // A matches `entry[0] == a.plus0` (equal to `b.plus0` here): `pi10` takes B's entry; on
        // `entry[1] == a.plus4` `pi9` takes this entry, and the walk stops if B's entry is found.
        if e.e0 == a.plus0 {
            pi10 = pparam1;
            if e.e1 == a.plus4 {
                pi9 = Some(i);
                if pparam1.is_some() {
                    break;
                }
            }
        }
        // B matches: `pi10` and `pparam1` take this entry; the collate follows if A's is found.
        if e.e0 == b.plus0 && e.e1 == b.plus4 {
            pi10 = Some(i);
            pparam1 = Some(i);
            if let Some(ai) = pi9 {
                return craft_collate(craft, ai, i);
            }
        }
    }
    // Collate only if A's entry and `pi10` are both present.
    if let (Some(ai), Some(bi)) = (pi9, pi10) {
        return craft_collate(craft, ai, bi);
    }
    Cmp::Eq
}

/// The name pick of `0x4fd540` (`entry[loc + 0x13]`, else `entry[loc + 10]` when null or empty) and
/// its collate; a null name on either side gives 0.
fn craft_collate(craft: &[Entry4fd540], ai: usize, bi: usize) -> Cmp {
    let pick = |e: &Entry4fd540| -> u32 {
        if e.name_primary == 0 || e.primary_empty {
            e.name_fallback
        } else {
            e.name_primary
        }
    };
    let na = pick(&craft[ai]);
    let nb = pick(&craft[bi]);
    if na != 0 && nb != 0 {
        Cmp::Collate {
            a: na,
            b: nb,
            tie: None,
        }
    } else {
        Cmp::Eq
    }
}

/// Priority index of `group` in `order` (`0xb739c0`): a full scan, **last** match wins, default
/// `0` (the loop does not break on a match).
fn find_last(order: &[i32], group: i32) -> usize {
    let mut idx = 0usize;
    for (i, &g) in order.iter().enumerate() {
        if g == group {
            idx = i;
        }
    }
    idx
}

/// Class index of `class_id` in `class_table` (`0xbb7040`, ≤20 entries): the **first** match,
/// default `0` (the loop breaks on a match).
fn find_class(class_table: &[i32], class_id: i32) -> usize {
    class_table
        .iter()
        .take(0x14)
        .position(|&c| c == class_id)
        .unwrap_or(0)
}

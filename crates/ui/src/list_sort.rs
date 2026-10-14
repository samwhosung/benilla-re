//! Comparators of the interface's sorted lists that call the collator they are given: the mail
//! stationery list, the spell book, the auction list, the guild roster, the reputation panel and
//! the trainer lists. Each reads two records, runs an ordered key cascade and returns the `-1`, `0`
//! or `+1` that fixes the row order, or the collator's value at a tie. Only the guild roster's
//! `0x4d0d50` has a float key, compared with `<=`.
//!
//! Two inputs are fed in rather than recomputed. The record-to-object resolution (`0x55ba30`,
//! `0x55f080` and the name caches) is runtime state: a resolved object is passed as `Option`, `None`
//! for the cache miss the cascade routes to one end. The locale string collator (`0x64a4c0`, or
//! `0x64a480` for the player name cache) is a closure over the two locale-selected name pointers; the
//! cascade decides when to collate, with which pair, and whether to negate the result. The asc and
//! desc senses follow the bytes.
//!
//! Entries: `systems/ui/mailsend.md#the-stationery-list`, `systems/ui/spellbook.md#book-order`,
//! `systems/ui/auctionlists.md#sort-columns`, `systems/ui/guild.md#the-roster-order`,
//! `systems/ui/reputation.md#sorting-and-counts`, `systems/ui/trainerlist.md#row-order`.

/// Negate `r` when the live direction flag is set (`0x4d0d50`'s per-column `dir`: `neg eax`).
#[inline]
fn neg_if(dir: bool, r: i32) -> i32 {
    if dir {
        -r
    } else {
        r
    }
}

// `CompareStationery` (`0x4ada90`): `0x4ad970` sorts `[0xb6ee30]`, an array of stationery records,
// with it (`push 0x4ada90` at `0x4ada74`).

/// A stationery entry (`0x18` bytes) as the cascade reads it (`*param` is the record).
#[derive(Clone, Copy, Debug)]
pub struct StationeryRecFields {
    /// `+0x8`: the always-available bit, `Stationery.dbc` `Flags & 1` (part of the availability gate).
    pub f8: u32,
    /// `+0x10`: the low half of the carried item's GUID (the availability gate and the carried key).
    pub f10: u32,
    /// `+0x14`: the high half of the same GUID.
    pub f14: u32,
}

/// The item template for the record's `itemId` (`0x55ba30` on `0xc0e2a0`, callback 0, a
/// passive peek); `None` = not cached.
#[derive(Clone, Copy, Debug)]
pub struct StationeryItemTemplate {
    /// `+0x24` — `BuyPrice` in copper (signed; `objB <= objA ⇒ +1`, i.e. the CHEAPER sorts first).
    pub f24: i32,
    /// `+0x08` — `Name[0]`, handed to the collator.
    pub name: u32,
}

/// `CompareStationery` (`0x4ada90`). Cascade: availability (`+8 | +0x10 | +0x14` nonzero, available
/// first) -> carried (`+0x10 | +0x14`, carried before merely available) -> the item template by
/// `+0x4` (cached before uncached) -> the buy price `obj+0x24`, the cheaper first (`objB <= objA`
/// gives +1) -> the name (`0x64a4c0`).
pub fn sortcmp_4ada90(
    a: &StationeryRecFields,
    b: &StationeryRecFields,
    oa: Option<StationeryItemTemplate>,
    ob: Option<StationeryItemTemplate>,
    collate: &mut dyn FnMut(u32, u32) -> i32,
) -> i32 {
    let active = |r: &StationeryRecFields| (r.f8 != 0 || r.f10 != 0 || r.f14 != 0) as i32;
    let aa = active(a);
    let ab = active(b);
    if aa != ab {
        // unavailable (0) sorts last: `(avail==0)*2-1`.
        return if aa == 0 { 1 } else { -1 };
    }
    let in_prog = |r: &StationeryRecFields| r.f10 != 0 || r.f14 != 0;
    if in_prog(a) && !in_prog(b) {
        return -1;
    }
    if in_prog(b) && !in_prog(a) {
        return 1;
    }
    match (oa, ob) {
        (None, None) => 0,
        (Some(oa), Some(ob)) => {
            if oa.f24 != ob.f24 {
                // `(objB+0x24 <= objA+0x24) * 2 - 1` — the cheaper template sorts first.
                return if ob.f24 <= oa.f24 { 1 } else { -1 };
            }
            collate(oa.name, ob.name)
        }
        // exactly one resolved: `(objA==null)*2-1` — A uncached ⇒ +1, A cached ⇒ -1.
        (None, Some(_)) => 1,
        (Some(_), None) => -1,
    }
}

// `CompareBookSpells` (`0x4b30c0`).

/// A spell of the book (`0xc0d788[rec+0]`); `None` = name-cache miss.
#[derive(Clone, Copy, Debug)]
pub struct BookSpell {
    /// `0x4b24f0(*obj)`: the position of the spell's tab (unsigned, ascending).
    pub tab_pos: u32,
    /// `0x4b3c30(obj)`: the rank (unsigned; compared only when both are nonzero).
    pub rank: u32,
    /// `obj+0x1e0`: the name pointer.
    pub name: u32,
    /// `obj+0x204`: the rank string pointer.
    pub rank_text: u32,
}

/// `CompareBookSpells` (`0x4b30c0`, collator `0x64a480`). Cascade: both spells must resolve (else 0)
/// -> tab position (ascending) -> name `+0x1e0` -> rank (both nonzero, ascending) -> rank string
/// `+0x204`.
pub fn sortcmp_4b30c0(
    oa: Option<BookSpell>,
    ob: Option<BookSpell>,
    collate: &mut dyn FnMut(u32, u32) -> i32,
) -> i32 {
    let (oa, ob) = match (oa, ob) {
        (Some(oa), Some(ob)) => (oa, ob),
        _ => return 0,
    };
    if oa.tab_pos != ob.tab_pos {
        // `(-(posA < posB) & 0xfffffffe) + 1`: posA < posB gives -1.
        return if oa.tab_pos < ob.tab_pos { -1 } else { 1 };
    }
    let c = collate(oa.name, ob.name);
    if c != 0 {
        return c;
    }
    if oa.rank != 0 && ob.rank != 0 {
        // rankA < rankB gives -1, otherwise `rankB < rankA` as 1 or 0 (unsigned).
        if oa.rank < ob.rank {
            return -1;
        }
        return (ob.rank < oa.rank) as i32;
    }
    collate(oa.rank_text, ob.rank_text)
}

// The auction list comparator `0x4cd940`: a sort column in `0..=7`, the columns of the auction frame.

/// The auction record fields the columns read; the record is read directly, not through a pointer.
#[derive(Clone, Copy, Debug)]
pub struct AuctionRec {
    /// `+0x40`, the bid; `+0x28` stands in when it is 0 (column 2). Signed.
    pub f40: i32,
    /// `+0x28`, the minimum bid. Signed.
    pub f28: i32,
    /// `+0x34`, the deadline (column 3). Unsigned.
    pub f34: u32,
    /// `+0x30`, the buyout (column 4). Unsigned.
    pub f30: u32,
    /// `+0x38`, the low dword of the bidder GUID (column 5).
    pub f38: u32,
    /// `+0x3c`, the high dword of the bidder GUID (column 5).
    pub f3c: u32,
}

/// The cached item record of an auction (`0x55ba30(rec+0x8)`), read by the level, quality and name
/// columns; `None` = not cached.
#[derive(Clone, Copy, Debug)]
pub struct AuctionItem {
    /// `obj+0x3c`, the required level, at least 1 (column 0). Signed.
    pub f3c: i32,
    /// `obj+0x1c`, the quality (column 1). Signed.
    pub f1c: i32,
    /// `obj+0x8`, the name pointer (column 6).
    pub name8: u32,
}

/// The auction list comparator `0x4cd940`, switched on `mode` (its third argument, `0..=7`):
/// 0 level (`obj+0x3c`, at least 1), 1 quality (`obj+0x1c`, higher first), 2 bid (`rec+0x40`, else
/// `+0x28`), 3 deadline (`rec+0x34`, soonest first), 4 buyout (`rec+0x30`, highest first),
/// 5 status (2 for the local player's bid, 1 for another's, 0 for none; your bid first), 6 item
/// name (`obj+0x8`), 7 seller (the owner's cached name through `0x55f080(rec+0x20, +0x24)`).
/// An unknown column or an equal key gives 0.
#[allow(clippy::too_many_arguments)]
pub fn sortcmp_4cd940(
    mode: u32,
    a: &AuctionRec,
    b: &AuctionRec,
    oa: Option<AuctionItem>,
    ob: Option<AuctionItem>,
    note_a: Option<u32>,
    note_b: Option<u32>,
    local_guid_lo: u32,
    local_guid_hi: u32,
    collate: &mut dyn FnMut(u32, u32) -> i32,
) -> i32 {
    match mode {
        0 => {
            let (oa, ob) = match (oa, ob) {
                (Some(oa), Some(ob)) => (oa, ob),
                _ => return 0, // either GUID unresolved ⇒ 0
            };
            let ca = oa.f3c.max(1);
            let cb = ob.f3c.max(1);
            if ca == cb {
                return 0;
            }
            // `(cb <= ca) * 2 - 1`.
            if cb <= ca {
                1
            } else {
                -1
            }
        }
        1 => {
            if let (Some(oa), Some(ob)) = (oa, ob) {
                if oa.f1c != ob.f1c {
                    // `(objA+0x1c < objB+0x1c)*2-1` — `objA<objB ⇒ +1`.
                    return if oa.f1c < ob.f1c { 1 } else { -1 };
                }
            }
            0
        }
        2 => {
            let ka = if a.f40 == 0 { a.f28 } else { a.f40 };
            let kb = if b.f40 == 0 { b.f28 } else { b.f40 };
            if ka != kb {
                // `(kb <= ka)*2-1` — ascending.
                return if kb <= ka { 1 } else { -1 };
            }
            0
        }
        3 => {
            if a.f34 != b.f34 {
                // `(-(A<B) & 0xfffffffe) + 1` — ascending.
                return if a.f34 < b.f34 { -1 } else { 1 };
            }
            0
        }
        4 => {
            if a.f30 != b.f30 {
                // `(-(A<B) & 2) - 1` — descending.
                return if a.f30 < b.f30 { 1 } else { -1 };
            }
            0
        }
        5 => {
            // online-rank: self(GUID == local player)=2, other(nonzero)=1, empty=0; descending.
            let rank = |lo: u32, hi: u32| -> i32 {
                if lo == local_guid_lo && hi == local_guid_hi {
                    2
                } else if lo != 0 || hi != 0 {
                    1
                } else {
                    0
                }
            };
            let ra = rank(a.f38, a.f3c);
            let rb = rank(b.f38, b.f3c);
            if ra == rb {
                return 0;
            }
            // `(rankA < rankB) * 2 - 1`: the lower rank sorts later.
            if ra < rb {
                1
            } else {
                -1
            }
        }
        6 => {
            if let (Some(oa), Some(ob)) = (oa, ob) {
                return collate(oa.name8, ob.name8);
            }
            0
        }
        7 => {
            // note via 0x55f080; null-a sorts last.
            match (note_a, note_b) {
                (None, nb) => nb.is_some() as i32, // `(noteB != 0)`
                (Some(_), None) => -1,             // `0xffffffff`
                (Some(na), Some(nb)) => collate(na, nb),
            }
        }
        _ => 0,
    }
}

// `CompareGuildMembers` (`0x4d0d50`): `OnGuildRoster` (`0x4d0ad0`), `SetGuildShowOffline` (`0x4d0f70`)
// and `PromoteGuildSortKey` (`0x4d0fb0`) sort the array at `[0xb72704]` of `[0xb73118]` four-byte
// elements with it, and it walks the live `{column, direction}` table at `0xb72680`.

/// A guild member record, read through the live column table.
#[derive(Clone, Copy, Debug)]
pub struct GuildMemberRec {
    /// `+0x38` (col 1, signed `setge`).
    pub f38: i32,
    /// `+0x40` (col 3 index into `0xc0e048`, name `obj+0x2c`). Signed index.
    pub f40: i32,
    /// `+0x3c` (col 4 index into `0xc0def4`, name `obj+0x14`). Signed index.
    pub f3c: i32,
    /// `+0x44` (col 0, unsigned).
    pub f44: u32,
    /// `+0x48` (col 6 presence + the pre-gate). Unsigned.
    pub f48: u32,
    /// `+0x150` (col 6 float, compared `<=`).
    pub f150: f32,
    /// `+0x8` name pointer (col 2, record-direct).
    pub name8: u32,
    /// `+0x4e` name pointer (col 7, `0x4d09b0` = `rec+0x4e`).
    pub name4e: u32,
}

/// `CompareGuildMembers` (`0x4d0d50`). A pre-gate on `+0x48` (only when `0xb73124 == 0`, show offline
/// off) puts every offline member last; then a fixed walk of the live `[(col, dir); 8]` table at
/// `0xb72680`: the first column with a nonzero result wins, negated when its `dir` is set; all equal
/// gives 0. `obj3` and `obj4` are the zone name (`0xc0e048[+0x40]`) and class name
/// (`0xc0def4[+0x3c]`) pointers, `None` when the row is missing, which skips the column. Column 5
/// and any column above 7 order nothing.
#[allow(clippy::too_many_arguments)]
pub fn sortcmp_4d0d50(
    a: &GuildMemberRec,
    b: &GuildMemberRec,
    table: &[(u32, u32); 8],
    pregate_active: bool, // true ⇔ 0xb73124 == 0
    obj3_a: Option<u32>,  // 0xc0e048[a.f40] -> name+0x2c (col 3)
    obj3_b: Option<u32>,
    obj4_a: Option<u32>, // 0xc0def4[a.f3c] -> name+0x14 (col 4)
    obj4_b: Option<u32>,
    collate: &mut dyn FnMut(u32, u32) -> i32,
) -> i32 {
    // Pre-gate: when active and exactly one `+0x48` is zero, that side sorts last (`+0x48==0 ⇒ +1`).
    if pregate_active && ((a.f48 == 0) != (b.f48 == 0)) {
        return if a.f48 == 0 { 1 } else { -1 };
    }
    for &(col, dir) in table {
        let dir = dir != 0;
        // `Some(r)` returns (after dir); `None` continues to the next column.
        let r: Option<i32> = match col {
            0 => {
                if a.f44 == b.f44 {
                    None
                } else {
                    // `(-(recB44 < recA44) & 0xfffffffe) + 1`.
                    Some(if b.f44 < a.f44 { -1 } else { 1 })
                }
            }
            1 => {
                if a.f38 == b.f38 {
                    None
                } else {
                    // `setge (recA38 >= recB38)` — they differ here.
                    Some(if a.f38 > b.f38 { 1 } else { -1 })
                }
            }
            2 => {
                let r = collate(a.name8, b.name8);
                (r != 0).then_some(r)
            }
            3 => match (obj3_a, obj3_b) {
                (Some(ka), Some(kb)) => {
                    let r = collate(ka, kb);
                    (r != 0).then_some(r)
                }
                _ => None,
            },
            4 => match (obj4_a, obj4_b) {
                (Some(ka), Some(kb)) => {
                    let r = collate(ka, kb);
                    (r != 0).then_some(r)
                }
                _ => None,
            },
            6 => {
                if a.f48 != 0 && b.f48 != 0 {
                    None // both present ⇒ next column
                } else if a.f48 == b.f48 {
                    // both zero: `(recB_f150 <= recA_f150) ? 1 : -1` (NaN ⇒ -1); always returns.
                    return neg_if(dir, if b.f150 <= a.f150 { 1 } else { -1 });
                } else {
                    // exactly one zero: `(recA+0x48 == 0) ? +1 : -1`; always returns.
                    return neg_if(dir, if a.f48 == 0 { 1 } else { -1 });
                }
            }
            7 => {
                let r = collate(a.name4e, b.name4e);
                (r != 0).then_some(r)
            }
            _ => None, // col 5 and any col > 7
        };
        if let Some(r) = r {
            return neg_if(dir, r);
        }
    }
    0
}

// `0x4d3070`: the comparator that `0x4d2ed0` passes to `qsort` (`push 0x4d3070` at `0x4d2fd9`).

/// A record of the list `0x4d3070` orders.
#[derive(Clone, Copy, Debug)]
pub struct Rec4d3070 {
    /// `+0x0`: a zero sentinel and the `0xc0d92c` name-cache index. Signed.
    pub f0: i32,
    /// `+0x4`: the key matched against the order list at `0xb73170`.
    pub f4: u32,
    /// `+0x8`: the presence flag.
    pub f8: u32,
    /// `+0xc`: a flag; a difference gives `(recA+0xc != 0) * 2 - 1`.
    pub fc: u32,
}

/// Last index in `ids` whose entry equals `key`, default 0 (mirrors the binary's priority scan:
/// each match overwrites the position, unfound ⇒ 0).
fn find_pos(ids: &[u32], key: u32) -> i32 {
    let mut pos = 0i32;
    for (i, &id) in ids.iter().enumerate() {
        if id == key {
            pos = i as i32;
        }
    }
    pos
}

/// `0x4d3070`. Cascade: presence `+0x8` -> index of `+0x4` in the order list `0xb73170` (ascending)
/// -> `+0x0 == 0` (a zero sorts first) -> flag `+0xc` -> the name (`0xc0d92c[+0x0]`, `obj+0xc`,
/// `0x64a4c0`).
pub fn sortcmp_4d3070(
    a: &Rec4d3070,
    b: &Rec4d3070,
    prio_ids: &[u32],
    oa: Option<u32>, // 0xc0d92c[a.f0] -> name+0xc
    ob: Option<u32>,
    collate: &mut dyn FnMut(u32, u32) -> i32,
) -> i32 {
    let pa = a.f8 != 0;
    let pb = b.f8 != 0;
    if !pa && !pb {
        return 0;
    }
    if pa != pb {
        // `(recA+0x8 == 0)*2-1`.
        return if a.f8 == 0 { 1 } else { -1 };
    }
    // both present
    let posa = find_pos(prio_ids, a.f4);
    let posb = find_pos(prio_ids, b.f4);
    if posa != posb {
        // `setge (posA >= posB)` — ascending by priority index.
        return if posb < posa { 1 } else { -1 };
    }
    if a.f0 == 0 {
        return -1;
    }
    if b.f0 == 0 {
        return 1;
    }
    if a.fc != b.fc {
        // `(recA+0xc != 0)*2-1`.
        return if a.fc != 0 { 1 } else { -1 };
    }
    match (oa, ob) {
        (Some(ka), Some(kb)) => collate(ka, kb),
        _ => 0, // either name-cache miss ⇒ 0
    }
}

// `CompareReputationHeaders` (`0x4d5dc0`): `0x4d5c40` sorts the header objects with it (`0x4d5cc9`).

/// A reputation panel header record (8 bytes at `0xb736c0[i]`).
#[derive(Clone, Copy, Debug)]
pub struct ReputationHeader {
    /// `+0x0` — the header key: a `Faction.dbc` id, or the synthetic `0` / `-1`. Also the
    /// `0xc0dd50` id-map index. Signed.
    pub f0: i32,
    /// `+0x4` — child count (recomputed by `0x4d5c40` phase 1).
    pub f4: u32,
}

/// `CompareReputationHeaders` (`0x4d5dc0`). Cascade, as which appears first on screen: a header with
/// children (`+0x4 != 0`) first; with neither key in `Faction.dbc`, the larger raw key `+0x0` first
/// (`idxA < idxB` gives +1, so 0 precedes -1); with both, the localized name (`row+0x4c`,
/// `0x64a4c0`) ascending; otherwise the one with a row first.
pub fn sortcmp_4d5dc0(
    a: &ReputationHeader,
    b: &ReputationHeader,
    oa: Option<u32>, // 0xc0dd50[a.f0] -> name+0x4c
    ob: Option<u32>,
    collate: &mut dyn FnMut(u32, u32) -> i32,
) -> i32 {
    let pa = a.f4 != 0;
    let pb = b.f4 != 0;
    if !pa && !pb {
        return 0;
    }
    if pa != pb {
        // `(recA+0x4 == 0)*2-1`.
        return if a.f4 == 0 { 1 } else { -1 };
    }
    match (oa, ob) {
        (None, None) => {
            // both unmapped: `(idxA < idxB)*2-1` — descending by raw id.
            if a.f0 < b.f0 {
                1
            } else {
                -1
            }
        }
        (Some(ka), Some(kb)) => collate(ka, kb),
        // mapped-first: `(objA==null)*2-1` — A mapped ⇒ -1, A unmapped ⇒ +1.
        (Some(_), None) => -1,
        (None, Some(_)) => 1,
    }
}

// `CompareReputationRows` (`0x4d5e70`): `0x4d5c40` sorts the entries with it (`0x4d5da3`).

/// A reputation panel entry (20 bytes at `0xb73258[i]`; the list id at `+0x0` is not read).
#[derive(Clone, Copy, Debug)]
pub struct ReputationRow {
    /// `+0x4` — the `Faction.dbc` id (the `0xc0dd50` id-map index). Signed.
    pub f4: i32,
    /// `+0x8`: the header bit (bit 3 of the server's flag byte). Zero for a child.
    pub f8: u32,
    /// `+0xc`: the position of the entry's header, written after the header sort. Unsigned.
    pub fc: u32,
}

/// `CompareReputationRows` (`0x4d5e70`). A collapsed child is an entry with `+0x8 == 0`, a header
/// position below 32 and a clear bit in `special_mask` (`0x84a0a4` holds the expanded bits). When the
/// header positions differ, a collapsed child sorts last, otherwise the position ascends
/// (`catB <= catA` gives +1); with equal positions the header entry comes first (`+0x8`), then the
/// children by localized name (`row+0x4c`) ascending.
pub fn sortcmp_4d5e70(
    a: &ReputationRow,
    b: &ReputationRow,
    special_mask: u32, // 0x84a0a4
    oa: Option<u32>,   // 0xc0dd50[a.f4] -> name+0x4c
    ob: Option<u32>,
    collate: &mut dyn FnMut(u32, u32) -> i32,
) -> i32 {
    let special = |r: &ReputationRow| {
        r.f8 == 0 && r.fc < 0x20 && (1u32.wrapping_shl(r.fc) & special_mask) == 0
    };
    if a.fc != b.fc {
        let sa = special(a);
        let sb = special(b);
        if !sa {
            if !sb {
                // `(catB <= catA)*2-1` — they differ here.
                return if b.fc <= a.fc { 1 } else { -1 };
            }
            return -1; // B special only
        }
        // A special: `(uint)!specialB` — 1 if B not special, else 0.
        return (!sb) as i32;
    }
    // equal category
    if a.f8 != 0 || b.f8 != 0 {
        // `(recA+0x8 == 0)*2-1`.
        return if a.f8 == 0 { 1 } else { -1 };
    }
    match (oa, ob) {
        (Some(ka), Some(kb)) => collate(ka, kb),
        _ => 0,
    }
}

// `CompareTrainerRows` (`0x4d85c0`): the class and pet trainer rows (`systems/ui/trainerlist.md`).

/// A trainer row of the class comparator.
#[derive(Clone, Copy, Debug)]
pub struct TrainerRow {
    /// `+0x34`: the visible flag.
    pub f34: u32,
    /// `+0x4`: the group key matched against the header list at `0xb739c0`.
    pub f4: u32,
    /// `+0x0`: the spell id, which indexes the cache at `0xc0d788`; a negative value sorts first.
    /// Signed.
    pub f0: i32,
    /// `+0x14` (byte): the required level.
    pub byte14: u8,
    /// `+0x1c`: the required skill value (unsigned).
    pub f1c: u32,
}

/// The record a trainer row names (`0xc0d788[rec+0]`): two name pointers.
#[derive(Clone, Copy, Debug)]
pub struct TrainerSpellRec {
    /// `obj+0x1e0`: the name.
    pub name1e0: u32,
    /// `obj+0x204`: the rank text.
    pub name204: u32,
}

/// `CompareTrainerRows` (`0x4d85c0`, collator `0x64a480`). Cascade: visible (`+0x34`) -> position of
/// `+0x4` in the header list (ascending) -> a negative `+0x0` first -> the spell must resolve ->
/// level `+0x14` -> skill value `+0x1c` -> name `+0x1e0`, then rank text `+0x204`.
pub fn sortcmp_4d85c0(
    a: &TrainerRow,
    b: &TrainerRow,
    order_ids: &[u32],
    oa: Option<TrainerSpellRec>, // 0xc0d788[a.f0]
    ob: Option<TrainerSpellRec>,
    collate: &mut dyn FnMut(u32, u32) -> i32,
) -> i32 {
    let pa = a.f34 != 0;
    let pb = b.f34 != 0;
    if !pa && !pb {
        return 0;
    }
    if pa != pb {
        return if pa { -1 } else { 1 };
    }
    // both valid: order index
    let posa = find_pos(order_ids, a.f4);
    let posb = find_pos(order_ids, b.f4);
    if posa != posb {
        // `(posB < posA)*2-1` — ascending by order index.
        return if posb < posa { 1 } else { -1 };
    }
    // sign gate (the +0 field, also the name-cache index)
    if a.f0 < 0 || b.f0 < 0 {
        // `(-1 < idxA)*2-1` — A non-negative ⇒ +1, A negative ⇒ -1.
        return if a.f0 >= 0 { 1 } else { -1 };
    }
    // name-cache resolve: objA miss ⇒ 0; objA mapped & objB miss ⇒ 0.
    let (oa, ob) = match (oa, ob) {
        (Some(oa), Some(ob)) => (oa, ob),
        _ => return 0,
    };
    if a.byte14 != b.byte14 {
        // `(-(recB_byte < recA_byte) & 2) - 1`.
        return if b.byte14 < a.byte14 { 1 } else { -1 };
    }
    if a.f1c != b.f1c {
        // `(-(recB+0x1c < recA+0x1c) & 2) - 1` (unsigned).
        return if b.f1c < a.f1c { 1 } else { -1 };
    }
    let c = collate(oa.name1e0, ob.name1e0);
    if c != 0 {
        return c;
    }
    collate(oa.name204, ob.name204)
}

// `CompareTradeskillTrainerRows` (`0x4d8760`): the tradeskill trainer rows.

/// A trainer row of the tradeskill comparator.
#[derive(Clone, Copy, Debug)]
pub struct TradeskillTrainerRow {
    /// `+0x34` — valid/present gate.
    pub f34: u32,
    /// `+0x4`: the group key, compared directly.
    pub f4: u32,
    /// `+0x0` — sign key AND the `0xc0d788` name-cache index. Signed.
    pub f0: i32,
    /// `+0x1c` — the secondary key (unsigned).
    pub f1c: u32,
}

/// `CompareTradeskillTrainerRows` (`0x4d8760`, collator `0x64a480`). Cascade: visible (`+0x34`) ->
/// group key `+0x4` (`recB < recA` gives +1) -> a negative `+0x0` first -> the spell must resolve ->
/// skill value `+0x1c` -> name `+0x1e0`.
pub fn sortcmp_4d8760(
    a: &TradeskillTrainerRow,
    b: &TradeskillTrainerRow,
    oa: Option<u32>, // 0xc0d788[a.f0] -> name+0x1e0
    ob: Option<u32>,
    collate: &mut dyn FnMut(u32, u32) -> i32,
) -> i32 {
    let pa = a.f34 != 0;
    let pb = b.f34 != 0;
    if !pa && !pb {
        return 0;
    }
    if pa != pb {
        return if pa { -1 } else { 1 };
    }
    // both valid
    if a.f4 != b.f4 {
        // `(recB+0x4 < recA+0x4)*2-1`.
        return if b.f4 < a.f4 { 1 } else { -1 };
    }
    if a.f0 < 0 || b.f0 < 0 {
        // `(-1 < idxA)*2-1`.
        return if a.f0 >= 0 { 1 } else { -1 };
    }
    let (ka, kb) = match (oa, ob) {
        (Some(ka), Some(kb)) => (ka, kb),
        _ => return 0, // either name-cache miss ⇒ 0
    };
    if a.f1c != b.f1c {
        // `(-(recB+0x1c < recA+0x1c) & 2) - 1` (unsigned).
        return if b.f1c < a.f1c { 1 } else { -1 };
    }
    collate(ka, kb)
}

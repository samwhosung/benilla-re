//! `AutoLootAll` (`0x4c1fa0`) against the exe, run whole (`systems/ui/loot.md#auto-loot` and
//! `#window-state-after-the-sweep`).
//!
//! The test compares what the sweep calls, in order, and what it stores. Every callee but four
//! is stubbed, logs its address and arguments, and answers what the test chooses (the objects
//! for the active player and the loot source, the item records). The four that run for real are
//! `ClntObjMgrGetActivePlayer` (`0x468550`) over a manager block the test writes,
//! `GetLootItemRollState` (`0x5ebce0`) over the wire table `0xc4d4e8`, `SetAutoLoot` (`0x5df460`)
//! with 0, and `LootHasContents` (`0x4c2a70`) over the loot cells. The rows, the money cell and
//! the player's auto-loot byte are read back after each call. The money line's callees are
//! stubbed and logged by address only: the strings they build are not compared.

use benilla_re_difftest::{Emu, RegisterX86, StubIo, Wow};
use benilla_re_ui::auto_loot::{
    sweep, Call, Row, World, ADD_CHAT_LINE, COIN_SOUND, END_LOOT, EVENT_LOOT_BIND_CONFIRM,
    EVENT_LOOT_OPENED, GET_RECORD, GET_TEXT, ITEM_SOUND, OBJECT_PTR, SEND_AUTOSTORE,
    SEND_LOOT_MONEY, SIGNAL_EVENT, SIGNAL_EVENT2, SPLIT_COPPER, STR_COPY, STR_PRINTF,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

const SWEEP: u64 = 0x4c1fa0;
const PENDING: u64 = 0xb71b44;
const SOURCE_GUID: u64 = 0xb71b48;
const MONEY: u64 = 0xb71ba0;
const SHOWN: u64 = 0xb71bc4;
const ROWS: u64 = 0xb71968;
const BIND_ROW: u64 = 0x847cec;
/// The first party member's GUID, and the raid member count.
const PARTY: u64 = 0xbc6f48;
const RAID: u64 = 0xb713e0;
/// The wire table: 24 bytes per slot, the slot type at `+0x14`.
const WIRE_TABLE: u64 = 0xc4d4e8;
/// The manager pointer `0x468550` reads, and the GUID at `+0xc0` of what it points to.
const MGR_PTR: u64 = 0xb41414;
/// The player's auto-loot byte.
const BYTE_OFFSET: u64 = 0x1d30;

type Log = Rc<RefCell<Vec<Call>>>;

fn ecx(io: &StubIo) -> u32 {
    io.reg(RegisterX86::ECX) as u32
}

fn edx(io: &StubIo) -> u32 {
    io.reg(RegisterX86::EDX) as u32
}

fn logged(
    e: &mut Emu,
    log: &Log,
    addr: u32,
    pop: u32,
    args: fn(&StubIo) -> [u32; 4],
    ret: impl Fn(&StubIo) -> u32 + 'static,
) {
    let log = log.clone();
    e.stub_fn(u64::from(addr), pop, move |io| {
        log.borrow_mut().push(Call {
            addr,
            args: args(io),
        });
        ret(io)
    });
}

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s >> 8
}

/// The item pool: an entry's record is the same wherever a row names it.
const POOL: [(u32, u32, u32, u32); 6] = [
    // entry, bonding, quality, sound
    (0x101, 0, 1, 11),
    (0x102, 1, 1, 12),
    (0x103, 1, 2, 13),
    (0x104, 1, 4, 14),
    (0x105, 2, 3, 15),
    (0x106, 0, 5, 16),
];

fn random_world(s: &mut u32, player: u32, source: u32) -> World {
    let pick = |s: &mut u32, n: u32| lcg(s) % n;
    let mut rows = [None; 16];
    // The wire slot a row carries is not its array position (the copier packs the rows): a
    // shuffled slot per position tells the bind confirmation's position from the slot sent.
    let mut slots: [u8; 16] = std::array::from_fn(|i| i as u8);
    for i in (1..16).rev() {
        slots.swap(i, pick(s, i as u32 + 1) as usize);
    }
    let count = match pick(s, 6) {
        0 => 0,
        n => n + pick(s, 6),
    };
    for _ in 0..count {
        let at = pick(s, 16) as usize;
        let (entry, bonding, quality, sound) = POOL[pick(s, 6) as usize];
        rows[at] = Some(Row {
            entry,
            wire_slot: slots[at],
            slot_type: if pick(s, 5) == 0 { pick(s, 4) + 1 } else { 0 },
            bonding,
            quality,
            sound,
        });
    }
    World {
        pending: if pick(s, 6) == 0 { 1 + pick(s, 3) } else { 0 },
        active: 0x0000_0004_0000_0abc + u64::from(pick(s, 2)),
        player: (pick(s, 8) != 0).then_some(player),
        money: [0, 0, -1, 1, 100, 250, 10_000, 10_001, 10_100, 123_456][pick(s, 10) as usize],
        shown: pick(s, 2),
        in_party: pick(s, 3) == 0,
        in_raid: pick(s, 4) == 0,
        source_guid: 0xf130_0000_0000_5eed | (u64::from(pick(s, 4)) << 32),
        source: (source != 0) && pick(s, 8) != 0,
        rows,
        flag: pick(s, 2),
    }
}

#[test]
fn auto_loot_sweep_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let log: Log = Rc::default();
    let player_cell = Rc::new(Cell::new(0u32));
    let source_cell = Rc::new(Cell::new(0u32));
    let records: Rc<RefCell<HashMap<u32, u32>>> = Rc::default();

    // The objects the two lookups return, by mask.
    let (pc, sc) = (player_cell.clone(), source_cell.clone());
    logged(
        &mut e,
        &log,
        OBJECT_PTR,
        3,
        |io| [ecx(io), io.arg(0), io.arg(1), io.arg(2)],
        move |io| if ecx(io) == 0x10 { pc.get() } else { sc.get() },
    );
    logged(
        &mut e,
        &log,
        SIGNAL_EVENT,
        0,
        |io| [ecx(io), 0, 0, 0],
        |_| 0,
    );
    logged(
        &mut e,
        &log,
        SIGNAL_EVENT2,
        0,
        |io| [io.arg(0), io.arg(1), io.arg(2), 0],
        |_| 0,
    );
    logged(
        &mut e,
        &log,
        SEND_LOOT_MONEY,
        0,
        |io| [ecx(io), 0, 0, 0],
        |_| 0,
    );
    logged(
        &mut e,
        &log,
        SEND_AUTOSTORE,
        1,
        |io| [ecx(io), io.arg(0), 0, 0],
        |_| 0,
    );
    let recs = records.clone();
    logged(
        &mut e,
        &log,
        GET_RECORD,
        5,
        |io| [ecx(io), io.arg(0), io.arg(1), io.arg(2)],
        move |io| recs.borrow()[&io.arg(0)],
    );
    logged(
        &mut e,
        &log,
        ITEM_SOUND,
        0,
        |io| [ecx(io), edx(io), 0, 0],
        |_| 0,
    );
    logged(&mut e, &log, COIN_SOUND, 0, |io| [ecx(io), 0, 0, 0], |_| 0);
    logged(
        &mut e,
        &log,
        END_LOOT,
        1,
        |io| [ecx(io) & 0xff, edx(io) & 0xff, io.arg(0), 0],
        |_| 0,
    );
    // The money line: by address only.
    // SplitCopper(amount in ecx, out in edx): gold, silver and copper, so the line's string
    // choices run; their effects are not compared.
    let l = log.clone();
    e.stub_fn(u64::from(SPLIT_COPPER), 0, move |io| {
        l.borrow_mut().push(Call {
            addr: SPLIT_COPPER,
            args: [0; 4],
        });
        let (money, out) = (ecx(io), u64::from(edx(io)));
        io.write_u32(out, money / 10000);
        io.write_u32(out + 4, money % 10000 / 100);
        io.write_u32(out + 8, money % 100);
        0
    });
    logged(&mut e, &log, GET_TEXT, 1, |_| [0; 4], |_| 0x1000);
    logged(&mut e, &log, STR_COPY, 3, |_| [0; 4], |_| 0);
    logged(&mut e, &log, STR_PRINTF, 0, |_| [0; 4], |_| 0);
    logged(&mut e, &log, ADD_CHAT_LINE, 10, |_| [0; 4], |_| 0);

    // Memory the sweep reads: the player object, the manager, one record per pool item.
    let player = e.alloc(0x2000) as u32;
    let source = e.alloc(0x40) as u32;
    let mgr = e.alloc(0x100);
    for &(entry, bonding, quality, sound) in &POOL {
        let rec = e.alloc(0x200) as u32;
        e.write_u32(u64::from(rec) + 0x194, bonding);
        e.write_u32(u64::from(rec) + 0x1c, quality);
        e.write_u32(u64::from(rec) + 0x18, sound);
        records.borrow_mut().insert(entry, rec);
    }

    let mut s = 0x1007_5eed_u32;
    let mut cases = 0u32;
    let mut returned_pending = 0u32;
    let mut no_player = 0u32;
    let mut coin_lines = 0u32;
    let mut bind_confirms = 0u32;
    let mut skipped_bind = 0u32;
    let mut stored = 0u32;
    let mut opened = 0u32;
    let mut flagged = 0u32;
    let mut closed_empty = 0u32;
    let mut taken_coin = 0u32;
    let mut sources_missing = 0u32;
    let mut other_types = 0u32;
    let mut slot_sent_apart = 0u32;
    let mut bind_apart = 0u32;
    for case in 0..4000u32 {
        let w = random_world(&mut s, player, source);
        // The cells.
        e.write_u32(PENDING, w.pending);
        e.write(SOURCE_GUID, &w.source_guid.to_le_bytes());
        e.write_u32(MONEY, w.money as u32);
        e.write_u32(SHOWN, w.shown);
        e.write(PARTY, &u64::from(w.in_party).to_le_bytes());
        e.write_u32(RAID, u32::from(w.in_raid));
        e.write_u32(BIND_ROW, u32::MAX);
        let mut rows = [0u8; 16 * 0x1c];
        for (i, row) in w.rows.iter().enumerate() {
            if let Some(r) = row {
                let at = i * 0x1c;
                rows[at + 4..at + 8].copy_from_slice(&r.entry.to_le_bytes());
                rows[at + 8..at + 12].copy_from_slice(&(0x2000 + r.entry).to_le_bytes());
                rows[at + 12..at + 16].copy_from_slice(&1u32.to_le_bytes());
                rows[at + 0x18] = r.wire_slot;
                e.write_u32(WIRE_TABLE + 24 * u64::from(r.wire_slot) + 0x14, r.slot_type);
            }
        }
        e.write(ROWS, &rows);
        e.write(mgr + 0xc0, &w.active.to_le_bytes());
        e.write_u32(MGR_PTR, mgr as u32);
        let byte_before = (lcg(&mut s) & 1) as u8;
        e.write(u64::from(player) + BYTE_OFFSET, &[byte_before]);
        player_cell.set(w.player.unwrap_or(0));
        source_cell.set(if w.source { source } else { 0 });
        let money_before = e.read(MONEY, 4);

        log.borrow_mut().clear();
        e.call(SWEEP, w.flag, 0, &[]);
        let (want, stores) = sweep(&w);
        let got = log.borrow().clone();
        assert_eq!(got, want, "case {case}: {w:?}");

        // The sweep stores no row and not the money cell; it writes the byte, the bind row and
        // the shown cell as the model says.
        assert_eq!(e.read(ROWS, rows.len()), rows, "case {case}: rows");
        assert_eq!(e.read(MONEY, 4), money_before, "case {case}: money");
        let byte = e.read(u64::from(player) + BYTE_OFFSET, 1)[0];
        assert_eq!(
            byte,
            if stores.byte_cleared { 0 } else { byte_before },
            "case {case}: auto-loot byte"
        );
        assert_eq!(
            e.read_u32(BIND_ROW),
            stores.bind_row.unwrap_or(u32::MAX),
            "case {case}: bind row"
        );
        assert_eq!(
            e.read_u32(SHOWN),
            if stores.shown_set { 1 } else { w.shown },
            "case {case}: shown cell"
        );

        cases += 1;
        let has = |a: u32| got.iter().any(|c| c.addr == a);
        returned_pending += u32::from(w.pending != 0);
        no_player += u32::from(w.pending == 0 && w.player.is_none());
        coin_lines += u32::from(has(ADD_CHAT_LINE));
        bind_confirms += u32::from(has(SIGNAL_EVENT2));
        stored += u32::from(has(SEND_AUTOSTORE));
        opened += u32::from(
            got.iter()
                .any(|c| c.addr == SIGNAL_EVENT && c.args[0] == EVENT_LOOT_OPENED),
        );
        flagged += u32::from(
            w.pending == 0
                && w.player.is_some()
                && w.flag != 0
                && stores.byte_cleared
                && !has(END_LOOT),
        );
        closed_empty += u32::from(has(END_LOOT));
        taken_coin += u32::from(w.money == -1 && has(SEND_LOOT_MONEY));
        sources_missing += u32::from(stores.byte_cleared && !w.source);
        other_types += u32::from(w.rows.iter().flatten().any(|r| r.slot_type != 0));
        let confirmable = w
            .rows
            .iter()
            .flatten()
            .filter(|r| r.slot_type == 0 && r.bonding == 1 && r.quality >= 2)
            .count();
        skipped_bind += u32::from(confirmable > 1 && stores.bind_row.is_some());
        slot_sent_apart += u32::from(
            stores.byte_cleared
                && w.rows.iter().enumerate().any(|(i, r)| {
                    r.is_some_and(|r| {
                        r.slot_type == 0
                            && !(r.bonding == 1 && r.quality >= 2)
                            && usize::from(r.wire_slot) != i
                    })
                }),
        );
        bind_apart += u32::from(
            stores
                .bind_row
                .is_some_and(|i| w.rows[i as usize].is_some_and(|r| u32::from(r.wire_slot) != i)),
        );
        let _ = EVENT_LOOT_BIND_CONFIRM;
    }
    eprintln!(
        "auto_loot_sweep: {cases} cases: {returned_pending} return on pending fetches, \
         {no_player} without the player, {coin_lines} print the money line, {bind_confirms} \
         confirm a bind, {skipped_bind} with a second bind row left, {stored} send rows, \
         {opened} signal LOOT_OPENED, {flagged} swept with the flag set, {closed_empty} close \
         empty, {taken_coin} re-request a taken coin row, {sources_missing} with the source gone, \
         {other_types} with rows of another type"
    );
    for (name, n) in [
        ("pending", returned_pending),
        ("no player", no_player),
        ("money line", coin_lines),
        ("bind confirm", bind_confirms),
        ("second bind row", skipped_bind),
        ("sent rows", stored),
        ("LOOT_OPENED", opened),
        ("flag set", flagged),
        ("closed empty", closed_empty),
        ("taken coin row", taken_coin),
        ("source gone", sources_missing),
        ("other types", other_types),
        ("slot sent apart from its position", slot_sent_apart),
        ("bind position apart from its slot", bind_apart),
    ] {
        assert!(n > 0, "no case reached: {name}");
    }
}

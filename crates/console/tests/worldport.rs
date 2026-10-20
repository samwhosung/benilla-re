//! The `worldport` command (`CCommand_WorldTeleport`, `0x404dc0`) run whole in the emulator
//! against `worldport` (`systems/console/commands.md`, "World port command" and "World port
//! message").
//!
//! The handler runs with its own `SStrTokenize`, `SStrChr`, `SStrToUnsigned`, `SStrToFloat` (with
//! its table builder and the C runtime's `pow`), `CDataStore::PutUInt32` and `PutFloat`, and the
//! store's destructor up to the free of its buffer, `SMemFree`, which is a stub. Replaced by stubs
//! are the callees that reach other systems: the active player lookup (`0x468550`, `0x468460`), the player's position and facing (the vtable slots
//! `0x5f1f10` and `0x5f1f30`), `ConsoleWrite` and `ConsoleWriteA`, `OsGetAsyncTimeMs`,
//! `NetClient::Send`, and the allocator, which is an arena. The message is read at the send, byte
//! for byte, and the stubs' calls are compared in order with their arguments.
//!
//! The reimplementation takes its powers of ten from the exe's `pow`, called through a trampoline,
//! as `sstrtofloat_whole` does, so `pow` is not under test; the inputs keep every step within the
//! powers that oracle holds (`docs/METHOD.md#the-harness`: `fyl2x` and `f2xm1` run in the host's
//! `double`).

mod common;

use benilla_re_console::worldport::{worldport, Maps, Outcome, Player, BAD_WORLD, USAGE};
use benilla_re_difftest::{Emu, Wow};
use benilla_re_storm::{pow10_table, str_to_float};
use common::next;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

const POW_RANGE: i32 = 100;

const GET_ACTIVE_PLAYER: u64 = 0x468550;
const OBJECT_PTR: u64 = 0x468460;
const POSITION_SLOT: u64 = 0x5f1f10;
const FACING_SLOT: u64 = 0x5f1f30;
const CONSOLE_WRITE: u64 = 0x63cb50;
const CONSOLE_WRITE_A: u64 = 0x63cd00;
const OS_GET_ASYNC_TIME_MS: u64 = 0x42c010;
const NET_SEND: u64 = 0x5379a0;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const SMEM_FREE: u64 = 0x646430;
const SERR_SET_LAST_ERROR: u64 = 0x64e850;

const HOLDER: u64 = 0x8827c8;
const MAP_ID_MAP: u64 = 0xc0daa8;
const MAP_LARGEST_ID: u64 = 0xc0daac;
const PLAYER_VTABLE_SLOTS: u64 = 0x14;

/// Ids with a `Map.dbc` row in the test's id map, which holds ids 0 to `LARGEST`.
const ROWS: [u32; 7] = [0, 1, 13, 30, 489, 530, 100];
const LARGEST: i32 = 530;
const CONNECTION: u32 = 0x00c0_ffee;

/// What a stub saw, in order, apart from the facing slot (a value stub that logs only in the
/// emulator's own call log).
#[derive(Debug, PartialEq)]
enum Seen {
    Lookup {
        mask: u32,
        line: u32,
    },
    Position {
        this: u32,
    },
    Text {
        text: Vec<u8>,
        class: u32,
    },
    Format {
        format: Vec<u8>,
        class: u32,
        arg: u32,
    },
    Send {
        connection: u32,
        bytes: Vec<u8>,
    },
}

/// The state the stubs read and the log they write.
struct World {
    player: Cell<u32>,
    position: Cell<[f32; 3]>,
    time: Cell<u32>,
    log: RefCell<Vec<Seen>>,
}

/// `pow(10.0, n)` as the exe's `pow` answers it, for `n` within ±`POW_RANGE`.
fn pow10_oracle(emu: &mut Emu) -> HashMap<i32, f64> {
    let ten = emu.alloc(8);
    emu.write(ten, &10f64.to_le_bytes());
    let code = emu.alloc(0x20);
    let mut c = vec![0xdd, 0x05];
    c.extend_from_slice(&(ten as u32).to_le_bytes());
    c.extend_from_slice(&[0xdb, 0x44, 0x24, 0x04]);
    let call_at = code + c.len() as u64;
    let rel = (0x73f940i64 - (call_at as i64 + 5)) as i32;
    c.push(0xe8);
    c.extend_from_slice(&rel.to_le_bytes());
    c.push(0xc3);
    emu.write(code, &c);
    (-POW_RANGE..=POW_RANGE)
        .map(|n| (n, emu.call_f64(code, 0, 0, &[n as u32])))
        .collect()
}

/// One argument line: a map token, then up to five more, each now and then quoted, joined by
/// delimiters, with a tail the command never reads.
fn line(s: &mut u32, maps: &[&str], nums: &[&str]) -> Vec<u8> {
    let joiners = [" ", "  ", "\t", "\r", "\n", " \t ", "\" \""];
    let count = (next(s) >> 8) % 7;
    let mut out = String::new();
    if next(s) >> 8 & 3 == 0 {
        out.push(' ');
    }
    for i in 0..count {
        let pool = if i == 0 { maps } else { nums };
        let tok = pool[(next(s) >> 8) as usize % pool.len()];
        if next(s) >> 8 & 7 == 0 && !tok.is_empty() {
            out.push('"');
            out.push_str(tok);
            out.push('"');
        } else {
            out.push_str(tok);
        }
        if i + 1 < count {
            out.push_str(joiners[(next(s) >> 8) as usize % joiners.len()]);
        }
    }
    if next(s) >> 8 & 7 == 0 {
        out.push_str(" 1 2 3 junk");
    }
    out.into_bytes()
}

#[test]
fn worldport_whole_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let pows = pow10_oracle(&mut emu);

    // The memory the handler reads: the player object with its vtable, the id map, the holder.
    let player_obj = emu.alloc(0x10);
    let vtable = emu.alloc(0x40);
    emu.write_u32(player_obj, vtable as u32);
    emu.write_u32(vtable + PLAYER_VTABLE_SLOTS, POSITION_SLOT as u32);
    emu.write_u32(vtable + PLAYER_VTABLE_SLOTS + 4, FACING_SLOT as u32);
    let facing_cell = emu.alloc(8);
    let id_map = emu.alloc(4 * (LARGEST as u64 + 1));
    for id in ROWS {
        emu.write_u32(id_map + 4 * u64::from(id), 0x1000 + id);
    }
    emu.write_u32(MAP_ID_MAP, id_map as u32);
    emu.write_u32(MAP_LARGEST_ID, LARGEST as u32);
    let holder = emu.alloc(0x28);
    emu.write_u32(holder + 0x24, CONNECTION);
    emu.write_u32(HOLDER, holder as u32);
    let mut has_row = vec![false; LARGEST as usize + 1];
    for id in ROWS {
        has_row[id as usize] = true;
    }

    // The allocator arena, and the stubs.
    let arena = emu.alloc(0x20000);
    emu.stub_alloc_sized(SMEM_ALLOC, arena, arena + 0x20000, 0, 4);
    emu.stub_realloc_sized(SMEM_REALLOC, 5);
    emu.stub(SMEM_FREE, 0, 4);
    emu.stub(SERR_SET_LAST_ERROR, 0, 1);
    let w = Rc::new(World {
        player: Cell::new(0),
        position: Cell::new([0.0; 3]),
        time: Cell::new(0),
        log: RefCell::new(Vec::new()),
    });
    emu.stub(GET_ACTIVE_PLAYER, 0x1234, 0);
    {
        let w = w.clone();
        emu.stub_fn(OBJECT_PTR, 3, move |io| {
            w.log.borrow_mut().push(Seen::Lookup {
                mask: io.reg(benilla_re_difftest::RegisterX86::ECX) as u32,
                line: io.arg(2),
            });
            w.player.get()
        });
    }
    {
        let w = w.clone();
        emu.stub_fn(POSITION_SLOT, 1, move |io| {
            let out = io.arg(0);
            for (i, v) in w.position.get().iter().enumerate() {
                io.write_u32(u64::from(out) + 4 * i as u64, v.to_bits());
            }
            w.log.borrow_mut().push(Seen::Position {
                this: io.reg(benilla_re_difftest::RegisterX86::ECX) as u32,
            });
            out
        });
    }
    emu.stub_f64(FACING_SLOT, facing_cell, 0);
    {
        let w = w.clone();
        emu.stub_fn(CONSOLE_WRITE, 0, move |io| {
            let text = io.read_cstr(io.reg(benilla_re_difftest::RegisterX86::ECX));
            let class = io.reg(benilla_re_difftest::RegisterX86::EDX) as u32;
            w.log.borrow_mut().push(Seen::Text { text, class });
            0
        });
    }
    {
        let w = w.clone();
        emu.stub_fn(CONSOLE_WRITE_A, 0, move |io| {
            let format = io.read_cstr(u64::from(io.arg(0)));
            w.log.borrow_mut().push(Seen::Format {
                format,
                class: io.arg(1),
                arg: io.arg(2),
            });
            0
        });
    }
    {
        let w = w.clone();
        emu.stub_fn(OS_GET_ASYNC_TIME_MS, 0, move |_| w.time.get());
    }
    {
        let w = w.clone();
        emu.stub_fn(NET_SEND, 1, move |io| {
            let store = u64::from(io.arg(0));
            let buffer = u64::from(io.read_u32(store + 4));
            let size = io.read_u32(store + 0x10) as usize;
            let bytes = io.read(buffer, size);
            w.log.borrow_mut().push(Seen::Send {
                connection: io.reg(benilla_re_difftest::RegisterX86::ECX) as u32,
                bytes,
            });
            1
        });
    }

    // The float reader: the exe's table and powers of ten.
    let missing = Cell::new(false);
    let pow10 = |n: i32| -> f64 {
        pows.get(&n).copied().unwrap_or_else(|| {
            missing.set(true);
            f64::NAN
        })
    };
    let table = pow10_table(pow10);
    let to_float = |t: &[u8]| str_to_float(t, &table, pow10);

    let maps_pool = [
        "0",
        "1",
        "13",
        "30",
        "489",
        "530",
        "100",
        "531",
        "2",
        "abc",
        "-1",
        "1x",
        "007",
        "4294967296",
        "4294967295",
        "2147483648",
        "2147483647",
        "99999999999",
        "",
    ];
    let nums_pool = [
        "0",
        "1",
        "-1",
        "5",
        "123.5",
        "-0.25",
        "1000000",
        "16000.75",
        "1e2",
        "abc",
        "12abc",
        "",
        "-",
        "0.1",
        "99999999999",
        "3.4e38",
        "1e39",
        "-1e39",
        "1,2",
        "0.000001",
        "45",
        "359.9",
        "0123456789012345678901234567890123456789012345678901234567890123456789",
    ];

    let args_at = emu.alloc(0x400);
    let positions = [
        [0.0f32, 0.0, 0.0],
        [-8949.95, -132.493, 83.5232],
        [1629.36, -4373.39, 35.5],
        [16000.5, -16000.5, 1.0e30],
    ];
    let facings = [0.0f32, 1.5, -3.0, 6.0];
    let reset = |emu: &mut Emu| {
        emu.reset_bump(arena);
        emu.write(arena, &vec![0u8; 0x4000]);
        emu.clear_calls();
        w.log.borrow_mut().clear();
    };
    let call_addrs = |emu: &Emu| -> Vec<u64> {
        emu.calls()
            .iter()
            .map(|c| c.addr)
            .filter(|a| {
                [
                    GET_ACTIVE_PLAYER,
                    OBJECT_PTR,
                    POSITION_SLOT,
                    FACING_SLOT,
                    CONSOLE_WRITE,
                    CONSOLE_WRITE_A,
                    OS_GET_ASYNC_TIME_MS,
                    NET_SEND,
                ]
                .contains(a)
            })
            .collect()
    };

    let mut s = 0x0404_dc01u32;
    let mut tally = [0usize; 4];
    let mut checked = 0;
    for case in 0..9000 {
        let text = line(&mut s, &maps_pool, &nums_pool);
        let present = case % 40 != 7;
        let unloaded = case % 53 == 11;
        let player = Player {
            position: positions[(next(&mut s) >> 8) as usize % positions.len()],
            facing: facings[(next(&mut s) >> 8) as usize % facings.len()],
        };
        let time = next(&mut s);
        w.player.set(if present { player_obj as u32 } else { 0 });
        w.position.set(player.position);
        w.time.set(time);
        emu.write(facing_cell, &f64::from(player.facing).to_le_bytes());
        emu.write_u32(
            MAP_LARGEST_ID,
            if unloaded { u32::MAX } else { LARGEST as u32 },
        );
        let mut z = text.clone();
        z.push(0);
        emu.write(args_at, &z);
        reset(&mut emu);

        missing.set(false);
        let maps = Maps {
            largest_id: if unloaded { -1 } else { LARGEST },
            has_row: &has_row,
        };
        let want = worldport(&text, present.then_some(&player), &maps, to_float);
        if missing.get() {
            continue;
        }
        let got = emu.call(0x404dc0, 0, args_at as u32, &[]);
        let what = format!("{:?}", String::from_utf8_lossy(&text));

        let mut calls = vec![GET_ACTIVE_PLAYER, OBJECT_PTR];
        let mut seen = vec![Seen::Lookup {
            mask: 0x10,
            line: 0x90,
        }];
        let ret = match want {
            Outcome::NoPlayer => {
                tally[0] += 1;
                1
            }
            Outcome::Usage => {
                tally[1] += 1;
                calls.extend([POSITION_SLOT, FACING_SLOT, CONSOLE_WRITE]);
                seen.push(Seen::Position {
                    this: player_obj as u32,
                });
                seen.push(Seen::Text {
                    text: USAGE.to_vec(),
                    class: 4,
                });
                0
            }
            Outcome::BadWorld(id) => {
                tally[2] += 1;
                calls.extend([POSITION_SLOT, FACING_SLOT, CONSOLE_WRITE_A]);
                seen.push(Seen::Position {
                    this: player_obj as u32,
                });
                seen.push(Seen::Format {
                    format: BAD_WORLD.to_vec(),
                    class: 3,
                    arg: id as u32,
                });
                0
            }
            Outcome::Teleport(t) => {
                tally[3] += 1;
                calls.extend([POSITION_SLOT, FACING_SLOT, OS_GET_ASYNC_TIME_MS, NET_SEND]);
                seen.push(Seen::Position {
                    this: player_obj as u32,
                });
                let mut bytes = Vec::new();
                for v in [8, time, t.map] {
                    bytes.extend_from_slice(&u32::to_le_bytes(v));
                }
                for v in t.position {
                    bytes.extend_from_slice(&v.to_bits().to_le_bytes());
                }
                bytes.extend_from_slice(&t.facing.to_bits().to_le_bytes());
                seen.push(Seen::Send {
                    connection: CONNECTION,
                    bytes,
                });
                1
            }
        };
        if !present && !matches!(want, Outcome::NoPlayer) {
            panic!("the model sends with no player: {what}");
        }
        assert_eq!(got, ret, "return: {what}");
        assert_eq!(call_addrs(&emu), calls, "calls: {what}");
        assert_eq!(*w.log.borrow(), seen, "arguments: {what}");
        checked += 1;
    }
    assert!(
        tally.iter().all(|&n| n > 100),
        "every outcome is exercised: {tally:?}"
    );

    // With no holder yet, the send makes one: 0x28 bytes under the allocator's name, its count
    // at +0x20 cleared, and the connection read from +0x24.
    emu.write_u32(HOLDER, 0);
    reset(&mut emu);
    w.player.set(player_obj as u32);
    w.position.set([1.0, 2.0, 3.0]);
    emu.write(facing_cell, &0.5f64.to_le_bytes());
    emu.write(args_at, b"1 4 5\0");
    emu.call(0x404dc0, 0, args_at as u32, &[]);
    let made = u64::from(emu.read_u32(HOLDER));
    assert_ne!(made, 0, "the holder is made");
    assert_eq!(emu.read_u32(made + 0x20), 0, "its count is cleared");
    let first = emu
        .calls()
        .iter()
        .find(|c| c.addr == SMEM_ALLOC)
        .expect("an allocation");
    assert_eq!(
        (first.args[0], first.args[1], first.args[3]),
        (0x28, 0x82e200, 0)
    );
    assert!(
        matches!(
            w.log.borrow().last(),
            Some(Seen::Send { connection: 0, .. })
        ),
        "the send reads +0x24 of the new holder"
    );
    eprintln!("worldport_whole_difftest: {checked} lines, outcomes {tally:?}");
}

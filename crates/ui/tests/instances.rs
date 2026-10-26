//! The instance messages' arithmetic against `WoW.exe` in the emulator
//! (`systems/ui/instances.md`):
//!
//! * `instances_format_duration`: `FormatDurationDhms` (`0x49e9d0`) whole, with `SStrPrintf`
//!   (`0x64a7f0`) stubbed to log its arguments; the four numbers it gets are compared.
//! * `instances_lockout_warning`: `HandleRaidInstanceMessagePacket` (`0x49e1c0`) whole, with
//!   `CDataStore::GetInt32` (`0x418e30`) feeding the three words, `FrameScript_GetText`
//!   (`0x703bf0`) answering a template per key and logging the ordinal, `SStrPrintf` filling
//!   `%s` and `%d` as `ISStrVPrintf` does for a result that fits, and `AddChatLine` (`0x49a870`)
//!   logging the line and its type. `SStrCopy` (`0x64a5a0`) runs as the client's.
//! * `instances_reserve_saved_list`: `ReserveRaidInstanceInfos` (`0x4a3370`) whole, with
//!   `SMemReAlloc` (`0x646320`), `SMemAlloc` (`0x6462e0`) and `SMemFree` (`0x646430`) stubbed.
//!
//! None of the stubs touches a value the compared results come from but through the arguments
//! the function passes it.
use std::cell::RefCell;
use std::rc::Rc;

use benilla_re_difftest::{Emu, RegisterX86, StubIo, Wow};
use benilla_re_ui::instances::{lockout_warning, reserve, split_duration, ArrayHeader};

const SSTRPRINTF: u64 = 0x64_a7f0;
const GETINT32: u64 = 0x41_8e30;
const GETTEXT: u64 = 0x70_3bf0;
const ADDCHATLINE: u64 = 0x49_a870;
const SMEMREALLOC: u64 = 0x64_6320;
const SMEMALLOC: u64 = 0x64_62e0;
const SMEMFREE: u64 = 0x64_6430;

const MAP_MAX: u64 = 0xc0_daac;
const MAP_INDEX: u64 = 0xc0_daa8;
const LOCALE: u64 = 0xc0_e080;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*s).rotate_right(8)
}

/// `%s` and `%d` into `[arg0]`, at most `arg1 - 1` bytes and a NUL, from the varargs at `arg3`.
fn mini_printf(io: &mut StubIo<'_, '_>) -> u32 {
    let dest = u64::from(io.arg(0));
    let size = io.arg(1) as usize;
    let fmt = io.read_cstr(u64::from(io.arg(2)));
    let mut out = Vec::new();
    let mut next_arg = 3;
    let mut i = 0;
    while i < fmt.len() {
        if fmt[i] == b'%' && i + 1 < fmt.len() {
            let v = io.arg(next_arg);
            next_arg += 1;
            match fmt[i + 1] {
                b's' => out.extend(io.read_cstr(u64::from(v))),
                b'd' => out.extend((v as i32).to_string().bytes()),
                other => panic!("unexpected spec %{}", other as char),
            }
            i += 2;
        } else {
            out.push(fmt[i]);
            i += 1;
        }
    }
    out.truncate(size.saturating_sub(1));
    out.push(0);
    io.write(dest, &out);
    (out.len() - 1) as u32
}

fn durations() -> Vec<i32> {
    let mut v = vec![
        0,
        1,
        -1,
        59,
        60,
        61,
        -59,
        -60,
        -61,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
    ];
    for (unit, limit) in [
        (60i64, 2 * 86_400),
        (3_600, 31 * 86_400),
        (86_400, 366 * 86_400),
    ] {
        let mut k = 0i64;
        while k * unit <= limit {
            for d in [-1i64, 0, 1] {
                for sign in [1i64, -1] {
                    v.push((sign * (k * unit + d)) as i32);
                }
            }
            k += 1;
        }
    }
    let mut s = 0x49e9_d000u32;
    for _ in 0..20_000 {
        v.push(next(&mut s) as i32);
    }
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn instances_format_duration() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let log: Rc<RefCell<Vec<[u32; 7]>>> = Rc::default();
    {
        let log = Rc::clone(&log);
        emu.stub_fn(SSTRPRINTF, 0, move |io| {
            let a: [u32; 7] = std::array::from_fn(|i| io.arg(i as u64));
            log.borrow_mut().push(a);
            0
        });
    }
    let buf = emu.alloc(0x100);
    let mut total = 0u32;
    for s in durations() {
        log.borrow_mut().clear();
        emu.call(0x49_e9d0, buf as u32, 0x100, &[s as u32]);
        let calls = log.borrow();
        assert_eq!(calls.len(), 1, "one SStrPrintf for {s}");
        let a = calls[0];
        assert_eq!(
            (a[0], a[1], a[2]),
            (buf as u32, 0x100, 0x84_49f8),
            "buffer, size, format"
        );
        let d = split_duration(s);
        assert_eq!(
            [a[3], a[4], a[5], a[6]].map(|x| x as i32),
            [d.days, d.hours, d.minutes, d.seconds],
            "seconds={s}"
        );
        total += 1;
    }
    eprintln!("instances_format_duration: {total} bit-exact (0x49e9d0 whole)");
}

/// The lines `AddChatLine` was given, with their types.
type Lines = Rc<RefCell<Vec<(Vec<u8>, u32)>>>;

struct LockoutRig {
    reads: Rc<RefCell<Vec<u32>>>,
    texts: Rc<RefCell<Vec<(String, i32)>>>,
    lines: Lines,
    store: u64,
}

/// A template per key: the key itself, then the `%s` and `%d`s its arm fills.
fn template_of(key: &str) -> String {
    let n = if key == "RAID_INSTANCE_WELCOME" { 3 } else { 1 };
    format!("{key}:%s{}", "|%d".repeat(n))
}

fn lockout_rig(emu: &mut Emu) -> LockoutRig {
    let reads: Rc<RefCell<Vec<u32>>> = Rc::default();
    let texts: Rc<RefCell<Vec<(String, i32)>>> = Rc::default();
    let lines: Lines = Rc::default();
    {
        let reads = Rc::clone(&reads);
        emu.stub_fn(GETINT32, 1, move |io| {
            let v = reads.borrow_mut().remove(0);
            let out = u64::from(io.arg(0));
            io.write_u32(out, v);
            io.reg(RegisterX86::ECX) as u32
        });
    }
    // One template cell per key, written on first use.
    let cells = emu.alloc(0x400);
    {
        let texts = Rc::clone(&texts);
        let mut slots: Vec<String> = Vec::new();
        emu.stub_fn(GETTEXT, 1, move |io| {
            let key = String::from_utf8(io.read_cstr(io.reg(RegisterX86::ECX))).unwrap();
            let ordinal = io.reg(RegisterX86::EDX) as u32 as i32;
            texts.borrow_mut().push((key.clone(), ordinal));
            let i = match slots.iter().position(|k| *k == key) {
                Some(i) => i,
                None => {
                    slots.push(key.clone());
                    slots.len() - 1
                }
            };
            let at = cells + 0x80 * i as u64;
            let mut t = template_of(&key).into_bytes();
            t.push(0);
            io.write(at, &t);
            at as u32
        });
    }
    emu.stub_fn(SSTRPRINTF, 0, mini_printf);
    {
        let lines = Rc::clone(&lines);
        emu.stub_fn(ADDCHATLINE, 10, move |io| {
            let text = io.read_cstr(io.reg(RegisterX86::ECX));
            let ty = io.reg(RegisterX86::EDX) as u32;
            for i in 0..10 {
                assert_eq!(io.arg(i), 0, "AddChatLine argument {i}");
            }
            lines.borrow_mut().push((text, ty));
            0
        });
    }
    // Two maps: id 1 named "Molten Core", id 3 present in range without a record.
    let index = emu.alloc(0x40);
    let rec = emu.alloc(0x100);
    let name = emu.alloc(0x40);
    emu.write(name, b"Molten Core\0");
    emu.write_u32(rec + 0x10, name as u32);
    emu.write_u32(index + 4, rec as u32);
    emu.write_u32(MAP_INDEX, index as u32);
    emu.write_u32(MAP_MAX, 4);
    emu.write_u32(LOCALE, 0);
    let store = emu.alloc(0x40);
    LockoutRig {
        reads,
        texts,
        lines,
        store,
    }
}

#[test]
fn instances_lockout_warning() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rig = lockout_rig(&mut emu);
    let mut seconds = vec![
        0, 1, 59, 60, 61, 3_599, 3_600, 3_601, 86_399, 86_400, 86_401, -1, -60, -3_601,
    ];
    seconds.extend([i32::MAX, i32::MIN, 7 * 86_400 + 5 * 3_600 + 17 * 60 + 9]);
    let mut s = 0x49e1_c000u32;
    for _ in 0..400 {
        seconds.push(next(&mut s) as i32);
    }
    let maps: &[(i32, &str)] = &[(1, "Molten Core"), (3, "3"), (5, "5"), (-2, "-2")];
    let mut total = 0u32;
    for ty in -1..=6i32 {
        for &(map, map_name) in maps {
            for &t in &seconds {
                rig.reads
                    .borrow_mut()
                    .extend([ty as u32, map as u32, t as u32]);
                rig.texts.borrow_mut().clear();
                rig.lines.borrow_mut().clear();
                let ret = emu.call(0x49_e1c0, 0, 0x2fa, &[0, rig.store as u32]);
                assert_eq!(ret, 1);
                assert!(rig.reads.borrow().is_empty(), "three reads");
                let want = lockout_warning(ty, t);
                let texts = rig.texts.borrow().clone();
                let lines = rig.lines.borrow().clone();
                match want {
                    None => {
                        assert!(texts.is_empty() && lines.is_empty(), "type {ty}: nothing");
                    }
                    Some(w) => {
                        assert_eq!(texts, [(w.key.to_string(), w.ordinal)], "type {ty} t={t}");
                        let mut line = format!("{}:{map_name}", w.key);
                        for n in &w.numbers {
                            line.push_str(&format!("|{n}"));
                        }
                        assert_eq!(
                            lines,
                            [(line.into_bytes(), 10)],
                            "type {ty} map {map} t={t}"
                        );
                    }
                }
                total += 1;
            }
        }
    }
    eprintln!("instances_lockout_warning: {total} bit-exact (0x49e1c0 whole)");
}

#[test]
fn instances_reserve_saved_list() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // The allocator stubs: SMemReAlloc answers a fresh block or null by the cell `fail`, and logs
    // the size; SMemAlloc answers a fresh block and logs the size; SMemFree logs the pointer.
    let pool = emu.alloc(0x40_0000);
    let bump = Rc::new(RefCell::new(pool));
    let fail = Rc::new(RefCell::new(false));
    let log: Rc<RefCell<Vec<(u64, u32, u32)>>> = Rc::default();
    let fresh = |bump: &Rc<RefCell<u64>>| {
        let mut b = bump.borrow_mut();
        let p = *b;
        *b += 0x1000;
        p as u32
    };
    {
        let (bump, fail, log) = (Rc::clone(&bump), Rc::clone(&fail), Rc::clone(&log));
        emu.stub_fn(SMEMREALLOC, 5, move |io| {
            log.borrow_mut().push((SMEMREALLOC, io.arg(0), io.arg(1)));
            if *fail.borrow() {
                0
            } else {
                fresh(&bump)
            }
        });
    }
    {
        let (bump, log) = (Rc::clone(&bump), Rc::clone(&log));
        emu.stub_fn(SMEMALLOC, 4, move |io| {
            log.borrow_mut().push((SMEMALLOC, io.arg(0), 0));
            fresh(&bump)
        });
    }
    {
        let log = Rc::clone(&log);
        emu.stub_fn(SMEMFREE, 4, move |io| {
            log.borrow_mut().push((SMEMFREE, io.arg(0), 0));
            1
        });
    }
    let header = emu.alloc(0x10);
    let old = emu.alloc(0x400);
    let records: Vec<u8> = (0..0x400u32).map(|i| (i * 7 + 3) as u8).collect();
    emu.write(old, &records);
    let mut total = 0u32;
    for capacity in [0u32, 3, 8] {
        for count in 0..=capacity {
            for granularity in [0u32, 1, 7, 21] {
                for extra in 0..=70u32 {
                    for round in [false, true] {
                        for null in [false, true] {
                            if bump.borrow().saturating_sub(pool) > 0x3f_0000 {
                                *bump.borrow_mut() = pool;
                            }
                            let data = if capacity == 0 { 0 } else { old as u32 };
                            emu.write_u32(header, capacity);
                            emu.write_u32(header + 4, count);
                            emu.write_u32(header + 8, data);
                            emu.write_u32(header + 0xc, granularity);
                            *fail.borrow_mut() = null;
                            log.borrow_mut().clear();
                            emu.call(0x4a_3370, header as u32, 0, &[extra, u32::from(round)]);
                            let got = ArrayHeader {
                                capacity: emu.read_u32(header),
                                count: emu.read_u32(header + 4),
                                granularity: emu.read_u32(header + 0xc),
                            };
                            let h = ArrayHeader {
                                capacity,
                                count,
                                granularity,
                            };
                            let calls = log.borrow().clone();
                            let what = format!(
                                "cap={capacity} count={count} gran={granularity} extra={extra} round={round} null={null}"
                            );
                            match reserve(h, extra, round) {
                                None => {
                                    assert_eq!(got, h, "{what}: unchanged");
                                    assert!(calls.is_empty(), "{what}: no allocator call");
                                }
                                Some((want, bytes)) => {
                                    assert_eq!(got, want, "{what}: header");
                                    assert_eq!(calls[0], (SMEMREALLOC, data, bytes), "{what}");
                                    let new_data = emu.read_u32(header + 8);
                                    if null {
                                        assert_eq!(calls[1], (SMEMALLOC, bytes, 0), "{what}");
                                        if data != 0 {
                                            assert_eq!(calls[2], (SMEMFREE, data, 0), "{what}");
                                            let n = (want.capacity.min(count) * 12) as usize;
                                            assert_eq!(
                                                emu.read(u64::from(new_data), n),
                                                records[..n].to_vec(),
                                                "{what}: copied records"
                                            );
                                        } else {
                                            assert_eq!(calls.len(), 2, "{what}");
                                        }
                                    } else {
                                        assert_eq!(calls.len(), 1, "{what}");
                                    }
                                    assert_ne!(new_data, 0, "{what}: new storage");
                                }
                            }
                            total += 1;
                        }
                    }
                }
            }
        }
    }
    eprintln!("instances_reserve_saved_list: {total} bit-exact (0x4a3370 whole)");
}

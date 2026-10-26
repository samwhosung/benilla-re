//! The collector's marking in `lgc.c`, run against the exe: `valismarked` (`0x6f79f0`) and
//! `cleartablekeys` (`0x6f7970`, with `removekey` `0x6f79d0` inside) whole, `checkstacksizes`
//! (`0x6f7900`) whole with its two resizes stubbed, and `traversetable` (`0x6f7590`) whole with
//! `reallymarkobject` (`0x6f74a0`) and `luaT_gettm` (`0x6f7ba0`) stubbed and `_strchr` run.

use std::cell::RefCell;
use std::rc::Rc;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lua::gc::{checkstacksizes, clear_table_keys, traversetable, valismarked, Slot};

const VALISMARKED: u64 = 0x6f79f0;
const CLEARTABLEKEYS: u64 = 0x6f7970;
const CHECKSTACKSIZES: u64 = 0x6f7900;
const TRAVERSETABLE: u64 = 0x6f7590;
const REALLYMARKOBJECT: u64 = 0x6f74a0;
const LUAT_GETTM: u64 = 0x6f7ba0;
const REALLOC_CI: u64 = 0x6f5ef0;
const REALLOC_STACK: u64 = 0x6f5e20;
/// The execution-context word a nil takes (`s_scriptContext`).
const CONTEXT: u64 = 0xceeac0;

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// The tags a slot takes: -1 (`LUA_TNONE`) to 8, and 9 past the last.
const TAGS: [i32; 11] = [-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

/// Every tag against mark bytes with bit 0, bit 4, both, neither and the rest of the byte set; it
/// compares the answer and the object's byte after.
#[test]
fn valismarked_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x20);
    let val = emu.alloc(0x10);
    for &tag in &TAGS {
        for marked in [0u8, 1, 0x10, 0x11, 0xfe, 0xff, 0x06, 0x80] {
            emu.write(obj, &[0, 0, 0, 0, 0, marked, 0, 0]);
            emu.write_u32(val, tag as u32);
            emu.write_u32(val + 8, obj as u32);
            let got = emu.call(VALISMARKED, val as u32, 0, &[]);
            let (want, byte) = valismarked(tag, marked);
            assert_eq!(got, u32::from(want), "tag {tag} marked {marked:#x}");
            assert_eq!(
                emu.read(obj + 5, 1)[0],
                byte,
                "byte, tag {tag} marked {marked:#x}"
            );
        }
    }
}

/// Writes a node array of `nodes` at `at` (40-byte nodes, key at `+0`, value at `+0x10`), each
/// collectable slot pointing at its own object, a byte at `+5`, from `objs`.
fn write_nodes(emu: &mut Emu, at: u64, objs: u64, nodes: &[(Slot, Slot)]) {
    for (i, (k, v)) in nodes.iter().enumerate() {
        let n = at + 0x28 * i as u64;
        for (j, s) in [k, v].into_iter().enumerate() {
            let o = objs + 0x10 * (2 * i + j) as u64;
            emu.write(o, &[0, 0, 0, 0, 0, s.marked, 0, 0]);
            emu.write_u32(n + 0x10 * j as u64, s.tag as u32);
            emu.write_u32(n + 0x10 * j as u64 + 4, s.context);
            emu.write_u32(n + 0x10 * j as u64 + 8, o as u32);
        }
    }
}

fn read_nodes(emu: &Emu, at: u64, objs: u64, n: usize) -> Vec<(Slot, Slot)> {
    (0..n)
        .map(|i| {
            let node = at + 0x28 * i as u64;
            let slot = |j: usize| Slot {
                tag: emu.read_u32(node + 0x10 * j as u64) as i32,
                context: emu.read_u32(node + 0x10 * j as u64 + 4),
                marked: emu.read(objs + 0x10 * (2 * i + j) as u64 + 5, 1)[0],
            };
            (slot(0), slot(1))
        })
        .collect()
}

fn random_slot(s: &mut u32) -> Slot {
    Slot {
        tag: TAGS[(lcg(s) >> 8) as usize % TAGS.len()],
        context: lcg(s),
        marked: [0u8, 1, 0x10, 0x11, 0x06, 0xfe][(lcg(s) >> 8) as usize % 6],
    }
}

/// A chain of one to three tables of node arrays of 1 to 64 nodes (`lsizenode` 0 to 6), random
/// tags and mark bytes, linked through `+0x18`, with a context word: every key tag, value tag and
/// mark byte compared after.
#[test]
fn cleartablekeys_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let tables = emu.alloc(0x100);
    let node_area = emu.alloc(3 * 0x28 * 64);
    let obj_area = emu.alloc(3 * 0x10 * 128);
    let mut s = 0x6f79_7000u32;
    for round in 0..300 {
        let context = lcg(&mut s);
        emu.write_u32(CONTEXT, context);
        let count = 1 + round % 3;
        let mut want = Vec::new();
        for t in 0..count {
            let lsize = (lcg(&mut s) >> 8) % 7;
            let n = 1usize << lsize;
            let nodes: Vec<(Slot, Slot)> = (0..n)
                .map(|_| (random_slot(&mut s), random_slot(&mut s)))
                .collect();
            let table = tables + 0x20 * t as u64;
            let at = node_area + (0x28 * 64 * t) as u64;
            let objs = obj_area + (0x10 * 128 * t) as u64;
            write_nodes(&mut emu, at, objs, &nodes);
            emu.write(table + 7, &[lsize as u8]);
            emu.write_u32(table + 0x10, at as u32);
            let next = if t + 1 < count { table + 0x20 } else { 0 };
            emu.write_u32(table + 0x18, next as u32);
            let mut cleared = nodes.clone();
            clear_table_keys(&mut cleared, context);
            want.push((at, objs, cleared));
        }
        emu.call(CLEARTABLEKEYS, tables as u32, 0, &[]);
        for (at, objs, cleared) in want {
            assert_eq!(
                read_nodes(&emu, at, objs, cleared.len()),
                cleared,
                "round {round}"
            );
        }
    }
    emu.call(CLEARTABLEKEYS, 0, 0, &[]);
}

/// The call-record and stack sizes around each threshold: the record counts in use 0 to 40 against
/// sizes 0 to 64, `max` at 0 to 400 slots above the stack (and a few bytes off a slot) against
/// stack sizes 0 to 200 and some far past; the resize calls compared with their new sizes.
#[test]
fn checkstacksizes_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let l = emu.alloc(0x60);
    let seen: Rc<RefCell<Vec<(u64, u32)>>> = Rc::default();
    for f in [REALLOC_CI, REALLOC_STACK] {
        let seen = seen.clone();
        emu.stub_fn(f, 0, move |io| {
            seen.borrow_mut().push((f, io.reg(RegisterX86::EDX) as u32));
            0
        });
    }
    let base = 0x1000_0000u32;
    let mut cases: Vec<(i32, u16, i32, i32)> = Vec::new();
    for used in 0..=40i32 {
        for size in [0u16, 1, 15, 16, 17, 20, 32, 33, 40, 64, 100, 161, 0xffff] {
            cases.push((used * 24, size, 0, 0));
        }
    }
    for slots in (0..=400i32).step_by(7).chain([22, 23, 24, 45, 46, 50]) {
        for size in [0i32, 89, 90, 91, 92, 93, 100, 180, 181, 184, 200, 1000, -4] {
            for off in [0i32, 1, 15] {
                cases.push((0, 8, slots * 16 + off, size));
            }
        }
    }
    let mut s = 0x6f79_0000u32;
    for _ in 0..2000 {
        cases.push((
            ((lcg(&mut s) >> 20) as i32) * 24 + (lcg(&mut s) % 24) as i32,
            (lcg(&mut s) >> 16) as u16 % 300,
            (lcg(&mut s) >> 18) as i32,
            (lcg(&mut s) >> 20) as i32,
        ));
    }
    for (ci_bytes, size_ci, stack_bytes, stacksize) in cases {
        emu.write_u32(l + 0x28, base);
        emu.write_u32(l + 0x14, base.wrapping_add(ci_bytes as u32));
        emu.write(l + 0x2c, &size_ci.to_le_bytes());
        emu.write_u32(l + 0x1c, base);
        emu.write_u32(l + 0x20, stacksize as u32);
        seen.borrow_mut().clear();
        emu.call(
            CHECKSTACKSIZES,
            l as u32,
            base.wrapping_add(stack_bytes as u32),
            &[],
        );
        let (ci, stack) = checkstacksizes(ci_bytes, size_ci, stack_bytes, stacksize);
        let mut want = Vec::new();
        if let Some(n) = ci {
            want.push((REALLOC_CI, n));
        }
        if let Some(n) = stack {
            want.push((REALLOC_STACK, n as u32));
        }
        assert_eq!(
            *seen.borrow(),
            want,
            "ci {ci_bytes} size_ci {size_ci} stack {stack_bytes} stacksize {stacksize}"
        );
    }
}

/// Tables with arrays of 0 to 12 slots and node arrays of 1 to 32 nodes, random tags and mark
/// bytes, metatables with each mark byte and flag bit 3 set or clear, and modes: none, a number,
/// and the strings "", "k", "v", "kv", "vk", "xkx", "K", "abc" and one with `v` after a NUL. It
/// compares the objects marked, in order, the table's mark byte, the list it joined and its link.
#[test]
fn traversetable_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let st = emu.alloc(0x20);
    let g = emu.alloc(0x100);
    let h = emu.alloc(0x40);
    let mt = emu.alloc(0x20);
    let mode_obj = emu.alloc(0x10);
    let mode_str = emu.alloc(0x40);
    let array_area = emu.alloc(0x10 * 12);
    let array_objs = emu.alloc(0x10 * 12);
    let node_area = emu.alloc(0x28 * 32);
    let node_objs = emu.alloc(0x10 * 64);
    let marks: Rc<RefCell<Vec<u32>>> = Rc::default();
    {
        let marks = marks.clone();
        emu.stub_fn(REALLYMARKOBJECT, 0, move |io| {
            assert_eq!(io.reg(RegisterX86::ECX), st, "reallymarkobject's state");
            marks.borrow_mut().push(io.reg(RegisterX86::EDX) as u32);
            0
        });
    }
    let mode_ptr = Rc::new(RefCell::new(0u32));
    let asked = Rc::new(RefCell::new(false));
    {
        let (mode_ptr, asked) = (mode_ptr.clone(), asked.clone());
        emu.stub_fn(LUAT_GETTM, 1, move |io| {
            assert_eq!(io.reg(RegisterX86::ECX), mt, "luaT_gettm's table");
            assert_eq!(io.reg(RegisterX86::EDX), 3, "luaT_gettm's event, TM_MODE");
            assert_eq!(io.arg(0), 0x5eed_0003, "luaT_gettm's name, tmname[3]");
            *asked.borrow_mut() = true;
            *mode_ptr.borrow()
        });
    }
    emu.write_u32(g + 0x8c, 0x5eed_0003);
    emu.write_u32(st + 0x10, g as u32);
    let modes: [Option<(i32, &[u8])>; 12] = [
        None,
        Some((3, b"kv")),
        Some((4, b"")),
        Some((4, b"k")),
        Some((4, b"v")),
        Some((4, b"kv")),
        Some((4, b"vk")),
        Some((4, b"xkx")),
        Some((4, b"K")),
        Some((4, b"abc")),
        Some((4, b"k\0v")),
        Some((5, b"k")),
    ];
    let mut s = 0x6f75_9000u32;
    for round in 0..3000 {
        let mode = modes[round % modes.len()];
        let mt_marked = [0u8, 1, 0x10, 0x06][(lcg(&mut s) >> 8) as usize % 4];
        let mt_flags = if lcg(&mut s) & 0x100 != 0 {
            8u8
        } else {
            0x37 & !8
        };
        let marked = (lcg(&mut s) >> 8) as u8;
        let array: Vec<Slot> = (0..(lcg(&mut s) >> 8) % 13)
            .map(|_| random_slot(&mut s))
            .collect();
        let lsize = (lcg(&mut s) >> 8) % 6;
        let nodes: Vec<(Slot, Slot)> = (0..1usize << lsize)
            .map(|_| (random_slot(&mut s), random_slot(&mut s)))
            .collect();
        // The state's lists, each with a sentinel head the table's link takes.
        for (i, head) in [0x1111u32, 0x2222, 0x3333, 0x4444].into_iter().enumerate() {
            emu.write_u32(st + 4 * i as u64, head);
        }
        emu.write(h, &[0, 0, 0, 0, 0, marked, 0xaa, lsize as u8]);
        emu.write_u32(h + 8, mt as u32);
        emu.write_u32(h + 0xc, array_area as u32);
        emu.write_u32(h + 0x10, node_area as u32);
        emu.write_u32(h + 0x18, 0xdead_beef);
        emu.write_u32(h + 0x1c, array.len() as u32);
        emu.write(mt, &[0, 0, 0, 0, 0, mt_marked, mt_flags, 0]);
        for (i, a) in array.iter().enumerate() {
            let o = array_objs + 0x10 * i as u64;
            emu.write(o, &[0, 0, 0, 0, 0, a.marked, 0, 0]);
            emu.write_u32(array_area + 0x10 * i as u64, a.tag as u32);
            emu.write_u32(array_area + 0x10 * i as u64 + 8, o as u32);
        }
        write_nodes(&mut emu, node_area, node_objs, &nodes);
        match mode {
            None => *mode_ptr.borrow_mut() = 0,
            Some((tag, text)) => {
                let mut z = text.to_vec();
                z.push(0);
                emu.write(mode_str + 0x10, &z);
                emu.write_u32(mode_obj, tag as u32);
                emu.write_u32(mode_obj + 8, mode_str as u32);
                *mode_ptr.borrow_mut() = mode_obj as u32;
            }
        }
        marks.borrow_mut().clear();
        *asked.borrow_mut() = false;
        emu.call(TRAVERSETABLE, st as u32, h as u32, &[]);
        let want = traversetable(marked, mt_marked, mt_flags, mode, &array, &nodes);
        let what = format!("round {round}, mode {mode:?}");
        let mut order = Vec::new();
        if want.marked_metatable {
            order.push(mt as u32);
        }
        order.extend(
            want.array
                .iter()
                .map(|&i| (array_objs + 0x10 * i as u64) as u32),
        );
        order.extend(
            want.nodes
                .iter()
                .map(|&(i, v)| (node_objs + 0x10 * (2 * i + usize::from(v)) as u64) as u32),
        );
        assert_eq!(*marks.borrow(), order, "marks, {what}");
        assert_eq!(*asked.borrow(), want.asked_mode, "mode asked, {what}");
        assert_eq!(emu.read(h + 5, 1)[0], want.marked, "mark byte, {what}");
        let heads = [0x2222u32, 0x3333, 0x4444];
        match want.list {
            None => assert_eq!(emu.read_u32(h + 0x18), 0xdead_beef, "link, {what}"),
            Some(l) => {
                assert_eq!(
                    emu.read_u32(st + 4 * u64::from(l)),
                    h as u32,
                    "list, {what}"
                );
                assert_eq!(
                    emu.read_u32(h + 0x18),
                    heads[l as usize - 1],
                    "link, {what}"
                );
            }
        }
    }
}

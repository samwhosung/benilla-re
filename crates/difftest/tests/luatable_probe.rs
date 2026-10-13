//! Where the client's Lua 5.0 stores a table's entries, and the order its traversal yields them.
//!
//! The SavedVariables writer writes a list as `[n] = v` fields. `recfield` counts a hash entry
//! only in its `TK_NAME` arm, so a `[expr] = v` field counts toward neither part: the
//! `OP_NEWTABLE` it compiles to carries `B = 0` and `C = 0`. With `C = 0` the table starts on the
//! shared dummy node, the first `SETTABLE` finds no free node and calls `rehash`, and `rehash` moves
//! dense integer keys into the array part, which `luaH_next` walks in ascending order. So such a
//! list comes back `1, 2, 3, …`, as a positional constructor's does.
//!
//! `lua_open` (`0x6f6d20`) builds a real `lua_State`; `luaL_loadbuffer` (`0x6f5690`) and
//! `lua_pcall` (`0x6f41a0`) compile and run a chunk; `lua_pushstring` (`0x6f3890`) and
//! `lua_gettable` (`0x6f3a40`) on `LUA_GLOBALSINDEX` fetch a global; `lua_pushnil` (`0x6f37f0`)
//! and `lua_next` (`0x6f4450`) walk it. A `TObject` is 16 bytes, tag at +0 and number at +8;
//! `L->top` is `[L+8]`. A `Table` is 0x20 bytes: `lsizenode` +7, `node` +0x10, `firstfree` +0x14,
//! `sizearray` +0x1c; a `Node` is 0x28 bytes, key then value at +0x10. A `Proto`'s code is at +0xc
//! and its size at +0x2c, reached through `LClosure.p` at +0xc. The stubs are `luaparse_probe.rs`'s.
//!
//! The control `counterfactual_eight_node_table` builds the array-less 8-node table with
//! `luaH_new` (`0x6fa4f0`, `lnhash = 3`), inserts the same keys with `luaH_set` (`0x6fa840`), and
//! walks them out of order (`5, 1, 2, 3, 4, 6`): the ascending order comes from where keys land.

use benilla_re_difftest::{fixture, Emu, Wow};

const LUA_OPEN: u64 = 0x6f6d20;
const LUAL_LOADBUFFER: u64 = 0x6f5690;
const LUA_PCALL: u64 = 0x6f41a0;
const LUA_SETTOP: u64 = 0x6f3080;
const LUA_PUSHSTRING: u64 = 0x6f3890;
const LUA_GETTABLE: u64 = 0x6f3a40;
const LUA_PUSHNIL: u64 = 0x6f37f0;
const LUA_NEXT: u64 = 0x6f4450;
const LUAH_NEW: u64 = 0x6fa4f0;
const LUAH_SET: u64 = 0x6fa840;
const LUAH_NEXT: u64 = 0x6fa2a0;

const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const SMEM_FREE: u64 = 0x646430;
const SETJMP3: u64 = 0x742090;
const RAWRUN_INSTRUMENT: u64 = 0x42a320;
const CRT_SPRINTF: u64 = 0x73fec4;

const LUA_GLOBALSINDEX: u32 = 0xffff_d8ef; // -10001
const ARENA_BYTES: u64 = 0xE0_0000;

fn fresh(wow: &Wow) -> (Emu<'_>, u32) {
    let mut emu = Emu::new(wow);
    let arena = emu.alloc(ARENA_BYTES);
    emu.stub_alloc_sized(SMEM_ALLOC, arena, arena + ARENA_BYTES, 0, 4);
    emu.stub_realloc_sized(SMEM_REALLOC, 5);
    emu.stub(SMEM_FREE, 0, 4);
    emu.stub(SETJMP3, 0, 0);
    emu.stub(RAWRUN_INSTRUMENT, 0, 0);
    let digit = emu.alloc(2);
    emu.write(digit, b"1\0");
    emu.stub_write_through(CRT_SPRINTF, 0, digit, 2, 1, 0);
    let l = emu.call(LUA_OPEN, 0, 0, &[]);
    assert_ne!(l, 0, "lua_open returned NULL");
    (emu, l)
}

fn cstr(emu: &mut Emu, s: &str) -> u64 {
    let p = emu.alloc(s.len() as u64 + 1);
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    emu.write(p, &b);
    p
}

/// (type tag, the double payload) of the 16-byte TObject at `va`.
fn tobject(emu: &Emu, va: u64) -> (u32, f64) {
    let tt = emu.read_u32(va);
    let mut a = [0u8; 8];
    a.copy_from_slice(&emu.read(va + 8, 8));
    (tt, f64::from_le_bytes(a))
}

fn f64_at(emu: &Emu, va: u64) -> f64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&emu.read(va, 8));
    f64::from_le_bytes(a)
}

fn load(emu: &mut Emu, l: u32, src: &[u8]) {
    let buf = emu.alloc(src.len() as u64 + 1);
    emu.write(buf, src);
    let nm = cstr(emu, "probe");
    assert_eq!(
        emu.call(
            LUAL_LOADBUFFER,
            l,
            buf as u32,
            &[src.len() as u32, nm as u32]
        ),
        0,
        "luaL_loadbuffer status"
    );
}

/// `sizearray` and `lsizenode` of the table a chunk left in a global.
struct Layout {
    sizearray: u32,
    lsizenode: u8,
}

/// Compiles and runs `src`, fetches the table at `path` under `_G`, and walks it with `lua_next`.
/// Returns the (key, value) pairs in the order yielded, and the table's layout.
fn enumerate(wow: &Wow, src: &[u8], path: &[&str]) -> (Vec<(f64, f64)>, Layout) {
    let (mut emu, l) = fresh(wow);
    load(&mut emu, l, src);
    assert_eq!(emu.call(LUA_PCALL, l, 0, &[0, 0]), 0, "lua_pcall status");

    emu.call(LUA_SETTOP, l, 0, &[]);
    // Absolute stack indices: path[0] lands at 1, path[1] at 2, and so on.
    for (i, key) in path.iter().enumerate() {
        let k = cstr(&mut emu, key);
        emu.call(LUA_PUSHSTRING, l, k as u32, &[]);
        let container = if i == 0 { LUA_GLOBALSINDEX } else { i as u32 };
        emu.call(LUA_GETTABLE, l, container, &[]);
    }
    let depth = path.len() as u32;
    let top = u64::from(emu.read_u32(u64::from(l) + 8));
    assert_eq!(emu.read_u32(top - 0x10), 5, "target is a table (tag 5)");
    let t = u64::from(emu.read_u32(top - 0x10 + 8));
    let layout = Layout {
        sizearray: emu.read_u32(t + 0x1c),
        lsizenode: emu.read(t, 8)[7],
    };

    let mut out = Vec::new();
    emu.call(LUA_PUSHNIL, l, 0, &[]);
    loop {
        if emu.call(LUA_NEXT, l, depth, &[]) == 0 {
            break;
        }
        let top = u64::from(emu.read_u32(u64::from(l) + 8));
        let (ktt, k) = tobject(&emu, top - 0x20);
        let (vtt, v) = tobject(&emu, top - 0x10);
        assert_eq!(ktt, 3, "key is a number");
        assert_eq!(vtt, 3, "value is a number");
        out.push((k, v));
        emu.call(LUA_SETTOP, l, 0xffff_fffe, &[]); // pop the value, keep the key
        assert!(out.len() <= 64, "runaway traversal");
    }
    (out, layout)
}

fn keys_vals(pairs: &[(f64, f64)]) -> (Vec<i64>, Vec<i64>) {
    (
        pairs.iter().map(|(k, _)| *k as i64).collect(),
        pairs.iter().map(|(_, v)| *v as i64).collect(),
    )
}

/// The instruction words compiled from `src`, read from its `Proto`.
fn emitted_code(wow: &Wow, src: &str) -> Vec<u32> {
    let (mut emu, l) = fresh(wow);
    load(&mut emu, l, src.as_bytes());
    let top = u64::from(emu.read_u32(u64::from(l) + 8));
    assert_eq!(emu.read_u32(top - 0x10), 6, "loadbuffer left a function");
    let cl = u64::from(emu.read_u32(top - 0x10 + 8));
    let proto = u64::from(emu.read_u32(cl + 0x0c));
    let code = u64::from(emu.read_u32(proto + 0x0c));
    let sizecode = emu.read_u32(proto + 0x2c);
    assert!(
        (1..=64).contains(&sizecode),
        "implausible sizecode {sizecode}: the Proto layout differs"
    );
    (0..u64::from(sizecode))
        .map(|i| emu.read_u32(code + i * 4))
        .collect()
}

/// The VM's instruction encoding: `OP` bits 0-5, `C` 6-14, `B` 15-23, `A` 24-31.
fn decode(w: u32) -> (u32, u32, u32, u32) {
    (w & 0x3f, w >> 24, (w >> 15) & 0x1ff, (w >> 6) & 0x1ff)
}

/// What the compiler asks for, and where the keys end up.
#[test]
fn explicit_intkey_constructor_asks_for_nothing_and_rehashes_into_the_array() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let src = "t = { [1] = 0, [2] = 1, [3] = 2, [4] = 3, [5] = 4, [6] = -2 }";
    let code = emitted_code(&wow, src);
    let (op, a, b, c) = decode(code[0]);
    println!(
        "EXPLICIT-KEY NEWTABLE {:#010x} -> OP={op} A={a} B={b} C={c}",
        code[0]
    );
    assert_eq!(op, 10, "first instruction is OP_NEWTABLE");
    // `recfield`'s cc->nh++ is in its TK_NAME arm, so a `[expr] = v` field counts toward neither:
    // int2fb(cc.na = 0) = 0 and luaO_log2(cc.nh = 0) + 1 = 0.
    assert_eq!((b, c), (0, 0), "no array hint AND no hash hint");
    // Each field is an OP_SETTABLE (9); there is no OP_SETLIST.
    assert_eq!(code[1..7].iter().filter(|w| *w & 0x3f == 9).count(), 6);
    assert!(code.iter().all(|w| w & 0x3f != 31), "no OP_SETLIST emitted");

    let (pairs, layout) = enumerate(&wow, src.as_bytes(), &["t"]);
    println!(
        "EXPLICIT-KEY sizearray={} lsizenode={}",
        layout.sizearray, layout.lsizenode
    );
    // C = 0 starts on the dummy node, so the first SETTABLE rehashes, moving the dense integer keys
    // to the array part; computesizes gives 8 for 6 keys.
    assert_eq!(layout.sizearray, 8, "keys migrated into the array part");
    assert_eq!(layout.lsizenode, 0, "hash part is still the dummy node");
    let (keys, vals) = keys_vals(&pairs);
    println!("EXPLICIT-KEY keys={keys:?} vals={vals:?}");
    assert_eq!(keys, vec![1, 2, 3, 4, 5, 6], "ascending");
    assert_eq!(vals, vec![0, 1, 2, 3, 4, -2]);
}

/// A positional constructor reaches the same order another way.
#[test]
fn positional_constructor_sizes_the_array_up_front() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let src = "t = { 0, 1, 2, 3, 4, -2 }";
    let code = emitted_code(&wow, src);
    let (op, _, b, c) = decode(code[0]);
    println!(
        "POSITIONAL NEWTABLE {:#010x} -> OP={op} B={b} C={c}",
        code[0]
    );
    assert_eq!(op, 10);
    // listfield increments cc.na (and cc.tostore); int2fb(6) = 6, luaO_log2(0) + 1 = 0.
    assert_eq!((b, c), (6, 0), "array hint 6, no hash hint");
    assert_eq!(
        code.iter().filter(|w| *w & 0x3f == 31).count(),
        1,
        "one OP_SETLIST at the close"
    );

    let (pairs, layout) = enumerate(&wow, src.as_bytes(), &["t"]);
    println!(
        "POSITIONAL sizearray={} lsizenode={}",
        layout.sizearray, layout.lsizenode
    );
    assert_eq!(layout.sizearray, 6, "sized exactly, no rehash");
    assert_eq!(layout.lsizenode, 0);
    let (keys, vals) = keys_vals(&pairs);
    println!("POSITIONAL keys={keys:?} vals={vals:?}");
    assert_eq!(keys, vec![1, 2, 3, 4, 5, 6]);
    assert_eq!(vals, vec![0, 1, 2, 3, 4, -2]);
}

/// The array part is walked by index, so neither insertion order nor construction moves a dense
/// integer key.
#[test]
fn integer_key_order_is_independent_of_insertion_order() {
    let Some(wow) = Wow::load() else {
        return;
    };
    for src in [
        "t = {} t[1]=0 t[2]=1 t[3]=2 t[4]=3 t[5]=4 t[6]=-2",
        "t = {} t[6]=-2 t[5]=4 t[4]=3 t[3]=2 t[2]=1 t[1]=0",
        "t = { [2] = 1, [1] = 0, [6] = -2, [3] = 2, [5] = 4, [4] = 3 }",
    ] {
        let (pairs, _) = enumerate(&wow, src.as_bytes(), &["t"]);
        let (keys, vals) = keys_vals(&pairs);
        println!("{src:?}\n  keys={keys:?} vals={vals:?}");
        assert_eq!(keys, vec![1, 2, 3, 4, 5, 6], "{src:?}");
        assert_eq!(vals, vec![0, 1, 2, 3, 4, -2], "{src:?}");
    }
}

/// A `[1..n]` list walks in ascending order for every length up to 24.
#[test]
fn explicit_intkey_list_is_ascending_for_every_length() {
    let Some(wow) = Wow::load() else {
        return;
    };
    for n in 1..=24usize {
        let body: Vec<String> = (1..=n)
            .map(|i| format!("[{i}] = {}", 100 - i as i64))
            .collect();
        let src = format!("t = {{ {} }}", body.join(", "));
        let (pairs, _) = enumerate(&wow, src.as_bytes(), &["t"]);
        let (keys, vals) = keys_vals(&pairs);
        println!("n={n:2} keys={keys:?}");
        assert_eq!(keys, (1..=n as i64).collect::<Vec<_>>(), "n={n}");
        assert_eq!(
            vals,
            (1..=n as i64).map(|i| 100 - i).collect::<Vec<_>>(),
            "n={n}"
        );
    }
}

/// A key the array part cannot hold (0, a negative or a sparse one) goes to the node part and is
/// yielded after every array key.
#[test]
fn non_array_keys_land_in_the_node_part_and_come_last() {
    let Some(wow) = Wow::load() else {
        return;
    };
    for (src, want_keys, want_sizearray) in [
        ("t = { [1] = 10, [100] = 99 }", vec![1i64, 100], 1u32),
        ("t = { [0] = 9, [1] = 1, [2] = 2 }", vec![1, 2, 0], 2),
        ("t = { [1] = 1, [2] = 2, [-1] = 7 }", vec![1, 2, -1], 2),
    ] {
        let (pairs, layout) = enumerate(&wow, src.as_bytes(), &["t"]);
        let (keys, _) = keys_vals(&pairs);
        println!(
            "{src:?} sizearray={} lsizenode={} keys={keys:?}",
            layout.sizearray, layout.lsizenode
        );
        assert_eq!(keys, want_keys, "{src:?}");
        assert_eq!(layout.sizearray, want_sizearray, "{src:?}");
    }
}

/// The control: a table the constructor never makes, 8 nodes and no array, built with `luaH_new`
/// (`0x6fa4f0`), filled with `luaH_set` (`0x6fa840`) and walked with `luaH_next` (`0x6fa2a0`),
/// comes out in node order, not ascending.
///
/// `hashnum` (`0x6fa260`) adds 1.0 (`fadd [0x8015b8]`), adds the two dwords and divides by
/// `((1<<lsizenode)-1)|1`, so with 8 nodes keys 1 to 6 hash to 1, 3, 5, 6, 0, 1; key 6 collides
/// with key 1 and takes the last free node, 7, since `firstfree` starts at the last node.
#[test]
fn counterfactual_eight_node_table() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut emu, l) = fresh(&wow);
    let t = emu.call(LUAH_NEW, l, 0, &[3]); // narray = 0, lnhash = 3 => 8 nodes
    let tv = u64::from(t);
    assert_eq!(emu.read_u32(tv + 0x1c), 0, "no array part");
    assert_eq!(emu.read(tv, 8)[7], 3, "8 nodes");
    let node = u64::from(emu.read_u32(tv + 0x10));
    assert_eq!(
        (i64::from(emu.read_u32(tv + 0x14)) - node as i64) / 40,
        7,
        "firstfree starts at the last node"
    );

    let key = emu.alloc(0x10);
    let mut slots = Vec::new();
    for (k, v) in [(1i64, 0i64), (2, 1), (3, 2), (4, 3), (5, 4), (6, -2)] {
        emu.write_u32(key, 3); // LUA_TNUMBER
        emu.write(key + 8, &(k as f64).to_le_bytes());
        let slot = u64::from(emu.call(LUAH_SET, l, t, &[key as u32]));
        emu.write_u32(slot, 3);
        emu.write(slot + 8, &(v as f64).to_le_bytes());
        slots.push((slot - 0x10 - node) / 40);
    }
    println!("counterfactual slots = {slots:?}");
    assert_eq!(
        slots,
        vec![1, 3, 5, 6, 0, 7],
        "mainpositions + the collision"
    );
    assert_eq!(emu.read_u32(tv + 0x1c), 0, "no rehash fired");
    assert_eq!(emu.read(tv, 8)[7], 3, "still 8 nodes");

    let ks = emu.alloc(0x20);
    emu.write_u32(ks, 0); // a nil key starts the walk
    let mut out = Vec::new();
    loop {
        if emu.call(LUAH_NEXT, l, t, &[ks as u32]) == 0 {
            break;
        }
        out.push((f64_at(&emu, ks + 8) as i64, f64_at(&emu, ks + 0x18) as i64));
        assert!(out.len() <= 16);
    }
    println!("counterfactual luaH_next order = {out:?}");
    assert_eq!(
        out,
        vec![(5, 4), (1, 0), (2, 1), (3, 2), (4, 3), (6, -2)],
        "node order, not ascending"
    );
}

/// A SavedVariables file the client wrote (a `Bagnon_Core.lua`, from `BENILLA_RE_SAVEDVARS_LUA`),
/// run through the client's own Lua, walks its list in ascending order.
#[test]
fn reference_savedvariables_list_traversal_order() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let Some(src) = fixture("BENILLA_RE_SAVEDVARS_LUA") else {
        return;
    };
    let (pairs, layout) = enumerate(&wow, &src, &["BagnonSets", "Bagnon", "bags"]);
    let (keys, vals) = keys_vals(&pairs);
    println!(
        "REAL-FILE bags sizearray={} lsizenode={} keys={keys:?} vals={vals:?}",
        layout.sizearray, layout.lsizenode
    );
    assert_eq!(
        keys,
        vec![1, 2, 3, 4, 5, 6],
        "the real file walks ascending"
    );
    assert_eq!(layout.lsizenode, 0, "the real list has no hash part");
}

//! The texture layer's decisions against the exe (`src/textures.rs`): the size limit
//! `TextureLimitSize` (`0x448b60`), the BLP format chooser (`0x44a790`), the request gate of
//! `TexturePoolAcquire` (`0x448450`) and the creation checks of `GxTexCreate` (`0x58ab10`). The
//! device's capability block (`0x58a230`) is a stub that returns a block the test fills.
//!
//! `0x448450` is run to its decision: a refused request returns at `0x448665`, a pooled size
//! reaches the pool lists, and a size the pool does not take reaches `0x58ac40`, which a stub logs.
//! The pool lists hold one record each that never matches, so a request that reaches a list ends in
//! `0x58ac40` too; the list it read shows in the record the getter `0x58ada0` was asked about. The
//! reuse arm (`0x448572` to `0x448640`) is not run. `0x58ab10` is run for texture type 0 up to its
//! device call, which a stub logs; the type tests (`0x58ab1c` to `0x58ab3a`) and the type 2 arm
//! (`0x58ab93`) are not exercised.

use std::cell::RefCell;
use std::rc::Rc;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_gx::textures::{
    choose_format, create_accepts, limit_size, pool_route, CreateCaps, CreateRequest, PoolRoute,
    TYPE_CODE_FORMAT,
};

mod common;
use common::next;

const GX_CAPS: u64 = 0x58a230;
const SET_LAST_ERROR: u64 = 0x64e850;
const LIMIT_SIZE: u64 = 0x448b60;
const CHOOSE_FORMAT: u64 = 0x44a790;
const POOL_ACQUIRE: u64 = 0x448450;
const CREATE_FROM_DESC: u64 = 0x58ac40;
const POOL_GETTER: u64 = 0x58ada0;
const GX_TEX_CREATE: u64 = 0x58ab10;
/// The pointer to the device object; its first dword is the vtable.
const DEVICE: u64 = 0xc0ed38;
/// The first pool list's head link; the lists are 12 bytes apart from `0xb05bec`.
const POOL_HEADS: u64 = 0xb05bf0;

const SIDES: [u32; 26] = [
    0, 1, 2, 3, 4, 6, 7, 8, 12, 15, 16, 17, 24, 31, 32, 33, 48, 64, 100, 128, 256, 512, 513, 768,
    1000, 1023,
];
const BIG_SIDES: [u32; 10] = [
    1024,
    1025,
    1536,
    2048,
    4095,
    4096,
    5000,
    8192,
    16384,
    0xffff_ffff,
];

fn caps_block(e: &mut Emu) -> u64 {
    let caps = e.alloc(0x100);
    e.stub(GX_CAPS, caps as u32, 0);
    caps
}

#[test]
fn texture_limit_size_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let caps = caps_block(&mut e);
    e.stub(SET_LAST_ERROR, 0, 1);
    let cell = e.alloc(0x10);
    let mut checked = 0u32;
    for limit in [
        0u32, 1, 255, 256, 257, 512, 1000, 1024, 2048, 4096, 8192, 16384,
    ] {
        e.write_u32(caps + 0x60, limit);
        for w in SIDES.iter().chain(BIG_SIDES.iter()).copied() {
            for h in SIDES.iter().chain(BIG_SIDES.iter()).copied().step_by(3) {
                for dropped in [0u32, 5] {
                    e.write_u32(cell, w);
                    e.write_u32(cell + 4, h);
                    e.write_u32(cell + 8, dropped);
                    e.clear_calls();
                    e.call(
                        LIMIT_SIZE,
                        cell as u32,
                        (cell + 4) as u32,
                        &[(cell + 8) as u32],
                    );
                    let binary = (e.read_u32(cell), e.read_u32(cell + 4), e.read_u32(cell + 8));
                    let ours = limit_size(w, h, dropped, limit).unwrap_or((w, h, dropped));
                    assert_eq!(binary, ours, "w={w} h={h} dropped={dropped} limit={limit}");
                    let errors = e.calls().iter().filter(|c| c.addr == SET_LAST_ERROR);
                    assert_eq!(
                        errors.map(|c| c.args[0]).collect::<Vec<_>>(),
                        if limit == 0 { vec![0x57] } else { vec![] },
                        "the error call at limit {limit}"
                    );
                    checked += 1;
                }
            }
        }
    }
    eprintln!("limit_size: {checked} cases bit-exact vs WoW.exe @ 0x448b60");
}

#[test]
fn texture_format_choice_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let caps = caps_block(&mut e);
    let out = e.alloc(0x10);
    for (i, &f) in TYPE_CODE_FORMAT.iter().enumerate() {
        assert_eq!(
            e.read_u32(0x835258 + 4 * i as u64),
            f,
            "the table at 0x835258"
        );
    }
    const UNSET: u32 = 0xdead_beef;
    let mut stored = 0u32;
    let mut untouched = 0u32;
    for flags in 0..8u32 {
        let (dxt1, dxt3, dxt5) = (flags & 1 != 0, flags & 2 != 0, flags & 4 != 0);
        // The flags are read as non-zero words, not as 1.
        e.write_u32(caps + 0x40, if dxt1 { 0x0100_0000 } else { 0 });
        e.write_u32(caps + 0x44, if dxt3 { 7 } else { 0 });
        e.write_u32(caps + 0x48, if dxt5 { 1 } else { 0 });
        for alpha_type in (0..=12u32).chain([0x7f, 0xff, 0x100, 0xffff_ffff]) {
            for alpha_depth in [0u32, 1, 2, 3, 4, 5, 8, 0xff] {
                e.write_u32(out, UNSET);
                e.write_u32(out + 4, UNSET);
                e.call(
                    CHOOSE_FORMAT,
                    out as u32,
                    (out + 4) as u32,
                    &[alpha_type, alpha_depth],
                );
                let binary = (e.read_u32(out), e.read_u32(out + 4));
                match choose_format(alpha_type, alpha_depth, dxt1, dxt3, dxt5) {
                    Some(c) => {
                        assert_eq!(
                            binary,
                            (c.type_code, c.format),
                            "type={alpha_type} depth={alpha_depth} dxt={dxt1}{dxt3}{dxt5}"
                        );
                        stored += 1;
                    }
                    None => {
                        assert_eq!(binary, (UNSET, UNSET), "type={alpha_type} stores nothing");
                        untouched += 1;
                    }
                }
            }
        }
    }
    assert!(stored > 0 && untouched > 0);
    eprintln!("choose_format: {stored} stores and {untouched} untouched, bit-exact vs 0x44a790");
}

#[test]
fn texture_pool_route_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let caps = caps_block(&mut e);
    // `0x58a980` (the flags word) clamps the anisotropy to `caps + 0xac`.
    e.write_u32(caps + 0xac, 16);
    // One record per list, none of which ever matches: the getter reports a different format.
    let nodes = e.alloc(0x40 * 25);
    for class in 0..25u64 {
        let node = nodes + 0x40 * class;
        e.write_u32(node, 0x100 * (class as u32 + 1));
        e.write_u32(node + 0xc, 1);
        e.write_u32(POOL_HEADS + 12 * class, node as u32);
    }
    let asked: Rc<RefCell<Vec<u32>>> = Rc::default();
    let sink = asked.clone();
    e.stub_fn(POOL_GETTER, 0, move |io| {
        let record = io.reg(benilla_re_difftest::RegisterX86::ECX) as u32;
        let desc = io.reg(benilla_re_difftest::RegisterX86::EDX);
        sink.borrow_mut().push(record);
        // desc +0x10 is the format and +0x18 the flags: neither can equal the request.
        io.write_u32(desc + 0x10, 0x1234_5678);
        io.write_u32(desc + 0x18, 0);
        0
    });
    let created: Rc<RefCell<Vec<[u32; 9]>>> = Rc::default();
    let sink = created.clone();
    e.stub_fn(CREATE_FROM_DESC, 0, move |io| {
        let desc = io.reg(benilla_re_difftest::RegisterX86::ECX);
        let out = io.reg(benilla_re_difftest::RegisterX86::EDX);
        let mut d = [0u32; 9];
        for (i, v) in d.iter_mut().enumerate() {
            *v = io.read_u32(desc + 4 * i as u64);
        }
        sink.borrow_mut().push(d);
        io.write_u32(out, 0x5151_0000);
        0
    });

    let mut seed = 0x4484_5000u32;
    let mut routes = [0u32; 3];
    let mut classes = [false; 25];
    let sides: Vec<u32> = SIDES.iter().chain(BIG_SIDES.iter()).copied().collect();
    for &w in &sides {
        for &h in &sides {
            for format in [0u32, 1, 4, 5, 7, 8, 9, 0xffff_ffff] {
                let flags = next(&mut seed) >> 7;
                let owner = next(&mut seed);
                let callback = next(&mut seed);
                let code = next(&mut seed) % 8;
                asked.borrow_mut().clear();
                created.borrow_mut().clear();
                let got = e.call(POOL_ACQUIRE, w, h, &[format, flags, owner, callback, code]);
                let route = pool_route(w, h, format);
                let ctx = format!("w={w} h={h} format={format}");
                match route {
                    PoolRoute::Refused => {
                        assert_eq!(got, 0, "{ctx}");
                        assert!(
                            asked.borrow().is_empty() && created.borrow().is_empty(),
                            "{ctx}"
                        );
                        routes[0] += 1;
                    }
                    PoolRoute::Create | PoolRoute::Pool(_) => {
                        if let PoolRoute::Pool(class) = route {
                            assert_eq!(*asked.borrow(), [0x100 * (class + 1)], "{ctx}");
                            classes[class as usize] = true;
                            routes[2] += 1;
                        } else {
                            assert!(asked.borrow().is_empty(), "{ctx}");
                            routes[1] += 1;
                        }
                        assert_eq!(got, 0x5151_0000, "{ctx}");
                        let c = created.borrow();
                        assert_eq!(c.len(), 1, "{ctx}");
                        // The record GxTexCreate takes: type 0, size, 0, the format, the format
                        // the type code maps to, the flags without bit 6, the owner, the callback.
                        let want = [
                            0,
                            w,
                            h,
                            0,
                            format,
                            TYPE_CODE_FORMAT[code as usize],
                            flags & !0x40,
                            owner,
                            callback,
                        ];
                        assert_eq!(c[0], want, "{ctx}");
                    }
                }
            }
        }
    }
    assert!(routes.iter().all(|&n| n > 0) && classes.iter().all(|&c| c));
    eprintln!(
        "pool_route: {} refused, {} created, {} pooled, all 25 lists read, bit-exact vs 0x448450",
        routes[0], routes[1], routes[2]
    );
}

#[test]
fn texture_create_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let caps = caps_block(&mut e);
    e.stub(SET_LAST_ERROR, 0, 1);
    // The device object: its vtable slot +0xb4 is the call that makes the texture.
    let device = e.alloc(0x10);
    let vtable = e.alloc(0x100);
    let create_slot = e.alloc(0x10);
    e.write_u32(device, vtable as u32);
    e.write_u32(vtable + 0xb4, create_slot as u32);
    e.write_u32(DEVICE, device as u32);
    e.stub(create_slot, 1, 10);
    let out = e.alloc(0x10);

    let sides = [
        0u32, 1, 7, 8, 9, 16, 24, 32, 256, 512, 1000, 1024, 4096, 5000, 8192,
    ];
    let mut seed = 0x58ab_1000u32;
    let (mut accepted, mut refused) = (0u32, 0u32);
    for max_side in [256u32, 512, 1024, 4096] {
        for (generates_mips, mode5) in [(0u32, 0u32), (1, 0), (0, 1), (3, 5)] {
            let cp = CreateCaps {
                type_supported: 1,
                max_side,
                generates_mips,
                mode5,
            };
            e.write_u32(caps + 0x50, 1);
            e.write_u32(caps + 0x60, max_side);
            e.write_u32(caps + 0x28, generates_mips);
            e.write_u32(caps + 0xa8, mode5);
            for &w in &sides {
                for &h in &sides {
                    for format in [0u32, 1, 4, 5, 6, 7, 8, 9, 10, 0xffff_ffff] {
                        let r = next(&mut seed);
                        let flags =
                            [0u32, 1, 3, 5, 0x40, 0x45, 0x23, r >> 20][(r as usize >> 3) % 8];
                        let data_format = [0u32, 1, 9, 10][(r as usize >> 9) % 4];
                        let has_callback = r & 1 != 0;
                        let req = CreateRequest {
                            width: w,
                            height: h,
                            format,
                            data_format,
                            flags,
                            has_callback,
                        };
                        e.write_u32(out, 0xabcd);
                        e.clear_calls();
                        // type 0, width, then height, 0, format, data format, flags, owner,
                        // callback and the out pointer.
                        e.call(
                            GX_TEX_CREATE,
                            0,
                            w,
                            &[
                                h,
                                0,
                                format,
                                data_format,
                                flags,
                                0x77,
                                u32::from(has_callback) * 0x1000,
                                out as u32,
                            ],
                        );
                        let device_calls =
                            e.calls().iter().filter(|c| c.addr == create_slot).count();
                        let want = create_accepts(&req, &cp);
                        assert_eq!(
                            device_calls == 1,
                            want,
                            "w={w} h={h} format={format} data={data_format} flags={flags:#x} \
                             callback={has_callback} caps={cp:?}"
                        );
                        if want {
                            accepted += 1;
                        } else {
                            refused += 1;
                            assert_eq!(e.read_u32(out), 0, "a refusal clears the out pointer");
                        }
                    }
                }
            }
        }
    }
    assert!(accepted > 0 && refused > 0);
    eprintln!("create_accepts: {accepted} accepted, {refused} refused, bit-exact vs 0x58ab10");
}

//! Two chat computations against `WoW.exe` in the emulator:
//!
//! * `chat_type_colour`: `GetChatTypeColor` (`0x49e990`) whole, on a registry of distinct bytes
//!   (`systems/ui/chatcolors.md`).
//! * `channel_list_print`: `PrintChannelMemberList` (`0x49c7e0`) whole (`systems/ui/chatchannels.md`),
//!   on a joined-channel slot built in scratch memory with its member list linked as the handler
//!   links it. The name cache (`0x55f080`) answers a record per known GUID, `SStrPrintf`
//!   (`0x64a7f0`) fills `%s` as `ISStrVPrintf` does, cutting a result that fills the buffer,
//!   `AddChatLine` (`0x49a870`) logs each line with its type and channel, and `SMemFree`
//!   (`0x646430`) logs each node. `SStrCopy`, `SStrPack`, `SStrLen` and the list unlink
//!   `0x4a3d70` run as the client's.
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use benilla_re_difftest::{Emu, RegisterX86, StubIo, Wow};
use benilla_re_ui::chat_lists::{
    channel_list_lines, colour_address, pack_colour, Member, ENTRY_SIZE, REGISTRY, TYPE_COUNT,
};

const GETRECORD: u64 = 0x55_f080;
const SSTRPRINTF: u64 = 0x64_a7f0;
const ADDCHATLINE: u64 = 0x49_a870;
const SMEMFREE: u64 = 0x64_6430;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*s).rotate_right(8)
}

#[test]
fn chat_type_colour() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let bytes: Vec<u8> = (0..94 * ENTRY_SIZE).map(|i| (i * 13 + 5) as u8).collect();
    emu.write(u64::from(REGISTRY), &bytes);
    let out = emu.alloc(4);
    let mut total = 0u32;
    let mut types: Vec<i32> = (-94..=95).collect();
    types.push(i32::MAX);
    for ty in types {
        emu.write_u32(out, 0x1234_5678);
        emu.call(0x49_e990, out as u32, ty as u32, &[]);
        let want = match colour_address(ty) {
            None => 0xffff_ffff,
            Some(at) => {
                let rgb = emu.read(u64::from(at), 3);
                pack_colour([rgb[0], rgb[1], rgb[2]])
            }
        };
        assert_eq!(emu.read_u32(out), want, "type {ty}");
        if (0..TYPE_COUNT).contains(&ty) {
            let base = (ty as u32 * ENTRY_SIZE + 0x40) as usize;
            assert_eq!(
                want,
                pack_colour([bytes[base], bytes[base + 1], bytes[base + 2]])
            );
        }
        total += 1;
    }
    eprintln!("chat_type_colour: {total} bit-exact (0x49e990 whole)");
}

/// `%s` into `[arg0]`, at most `arg1 - 1` bytes and a NUL, from the varargs at `arg3`.
fn mini_printf(io: &mut StubIo<'_, '_>) -> u32 {
    let dest = u64::from(io.arg(0));
    let size = io.arg(1) as usize;
    let fmt = io.read_cstr(u64::from(io.arg(2)));
    let mut out = Vec::new();
    let mut next_arg = 3;
    let mut i = 0;
    while i < fmt.len() {
        if fmt[i] == b'%' && fmt.get(i + 1) == Some(&b's') {
            out.extend(io.read_cstr(u64::from(io.arg(next_arg))));
            next_arg += 1;
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

/// The lines `AddChatLine` was given: text, type and channel.
type Lines = Rc<RefCell<Vec<(Vec<u8>, u32, Vec<u8>)>>>;

struct Rig {
    names: Rc<RefCell<HashMap<u64, u32>>>,
    lines: Lines,
    freed: Rc<RefCell<Vec<u32>>>,
    slot: u64,
    nodes: u64,
    records: u64,
}

fn rig(emu: &mut Emu) -> Rig {
    let names: Rc<RefCell<HashMap<u64, u32>>> = Rc::default();
    let lines: Lines = Rc::default();
    let freed: Rc<RefCell<Vec<u32>>> = Rc::default();
    {
        let names = Rc::clone(&names);
        emu.stub_fn(GETRECORD, 6, move |io| {
            assert_eq!(io.reg(RegisterX86::ECX), 0xc0_e228, "the name cache");
            assert_eq!((io.arg(3), io.arg(4), io.arg(5)), (0, 0, 0), "no callback");
            let guid = u64::from(io.arg(0)) | u64::from(io.arg(1)) << 32;
            names.borrow().get(&guid).copied().unwrap_or(0)
        });
    }
    emu.stub_fn(SSTRPRINTF, 0, mini_printf);
    {
        let lines = Rc::clone(&lines);
        emu.stub_fn(ADDCHATLINE, 10, move |io| {
            let text = io.read_cstr(io.reg(RegisterX86::ECX));
            let ty = io.reg(RegisterX86::EDX) as u32;
            let channel = io.read_cstr(u64::from(io.arg(2)));
            for i in [0, 1, 3, 4, 5, 6, 7, 8, 9] {
                assert_eq!(io.arg(i), 0, "AddChatLine argument {i}");
            }
            lines.borrow_mut().push((text, ty, channel));
            0
        });
    }
    {
        let freed = Rc::clone(&freed);
        emu.stub_fn(SMEMFREE, 4, move |io| {
            freed.borrow_mut().push(io.arg(0));
            1
        });
    }
    let slot = emu.alloc(0xa0);
    emu.write(slot + 4, b"World\0");
    Rig {
        names,
        lines,
        freed,
        slot,
        nodes: emu.alloc(0x18 * 64),
        records: emu.alloc(0x148 * 64),
    }
}

/// Links `n` nodes into the slot's list at `+0x84` as `0x49c690` does, at link offset 0, each
/// with its GUID and flags, and gives every cached member a name record.
fn build(emu: &mut Emu, r: &Rig, channel_flags: u8, members: &[(Member, bool)]) {
    let term = r.slot + 0x88;
    emu.write_u32(r.slot + 0x84, 0);
    emu.write(r.slot + 0x90, &[channel_flags]);
    r.names.borrow_mut().clear();
    let mut prev_link = term;
    for (i, (m, cached)) in members.iter().enumerate() {
        let node = r.nodes + 0x18 * i as u64;
        let guid = 0x0100_0000_0000_0000 | (i as u64 + 1);
        emu.write_u32(node, prev_link as u32);
        let next = if i + 1 < members.len() {
            (node + 0x18) as u32
        } else {
            term as u32 | 1
        };
        emu.write_u32(node + 4, next);
        emu.write(node + 8, &guid.to_le_bytes());
        emu.write(node + 0x10, &[m.flags]);
        if i == 0 {
            emu.write_u32(term + 4, node as u32);
        }
        prev_link = node;
        if *cached {
            let rec = r.records + 0x148 * i as u64;
            emu.write(rec, &[0u8; 0x148]);
            emu.write(rec, &m.name);
            emu.write(rec + 0x30, &m.second);
            r.names.borrow_mut().insert(guid, rec as u32);
        }
    }
    emu.write_u32(term, prev_link as u32);
    if members.is_empty() {
        emu.write_u32(term + 4, term as u32 | 1);
    }
}

fn word(s: &mut u32, len: usize, spaces: bool) -> Vec<u8> {
    (0..len)
        .map(|_| match next(s) % 40 {
            0 if spaces => b' ',
            1 if spaces => b'\t',
            k => b'a' + (k % 26) as u8,
        })
        .collect()
}

#[test]
fn channel_list_print() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let r = rig(&mut emu);
    let mut s = 0x49c7_e000u32;
    let mut total = 0u32;
    let (mut split, mut cut, mut waited, mut bound) = (0u32, 0u32, 0u32, 0u32);
    for case in 0..600u32 {
        let n = match case {
            0 => 0,
            1..=40 => case as usize,
            _ => (next(&mut s) % 60) as usize,
        };
        let channel_flags = [0u8, 2, 0xff][(case % 3) as usize];
        let missing = if case % 7 == 3 && n > 0 {
            Some((next(&mut s) as usize) % n)
        } else {
            None
        };
        let members: Vec<(Member, bool)> = (0..n)
            .map(|i| {
                let long = next(&mut s).is_multiple_of(5);
                let name_len = if long {
                    30 + (next(&mut s) % 17) as usize
                } else {
                    2 + (next(&mut s) % 11) as usize
                };
                // A long name with a second string of 260 to 279 characters takes the 0x130 bound
                // of the name buffer (`0x49c95e`).
                let second_len = if long && next(&mut s).is_multiple_of(4) {
                    260 + (next(&mut s) % 20) as usize
                } else if next(&mut s).is_multiple_of(3) {
                    (next(&mut s) % 20) as usize
                } else {
                    0
                };
                let m = Member {
                    name: word(&mut s, name_len, false),
                    second: word(&mut s, second_len, true),
                    flags: (next(&mut s) & 7) as u8,
                };
                (m, missing != Some(i))
            })
            .collect();
        build(&mut emu, &r, channel_flags, &members);
        r.lines.borrow_mut().clear();
        r.freed.borrow_mut().clear();
        emu.call(0x49_c7e0, r.slot as u32, 0, &[]);
        let lines = r.lines.borrow().clone();
        let freed = r.freed.borrow().clone();
        if missing.is_some() {
            assert!(
                lines.is_empty() && freed.is_empty(),
                "case {case}: waits for a name"
            );
            waited += 1;
            assert_eq!(
                emu.read_u32(r.slot + 0x8c),
                r.nodes as u32,
                "case {case}: list kept"
            );
        } else {
            let plain: Vec<Member> = members.iter().map(|(m, _)| m.clone()).collect();
            let want = channel_list_lines(channel_flags, &plain);
            let got: Vec<Vec<u8>> = lines.iter().map(|l| l.0.clone()).collect();
            assert_eq!(got, want, "case {case}: lines");
            split += u32::from(want.len() > 1);
            cut += u32::from(plain.iter().any(|m| m.name.len() + m.second.len() + 4 > 50));
            bound += u32::from(plain.iter().any(|m| {
                let kept = m
                    .second
                    .iter()
                    .filter(|&&c| c != b' ' && c != b'\t')
                    .count();
                m.name.len() + 1 + kept >= 0x130
            }));
            for l in &lines {
                assert_eq!(
                    (l.1, l.2.as_slice()),
                    (0x11, &b"World"[..]),
                    "case {case}: type, channel"
                );
            }
            let want_freed: Vec<u32> = (0..n).map(|i| (r.nodes + 0x18 * i as u64) as u32).collect();
            assert_eq!(freed, want_freed, "case {case}: every node freed in order");
            assert_eq!(
                emu.read_u32(r.slot + 0x8c),
                (r.slot + 0x88) as u32 | 1,
                "case {case}: list empty"
            );
        }
        total += 1;
    }
    assert!(
        split > 0 && cut > 0 && waited > 0 && bound > 0,
        "split {split}, cut {cut}, waited {waited}, bound {bound}"
    );
    eprintln!(
        "channel_list_print: {total} bit-exact (0x49c7e0 whole; {split} split, {cut} cut, {waited} waiting, {bound} at the bound)"
    );
}

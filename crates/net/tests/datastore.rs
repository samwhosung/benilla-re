//! The data store's reads and writes (`systems/net/messages.md`), run as sequences of calls on
//! three kinds of store, the state compared after every call.

mod common;

use benilla_re_difftest::Emu;
use benilla_re_net::{DataStore, Fetch};
use common::{Lcg, Wow};

/// The plain store's vtable and the network layer's inline store's.
const VT_HEAP: u32 = 0x7ff9e4;
const VT_INLINE: u32 = 0x80a7a8;
/// `SMemAlloc` (`__stdcall`, 4 arguments), `SMemReAlloc` (5) and `SErrSetLastError` (1), the
/// calls out of the store; the first two are replaced by an arena, the third is logged.
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const SERR_SET_LAST_ERROR: u64 = 0x64e850;

/// Positions a windowed store holds, all backed by one buffer.
const WINDOW_SPAN: u32 = 0x1000;
/// The window widths the tests give [`Fetch::Window`] stores.
const WIDTHS: [u32; 4] = [1, 4, 7, 16];
/// The arena's start, where a heap store's first buffer sits, and the bump allocator's start.
const ARENA_SIZE: u64 = 0x200000;
const BUMP_OFFSET: u64 = 0x10000;

/// One call on a store.
#[derive(Clone, Debug)]
enum Op {
    /// A fixed-width `Put`: the function and the bytes.
    Put(u64, Vec<u8>),
    PutF32(u32),
    /// A `Set` at a position: the function, the position, the bytes.
    SetAt(u64, u32, Vec<u8>),
    /// `PutArray` (`0x418570`) or `PutData` (`0x418c10`): the function and the bytes, `None` for a
    /// null source with the count.
    PutArray(u64, Option<Vec<u8>>, u32),
    PutCString(Option<Vec<u8>>),
    /// A fixed-width `Get`: the function and the width.
    Get(u64, u32),
    /// `GetData` (`0x419400`) or its forwarder (`0x419aa0`): the function, the count, whether the
    /// destination is null.
    GetData(u64, u32, bool),
    GetCString(u32, bool),
    InSitu(u32),
    IsRead,
}

struct Rig<'a> {
    e: Emu<'a>,
    obj: u64,
    backing: u64,
    arena: u64,
    src: u64,
    dst: u64,
    out: u64,
    /// The vtable of a [`Fetch::Window`] store of each width in [`WIDTHS`].
    window_vt: Vec<u64>,
}

/// A read fetch that moves the window to `[pos, pos + w)` with the buffer at
/// `backing + pos`, refusing more than `w` bytes; `ret 0x14`.
fn read_island(w: u32, backing: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, 0x08, 0x3d]; // mov eax,[esp+8]; cmp eax,imm32
    c.extend_from_slice(&w.to_le_bytes());
    c.extend_from_slice(&[0x77, 0x27]); // ja fail
    c.extend_from_slice(&write_body(w, backing));
    c.extend_from_slice(&[0xc2, 0x14, 0x00, 0x31, 0xc0, 0xc2, 0x14, 0x00]);
    c
}

/// The window move both fetches share, up to the `ret`: `[&base] = pos`, `[&alloc] = w`,
/// `[&buffer] = backing + pos`, `eax = 1`.
fn write_body(w: u32, backing: u32) -> Vec<u8> {
    let mut c = vec![
        0x8b, 0x44, 0x24, 0x04, // mov eax,[esp+4]
        0x8b, 0x54, 0x24, 0x10, // mov edx,[esp+0x10]
        0x89, 0x02, // mov [edx],eax
        0x8b, 0x54, 0x24, 0x14, // mov edx,[esp+0x14]
        0xc7, 0x02, // mov dword [edx],imm32
    ];
    c.extend_from_slice(&w.to_le_bytes());
    c.push(0x05); // add eax,imm32
    c.extend_from_slice(&backing.to_le_bytes());
    c.extend_from_slice(&[0x8b, 0x54, 0x24, 0x0c, 0x89, 0x02]); // mov edx,[esp+0xc]; mov [edx],eax
    c.extend_from_slice(&[0xb8, 1, 0, 0, 0]); // mov eax,1
    c
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut e = Emu::new(wow);
        let obj = e.alloc(0x420);
        let backing = e.alloc(u64::from(WINDOW_SPAN) + 0x100);
        let arena = e.alloc(ARENA_SIZE);
        let src = e.alloc(0x800);
        let dst = e.alloc(0x800);
        let out = e.alloc(0x10);
        e.stub_alloc_sized(SMEM_ALLOC, arena + BUMP_OFFSET, arena + ARENA_SIZE, 0, 4);
        e.stub_realloc_sized(SMEM_REALLOC, 5);
        e.stub(SERR_SET_LAST_ERROR, 0, 1);
        let mut window_vt = Vec::new();
        for &w in &WIDTHS {
            let vt = e.alloc(0x40);
            let r = e.alloc(0x40);
            let wr = e.alloc(0x40);
            e.write(r, &read_island(w, backing as u32));
            let mut wcode = write_body(w, backing as u32);
            wcode.extend_from_slice(&[0xc2, 0x1c, 0x00]); // ret 0x1c
            e.write(wr, &wcode);
            e.write_u32(vt + 8, r as u32);
            e.write_u32(vt + 12, wr as u32);
            window_vt.push(vt);
        }
        Rig {
            e,
            obj,
            backing,
            arena,
            src,
            dst,
            out,
            window_vt,
        }
    }

    /// The address of the byte at position `p`, from the store's buffer and base.
    fn phys(&self, p: u32) -> u64 {
        let buffer = self.e.read_u32(self.obj + 4);
        let base = self.e.read_u32(self.obj + 8);
        u64::from(buffer.wrapping_sub(base).wrapping_add(p))
    }

    /// Writes the model's state into the emulated store.
    fn load(&mut self, m: &DataStore) {
        self.e.write(self.arena, &vec![0u8; ARENA_SIZE as usize]);
        self.e.reset_bump(self.arena + BUMP_OFFSET);
        self.e.clear_calls();
        let (vt, buffer) = match m.fetch {
            Fetch::Heap => (VT_HEAP, if m.alloc == 0 { 0 } else { self.arena as u32 }),
            Fetch::Inline => (
                VT_INLINE,
                if m.inline {
                    self.obj as u32 + 0x18
                } else {
                    self.arena as u32
                },
            ),
            Fetch::Window(w) => {
                let i = WIDTHS.iter().position(|&x| x == w).expect("width");
                (
                    self.window_vt[i] as u32,
                    (self.backing as u32).wrapping_add(m.base),
                )
            }
        };
        for (off, v) in [vt, buffer, m.base, m.alloc, m.size, m.read]
            .into_iter()
            .enumerate()
        {
            self.e.write_u32(self.obj + 4 * off as u64, v);
        }
        match m.fetch {
            Fetch::Window(_) => {
                let bytes: Vec<u8> = (0..WINDOW_SPAN).map(|p| m.byte(p)).collect();
                self.e.write(self.backing, &bytes);
            }
            _ if buffer != 0 => {
                let bytes: Vec<u8> = (0..m.alloc).map(|p| m.byte(p)).collect();
                self.e.write(u64::from(buffer), &bytes);
            }
            _ => {}
        }
    }

    /// Compares the emulated store with the model: the six fields as the model has them, the
    /// inline flag, the written bytes `[0, size)` and the errors set since [`Rig::load`].
    fn compare(&self, m: &DataStore, ctx: &str) {
        let f = |o: u64| self.e.read_u32(self.obj + o);
        assert_eq!(
            (f(8), f(12), f(16), f(20)),
            (m.base, m.alloc, m.size, m.read),
            "{ctx}: (base, alloc, size, read)"
        );
        if m.fetch == Fetch::Inline {
            assert_eq!(
                f(4) == self.obj as u32 + 0x18,
                m.inline,
                "{ctx}: inline buffer"
            );
        }
        if f(4) != 0 && m.size != 0 {
            let got = self.e.read(self.phys(0), m.size as usize);
            let want: Vec<u8> = (0..m.size).map(|p| m.byte(p)).collect();
            assert_eq!(got, want, "{ctx}: bytes");
        }
        let errors: Vec<u32> = self
            .e
            .calls()
            .iter()
            .filter(|c| c.addr == SERR_SET_LAST_ERROR)
            .map(|c| c.args[0])
            .collect();
        assert_eq!(errors, m.errors, "{ctx}: errors");
    }

    /// Runs one call on both and compares what it returns or writes outside the store.
    fn step(&mut self, m: &mut DataStore, op: &Op, ctx: &str) {
        const FILL: u8 = 0xcc;
        self.e.write(self.dst, &[FILL; 0x200]);
        let obj = self.obj as u32;
        match op {
            Op::Put(f, bytes) => {
                let mut v = bytes.clone();
                v.resize(8, 0);
                let lo = u32::from_le_bytes(v[0..4].try_into().unwrap());
                let hi = u32::from_le_bytes(v[4..8].try_into().unwrap());
                let args: Vec<u32> = if bytes.len() == 8 {
                    vec![lo, hi]
                } else {
                    vec![lo]
                };
                self.e.call(*f, obj, 0, &args);
                m.put(bytes);
            }
            Op::PutF32(bits) => {
                self.e.call(0x4183d0, obj, 0, &[*bits]);
                m.put_f32(*bits);
            }
            Op::SetAt(f, pos, bytes) => {
                let mut v = bytes.clone();
                v.resize(4, 0);
                let val = u32::from_le_bytes(v[0..4].try_into().unwrap());
                self.e.call(*f, obj, 0, &[*pos, val]);
                m.set_at(*pos, bytes);
            }
            Op::PutArray(f, src, n) => {
                let p = match src {
                    Some(b) => {
                        self.e.write(self.src, b);
                        self.src as u32
                    }
                    None => 0,
                };
                self.e.call(*f, obj, 0, &[p, *n]);
                m.put_array(src.as_deref(), *n);
            }
            Op::PutCString(s) => {
                let p = match s {
                    Some(b) => {
                        let mut z = b.clone();
                        z.push(0);
                        self.e.write(self.src, &z);
                        self.src as u32
                    }
                    None => 0,
                };
                self.e.call(0x418430, obj, 0, &[p]);
                m.put_cstring(s.as_deref());
            }
            Op::Get(f, n) => {
                self.e.call(*f, obj, 0, &[self.dst as u32]);
                let want = m.get(*n).unwrap_or_else(|| vec![FILL; *n as usize]);
                assert_eq!(self.e.read(self.dst, *n as usize), want, "{ctx}: value");
            }
            Op::GetData(f, n, null) => {
                let p = if *null { 0 } else { self.dst as u32 };
                self.e.call(*f, obj, 0, &[p, *n]);
                let mut want = vec![FILL; *n as usize];
                m.get_data(&mut want, *null);
                assert_eq!(self.e.read(self.dst, *n as usize), want, "{ctx}: data");
            }
            Op::GetCString(cap, null) => {
                let p = if *null { 0 } else { self.dst as u32 };
                self.e.call(0x4191b0, obj, 0, &[p, *cap]);
                let mut want = vec![FILL; *cap as usize];
                m.get_cstring(&mut want, *null);
                assert_eq!(self.e.read(self.dst, *cap as usize), want, "{ctx}: string");
            }
            Op::InSitu(n) => {
                self.e.write_u32(self.out, 0xdead_beef);
                self.e.call(0x419ac0, obj, 0, &[self.out as u32, *n]);
                let got = self.e.read_u32(self.out);
                let want = match m.get_in_situ(*n) {
                    Some(pos) => self.phys(pos) as u32,
                    None => 0,
                };
                assert_eq!(got, want, "{ctx}: pointer");
            }
            Op::IsRead => {
                let got = self.e.call(0x401d10, obj, 0, &[]);
                assert_eq!(got, m.is_read(), "{ctx}: is-read");
            }
        }
        self.compare(m, ctx);
    }
}

/// Bytes with a NUL about one in eight, so strings end at every distance.
fn bytes(g: &mut Lcg, n: usize) -> Vec<u8> {
    (0..n)
        .map(|_| {
            let v = g.next() >> 16;
            if v.is_multiple_of(8) {
                0
            } else {
                v as u8
            }
        })
        .collect()
}

/// A count: mostly small, with 0, 1 and counts that cross a window or the inline buffer.
fn count(g: &mut Lcg) -> u32 {
    match g.next() >> 28 {
        0 => 0,
        1 => 1,
        2 => 0x100 + (g.next() >> 16) % 0x340,
        _ => (g.next() >> 16) % 40,
    }
}

/// A starting store of the kind: its window, size, cursor (at 0, inside, at the size, or latched
/// one past it) and bytes. A plain store's bytes past its window, which it does not hold, read
/// as 0 on both sides.
fn start(g: &mut Lcg, fetch: Fetch) -> DataStore {
    let mut m = DataStore::new(fetch);
    let cursor = |g: &mut Lcg, size: u32| match g.next() >> 29 {
        0 => 0,
        1 => size,
        2 => size + 1,
        _ => (g.next() >> 8) % (size + 1),
    };
    match fetch {
        Fetch::Heap => {
            if g.next() >> 30 != 0 {
                m.alloc = [0x100, 0x200, 0x300][(g.next() >> 16) as usize % 3];
                m.size = (g.next() >> 8) % (m.alloc + 1);
                m.data = bytes(g, m.alloc as usize);
                // A size past the window: a read there asks slot 2, which refuses.
                if g.next() >> 29 == 0 {
                    m.size = m.alloc + 1 + (g.next() >> 8) % 0x40;
                    m.read = m.alloc - 2 + (g.next() >> 8) % (m.size - m.alloc + 3);
                    return m;
                }
            }
            m.read = cursor(g, m.size);
        }
        Fetch::Inline => {
            m.inline = true;
            m.alloc = 0x400;
            m.size = if g.next() >> 31 == 0 {
                (g.next() >> 8) % 0x40
            } else {
                0x3c0 + (g.next() >> 8) % 0x41
            };
            m.data = bytes(g, m.alloc as usize);
            m.read = cursor(g, m.size);
        }
        Fetch::Window(w) => {
            m.size = (g.next() >> 8) % (WINDOW_SPAN / 2);
            m.base = (g.next() >> 8) % (m.size + 1);
            // A window wider than the vtable moves: a `GetData` chunk that fills it is refused.
            m.alloc = if g.next() >> 30 == 0 {
                w + 1 + (g.next() >> 8) % 32
            } else {
                w
            };
            m.data = bytes(g, WINDOW_SPAN as usize);
            m.read = cursor(g, m.size);
        }
    }
    m
}

fn op(g: &mut Lcg, m: &DataStore) -> Op {
    const PUTS: [(u64, usize); 8] = [
        (0x418070, 1),
        (0x4180d0, 2),
        (0x418130, 2),
        (0x418190, 4),
        (0x4181f0, 4),
        (0x4182b0, 4),
        (0x418370, 8),
        (0x4183d0, 4),
    ];
    const SETS: [(u64, usize); 4] = [(0x417cd0, 1), (0x417d70, 2), (0x417dc0, 4), (0x417e10, 4)];
    const GETS: [(u64, u32); 8] = [
        (0x418cb0, 1),
        (0x418d30, 2),
        (0x418db0, 2),
        (0x418e30, 4),
        (0x418eb0, 4),
        (0x418fb0, 4),
        (0x4190b0, 8),
        (0x419130, 4),
    ];
    let pick = |g: &mut Lcg, k: u32| ((g.next() >> 16) % k) as usize;
    match g.next() % 16 {
        0 | 1 => {
            let (f, n) = PUTS[pick(g, 8)];
            if f == 0x4183d0 {
                // No signalling NaN: the processor's `fld` quiets one, the emulator's keeps it.
                let specials = [0x7fc0_0001, 0xffc0_0000, 0x7f80_0000, 0x8000_0000, 1, 0];
                let bits = if g.next() >> 30 == 0 {
                    specials[pick(g, 6)]
                } else {
                    g.next()
                };
                Op::PutF32(benilla_re_net::quiet_f32(bits))
            } else {
                Op::Put(f, bytes(g, n))
            }
        }
        2 => {
            let (f, n) = SETS[pick(g, 4)];
            let limit = m.size.saturating_add(8).max(1);
            Op::SetAt(f, (g.next() >> 8) % limit, bytes(g, n))
        }
        3 | 4 => {
            let f = if g.next() >> 31 == 0 {
                0x418570
            } else {
                0x418c10
            };
            let n = count(g);
            if g.next() >> 28 == 0 {
                Op::PutArray(f, None, n)
            } else {
                Op::PutArray(f, Some(bytes(g, n as usize)), n)
            }
        }
        5 => {
            if g.next() >> 28 == 0 {
                Op::PutCString(None)
            } else {
                let n = (g.next() >> 16) as usize % 40;
                Op::PutCString(Some(bytes(g, n).into_iter().map(|b| b | 1).collect()))
            }
        }
        6..=9 => {
            let (f, n) = GETS[pick(g, 8)];
            Op::Get(f, n)
        }
        10 | 11 => {
            let f = if g.next() >> 31 == 0 {
                0x419400
            } else {
                0x419aa0
            };
            Op::GetData(f, count(g).min(0x1ff), g.next() >> 28 == 0)
        }
        12 | 13 => Op::GetCString((g.next() >> 16) % 48, g.next() >> 28 == 0),
        14 => {
            let n = match g.next() >> 29 {
                0 => 0xffff_ffff,
                1 => 0x8000_0000,
                2 => 0,
                _ => (g.next() >> 16) % 24,
            };
            Op::InSitu(n)
        }
        _ => Op::IsRead,
    }
}

/// Runs `sequences` sequences of 40 calls on stores of each kind in `kinds`, comparing after
/// every call.
fn run(kinds: &[Fetch], sequences: u32, seed: u32) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut g = Lcg(seed);
    for &kind in kinds {
        for s in 0..sequences {
            let mut m = start(&mut g, kind);
            rig.load(&m);
            for i in 0..40 {
                let o = op(&mut g, &m);
                m.errors.clear();
                rig.e.clear_calls();
                let ctx = format!("{kind:?} sequence {s} call {i} {o:?}");
                rig.step(&mut m, &o, &ctx);
            }
        }
    }
}

/// Reads and writes on the plain store, whose buffer grows by 256-byte steps and whose reads
/// never move the window.
#[test]
fn datastore_heap() {
    run(&[Fetch::Heap], 300, 0x0bad_5eed);
}

/// Reads and writes on the network layer's store, which starts in its own 1,024-byte buffer and
/// moves to the heap at the first write past it.
#[test]
fn datastore_inline() {
    run(&[Fetch::Inline], 300, 0x1a2b_3c4d);
}

/// Reads and writes on stores whose window is 1, 4, 7 or 16 bytes, so that every read and write
/// moves it, and reads wider than the window fail.
#[test]
fn datastore_window() {
    let kinds: Vec<Fetch> = WIDTHS.iter().map(|&w| Fetch::Window(w)).collect();
    run(&kinds, 150, 0x7357_0001);
}

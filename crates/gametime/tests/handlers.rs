//! The server's game-time handlers against the exe (`systems/gametime.md`): `0x6c5de0` (opcode
//! `0x47`), `0x6c6010` (`0x43`) and `0x6c6120` (`0x45`), each run whole on a data store built the
//! way the dispatcher hands one over (the opcode's two bytes, then the body, the cursor after the
//! opcode). The store's readers, `IsRead`, the decode `0x642c30` and, for the speed, the clamp
//! `0x643820` run as the exe has them; the calls out of the game-time module and the console sinks
//! only record. The recorded calls and the value returned are compared with the model.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::RegisterX86::{ECX, EDX};
use benilla_re_difftest::{Emu, StubIo, Wow};
use benilla_re_gametime::{
    game_speed_set, game_time_set_packet, game_time_update, server_time, Call, Outcome, Time,
    SCALE_MAX_BITS, SCALE_MIN_BITS,
};
use std::cell::RefCell;
use std::rc::Rc;

const CLOCK: u64 = 0xce8538;
const CONSOLE_WRITE_A: u64 = 0x63cd00;
const CONSOLE_WRITE: u64 = 0x63cb50;
const SPRINTF: u64 = 0x64a7f0;
const SET: u64 = 0x6434d0;
const SYNC: u64 = 0x6436c0;
const BRIDGE: u64 = 0x6c5f50;
/// The plain data store's vtable: slot 2 moves the window, slot 5 is `IsRead` (`0x401d10`).
const VT_STORE: u32 = 0x7ff9e4;
const MALFORMED: u32 = 0x86c4d0;
const SPEED_LINE: u32 = 0x86c4ac;
const SERVER_TIME_LINE: u32 = 0x86c51c;
const FORMAT_TIME: u64 = 0x642c80;

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        (self.next() >> 8) % n
    }
}

/// A float that is not a signalling NaN (`docs/METHOD.md`, the harness: the emulator's `fld` keeps
/// one signalling where the processor quiets it).
fn quiet(bits: u32) -> u32 {
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        bits | 0x0040_0000
    } else {
        bits
    }
}

fn read_time(io: &StubIo, va: u64) -> Time {
    let b = io.read(va, 28);
    let mut t = [0i32; 7];
    for (i, slot) in t.iter_mut().enumerate() {
        *slot = i32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
    }
    t
}

struct Rig<'a> {
    emu: Emu<'a>,
    store: u64,
    buffer: u64,
    calls: Rc<RefCell<Vec<Call>>>,
    problems: Rc<RefCell<Vec<String>>>,
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut emu = Emu::new(wow);
        let store = emu.alloc(0x20);
        let buffer = emu.alloc(0x100);
        let calls: Rc<RefCell<Vec<Call>>> = Rc::default();
        let problems: Rc<RefCell<Vec<String>>> = Rc::default();
        // `0x63cd00(format, class, …)`: cdecl.
        let (c, p) = (calls.clone(), problems.clone());
        emu.stub_fn(CONSOLE_WRITE_A, 0, move |io| {
            if (io.arg(0), io.arg(1)) != (MALFORMED, 0) {
                p.borrow_mut().push("console line format or class".into());
            }
            c.borrow_mut().push(Call::Malformed {
                opcode: io.arg(2),
                size: io.arg(3),
                read: io.arg(4),
            });
            0
        });
        // `0x642c80(time, buffer, size)`: fastcall with the size on the stack, returns the buffer.
        let (c, p) = (calls.clone(), problems.clone());
        let formatted = Rc::new(RefCell::new(0u32));
        let f = formatted.clone();
        emu.stub_fn(FORMAT_TIME, 1, move |io| {
            if io.arg(0) != 0x80 {
                p.borrow_mut().push("format size".into());
            }
            let buffer = io.reg(EDX) as u32;
            *f.borrow_mut() = buffer;
            let time = read_time(io, io.reg(ECX));
            c.borrow_mut().push(Call::FormatTime { time });
            buffer
        });
        // `0x64a7f0(buffer, size, format, …)`: cdecl. The speed line passes the two doubles on the
        // stack; the server-time line passes the formatter's buffer.
        let (c, p) = (calls.clone(), problems.clone());
        let last = Rc::new(RefCell::new(0u32));
        let l = last.clone();
        emu.stub_fn(SPRINTF, 0, move |io| {
            let dbl =
                |i: u64| f64::from_bits(u64::from(io.arg(i)) | u64::from(io.arg(i + 1)) << 32);
            if io.arg(1) != 0x100 {
                p.borrow_mut().push("printf size".into());
            }
            *l.borrow_mut() = io.arg(0);
            match io.arg(2) {
                SPEED_LINE => c.borrow_mut().push(Call::SpeedLine {
                    old: dbl(3),
                    new: dbl(5),
                }),
                SERVER_TIME_LINE => {
                    if io.arg(3) != *formatted.borrow() {
                        p.borrow_mut()
                            .push("server time text is not the formatter's buffer".into());
                    }
                    c.borrow_mut().push(Call::TimeLine);
                }
                other => p.borrow_mut().push(format!("printf format {other:#x}")),
            }
            0
        });
        // `0x63cb50(text, class)`: fastcall.
        let (c, p) = (calls.clone(), problems.clone());
        emu.stub_fn(CONSOLE_WRITE, 0, move |io| {
            if io.reg(ECX) as u32 != *last.borrow() {
                p.borrow_mut()
                    .push("console text is not the printf buffer".into());
            }
            c.borrow_mut().push(Call::ConsoleWrite {
                class: io.reg(EDX) as u32,
            });
            0
        });
        // `0x6434d0(time, force)` and `0x6436c0(time, force)` on the clock: thiscall, `ret 8`.
        for (addr, sync) in [(SET, false), (SYNC, true)] {
            let (c, p) = (calls.clone(), problems.clone());
            emu.stub_fn(addr, 2, move |io| {
                if io.reg(ECX) != CLOCK {
                    p.borrow_mut().push("clock call on another object".into());
                }
                if io.arg(1) > 1 {
                    p.borrow_mut().push("force is not 0 or 1".into());
                }
                let time = read_time(io, u64::from(io.arg(0)));
                let force = io.arg(1) == 1;
                c.borrow_mut().push(if sync {
                    Call::SyncTime { time, force }
                } else {
                    Call::SetTime { time, force }
                });
                0
            });
        }
        // `0x6c5f50()`: cdecl, no arguments.
        let c = calls.clone();
        emu.stub_fn(BRIDGE, 0, move |_| {
            c.borrow_mut().push(Call::Bridge);
            0
        });
        Rig {
            emu,
            store,
            buffer,
            calls,
            problems,
        }
    }

    /// Builds the store the dispatcher gives a handler and runs it with the junk the handler
    /// ignores in its `ecx` and time argument.
    fn run(&mut self, handler: u64, opcode: u32, body: &[u8], g: &mut Lcg) -> u32 {
        let mut bytes = (opcode as u16).to_le_bytes().to_vec();
        bytes.extend_from_slice(body);
        self.emu.write(self.buffer, &bytes);
        let n = bytes.len() as u32;
        for (i, v) in [VT_STORE, self.buffer as u32, 0, n, n, 2]
            .into_iter()
            .enumerate()
        {
            self.emu.write_u32(self.store + 4 * i as u64, v);
        }
        self.calls.borrow_mut().clear();
        self.emu
            .call(handler, g.next(), opcode, &[g.next(), self.store as u32])
    }

    fn check(&self, got: u32, want: &Outcome, ctx: &str) {
        assert!(
            self.problems.borrow().is_empty(),
            "{ctx}: {:?}",
            self.problems.borrow()
        );
        assert_eq!(got, want.eax, "{ctx}: eax");
        assert_eq!(*self.calls.borrow(), want.calls, "{ctx}: calls");
        assert_eq!(
            self.emu.read_u32(CLOCK + 0x2c),
            want.scale.to_bits(),
            "{ctx}: scale"
        );
    }
}

/// Bodies of 0 to 10 bytes: a body of 4 is the valid one, shorter and longer ones are malformed.
fn body(g: &mut Lcg, first: u32) -> Vec<u8> {
    let len = match g.below(4) {
        0 => 4,
        1 => g.below(11),
        _ => 4,
    } as usize;
    let mut b: Vec<u8> = (0..len).map(|_| g.next() as u8).collect();
    if len >= 4 {
        b[..4].copy_from_slice(&first.to_le_bytes());
    }
    b
}

/// `0x6c5de0` on rates at and around the clamp's bounds, the specials and random bits, with the
/// clock's old scale random in range.
#[test]
fn gametime_6c5de0_game_speed_set() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut g = Lcg(0x6c5d_e000);
    let specials = [
        0u32,
        0x8000_0000,
        SCALE_MIN_BITS - 1,
        SCALE_MIN_BITS,
        SCALE_MIN_BITS + 1,
        SCALE_MAX_BITS - 1,
        SCALE_MAX_BITS,
        SCALE_MAX_BITS + 1,
        0x3f80_0000,
        0xbf80_0000,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0xffc0_0000,
        0x0000_0001,
        0x7f7f_ffff,
    ];
    let mut total = 0u32;
    for i in 0..6000u32 {
        let bits = if (i as usize) < specials.len() * 4 {
            specials[i as usize % specials.len()]
        } else {
            quiet(g.next())
        };
        let old = match g.below(4) {
            0 => SCALE_MIN_BITS,
            1 => SCALE_MAX_BITS,
            _ => (1.0f32 / 60.0 + (g.below(1 << 20) as f32 / (1 << 20) as f32) * 59.98).to_bits(),
        };
        rig.emu.write_u32(CLOCK + 0x2c, old);
        let b = body(&mut g, bits);
        let opcode = if g.below(3) == 0 {
            g.next() & 0xffff
        } else {
            0x47
        };
        let got = rig.run(0x6c5de0, opcode, &b, &mut g);
        let want = game_speed_set(opcode, &b, f32::from_bits(old));
        rig.check(
            got,
            &want,
            &format!("body {b:x?} old {old:#x} opcode {opcode:#x}"),
        );
        total += 1;
    }
    eprintln!("gametime_6c5de0_game_speed_set: {total} cases (read, clamp, speed line)");
}

/// `0x6c6010` on packed dates: the dword decoded and the sync called without `force`, or the
/// malformed line.
#[test]
fn gametime_6c6010_game_time_update() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut g = Lcg(0x6c60_1000);
    let mut total = 0u32;
    for i in 0..6000u32 {
        let packed = match i {
            0 => 0,
            1 => 0xffff_ffff,
            _ => g.next(),
        };
        rig.emu.write_u32(CLOCK + 0x2c, SCALE_MIN_BITS);
        let b = body(&mut g, packed);
        let opcode = if g.below(3) == 0 {
            g.next() & 0xffff
        } else {
            0x43
        };
        let got = rig.run(0x6c6010, opcode, &b, &mut g);
        let want = game_time_update(opcode, &b, f32::from_bits(SCALE_MIN_BITS));
        rig.check(got, &want, &format!("body {b:x?} opcode {opcode:#x}"));
        total += 1;
    }
    eprintln!("gametime_6c6010_game_time_update: {total} cases (read, decode, sync)");
}

/// `0x6c6120` on packed dates: the dword decoded, the clock set with `force` and the bridge run,
/// or the malformed line.
#[test]
fn gametime_6c6120_game_time_set() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut g = Lcg(0x6c61_2000);
    let mut total = 0u32;
    for i in 0..6000u32 {
        let packed = match i {
            0 => 0,
            1 => 0xffff_ffff,
            _ => g.next(),
        };
        rig.emu.write_u32(CLOCK + 0x2c, SCALE_MAX_BITS);
        let b = body(&mut g, packed);
        let opcode = if g.below(3) == 0 {
            g.next() & 0xffff
        } else {
            0x45
        };
        let got = rig.run(0x6c6120, opcode, &b, &mut g);
        let want = game_time_set_packet(opcode, &b, f32::from_bits(SCALE_MAX_BITS));
        rig.check(got, &want, &format!("body {b:x?} opcode {opcode:#x}"));
        total += 1;
    }
    eprintln!("gametime_6c6120_game_time_set: {total} cases (read, decode, set, bridge)");
}

/// `0x6c6080` on packed dates: the dword decoded and formatted into a 128-byte buffer, written as
/// `Server game time is %s` in class 0, the clock untouched; or the malformed line.
#[test]
fn gametime_6c6080_server_time() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut g = Lcg(0x6c60_8000);
    let mut total = 0u32;
    for i in 0..6000u32 {
        let packed = match i {
            0 => 0,
            1 => 0xffff_ffff,
            _ => g.next(),
        };
        rig.emu.write_u32(CLOCK + 0x2c, SCALE_MIN_BITS);
        let b = body(&mut g, packed);
        let opcode = if g.below(3) == 0 {
            g.next() & 0xffff
        } else {
            0x49
        };
        let got = rig.run(0x6c6080, opcode, &b, &mut g);
        let want = server_time(opcode, &b, f32::from_bits(SCALE_MIN_BITS));
        rig.check(got, &want, &format!("body {b:x?} opcode {opcode:#x}"));
        total += 1;
    }
    eprintln!("gametime_6c6080_server_time: {total} cases (read, decode, format, line)");
}

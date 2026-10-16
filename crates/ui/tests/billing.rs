//! Differential tests of the play-time warning against `WoW.exe` in the emulator
//! (`systems/ui/billing.md`):
//!
//! * `billing_minutes_left`: the slice `0x49569b` to `0x4956b7` of `CGGameUI::Update` (`fild`, the
//!   `0x804590` product, `ceil` `0x73fdf5`, `__ftol` `0x40a2b0`), all of it the exe's own code.
//! * `billing_arm`: the slice `0x48f636` to `0x48f67f` of `CGGameUI::InitializeGame`, with the
//!   clock `0x42c010` and the session getter `0x5ab490` stubbed.
//! * `billing_poll`: the poll `0x495679` to `0x495830` of `CGGameUI::Update`, whole, with the same
//!   two stubs and the callees that show something stubbed: `0x703f50`, `0x703e50`, `0x703bf0`,
//!   `0x64a7f0`, `0x49cae0` and `0x49a870`. Each stub logs its arguments and none of them touches
//!   the three words, which only these two functions name (`systems/ui/billing.md`).
//!
//! Inputs span the minute boundaries 0, 1, 5, 6, 30, 31, 600, 601 and 4320 exactly and a step
//! either side, the line clock before, on and after its time, the deadline before, on and after
//! the clock, the wrap of the millisecond clock, both latch values and each plan flag.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::billing::{
    arm, minutes_left, poll, Shown, Warning, ARM_MAX_MINUTES, MINUTES_PER_MS_BITS, MINUTE_MS,
};

const DEADLINE: u64 = 0xb4e360;
const NEXT_LINE: u64 = 0xb4e364;
const DIALOG: u64 = 0xb4e368;
const LATCH: u64 = 0x8435fc;
const EMPTY: u32 = 0x88_2748;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*s).rotate_right(8)
}

struct Frame {
    ebp: u64,
    esp: u64,
}

fn frame(emu: &mut Emu) -> Frame {
    let stack = emu.alloc(0x4000);
    let ebp = stack + 0x2000;
    Frame {
        ebp,
        esp: ebp - 0x820,
    }
}

fn set_frame(emu: &mut Emu, f: &Frame) {
    emu.set_reg(RegisterX86::EBP, f.ebp);
    emu.set_reg(RegisterX86::ESP, f.esp);
}

// Lefts around every whole minute, and the extremes of the dword.
fn boundary_lefts() -> Vec<u32> {
    let mut v = vec![
        0,
        1,
        2,
        59_999,
        60_000,
        60_001,
        u32::MAX,
        u32::MAX - 1,
        1 << 31,
    ];
    v.push((1 << 31) - 1);
    v.push((1 << 31) + 1);
    // Every multiple of a minute up to 2^32 ms (71,582.8 minutes), three milliseconds either way.
    for n in 0..=71_583u32 {
        let base = n.saturating_mul(MINUTE_MS);
        for d in [-3i64, -2, -1, 0, 1, 2, 3] {
            let l = i64::from(base) + d;
            if (0..=i64::from(u32::MAX)).contains(&l) {
                v.push(l as u32);
            }
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn billing_minutes_left() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    assert_eq!(
        u32::from_le_bytes(emu.read(0x80_4590, 4).try_into().unwrap()),
        MINUTES_PER_MS_BITS,
        "the factor at 0x804590"
    );

    let run = |emu: &mut Emu, left: u32| -> u32 {
        set_frame(emu, &f);
        // `mov [ebp-8],esi` with `esi = deadline - now`, `mov [ebp-4],ebx` with `ebx = 0`.
        emu.write(f.ebp - 8, &left.to_le_bytes());
        emu.write(f.ebp - 4, &0u32.to_le_bytes());
        emu.run(0x49_569b, 0x49_56b7);
        emu.get_reg(RegisterX86::EAX) as u32
    };

    let mut total = 0u32;
    for left in boundary_lefts() {
        assert_eq!(run(&mut emu, left), minutes_left(left), "left={left}");
        total += 1;
    }
    let mut s = 0x4956_9b00u32;
    for _ in 0..20_000 {
        let left = next(&mut s);
        assert_eq!(run(&mut emu, left), minutes_left(left), "left={left}");
        total += 1;
    }
    // The first left of every reading n, searched in the model from 100 ms below the minute (the
    // offset reaches 47 ms by n = 71,583), and the exe on it and on the left below it.
    for n in 1..=71_583u32 {
        let mut first = (MINUTE_MS * (n - 1)).saturating_sub(100);
        while minutes_left(first) < n {
            first += 1;
        }
        assert_eq!(
            minutes_left(first - 1),
            n - 1,
            "model: one below the first of {n}"
        );
        assert_eq!(run(&mut emu, first), n, "first left of {n}");
        assert_eq!(
            run(&mut emu, first - 1),
            n - 1,
            "one below the first of {n}"
        );
        total += 2;
    }
    eprintln!("billing_minutes_left: {total} bit-exact (slice 0x49569b..0x4956b7)");
}

#[test]
fn billing_arm() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let session = emu.alloc(0x2000);
    let now = Rc::new(Cell::new(0u32));
    {
        let now = Rc::clone(&now);
        emu.stub_fn(0x42_c010, 0, move |_| now.get());
    }
    emu.stub(0x5a_b490, session as u32, 0);

    let run = |emu: &mut Emu, prev: Warning, remaining: u32, flags: u8, at: u32| -> Warning {
        now.set(at);
        emu.write_u32(session + 0x1af4, remaining);
        emu.write(session + 0x1afc, &[flags]);
        emu.write_u32(DEADLINE, prev.deadline);
        emu.write_u32(NEXT_LINE, prev.next_line);
        emu.write_u32(DIALOG, prev.dialog_shown);
        set_frame(emu, &f);
        // `edi` is zero from `xor edi,edi` at `0x48f482`.
        emu.set_reg(RegisterX86::EDI, 0);
        emu.set_reg(RegisterX86::ESI, 0x1234_5678);
        emu.run(0x48_f636, 0x48_f67f);
        Warning {
            deadline: emu.read_u32(DEADLINE),
            next_line: emu.read_u32(NEXT_LINE),
            dialog_shown: emu.read_u32(DIALOG),
        }
    };

    let prev = Warning {
        deadline: 0xdead_0001,
        next_line: 0xdead_0002,
        dialog_shown: 7,
    };
    let mut total = 0u32;
    let mut check = |emu: &mut Emu, remaining: u32, flags: u8, at: u32| {
        let got = run(emu, prev, remaining, flags, at);
        assert_eq!(
            got,
            arm(prev, remaining, flags, at),
            "remaining={remaining} flags={flags:#x} now={at}"
        );
        total += 1;
    };
    let nows = [0u32, 1, 5_000, 0x10_0000, u32::MAX - 100, u32::MAX];
    let mut remainings = vec![
        0u32,
        1,
        2,
        4319,
        4320,
        4321,
        4322,
        0x7fff_ffff,
        0x8000_0000,
        u32::MAX,
    ];
    remainings.extend(1..=ARM_MAX_MINUTES + 3);
    for &remaining in &remainings {
        for flags in [0u8, 1, 2, 3, 8, 0x0a, 0x28, 0xff, 0xfd] {
            for &at in &nows {
                check(&mut emu, remaining, flags, at);
            }
        }
    }
    // The deadline that wraps to exactly zero reads as unarmed.
    check(&mut emu, 7, 0, 0u32.wrapping_sub(7 * MINUTE_MS));
    let mut s = 0x48f6_3600u32;
    for _ in 0..5_000 {
        let remaining = if next(&mut s) & 1 == 0 {
            next(&mut s) % 5000
        } else {
            next(&mut s)
        };
        check(&mut emu, remaining, next(&mut s) as u8, next(&mut s));
    }
    eprintln!("billing_arm: {total} bit-exact (slice 0x48f636..0x48f67f)");
}

/// One logged call of a stubbed callee.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Call {
    Dialog {
        id: u32,
        format: u32,
        minutes: u32,
    },
    Event {
        id: u32,
    },
    GetText {
        key: u32,
        ordinal: u32,
        gender: u32,
    },
    Format {
        buf: u32,
        size: u32,
        text: u32,
        minutes: u32,
    },
    Queue {
        kind: u32,
        text: u32,
        args: [u32; 11],
    },
    Add {
        text: u32,
        kind: u32,
        args: [u32; 10],
    },
}

#[derive(Default)]
struct Log {
    calls: Vec<Call>,
}

const TEXT_KEY: u32 = 0x84_33f0;
const FORMAT_D: u32 = 0x83_5154;

/// The shown events a poll's calls spell, checking every argument the call carries.
fn decode(calls: &[Call], buf: u32, fake_text: u32) -> Vec<Shown> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < calls.len() {
        match &calls[i] {
            Call::Dialog {
                id,
                format,
                minutes,
            } => {
                assert_eq!((*id, *format), (0x205, FORMAT_D), "BILLING_NAG_DIALOG call");
                out.push(Shown::Dialog(*minutes));
                i += 1;
            }
            Call::Event { id } => {
                assert_eq!(*id, 0x206, "IGR_BILLING_NAG_DIALOG call");
                out.push(Shown::InternetCafeDialog);
                i += 1;
            }
            Call::GetText {
                key,
                ordinal,
                gender,
            } => {
                assert_eq!(
                    (*key, *gender),
                    (TEXT_KEY, 0),
                    "GetText(BILLING_NAG_WARNING, nil, m)"
                );
                let Call::Format {
                    buf: b,
                    size,
                    text,
                    minutes,
                } = calls[i + 1]
                else {
                    panic!("GetText is followed by SStrPrintf: {calls:?}");
                };
                assert_eq!((b, size, text, minutes), (buf, 0x800, fake_text, *ordinal));
                match &calls[i + 2] {
                    Call::Queue { kind, text, args } => {
                        assert_eq!(
                            (*kind, *text),
                            (0xa, buf),
                            "the queued line is chat type 10"
                        );
                        assert_eq!(args[0..4], [0, 0, 0, 0]);
                        assert_eq!(
                            args[4..],
                            [1, 0, EMPTY, 0, 0, EMPTY, 0],
                            "the queued record's fields"
                        );
                        out.push(Shown::Line {
                            minutes: *ordinal,
                            held: true,
                        });
                    }
                    Call::Add { text, kind, args } => {
                        assert_eq!((*kind, *text), (0xa, buf), "the shown line is chat type 10");
                        assert_eq!(*args, [0; 10]);
                        out.push(Shown::Line {
                            minutes: *ordinal,
                            held: false,
                        });
                    }
                    other => panic!("a line ends in a queue or a show: {other:?}"),
                }
                i += 3;
            }
            other => panic!("unexpected call {other:?} in {calls:?}"),
        }
    }
    out
}

struct Poller {
    emu: Emu<'static>,
    f: Frame,
    session: u64,
    now: Rc<Cell<u32>>,
    log: Rc<RefCell<Log>>,
    fake_text: u32,
}

impl Poller {
    fn new(wow: &Wow) -> Poller {
        let mut emu = Emu::new(wow);
        let f = frame(&mut emu);
        let session = emu.alloc(0x2000);
        let fake_text = emu.alloc(0x40) as u32;
        let now = Rc::new(Cell::new(0u32));
        let log = Rc::new(RefCell::new(Log::default()));
        {
            let now = Rc::clone(&now);
            emu.stub_fn(0x42_c010, 0, move |_| now.get());
        }
        emu.stub(0x5a_b490, session as u32, 0);
        {
            let log = Rc::clone(&log);
            emu.stub_fn(0x70_3f50, 0, move |io| {
                log.borrow_mut().calls.push(Call::Dialog {
                    id: io.arg(0),
                    format: io.arg(1),
                    minutes: io.arg(2),
                });
                0
            });
        }
        {
            let log = Rc::clone(&log);
            emu.stub_fn(0x70_3e50, 0, move |io| {
                log.borrow_mut().calls.push(Call::Event {
                    id: io.reg(RegisterX86::ECX) as u32,
                });
                0
            });
        }
        {
            let log = Rc::clone(&log);
            emu.stub_fn(0x70_3bf0, 1, move |io| {
                log.borrow_mut().calls.push(Call::GetText {
                    key: io.reg(RegisterX86::ECX) as u32,
                    ordinal: io.reg(RegisterX86::EDX) as u32,
                    gender: io.arg(0),
                });
                fake_text
            });
        }
        {
            let log = Rc::clone(&log);
            emu.stub_fn(0x64_a7f0, 0, move |io| {
                let mut l = log.borrow_mut();
                l.calls.push(Call::Format {
                    buf: io.arg(0),
                    size: io.arg(1),
                    text: io.arg(2),
                    minutes: io.arg(3),
                });
                0
            });
        }
        {
            let log = Rc::clone(&log);
            emu.stub_fn(0x49_cae0, 11, move |io| {
                let mut args = [0u32; 11];
                for (i, a) in args.iter_mut().enumerate() {
                    *a = io.arg(i as u64);
                }
                log.borrow_mut().calls.push(Call::Queue {
                    kind: io.reg(RegisterX86::ECX) as u32,
                    text: io.reg(RegisterX86::EDX) as u32,
                    args,
                });
                0
            });
        }
        {
            let log = Rc::clone(&log);
            emu.stub_fn(0x49_a870, 10, move |io| {
                let mut args = [0u32; 10];
                for (i, a) in args.iter_mut().enumerate() {
                    *a = io.arg(i as u64);
                }
                log.borrow_mut().calls.push(Call::Add {
                    text: io.reg(RegisterX86::ECX) as u32,
                    kind: io.reg(RegisterX86::EDX) as u32,
                    args,
                });
                0
            });
        }
        Poller {
            emu,
            f,
            session,
            now,
            log,
            fake_text,
        }
    }

    /// One poll in the exe: the words in, the events and the words out.
    fn run(&mut self, w: Warning, at: u32, flags: u8, held: bool) -> (Warning, Vec<Shown>) {
        self.now.set(at);
        self.log.borrow_mut().calls.clear();
        self.emu.write(self.session + 0x1afc, &[flags]);
        self.emu.write_u32(DEADLINE, w.deadline);
        self.emu.write_u32(NEXT_LINE, w.next_line);
        self.emu.write_u32(DIALOG, w.dialog_shown);
        self.emu.write_u32(LATCH, u32::from(held));
        set_frame(&mut self.emu, &self.f);
        // `ebx` is zero from the prologue; the other two are the caller's.
        self.emu.set_reg(RegisterX86::EBX, 0);
        self.emu.set_reg(RegisterX86::ESI, 0x1111_1111);
        self.emu.set_reg(RegisterX86::EDI, 0x2222_2222);
        self.emu.run(0x49_5679, 0x49_5830);
        let after = Warning {
            deadline: self.emu.read_u32(DEADLINE),
            next_line: self.emu.read_u32(NEXT_LINE),
            dialog_shown: self.emu.read_u32(DIALOG),
        };
        let calls = self.log.borrow().calls.clone();
        let buf = (self.f.ebp - 0x808) as u32;
        (after, decode(&calls, buf, self.fake_text))
    }
}

#[test]
fn billing_poll() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut p = Poller::new(&wow);
    let mut total = 0u32;
    // What the cases reached, so a silent gap in the inputs fails.
    let mut seen = [0u32; 8];
    let mut check = |p: &mut Poller, w: Warning, at: u32, flags: u8, held: bool| -> Warning {
        let (got_w, got) = p.run(w, at, flags, held);
        for e in &got {
            match *e {
                Shown::Dialog(_) => seen[0] += 1,
                Shown::InternetCafeDialog => seen[1] += 1,
                Shown::Line { minutes, held } => {
                    seen[if held { 2 } else { 3 }] += 1;
                    seen[if minutes <= 5 { 4 } else { 5 }] += 1;
                }
            }
        }
        if w.deadline != 0 && got_w.deadline == 0 {
            seen[6] += 1;
        }
        if w.next_line == 0 && got_w.next_line != 0 {
            seen[7] += 1;
        }
        let mut model = w;
        let want = poll(&mut model, at, flags, held);
        assert_eq!(
            (got_w, &got),
            (model, &want),
            "state={w:?} now={at} flags={flags:#x} held={held}"
        );
        total += 1;
        got_w
    };

    // States around every threshold: the deadline `left` ahead of the clock, the line clock
    // before, on and after it, the latch either way, each plan.
    let lefts_min = [0u32, 1, 2, 5, 6, 29, 30, 31, 599, 600, 601, 4320, 4321];
    let mut lefts: Vec<u32> = vec![0, 1, 59_999, u32::MAX, 1 << 31, (1 << 31) + 1];
    for m in lefts_min {
        for d in [-2i64, -1, 0, 1, 2] {
            let l = i64::from(m) * i64::from(MINUTE_MS) + d;
            if (0..=i64::from(u32::MAX)).contains(&l) {
                lefts.push(l as u32);
            }
        }
    }
    for at in [5u32, 1_000, 0x4000_0000, 0xffff_fff0] {
        for &left in &lefts {
            for next_off in [None, Some(-1i64), Some(0), Some(1), Some(100_000)] {
                for dialog_shown in [0u32, 1] {
                    for flags in [0u8, 8, 2, 0x28, 0xf7, 0xff] {
                        for held in [false, true] {
                            let w = Warning {
                                deadline: at.wrapping_add(left),
                                next_line: match next_off {
                                    None => 0,
                                    Some(o) => at.wrapping_add(o as u32),
                                },
                                dialog_shown,
                            };
                            check(&mut p, w, at, flags, held);
                        }
                    }
                }
            }
        }
    }
    // A deadline of zero is unarmed whatever else the words hold.
    for flags in [0u8, 8] {
        check(
            &mut p,
            Warning {
                deadline: 0,
                next_line: 5,
                dialog_shown: 0,
            },
            100,
            flags,
            false,
        );
    }

    // Whole countdowns from the arm: frames of assorted spacing, the state carried through the
    // exe's own words. The model and the exe must agree at every frame.
    let mut s = 0x4956_7900u32;
    for remaining in [1u32, 2, 3, 5, 6, 7, 29, 30, 31, 32, 35, 36, 600, 601, 602] {
        for flags in [0u8, 8] {
            for frame_ms in [1u32, 16, 997, 15_000, 59_999, 60_000, 299_999, 300_000] {
                for held_first in [false, true] {
                    let start = next(&mut s);
                    let mut w = arm(Warning::default(), remaining, flags, start);
                    let mut now = start;
                    let end = start.wrapping_add(remaining * MINUTE_MS + 3 * 300_000);
                    let mut polls = 0u32;
                    while (end.wrapping_sub(now) as i32) > 0 && polls < 1_500 {
                        now = now.wrapping_add(1 + next(&mut s) % (2 * frame_ms));
                        let held = held_first && polls < 2;
                        w = check(&mut p, w, now, flags, held);
                        polls += 1;
                    }
                }
            }
        }
    }
    // Fuzz over unconstrained words.
    for _ in 0..30_000 {
        let at = next(&mut s);
        let w = Warning {
            deadline: if next(&mut s).is_multiple_of(8) {
                0
            } else {
                at.wrapping_add(next(&mut s) % 40_000_000)
            },
            next_line: match next(&mut s) % 4 {
                0 => 0,
                1 => at.wrapping_sub(next(&mut s) % 400_000),
                2 => at.wrapping_add(next(&mut s) % 400_000),
                _ => next(&mut s),
            },
            dialog_shown: next(&mut s) & 1,
        };
        check(&mut p, w, at, next(&mut s) as u8, next(&mut s) & 1 == 1);
    }
    assert!(
        seen.iter().all(|&n| n > 0),
        "dialog, cafe dialog, held line, shown line, line at <=5, line above 5, deadline cleared, first poll: {seen:?}"
    );
    eprintln!("billing_poll: {total} bit-exact (0x495679..0x495830, whole), reached {seen:?}");
}

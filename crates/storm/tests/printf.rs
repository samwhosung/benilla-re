//! Storm's ordered printf, run whole: `vsnoprintf` (`0x6697d0`) and `ISStrVPrintf` (`0x64a820`),
//! with `ParseFormatSpecifier` (`0x669ab0`), the two spec rewrites (`0x669c50`, `0x669cc0`),
//! `__alloca_probe`, `_isdigit`, `_strchr`, `_strstr` and the copy `0x4100f0` running as the
//! client's. The C runtime's `_snprintf` (`0x740107`, `cdecl`) is a stub that renders the spec it
//! is handed and the dwords pushed after it, under that function's contract for the room it is
//! given: the text and a NUL when shorter, the text alone and its length when it fills the room,
//! the room's worth and -1 when longer. So the tests prove the binding of arguments to specs, the
//! rewrites and the cursor arithmetic, not the CRT's conversions.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_storm::printf::{isstrvprintf, vsnoprintf};

const VSNOPRINTF: u64 = 0x6697d0;
const ISSTRVPRINTF: u64 = 0x64a820;
const SNPRINTF: u64 = 0x740107;
/// `[0xc53f10]`, Storm option 10: nonzero selects the ordered formatter.
const ORDERED_OPTION: u64 = 0xc53f10;
/// The shared memory window: the buffer starts `LEAD` bytes in, so writes before it show.
const REGION: usize = 0x1000;
const LEAD: usize = 0x100;
const SENTINEL: u8 = 0xaa;

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// Whether the stub reads a second dword for this spec: an `I64` length or a floating conversion.
fn wide(spec: &[u8]) -> bool {
    spec.windows(3).any(|w| w == b"I64")
        || spec
            .last()
            .is_some_and(|b| matches!(b, b'E' | b'F' | b'G' | b'e' | b'f' | b'g'))
}

fn render(spec: &[u8], pushed: &[u32]) -> Vec<u8> {
    let mut out = b"[".to_vec();
    out.extend_from_slice(spec);
    out.extend_from_slice(format!("|{:x}", pushed[0]).as_bytes());
    if wide(spec) {
        out.extend_from_slice(format!(",{:x}", pushed[1]).as_bytes());
    }
    out.push(b']');
    out
}

/// `_snprintf`'s contract for a room of `count` bytes; returns what to write and the result.
fn contract(text: &[u8], count: i32) -> (Vec<u8>, i32) {
    let count = count.max(0) as usize;
    if text.len() < count {
        let mut w = text.to_vec();
        w.push(0);
        (w, text.len() as i32)
    } else if text.len() == count {
        (text.to_vec(), count as i32)
    } else {
        (text[..count].to_vec(), -1)
    }
}

fn snprintf_ours(mem: &mut [u8], pos: i64, count: i32, spec: &[u8], pushed: &[u32]) -> i32 {
    let (bytes, ret) = contract(&render(spec, pushed), count);
    mem[pos as usize..pos as usize + bytes.len()].copy_from_slice(&bytes);
    ret
}

struct Rig<'a> {
    emu: Emu<'a>,
    region: u64,
    fmt: u64,
    va: u64,
}

impl<'a> Rig<'a> {
    fn new(wow: &Wow) -> Rig<'a> {
        let mut emu = Emu::new(wow);
        let region = emu.alloc(REGION as u64);
        let fmt = emu.alloc(0x400);
        let va = emu.alloc(0x200);
        emu.stub_fn(SNPRINTF, 0, |io| {
            let dest = u64::from(io.arg(0));
            let count = io.arg(1) as i32;
            let spec = io.read_cstr(u64::from(io.arg(2)));
            let pushed = [io.arg(3), io.arg(4)];
            let (bytes, ret) = contract(&render(&spec, &pushed), count);
            io.write(dest, &bytes);
            ret as u32
        });
        emu.write(ORDERED_OPTION, &1u32.to_le_bytes());
        Rig {
            emu,
            region,
            fmt,
            va,
        }
    }

    /// Writes the format (zeros after it) and the `va_list`, and fills the window.
    fn load(&mut self, fmt: &[u8], args: &[u32]) {
        let mut f = fmt.to_vec();
        f.resize(0x400, 0);
        self.emu.write(self.fmt, &f);
        for (i, a) in args.iter().enumerate() {
            self.emu.write_u32(self.va + 4 * i as u64, *a);
        }
        self.emu.write(self.region, &[SENTINEL; REGION]);
    }
}

fn check_vsnoprintf(rig: &mut Rig<'_>, fmt: &[u8], args: &[u32], size: u32, ctx: &str) {
    rig.load(fmt, args);
    let dest = rig.region + LEAD as u64;
    let got = rig.emu.call(
        VSNOPRINTF,
        0,
        0,
        &[dest as u32, size, rig.fmt as u32, rig.va as u32],
    );
    let got_mem = rig.emu.read(rig.region, REGION);
    let mut mem = vec![SENTINEL; REGION];
    let want = vsnoprintf(&mut mem, LEAD, size, fmt, args, &mut snprintf_ours) as u32;
    assert_eq!(
        got,
        want,
        "{ctx}: return, fmt={:?} size={size}",
        String::from_utf8_lossy(fmt)
    );
    assert!(
        got_mem == mem,
        "{ctx}: memory, fmt={:?} size={size}\n binary={:?}\n   ours={:?}",
        String::from_utf8_lossy(fmt),
        String::from_utf8_lossy(&got_mem[LEAD - 8..LEAD + 120]),
        String::from_utf8_lossy(&mem[LEAD - 8..LEAD + 120])
    );
}

/// The fixed cases each rule of the binding rests on.
const PROBES: &[(&[u8], &str)] = &[
    (b"%2$s %s", "a plain spec after %2$ reads argument 3"),
    (b"%2$d", "argument 1 unnamed: read as 4 bytes"),
    (b"%3$I64d %1$d", "argument 2 unnamed between two named"),
    (b"%1$d %1$x %1$d", "one argument named three times"),
    (
        b"%1$s %1$lld",
        "the last spec decides the kind: 8 bytes for both",
    ),
    (b"%lld %I64x %llu %ld", "ll becomes I64; one l stays"),
    (b"%%|100%% %d %%", "%% takes no argument"),
    (b"%0$d %d", "%0$ reads argument 1"),
    (b"%f %e %g", "doubles"),
    (b"%-+ #08.3d|%5.2s|%c|%p|%S|%X", "flags, width, precision"),
    (b"%hd %zd %nq d %*d", "bytes the parser walks over"),
    (b"%12$s", "a high index"),
    (b"%1$2$d", "a second $ reads the digits since the first"),
    (
        b"abc%",
        "a lone % at the end reads the NUL and the byte after it",
    ),
    (
        b"\x80\xff literal high bytes %d",
        "high bytes in literal text",
    ),
    (b"", "empty"),
    (b"%s", "a cut conversion at dest: the NUL lands at dest[-1]"),
    (
        b"%9\xf7$d %d",
        "0xf7 is a digit worth -57: argument 33, then 34",
    ),
    (
        b"%9\xfb$s|%9\xfd$x",
        "0xfb and 0xfd are digits: arguments 37 and 39",
    ),
    (
        b"%1\xf7\xfb\xfdd %\xedd %\xeed",
        "the three in a width; 0xed and 0xee are not digits",
    ),
];

#[test]
fn vsnoprintf_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut s = 0x6697_d000u32;
    let mut cases = 0u32;
    // A quiet NaN double at dwords 2 and 3. A signalling one is left out: the x87 quiets it in
    // the `fld` at `0x6698f6`, which the emulator does not (`docs/METHOD.md`, The harness).
    let mut args: Vec<u32> = (0..64).map(|i| 0x1000 + i).collect();
    args[2] = 1;
    args[3] = 0x7ff8_0000;
    for &(fmt, ctx) in PROBES {
        for size in [0x200u32, 1, 2, 3, 8, 13, 24, 40] {
            check_vsnoprintf(&mut rig, fmt, &args, size, ctx);
            cases += 1;
        }
    }
    // Every room from 1 to 80 against a format of several specs: each spec meets an exact fit
    // (a NUL one past the buffer) and a cut (the position steps back one).
    let long: &[u8] = b"ab%sc%2$dd%1$I64x %% e%fgh%lld";
    for size in 1..=80u32 {
        check_vsnoprintf(&mut rig, long, &args, size, "every room");
        cases += 1;
    }
    // Random formats over the parser's classes, with indices whose widths agree per index.
    const CONVS: &[u8] = b"diuxXcCsSpPfeEgG";
    for n in 0..3000 {
        let wide_index: Vec<bool> = (0..8).map(|_| lcg(&mut s).is_multiple_of(3)).collect();
        let mut fmt = Vec::new();
        // The index the next plain spec takes, tracked as the first pass does.
        let mut next = 0usize;
        let pieces = 1 + lcg(&mut s) % 10;
        for _ in 0..pieces {
            match lcg(&mut s) % 6 {
                0 => fmt.extend_from_slice(b"%%"),
                1 => {
                    for _ in 0..1 + lcg(&mut s) % 6 {
                        let b = b" abcxyz.,:-=\x80\xfe"[(lcg(&mut s) % 14) as usize];
                        fmt.push(b);
                    }
                }
                _ => {
                    fmt.push(b'%');
                    // A numbered spec most of the time; a plain one takes whatever index follows.
                    let numbered = !lcg(&mut s).is_multiple_of(3);
                    let index = if numbered {
                        (lcg(&mut s) % 8) as usize
                    } else {
                        next
                    };
                    if numbered {
                        fmt.extend_from_slice(format!("{}$", index + 1).as_bytes());
                    }
                    next = index + 1;
                    for _ in 0..lcg(&mut s) % 3 {
                        fmt.push(b"-+ #0"[(lcg(&mut s) % 5) as usize]);
                    }
                    if lcg(&mut s).is_multiple_of(2) {
                        fmt.extend_from_slice(format!("{}", lcg(&mut s) % 12).as_bytes());
                    }
                    if lcg(&mut s).is_multiple_of(3) {
                        fmt.extend_from_slice(format!(".{}", lcg(&mut s) % 6).as_bytes());
                    }
                    // Every spec on an index has that index's width, so the last one agrees.
                    let wants_wide = wide_index.get(index).copied().unwrap_or(false);
                    let conv = if wants_wide {
                        match lcg(&mut s) % 3 {
                            0 => b"lld".to_vec(),
                            1 => b"I64x".to_vec(),
                            _ => vec![b"feEgG"[(lcg(&mut s) % 5) as usize]],
                        }
                    } else {
                        let c = CONVS[(lcg(&mut s) % 11) as usize];
                        if lcg(&mut s).is_multiple_of(4) && c != b's' {
                            vec![b'l', c]
                        } else {
                            vec![c]
                        }
                    };
                    fmt.extend_from_slice(&conv);
                }
            }
        }
        let size = match lcg(&mut s) % 4 {
            0 => 0x200,
            _ => 1 + lcg(&mut s) % 100,
        };
        let mut a: Vec<u32> = (0..64).map(|_| lcg(&mut s)).collect();
        if lcg(&mut s).is_multiple_of(4) {
            let at = (lcg(&mut s) % 62) as usize;
            a[at] = 7;
            a[at + 1] = 0xfff8_0000;
        }
        check_vsnoprintf(&mut rig, &fmt, &a, size, &format!("fuzz {n}"));
        cases += 1;
    }
    eprintln!("vsnoprintf: {cases} cases bit-exact vs WoW.exe @ 0x6697d0");
}

#[test]
fn isstrvprintf_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let args: Vec<u32> = (0..64).map(|i| 0x2000 + i).collect();
    let mut cases = 0u32;
    for &(fmt, ctx) in PROBES {
        for size in [0u32, 1, 2, 5, 17, 33, 0x200, 0x7fff_ffff] {
            rig.load(fmt, &args);
            let dest = rig.region + LEAD as u64;
            let got = rig.emu.call(
                ISSTRVPRINTF,
                dest as u32,
                size,
                &[rig.fmt as u32, rig.va as u32],
            );
            let got_mem = rig.emu.read(rig.region, REGION);
            let mut mem = vec![SENTINEL; REGION];
            let want = isstrvprintf(&mut mem, LEAD, size, fmt, &args, &mut snprintf_ours);
            assert_eq!(got, want, "{ctx}: return, size={size:#x}");
            assert!(got_mem == mem, "{ctx}: memory, size={size:#x}");
            cases += 1;
        }
    }
    eprintln!("isstrvprintf: {cases} cases bit-exact vs WoW.exe @ 0x64a820");
}

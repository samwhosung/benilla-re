//! The OpenGL device's arithmetic (`src/device.rs`) against the exe: each test runs the function, or a
//! slice of it, in the emulator on synthesized and fuzzed inputs and compares the results bit for bit.
//! These are the OpenGL device's functions (vtable `0x809af8`); their Direct3D twins are not
//! transcribed.

use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// `0x5a0000` — `__stdcall(src: *const [f32;16], dst: *mut [f32;16])`, `ret 8`. Copies
/// the row-major 4×4 matrix, `fchs`-negating the Z column (elements 2/6/10/14), `rep movs` to dst.
#[test]
fn z_flip_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F); // _PC_53 = WoW runtime (fld/fstp/fchs are exact regardless)
    let src = emu.alloc(0x40); // 16 × f32 input matrix
    let dst = emu.alloc(0x40); // 16 × f32 output

    let mut checked = 0u64;
    for m in synth_matrices() {
        for (i, &f) in m.iter().enumerate() {
            emu.write_u32(src + i as u64 * 4, f.to_bits());
        }
        // __stdcall: two stack args (src, dst); the callee pops them (ret 8) — the sentinel halt
        // does not depend on the post-return ESP, so `call` handles it.
        emu.call(0x5a0000, 0, 0, &[src as u32, dst as u32]);
        let mut binary = [0f32; 16];
        for (i, b) in binary.iter_mut().enumerate() {
            *b = emu.read_f32(dst + i as u64 * 4);
        }
        let ours = benilla_re_gx::z_flip(&m);
        for i in 0..16 {
            assert_eq!(
                binary[i].to_bits(),
                ours[i].to_bits(),
                "z_flip[{i}] m={m:?}: binary={:#010x} ours={:#010x}",
                binary[i].to_bits(),
                ours[i].to_bits()
            );
        }
        checked += 1;
    }
    eprintln!("z_flip: {checked} matrices bit-exact vs WoW.exe @ 0x5a0000");
}

/// `0x59c100` — byte→normalized-float. Diff the per-channel slice 0x59c13d–0x59c14d:
/// `fild [ebp-0xc]; fmul 1/255; fstp [ebp-0x24]` (the interleaved integer `mov`s prepare the next
/// channel and do not touch the FPU). Stage the int byte at `[ebp-0xc]`, read the float at `[ebp-0x24]`.
#[test]
fn color_norm_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let frame = emu.alloc(0x80);
    let ebp = frame + 0x40; // [ebp-0xc]=frame+0x34 (in), [ebp-0x24]=frame+0x1c (out)
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::ESP, frame);

    let mut checked = 0u64;
    for b in 0u8..=255 {
        emu.write_u32(ebp - 0xc, u32::from(b)); // the byte as an i32 at [ebp-0xc]
        emu.run(0x59c13d, 0x59c150); // fild; fmul 1/255; fstp [ebp-0x24]
        let binary = emu.read_f32(ebp - 0x24).to_bits();
        let ours = benilla_re_gx::color_norm(b).to_bits();
        assert_eq!(
            binary, ours,
            "color_norm({b}): binary={binary:#010x} ours={ours:#010x}"
        );
        checked += 1;
    }
    eprintln!("color_norm: {checked} bytes bit-exact vs WoW.exe @ 0x59c100");
}

/// `0x593040` — scalar-modulated packed-colour unpack. Diff the per-call kernel slice
/// 0x593135–0x59317d: pre-scale `s = scalar·(1/255)` (kept as an f64), then each byte `· s` → f32,
/// stored in the channel order `[b2, b1, b0, b3]`. Stage the packed `u32` at `[ebp+0x8]` and the scalar
/// at `[ebp-0x4]` (the helper `0x597ac0` normally fills these; the test stages past it), read the 4 floats.
#[test]
fn color_modulate_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let frame = emu.alloc(0x80);
    let ebp = frame + 0x40; // locals at [ebp-0x18..-0x4], the packed dword arg at [ebp+0x8]
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::ESP, frame);

    // scalars: the [0,1] intensities the device feeds, plus out-of-band edges.
    let scalars = [
        0.0f32,
        1.0,
        0.5,
        0.25,
        1.0 / 255.0,
        0.999_999,
        2.0,
        -1.0,
        0.003_921_57,
    ];
    // packed colours: the byte edges in every lane + an LCG fuzz.
    let mut packs: Vec<u32> = vec![
        0x0000_0000,
        0xffff_ffff,
        0x0001_7f80,
        0xff80_7f01,
        0x8040_2010,
        0x0102_0408,
    ];
    let mut s: u32 = 0x9e37_79b9;
    for _ in 0..4000 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        packs.push(s);
    }

    let mut checked = 0u64;
    for &packed in &packs {
        for &scalar in &scalars {
            emu.write_u32(ebp + 0x8, packed); // the four bytes at [ebp+0x8..0xb]
            emu.write_u32(ebp - 0x4, scalar.to_bits()); // the scalar float
            emu.run(0x593135, 0x59317d); // pre-scale + 4× (byte · s) -> 4 f32s
            let binary = [
                emu.read_f32(ebp - 0x18),
                emu.read_f32(ebp - 0x14),
                emu.read_f32(ebp - 0x10),
                emu.read_f32(ebp - 0xc),
            ];
            let ours = benilla_re_gx::color_modulate(packed, scalar);
            for i in 0..4 {
                assert_eq!(
                    binary[i].to_bits(),
                    ours[i].to_bits(),
                    "color_modulate({packed:#010x}, {scalar})[{i}]: binary={:#010x} ours={:#010x}",
                    binary[i].to_bits(),
                    ours[i].to_bits()
                );
            }
            checked += 1;
        }
    }
    eprintln!("color_modulate: {checked} (colour,scalar) pairs bit-exact vs WoW.exe @ 0x593040");
}

/// `0x59c730` — viewport NDC→pixel. Diff the core math slice 0x59c786–0x59c7cc: scale
/// each edge at 53-bit precision, `__ftol`-truncate, difference the truncated edges. Stage the two params at
/// `[ebp-0x14]`(py)/`[ebp-0x10]`(px) (jumping past the accessor + RT-branch into the NDC merge) and the
/// four scales at `[esi+0xf38..0xf44]`; read the results from `edi/ebx/ecx/eax` (x/y/w/h).
#[test]
fn viewport_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let dev = emu.alloc(0x1000); // the device struct (esi); scales live at +0xf38..0xf44
    let stack = emu.alloc(0x200);
    let esp = stack + 0x100; // room below for the four __ftol `call`s (cdecl)
    let ebp = stack + 0x180; // [ebp-0x14]=stack+0x16c, [ebp-0x10]=stack+0x170
    emu.set_reg(RegisterX86::ESI, dev);
    emu.set_reg(RegisterX86::EBP, ebp);

    // f32 in [lo, hi] from a u32 sample.
    let f = |s: u32, lo: f32, hi: f32| -> f32 {
        lo + (s >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
    };

    let mut s: u32 = 0xc0ff_ee11;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    // realistic device scales (screen/pixel dims) + a fuzz; params are NDC-ish [-1.5, 1.5]. Then
    // NaN, the infinities and products past `i32` and `i64`, where `__ftol` returns the low dword
    // of its qword, 0 for the indefinite.
    let mut checked = 0u64;
    let mut run = |emu: &mut Emu, px: f32, py: f32, sx0: f32, sx1: f32, sy0: f32, sy1: f32| {
        emu.write_u32(dev + 0xf38, sx0.to_bits());
        emu.write_u32(dev + 0xf3c, sx1.to_bits());
        emu.write_u32(dev + 0xf40, sy0.to_bits());
        emu.write_u32(dev + 0xf44, sy1.to_bits());
        emu.write_u32(ebp - 0x14, py.to_bits()); // the Y param (A)
        emu.write_u32(ebp - 0x10, px.to_bits()); // the X param (B)

        emu.set_reg(RegisterX86::ESP, esp); // reset: the slice ends after one un-popped `push eax`
        emu.run(0x59c786, 0x59c7cc); // NDC merge -> through the two edge subtractions
        let binary = [
            emu.get_reg(RegisterX86::EDI) as u32 as i32, // x0
            emu.get_reg(RegisterX86::EBX) as u32 as i32, // y0
            emu.get_reg(RegisterX86::ECX) as u32 as i32, // width
            emu.get_reg(RegisterX86::EAX) as u32 as i32, // height
        ];
        let ours = benilla_re_gx::viewport(px, py, sx0, sx1, sy0, sy1);
        assert_eq!(
            binary, ours,
            "viewport(px={px}, py={py}, sx0={sx0}, sx1={sx1}, sy0={sy0}, sy1={sy1})"
        );
        checked += 1;
    };
    for _ in 0..25_000 {
        let px = f(next(), -1.5, 1.5);
        let py = f(next(), -1.5, 1.5);
        let sx0 = f(next(), 0.0, 4096.0);
        let sx1 = f(next(), 0.0, 4096.0);
        let sy0 = f(next(), 0.0, 4096.0);
        let sy1 = f(next(), 0.0, 4096.0);
        run(&mut emu, px, py, sx0, sx1, sy0, sy1);
    }
    for bad in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0e30,
        -1.0e30,
        3.0e9,
        -3.0e9,
        4_294_967_296.0,
        1.0e19,
    ] {
        run(&mut emu, bad, 0.5, 800.0, 1600.0, 600.0, 1200.0);
        run(&mut emu, 0.5, bad, 800.0, 1600.0, 600.0, 1200.0);
        run(&mut emu, 0.5, 0.5, bad, 1600.0, bad, 1200.0);
        run(&mut emu, 0.5, 0.5, 800.0, bad, 600.0, bad);
        run(&mut emu, 1.5, -1.5, 3.0e9, bad, 3.0e9, bad);
    }
    eprintln!("viewport: {checked} rects bit-exact vs WoW.exe @ 0x59c730");
}

/// Synthesized 4×4 matrices: identity, single-value fills over the edge cases (±0, ±1, large, the
/// f32 extremes, a denormal), and a large LCG fuzz of finite/inf values. NaN is excluded — the x87
/// `fld`/`fstp` round-trip quiets a signaling NaN, but a matrix never holds one (not a real input).
fn synth_matrices() -> Vec<[f32; 16]> {
    let edges = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e30,
        -1e30,
        f32::MIN_POSITIVE,
        f32::from_bits(1), // smallest denormal
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        123.456,
        -789.012,
    ];
    let mut v = Vec::new();
    v.push([
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);
    for &e in &edges {
        v.push([e; 16]);
    }
    // a matrix whose every element is a distinct edge value
    let mut mixed = [0f32; 16];
    for (i, x) in mixed.iter_mut().enumerate() {
        *x = edges[i];
    }
    v.push(mixed);

    let mut s: u32 = 0x6a09_e667;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..3000 {
        let mut m = [0f32; 16];
        for x in m.iter_mut() {
            let mut f = f32::from_bits(next());
            while f.is_nan() {
                f = f32::from_bits(next()); // exclude NaN (fld/fstp quiets sNaN); inf/finite are fine
            }
            *x = f;
        }
        v.push(m);
    }
    v
}

/// The device-scale aspect-fit rect-edge family, `0x5910a0`, `0x5910e0`, `0x591120` and `0x591160`. Each is a
/// `__thiscall(this = rect)` that reads the runtime device-scale `f32` global `[0xc0ef60]`, and (when
/// it is non-zero) re-fits one rect edge from the opposite axis' extent: `trunc(ext / scale)` (the X
/// pair) or `trunc(ext · scale)` (the Y pair), at 53-bit precision, with the real `__ftol` (`0x40a2b0`) executing.
/// We stage the four `i32` fields in a scratch struct, write the global (it sits inside `.data`'s
/// VirtualSize so the harness maps it), `call`, and read the fields back.
fn check_aspect_fit(addr: u64, name: &str, ours_fn: fn([i32; 4], f32) -> [i32; 4]) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let st = emu.alloc(0x20); // the 4-int rect (ecx = this)

    let mut checked = 0u64;
    for (rect, scale) in aspect_fit_cases() {
        for (i, &v) in rect.iter().enumerate() {
            emu.write_u32(st + i as u64 * 4, v as u32);
        }
        emu.write_u32(0xc0ef60, scale.to_bits()); // device-scale global (.bss, inside .data's vsize)
        emu.call(addr, st as u32, 0, &[]); // __thiscall: ecx = this, no stack args; real __ftol runs
        let binary = [
            emu.read_u32(st) as i32,
            emu.read_u32(st + 4) as i32,
            emu.read_u32(st + 8) as i32,
            emu.read_u32(st + 0xc) as i32,
        ];
        let ours = ours_fn(rect, scale);
        assert_eq!(
            binary, ours,
            "{name}(rect={rect:?}, scale={scale}): binary={binary:?} ours={ours:?}"
        );
        checked += 1;
    }
    eprintln!("{name}: {checked} (rect,scale) cases bit-exact vs WoW.exe @ {addr:#x}");
}

/// (rect, scale) cases for the aspect-fit family: explicit edge rects × aspect-ratio scales (incl. the
/// `scale == 0` guard and negative scales) + a 20k LCG fuzz with coords in ±30k and scales in
/// `[0.02, 50)`, then scales that make the quotient or product NaN, infinite, past `i32` or past
/// `i64`, where `__ftol` returns the low dword of its qword, 0 for the indefinite.
fn aspect_fit_cases() -> Vec<([i32; 4], f32)> {
    let scales = [
        0.0f32,
        1.0,
        0.75,
        1.333_333,
        1.777_778,
        0.5625,
        2.0,
        0.5,
        0.1,
        10.0,
        3.0,
        -1.0,
        -0.75,
        0.003_921_57,
    ];
    let rects = [
        [0, 0, 0, 0],
        [0, 0, 1024, 768],
        [0, 0, 1920, 1080],
        [100, 200, 700, 500],
        [-400, -300, 400, 300],
        [10, 10, 11, 12],
        [-5000, -5000, 5000, 5000],
        [50000, -50000, -50000, 50000],
    ];
    let mut cases: Vec<([i32; 4], f32)> = Vec::new();
    for &rect in &rects {
        for &s in &scales {
            cases.push((rect, s));
        }
    }
    let mut st: u32 = 0xa5a5_1234;
    let mut next = || {
        st = st.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        st
    };
    // Scales that make the extent's quotient or product NaN, infinite, past `i32` or past `i64`.
    for &rect in &rects {
        for s in [
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            1.0e-30,
            -1.0e-30,
            1.0e-12,
            1.0e30,
            -1.0e30,
            1.0e8,
            1.0e17,
            1.0e-7,
        ] {
            cases.push((rect, s));
        }
    }
    let c = |n: u32| (n >> 8) as i32 % 60_000 - 30_000;
    for _ in 0..20_000 {
        let rect = [c(next()), c(next()), c(next()), c(next())];
        let s = 0.02 + (next() >> 8) as f32 / (1u32 << 24) as f32 * 49.98; // [0.02, 50)
        cases.push((rect, s));
    }
    cases
}

#[test]
fn aspect_fit_min_x_matches_binary() {
    check_aspect_fit(
        0x5910a0,
        "aspect_fit_min_x",
        benilla_re_gx::aspect_fit_min_x,
    );
}
#[test]
fn aspect_fit_max_x_matches_binary() {
    check_aspect_fit(
        0x5910e0,
        "aspect_fit_max_x",
        benilla_re_gx::aspect_fit_max_x,
    );
}
#[test]
fn aspect_fit_min_y_matches_binary() {
    check_aspect_fit(
        0x591120,
        "aspect_fit_min_y",
        benilla_re_gx::aspect_fit_min_y,
    );
}
#[test]
fn aspect_fit_max_y_matches_binary() {
    check_aspect_fit(
        0x591160,
        "aspect_fit_max_y",
        benilla_re_gx::aspect_fit_max_y,
    );
}

/// `0x591680` — display gamma-ramp builder. `__thiscall(this=table_base, gamma)`,
/// `ret 4`. The exe fills `3×256` `u16` over `[this+0..0x600)` with
/// `__ftol(pow(i/255, gamma)·65535)` (same word in all three channels). Emulate the WHOLE function —
/// its own x87 `pow` (`0x73f940`, `fyl2x`/`f2xm1`) and `__ftol` (`0x40a2b0`) execute in Unicorn — and
/// compare the `u16` table; the final truncation absorbs `pow`'s sub-ulp difference from the host's.
#[test]
fn gamma_ramp_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let table = emu.alloc(0x600 + 0x40); // 3×256 u16 = 0x600 bytes, padded

    // realistic gamma-slider exponents + a fuzz over (0.05, 5.0].
    let mut gammas: Vec<f32> = vec![1.0, 0.5, 2.0, 2.2, 1.0 / 2.2, 0.8, 1.5, 3.0, 0.1, 4.0];
    let mut s: u32 = 0x5151_3713;
    for _ in 0..40 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        gammas.push(0.05 + (s >> 8) as f32 / (1u32 << 24) as f32 * 4.95);
    }
    // For a positive exponent `pow(i/255, gamma)` lies in `[0, 1]`, so the `__ftol` input never
    // passes 65535. A negative or NaN exponent sends the exe's `pow` (`0x73f940`) down an error path
    // that faults in the emulator, so no probe reaches `__ftol` with an out-of-range input.

    let mut checked = 0u64;
    for g in gammas {
        emu.call(0x591680, table as u32, 0, &[g.to_bits()]); // thiscall: this=table, arg=gamma
        let mut binary = [0u16; 768];
        for (i, w) in binary.iter_mut().enumerate() {
            let b = emu.read(table + i as u64 * 2, 2);
            *w = u16::from_le_bytes([b[0], b[1]]);
        }
        let ours = benilla_re_gx::gamma_ramp(g);
        for i in 0..768 {
            assert_eq!(
                binary[i], ours[i],
                "gamma_ramp({g})[{i}]: binary={} ours={}",
                binary[i], ours[i]
            );
        }
        checked += 1;
    }
    eprintln!("gamma_ramp: {checked} gammas (3×256 words each) bit-exact vs WoW.exe @ 0x591680");
}

/// `0x591fd0` — viewport aspect ratio. Diff the tail 0x592021–0x592038:
/// `fild dword [eax+0x214]; fidiv dword [eax+0x218]; fstp dword [0xc0ef60]`. Stage width@`[eax+0x214]`
/// and height@`[eax+0x218]` (eax = a scratch struct), read the `f32` result from the global `[0xc0ef60]`
/// (in `.data`'s mapped bss tail). EAX is reset per case (the slice's `mov eax,1` clobbers it post-read).
#[test]
fn aspect_ratio_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let st = emu.alloc(0x300); // device struct; width@+0x214, height@+0x218

    let mut cases: Vec<(i32, i32)> = vec![
        (1024, 768),
        (1920, 1080),
        (1280, 1024),
        (800, 600),
        (640, 480),
        (1, 1),
        (1, 3),
        (16, 9),
        (4, 3),
        (0, 1),
        (-1024, 768),
        (1024, -768),
        (65535, 1),
    ];
    let mut s: u32 = 0x1357_9bdf;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..20_000 {
        let w = (next() >> 8) as i32 % 8193 - 4096; // [-4096, 4096]
        let mut h = (next() >> 8) as i32 % 4096 + 1; // [1, 4096] (never 0)
        if next() & 0x100 == 0 {
            h = -h;
        }
        cases.push((w, h));
    }

    let mut checked = 0u64;
    for (w, h) in cases {
        emu.write_u32(st + 0x214, w as u32);
        emu.write_u32(st + 0x218, h as u32);
        emu.set_reg(RegisterX86::EAX, st); // restore eax (the slice's `mov eax,1` clobbered it)
        emu.run(0x592021, 0x592038); // fild; fidiv; mov eax,1; fstp [0xc0ef60]
        let binary = emu.read_f32(0xc0ef60).to_bits();
        let ours = benilla_re_gx::aspect_ratio(w, h).to_bits();
        assert_eq!(
            binary, ours,
            "aspect_ratio({w}, {h}): binary={binary:#010x} ours={ours:#010x}"
        );
        checked += 1;
    }
    eprintln!("aspect_ratio: {checked} (w,h) bit-exact vs WoW.exe @ 0x591fd0");
}

/// `0x5926c0` — matrix Z-row negate. Diff the fmul block 0x5926f3–0x592723:
/// `fld [esi+k]; fmul dword [0x8030a0](=-1.0); fstp [esi+k]` for k=0x20,0x24,0x28,0x2c. Stage the four
/// floats at `[esi+0x20..0x2c]`, read them back negated.
#[test]
fn negate_z_row_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let mat = emu.alloc(0x40); // matrix; row-2 elements at +0x20..0x2c

    let edges = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e30,
        -1e30,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        123.456,
        -789.012,
    ];
    let mut rows: Vec<[f32; 4]> = Vec::new();
    for &e in &edges {
        rows.push([e; 4]);
    }
    let mut s: u32 = 0x2545_f491;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..5000 {
        let mut r = [0f32; 4];
        for x in r.iter_mut() {
            let mut f = f32::from_bits(next());
            while f.is_nan() {
                f = f32::from_bits(next()); // a matrix element is never NaN (not a real input)
            }
            *x = f;
        }
        rows.push(r);
    }

    let mut checked = 0u64;
    for row in rows {
        for (i, &f) in row.iter().enumerate() {
            emu.write_u32(mat + 0x20 + i as u64 * 4, f.to_bits());
        }
        emu.set_reg(RegisterX86::ESI, mat);
        emu.run(0x5926f3, 0x592723); // 4× (fld; fmul -1.0; fstp)
        let mut binary = [0f32; 4];
        for (i, b) in binary.iter_mut().enumerate() {
            *b = emu.read_f32(mat + 0x20 + i as u64 * 4);
        }
        let ours = benilla_re_gx::negate_z_row(&row);
        for i in 0..4 {
            assert_eq!(
                binary[i].to_bits(),
                ours[i].to_bits(),
                "negate_z_row({row:?})[{i}]: binary={:#010x} ours={:#010x}",
                binary[i].to_bits(),
                ours[i].to_bits()
            );
        }
        checked += 1;
    }
    eprintln!("negate_z_row: {checked} rows bit-exact vs WoW.exe @ 0x5926c0");
}

/// `0x593330` — vec3 in-place subtract. Diff the fsub block 0x593387–0x5933b7:
/// `dst[k] -= v[k]` for k=0,1,2 (`fld dst[k]; fsub v[k]; fstp dst[k]`, the third via an `[ebp-0x8]`
/// spill of `v[2]`). `esi` = the dst struct (vec3 at `[esi+0..8]`; the slice also touches `+0xc`/`+0x50`,
/// harmless), `ecx` = the offset vec3, `ebp` supplies the spill slot. Read `dst[0..3]` back.
#[test]
fn sub_vec3_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let dst = emu.alloc(0x60); // esi: vec3 at +0,+4,+8; touches +0xc, +0x50
    let v = emu.alloc(0x10); // ecx: the offset vec3
    let frame = emu.alloc(0x40);
    let ebp = frame + 0x20; // [ebp-0x8] spill slot = frame+0x18

    // finite-only operands (inf operands could make inf-inf -> a NaN whose payload differs; a vec3
    // position is always finite). finite minus finite may overflow to inf, which both sides round to
    // identically, so that edge is left in.
    let edges = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        100.0,
        -100.0,
        1e20,
        -1e20,
        f32::MIN_POSITIVE,
        f32::MAX,
        123.456,
        -789.012,
        16_777_216.0,
        0.000_001,
    ];
    let mut cases: Vec<([f32; 3], [f32; 3])> = Vec::new();
    for &a in &edges {
        for &b in &edges {
            cases.push(([a, a, a], [b, b, b]));
        }
    }
    let mut s: u32 = 0x7f4a_7c15;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    let fin = |mut x: u32| -> f32 {
        loop {
            let f = f32::from_bits(x);
            if f.is_finite() {
                return f;
            }
            x = x.wrapping_add(0x0010_0000); // step the exponent out of inf/NaN
        }
    };
    for _ in 0..6000 {
        let a = [fin(next()), fin(next()), fin(next())];
        let b = [fin(next()), fin(next()), fin(next())];
        cases.push((a, b));
    }

    let mut checked = 0u64;
    for (a, b) in cases {
        for i in 0..3 {
            emu.write_u32(dst + i as u64 * 4, a[i].to_bits());
            emu.write_u32(v + i as u64 * 4, b[i].to_bits());
        }
        emu.set_reg(RegisterX86::ESI, dst);
        emu.set_reg(RegisterX86::ECX, v);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.run(0x593387, 0x5933b7);
        let binary = [
            emu.read_f32(dst),
            emu.read_f32(dst + 4),
            emu.read_f32(dst + 8),
        ];
        let ours = benilla_re_gx::sub_vec3(&a, &b);
        for i in 0..3 {
            assert_eq!(
                binary[i].to_bits(),
                ours[i].to_bits(),
                "sub_vec3({a:?} - {b:?})[{i}]: binary={:#010x} ours={:#010x}",
                binary[i].to_bits(),
                ours[i].to_bits()
            );
        }
        checked += 1;
    }
    eprintln!("sub_vec3: {checked} pairs bit-exact vs WoW.exe @ 0x593330");
}

/// `0x59c820` — directional-light vector negate. Diff the negate slice 0x59c90c–0x59c930:
/// `fld [esi-0x4c]; fchs; fstp [ebp-0x20]` ×4 (the four `[esi-0x4c..-0x40]` light-vector floats). Stage
/// the vector below `esi`, read the negated floats from `[ebp-0x20..-0x14]`. The slice has one un-popped
/// `push ecx`, so reset `ESP` each iteration (as in the viewport test).
#[test]
fn light_dir_negate_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let src = emu.alloc(0x80); // light vector at [esi-0x4c..-0x40]
    let frame = emu.alloc(0x80); // output at [ebp-0x20..-0x14]
    let esi = src + 0x50; // [esi-0x4c]=src+4 .. [esi-0x40]=src+0x10
    let ebp = frame + 0x40; // [ebp-0x20]=frame+0x20 .. [ebp-0x14]=frame+0x2c
    emu.set_reg(RegisterX86::ESI, esi);
    emu.set_reg(RegisterX86::EBP, ebp);

    let mut checked = 0u64;
    for v in synth_vec4() {
        for (i, &f) in v.iter().enumerate() {
            emu.write_u32(esi - 0x4c + i as u64 * 4, f.to_bits());
        }
        emu.set_reg(RegisterX86::ESP, frame + 0x10); // reset: the slice's `push ecx` is un-popped
        emu.run(0x59c90c, 0x59c930); // fld; fchs; fstp ×4 -> [ebp-0x20..-0x14]
        let binary = [
            emu.read_f32(ebp - 0x20),
            emu.read_f32(ebp - 0x1c),
            emu.read_f32(ebp - 0x18),
            emu.read_f32(ebp - 0x14),
        ];
        let ours = benilla_re_gx::light_dir_negate(&v);
        for i in 0..4 {
            assert_eq!(
                binary[i].to_bits(),
                ours[i].to_bits(),
                "light_dir_negate[{i}] v={v:?}: binary={:#010x} ours={:#010x}",
                binary[i].to_bits(),
                ours[i].to_bits()
            );
        }
        checked += 1;
    }
    eprintln!("light_dir_negate: {checked} vectors bit-exact vs WoW.exe @ 0x59c820");
}

/// `0x59fe20`, the projection matrix load. Compare the two arithmetic slices:
/// (a) the copy+Z-row-negate 0x59fe2c–0x59fea0 (`fld [eax+k]; [fchs;] fstp [ebp-0x40+k]` ×16, `fchs`
/// on offsets 0x20/0x24/0x28/0x2c = indices 8/9/10/11), and (b) the FBO Y-flip 0x59ff8e–0x59ffc5
/// (`fld; fmul [0x8030a0]=-1.0; fstp` on indices 1/5/9/13). Both slices interleave un-popped pushes,
/// so reset `ESP` each iteration; slice (a) also reads `[esi+0x269c]` (esi←ecx), so point `ecx` at a
/// mapped device buffer.
#[test]
fn proj_matrix_load_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let mat = emu.alloc(0x40); // input 4×4 at [eax]
    let dev = emu.alloc(0x2800); // ecx: the slice reads [esi+0x269c] after `mov esi,ecx`
    let frame = emu.alloc(0x100); // stack matrix at [ebp-0x40..-0x4]
    let ebp = frame + 0x80;

    // (a) copy + Z-row negate (indices 8/9/10/11).
    let mut checked_a = 0u64;
    for m in synth_matrices() {
        for (i, &f) in m.iter().enumerate() {
            emu.write_u32(mat + i as u64 * 4, f.to_bits());
        }
        emu.set_reg(RegisterX86::EAX, mat);
        emu.set_reg(RegisterX86::ECX, dev);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, frame + 0x40); // below the locals; push esi/edi un-popped
        emu.run(0x59fe2c, 0x59fea0);
        let mut binary = [0f32; 16];
        for (i, b) in binary.iter_mut().enumerate() {
            *b = emu.read_f32(ebp - 0x40 + i as u64 * 4);
        }
        let ours = benilla_re_gx::proj_zrow_negate(&m);
        for i in 0..16 {
            assert_eq!(
                binary[i].to_bits(),
                ours[i].to_bits(),
                "proj_zrow_negate[{i}] m={m:?}: binary={:#010x} ours={:#010x}",
                binary[i].to_bits(),
                ours[i].to_bits()
            );
        }
        checked_a += 1;
    }

    // (b) Y-column scale by -1.0 (indices 1/5/9/13 → locals [ebp-0x3c]/[ebp-0x2c]/[ebp-0x1c]/[ebp-0xc]).
    let off = [ebp - 0x3c, ebp - 0x2c, ebp - 0x1c, ebp - 0xc];
    let mut checked_b = 0u64;
    for chunk in synth_f32_scalars().chunks(4) {
        if chunk.len() < 4 {
            continue;
        }
        for (k, &slot) in off.iter().enumerate() {
            emu.write_u32(slot, chunk[k].to_bits());
        }
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, frame + 0x40); // push 0x0 / push 0x404 are un-popped
        emu.run(0x59ff8e, 0x59ffc5);
        for (k, &slot) in off.iter().enumerate() {
            let binary = emu.read_f32(slot).to_bits();
            let ours = benilla_re_gx::proj_ycol_scale(chunk[k]).to_bits();
            assert_eq!(
                binary, ours,
                "proj_ycol_scale({})[{k}]: binary={binary:#010x} ours={ours:#010x}",
                chunk[k]
            );
        }
        checked_b += 1;
    }
    eprintln!(
        "proj_matrix_load: {checked_a} matrices (Z-row) + {checked_b}×4 scalars (Y-col) bit-exact vs WoW.exe @ 0x59fe20"
    );
}

/// `0x58ca60` — rect shrink test (case-0). Diff the compare slice 0x58cabf–0x58caef:
/// `fld [edx+0xc]; fsub [edx+0x4]` / `fld [ecx+0xc]; fsub [ecx+0x4]; fcompp` (height), then the same on
/// `[+0x8]-[+0]` (width), setting `bl`=1 iff `b` is strictly smaller than `a` in either. Point `edx`/`ecx`
/// at the two rects, run, read `bl` (the slice has no stack use). The result is observed as the boolean.
#[test]
fn rect_b_smaller_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let ra = emu.alloc(0x20); // rect a (edx) = [ebp+0xc] arg
    let rb = emu.alloc(0x20); // rect b: slice does `mov ecx,eax` @0x58cac2, so stage it in EAX
    emu.set_reg(RegisterX86::EDX, ra);

    let mut checked = 0u64;
    for (a, b) in synth_rect_pairs() {
        for i in 0..4 {
            emu.write_u32(ra + i as u64 * 4, a[i].to_bits());
            emu.write_u32(rb + i as u64 * 4, b[i].to_bits());
        }
        emu.set_reg(RegisterX86::EAX, rb); // re-arm: `fnstsw ax` clobbers EAX's low word each run
        emu.set_reg(RegisterX86::EBX, 0); // clear so bl is the only contributor
        emu.run(0x58cabf, 0x58caef); // fld/fsub/fcompp ×2 -> bl
        let binary = (emu.get_reg(RegisterX86::EBX) as u8) != 0;
        let ours = benilla_re_gx::rect_b_smaller(&a, &b);
        assert_eq!(
            binary, ours,
            "rect_b_smaller(a={a:?}, b={b:?}): binary={binary} ours={ours}"
        );
        checked += 1;
    }
    eprintln!("rect_b_smaller: {checked} rect pairs bit-exact vs WoW.exe @ 0x58ca60");
}

/// The f32 edge set shared by the negate/scale fuzzers (NaN excluded — `fld`/`fstp` quiets an sNaN and
/// these are never real inputs; ±0, ±inf, denormals, the extremes are all real matrix/vector values).
fn f32_edges() -> [f32; 16] {
    [
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e30,
        -1e30,
        f32::MIN_POSITIVE,
        f32::from_bits(1), // smallest denormal
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        123.456,
        -789.012,
    ]
}

/// Synthesized light 4-vectors: each edge filled, a mixed-edge vector, and an LCG fuzz (NaN excluded).
fn synth_vec4() -> Vec<[f32; 4]> {
    let edges = f32_edges();
    let mut v: Vec<[f32; 4]> = Vec::new();
    for &e in &edges {
        v.push([e; 4]);
    }
    v.push([edges[2], edges[7], edges[10], edges[1]]);
    let mut s: u32 = 0x1357_9bdf;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..3000 {
        let mut a = [0f32; 4];
        for x in a.iter_mut() {
            let mut f = f32::from_bits(next());
            while f.is_nan() {
                f = f32::from_bits(next());
            }
            *x = f;
        }
        v.push(a);
    }
    v
}

/// A flat stream of f32 scalars (edges + LCG fuzz, NaN excluded) for the Y-column scale, consumed 4 at
/// a time (one per the four lanes 1/5/9/13).
fn synth_f32_scalars() -> Vec<f32> {
    let mut v: Vec<f32> = f32_edges().to_vec();
    let mut s: u32 = 0x2468_ace0;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..4000 {
        let mut f = f32::from_bits(next());
        while f.is_nan() {
            f = f32::from_bits(next());
        }
        v.push(f);
    }
    v
}

/// Synthesized (a, b) rect pairs `[x0,y0,x1,y1]`: structured boundary cases (equal extents, b smaller
/// in one axis, b larger), NaN/inf coordinates (are unordered and not less in both the exe and Rust), and a
/// broad LCG fuzz over the full f32 domain (the difference of two `f32` is always representable, so the
/// 53-bit `fsub` and `fcompp` matches the `f64` subtract + `<` bit-for-bit).
fn synth_rect_pairs() -> Vec<([f32; 4], [f32; 4])> {
    let mut v: Vec<([f32; 4], [f32; 4])> = vec![
        // equal extents -> not-less in either -> false
        ([0.0, 0.0, 100.0, 50.0], [10.0, 10.0, 110.0, 60.0]),
        // b shorter in height -> true
        ([0.0, 0.0, 100.0, 50.0], [0.0, 0.0, 100.0, 49.0]),
        // b narrower in width, taller in height -> true (width branch)
        ([0.0, 0.0, 100.0, 50.0], [0.0, 0.0, 99.0, 80.0]),
        // b larger both ways -> false
        ([0.0, 0.0, 100.0, 50.0], [0.0, 0.0, 200.0, 100.0]),
        // exact-equal deltas via different absolute coords -> false (boundary)
        ([5.0, 7.0, 15.0, 27.0], [100.0, 200.0, 110.0, 220.0]),
        // NaN coordinate -> unordered
        ([0.0, 0.0, 100.0, 50.0], [0.0, f32::NAN, 100.0, 50.0]),
        ([f32::NAN, 0.0, 100.0, 50.0], [0.0, 0.0, 100.0, 50.0]),
        // inf coords
        (
            [0.0, 0.0, f32::INFINITY, 50.0],
            [0.0, 0.0, f32::INFINITY, 50.0],
        ),
        (
            [0.0, f32::NEG_INFINITY, 100.0, 50.0],
            [0.0, 0.0, 100.0, 50.0],
        ),
    ];
    let mut s: u32 = 0x0bad_f00d;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    // realistic screen-coordinate fuzz: near-equal extents to stress the strict-less boundary.
    let coord = |s: u32| -> f32 { (s >> 8) as f32 / (1u32 << 24) as f32 * 4096.0 - 1024.0 };
    let d = |s: u32| -> f32 { ((s >> 8) as f32 / (1u32 << 24) as f32 - 0.5) * 4.0 };
    for _ in 0..6000 {
        let a = [coord(next()), coord(next()), coord(next()), coord(next())];
        // b = a perturbed by a small delta in each lane (sometimes zero) to hit ==, <, > extents.
        let b = [
            a[0] + d(next()),
            a[1] + d(next()),
            a[2] + d(next()),
            a[3] + d(next()),
        ];
        v.push((a, b));
    }
    // full-domain raw-bit fuzz (exercise the representability claim across all exponents).
    for _ in 0..3000 {
        let mut a = [0f32; 4];
        let mut b = [0f32; 4];
        for x in a.iter_mut() {
            *x = f32::from_bits(next());
        }
        for x in b.iter_mut() {
            *x = f32::from_bits(next());
        }
        v.push((a, b));
    }
    v
}

/// `0x5956f0` — the screen-space textured-quad corner transform. Run the slice
/// 0x5959a0–0x595b60 (the recip-guard + the four NDC values + the quad assembly), staging `edx` = the
/// rect (rw@+8, rh@+0xc), `esi` = the device (off1@+0x2888, off2@+0x288c, mode@+0x1fc), `[ebp-0x28]` =
/// `c1`, `[ebp-0x24]` = `c2`, `ebx` = 0 (the `fild qword` high dword), `ecx` = the vertex buffer; read
/// the four vertices' `(x, y)` at stride 0x18.
#[test]
fn quad_corners_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let vtx = emu.alloc(0x80); // ecx: 4 vertices × 0x18
    let dev = emu.alloc(0x2900); // esi: off1@+0x2888, off2@+0x288c, mode@+0x1fc
    let rect = emu.alloc(0x20); // edx: rw@+8, rh@+0xc
    let frame = emu.alloc(0x100);
    let ebp = frame + 0x80; // [ebp-0x28]=frame+0x58, [ebp-0x24]=frame+0x5c

    // (c1, c2, off1, off2, rh, rw, mode) cases: edges (zero/negative reciprocals → guard; c<off → u32
    // wrap), realistic viewport sizes, both centering modes, plus an LCG fuzz.
    let mut cases: Vec<(i32, i32, i32, i32, f32, f32, u32)> = vec![
        (100, 200, 0, 0, 768.0, 1024.0, 0),
        (100, 200, 0, 0, 768.0, 1024.0, 1),
        (0, 0, 0, 0, 1080.0, 1920.0, 0),
        (50, 50, 100, 100, 600.0, 800.0, 0), // c < off → u32 wrap
        (50, 50, 100, 100, 600.0, 800.0, 1),
        (10, 10, 0, 0, 0.0, 0.0, 0),       // recip guard → 0
        (10, 10, 0, 0, -480.0, -640.0, 1), // negative → guard → 0
        (1920, 1080, 0, 0, 1.0, 1.0, 0),
        (32, 32, 16, 16, 480.0, 640.0, 1),
    ];
    let mut s: u32 = 0x5956_f001;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..20_000 {
        let c1 = (next() >> 8) as i32 % 4000 - 200;
        let c2 = (next() >> 8) as i32 % 4000 - 200;
        let off1 = (next() >> 8) as i32 % 400 - 100;
        let off2 = (next() >> 8) as i32 % 400 - 100;
        let rh = (next() >> 8) as f32 / (1u32 << 24) as f32 * 2200.0 - 100.0; // incl. ≤0 (guard)
        let rw = (next() >> 8) as f32 / (1u32 << 24) as f32 * 2200.0 - 100.0;
        let mode = (next() >> 8) & 1;
        cases.push((c1, c2, off1, off2, rh, rw, mode));
    }

    let mut checked = 0u64;
    for (c1, c2, off1, off2, rh, rw, mode) in cases {
        emu.write_u32(rect + 0x8, rw.to_bits());
        emu.write_u32(rect + 0xc, rh.to_bits());
        emu.write_u32(dev + 0x2888, off1 as u32);
        emu.write_u32(dev + 0x288c, off2 as u32);
        emu.write_u32(dev + 0x1fc, mode);
        emu.write_u32(ebp - 0x28, c1 as u32);
        emu.write_u32(ebp - 0x24, c2 as u32);
        emu.set_reg(RegisterX86::EDX, rect); // [ebp-0x10] is reused as the fild hi-dword; edx holds rect
        emu.set_reg(RegisterX86::ESI, dev);
        emu.set_reg(RegisterX86::ECX, vtx);
        emu.set_reg(RegisterX86::EBX, 0); // the forced-zero high dword of `fild qword`
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, frame + 0x10); // slice has no push/pop/call; esp just mapped
        emu.run(0x5959a0, 0x595b60);
        let binary = [
            [emu.read_f32(vtx), emu.read_f32(vtx + 4)],
            [emu.read_f32(vtx + 0x18), emu.read_f32(vtx + 0x1c)],
            [emu.read_f32(vtx + 0x30), emu.read_f32(vtx + 0x34)],
            [emu.read_f32(vtx + 0x48), emu.read_f32(vtx + 0x4c)],
        ];
        let ours = benilla_re_gx::quad_corners(c1, c2, off1, off2, rh, rw, mode == 1);
        for i in 0..4 {
            for j in 0..2 {
                assert_eq!(
                    binary[i][j].to_bits(),
                    ours[i][j].to_bits(),
                    "quad_corners(c1={c1},c2={c2},off1={off1},off2={off2},rh={rh},rw={rw},mode={mode}) vtx{i}[{j}]: binary={:#010x} ours={:#010x}",
                    binary[i][j].to_bits(),
                    ours[i][j].to_bits()
                );
            }
        }
        checked += 1;
    }
    eprintln!("quad_corners: {checked} cases (4 corners each) bit-exact vs WoW.exe @ 0x5956f0");
}

/// `0x59baa0` — device render-state scale `x·(−65536)`. Run the state-case slice
/// 0x59bef7–0x59bf0a (`fld [ebp+0xc]; push ecx; fmul 65536; fmul -1; fstp [esp]`): stage the param at
/// `[ebp+0xc]`, read the `f32` result the `push ecx` placed at `esp-4`.
#[test]
fn scale_neg_65536_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let stack = emu.alloc(0x100);
    let ebp = stack + 0x40; // [ebp+0xc] = stack+0x4c
    let esp0 = stack + 0xc0; // push ecx → fstp [esp] writes at esp0-4

    let mut xs: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        4.0,
        -4.0,
        1e30,
        -1e30,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        123.456,
        -789.012,
        0.001,
    ];
    let mut s: u32 = 0x59ba_a001;
    for _ in 0..8000 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let mut f = f32::from_bits(s);
        while f.is_nan() {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            f = f32::from_bits(s);
        }
        xs.push(f);
    }

    let mut checked = 0u64;
    for x in xs {
        emu.write_u32(ebp + 0xc, x.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp0);
        emu.run(0x59bef7, 0x59bf0a); // fld; push ecx; fmul 65536; fmul -1; fstp [esp]
        let binary = emu.read_f32(esp0 - 4).to_bits();
        let ours = benilla_re_gx::scale_neg_65536(x).to_bits();
        assert_eq!(
            binary, ours,
            "scale_neg_65536({x}): binary={binary:#010x} ours={ours:#010x}"
        );
        checked += 1;
    }
    eprintln!("scale_neg_65536: {checked} values bit-exact vs WoW.exe @ 0x59baa0");
}

/// `0x59d120` — matrix reorient (transpose upper-3×3 + negate translation row). Run the
/// fixup slice 0x59d2e3–0x59d32b: stage `edi` = the row-major 4×4, read it back (swaps of (1,4)/(2,8)/
/// (6,9); elements 12/13/14 negated via `fmul -1.0`). Domain: `synth_matrices` (no NaN — the binary
/// `fld`/`fstp`-moves one side of each swap, exact for non-NaN, and a matrix never holds a NaN).
#[test]
fn matrix_reorient_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let mat = emu.alloc(0x40);

    let mut checked = 0u64;
    for m in synth_matrices() {
        for (i, &f) in m.iter().enumerate() {
            emu.write_u32(mat + i as u64 * 4, f.to_bits());
        }
        emu.set_reg(RegisterX86::EDI, mat);
        emu.run(0x59d2e3, 0x59d32b); // swaps + 3× (fld; fmul -1.0; fstp)
        let mut binary = [0f32; 16];
        for (i, b) in binary.iter_mut().enumerate() {
            *b = emu.read_f32(mat + i as u64 * 4);
        }
        let ours = benilla_re_gx::matrix_reorient(&m);
        for i in 0..16 {
            assert_eq!(
                binary[i].to_bits(),
                ours[i].to_bits(),
                "matrix_reorient[{i}] m={m:?}: binary={:#010x} ours={:#010x}",
                binary[i].to_bits(),
                ours[i].to_bits()
            );
        }
        checked += 1;
    }
    eprintln!("matrix_reorient: {checked} matrices bit-exact vs WoW.exe @ 0x59d120");
}

/// `0x59d350` — render-state switch whose only FP is the byte→float `×1/255` normalize,
/// inlined per case (17×, identical math to [`color_norm`] @ 0x59c100). Diff this function's own
/// representative slice 0x59d3ef–0x59d402 (`fild [ebp+8]; fmul [0x809ffc]; fstp [ebp-0x10]`): stage the
/// int byte at `[ebp+8]`, read the `f32` at `[ebp-0x10]`. Confirms 0x59d350's bytes match `color_norm`.
#[test]
fn color_norm_at_0x59d350_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let frame = emu.alloc(0x80);
    let ebp = frame + 0x40; // [ebp+8]=frame+0x48 (in), [ebp-0x10]=frame+0x30 (out)
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::ESP, frame);

    let mut checked = 0u64;
    for b in 0u8..=255 {
        emu.write_u32(ebp + 8, u32::from(b));
        emu.run(0x59d3ef, 0x59d402); // fild; fmul 1/255; fstp [ebp-0x10]
        let binary = emu.read_f32(ebp - 0x10).to_bits();
        let ours = benilla_re_gx::color_norm(b).to_bits();
        assert_eq!(
            binary, ours,
            "color_norm@0x59d350({b}): binary={binary:#010x} ours={ours:#010x}"
        );
        checked += 1;
    }
    eprintln!("color_norm@0x59d350: {checked} bytes bit-exact vs WoW.exe @ 0x59d350");
}

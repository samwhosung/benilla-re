//! `SetWorldObjectBounds` (`0x6717d0`) and `CWorld::PrepareAreaOfInterest` (`0x672730`), as slices:
//! `0x6717d0` in six stretches from `0x6717da` to `0x6719f8`, each named in the test, and
//! `0x672730` from `0x672736` to `0x6728e6`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::camera::{
    camera_fwd_mag, camera_fwd_mag_stored, camera_fwd_renorm_decision, camera_inv_mag,
    camera_move_dist2, camera_move_flag, camera_pos_delta, camera_right_half, camera_up_renorm_mag,
    window_box_150, window_chunk_indices, window_farclip_box, window_farclip_box_live,
    window_farclip_box_step, MAP_CENTER, RENORM_EPS,
};
use common::{assert_f32, next};

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// AH after `fcomp; fnstsw ax`.
fn read_ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// Whether `test ah,5; jp` jumps, from the AH the exe produced: on an even count of C0 and C2.
fn ah5_jp_taken(ah: u8) -> bool {
    (ah & 5).count_ones().is_multiple_of(2)
}

#[test]
fn camera_set_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x100); // esi = the object
    let news = e.alloc(0x100); // ebx = the new state
    let frame = e.alloc(0x100); // a frame for the reciprocal's stores
    let rvec = e.alloc(0x20); // eax in the fifth stretch
    let mut s = 0x6717_0000u32;
    let ebp = frame + 0x80; // the [ebp-N] slots land in `frame`

    // 0x6717da to 0x671805: esi = this (old at +0x24), ebx = new (at +0x30); after `push edi; sub
    // esp,0xc` it stores dz at ESP − 8, dy at − 0xc, dx at − 0x10.
    let delta_case = |e: &mut Emu, new_pos: [f32; 3], old_pos: [f32; 3], ctx: &str| {
        e.write_u32(news + 0x30, new_pos[0].to_bits());
        e.write_u32(news + 0x34, new_pos[1].to_bits());
        e.write_u32(news + 0x38, new_pos[2].to_bits());
        e.write_u32(this + 0x24, old_pos[0].to_bits());
        e.write_u32(this + 0x28, old_pos[1].to_bits());
        e.write_u32(this + 0x2c, old_pos[2].to_bits());
        // the stretch's `mov esi,ecx` sets ESI; EBX comes from the `mov ebx,edx` at 0x6717d8 before
        // it, so it is set here
        e.set_reg(RegisterX86::ECX, this);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBX, news);
        e.set_reg(RegisterX86::EBP, ebp); // `mov [ebp-4],esi` at 0x6717f3 writes here
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6717da, 0x671805);
        let bin = [
            e.read_f32(SLICE_ESP - 0x10), // dx
            e.read_f32(SLICE_ESP - 0xc),  // dy
            e.read_f32(SLICE_ESP - 0x8),  // dz
        ];
        let ours = camera_pos_delta(new_pos, old_pos);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} delta[{k}]"));
        }
    };

    // 0x67180a to 0x67182a, to after `fnstsw ax`: dz at [ebp-0x10], dy [ebp-0x14], dx [ebp-0x18]
    let slot = |disp: u64| ebp - disp;
    let move_gate = |e: &mut Emu, d: [f32; 3], ctx: &str| {
        e.write_u32(slot(0x18), d[0].to_bits()); // dx
        e.write_u32(slot(0x14), d[1].to_bits()); // dy
        e.write_u32(slot(0x10), d[2].to_bits()); // dz
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x67180a, 0x67182a);
        let ah = read_ah(e);
        // `test ah,5; jp 0x671836`: eax = 1 when the jump is not taken
        let bin_flag = !ah5_jp_taken(ah);
        let d2 = camera_move_dist2(d);
        assert_eq!(
            camera_move_flag(d2),
            bin_flag,
            "{ctx} move-flag (d²={d2}): binary_ah={ah:#04x}"
        );
    };

    // 0x671853 to 0x671886, to after `fnstsw ax`: the vector at ebx, the length stored at [esi+0x30]
    let renorm_gate = |e: &mut Emu, fwd: [f32; 3], ctx: &str| {
        e.write_u32(news, fwd[0].to_bits());
        e.write_u32(news + 4, fwd[1].to_bits());
        e.write_u32(news + 8, fwd[2].to_bits());
        e.write_u32(this + 0x30, 0xdead_beef);
        e.set_reg(RegisterX86::EBX, news);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x671853, 0x671886);
        let ah = read_ah(e);
        let mag = camera_fwd_mag(fwd);
        // the length `fst dword [esi+0x30]` stored
        assert_f32(
            e.read_f32(this + 0x30),
            camera_fwd_mag_stored(mag),
            &format!("{ctx} fwd-mag-stored"),
        );
        // `test ah,5; jnp 0x671901`: it renormalizes when the jump is not taken, an even count
        let bin_renorm = ah5_jp_taken(ah);
        assert_eq!(
            camera_fwd_renorm_decision(mag),
            bin_renorm,
            "{ctx} renorm-decision (mag={mag}): binary_ah={ah:#04x}"
        );
    };

    // 0x6718b6 to 0x6718d8: [ebp-4] = this; it reloads [this+0x30], stores 1/len to [ebp-8]; the
    // `rep movsd` among it copies inside `frame`
    let inv_case = |e: &mut Emu, mag_f32: f32, ctx: &str| {
        e.write_u32(slot(0x4), this as u32);
        e.write_u32(this + 0x30, mag_f32.to_bits()); // the stored length
        e.write_u32(slot(0x8), 0xdead_beef); // the result slot
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6718b6, 0x6718d8);
        assert_f32(
            e.read_f32(slot(0x8)),
            camera_inv_mag(mag_f32),
            &format!("{ctx} inv-mag (mag={mag_f32})"),
        );
    };

    // 0x671986 to 0x6719b1: eax = the vector; after `sub esp,0xc` z·½ at ESP − 4, y·½ − 8, x·½ − 0xc
    let right_case = |e: &mut Emu, right: [f32; 3], ctx: &str| {
        e.write_u32(rvec, right[0].to_bits());
        e.write_u32(rvec + 4, right[1].to_bits());
        e.write_u32(rvec + 8, right[2].to_bits());
        e.set_reg(RegisterX86::EAX, rvec);
        e.set_reg(RegisterX86::EBP, ebp); // only a `lea` uses it
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x671986, 0x6719b1);
        let bin = [
            e.read_f32(SLICE_ESP - 0xc), // r.x·½
            e.read_f32(SLICE_ESP - 0x8), // r.y·½
            e.read_f32(SLICE_ESP - 0x4), // r.z·½
        ];
        let ours = camera_right_half(right);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} right½[{k}]"));
        }
    };

    // 0x6719cc to 0x6719f8: esi = this (+0x50), eax = this + 0x5c; the length stored at [esi+0x68]
    let up_case = |e: &mut Emu, up: [f32; 3], base: [f32; 3], ctx: &str| {
        e.write_u32(this + 0x50, up[0].to_bits());
        e.write_u32(this + 0x54, up[1].to_bits());
        e.write_u32(this + 0x58, up[2].to_bits());
        e.write_u32(this + 0x5c, base[0].to_bits());
        e.write_u32(this + 0x60, base[1].to_bits());
        e.write_u32(this + 0x64, base[2].to_bits());
        e.write_u32(this + 0x68, 0xdead_beef);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EAX, this + 0x5c);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6719cc, 0x6719f8);
        assert_f32(
            e.read_f32(this + 0x68),
            camera_up_renorm_mag(up, base),
            &format!("{ctx} up-renorm-mag"),
        );
    };

    // fixed cases first, with each test's edge
    delta_case(&mut e, [0.0; 3], [0.0; 3], "delta zeros");
    delta_case(&mut e, [1.0, 2.0, 3.0], [0.5, -0.5, 1.0], "delta basic");

    // The move test at 0.001: f32 deltas rarely give it exactly, so it is probed either side and at
    // the delta whose square is 0.001 in f64.
    move_gate(&mut e, [0.0; 3], "move zero (d²=0 < eps -> moved-flag=1)");
    move_gate(
        &mut e,
        [1.0, 0.0, 0.0],
        "move large (d²=1 >= eps -> flag=0)",
    );
    move_gate(&mut e, [0.03162277, 0.0, 0.0], "move ~sqrt(eps)"); // d² near 0.001
    move_gate(&mut e, [0.031, 0.0, 0.0], "move just-below eps");
    move_gate(&mut e, [0.032, 0.0, 0.0], "move just-above eps");
    // dx with dx·dx = 0.001 in f64
    {
        let dx = (f64::from(f32::from_bits(0x3a83_126f))).sqrt() as f32;
        move_gate(&mut e, [dx, 0.0, 0.0], "move d²==eps boundary");
    }

    // The length test at 2^-22: a vector [m, 0, 0] has length |m|.
    renorm_gate(
        &mut e,
        [1.0, 0.0, 0.0],
        "renorm unit (mag=1 -> |Δ|=0 < eps -> copy-as-is)",
    );
    renorm_gate(
        &mut e,
        [2.0, 0.0, 0.0],
        "renorm mag=2 (|Δ|=1 >= eps -> renormalize)",
    );
    {
        let eps = RENORM_EPS;
        renorm_gate(&mut e, [1.0 + eps, 0.0, 0.0], "renorm mag=1+eps boundary");
        renorm_gate(&mut e, [1.0 - eps, 0.0, 0.0], "renorm mag=1-eps boundary");
        // |len − 1| on the edge: 1 + 2 ulp
        let m = f32::from_bits((1.0f32).to_bits() + 2); // 1 + 2.38e-7
        renorm_gate(&mut e, [m, 0.0, 0.0], "renorm mag=1+2ulp (==eps region)");
    }

    inv_case(&mut e, 1.0, "inv mag=1");
    inv_case(&mut e, 2.0, "inv mag=2");
    inv_case(&mut e, 0.5, "inv mag=0.5");
    inv_case(&mut e, 3.0, "inv mag=3 (0.333…)");

    right_case(&mut e, [0.0; 3], "right zeros");
    right_case(&mut e, [1.0, -2.0, 3.5], "right basic");

    up_case(&mut e, [1.0, 0.0, 0.0], [0.0; 3], "up unit-x");
    up_case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], "up 3-4-5");
    up_case(&mut e, [5.0, 6.0, 7.0], [1.0, 2.0, 3.0], "up offset-base");

    let mut checked = 0u32;
    for i in 0..2500u32 {
        let bits = i % 5 == 0;
        let v3 = |s: &mut u32, bits: bool, lo: f32, hi: f32| -> [f32; 3] {
            std::array::from_fn(|_| {
                if bits {
                    frand_bits(s)
                } else {
                    frand(s, lo, hi)
                }
            })
        };
        let new_pos = v3(&mut s, bits, -5000.0, 5000.0);
        let old_pos = v3(&mut s, bits, -5000.0, 5000.0);
        delta_case(&mut e, new_pos, old_pos, &format!("fuzz{i}"));
        // small deltas, so the move test goes both ways often
        let md = if i % 3 == 0 {
            v3(&mut s, bits, -2.0, 2.0)
        } else {
            v3(&mut s, false, -0.1, 0.1)
        };
        move_gate(&mut e, md, &format!("fuzz{i}"));
        // vectors of near unit length, so the length test goes both ways
        let fwd = if i % 4 == 0 {
            v3(&mut s, bits, -2.0, 2.0)
        } else {
            // an axis scaled by about 1 ± a little
            let sc = frand(&mut s, 0.999_99, 1.000_01);
            [
                sc,
                frand(&mut s, -0.001, 0.001),
                frand(&mut s, -0.001, 0.001),
            ]
        };
        renorm_gate(&mut e, fwd, &format!("fuzz{i}"));
        let m = if i % 6 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.01, 1000.0)
        };
        inv_case(&mut e, m, &format!("fuzz{i}"));
        right_case(
            &mut e,
            v3(&mut s, bits, -1000.0, 1000.0),
            &format!("fuzz{i}"),
        );
        let up = v3(&mut s, bits, -1000.0, 1000.0);
        let base = v3(&mut s, bits, -1000.0, 1000.0);
        up_case(&mut e, up, base, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "camera_set: {checked} fuzz iters bit-exact vs WoW.exe @ 0x6717d0 slices \
         [6717da,671805)/[67180a,67182a)/[671853,671886)/[6718b6,6718d8)/[671986,6719b1)/[6719cc,6719f8)"
    );
}

/// `0x672736` to `0x6728e6` has no branches: it writes each box to its globals and the chunk
/// indices to frame slots. ECX is the centre and `[0xc7b498]` the far clip.
#[test]
fn camera_window_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let center = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x6727_0000u32;

    // the boxes' globals
    const G_B8: u64 = 0xc631b8; // cx-far
    const G_B4: u64 = 0xc631b4; // cy-far
    const G_C0: u64 = 0xc631c0; // far+cx
    const G_BC: u64 = 0xc631bc; // far+cy
    const G_A4: u64 = 0xc631a4; // (cx-far)-33.333
    const G_A0: u64 = 0xc631a0; // (cy-far)-33.333
    const G_AC: u64 = 0xc631ac; // (far+cx)+33.333
    const G_A8: u64 = 0xc631a8; // (far+cy)+33.333
                                // the ±150 box, low corner then high
    const G_2520: u64 = 0xc62520; // -150 x
    const G_2524: u64 = 0xc62524; // -150 y
    const G_2528: u64 = 0xc62528; // -150 z
    const G_252C: u64 = 0xc6252c; // +150 x
    const G_2530: u64 = 0xc62530; // +150 y
    const G_2534: u64 = 0xc62534; // +150 z
                                  // the far clip box on three axes
    const G_3240: u64 = 0xc63240; // c-far x
    const G_3244: u64 = 0xc63244; // c-far y
    const G_3248: u64 = 0xc63248; // c-far z
    const G_324C: u64 = 0xc6324c; // far+c x
    const G_3250: u64 = 0xc63250; // far+c y
    const G_3254: u64 = 0xc63254; // far+c z
    const FARCLIP: u64 = 0xc7b498;

    let slot = |disp: u64| ebp - disp;
    let s_idx = slot(0xc); // [ebp-0xc], the x chunk index
    let s_idy = slot(0x4); // [ebp-4], the y chunk index

    let case = |e: &mut Emu, c: [f32; 3], farclip: f32, ctx: &str| {
        e.write_u32(center, c[0].to_bits());
        e.write_u32(center + 4, c[1].to_bits());
        e.write_u32(center + 8, c[2].to_bits());
        e.write_u32(FARCLIP, farclip.to_bits());
        e.set_reg(RegisterX86::ECX, center);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x672736, 0x6728e6);

        // the far clip box
        let fb = window_farclip_box(c, farclip);
        assert_f32(e.read_f32(G_B8), fb[0], &format!("{ctx} far cx-far"));
        assert_f32(e.read_f32(G_B4), fb[1], &format!("{ctx} far cy-far"));
        assert_f32(e.read_f32(G_C0), fb[2], &format!("{ctx} far far+cx"));
        assert_f32(e.read_f32(G_BC), fb[3], &format!("{ctx} far far+cy"));
        // widened by 33.333
        let fs = window_farclip_box_step(fb);
        assert_f32(e.read_f32(G_A4), fs[0], &format!("{ctx} step lo-x"));
        assert_f32(e.read_f32(G_A0), fs[1], &format!("{ctx} step lo-y"));
        assert_f32(e.read_f32(G_AC), fs[2], &format!("{ctx} step hi-x"));
        assert_f32(e.read_f32(G_A8), fs[3], &format!("{ctx} step hi-y"));
        // the ±150 box
        let (lo150, hi150) = window_box_150(c);
        assert_f32(e.read_f32(G_2520), lo150[0], &format!("{ctx} 150 lo-x"));
        assert_f32(e.read_f32(G_2524), lo150[1], &format!("{ctx} 150 lo-y"));
        assert_f32(e.read_f32(G_2528), lo150[2], &format!("{ctx} 150 lo-z"));
        assert_f32(e.read_f32(G_252C), hi150[0], &format!("{ctx} 150 hi-x"));
        assert_f32(e.read_f32(G_2530), hi150[1], &format!("{ctx} 150 hi-y"));
        assert_f32(e.read_f32(G_2534), hi150[2], &format!("{ctx} 150 hi-z"));
        // the far clip box on three axes
        let (lof, hif) = window_farclip_box_live(c, farclip);
        assert_f32(e.read_f32(G_3240), lof[0], &format!("{ctx} liveF lo-x"));
        assert_f32(e.read_f32(G_3244), lof[1], &format!("{ctx} liveF lo-y"));
        assert_f32(e.read_f32(G_3248), lof[2], &format!("{ctx} liveF lo-z"));
        assert_f32(e.read_f32(G_324C), hif[0], &format!("{ctx} liveF hi-x"));
        assert_f32(e.read_f32(G_3250), hif[1], &format!("{ctx} liveF hi-y"));
        assert_f32(e.read_f32(G_3254), hif[2], &format!("{ctx} liveF hi-z"));
        // the chunk indices
        let (ix, iy) = window_chunk_indices(c);
        assert_eq!(
            e.read_u32(s_idx) as i32,
            ix,
            "{ctx} chunk idx_x (c.x={})",
            c[0]
        );
        assert_eq!(
            e.read_u32(s_idy) as i32,
            iy,
            "{ctx} chunk idx_y (c.y={})",
            c[1]
        );
    };

    // fixed cases, and a tie of the chunk index
    case(&mut e, [0.0; 3], 1000.0, "zeros");
    case(
        &mut e,
        [17066.666, 17066.666, 0.0],
        1000.0,
        "at map-center (chunk idx ~0)",
    );
    case(&mut e, [-17066.666, 34133.33, 100.0], 533.333_3, "span");
    // a tie: `neg_x·0.03 − 0.5` half an integer, with neg_x = 33.333, so cx = C − 33.333
    case(
        &mut e,
        [
            f64::from(MAP_CENTER) as f32 - 33.333_332,
            f64::from(MAP_CENTER) as f32,
            0.0,
        ],
        1000.0,
        "chunk-idx tie probe (prod~1.0)",
    );

    let mut checked = 0u32;
    for i in 0..2500u32 {
        let bits = i % 5 == 0;
        let c: [f32; 3] = std::array::from_fn(|_| {
            if bits {
                frand_bits(&mut s)
            } else {
                frand(&mut s, -5000.0, 40000.0)
            }
        });
        let farclip = if i % 7 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 50.0, 2000.0)
        };
        case(&mut e, c, farclip, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "camera_window: {checked} fuzz iters bit-exact vs WoW.exe @ 0x672730 slice [672736,6728e6)"
    );
}

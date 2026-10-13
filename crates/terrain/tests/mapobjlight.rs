//! The map object lights the world reads: `LightFalloff` (`0x69e1c0`) and `PackLightColor`
//! (`0x6a77e0`) run whole, and `GetCameraInteriorFog` (`0x69de20`) and `GetEntityInteriorLight`
//! (`0x69dc30`) as slices. A stretch run in an island is the exe's own bytes, checked, copied
//! between a load and a store.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::mapobjlight::{
    interior_light_rotate, light_distance, wmo_mocv_normalize_alpha, wmo_molt_attenuation,
    InteriorLightMatrix,
};
use common::{assert_f32, frand3, fuzz_f32, next, read_f64, Lcg, SLICE_ESP};

/// A finite coordinate over six ranges, from 1e-4 to 1e20.
fn fuzz_pos(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,         // [-1, 1)
        1 => (m * 2000.0 - 1000.0) as f32,   // world-ish coords
        2 => (m * 34000.0 - 17000.0) as f32, // WoW map extent
        3 => (m * 1.0e6 - 5.0e5) as f32,     // large
        4 => (m * 1.0e-4) as f32,            // tiny
        _ => (m * 1.0e20 - 5.0e19) as f32,   // huge (overflow the square)
    }
}

/// `0x69e1c0` whole: `ecx` a light with its start at `+0x10` and end at `+0x14`, the distance on
/// the stack; the result is read from `st0`. Distances below 0, at 0, at the start and the end,
/// beyond the end, NaN and infinite; equal and inverted ends.
#[test]
fn wmo_molt_attenuation_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let rec = emu.alloc(0x18);

    let structured: &[(f32, f32, f32)] = &[
        // (start, end, d)
        (2.0, 10.0, -1.0),          // clamp floor
        (2.0, 10.0, 0.0),           // == floor (≥ branch)
        (2.0, 10.0, 1.0),           // d < start → 1.0
        (2.0, 10.0, 2.0),           // d == start → divide path, num 0
        (2.0, 10.0, 6.0),           // interior
        (2.0, 10.0, 10.0),          // d == end
        (2.0, 10.0, 50.0),          // clamp to end
        (5.0, 5.0, 7.0),            // start == end → 0/0 on the divide path
        (5.0, 5.0, 2.0),            // start == end, d < start → 1.0
        (10.0, 2.0, 6.0),           // end < start (malformed record)
        (-4.0, 8.0, -2.0),          // negative start, clamp floor then divide
        (2.0, 10.0, f32::NAN),      // unordered at every compare
        (2.0, 10.0, f32::INFINITY), // clamp to end
    ];
    let mut s = 0x6d6f_6c74u32;
    let fuzz = (0..400).map(|_| (fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)));
    for (start, end, d) in structured.iter().copied().chain(fuzz) {
        emu.write(rec + 0x10, &start.to_le_bytes());
        emu.write(rec + 0x14, &end.to_le_bytes());
        // __thiscall: ecx = the light record, stack arg = d. ret 4. Result left in st0.
        let binary = emu.call_f32(0x69e1c0, rec as u32, 0, &[d.to_bits()]);
        let ours = wmo_molt_attenuation(start, end, d) as f32;
        assert_f32(
            binary,
            ours,
            &format!("wmo_molt_attenuation({start},{end},{d})"),
        );
    }
}

/// `0x69de20` from `0x69dfb2` to `0x69dfd9` in an island with stores appended: the light at `ecx`,
/// the sample at `[ebp-0x24]`; both the stored distance and the unrounded one are checked.
#[test]
fn light_distance_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // ---- byte-verify + assemble the live-f64 island (FP block 0x69dfac..0x69dfd3 + fstp qword)
    const ISL_VA: u64 = 0x0069_dfac;
    const ISL_LEN: usize = 0x0069_dfd3 - 0x0069_dfac; // 39 bytes, through fsqrt@69dfd1
    let frag = e.read(ISL_VA, ISL_LEN);
    // The exe's bytes of the block, as the model reads them.
    assert_eq!(
        frag,
        vec![
            0xd9, 0x44, 0x01, 0x04, // fld   dword [ecx+eax+0x4]   (Lx)
            0x03, 0xc8, //             add   ecx, eax              (eax=0 → no-op here)
            0xd8, 0x65, 0xdc, //       fsub  dword [ebp-0x24]      (Sx) → dx
            0xd9, 0x41, 0x08, //       fld   dword [ecx+0x8]       (Ly)
            0xd8, 0x65, 0xe0, //       fsub  dword [ebp-0x20]      (Sy) → dy
            0xd9, 0x41, 0x0c, //       fld   dword [ecx+0xc]       (Lz)
            0xd8, 0x65, 0xe4, //       fsub  dword [ebp-0x1c]      (Sz) → dz
            0xd9, 0xc0, //             fld   st(0)
            0xd8, 0xc9, //             fmul  st, st(1)             → dz²
            0xd9, 0xc2, //             fld   st(2)
            0xd8, 0xcb, //             fmul  st, st(3)             → dy²
            0xde, 0xc1, //             faddp st(1), st             → dz²+dy² (LIVE f64)
            0xd9, 0xc3, //             fld   st(3)
            0xd8, 0xcc, //             fmul  st, st(4)             → dx²
            0xde, 0xc1, //             faddp st(1), st             → sum (LIVE f64)
            0xd9, 0xfa, //             fsqrt                       → dist (LIVE f64)
        ],
        "molt FP-block bytes changed",
    );

    let light_buf = e.alloc(0x40); // ECX: Lx@+4, Ly@+8, Lz@+0xc
    let frame = e.alloc(0x100); // EBP frame: Sx/Sy/Sz @ [ebp-0x24]/-0x20/-0x1c, f32 result @ [ebp+0x8]
    let ebp = frame + 0x80;
    let out_f64 = e.alloc(0x10); // island fstp qword target
    let island = e.alloc(0x40);

    let mut code = frag.clone();
    code.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [abs32]  (DD /3, disp32 absolute)
    code.extend_from_slice(&(out_f64 as u32).to_le_bytes());
    let island_end = island + code.len() as u64;
    e.write(island, &code);

    let run_case = |e: &mut Emu, light: [f32; 3], sample: [f32; 3], ctx: &str| {
        // Light position at [ecx+4]/[ecx+8]/[ecx+0xc].
        e.write_u32(light_buf + 0x4, light[0].to_bits());
        e.write_u32(light_buf + 0x8, light[1].to_bits());
        e.write_u32(light_buf + 0xc, light[2].to_bits());
        // Sample position at [ebp-0x24]/-0x20/-0x1c.
        e.write_u32(ebp - 0x24, sample[0].to_bits());
        e.write_u32(ebp - 0x20, sample[1].to_bits());
        e.write_u32(ebp - 0x1c, sample[2].to_bits());

        let (ours_f32, ours_f64) = light_distance(light, sample);

        // ---- stored-f32 observable: run the binary slice, read [ebp+0x8] (the fst dword).
        e.write_u32(ebp + 0x8, 0xdead_beef); // junk-seed so a non-write diverges
        e.set_reg(RegisterX86::EAX, 0);
        e.set_reg(RegisterX86::ECX, light_buf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x0069_dfac, 0x0069_dfdc); // halt AT fcomp, PAST fst dword [ebp+0x8]@69dfd9
        let bin_f32 = e.read_f32(ebp + 0x8);
        assert_f32(bin_f32, ours_f32, &format!("{ctx} dist_f32"));

        // ---- live-f64 observable: the island leaves st0 = dist (un-rounded), fstp qword'd.
        e.write_u32(out_f64, 0xdead_beef);
        e.write_u32(out_f64 + 4, 0xdead_beef);
        e.set_reg(RegisterX86::EAX, 0);
        e.set_reg(RegisterX86::ECX, light_buf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(island, island_end);
        let bin_f64 = read_f64(e, out_f64);
        assert_eq!(
            bin_f64.to_bits(),
            ours_f64.to_bits(),
            "{ctx} dist_f64: binary={:#018x} ours={:#018x}",
            bin_f64.to_bits(),
            ours_f64.to_bits()
        );
    };

    // exact / boundary probes:
    run_case(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "equal points → 0");
    run_case(&mut e, [5.0, 7.0, 9.0], [5.0, 7.0, 9.0], "coincident → 0");
    run_case(&mut e, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0], "3-4-0 → 5 exact");
    run_case(&mut e, [1.0, 2.0, 2.0], [0.0, 0.0, 0.0], "1-2-2 → 3 exact");
    run_case(&mut e, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], "unit-x → 1");
    run_case(&mut e, [-10.0, 5.0, -3.0], [10.0, -5.0, 3.0], "mixed sign");
    run_case(
        &mut e,
        [1.0e20, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "huge → square overflows f32 (f64 holds it)",
    );
    run_case(
        &mut e,
        [1.0e-20, 1.0e-20, 1.0e-20],
        [0.0, 0.0, 0.0],
        "tiny → underflow-region distance",
    );

    let mut s = 0x69de_2020u32;
    for i in 0..6000u32 {
        let light = frand3(&mut s, -2000.0, 2000.0);
        let sample = frand3(&mut s, -2000.0, 2000.0);
        run_case(&mut e, light, sample, &format!("molt dist fuzz[{i}]"));
    }
    // a second sweep over extreme magnitudes (overflow / underflow of the squares).
    let mut s = 0x69df_acacu32;
    for i in 0..4000u32 {
        let light = [fuzz_pos(&mut s), fuzz_pos(&mut s), fuzz_pos(&mut s)];
        let sample = [fuzz_pos(&mut s), fuzz_pos(&mut s), fuzz_pos(&mut s)];
        run_case(&mut e, light, sample, &format!("molt dist xfuzz[{i}]"));
    }
}

/// `0x69dc30` from `0x69dd03` to `0x69dd58`: the normal at `[ebp-0x14]`, the matrix at `esi`; the
/// result is read from `[ebp-0x28]`.
#[test]
fn interior_light_rotate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mat = e.alloc(0x40); // esi = matrix base; the 9 entries at {0,4,8, 0x10,0x14,0x18, 0x20,0x24,0x28}
    let obj = e.alloc(0x100); // edi: the binary writes [edi+0x98] then [edi+0xe4/0xe8/0xec]
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, n: [f32; 3], m: &InteriorLightMatrix, ctx: &str| {
        // 9 matrix entries in the binary's read order.
        for k in 0..3 {
            e.write_u32(mat + 4 * k as u64, m.c0[k].to_bits()); // [esi+0/4/8]
            e.write_u32(mat + 0x10 + 4 * k as u64, m.ca[k].to_bits()); // [esi+0x10/0x14/0x18]
            e.write_u32(mat + 0x20 + 4 * k as u64, m.cb[k].to_bits()); // [esi+0x20/0x24/0x28]
        }
        // normal: n0=[ebp-0x14], n1=[ebp-0x18], n2=[ebp-0x1c].
        e.write_u32(ebp - 0x14, n[0].to_bits());
        e.write_u32(ebp - 0x18, n[1].to_bits());
        e.write_u32(ebp - 0x1c, n[2].to_bits());
        e.set_reg(RegisterX86::ESI, mat);
        e.set_reg(RegisterX86::EDI, obj);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // Halt at 0x69dd61 (the `pop edi`) so all three outputs are both `fstp`ed to
        // [ebp-0x28]/-0x24/ -0x20 AND copied by the trailing `mov`s to [edi+0xe4/0xe8/0xec] (the
        // last copy is at 0x69dd5e, after the third fstp@0x69dd58 — halting at 0x69dd5b would miss
        // it).
        e.run(0x69dcf7, 0x69dd61);
        let bin = [
            e.read_f32(ebp - 0x28),
            e.read_f32(ebp - 0x24),
            e.read_f32(ebp - 0x20),
        ];
        let ours = interior_light_rotate(n, m);
        for (k, &o) in ours.iter().enumerate() {
            assert_f32(bin[k], o, &format!("{ctx} out[{k}]"));
            // also confirm the binary copied each output to [edi+0xe4/0xe8/0xec].
            assert_f32(
                e.read_f32(obj + 0xe4 + 4 * k as u64),
                o,
                &format!("{ctx} obj+0xe4[{k}]"),
            );
        }
    };

    // identity rotation: out == n.
    run_case(
        &mut e,
        [1.0, 2.0, 3.0],
        &InteriorLightMatrix {
            c0: [1.0, 0.0, 0.0], // n2 multiplicands (col for out0/1/2)
            ca: [0.0, 1.0, 0.0], // n1 multiplicands
            cb: [0.0, 0.0, 1.0], // n0 multiplicands
        },
        "rotate identity",
    );
    // zero normal -> zero out.
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        &InteriorLightMatrix {
            c0: [1.0, 2.0, 3.0],
            ca: [4.0, 5.0, 6.0],
            cb: [7.0, 8.0, 9.0],
        },
        "rotate zero-normal",
    );

    let mut s = 0x69dc_30a0u32;
    for i in 0..6000u32 {
        let n = frand3(&mut s, -10.0, 10.0);
        let m = InteriorLightMatrix {
            c0: frand3(&mut s, -2.0, 2.0),
            ca: frand3(&mut s, -2.0, 2.0),
            cb: frand3(&mut s, -2.0, 2.0),
        };
        run_case(&mut e, n, &m, &format!("rotate fuzz[{i}]"));
    }
}

/// `0x6a77e0` whole with the third argument 0, so the colour branch is not taken: every colour
/// byte against limits around it, and both outputs read back.
#[test]
fn wmo_mocv_normalize_alpha_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);

    let src = emu.alloc(0x10);
    let out_bright = emu.alloc(0x10);
    let out_alpha = emu.alloc(0x10);

    let mut rng = Lcg(0x61_6c_70_68);
    let (mut rescaled, mut untouched) = (0u32, 0u32);
    let mut saw_zero_max = false;
    for _ in 0..2000u32 {
        // src color dword; occasionally force all-zero RGB to exercise the maxch floor.
        let s = if (rng.next() >> 8).is_multiple_of(17) {
            rng.next() & 0xff00_0000 // RGB=0, random A
        } else {
            rng.next()
        };
        let src_bytes = s.to_le_bytes();
        let bgr_max = src_bytes[0].max(src_bytes[1]).max(src_bytes[2]);
        if bgr_max == 0 {
            saw_zero_max = true;
        }
        // alpha_lim swept across the maxch range so both `P5 < maxch` and `P5 >= maxch` fire.
        let alpha_lim = (rng.next() >> 8) as u8;

        emu.write_u32(src, s);
        emu.write_u32(out_bright, 0xdead_beef);
        emu.write_u32(out_alpha, 0xdead_beef);
        // __cdecl(src, out_bright, P3=0, out_alpha, alpha_lim). ret 0x14.
        emu.call(
            0x6a77e0,
            0,
            0,
            &[
                src as u32,
                out_bright as u32,
                0, // P3 = 0 → brightness branch gated off
                out_alpha as u32,
                u32::from(alpha_lim),
            ],
        );
        let b_bright = emu.read_u32(out_bright);
        let b_alpha = emu.read_u32(out_alpha);

        let ours = wmo_mocv_normalize_alpha(src_bytes, alpha_lim);
        // out_bright must equal raw src (HSV branch never ran).
        assert_eq!(
            b_bright, s,
            "normalize_alpha out_bright src={s:#010x} alpha_lim={alpha_lim}: HSV branch must be gated off"
        );
        // out_alpha equals the model's.
        assert_eq!(
            b_alpha,
            u32::from_le_bytes(ours.bgra),
            "normalize_alpha out_alpha src={s:#010x} alpha_lim={alpha_lim}: binary={b_alpha:#010x} ours={:#010x} maxch={}",
            u32::from_le_bytes(ours.bgra),
            ours.maxch
        );
        let maxch = if bgr_max == 0 { 1 } else { bgr_max };
        if alpha_lim < maxch {
            rescaled += 1;
        } else {
            untouched += 1;
        }
    }
    assert!(
        rescaled > 0 && untouched > 0 && saw_zero_max,
        "branches: rescaled={rescaled} untouched={untouched} saw_zero_max={saw_zero_max}"
    );
}

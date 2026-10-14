//! `PackColourBytes` (`0x6d5270`), `LerpColourBytes` (`0x6d51e0`), `UpdateSunDirection`
//! (`0x6d3a10`), the darkened ambient of `BuildLightColours` (`0x6d37e0`), `UpdateCelestialBodies`
//! (`0x6d3b80`) and `BuildCloudDome` (`0x6d0530`), each whole, against `dn_pack_bytes_color`,
//! `dn_channel_lerp8`, `dn_sun_dir`, `dn_fog_ambient_darkened`, `dn_celestial_positions` and
//! `dn_sun_trig_table`. The sine and cosine are the host's in the emulator and in the
//! transcriptions.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::dn_sky::{
    dn_celestial_positions, dn_channel_lerp8, dn_fog_ambient_darkened, dn_pack_bytes_color,
    dn_sun_dir, dn_sun_trig_table, CelestialIn,
};
use common::{assert_f32, next};

/// `0x6d5270` with full random dwords, to show only the low byte of each argument is read.
#[test]
fn dn_pack_bytes_color_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out_p = emu.alloc(4);
    let mut s = 0x7061_636bu32;
    for case in 0..400 {
        let a = next(&mut s);
        let b = next(&mut s);
        let c = next(&mut s);
        let d = next(&mut s);
        emu.call(0x6d5270, out_p as u32, 0, &[a, b, c, d]);
        let binary = emu.read_u32(out_p);
        let ours = dn_pack_bytes_color(a as u8, b as u8, c as u8, d as u8);
        assert_eq!(binary, ours, "pack (case {case})");
    }
}

/// `0x6d51e0` with weights 0, 0xff (the copy), and others: the high byte of the weight is read as
/// well, so full dwords are tried.
#[test]
fn dn_channel_lerp8_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let dst_p = emu.alloc(4);
    let src_p = emu.alloc(4);
    let mut s = 0x6c72_7038_u32;
    for case in 0..600 {
        let dst = next(&mut s);
        let src = next(&mut s);
        let a = match case % 4 {
            0 => 0u32,
            1 => 0xff,
            2 => (next(&mut s) >> 24) & 0xff,
            _ => next(&mut s) & 0xff,
        };
        emu.write_u32(dst_p, dst);
        emu.write_u32(src_p, src);
        emu.call(0x6d51e0, dst_p as u32, 0, &[a, src_p as u32]);
        let binary = emu.read_u32(dst_p);
        let ours = dn_channel_lerp8(dst, src, a);
        assert_eq!(
            binary, ours,
            "lerp8 (case {case} dst={dst:#010x} src={src:#010x} a={a:#x})"
        );
    }
}

/// The globals `0x6d3a10` reads and writes. The track it builds on first use is rebuilt each call
/// by clearing the flag.
const G_TIME_OF_DAY: u64 = 0xce9b64;
const G_SUN_FLAG: u64 = 0xce8dd0;
const G_SUN_X: u64 = 0xce9cc8;
const G_SUN_Y: u64 = 0xce9ccc;
const G_SUN_Z: u64 = 0xce9cd0;

#[test]
fn dn_sun_dir_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // The first-use build registers an exit hook through `0x409aef`; it is stubbed.
    emu.stub(0x409aef, 0, 0);
    let mut s = 0x7375_6e64_u32;
    for case in 0..400 {
        let phase = match case % 8 {
            0 => 0.0f32,
            1 => 0.25,
            2 => 0.5,
            3 => 0.75,
            4 => 1.0,
            _ => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32,
        };
        emu.write_u32(G_TIME_OF_DAY, phase.to_bits());
        emu.write_u32(G_SUN_FLAG, 0);
        emu.call(0x6d3a10, 0, 0, &[]);
        let bx = emu.read_f32(G_SUN_X);
        let by = emu.read_f32(G_SUN_Y);
        let bz = emu.read_f32(G_SUN_Z);
        let [ox, oy, oz] = dn_sun_dir(phase);
        assert_f32(bx, ox, &format!("sun.x (case {case} phase={phase})"));
        assert_f32(by, oy, &format!("sun.y (case {case} phase={phase})"));
        assert_f32(bz, oz, &format!("sun.z (case {case} phase={phase})"));
    }
}

/// `0x6d37e0` whole, reading back only the darkened field `[0xce9cf4]` (`dn_color` covers the
/// rest): the colour at `0xce9c10` and the alpha byte `0xce9c5a` are random, so every channel value
/// runs.
const G_FOG_BASE: u64 = 0xce9c10;
const G_FOG_ALPHA: u64 = 0xce9c5a;
const G_FOG_AMBIENT_OUT: u64 = 0xce9cf4;

#[test]
fn dn_fog_ambient_darkened_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x666f_6764_u32;
    for case in 0..500 {
        let base = next(&mut s);
        let alpha = (next(&mut s) >> 24) as u8;
        emu.write_u32(G_FOG_BASE, base);
        let word = G_FOG_ALPHA & !0x3;
        let off = (G_FOG_ALPHA & 0x3) as u32 * 8;
        let cur = emu.read_u32(word);
        emu.write_u32(word, (cur & !(0xffu32 << off)) | ((alpha as u32) << off));
        emu.call(0x6d37e0, 0, 0, &[]);
        let binary = emu.read_u32(G_FOG_AMBIENT_OUT);
        let ours = dn_fog_ambient_darkened(base, alpha);
        assert_eq!(
            binary, ours,
            "fog ambient darken (case {case} base={base:#010x} alpha={alpha:#x})"
        );
    }
}

/// The globals `0x6d3b80` reads. The eight tracks it builds on first use are rebuilt each call by
/// clearing the flag.
const G_DN_TIME: u64 = 0xce9b64;
const G_DN_MOON_EPOCH: u64 = 0xce9b68;
const G_DN_OFF_X: u64 = 0xce9b78;
const G_DN_OFF_Y: u64 = 0xce9b7c;
const G_DN_OFF_Z: u64 = 0xce9b80;
const G_DN_SUN_SCALE_MUL: u64 = 0xce9870;
const G_DN_MOON_SCALE_MUL: u64 = 0xce9890;
const G_DN_BODY4_SCALE_MUL: u64 = 0xce98b0;
const G_DN_DAY_LENGTH: u64 = 0xce98b4;
const G_CELEST_FLAG: u64 = 0xce8d8c;

/// What it writes; each position is three contiguous floats.
const G_SUN_POS: u64 = 0xce9858;
const G_SUN_POS_COPY: u64 = 0xce97b4;
const G_SUN_SCALE: u64 = 0xce986c;
const G_MOON_POS: u64 = 0xce9878;
const G_MOON_POS_COPY: u64 = 0xce9704;
const G_MOON_SCALE: u64 = 0xce988c;
const G_MOON_SCALE_COPY_A: u64 = 0xce9788;
const G_MOON_SCALE_COPY_B: u64 = 0xce978c;
const G_BODY4_POS: u64 = 0xce9898;
const G_BODY4_SCALE: u64 = 0xce98ac;
const G_DAY_BLEND: u64 = 0xce9d08;

/// Runs `0x6d3b80` on one set of globals and compares everything it writes.
#[allow(clippy::too_many_arguments)]
fn check_celestial(
    emu: &mut Emu,
    tod: f32,
    moon_epoch: f32,
    off: [f32; 3],
    sun_scale_mul: f32,
    moon_scale_mul: f32,
    body4_scale_mul: f32,
    day_length: f32,
    case: &str,
) {
    emu.write_u32(G_DN_TIME, tod.to_bits());
    emu.write_u32(G_DN_MOON_EPOCH, moon_epoch.to_bits());
    emu.write_u32(G_DN_OFF_X, off[0].to_bits());
    emu.write_u32(G_DN_OFF_Y, off[1].to_bits());
    emu.write_u32(G_DN_OFF_Z, off[2].to_bits());
    emu.write_u32(G_DN_SUN_SCALE_MUL, sun_scale_mul.to_bits());
    emu.write_u32(G_DN_MOON_SCALE_MUL, moon_scale_mul.to_bits());
    emu.write_u32(G_DN_BODY4_SCALE_MUL, body4_scale_mul.to_bits());
    emu.write_u32(G_DN_DAY_LENGTH, day_length.to_bits());
    emu.write_u32(G_CELEST_FLAG, 0);

    emu.call(0x6d3b80, 0, 0, &[]);

    let rd3 = |emu: &Emu, base: u64| {
        [
            emu.read_f32(base),
            emu.read_f32(base + 4),
            emu.read_f32(base + 8),
        ]
    };
    let bx = dn_celestial_positions(CelestialIn {
        offset: off,
        time_of_day: tod,
        moon_epoch,
        sun_scale_mul,
        moon_scale_mul,
        body4_scale_mul,
        day_length,
    });

    let bsun = rd3(emu, G_SUN_POS);
    let bsunc = rd3(emu, G_SUN_POS_COPY);
    let bmoon = rd3(emu, G_MOON_POS);
    let bmoonc = rd3(emu, G_MOON_POS_COPY);
    let bbody4 = rd3(emu, G_BODY4_POS);
    let pairs: [(f32, f32, &str); 21] = [
        (bsun[0], bx.sun_pos[0], "sun.x"),
        (bsun[1], bx.sun_pos[1], "sun.y"),
        (bsun[2], bx.sun_pos[2], "sun.z"),
        (bsunc[0], bx.sun_pos_copy[0], "sun_copy.x"),
        (bsunc[1], bx.sun_pos_copy[1], "sun_copy.y"),
        (bsunc[2], bx.sun_pos_copy[2], "sun_copy.z"),
        (emu.read_f32(G_SUN_SCALE), bx.sun_scale, "sun_scale"),
        (bmoon[0], bx.moon_pos[0], "moon.x"),
        (bmoon[1], bx.moon_pos[1], "moon.y"),
        (bmoon[2], bx.moon_pos[2], "moon.z"),
        (bmoonc[0], bx.moon_pos_copy[0], "moon_copy.x"),
        (bmoonc[1], bx.moon_pos_copy[1], "moon_copy.y"),
        (bmoonc[2], bx.moon_pos_copy[2], "moon_copy.z"),
        (emu.read_f32(G_MOON_SCALE), bx.moon_scale, "moon_scale"),
        (
            emu.read_f32(G_MOON_SCALE_COPY_A),
            bx.moon_scale_copy_a,
            "moon_scale_a",
        ),
        (
            emu.read_f32(G_MOON_SCALE_COPY_B),
            bx.moon_scale_copy_b,
            "moon_scale_b",
        ),
        (bbody4[0], bx.body4_pos[0], "body4.x"),
        (bbody4[1], bx.body4_pos[1], "body4.y"),
        (bbody4[2], bx.body4_pos[2], "body4.z"),
        (emu.read_f32(G_BODY4_SCALE), bx.body4_scale, "body4_scale"),
        (emu.read_f32(G_DAY_BLEND), bx.day_blend, "day_blend"),
    ];
    for &(b, o, name) in &pairs {
        assert_f32(b, o, &format!("celestial.{name} ({case} tod={tod})"));
    }
}

#[test]
fn dn_celestial_positions_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // The first-use builds register exit hooks through `0x409aef`; it is stubbed.
    emu.stub(0x409aef, 0, 0);
    let mut s = 0x6365_6c73_u32;
    for case in 0..400 {
        let tod = match case % 12 {
            0 => 0.0f32,
            1 => f32::from_bits(0x3e6a_aaab),
            2 => 0.5,
            3 => f32::from_bits(0x3f65_5555),
            4 => f32::from_bits(0x3f6a_aaab),
            5 => 1.0,
            6 => f32::from_bits(0x3e2a_aaab),
            7 => 0.25,
            8 => 0.75,
            _ => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32,
        };
        let moon_epoch = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 16) * 30.0) as f32;
        let off = [
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
        ];
        let sun_scale_mul = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 4.0) as f32;
        let moon_scale_mul = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 4.0) as f32;
        let body4_scale_mul = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 4.0) as f32;
        let day_length = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 4.0 + 0.5) as f32;

        check_celestial(
            &mut emu,
            tod,
            moon_epoch,
            off,
            sun_scale_mul,
            moon_scale_mul,
            body4_scale_mul,
            day_length,
            &format!("case {case}"),
        );
    }

    // The moon phase at its edges: negative epochs; epochs and day lengths that make a `fistp`
    // overflow to 0x80000000, which the unsigned compare of `eax` with `esi` and the unsigned
    // difference treat as large; epochs and times that are odd multiples of 1/65536 (the round of
    // `v·65536 − 0.5` is then a tie that goes to even); and a day length of 1. A NaN time
    // or epoch sends the C runtime's `floor` into its error path, which this does not run.
    let step = 1.0 / 65536.0;
    for &(tod, epoch, day_length) in &[
        (0.25f32, -5.5f32, 2.0f32),
        (0.5, -0.25, 1.0),
        (0.75, -100.0, 3.0),
        (step, step, 1.0),
        (3.0 * step, 5.0 * step, 1.0),
        (0.5 + step, 7.0 * step, 2.5),
        (0.5, 1.0, 1.0),
        (0.5, 2.0, 1.0),
        (0.0, 0.0, 1.0),
        (-0.0, -0.0, 1.0),
        (0.25, 12.0, 1.5),
        (0.25, 3.0, 0.5),
        (0.25, 40000.0, 1.0e6),
        (0.25, 40000.0, 3.0),
        (0.5, 1.0e5, 1.0e9),
    ] {
        check_celestial(
            &mut emu,
            tod,
            epoch,
            [0.1, -0.2, 0.3],
            1.0,
            1.0,
            1.0,
            day_length,
            &format!("moon edge tod={tod} epoch={epoch} day={day_length}"),
        );
    }
}

/// `0x6d0530` with the four vectors pre-sized (so the resize calls are skipped), at radii 1 (the
/// client's), 0.5, 2 and random ones. The positions and texture coordinates may differ by one ulp
/// from the transcription, whose sine and cosine are the host's and the emulator's are the host's
/// system library; the colours and the 374 indices are exact.
#[test]
fn dn_sun_trig_table_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x7472_6967_u32;

    let st = emu.alloc(0xa0);
    let pos_p = emu.alloc(0xc0 * 12);
    let uv_p = emu.alloc(0xc0 * 8);
    let col_p = emu.alloc(0xc0 * 4);
    let idx_p = emu.alloc(0x176 * 2 + 8);

    for case in 0..64 {
        let radius = match case {
            0 => 1.0f32,
            1 => 0.5,
            2 => 2.0,
            _ => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 4.0 + 0.01) as f32,
        };
        emu.write_u32(st + 0x70, 0xc0);
        emu.write_u32(st + 0x74, pos_p as u32);
        emu.write_u32(st + 0x7c, 0xc0);
        emu.write_u32(st + 0x80, uv_p as u32);
        emu.write_u32(st + 0x88, 0xc0);
        emu.write_u32(st + 0x8c, col_p as u32);
        emu.write_u32(st + 0x94, 0x176);
        emu.write_u32(st + 0x98, idx_p as u32);
        for off in (0..0xc0 * 12).step_by(4) {
            emu.write_u32(pos_p + off, 0);
        }
        for off in (0..0xc0 * 8).step_by(4) {
            emu.write_u32(uv_p + off, 0);
        }
        for off in (0..0xc0 * 4).step_by(4) {
            emu.write_u32(col_p + off, 0);
        }
        for off in (0..0x176 * 2 + 8).step_by(2) {
            emu.write(idx_p + off, &[0u8, 0u8]);
        }

        emu.call(0x6d0530, st as u32, 0, &[radius.to_bits()]);

        let ours = dn_sun_trig_table(radius);
        let vcount = (emu.read_u32(st + 0x9c) >> 16) as u16;
        let icount = (emu.read_u32(st + 0x9c) & 0xffff) as u16;
        assert_eq!(
            vcount, ours.vertex_count,
            "trig vertex_count (case {case} r={radius})"
        );
        assert_eq!(
            icount, ours.index_count,
            "trig index_count (case {case} r={radius})"
        );

        for i in 0..ours.pos.len() {
            for c in 0..3 {
                let b = emu.read_f32(pos_p + (i * 12 + c * 4) as u64);
                if !f32_within_1ulp(b, ours.pos[i][c]) {
                    panic!(
                        "trig.pos[{i}][{c}] beyond 1 ULP (case {case} r={radius}): binary={:#010x} ours={:#010x}",
                        b.to_bits(),
                        ours.pos[i][c].to_bits()
                    );
                }
            }
            for c in 0..2 {
                let b = emu.read_f32(uv_p + (i * 8 + c * 4) as u64);
                if !f32_within_1ulp(b, ours.uv[i][c]) {
                    panic!(
                        "trig.uv[{i}][{c}] beyond 1 ULP (case {case} r={radius}): binary={:#010x} ours={:#010x}",
                        b.to_bits(),
                        ours.uv[i][c].to_bits()
                    );
                }
            }
            let bc = emu.read_u32(col_p + (i * 4) as u64);
            assert_eq!(
                bc, ours.color[i],
                "trig.color[{i}] (case {case} r={radius})"
            );
        }
        for i in 0..ours.index.len() {
            let mut b = [0u8; 2];
            emu_read(&emu, idx_p + (i * 2) as u64, &mut b);
            let bi = u16::from_le_bytes(b);
            assert_eq!(
                bi, ours.index[i],
                "trig.index[{i}] (case {case} r={radius})"
            );
        }
    }
}

/// Whether two floats are within one ulp, any two NaNs counting as equal; the monotonic order of
/// `f32` keeps the distance right across sign and zero.
fn f32_within_1ulp(a: f32, b: f32) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    let mono = |x: f32| -> i64 {
        let bits = x.to_bits() as i32;
        if bits < 0 {
            (i32::MIN.wrapping_sub(bits)) as i64
        } else {
            bits as i64
        }
    };
    (mono(a) - mono(b)).abs() <= 1
}

/// Reads a `u16` through the 32-bit reader.
fn emu_read(emu: &Emu, va: u64, out: &mut [u8; 2]) {
    let word = va & !0x3;
    let shift = (va & 0x3) as u32 * 8;
    let v = (emu.read_u32(word) >> shift) as u16;
    *out = v.to_le_bytes();
}

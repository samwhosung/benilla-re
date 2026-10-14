//! Unit float leaves in `[0x607920, 0x614550]`: `CGUnit_C::GetDisplayAlpha` (`0x60d2d0`),
//! `IsWithinArrivalDistance` (`0x610d50`), `NamePlateSortKey` (`0x6089a0`), `WrapAngleToPi`
//! (`0x6084f0`), `CGUnit_C::GetMerchantDiscount` (`0x612b80`), `DrunkSway` (`0x60ab20`),
//! `CGUnit_C::GetNamePlateScreenPos` (`0x60f7f0`), `CGUnit_C::RestoreAlpha` (`0x60d180`),
//! `CGUnit_C::GetFootprintParams` (`0x607920`) and the colour read of `0x614550`. The entries are
//! in `systems/objectlayer/` `unitdisplay.md`, `unitmovement.md`, `unitnames.md`, `unitbody.md`,
//! `auras.md` and `units.md`.
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. The `fcos`
//! of `cosine_pulse` and the `fmod` of `angle_wrap_pi` use the host's math, as the emulator does.

use crate::x87::fcos;

/// 1/255 (`0x8026c8`, `0x3b808081`): a byte channel to a float.
pub const INV_255: f32 = f32::from_bits(0x3b80_8081);
/// 1.0 (`0x7ff9d8`).
pub const ONE: f32 = 1.0;
/// 2/3 (`0x80abfc`, `0x3f2aaaab`): the height the name-plate anchor is raised by.
pub const TWO_THIRDS: f32 = f32::from_bits(0x3f2a_aaab);
/// 1/360 (`0x80c4dc`, `0x3b360b61`): the arrival distance, compared as its square.
pub const DIST_THRESH: f32 = f32::from_bits(0x3b36_0b61);
/// The name-plate anchor, 0.4 and 0.3 (`0xc4da50`, `0xc4da54`).
pub const ANCHOR_X: f32 = f32::from_bits(0x3ecc_cccd);
/// See [`ANCHOR_X`].
pub const ANCHOR_Y: f32 = f32::from_bits(0x3e99_999a);
/// 0.1 (`0x801620`, `0x3dcccccd`): the vendor discount for a reaction of 5 or more.
pub const DISCOUNT_REPUTATION: f32 = f32::from_bits(0x3dcc_cccd);
/// 0.05 (`0x8029c8`, `0x3d4ccccd`): the discount for each honor rank step.
pub const DISCOUNT_HONOR_STEP: f32 = f32::from_bits(0x3d4c_cccd);
/// pi/180 (`0x7ffaac`, `0x3c8efa35`): degrees to radians for the pulse phase.
pub const DEG2RAD: f32 = f32::from_bits(0x3c8e_fa35);
/// The three frequency scales of the drunk sway's cosines (`0x80c5e0`, `0x80c5dc`, `0x80c5d8`).
pub const PULSE_F1: f32 = f32::from_bits(0x3dbe_76c9); // 0.093
/// See [`PULSE_F1`].
pub const PULSE_F2: f32 = f32::from_bits(0x3e1d_b22d); // 0.154
/// See [`PULSE_F1`].
pub const PULSE_F3: f32 = f32::from_bits(0x3e47_ae14); // 0.195
/// 1/3 (`0x80c5d4`, `0x3eaaaa9f`): the mean of the three cosines.
pub const PULSE_MEAN: f32 = f32::from_bits(0x3eaa_aa9f);
/// pi/240 (`0x80c4bc`, `0x3c567750`): the sway's amplitude per unit of drunkenness.
pub const PULSE_AMP: f32 = f32::from_bits(0x3c56_7750);

/// `CGUnit_C::GetDisplayAlpha` (`0x60d2d0`, vtable slot `+0x6c`): the display row's `+0x14` alpha
/// (0 to 255, an integer) times 1/255, or 1.0 without a row. `None` is `this+0xb34 == 0`. The
/// `fild` widens the integer and the product is rounded to `f32` when stored.
pub fn base_render_alpha(alpha_raw: Option<i32>) -> f32 {
    match alpha_raw {
        Some(raw) => (f64::from(raw) * f64::from(INV_255)) as f32,
        None => ONE,
    }
}

/// `IsWithinArrivalDistance` (`0x610d50`, `ecx`, `edx` the points): whether `|a - b|^2` is under
/// `(1/360)^2`. The differences stay at 53 bits and the squares sum as `(dx^2 + dz^2) + dy^2` (`fld
/// st(2)`, `fmul`, `fld st(1)`, `fmul`, `faddp`, then `fld st(2)`, `fmul`, `faddp`). `fcompp`
/// compares the squared threshold with the sum and `test ah,0x41; jne` returns 0 for less, equal
/// and NaN, so the result is `thresh^2 > dist^2` ordered.
pub fn dist2_within_threshold(a: [f32; 3], b: [f32; 3]) -> bool {
    let dx = f64::from(a[0]) - f64::from(b[0]);
    let dy = f64::from(a[1]) - f64::from(b[1]);
    let dz = f64::from(a[2]) - f64::from(b[2]);
    let dist2 = (dx * dx + dz * dz) + dy * dy;
    let thresh2 = f64::from(DIST_THRESH) * f64::from(DIST_THRESH);
    thresh2 > dist2
}

/// `NamePlateSortKey` (`0x6089a0`): the squared distance of a screen point from the anchor (0.4,
/// 0.3), `dx * dx + dy * dy` with the differences kept at 53 bits and the sum stored as `f32`. The
/// one-time seeding of the anchor is not arithmetic.
pub fn dist2_to_anchor(px: f32, py: f32) -> f32 {
    let dx = f64::from(px) - f64::from(ANCHOR_X);
    let dy = f64::from(py) - f64::from(ANCHOR_Y);
    let acc = dx * dx + dy * dy;
    acc as f32
}

/// `WrapAngleToPi` (`0x6084f0`, `ret 4`): the angle unchanged within `[-pi, pi]`, else wrapped by
/// `fmod`. The compares are `fcomp pi; test ah,0x41; jne` (an angle above pi, which NaN does not
/// take) and `fcomp -pi; test ah,5; jp` (an angle below -pi, which NaN does not take). Above pi the
/// result is `fmod(arg + pi, 2pi) - pi`, below -pi `fmod(arg - pi, 2pi) + pi`, where pi is the
/// `f32` `0x8030b0` and 2pi the `f64` at `0x803090`, `6.283185482025146`; the `fmod` is the exact
/// truncating remainder (`__CIfmod`, `0x73f90a`).
pub fn angle_wrap_pi(arg: f32) -> f32 {
    let pi = f64::from(std::f32::consts::PI);
    let neg_pi = -f64::from(std::f32::consts::PI);
    let two_pi = f64::from_bits(0x4019_21fb_6000_0000);
    let a = f64::from(arg);
    if a > pi {
        ((a + pi) % two_pi - pi) as f32
    } else if a < neg_pi {
        ((a - pi) % two_pi + pi) as f32
    } else {
        arg
    }
}

/// `CGUnit_C::GetMerchantDiscount` (`0x612b80`): the fraction the repair and taxi costs subtract
/// from 1.0. `reaction` is `UnitReaction(vendor, player)` and gives 0.1 from 5 up; `vendor_pvp` is
/// bit 12 of the vendor's `UNIT_FIELD_FLAGS` and `honor_rank` the player's `PLAYER_BYTES_3` byte 3.
/// With the flag set, a rank of 6 or more adds 0.05 and a rank of 8 or more adds another 0.05,
/// unsigned. The sum stays on `st0` at 53 bits and both callers consume it there, so it is returned
/// unrounded.
pub fn vendor_price_discount(reaction: i32, vendor_pvp: bool, honor_rank: u8) -> f64 {
    let mut acc = if reaction >= 5 {
        f64::from(DISCOUNT_REPUTATION)
    } else {
        0.0f64
    };
    if vendor_pvp && honor_rank >= 6 {
        acc += f64::from(DISCOUNT_HONOR_STEP);
        if honor_rank >= 8 {
            acc += f64::from(DISCOUNT_HONOR_STEP);
        }
    }
    acc
}

/// `DrunkSway` (`0x60ab20`, `ecx` = `n`, stack `t`, `ret 4`): the sway angle of a drunk unit, with
/// `n` the millisecond clock and `t` the drunk fraction.
///
/// `fcomp 0.0; test ah,0x44; jp` returns 0 only for `t == 0` (either zero); a negative `t` and NaN
/// compute. `n` is zero-extended by `fild qword`. The phase `p = n * pi/180 * t` is stored to
/// `[ebp-4]` by `fst` as `f32` while `st0` keeps the unrounded `p`: the first cosine takes `p *
/// 0.093` from the register, the second and third reload the stored float. The `fst` after the
/// third cosine is never read back, so the sum `(c1 + c2) + c3` uses the unrounded `c3`. The result
/// is `sum * (1/3) * (pi/240 * t)` stored as `f32`.
pub fn cosine_pulse(n: i32, t: f32) -> f32 {
    if t == 0.0 {
        return 0.0;
    }
    let n_f = f64::from(n as u32);
    let p = n_f * f64::from(DEG2RAD) * f64::from(t);
    let p_f32 = f64::from(p as f32);

    let c1 = fcos(p * f64::from(PULSE_F1));
    let c2 = fcos(p_f32 * f64::from(PULSE_F2));
    let c3 = fcos(p_f32 * f64::from(PULSE_F3));

    let sum = (c1 + c2) + c3;

    let mean = sum * f64::from(PULSE_MEAN);
    let amp_t = f64::from(PULSE_AMP) * f64::from(t);
    (mean * amp_t) as f32
}

/// `CGUnit_C::GetNamePlateScreenPos` (`0x60f7f0`): the muzzle point `0x608640` writes has its z
/// raised by 2/3, `fld z; fadd 2/3; fstp z`.
pub fn follow_anchor_lift(vz: f32) -> f32 {
    (f64::from(vz) + f64::from(TWO_THIRDS)) as f32
}

/// `CGUnit_C::RestoreAlpha` (`0x60d180`): the base alpha from slot `+0x6c` (see
/// [`base_render_alpha`]) times the first effect's alpha `[[this+0xb50]+0x78]` when `+0xb50` is
/// set, `fld node; fmul base`. Only the head of the list is applied.
pub fn recompute_render_alpha(base: f32, node_factor: Option<f32>) -> f32 {
    match node_factor {
        Some(r) => (f64::from(r) * f64::from(base)) as f32,
        None => base,
    }
}

/// `CGUnit_C::GetFootprintParams` (`0x607920`): the scalar from vtable slot `+0x1c` times two
/// struct fields (`+0xcc4`, `+0xcc8` on the display path, else `+0xc64`, `+0xc68`), each stored as
/// `f32`.
pub fn weapon_attach_offset(r: f32, scale_a: f32, scale_b: f32) -> [f32; 2] {
    let x = (f64::from(r) * f64::from(scale_a)) as f32;
    let y = (f64::from(r) * f64::from(scale_b)) as f32;
    [x, y]
}

/// The colour read of `0x614550`: three bytes of the light configuration, `[cfg+0x178]`,
/// `[cfg+0x179]` and `[cfg+0x17a]`, each `fild` times 1/255. The three stores pop the stack in
/// reverse, so the triple that reaches `model+0x190` is `[chan2, chan1, chan0]`.
pub fn decal_rgb(bytes: [u8; 3]) -> [f32; 3] {
    let chan = |b: u8| (f64::from(i32::from(b)) * f64::from(INV_255)) as f32;
    [chan(bytes[2]), chan(bytes[1]), chan(bytes[0])]
}

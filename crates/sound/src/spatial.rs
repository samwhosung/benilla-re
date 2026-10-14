//! Positions and distances: the axis remap around the FMOD 3D calls, the audibility gates, the
//! distance gain, and the emitter searches (`systems/sound/engine.md#positions-and-the-listener`).
//!
//! The client maps a world vector (x, y, z) to FMOD as (-y, z, x). The negation is the x87 `fchs`,
//! a sign flip that Rust's unary `-` matches, so `-(-0.0)` is `+0.0` and NaN keeps its payload.

/// The remap `Sound::SetPosition` (`0x7a5b10`) and `Sound::SetListener` (`0x7a5eb0`) apply to
/// every vector before an FSOUND 3D call: (x, y, z) to (-y, z, x).
pub fn remap_wow_to_fmod(v: [f32; 3]) -> [f32; 3] {
    [-v[1], v[2], v[0]]
}

/// `Sound::GetListenerPosition` (`0x7a5f50`): FMOD's listener position (fx, fy, fz) as the world
/// vector (fz, -fx, fy), the inverse of [`remap_wow_to_fmod`].
pub fn listener_get_attrs(fmod_pos: [f32; 3]) -> [f32; 3] {
    [fmod_pos[2], -fmod_pos[0], fmod_pos[1]]
}

/// The squared distance the gates compute, each difference and square at 53 bits, summed
/// `(dz² + dy²) + dx²` (`0x45ce16`, `0x4583fb`). The sign of a difference does not matter, since
/// `x · x == (-x) · (-x)`.
fn dist_sq(listener: [f32; 3], src: [f32; 3]) -> f64 {
    let dx = listener[0] as f64 - src[0] as f64;
    let dy = listener[1] as f64 - src[1] as f64;
    let dz = listener[2] as f64 - src[2] as f64;
    (dz * dz + dy * dy) + dx * dx
}

/// `IsWithinListenerDistance` (`0x45cdf0`): `maxdist² > d²`, strict, so equal distances and NaN are
/// out of range (`systems/sound/entries.md#playing-at-a-position`).
pub fn sound_audible_distance(listener: [f32; 3], src: [f32; 3], maxdist: f32) -> bool {
    let d2 = dist_sq(listener, src);
    let r2 = maxdist as f64 * maxdist as f64;
    r2 > d2
}

/// `PlayFootstep` (`0x458380`): whether the position is close enough to play, `d² <= r²` with `r`
/// 50.0 (`0x803144`) for `far` and 20.0 (`0x803140`) otherwise. Only `d² > r²` skips, so equal
/// distances and NaN play (`systems/sound/objects.md#footstep-sounds`).
pub fn env_sound_audible(listener: [f32; 3], src: [f32; 3], far: bool) -> bool {
    let radius = if far { 50.0f32 } else { 20.0f32 };
    let r2 = radius as f64 * radius as f64;
    let d2 = dist_sq(listener, src);
    // `!(d2 > r2)` and not `d2 <= r2`: NaN must read as audible.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    {
        !(d2 > r2)
    }
}

/// `PlaceWaterLoop` (`0x462960`): the new offset of a water loop, moved from `cur` toward `target`
/// by `min(|d|, 1/6)` along the unit direction (`systems/sound/ambience.md#water-sound-placement`).
/// It snaps to `target` when `|d|² <= 1/36` (`0x3ce38e39`, the step cap is `0x3e2aaaab`).
///
/// The x87 spills differ by axis, and each is reproduced: `dy` is rounded to `f32` while `dx` and
/// `dz` stay at 53 bits; the reciprocal divides the full-precision `|d|²` while the step takes the
/// square root of the `f32`-rounded sum (`fst [ebp+8]`); the unit direction is rounded to `f32` for
/// x and y; and `dir · step` is rounded to `f32` for y and z but not for x.
pub fn source_slew(cur: [f32; 3], target: [f32; 3]) -> [f32; 3] {
    let (cx, cy, cz) = (cur[0] as f64, cur[1] as f64, cur[2] as f64);
    let (tx, ty, tz) = (target[0] as f64, target[1] as f64, target[2] as f64);
    let dx = tx - cx;
    let dy = ((ty - cy) as f32) as f64;
    let dz = tz - cz;
    let sum_full = (dy * dy + dz * dz) + dx * dx;

    let eps = f32::from_bits(0x3ce3_8e39) as f64;
    if sum_full <= eps {
        // A NaN sum fails the ordered compare and takes the slew path.
        return target;
    }

    let inv_mag = 1.0 / sum_full.sqrt();
    let mag_b = ((sum_full as f32) as f64).sqrt();
    let maxstep = f32::from_bits(0x3e2a_aaab) as f64;
    // The `jnp` keeps `mag_b` only when it is below the cap; the cap takes equal and NaN.
    let step_mag = if mag_b < maxstep { mag_b } else { maxstep };

    let dirx = (dx * inv_mag) as f32 as f64;
    let diry = (dy * inv_mag) as f32 as f64;
    let dirz = dz * inv_mag;
    let nx = ((dirx * step_mag) + cx) as f32;
    let stepy = (diry * step_mag) as f32 as f64;
    let ny = (stepy + cy) as f32;
    let stepz = (dirz * step_mag) as f32 as f64;
    let nz = (stepz + cz) as f32;
    [nx, ny, nz]
}

/// The distance gain `Sound::ProcessCutoffList` (`0x7a5000`) stores at `+0x78` for a sound within
/// its cutoff: `1 - clamp(√d² - maxdist·0.9, 0, maxdist·0.1) / (maxdist·0.1)`, with 0.9 the float
/// `0x3f666666` (`0x804588`) and 0.1 the float `0x3dccccd0` (`0x81d728`)
/// (`systems/sound/engine.md#distance-cutoff`).
///
/// `d²` and `maxdist · 0.9` are each rounded to `f32` (`[ebp-8]`, `[ebp-0xc]`) before the square root
/// and the subtraction; `maxdist · 0.1` stays at 53 bits.
pub fn channel_dist_update(listener: [f32; 3], chan_pos: [f32; 3], maxdist: f32) -> f32 {
    let d2f = dist_sq(listener, chan_pos) as f32;
    let k1 = f32::from_bits(0x3f66_6666) as f64;
    let k2 = f32::from_bits(0x3dcc_ccd0) as f64;
    let md = maxdist as f64;
    let mk1 = (md * k1) as f32 as f64;
    let mk2 = md * k2;
    let f4 = (d2f as f64).sqrt() - mk1;
    // NaN fails the ordered compare, so it takes the 0 branch.
    let f3 = if f4 > 0.0 { f4 } else { 0.0 };
    let mut f6 = mk2;
    if f3 < mk2 {
        f6 = f4;
        if f4 <= 0.0 {
            f6 = 0.0;
        }
    }
    (1.0 - f6 / mk2) as f32
}

// `Sound::ProcessPanningList` (`0x7a4e00`) places a listed sound at unit distance from the
// listener. The rotation itself is built by `0x7bbab0` and applied by `0x7bb0e0`; the three
// functions below are the arithmetic around it (`systems/sound/engine.md#panning-list`).

/// The elevation angle of `Sound::ProcessPanningList` (`0x7a4e00`, `0x7a4e3b`): `elev` clamped to
/// 0 to 1 (below 0 gives 0; 1 or above, or NaN, gives 1) times `π/2`, the float `0x3fc90fdb`
/// (`0x808acc`), rounded to `f32`.
pub fn pos2d_synth_angle(elev: f32) -> f32 {
    // `!(elev < 1.0)` and not `elev >= 1.0`: NaN must reach the high branch.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let c = if elev < 0.0 {
        0.0f32
    } else if !(elev < 1.0) {
        1.0
    } else {
        elev
    };
    (c as f64 * (f32::from_bits(0x3fc9_0fdb) as f64)) as f32
}

/// The rotation axis of `Sound::ProcessPanningList` (`0x7a4e8e` to `0x7a4f7d`): the horizontal
/// direction (dx, dy) from the listener to the sound, normalised, crossed with up, as
/// (ndy, -ndx, 0). `dx` stays at 53 bits and `dy` is rounded to `f32` before squaring; both
/// normalised components are rounded to `f32`.
///
/// The client builds the zero components from `0 - x` and `x · 0 - y · 0`, so their signs are
/// fixed: `axis[1]` is `0.0 - ndx`, which is `+0.0` for `ndx = +0.0` where `-ndx` would be `-0.0`,
/// and `axis[2]` is `-0.0` when `ndx < 0` (`0x7a4f5b` to `0x7a4f7d`).
pub fn pos2d_synth_axis(chan_x: f32, chan_y: f32, listener_x: f32, listener_y: f32) -> [f32; 3] {
    let dx = chan_x as f64 - listener_x as f64;
    let dy = ((chan_y as f64 - listener_y as f64) as f32) as f64;
    let inv = 1.0 / (dy * dy + dx * dx).sqrt();
    let norm_dx = (dx * inv) as f32;
    let norm_dy = (dy * inv) as f32;
    let ax1 = (0.0f64 - norm_dx as f64) as f32;
    let ax2 = (norm_dx as f64 * 0.0 - norm_dy as f64 * 0.0) as f32;
    [norm_dy, ax1, ax2]
}

/// The position `Sound::ProcessPanningList` (`0x7a4e00`, `0x7a4f99` to `0x7a4fd0`) hands to
/// `FSOUND_3D_SetAttributes`: the rotated point plus the listener, each sum rounded to `f32`, then
/// the remap of [`remap_wow_to_fmod`].
pub fn pos2d_synth_output(transform: [f32; 3], listener: [f32; 3]) -> [f32; 3] {
    let out = [
        (transform[0] as f64 + listener[0] as f64) as f32,
        (transform[1] as f64 + listener[1] as f64) as f32,
        (transform[2] as f64 + listener[2] as f64) as f32,
    ];
    remap_wow_to_fmod(out)
}

/// The position the play functions pass on: `pos` with z raised by 2.0 yards (`0x801628`). x and y
/// are copied; z is `pos.z + 2.0` at 53 bits, rounded to `f32`. `PlayShieldImpact` (`0x457f20`),
/// `PlayAbsorbSound` (`0x458610`), `PlayMaterialSound` (`0x4584e0`), `PlayWeaponImpactOnItem`
/// (`0x457dc0`) and `PlaySheatheSound` (`0x458530`) build it before calling `0x458890`
/// (`systems/sound/objects.md#material-and-sheathe-sounds`).
pub fn sound_emit_pos_z2(pos: &[f32; 3]) -> [f32; 3] {
    [pos[0], pos[1], (f64::from(pos[2]) + 2.0) as f32]
}

/// `NearestEmitterSlot` (`0x461ca0`): the index of the active point nearest `query`, or -1 when
/// none is active. The points are 256 `[f32; 3]` at `this + i · 0xc` and the flags are bytes at
/// `this + 0xc00 + i` (`systems/sound/ambience.md#emitter-pump`).
///
/// Each `d² = (dz² + dx²) + dy²` is formed at 53 bits, but the running minimum is stored and
/// compared as an `f32` (`fstp dword [ebp-4]`, `fcom dword [ebp-4]`), so a new `d²` is compared
/// with the rounded minimum. The update is strict (`fcom`, `test ah,5`, `jp`): a tie and a NaN keep
/// the earlier index, and the first active point seeds the minimum.
pub fn nearest_active_point(
    points: &[[f32; 3]; 256],
    active: &[bool; 256],
    query: [f32; 3],
) -> i32 {
    let (qx, qy, qz) = (query[0] as f64, query[1] as f64, query[2] as f64);
    let mut best: i32 = -1;
    let mut min_d2: f32 = 0.0;
    for (i, (pt, &on)) in points.iter().zip(active.iter()).enumerate() {
        if !on {
            continue;
        }
        let dx = qx - pt[0] as f64;
        let dy = qy - pt[1] as f64;
        let dz = qz - pt[2] as f64;
        let d2 = (dz * dz + dx * dx) + dy * dy;
        if best == -1 || d2 < min_d2 as f64 {
            min_d2 = d2 as f32;
            best = i as i32;
        }
    }
    best
}

/// `FindFartherEmitterSlot` (`0x461a60`): the first active slot whose point is farther from the
/// listener than `query`, or -1. The listener is read with `0x7a5f50`; the layout is that of
/// [`nearest_active_point`] (`systems/sound/ambience.md#emitters`).
///
/// `distA² = (az² + ay²) + ax²` for the slot is at 53 bits throughout. For `distB²` of the query,
/// only the z difference is spilled to `f32` (`fstp dword [ebp-0x10]`, `0x461abd`) before squaring.
/// The compare is strict (`fcompp`, `test ah,5`, `jnp`): a tie or NaN goes on to the next slot.
pub fn first_slot_farther_than_query(
    points: &[[f32; 3]; 256],
    active: &[bool; 256],
    listener: [f32; 3],
    query: [f32; 3],
) -> i32 {
    let (lx, ly, lz) = (listener[0] as f64, listener[1] as f64, listener[2] as f64);
    let bx = query[0] as f64 - lx;
    let by = query[1] as f64 - ly;
    let bz = ((query[2] as f64 - lz) as f32) as f64;
    let dist_b2 = (bz * bz + by * by) + bx * bx;
    for (i, (pt, &on)) in points.iter().zip(active.iter()).enumerate() {
        if !on {
            continue;
        }
        let ax = pt[0] as f64 - lx;
        let ay = pt[1] as f64 - ly;
        let az = pt[2] as f64 - lz;
        let dist_a2 = (az * az + ay * ay) + ax * ax;
        if dist_b2 < dist_a2 {
            return i as i32;
        }
    }
    -1
}

/// `Sound::SetCutoffDistanceSquared` (`0x7a5ca0`): whether the sound stays, for a spatial sound
/// (flag `0x8000000` set, `0x4000000` clear) at `pos`. A `maxdist` of 0 keeps it (the list-remove
/// path); otherwise it stays when `maxdist² > d²`, strictly, so equal distances and NaN cull
/// (`systems/sound/engine.md#distance-cutoff`). `maxdist²` is formed at 53 bits.
pub fn max_distance_cull(listener: [f32; 3], pos: [f32; 3], maxdist: f32) -> bool {
    if maxdist == 0.0 {
        return true;
    }
    let d2 = dist_sq(listener, pos);
    let r2 = maxdist as f64 * maxdist as f64;
    r2 > d2
}

/// The radius `UpdateWaterSounds` (`0x462b50`, `0x462c65` to `0x462c82`) computes once into
/// `0xcf77c4`: `√(k² + k²)` with `k = 4.1666665` (`0x40855555`, at `0x803550`). `k²` is formed at 53
/// bits, doubled exactly, and the root is stored as an `f32`
/// (`systems/sound/ambience.md#water-sound-placement`).
pub fn group_threshold_radius() -> f32 {
    let k = f32::from_bits(0x4085_5555) as f64;
    let k2 = k * k;
    (k2 + k2).sqrt() as f32
}

/// The radius test of `UpdateWaterSounds` (`0x462c82` to `0x462cf5`) on a position `point`
/// relative to the player: with `|p|² = (pz² + px²) + py²` and `R²` both at 53 bits, `R² > |p|²`
/// (strict, `0x462ca6`) gives the position `(0, 0, R)` and the flag set; otherwise the point is
/// kept and the flag clear (`systems/sound/ambience.md#water-sound-placement`).
pub fn group_clamp_position(point: [f32; 3], radius: f32) -> ([f32; 3], bool) {
    let px = point[0] as f64;
    let py = point[1] as f64;
    let pz = point[2] as f64;
    let mag2 = (pz * pz + px * px) + py * py;
    let r2 = radius as f64 * radius as f64;
    if r2 > mag2 {
        ([0.0, 0.0, radius], true)
    } else {
        (point, false)
    }
}

/// The position `StartWaterLoop` (`0x462830`) passes to `Sound::SetPosition` (`0x7a5b10`): the
/// loop's offset after [`source_slew`] plus the vector passed as its second argument, each sum
/// rounded to `f32`. The client adds in a different operand order per axis, which cannot change a
/// sum (`systems/sound/ambience.md#water-sounds`).
pub fn source_pos_compose(cur: [f32; 3], offset: [f32; 3]) -> [f32; 3] {
    [
        (offset[0] as f64 + cur[0] as f64) as f32,
        (cur[1] as f64 + offset[1] as f64) as f32,
        (cur[2] as f64 + offset[2] as f64) as f32,
    ]
}

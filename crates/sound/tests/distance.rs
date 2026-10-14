//! The distance gates and gains and the emitter searches (`systems/sound/engine.md#distance-cutoff`,
//! `systems/sound/ambience.md`).

mod common;

use common::*;

/// Listener and source pairs for the distance gates: sources at distances bracketing 20 and 50 along
/// each axis, so the result depends on the last bits of `d²`, plus a fuzz of the vector inputs.
fn synth_gate_cases() -> Vec<([f32; 3], [f32; 3])> {
    let mut v: Vec<([f32; 3], [f32; 3])> = Vec::new();
    let vecs = synth_vec3();
    // Sources at radii bracketing 20 and 50, so the result depends on the last bits of d².
    for &r in &[
        19.9999f32, 20.0, 20.0001, 49.9999, 50.0, 50.0001, 0.0, 100.0,
    ] {
        for axis in 0..3 {
            let mut s = [0.0f32; 3];
            s[axis] = r;
            v.push(([0.0, 0.0, 0.0], s));
            v.push(([1.0, -2.0, 3.0], [1.0 + r, -2.0, 3.0]));
        }
    }
    // Listeners (as FMOD positions) paired with fuzzed sources.
    for (k, &fmod_l) in vecs.iter().enumerate() {
        v.push((fmod_l, vecs[(k + 5) % vecs.len()]));
    }
    v
}

/// 256 points cycled from the vector inputs, for the emitter searches.
fn synth_point_cloud() -> [[f32; 3]; 256] {
    let v = synth_vec3();
    let mut points = [[0.0f32; 3]; 256];
    for (i, p) in points.iter_mut().enumerate() {
        *p = v[i % v.len()];
    }
    points
}

fn synth_active_patterns() -> Vec<[bool; 256]> {
    let mut pats: Vec<[bool; 256]> = Vec::new();
    pats.push([false; 256]); // none active
    let mut single = [false; 256];
    single[42] = true;
    pats.push(single);
    let mut sparse = [false; 256];
    for &i in &[0usize, 3, 17, 64, 100, 200, 255] {
        sparse[i] = true;
    }
    pats.push(sparse);
    pats.push([true; 256]); // all active
    let mut stride = [false; 256];
    for i in (2..256).step_by(7) {
        stride[i] = true;
    }
    pats.push(stride);
    pats
}

/// Runs `IsWithinListenerDistance` (`0x45cdf0`) whole, with `Sound::GetListenerPosition` (`0x7a5f50`)
/// run for real on a chosen FMOD position (the `FSOUND_3D_Listener_GetAttributes` stub). The cases
/// include distances either side of each radius.
#[test]
fn sound_audible_distance() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let (src_buf, payload) = (d.emu.alloc(0x10), d.payload);
    d.emu.stub_write_through(0x7fdb50, 0, payload, 12, 0, 8); // FSOUND_3D_Listener_GetAttributes
    let maxdists = [0.0f32, 1.0, 20.0, 50.0, 100.0, 1e6, f32::NAN, f32::INFINITY];
    for (k, (fmod_l, src)) in synth_gate_cases().into_iter().enumerate() {
        let maxdist = maxdists[k % maxdists.len()];
        d.put_vec3(payload, &fmod_l);
        d.put_vec3(src_buf, &src);
        let al = d
            .emu
            .call(0x45cdf0, src_buf as u32, 0, &[maxdist.to_bits()])
            & 0xff;
        let listener = benilla_re_sound::listener_get_attrs(fmod_l);
        let want = benilla_re_sound::sound_audible_distance(listener, src, maxdist);
        assert_eq!(
            al == 1,
            want,
            "sound_audible_distance(L={listener:?}, src={src:?}, maxdist={maxdist:?})"
        );
    }
}

/// Runs `PlayFootstep` (`0x458380`) whole with the CVar lookup `0x63de30` returning a config whose
/// `+0x28` is nonzero (sound effects on), `GetFootstepSound` (`0x458450`) stubbed to return 0, and the listener supplied through
/// the `FSOUND_3D_Listener_GetAttributes` stub. Whether `0x458450` is reached shows the gate.
#[test]
fn env_sound_audible() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let (src_buf, payload) = (d.emu.alloc(0x10), d.payload);
    let cfg = d.emu.alloc(0x40);
    d.emu.write_u32(cfg + 0x28, 1); // sound effects on
    d.emu.stub(0x63de30, cfg as u32, 0); // CVar::Lookup returns the config
    d.emu.stub(0x458450, 0, 1); // GetFootstepSound returns 0; one stack argument
    d.emu.stub_write_through(0x7fdb50, 0, payload, 12, 0, 8); // FSOUND_3D_Listener_GetAttributes
    for (k, (fmod_l, src)) in synth_gate_cases().into_iter().enumerate() {
        let far = k % 2 == 0;
        d.put_vec3(payload, &fmod_l);
        d.put_vec3(src_buf, &src);
        d.emu.clear_calls();
        d.emu
            .call(0x458380, 0, src_buf as u32, &[0, 0, u32::from(far)]);
        let reached = d.emu.calls().iter().any(|c| c.addr == 0x458450);
        let listener = benilla_re_sound::listener_get_attrs(fmod_l);
        let want = benilla_re_sound::env_sound_audible(listener, src, far);
        assert_eq!(
            reached, want,
            "env_sound_audible(L={listener:?}, src={src:?}, far={far})"
        );
    }
}

/// Runs `Sound::ProcessCutoffList` (`0x7a5000`) whole on a one-node list (`0xcf5588` the head,
/// `0xcf5580` the link base) with the list and release helpers stubbed, over distances from 0 to
/// exactly the cutoff along each axis. It compares the gain stored at `+0x78`; beyond the cutoff the
/// sound is released and nothing is stored, so those cases are not run.
#[test]
fn channel_dist_update() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let node = d.chan;
    let listener_buf = d.emu.alloc(0x10);
    // A one-node list: the head is the node, the link base 0, the node's next 0.
    d.emu.write_u32(0xcf5588, node as u32);
    d.emu.write_u32(0xcf5580, 0);
    d.emu.write_u32(node + 4, 0);
    d.emu.write_u32(node + 0x30, 0); // bound handle (not -1)
    d.emu.write_u32(node + 0x38, 0); // flags: 0x20000 and the release bits clear
                                     // The list and release helpers on the gain path are stubbed away.
    for &addr in &[
        0x7a5870u64,
        0x7a5880,
        0x7a5900,
        0x7a59e0,
        0x7a65a0,
        0x7a6930,
        0x7a6960,
        0x7a4200,
    ] {
        d.emu.stub(addr, 0, 0);
    }
    const SENTINEL: u32 = 0xdead_beef;
    // Separations that are fractions of maxdist, spanning the 0.9·maxdist radius and the falloff band.
    let listeners = [[0.0f32, 0.0, 0.0], [10.0, -5.0, 3.0], [-100.0, 50.0, 7.0]];
    let maxdists = [5.0f32, 10.0, 50.0, 100.0];
    let fracs = [0.0f32, 0.5, 0.85, 0.9, 0.95, 0.99, 1.0];
    for &l in &listeners {
        d.put_vec3(listener_buf, &l);
        for &md in &maxdists {
            for &frac in &fracs {
                for axis in 0..3 {
                    let mut pos = l;
                    pos[axis] += md * frac; // md·frac along one axis
                    let dist = md * frac;
                    if dist > md {
                        continue; // out of range: released, no gain stored
                    }
                    d.put_vec3(node + 0x50, &pos);
                    d.put_f32(node + 0x70, md);
                    d.put_f32(node + 0x78, f32::from_bits(SENTINEL));
                    // The head is written again each case: the walk consumes it.
                    d.emu.write_u32(0xcf5588, node as u32);
                    d.emu.call(0x7a5000, listener_buf as u32, 0, &[]);
                    assert_f32(
                        d.emu.read_f32(node + 0x78),
                        benilla_re_sound::channel_dist_update(l, pos, md),
                        &format!("channel_dist_update(l={l:?}, pos={pos:?}, md={md})"),
                    );
                }
            }
        }
    }
}

/// Runs `Sound::SetCutoffDistanceSquared` (`0x7a5ca0`) whole on a spatial channel (flag `0x8000000`)
/// with `Sound::AddToCutoffList` (`0x7a6240`) and `Sound::RemoveFromCutoffList` (`0x7a62b0`) stubbed and
/// `Sound::GetListenerPosition` (`0x7a5f50`) run for real. It compares the returned keep flag over
/// the distances of `synth_gate_cases`, including a cutoff of 0, -0.0, NaN and infinity.
#[test]
fn max_distance_cull() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let chan = d.chan;
    let payload = d.payload;
    d.emu.write_u32(chan + 0x38, 0x0800_0000); // spatial flag set, 0x4000000 clear
    d.emu.stub(0x7a6240, 0, 0); // Sound::AddToCutoffList
    d.emu.stub(0x7a62b0, 0, 0); // Sound::RemoveFromCutoffList
    d.emu.stub_write_through(0x7fdb50, 0, payload, 12, 0, 8); // FSOUND_3D_Listener_GetAttributes
    let maxdists = [
        0.0f32,
        1.0,
        20.0,
        50.0,
        100.0,
        1e6,
        f32::NAN,
        f32::INFINITY,
        -0.0,
        5.0,
    ];
    for (k, (fmod_l, src)) in synth_gate_cases().into_iter().enumerate() {
        let maxdist = maxdists[k % maxdists.len()];
        d.put_vec3(payload, &fmod_l);
        d.put_vec3(chan + 0x50, &src); // the sound's position
        let al = d.emu.call(0x7a5ca0, chan as u32, 0, &[maxdist.to_bits()]) & 0xff;
        let listener = benilla_re_sound::listener_get_attrs(fmod_l);
        let want = benilla_re_sound::max_distance_cull(listener, src, maxdist);
        assert_eq!(
            al == 1,
            want,
            "max_distance_cull(L={listener:?}, src={src:?}, maxdist={maxdist:?})"
        );
    }
}

/// Runs `NearestEmitterSlot` (`0x461ca0`) whole on a synthesized table of 256 points and flags, over
/// active patterns from none to all, with queries that hit a point, miss by a hair, tie, and are NaN.
#[test]
fn nearest_active_point() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let this = d.emu.alloc(0xe00); // 256 points of 0xc bytes, then 256 flag bytes at +0xc00
    let q_buf = d.emu.alloc(0x10);
    let points = synth_point_cloud();
    let v = synth_vec3();
    // Queries: the origin, a point itself, near misses, a tie, far away, NaN, and a fuzz.
    let mut queries: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],
        points[100],
        [points[3][0] + 0.001, points[3][1], points[3][2]],
        [1e9, 1e9, 1e9],
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::NAN, 1.0],
        [
            (points[0][0] + points[255][0]) * 0.5,
            (points[0][1] + points[255][1]) * 0.5,
            (points[0][2] + points[255][2]) * 0.5,
        ],
    ];
    queries.extend(v.iter().take(40).copied());

    for active in &synth_active_patterns() {
        for (i, p) in points.iter().enumerate() {
            d.put_vec3(this + i as u64 * 0xc, p);
            d.emu.write(this + 0xc00 + i as u64, &[u8::from(active[i])]);
        }
        for &query in &queries {
            d.put_vec3(q_buf, &query);
            let eax = d.emu.call(0x461ca0, this as u32, 0, &[q_buf as u32]) as i32;
            assert_eq!(
                eax,
                benilla_re_sound::nearest_active_point(&points, active, query),
                "nearest_active_point(query={query:?})"
            );
        }
    }
}

/// Runs `FindFartherEmitterSlot` (`0x461a60`) whole on a synthesized table, with
/// `Sound::GetListenerPosition` (`0x7a5f50`) run for real on a chosen FMOD listener.
#[test]
fn first_slot_farther_than_query() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let this = d.emu.alloc(0xe00);
    let q_buf = d.emu.alloc(0x10);
    let payload = d.payload;
    d.emu.stub_write_through(0x7fdb50, 0, payload, 12, 0, 8); // FSOUND_3D_Listener_GetAttributes
    let points = synth_point_cloud();
    let fmod_listeners = [[0.0f32, 0.0, 0.0], [10.0, -5.0, 3.0], [-100.0, 50.0, 7.0]];
    // Queries from nearer than some slots (one is found) to farther than all (-1).
    let queries: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],
        [5.0, 5.0, 5.0],
        points[17],
        [1e9, 1e9, 1e9],
        [f32::NAN, 0.0, 0.0],
        [1.0, -2.0, 3.0],
        [-50.0, 25.0, 4.0],
    ];
    for active in &synth_active_patterns() {
        for (i, p) in points.iter().enumerate() {
            d.put_vec3(this + i as u64 * 0xc, p);
            d.emu.write(this + 0xc00 + i as u64, &[u8::from(active[i])]);
        }
        for &fmod_l in &fmod_listeners {
            d.put_vec3(payload, &fmod_l);
            let listener = benilla_re_sound::listener_get_attrs(fmod_l);
            for &query in &queries {
                d.put_vec3(q_buf, &query);
                let eax = d.emu.call(0x461a60, this as u32, 0, &[q_buf as u32]) as i32;
                assert_eq!(
                    eax,
                    benilla_re_sound::first_slot_farther_than_query(
                        &points, active, listener, query
                    ),
                    "first_slot_farther_than_query(L={listener:?}, query={query:?})"
                );
            }
        }
    }
}

/// Runs two slices of `UpdateWaterSounds` (`0x462b50`): `0x462c65` to `0x462c82` computes the
/// radius into `0xcf77c4`, and `0x462c82` to `0x462cf5` gates a point against it, giving the flag at
/// `[ebp-1]` and the position at `[ebp-0x18]`. The radii include NaN and the radius itself.
#[test]
fn group_update_radius_clamp() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);

    // The radius, computed by the function's own bytes.
    d.emu.set_reg(RegisterX86::EAX, 0); // al holds the byte at 0xcf77c0 that marks the radius computed
    d.emu.run(0x462c65, 0x462c82);
    let real_r = d.emu.read_f32(0xcf77c4);
    assert_f32(
        real_r,
        benilla_re_sound::group_threshold_radius(),
        "group_threshold_radius @0xcf77c4",
    );

    // The gate and the clamp, over radii and points.
    let pt = d.emu.alloc(0x10);
    let frame = d.emu.alloc(0x40);
    let ebp = frame + 0x40; // the locals lie at [ebp-0x2c, ebp-1]
    let esi_scratch = d.emu.alloc(0x10);
    let mut points: Vec<[f32; 3]> = synth_vec3();
    for &mag in &[real_r, 1.0, 5.0, 6.0, 100.0] {
        for &f in &[0.0f32, 0.5, 0.99, 1.0, 1.01, 2.0] {
            let m = mag * f;
            points.push([m, 0.0, 0.0]);
            points.push([0.0, m, 0.0]);
            points.push([0.0, 0.0, m]);
            let s = m * 0.5773503; // about 1/√3, so |p| is about m along the diagonal
            points.push([s, s, s]);
        }
    }
    let radii = [real_r, 0.0, 1.0, 5.892556, 10.0, 100.0, f32::NAN];
    for &radius in &radii {
        d.put_f32(0xcf77c4, radius);
        for &p in &points {
            d.put_vec3(pt, &p);
            d.emu.set_reg(RegisterX86::EDX, pt);
            d.emu.set_reg(RegisterX86::ESI, esi_scratch);
            d.emu.set_reg(RegisterX86::EBP, ebp);
            d.emu.run(0x462c82, 0x462cf5);
            let flag = d.emu.read(ebp - 1, 1)[0] != 0;
            let pos = d.read_vec3(ebp - 0x18);
            let (want_pos, want_flag) = benilla_re_sound::group_clamp_position(p, radius);
            assert_eq!(flag, want_flag, "group gate flag (p={p:?}, R={radius:?})");
            assert_vec3(
                pos,
                want_pos,
                &format!("group clamp pos (p={p:?}, R={radius:?})"),
            );
        }
    }
}

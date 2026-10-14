//! The axis remap around the FMOD 3D calls, the listener and the emitter positions
//! (`systems/sound/engine.md#positions-and-the-listener`).

mod common;

use common::*;

/// Runs `Sound::SetPosition` (`0x7a5b10`) whole on a channel with a bound handle, with
/// `FSOUND_3D_SetAttributes` (`0x7fdb56`) stubbed, with and without a velocity. The remapped vectors are
/// read through the pointers the stub captures.
#[test]
fn source_3d_attrs() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let chan = d.chan;
    d.emu.write_u32(chan + 0x38, 0x0800_0000); // the 3D flag
    d.emu.write_u32(chan + 0x88, 0); // Sound::IsSuspended false: reaches FMOD
    d.emu.write_u32(chan + 0x30, 0); // bound handle (not -1)
    d.emu.stub(0x7fdb56, 0, 3); // FSOUND_3D_SetAttributes(channel, pos*, vel*)
    let pos_buf = d.emu.alloc(0x10);
    let vel_buf = d.emu.alloc(0x10);
    let vecs = synth_vec3();
    for (k, &pos) in vecs.iter().enumerate() {
        let vel = vecs[(k + 1) % vecs.len()];
        // Even k passes a velocity, odd k a null pointer.
        let with_vel = k % 2 == 0;
        d.put_vec3(pos_buf, &pos);
        d.put_vec3(vel_buf, &vel);
        d.emu.clear_calls();
        let vel_arg = if with_vel { vel_buf as u32 } else { 0 };
        d.emu
            .call(0x7a5b10, chan as u32, 0, &[pos_buf as u32, vel_arg]);
        let calls = d.emu.calls();
        assert_eq!(calls.len(), 1, "source_3d_attrs: one FMOD call (k={k})");
        let pos_fmod = d.read_vec3(calls[0].args[1] as u64);
        assert_vec3(
            pos_fmod,
            benilla_re_sound::remap_wow_to_fmod(pos),
            &format!("source_3d_attrs pos (k={k}, pos={pos:?})"),
        );
        if with_vel {
            let vel_fmod = d.read_vec3(calls[0].args[2] as u64);
            assert_vec3(
                vel_fmod,
                benilla_re_sound::remap_wow_to_fmod(vel),
                &format!("source_3d_attrs vel (k={k}, vel={vel:?})"),
            );
        } else {
            assert_eq!(calls[0].args[2], 0, "source_3d_attrs: null vel ptr (k={k})");
        }
    }
}

/// Runs `Sound::SetListener` (`0x7a5eb0`) whole with `FSOUND_3D_Listener_SetAttributes` (`0x7fdb9e`)
/// stubbed. Position and velocity are read through the captured pointers, forward and up from the
/// six scalar arguments.
#[test]
fn listener_3d_attrs() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    d.emu.stub(0x7fdb9e, 0, 8); // FSOUND_3D_Listener_SetAttributes: eight arguments
    let pos_buf = d.emu.alloc(0x10);
    let vel_buf = d.emu.alloc(0x10);
    let fwd_buf = d.emu.alloc(0x10);
    let up_buf = d.emu.alloc(0x10);
    let vecs = synth_vec3();
    for (k, &pos) in vecs.iter().enumerate() {
        let vel = vecs[(k + 1) % vecs.len()];
        let fwd = vecs[(k + 2) % vecs.len()];
        let up = vecs[(k + 3) % vecs.len()];
        let with_vel = k % 2 == 0;
        d.put_vec3(pos_buf, &pos);
        d.put_vec3(vel_buf, &vel);
        d.put_vec3(fwd_buf, &fwd);
        d.put_vec3(up_buf, &up);
        d.emu.clear_calls();
        let vel_arg = if with_vel { vel_buf as u32 } else { 0 };
        d.emu.call(
            0x7a5eb0,
            pos_buf as u32,
            vel_arg,
            &[fwd_buf as u32, up_buf as u32],
        );
        let calls = d.emu.calls();
        assert_eq!(calls.len(), 1, "listener_3d_attrs: one FMOD call (k={k})");
        let a = calls[0].args;
        let ctx = format!("listener_3d_attrs (k={k})");
        // The position, through args[0].
        assert_vec3(
            d.read_vec3(a[0] as u64),
            benilla_re_sound::remap_wow_to_fmod(pos),
            &format!("{ctx} pos"),
        );
        // The velocity, through args[1], or null.
        if with_vel {
            assert_vec3(
                d.read_vec3(a[1] as u64),
                benilla_re_sound::remap_wow_to_fmod(vel),
                &format!("{ctx} vel"),
            );
        } else {
            assert_eq!(a[1], 0, "{ctx}: null vel ptr");
        }
        // Forward, as the scalars args[2..5].
        let fwd_fmod = [
            f32::from_bits(a[2]),
            f32::from_bits(a[3]),
            f32::from_bits(a[4]),
        ];
        assert_vec3(
            fwd_fmod,
            benilla_re_sound::remap_wow_to_fmod(fwd),
            &format!("{ctx} fwd"),
        );
        // Up, as the scalars args[5..8].
        let up_fmod = [
            f32::from_bits(a[5]),
            f32::from_bits(a[6]),
            f32::from_bits(a[7]),
        ];
        assert_vec3(
            up_fmod,
            benilla_re_sound::remap_wow_to_fmod(up),
            &format!("{ctx} up"),
        );
    }
}

/// Runs `Sound::GetListenerPosition` (`0x7a5f50`) whole; the `FSOUND_3D_Listener_GetAttributes`
/// stub (`0x7fdb50`) writes a chosen FMOD position through its pointer argument.
#[test]
fn listener_get_attrs() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let (out, payload) = (d.out, d.payload);
    // Registered once; each case rewrites the payload the stub copies out. GetAttributes writes
    // the 12-byte position through its first stack argument and pops eight.
    d.emu.stub_write_through(0x7fdb50, 0, payload, 12, 0, 8);
    for fmod_pos in synth_vec3() {
        d.put_vec3(payload, &fmod_pos);
        d.emu.call(0x7a5f50, out as u32, 0, &[]);
        assert_vec3(
            d.read_vec3(out),
            benilla_re_sound::listener_get_attrs(fmod_pos),
            &format!("listener_get_attrs({fmod_pos:?})"),
        );
    }
}

/// Runs `PlaceWaterLoop` (`0x462960`) whole on the emitter at index 0: current offset at `+0x1c`,
/// target at `+0x40`. The cases include a snap, steps either side of the snap distance, and a fuzz.
#[test]
fn source_slew() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let base = d.chan;
    let vecs = synth_vec3();
    let mut cases: Vec<([f32; 3], [f32; 3])> = Vec::new();
    for (k, &cur) in vecs.iter().enumerate() {
        let tgt = vecs[(k + 11) % vecs.len()];
        cases.push((cur, tgt));
        cases.push((cur, cur)); // a snap, Δ = 0
                                // Targets near the snap distance, |Δ|² of 1/36.
        cases.push((cur, [cur[0] + 0.1, cur[1], cur[2]]));
        cases.push((cur, [cur[0] + 0.16, cur[1] - 0.001, cur[2] + 0.05]));
    }
    for (cur, tgt) in cases {
        d.put_vec3(base + 0x1c, &cur);
        d.put_vec3(base + 0x40, &tgt);
        d.emu.call(0x462960, base as u32, 0, &[0]);
        assert_vec3(
            d.read_vec3(base + 0x1c),
            benilla_re_sound::source_slew(cur, tgt),
            &format!("source_slew(cur={cur:?}, tgt={tgt:?})"),
        );
    }
}

/// Runs `StartWaterLoop` (`0x462830`) whole with a bound sound (`base[0]` nonzero), so that
/// `PlaceWaterLoop` (`0x462960`) runs for real and `Sound::SetPosition` (`0x7a5b10`) is stubbed to
/// capture the position vector.
#[test]
fn source_pos_compose() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let base = d.chan;
    d.emu.write_u32(base, 1); // base[0], the sound, is nonzero
    d.emu.stub(0x7a5b10, 0, 2); // Sound::SetPosition(pos*, vel*): the position is args[0]
    let offset_buf = d.emu.alloc(0x10);
    let vecs = synth_vec3();
    let mut cases: Vec<([f32; 3], [f32; 3], [f32; 3])> = Vec::new();
    for (k, &cur) in vecs.iter().enumerate() {
        let target = vecs[(k + 11) % vecs.len()];
        let offset = vecs[(k + 5) % vecs.len()];
        cases.push((cur, target, offset));
        cases.push((cur, cur, offset)); // the slew snaps: cur is unchanged
    }
    for (cur, target, offset) in cases {
        d.put_vec3(base + 0x1c, &cur); // current offset of emitter 0
        d.put_vec3(base + 0x40, &target); // target offset of emitter 0
        d.put_vec3(offset_buf, &offset);
        d.emu.clear_calls();
        d.emu
            .call(0x462830, base as u32, 0, &[0, offset_buf as u32]); // index 0, the vector
        let calls = d.emu.calls();
        let fs = calls
            .iter()
            .find(|c| c.addr == 0x7a5b10)
            .unwrap_or_else(|| panic!("source_pos_compose: 0x7a5b10 not reached (cur={cur:?})"));
        let result = d.read_vec3(fs.args[0] as u64);
        let cur_slewed = benilla_re_sound::source_slew(cur, target);
        assert_vec3(
            result,
            benilla_re_sound::source_pos_compose(cur_slewed, offset),
            &format!("source_pos_compose(cur={cur:?}, target={target:?}, offset={offset:?})"),
        );
    }
}

/// Runs the slice `0x7a4b26` to `0x7a4b3d` of `Sound::Update` (`0x7a4ad0`), which rebuilds the
/// world listener position from the FMOD one read by `FSOUND_3D_Listener_GetAttributes`.
#[test]
fn pump_listener_flip() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let frame = d.emu.alloc(0x40);
    let ebp = frame + 0x20; // [ebp-0x18, ebp-0x4] lie inside `frame`
    for fmod_pos in synth_vec3() {
        d.emu.set_reg(RegisterX86::EBP, ebp);
        // The FMOD position GetAttributes wrote: fx, fy, fz at [ebp-0x18], [ebp-0x14], [ebp-0x10].
        d.put_vec3(ebp - 0x18, &fmod_pos);
        d.emu.run(0x7a4b26, 0x7a4b3d);
        // The world listener: fz, -fx, fy at [ebp-0xc], [ebp-0x8], [ebp-0x4].
        let listener = d.read_vec3(ebp - 0xc);
        assert_vec3(
            listener,
            benilla_re_sound::listener_get_attrs(fmod_pos),
            &format!("pump_listener_flip(fmod={fmod_pos:?})"),
        );
    }
}

/// Runs the slice `0x7a4dae` to `0x7a4ddf` of `Sound::ProcessUpdateList` (`0x7a4d50`), which stores
/// the object's position into the sound and builds the FMOD vector from it. It compares both.
#[test]
fn sibling_remap_flip() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let frame = d.emu.alloc(0x40);
    let stack = d.emu.alloc(0x200);
    let chan = d.chan;
    let ebp = frame + 0x20; // [ebp-0x18, ebp-0x4] lie inside `frame`
    for result in synth_vec3() {
        d.emu.set_reg(RegisterX86::EBP, ebp);
        d.emu.set_reg(RegisterX86::ESP, stack + 0x100); // room below for the three pushes
        d.emu.set_reg(RegisterX86::EBX, chan); // the sound; its position at +0x50 receives the copy
                                               // The object's position: x, y, z at [ebp-0xc], [ebp-0x8], [ebp-0x4].
        d.put_vec3(ebp - 0xc, &result);
        d.emu.run(0x7a4dae, 0x7a4ddf);
        // The FMOD vector: -y, z, x at [ebp-0x18], [ebp-0x14], [ebp-0x10].
        let fmod = d.read_vec3(ebp - 0x18);
        assert_vec3(
            fmod,
            benilla_re_sound::remap_wow_to_fmod(result),
            &format!("sibling_remap_flip fmod (result={result:?})"),
        );
        // The sound's position at +0x50 is a verbatim copy.
        let copy = d.read_vec3(chan + 0x50);
        assert_vec3(
            copy,
            result,
            &format!("sibling_remap_flip chan-pos copy (result={result:?})"),
        );
    }
}

/// Runs `Sound::ProcessPanningList` (`0x7a4e00`) whole on a one-node list (`0xcf55d4` the head,
/// `0xcf55cc` the link base). The rotation (`0x7bbab0`, `0x7bb0e0`) and `FSOUND_3D_SetAttributes` are
/// stubbed: the test reads the angle and axis passed to `0x7bbab0`, feeds a chosen result from
/// `0x7bb0e0`, and reads the position passed to FMOD.
#[test]
fn channel_2d_position_synth() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let node = d.chan;
    let listener_buf = d.emu.alloc(0x10);
    let xform = d.emu.alloc(0x10); // the result `0x7bb0e0` returns by address
                                   // A one-node list: the head is the node, the link base 0, the node's next 0.
    d.emu.write_u32(0xcf55d4, node as u32);
    d.emu.write_u32(0xcf55cc, 0);
    d.emu.write_u32(node + 4, 0);
    d.emu.write_u32(node + 0x30, 0); // bound handle (not -1)
    d.emu.stub(0x7bbab0, 0, 3); // builds the rotation: args are the angle and the axis pointer
    d.emu.stub(0x7bb0e0, xform as u32, 1); // applies it: returns our result
    d.emu.stub(0x7fdb56, 0, 3); // FSOUND_3D_SetAttributes: args[1] is the position
    let listeners = [[0.0f32, 0.0, 0.0], [12.0, -7.0, 3.0]];
    let elevs = [0.0f32, 0.25, 0.5, 0.75, 1.0, -0.3, 1.4, f32::NAN];
    let offsets = [[10.0f32, 0.0], [3.0, 4.0], [-8.0, 6.0], [1.0, -20.0]];
    let xforms = synth_vec3();
    for &l in &listeners {
        d.put_vec3(listener_buf, &l);
        for (ei, &elev) in elevs.iter().enumerate() {
            for (oi, &[ox, oy]) in offsets.iter().enumerate() {
                let (cx, cy) = (l[0] + ox, l[1] + oy);
                let t = xforms[(ei * offsets.len() + oi) % xforms.len()];
                d.put_f32(node + 0x4c, elev);
                d.put_f32(node + 0x50, cx);
                d.put_f32(node + 0x54, cy);
                d.put_vec3(xform, &t);
                d.emu.write_u32(0xcf55d4, node as u32); // the head is written again each case
                d.emu.clear_calls();
                d.emu.call(0x7a4e00, listener_buf as u32, 0, &[]);
                let calls = d.emu.calls();
                let ctx =
                    format!("channel_2d_position_synth(l={l:?}, elev={elev}, off=[{ox},{oy}])");
                // The angle and axis passed to `0x7bbab0`.
                let rot = calls
                    .iter()
                    .find(|c| c.addr == 0x7bbab0)
                    .unwrap_or_else(|| panic!("{ctx}: 0x7bbab0 not called"));
                assert_f32(
                    f32::from_bits(rot.args[0]),
                    benilla_re_sound::pos2d_synth_angle(elev),
                    &format!("{ctx} angle"),
                );
                assert_vec3(
                    d.read_vec3(rot.args[1] as u64),
                    benilla_re_sound::pos2d_synth_axis(cx, cy, l[0], l[1]),
                    &format!("{ctx} axis"),
                );
                // The position passed to FMOD, built from the chosen transform result.
                let fs = calls
                    .iter()
                    .find(|c| c.addr == 0x7fdb56)
                    .unwrap_or_else(|| panic!("{ctx}: FSOUND not called"));
                assert_vec3(
                    d.read_vec3(fs.args[1] as u64),
                    benilla_re_sound::pos2d_synth_output(t, l),
                    &format!("{ctx} output"),
                );
            }
        }
    }
}

/// Runs the position build of five play functions. `PlayShieldImpact` (`0x457f20`), `PlayAbsorbSound`
/// (`0x458610`) and `PlayMaterialSound` (`0x4584e0`) run whole, with their gates satisfied and the
/// play path `0x458890` stubbed to capture the position. `PlayWeaponImpactOnItem` (`0x457dc0`) and
/// `PlaySheatheSound` (`0x458530`) sit behind a type dispatch and a hash lookup, so only the build runs:
/// `0x457e12` to `0x457e3d` and `0x4585d0` to `0x4585f7`.
#[test]
fn sound_emit_pos_z2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    d.emu.write_u32(0xb05fb8, 1); // the enable global of `0x457f20`
    d.emu.write_u32(0xb05fb4, 1); // the enable global of `0x458610`
    d.emu.stub(0x458890, 0, 3); // PlaySoundAtPosition: args[0] is the position
    let this = d.emu.alloc(0x10);
    for &addr in &[0x457f20u64, 0x458610] {
        for pos in synth_vec3() {
            d.put_vec3(this, &pos);
            d.emu.clear_calls();
            d.emu.call(addr, this as u32, 0, &[]);
            let calls = d.emu.calls();
            assert_eq!(
                calls.len(),
                1,
                "sound_emit_pos_z2 @{addr:#x}: one downstream call"
            );
            let v = d.read_vec3(calls[0].args[0] as u64);
            assert_vec3(
                v,
                benilla_re_sound::sound_emit_pos_z2(&pos),
                &format!("sound_emit_pos_z2 @{addr:#x} pos={pos:?}"),
            );
        }
    }
    // `0x4584e0` takes the point in `edx` and is gated on an array: `ecx` within the count at
    // 0xc0da98, a nonzero entry in the array at 0xc0da94, and a nonzero `+8` in the entry.
    let array = d.emu.alloc(0x40);
    let entry = d.emu.alloc(0x40);
    d.emu.write_u32(0xc0da98, 4); // the count: index 0 is in range
    d.emu.write_u32(0xc0da94, array as u32); // the array
    d.emu.write_u32(array, entry as u32); // array[0] is the entry
    d.emu.write_u32(entry + 8, 1); // the entry's +8 is nonzero
    let pt = d.emu.alloc(0x10);
    for pos in synth_vec3() {
        d.put_vec3(pt, &pos);
        d.emu.clear_calls();
        d.emu.call(0x4584e0, 0, pt as u32, &[]); // ecx is the index 0, edx the point
        let calls = d.emu.calls();
        assert_eq!(
            calls.len(),
            1,
            "sound_emit_pos_z2 @0x4584e0: one downstream call"
        );
        let v = d.read_vec3(calls[0].args[0] as u64);
        assert_vec3(
            v,
            benilla_re_sound::sound_emit_pos_z2(&pos),
            &format!("sound_emit_pos_z2 @0x4584e0 pos={pos:?}"),
        );
    }
    // `0x457dc0` and `0x458530` sit behind a type dispatch and a hash lookup. Their build has the
    // bytes of `0x457f20`, so it runs alone as a slice, and its three contiguous floats are the
    // vector each passes to `0x458890`:
    //   0x457dc0: build 0x457e12 to 0x457e3d, point at [ebp+0xc], floats from [ebp-0x14].
    //   0x458530: build 0x4585d0 to 0x4585f7, point at [ebp+8], floats from [ebp-0xc].
    for &(addr, begin, until, pt_off, out_off) in &[
        (0x457dc0u64, 0x457e12u64, 0x457e3du64, 0xci64, -0x14i64),
        (0x458530, 0x4585d0, 0x4585f7, 0x8, -0xc),
    ] {
        for pos in synth_vec3() {
            let built = d.emit_build_slice(begin, until, pt_off, out_off, &pos);
            assert_vec3(
                built,
                benilla_re_sound::sound_emit_pos_z2(&pos),
                &format!("sound_emit_pos_z2 @{addr:#x} (build slice) pos={pos:?}"),
            );
        }
    }
}

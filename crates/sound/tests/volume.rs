//! Volume levels and fade rates, and the per-play variation: the values the client computes before it
//! calls FMOD (`systems/sound/engine.md`, `systems/sound/entries.md`).

mod common;

use common::*;

/// The threshold `0x801620` (0.1) that `Sound::FadeIn` and `Sound::FadeOut` compare the time against.
const MINVOL: f32 = f32::from_bits(0x3dcc_cccd);

/// Whether the fade functions take the audible path: `!(vol < MINVOL)`, so NaN does.
fn audible(vol: f32) -> bool {
    // `!(vol < MINVOL)` and not `vol >= MINVOL`: NaN takes the audible path.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    {
        !(vol < MINVOL)
    }
}

/// Runs `Sound::GetVolumeLevel` (`0x7a5d70`) whole on a channel with a bound handle (`+0x30` of 0) and
/// an unbound one (-1), over the volume inputs.
#[test]
fn channel_volume_curve() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let chan = d.chan;
    for &slot in &[0i32, -1] {
        d.emu.write_u32(chan + 0x30, slot as u32);
        for &v in &synth_f32() {
            d.put_f32(chan + 0x74, v);
            let got = d.emu.call(0x7a5d70, chan as u32, 0, &[]);
            assert_eq!(
                got,
                benilla_re_sound::channel_volume_curve(slot, v),
                "channel_volume_curve(slot={slot}, v={v:?}) [{:#010x}]",
                v.to_bits()
            );
        }
    }
}

/// Runs `Sound::SetMasterVolume` (`0x7a6500`) whole with `FSOUND_SetSFXMasterVolume` (`0x7fdbb6`)
/// stubbed, and compares the level it passes.
#[test]
fn sfx_master_volume() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    d.emu.stub(0x7fdbb6, 0, 1); // FSOUND_SetSFXMasterVolume: stdcall, one argument
    for &v in &synth_f32() {
        d.emu.clear_calls();
        d.emu.call(0x7a6500, 0, 0, &[v.to_bits()]);
        let calls = d.emu.calls();
        assert_eq!(calls.len(), 1, "sfx_master_volume(v={v:?}): one FMOD call");
        assert_eq!(
            calls[0].args[0],
            benilla_re_sound::sfx_master_volume(v),
            "sfx_master_volume(v={v:?}) [{:#010x}]",
            v.to_bits()
        );
    }
}

/// Runs `Sound::FadeIn` (`0x7a5730`) whole with `Sound::UpdateVolume` (`0x7a59e0`) stubbed and the
/// channel's `Sound::IsSuspended` test (`0x7a6720`) true, which skips the list work after the rate is
/// stored. On the audible path it compares the rate at `+0x48` and the clamped level at `+0x44`;
/// below 0.1 seconds it checks that `+0x48` is untouched.
#[test]
fn channel_set_volume() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let chan = d.chan;
    d.emu.write_u32(chan + 0x34, 1); // a stream, so the fade is not skipped
    d.emu.write_u32(chan + 0x88, 4); // Sound::IsSuspended true: skips the calls after the rate
    d.emu.stub(0x7a59e0, 0, 0); // Sound::UpdateVolume
    const SENTINEL: u32 = 0xdead_beef;
    let vols = synth_f32();
    for (k, &level) in synth_i32().iter().enumerate() {
        let vol = vols[k % vols.len()];
        d.put_f32(chan + 0x44, f32::from_bits(SENTINEL));
        d.put_f32(chan + 0x48, f32::from_bits(SENTINEL));
        d.emu
            .call(0x7a5730, chan as u32, 0, &[vol.to_bits(), level as u32]);
        if audible(vol) {
            let rate = d.emu.read_f32(chan + 0x48);
            assert_f32(
                rate,
                benilla_re_sound::channel_set_volume_rate(level, vol),
                &format!("channel_set_volume rate (level={level}, vol={vol:?})"),
            );
            assert_eq!(
                d.emu.read_u32(chan + 0x44),
                level.clamp(0, 255) as u32,
                "channel_set_volume clamped level (level={level})"
            );
        } else {
            assert_eq!(
                d.emu.read_u32(chan + 0x48),
                SENTINEL,
                "channel_set_volume mute path left +0x48 untouched (vol={vol:?})"
            );
        }
    }
}

/// Runs `Sound::FadeOut` (`0x7a5a50`) whole with `OsGetAsyncTimeMs` (`0x42c010`),
/// `Sound::AddToFadeList` (`0x7a5fc0`) and `Sound::Stop3` (`0x7a5950`) stubbed. On the audible path it
/// compares the level at `+0x44`, the rate at `+0x48` and the returned flag; below 0.1 seconds it
/// checks the flag is 0.
#[test]
fn channel_volume_rate_stream() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let chan = d.chan;
    d.emu.write_u32(chan + 0x30, 0); // bound handle (not -1)
    d.emu.write_u32(chan + 0x34, 1); // a stream
    d.emu.stub(0x42c010, 0, 0); // OsGetAsyncTimeMs
    d.emu.stub(0x7a5fc0, 0, 0); // Sound::AddToFadeList
    d.emu.stub(0x7a5950, 0, 0); // Sound::Stop3
    let vols = synth_f32();
    let chvols = synth_f32();
    for (k, &vol) in vols.iter().enumerate() {
        let ch_volume = chvols[(k + 7) % chvols.len()];
        d.put_f32(chan + 0x74, ch_volume);
        let al = d.emu.call(0x7a5a50, chan as u32, 0, &[vol.to_bits()]) & 0xff;
        if audible(vol) {
            assert_eq!(al, 1, "channel_volume_rate_stream audible→1 (vol={vol:?})");
            assert_f32(
                d.emu.read_f32(chan + 0x48),
                benilla_re_sound::channel_volume_rate_stream(0, ch_volume, vol),
                &format!("channel_volume_rate_stream rate (ch={ch_volume:?}, vol={vol:?})"),
            );
            assert_eq!(
                d.emu.read_u32(chan + 0x44),
                benilla_re_sound::channel_volume_curve(0, ch_volume),
                "channel_volume_rate_stream curve (ch={ch_volume:?})"
            );
        } else {
            assert_eq!(al, 0, "channel_volume_rate_stream mute→0 (vol={vol:?})");
        }
    }
}

/// Runs `Sound::SetVolume2` (`0x7a5dc0`) whole with `FSOUND_SetVolume` (`0x7fdb86`) stubbed and the
/// category factors written into `0x87cef4`, `0x87cef8` and `0x87cefc`, over each flag combination
/// that selects a factor. It compares the level passed to FMOD.
#[test]
fn channel_volume_mix() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let chan = d.chan;
    d.emu.write_u32(chan + 0x88, 0); // Sound::IsSuspended false: reaches FMOD
    d.emu.write_u32(chan + 0x30, 0); // bound handle (not -1)
    d.emu.stub(0x7fdb86, 0, 2); // FSOUND_SetVolume(channel, volume)
                                // The three category factors (0x87cef8 music, 0x87cefc ambience, 0x87cef4 sound).
    let (music, ambience, sound) = (0.8f32, 0.5f32, 0.3f32);
    d.put_f32(0x87cef8, music);
    d.put_f32(0x87cefc, ambience);
    d.put_f32(0x87cef4, sound);
    // Values for v and the gain, kept inside the range `__ftol` represents.
    let vals: Vec<f32> = {
        let mut v = vec![
            0.0f32,
            1.0,
            0.5,
            0.25,
            0.999,
            -0.5,
            2.0,
            f32::NAN,
            f32::INFINITY,
        ];
        for i in 0..=64 {
            v.push(i as f32 / 64.0);
        }
        v
    };
    for (k, &flags) in [0x2u32, 0x8, 0x10, 0x0].iter().enumerate() {
        for (j, &v) in vals.iter().enumerate() {
            let atten = vals[(j + k + 3) % vals.len()];
            d.emu.write_u32(chan + 0x38, flags);
            d.put_f32(chan + 0x78, atten);
            d.emu.clear_calls();
            d.emu.call(0x7a5dc0, chan as u32, 0, &[v.to_bits()]);
            let calls = d.emu.calls();
            assert_eq!(
                calls.len(),
                1,
                "channel_volume_mix: one SetVolume (flags={flags:#x})"
            );
            assert_eq!(
                calls[0].args[1],
                benilla_re_sound::channel_volume_mix(flags, v, atten, music, ambience, sound),
                "channel_volume_mix(flags={flags:#x}, v={v:?}, atten={atten:?})"
            );
        }
    }
}

/// Runs `Sound::SetVolume` (`0x7a5da0`) and the start of `Sound::SetVolume2` (`0x7a5dc0`) it calls,
/// with the suspend test true so that `Sound::SetVolume2` returns right after storing the volume.
/// The scaled volume is read at `+0x74`.
#[test]
fn volume_ramp_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let chan = d.chan;
    d.emu.write_u32(chan + 0x88, 4); // Sound::IsSuspended true: Sound::SetVolume2 returns early
    for &stepcount in &synth_i32() {
        d.emu.call(0x7a5da0, chan as u32, 0, &[stepcount as u32]);
        assert_f32(
            d.emu.read_f32(chan + 0x74),
            benilla_re_sound::volume_ramp_scale(stepcount),
            &format!("volume_ramp_scale(stepcount={stepcount})"),
        );
    }
}

/// Runs `Sound::FadeInVolume` (`0x7a57b0`) whole, with `__ftol` (`0x40a2b0`) run for real and
/// `Sound::FadeIn` (`0x7a5730`) stubbed to capture the level, its second argument.
#[test]
fn volume_to_byte() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let chan = d.chan;
    d.emu.stub(0x7a5730, 0, 2); // Sound::FadeIn(seconds, level): the level is args[1]
    for &v in &synth_f32() {
        d.emu.clear_calls();
        // [ebp+8] is the seconds, which only flows to args[0]; [ebp+0xc] is v.
        d.emu.call(0x7a57b0, chan as u32, 0, &[0, v.to_bits()]);
        let calls = d.emu.calls();
        assert_eq!(
            calls.len(),
            1,
            "volume_to_byte(v={v:?}): one channel_set_volume call"
        );
        assert_eq!(
            calls[0].args[1],
            benilla_re_sound::volume_to_byte(v),
            "volume_to_byte(v={v:?}) [{:#010x}]",
            v.to_bits()
        );
    }
}

/// Runs `Sound::ProcessFadeList` (`0x7a4c20`) whole on a one-node list (head `0xcf55e4`, link base
/// `0xcf55dc`) with the clock `0x42c010` fixed, the target level so large the clamp never acts,
/// `Sound::SetVolume` (`0x7a5da0`) stubbed to capture the step, and `Sound::RemoveFromFadeList`
/// (`0x7a6030`) stubbed. The start times run from elapsed 0 to wrapped values, and the rates are
/// positive, which is the fade-in path.
#[test]
fn ramp_step() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let node = d.chan; // aligned, so the list's tag bit is clear
    d.emu.write_u32(0xcf55e4, node as u32); // fade list head
    d.emu.write_u32(0xcf55dc, 0); // link base
    d.emu.write_u32(node + 4, 0); // next is 0: a one-node list
    d.emu.write_u32(node + 0x30, 0); // bound handle (not -1)
    d.emu.write_u32(node + 0x44, 0x7fff_ffff); // target level so large the clamp never acts
    let now: u32 = 0x0100_0000;
    d.emu.stub(0x42c010, now, 0); // OsGetAsyncTimeMs returns a fixed time
    d.emu.stub(0x7a5da0, 0, 1); // Sound::SetVolume: the step is args[0]
    d.emu.stub(0x7a6030, 0, 0); // Sound::RemoveFromFadeList, reached only at the target
    let starts: [u32; 8] = [
        now,                      // elapsed 0
        now.wrapping_sub(1),      // 1
        now.wrapping_sub(1000),   // 1000
        now.wrapping_sub(50_000), // 50000
        1,                        // elapsed now - 1
        0,                        // elapsed now
        now.wrapping_add(1),      // elapsed 0xffffffff
        now.wrapping_add(1000),   // elapsed wraps to a large value
    ];
    let rates: Vec<f32> = synth_f32()
        .into_iter()
        .filter(|r| r.is_finite() && *r > 0.0)
        .collect();
    for &start in &starts {
        for &rate in &rates {
            d.emu.write_u32(node + 0x40, start);
            d.put_f32(node + 0x48, rate);
            d.emu.clear_calls();
            d.emu.call(0x7a4c20, 0, 0, &[]);
            let calls = d.emu.calls();
            let step = calls
                .iter()
                .find(|c| c.addr == 0x7a5da0)
                .unwrap_or_else(|| {
                    panic!("ramp_step(start={start:#x}, rate={rate:?}): 0x7a5da0 not reached")
                })
                .args[0];
            assert_eq!(
                step,
                benilla_re_sound::ramp_step(now, start, rate),
                "ramp_step(now={now:#x}, start={start:#x}, rate={rate:?} [{:#010x}])",
                rate.to_bits()
            );
        }
    }
}

/// Runs `StepSoundDuck` (`0x457960`) whole with `UpdateServerSoundRegistry` (`0x457a50`) stubbed so
/// the direction flag `0xb05f34` holds, `CVar::Lookup` (`0x63de30`) returning a config whose `+0x24` is the
/// SoundVolume, and `Sound::SetSoundVolume` (`0x7a6390`) stubbed to capture the value. Elapsed times
/// straddle both durations, 500 and 3000 ms.
#[test]
fn music_volume_fade() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Snd::new(&wow);
    let cfg = d.emu.alloc(0x40);
    d.emu.stub(0x457a50, 0, 0); // UpdateServerSoundRegistry, which would change 0xb05f34
    d.emu.stub(0x63de30, cfg as u32, 0); // CVar::Lookup returns the config
    d.emu.stub(0x7a6390, 0, 0); // Sound::SetSoundVolume, cdecl: the value is args[0]
    let start: i32 = 100_000; // the start time at 0xb05fc0, nonzero
    let music_vols = [0.0f32, 0.25, 0.5, 1.0, 0.8];
    let config_vols = [1.0f32, 0.6, 0.0, 0.333];
    // Elapsed times either side of both durations, 500 and 3000.
    let offsets = [0i32, 1, 250, 499, 500, 501, 1500, 2999, 3000, 3001, 6000];
    for transitioning in [false, true] {
        d.emu.write_u32(0xb05f34, u32::from(transitioning));
        d.emu.write_u32(0xb05fc0, start as u32);
        for (i, &mv) in music_vols.iter().enumerate() {
            d.emu.write_u32(0xb05f88, mv.to_bits());
            let cv = config_vols[i % config_vols.len()];
            d.emu.write_u32(cfg + 0x24, cv.to_bits());
            for &off in &offsets {
                let now = start.wrapping_add(off);
                d.emu.write_u32(0xb05fc0, start as u32); // written again each case: the snap clears it
                d.emu.clear_calls();
                d.emu.call(0x457960, now as u32, 0, &[]);
                let got = d
                    .emu
                    .calls()
                    .iter()
                    .find(|c| c.addr == 0x7a6390)
                    .map(|c| f32::from_bits(c.args[0]));
                let want = benilla_re_sound::music_volume_fade(transitioning, mv, cv, off);
                let got = got.expect("0x7a6390 called");
                assert_f32(
                    got,
                    want,
                    &format!("music_volume_fade(t={transitioning}, mv={mv}, cv={cv}, off={off})"),
                );
            }
        }
    }
}

/// Runs `GetSoundDefPlayVolume` (`0x458c60`) whole, once with flag `0x800` clear (no draw) and
/// once per chosen draw with `MulHigh32` (`0x455c70`) stubbed to return it. A fresh emulator serves
/// each draw, since a stub with a fixed return can be registered once.
#[test]
fn variation_volume() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let bases = [0.0f32, 0.2, 0.5, 1.0, 2.0, -0.3];
    let params = synth_f32();
    // A fresh emulator per draw: the stub on `0x455c70` returns a fixed value and is registered
    // once, since a second registration stacks hooks. `None` is flag `0x800` clear: no draw.
    let draws: Vec<Option<i32>> = std::iter::once(None)
        .chain([0i32, 15, 16, 30, 100, 255, -5].into_iter().map(Some))
        .collect();
    for draw in draws {
        let mut d = Snd::new(&wow);
        let chan = d.chan;
        d.emu
            .write_u32(chan + 0x7c, if draw.is_some() { 0x800 } else { 0 });
        if let Some(rng) = draw {
            d.emu.stub(0x455c70, rng as u32, 0); // MulHigh32 returns the chosen draw
        }
        for (k, &base) in bases.iter().enumerate() {
            d.put_f32(chan + 0x78, base);
            let param2 = params[k % params.len()];
            let got = d
                .emu
                .call_f32(0x458c60, chan as u32, 0, &[param2.to_bits()]);
            let want = benilla_re_sound::variation_volume(draw, base, param2);
            assert_f32(
                got,
                want,
                &format!("variation_volume(draw={draw:?}, base={base}, p2={param2:?})"),
            );
        }
    }
}

/// Runs `ApplySoundDefVolumePitch` (`0x458da0`) whole with flag `0x400` set, `MulHigh32` (`0x455c70`)
/// stubbed to a chosen draw, `Sound::SetVolume2` (`0x7a5dc0`) stubbed away, and `Sound::SetFrequency`
/// (`0x7a5e40`) stubbed to capture the frequency. A fresh emulator serves each draw.
#[test]
fn variation_pitch() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let draws = [0i32, 1, 15, 85, 100, 200, 255, 1000, -85, -200];
    // A fresh emulator per draw, as in `variation_volume`.
    for &rng in &draws {
        let mut d = Snd::new(&wow);
        let chan = d.chan;
        d.emu.write_u32(chan + 0x7c, 0x400); // pitch on, 0x800 clear: the volume takes no draw
        d.emu.stub(0x7a5dc0, 0, 1); // Sound::SetVolume2
        d.emu.stub(0x7a5e40, 0, 1); // Sound::SetFrequency: the frequency is args[0]
        d.emu.stub(0x455c70, rng as u32, 0); // MulHigh32 returns the chosen draw
        d.emu
            .call(0x458da0, chan as u32, 0, &[chan as u32, chan as u32]);
        let freq = d
            .emu
            .calls()
            .iter()
            .find(|c| c.addr == 0x7a5e40)
            .map(|c| c.args[0]);
        assert_eq!(
            freq,
            Some(benilla_re_sound::variation_pitch_freq(rng) as u32),
            "variation_pitch(rng={rng})"
        );
    }
}

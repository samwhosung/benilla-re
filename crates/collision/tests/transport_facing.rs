//! The transport facing arithmetic. `CMovement::TransportFacingToWorld` (`0x7c4ae0`),
//! `CMovement::WorldFacingToTransport` (`0x7c4b30`) and `CMovement::SetWorldFacing` (`0x7c6eb0`,
//! with `CMovement::SetFacing` stubbed) run whole; the rest run as slices: `0x7c4c2e` to `0x7c4c7c`
//! of `0x7c4b80`, `0x7c538a` to `0x7c5396` of `0x7c5360`, `0x7c6540` to `0x7c6577` of `0x7c6520`,
//! `0x7c684f` to `0x7c6861` of `0x7c6840`, `0x7c6977` to `0x7c6990` of `0x7c6900`, `0x7c6f36` to
//! `0x7c6f49` of `0x7c6f30` and `0x7c6f76` to `0x7c6f8c` of `0x7c6f70`, the last two compared by
//! `ah`. `GetTransportFacing` (`0x630b70`) is replaced by a load of the chosen facing.

use benilla_re_collision::transport_facing::*;
use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

/// `[0xcf5d00]`, zero in the image until its static initializer stores 2π.
const TWO_PI_GLOBAL: u64 = 0x00cf_5d00;
const TWO_PI_BITS: u32 = 0x40c9_0fdb;

const ZERO_GLOBAL: u64 = 0x007f_fd74;

/// `GetTransportFacing`, which returns the transport's facing in `st0`.
const FACING_FN: u64 = 0x630b70;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// Asserts the same bits, or NaN on both sides.
fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// `ah` after `fnstsw ax`: C0 is bit 0, C2 bit 2, C3 bit 6.
fn ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// Replaces `0x630b70` with `fld dword [slot]; ret 8` and returns the slot.
fn install_facing_stub(e: &mut Emu) -> u64 {
    let slot = e.alloc(0x10);

    let mut bytes = vec![0xd9u8, 0x05];
    bytes.extend_from_slice(&(slot as u32).to_le_bytes());
    bytes.extend_from_slice(&[0xc2, 0x08, 0x00]);
    e.write(FACING_FN, &bytes);
    slot
}

/// Writes 2π to `[0xcf5d00]`, as its static initializer does.
fn install_wrap_globals(e: &mut Emu) {
    e.write_u32(TWO_PI_GLOBAL, TWO_PI_BITS);
    e.write_u32(ZERO_GLOBAL, 0);
}

const TWO_PI_F: f32 = 6.2831855;
/// Values at and one ulp around 0 and 2π, and NaN.
fn wrap_probe_values() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        1.0,
        std::f32::consts::PI,
        TWO_PI_F,
        TWO_PI_F - 1e-4,
        TWO_PI_F + 1e-4,
        6.2831855 * 1.5,
        -1e-4,
        -1.0,
        -std::f32::consts::PI,
        f32::from_bits(0x7fc0_0000),
    ]
}

/// `0x7c4ae0`, with and without a transport.
#[test]
fn tf_facing_to_transport_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let slot = install_facing_stub(&mut e);
    install_wrap_globals(&mut e);
    let this = e.alloc(0x80);
    let mut s = 0x7c4a_e000u32;
    let mut seen_pass = false;
    let mut seen_wrap = false;

    let mut case = |e: &mut Emu, facing: f32, arg: f32, has_transport: bool, ctx: &str| {
        e.write_u32(slot, facing.to_bits());

        e.write_u32(this + 0x38, if has_transport { 1 } else { 0 });
        e.write_u32(this + 0x3c, 0);
        let bin = e.call_f32(0x7c4ae0, this as u32, 0, &[arg.to_bits()]);
        let ours = facing_to_transport(facing, arg, has_transport);
        assert_f32(bin, ours, ctx);
        if has_transport {
            seen_wrap = true;
        } else {
            seen_pass = true;
        }
    };

    case(&mut e, 9.9, 1.234, false, "no-transport passthrough");
    case(&mut e, 9.9, -5.0, false, "no-transport passthrough neg");

    for &facing in &[0.0f32, 1.0, 3.0, TWO_PI_F, -1.0] {
        for &arg in &wrap_probe_values() {
            case(
                &mut e,
                facing,
                arg,
                true,
                &format!("wrap f={facing} a={arg}"),
            );
        }
    }

    case(
        &mut e,
        TWO_PI_F,
        0.0,
        true,
        "facing+arg == 2π exactly → kept",
    );
    case(&mut e, 0.0, TWO_PI_F, true, "arg == 2π, facing 0 → kept");
    for i in 0..4000u32 {
        let has = !(next(&mut s) >> 8).is_multiple_of(4);
        let facing = match i % 3 {
            0 => frand(&mut s, -TWO_PI_F, TWO_PI_F),
            1 => frand(&mut s, 0.0, TWO_PI_F),
            _ => frand_bits(&mut s),
        };
        let arg = match i % 3 {
            0 => frand(&mut s, -TWO_PI_F, TWO_PI_F),
            1 => TWO_PI_F - facing + frand(&mut s, -0.01, 0.01),
            _ => frand(&mut s, -2.0 * TWO_PI_F, 2.0 * TWO_PI_F),
        };
        case(&mut e, facing, arg, has, &format!("fuzz{i}"));
    }
    assert!(seen_pass, "no-transport passthrough never seen");
    assert!(seen_wrap, "transport wrap never seen");
}

/// `0x7c4b30`, with and without a transport.
#[test]
fn tf_facing_from_transport_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let slot = install_facing_stub(&mut e);
    install_wrap_globals(&mut e);
    let this = e.alloc(0x80);
    let mut s = 0x7c4b_3000u32;
    let mut seen_pass = false;
    let mut seen_wrap = false;

    let mut case = |e: &mut Emu, facing: f32, arg: f32, has_transport: bool, ctx: &str| {
        e.write_u32(slot, facing.to_bits());
        e.write_u32(this + 0x38, if has_transport { 1 } else { 0 });
        e.write_u32(this + 0x3c, 0);
        let bin = e.call_f32(0x7c4b30, this as u32, 0, &[arg.to_bits()]);
        let ours = facing_from_transport(facing, arg, has_transport);
        assert_f32(bin, ours, ctx);
        if has_transport {
            seen_wrap = true;
        } else {
            seen_pass = true;
        }
    };

    case(&mut e, 4.0, 2.0, false, "no-transport passthrough");
    for &facing in &[0.0f32, 1.0, 3.0, TWO_PI_F, -1.0] {
        for &arg in &wrap_probe_values() {
            case(
                &mut e,
                facing,
                arg,
                true,
                &format!("wrap f={facing} a={arg}"),
            );
        }
    }

    case(&mut e, 0.0, TWO_PI_F, true, "arg−facing == 2π → kept");
    case(&mut e, 1.0, 1.0, true, "arg−facing == 0 → kept");
    case(&mut e, 5.0, 1.0, true, "arg−facing == −4 < 0 → +2π");
    for i in 0..4000u32 {
        let has = !(next(&mut s) >> 8).is_multiple_of(4);
        let facing = frand(&mut s, -TWO_PI_F, TWO_PI_F);
        let arg = match i % 3 {
            0 => frand(&mut s, -TWO_PI_F, TWO_PI_F),
            1 => facing + frand(&mut s, -0.01, 0.01),
            _ => facing + TWO_PI_F + frand(&mut s, -0.01, 0.01),
        };
        case(&mut e, facing, arg, has, &format!("fuzz{i}"));
    }
    assert!(seen_pass, "no-transport passthrough never seen");
    assert!(seen_wrap, "transport wrap never seen");
}

/// `0x7c4c2e` to `0x7c4c7c`: the three products, the matrix at `[ebp-0x40]`.
#[test]
fn tf_dir_to_transport_rotation_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let this = e.alloc(0x40);
    let out = e.alloc(0x10);
    let mut s = 0x7c4b_8000u32;

    let case = |e: &mut Emu, m: [f32; 16], v: [f32; 3], ctx: &str| {
        for (i, &mi) in m.iter().enumerate() {
            e.write_u32(ebp - 0x40 + 4 * i as u64, mi.to_bits());
        }
        e.write_u32(this + 0x24, v[0].to_bits());
        e.write_u32(this + 0x28, v[1].to_bits());
        e.write_u32(this + 0x2c, v[2].to_bits());
        e.write_u32(ebp + 8, out as u32);
        e.set_reg(RegisterX86::EAX, out);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7c4c2e, 0x7c4c7c);
        let bin = [e.read_f32(out), e.read_f32(out + 4), e.read_f32(out + 8)];
        let ours = dir_to_transport_rotation(m, v);
        assert_f32(bin[0], ours[0], &format!("{ctx} out0"));
        assert_f32(bin[1], ours[1], &format!("{ctx} out1"));
        assert_f32(bin[2], ours[2], &format!("{ctx} out2"));
    };

    let mut ident = [0.0f32; 16];
    ident[0] = 1.0;
    ident[5] = 1.0;
    ident[10] = 1.0;
    case(&mut e, ident, [1.0, 2.0, 3.0], "identity");
    case(&mut e, ident, [-4.0, 5.5, -6.25], "identity neg");

    let dense: [f32; 16] = [
        0.36, 0.0, 0.48, 0.0, -0.8, 0.0, 0.6, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    case(&mut e, dense, [1.0, 2.0, 3.0], "dense");
    for i in 0..3000u32 {
        let mut m = [0.0f32; 16];
        for mi in m.iter_mut() {
            *mi = match i % 4 {
                0 => frand(&mut s, -1.0, 1.0),
                1 => frand(&mut s, -100.0, 100.0),
                _ => frand_bits(&mut s),
            };
        }
        let v = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        case(&mut e, m, v, &format!("fuzz{i}"));
    }
}

/// `0x7c538a` to `0x7c5396`: the step in seconds.
#[test]
fn tf_substep_dt_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x7c53_6000u32;

    let case = |e: &mut Emu, ms: u32, ctx: &str| {
        e.write_u32(ebp - 0x8, ms);
        e.write_u32(ebp - 0x4, 0);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7c538a, 0x7c5396);
        let bin = e.read_f32(ebp + 8);

        let ours = substep_dt(ms);
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 0, "ms 0");
    case(&mut e, 1, "ms 1");
    case(&mut e, 16, "ms 16 (60fps)");
    case(&mut e, 33, "ms 33 (30fps)");
    case(&mut e, 1000, "ms 1000");
    case(&mut e, 16_777_217, "ms beyond f32 mantissa");
    case(&mut e, 100_000_000, "ms large");
    case(
        &mut e,
        0xffff_ffff,
        "ms 0xffffffff (high count, not negative)",
    );
    for i in 0..3000u32 {
        let ms = match i % 3 {
            0 => (next(&mut s) >> 8) % 100,
            1 => (next(&mut s) >> 8) % 100_000,
            _ => next(&mut s) >> 8,
        };
        case(&mut e, ms, &format!("fuzz{i}"));
    }
}

/// `0x7c6540` to `0x7c6577`: `+0x14`.
#[test]
fn tf_spline_facing_accumulate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    install_wrap_globals(&mut e);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let ebx = e.alloc(0x100);
    let store = e.alloc(0x40);
    let mut s = 0x7c65_2000u32;
    let mut seen_sub = false;
    let mut seen_add = false;
    let mut seen_keep = false;

    let mut case = |e: &mut Emu, arg: f32, facing: f32, ctx: &str| {
        e.write_u32(ebp + 0xc, arg.to_bits());
        e.write_u32(store + 0x14, facing.to_bits());
        e.write_u32(ebx + 0xa4, store as u32);
        e.set_reg(RegisterX86::EBX, ebx);
        e.set_reg(RegisterX86::ECX, store);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7c6540, 0x7c6577);
        let bin = e.read_f32(store + 0x14);
        let ours = spline_facing_accumulate(arg, facing);
        assert_f32(bin, ours, ctx);
        let v = f64::from(arg) + f64::from(facing);
        if v > f64::from(TWO_PI_F) {
            seen_sub = true;
        } else if v < 0.0 {
            seen_add = true;
        } else {
            seen_keep = true;
        }
    };

    for &facing in &[0.0f32, 1.0, 3.0, TWO_PI_F, -1.0] {
        for &arg in &wrap_probe_values() {
            case(&mut e, arg, facing, &format!("wrap a={arg} f={facing}"));
        }
    }

    case(&mut e, TWO_PI_F, 0.0, "arg+facing == 2π → keep");
    case(&mut e, 0.0, 0.0, "0 → keep");
    case(&mut e, -0.5, -0.5, "−1 < 0 → +2π");
    case(&mut e, 4.0, 4.0, "8 > 2π → −2π");
    for i in 0..4000u32 {
        let facing = frand(&mut s, 0.0, TWO_PI_F);
        let arg = match i % 4 {
            0 => frand(&mut s, -TWO_PI_F, TWO_PI_F),
            1 => TWO_PI_F - facing + frand(&mut s, -0.01, 0.01),
            2 => -facing + frand(&mut s, -0.01, 0.01),
            _ => frand_bits(&mut s),
        };
        case(&mut e, arg, facing, &format!("fuzz{i}"));
    }
    assert!(seen_sub && seen_add && seen_keep, "all wrap arms exercised");
}

/// `0x7c684f` to `0x7c6861`: the negated facing.
#[test]
fn tf_negate_transport_facing_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let slot = install_facing_stub(&mut e);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x7c68_4000u32;

    let case = |e: &mut Emu, facing: f32, ctx: &str| {
        e.write_u32(slot, facing.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);

        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EDI, 0);
        e.set_reg(RegisterX86::EBX, 0);
        e.run(0x7c684f, 0x7c6861);
        let bin = e.read_f32(ebp - 0x4);
        let ours = negate_transport_facing(facing);
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 0.0, "−0");
    case(&mut e, 1.0, "−1");
    case(&mut e, -2.5, "−(−2.5) = 2.5");
    case(&mut e, std::f32::consts::PI, "−π");
    case(&mut e, f32::from_bits(0x7fc0_0000), "−NaN (sign flip)");
    for i in 0..3000u32 {
        let facing = match i % 2 {
            0 => frand(&mut s, -100.0, 100.0),
            _ => frand_bits(&mut s),
        };
        case(&mut e, facing, &format!("fuzz{i}"));
    }
}

/// `0x7c6977` to `0x7c6990`: `+0x80`.
#[test]
fn tf_setfacingto_residual_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x100);
    let target = e.alloc(0x40);
    let mut s = 0x7c69_0000u32;

    let case = |e: &mut Emu, facing_acc: f32, a: f32, b: f32, ctx: &str| {
        e.write_u32(this + 0x80, facing_acc.to_bits());
        e.write_u32(target + 0x1c, a.to_bits());
        e.write_u32(target + 0x2c, b.to_bits());
        e.write_u32(ebp - 0x8, 0);
        e.write_u32(ebp - 0x4, 0);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, target);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7c6977, 0x7c6990);
        let bin = e.read_f32(this + 0x80);
        let ours = setfacingto_residual(facing_acc, a, b);
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 5.0, 1.0, 2.0, "5 − (2−1) = 4");
    case(&mut e, 0.0, 0.0, 0.0, "0");
    case(&mut e, -3.0, 4.0, -2.0, "−3 − (−2−4) = 3");

    case(
        &mut e,
        163200.33,
        -638547.25,
        632_252.7,
        "live delta divergence",
    );
    case(&mut e, 0.0, 0.5, 16777216.0, "large delta f32-gap");
    for i in 0..4000u32 {
        let facing_acc = frand(&mut s, -100.0, 100.0);
        let a = match i % 3 {
            0 => frand(&mut s, -10.0, 10.0),
            1 => frand(&mut s, -1e6, 1e6),
            _ => frand_bits(&mut s),
        };
        let b = match i % 3 {
            0 => frand(&mut s, -10.0, 10.0),
            1 => frand(&mut s, -1e6, 1e6),
            _ => frand_bits(&mut s),
        };
        case(&mut e, facing_acc, a, b, &format!("fuzz{i}"));
    }
}

/// `0x7c6eb0`: the facing passed to `0x7c6f30`.
#[test]
fn tf_setfacing_to_world_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let slot = install_facing_stub(&mut e);
    install_wrap_globals(&mut e);

    e.stub(0x7c6f30, 0, 1);
    let this = e.alloc(0x80);
    let mut s = 0x7c6e_b000u32;
    let mut seen_pass = false;
    let mut seen_wrap = false;

    let mut case = |e: &mut Emu, facing: f32, arg: f32, has_transport: bool, ctx: &str| {
        e.write_u32(slot, facing.to_bits());
        e.write_u32(this + 0x38, if has_transport { 1 } else { 0 });
        e.write_u32(this + 0x3c, 0);
        e.clear_calls();
        e.call(0x7c6eb0, this as u32, 0, &[arg.to_bits()]);
        let rec = e
            .calls()
            .iter()
            .rev()
            .find(|c| c.addr == 0x7c6f30)
            .expect("0x7c6f30 SetFacing-core tail call not recorded");
        let bin = f32::from_bits(rec.args[0]);

        let eff_facing = if has_transport { facing } else { 0.0 };
        let ours = setfacing_to_world(eff_facing, arg);
        assert_f32(bin, ours, ctx);
        if has_transport {
            seen_wrap = true;
        } else {
            seen_pass = true;
        }
    };

    case(&mut e, 0.0, 1.0, false, "no-transport wrap(arg)");
    case(&mut e, 99.0, -1.0, false, "no-transport wrap(arg) neg");
    for &facing in &[0.0f32, 1.0, 3.0, TWO_PI_F, -1.0] {
        for &arg in &wrap_probe_values() {
            case(
                &mut e,
                facing,
                arg,
                true,
                &format!("wrap f={facing} a={arg}"),
            );
        }
    }
    case(&mut e, 0.0, TWO_PI_F, true, "arg−facing == 2π → keep");
    case(&mut e, 1.0, 1.0, true, "arg−facing == 0 → keep");
    case(&mut e, 5.0, 1.0, true, "arg−facing == −4 < 0 → +2π");
    for i in 0..4000u32 {
        let has = !(next(&mut s) >> 8).is_multiple_of(4);
        let facing = frand(&mut s, -TWO_PI_F, TWO_PI_F);
        let arg = match i % 3 {
            0 => frand(&mut s, -TWO_PI_F, TWO_PI_F),
            1 => facing + frand(&mut s, -0.01, 0.01),
            _ => facing + TWO_PI_F + frand(&mut s, -0.01, 0.01),
        };
        case(&mut e, facing, arg, has, &format!("fuzz{i}"));
    }
    assert!(seen_pass, "no-transport wrap never seen");
    assert!(seen_wrap, "transport wrap never seen");
}

/// `0x7c6f36` to `0x7c6f49`: `ah` against `setfacing_should_store`.
#[test]
fn tf_setfacing_should_store_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x40);
    let mut s = 0x7c6f_3000u32;
    let mut seen_store = false;
    let mut seen_skip = false;
    let eps = f32::from_bits(0x3580_0000);

    let mut case = |e: &mut Emu, current: f32, new: f32, ctx: &str| {
        e.write_u32(this + 0x1c, current.to_bits());
        e.write_u32(ebp + 8, new.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);

        e.run(0x7c6f36, 0x7c6f49);
        let m = ah(e) & 0x5;

        let bits = (m & 0x01).count_ones() + (m & 0x04).count_ones();
        let bin_store = bits.is_multiple_of(2);
        if bin_store {
            seen_store = true;
        } else {
            seen_skip = true;
        }
        assert_eq!(
            bin_store,
            setfacing_should_store(current, new),
            "{ctx}: bin_store={bin_store} ah&5={m:#04x} cur={current} new={new}"
        );
    };

    case(&mut e, 1.0, 2.0, "|1| >= eps → store");
    case(&mut e, 1.0, 1.0, "|0| < eps → skip");

    case(&mut e, 0.0, eps, "|diff| == eps exactly → store");
    case(&mut e, 0.0, eps * 0.5, "|diff| < eps → skip");
    case(
        &mut e,
        f32::from_bits(0x7fc0_0000),
        1.0,
        "NaN current → unordered → store",
    );
    case(&mut e, 1.0, f32::from_bits(0x7fc0_0000), "NaN new → store");
    for i in 0..5000u32 {
        let current = frand(&mut s, -10.0, 10.0);
        let new = match i % 4 {
            0 => current + frand(&mut s, -2.0 * 9.5e-7, 2.0 * 9.5e-7),
            1 => current,
            2 => current + frand(&mut s, -5.0, 5.0),
            _ => frand_bits(&mut s),
        };
        case(&mut e, current, new, &format!("fuzz{i}"));
    }
    assert!(seen_store, "store arm never seen");
    assert!(seen_skip, "skip arm never seen");
}

/// `0x7c6f76` to `0x7c6f8c`: `ah` against `setpitch_should_store`.
#[test]
fn tf_setpitch_should_store_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x40);
    let mut s = 0x7c6f_7000u32;
    let mut seen_store = false;
    let mut seen_skip = false;
    let eps = f32::from_bits(0x3580_0000);

    let mut case = |e: &mut Emu, current: f32, new: f32, ctx: &str| {
        e.write_u32(this + 0x20, current.to_bits());
        e.write_u32(ebp + 8, new.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EDI, 0);

        e.run(0x7c6f76, 0x7c6f8c);
        let m = ah(e) & 0x5;
        let bits = (m & 0x01).count_ones() + (m & 0x04).count_ones();
        let bin_store = bits.is_multiple_of(2);
        if bin_store {
            seen_store = true;
        } else {
            seen_skip = true;
        }
        assert_eq!(
            bin_store,
            setpitch_should_store(current, new),
            "{ctx}: bin_store={bin_store} ah&5={m:#04x} cur={current} new={new}"
        );
    };

    case(&mut e, 0.5, 0.9, "|0.4| >= eps → store");
    case(&mut e, 0.5, 0.5, "|0| < eps → skip");
    case(&mut e, 0.0, eps, "|diff| == eps exactly → store");
    case(&mut e, 0.0, eps * 0.5, "|diff| < eps → skip");
    case(
        &mut e,
        f32::from_bits(0x7fc0_0000),
        0.0,
        "NaN current → store",
    );
    for i in 0..5000u32 {
        let current = frand(&mut s, -1.6, 1.6);
        let new = match i % 4 {
            0 => current + frand(&mut s, -2.0 * 9.5e-7, 2.0 * 9.5e-7),
            1 => current,
            2 => current + frand(&mut s, -3.0, 3.0),
            _ => frand_bits(&mut s),
        };
        case(&mut e, current, new, &format!("fuzz{i}"));
    }
    assert!(seen_store, "store arm never seen");
    assert!(seen_skip, "skip arm never seen");
}

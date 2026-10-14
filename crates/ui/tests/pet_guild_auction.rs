//! The pet, auction, guild and aura scripts with arithmetic of their own (`GetPetHappiness`,
//! `CancelAuction`, `GetGuildRosterLastOnline`, `GetPlayerBuffTimeLeft`) against the exe: each is run
//! as the slice of instructions that computes the value, with `lua_pushnumber` stubbed.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::time_values::ms_to_seconds;
use benilla_re_ui::unit_values::{
    cancel_auction_min_increment, guild_last_online, pet_happiness_scale,
};
use fpkit::{next, rngf};

const LUA_PUSHNUMBER: u64 = 0x6f3810; // ret 8 -> pop_args = 2

struct Frame {
    ebp: u64,
    esp: u64,
    scratch: u64, // a mapped struct pointer (the record a slice reads through edi/eax)
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x400);
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    let ebp = stack + 0xe00; // ebp below esp's reads; [ebp-..] locals land in the stack region
    Frame { ebp, esp, scratch }
}

/// Row 0x4be900 GetPetHappiness — slice 0x4be9ae..0x4be9be:
///   fld dword [edi+4*esi+0x30]; sub esp,8; fmul dword [0x806b10]=100.0; fstp qword [esp].
/// fmul @0x4be9b5 (×1) — the only arithmetic operation in [0x4be900, 0x4bea0b). The product is the
/// un-narrowed f64. We feed the stat f32 at [edi+0x30] (esi=0), read the f64 the `fstp qword` lands
/// at [esp_initial - 8] (the slice's `sub esp,8` runs before the fstp). Halt at the `call 0x6f3810`.
#[test]
fn luabind_4be900_pet_happiness() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, field: f32| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EDI, f.scratch);
        emu.set_reg(RegisterX86::ESI, 0); // index 0 -> [edi+0x30]
        emu.write_u32(f.scratch + 0x30, field.to_bits());
        emu.run(0x4be9ae, 0x4be9be);
        let bytes = emu.read(f.esp - 0x8, 8);
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    // the happiness stat fields are small non-negative floats; spread includes 0, the canonical
    // 1.0/0.5, larger, and negative (no sign restriction in the arithmetic).
    let fields = [
        0.0f32,
        1.0,
        0.5,
        0.333_333_3,
        2.5,
        100.0,
        1.0e6,
        -1.0,
        -0.25,
    ];
    for &field in &fields {
        assert_eq!(
            run(&mut emu, field).to_bits(),
            pet_happiness_scale(field).to_bits(),
            "pet_happiness_scale field={field}"
        );
        total += 1;
    }
    let mut s = 0x4be9_0000u32;
    for _ in 0..5000 {
        let field = rngf(&mut s, -10.0, 10.0) as f32;
        assert_eq!(
            run(&mut emu, field).to_bits(),
            pet_happiness_scale(field).to_bits(),
            "pet_happiness_scale fuzz field={field}"
        );
        total += 1;
    }
    eprintln!("luabind_4be900_pet_happiness: {total} bit-exact (fld dword; fmul dword [100.0]; fstp qword)");
}

/// Row 0x4d0260 CancelAuction — slice 0x4d0338..0x4d0346:
///   fild qword [ebp-8]; fmul dword [0x807024]=0.05; call __ftol (0x40a2b0).
/// fmul @0x4d033b (×1) — the only arithmetic operation in [0x4d0260, 0x4d03d5). The real CRT __ftol
/// runs natively (truncate toward zero); read EAX. We feed the bid as the i64 at [ebp-8]. Halt at
/// the `cmp ebx,eax` (0x4d0346) __ftol returns to.
#[test]
fn luabind_4d0260_cancel_auction_min_increment() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, bid: i64| -> i32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        // [ebp-8] = bid (i64). The binary builds it as [edi+0x40] (i32) with the high dword 0; we
        // write the full i64 to be faithful to the qword fild.
        emu.write(f.ebp - 0x8, &bid.to_le_bytes());
        emu.run(0x4d0338, 0x4d0346);
        emu.get_reg(RegisterX86::EAX) as i32
    };

    let mut total = 0u32;
    // bids spanning the gold range; truncate pivots where bid·0.05 sits just below/at an integer.
    let bids = [
        0i64,
        1,
        19,
        20,
        21,
        100,
        999,
        1000,
        10_000,
        1_234_567,
        2_000_000_000,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
    ];
    for &bid in &bids {
        assert_eq!(
            run(&mut emu, bid),
            cancel_auction_min_increment(bid),
            "cancel_auction_min_increment bid={bid}"
        );
        total += 1;
    }
    let mut s = 0x4d02_6000u32;
    for _ in 0..5000 {
        let bid = i64::from(next(&mut s) >> 8); // non-negative copper bid
        assert_eq!(
            run(&mut emu, bid),
            cancel_auction_min_increment(bid),
            "cancel_auction_min_increment fuzz bid={bid}"
        );
        total += 1;
    }
    eprintln!("luabind_4d0260_cancel_auction_min_increment: {total} bit-exact (fild qword; fmul dword [0.05]; __ftol)");
}

/// Row 0x4d14a0 GetGuildRosterLastOnline — slice 0x4d1508..0x4d15a8 (whole owned breakdown):
///   fld [eax+0x150]=t; t·(1/365)→years(__ftol); rem=t-years·365; rem·(1/30)→months(__ftol);
///   days_rem=rem-months·30 (fstp dword f32); trunc→days(__ftol); (days_rem-days)·24→hours(__ftol).
/// arithmetic operations ×8: fmul@0x4d1514, fmul@0x4d1521, fsubp@0x4d1527, fmul@0x4d152f, fmul@0x4d153c,
/// fsubr@0x4d1542, fsubr@0x4d1587, fmul@0x4d158a — all in [0x4d14a0, 0x4d15b1). Three
/// native __ftol (0x40a2b0); the three lua_pushnumber (0x6f3810, ret 8) are STUBBED. Read the four
/// ints: years [ebp-4], months [ebp-8], days [ebp-0xc], hours [ebp-0x10] (final).
#[test]
fn luabind_4d14a0_guild_last_online() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    emu.stub(LUA_PUSHNUMBER, 0, 2); // ret 8 -> pop the 8-byte double arg

    let run = |emu: &mut Emu, t: f32| -> (i32, i32, i32, i32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EAX, f.scratch); // the looked-up record pointer
        emu.set_reg(RegisterX86::ESI, f.scratch); // ecx=esi=this for the push glue
        emu.write_u32(f.scratch + 0x150, t.to_bits()); // [eax+0x150] = LastOnline float
        emu.run(0x4d1508, 0x4d15a8);
        let years = emu.read_u32(f.ebp - 0x4) as i32;
        let months = emu.read_u32(f.ebp - 0x8) as i32;
        let days = emu.read_u32(f.ebp - 0xc) as i32;
        let hours = emu.read_u32(f.ebp - 0x10) as i32;
        (years, months, days, hours)
    };

    let mut total = 0u32;
    // Zero, the exact year and month boundaries, a few floats just below a month boundary (where
    // the reciprocal of 30 at 0x807334 decides the month: 0x3d088889), assorted LastOnline floats,
    // and a couple of negatives, as the arithmetic does not look at the sign.
    let probes = [
        0.0f32, 1.0, 29.0, 30.0, 30.5, 31.0, 364.0, 365.0, 366.0, 730.0, 100.25, 12345.0, 99999.0,
        0.5, -1.0, -400.0, 59.999, 89.999, 119.999, 179.995, 179.999, 209.999,
    ];
    for &t in &probes {
        assert_eq!(
            run(&mut emu, t),
            guild_last_online(t),
            "guild_last_online t={t}"
        );
        total += 1;
    }
    let mut s = 0x4d14_a000u32;
    for _ in 0..5000 {
        let t = rngf(&mut s, 0.0, 4000.0) as f32; // realistic days-offline magnitudes
        assert_eq!(
            run(&mut emu, t),
            guild_last_online(t),
            "guild_last_online fuzz t={t}"
        );
        total += 1;
    }
    // a second sweep over a wider range incl. negatives, to exercise __ftol toward-zero on both signs
    let mut s2 = 0x0badf00du32;
    for _ in 0..5000 {
        let t = rngf(&mut s2, -2000.0, 50000.0) as f32;
        assert_eq!(
            run(&mut emu, t),
            guild_last_online(t),
            "guild_last_online fuzz2 t={t}"
        );
        total += 1;
    }
    eprintln!("luabind_4d14a0_guild_last_online: {total} bit-exact (8 arithmetic operations; 3× __ftol; pushnumber stubbed)");
}

/// Row 0x4e4930 GetPlayerBuffTimeLeft — slice 0x4e497e..0x4e498c:
///   fild qword [ebp-8]; sub esp,8; fmul qword [0x801608]=0.001(f64); fstp qword [esp].
/// fmul @0x4e4986 (×1) — the only arithmetic operation in [0x4e4930, 0x4e499d). The product (ms→s)
/// keeps full f64. We feed the ms count as the i64 at [ebp-8], read the f64 the `fstp qword` lands at
/// [esp_initial - 8]. Halt at the `call 0x6f3810`.
#[test]
fn luabind_4e4930_buff_time_left() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, ms: i64| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write(f.ebp - 0x8, &ms.to_le_bytes());
        emu.run(0x4e497e, 0x4e498f); // halt at the `call 0x6f3810`, after the fstp stores the f64
        let bytes = emu.read(f.esp - 0x8, 8);
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    // ms-remaining counts: zero, sub-second, seconds, minutes, an hour, and the i32 ceiling.
    let mss = [
        0i64,
        1,
        500,
        999,
        1000,
        1500,
        60_000,
        3_600_000,
        2_000_000_000,
    ];
    for &ms in &mss {
        assert_eq!(
            run(&mut emu, ms).to_bits(),
            ms_to_seconds(ms).to_bits(),
            "ms_to_seconds ms={ms}"
        );
        total += 1;
    }
    let mut s = 0x4e49_3000u32;
    for _ in 0..5000 {
        let ms = i64::from(next(&mut s) >> 8); // non-negative ms remaining
        assert_eq!(
            run(&mut emu, ms).to_bits(),
            ms_to_seconds(ms).to_bits(),
            "ms_to_seconds fuzz ms={ms}"
        );
        total += 1;
    }
    eprintln!("luabind_4e4930_buff_time_left: {total} bit-exact (fild qword; fmul qword [0.001 f64]; fstp qword)");
}

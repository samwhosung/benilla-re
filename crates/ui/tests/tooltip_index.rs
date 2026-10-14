//! The tooltip scripts that take a 1-based index (`SetSpell`, `SetLootItem`, `SetMerchantItem`,
//! `SetTradePlayerItem`, `SetTradeTargetItem`, `SetMerchantCompareItem`) against the exe: each
//! subtracts the f64 1.0 at `0x8015b8` from the Lua number before `__ftol` truncates it. The test
//! first checks that every block's `fsub` has the bytes of the shared subtraction.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::tooltip::rebase_index;
use fpkit::next;

const ONE_FSUB_BYTES: [u8; 6] = [0xdc, 0x25, 0xb8, 0x15, 0x80, 0x00]; // fsub qword [0x8015b8]=1.0

// deterministic lua_tonumber args: integer 1-based indices (1..), the 0 / fractional / negative /
// huge cases the binary would feed __ftol, and the exact-1.0 boundary (re-base → 0.0).
const ARGS: [f64; 16] = [
    1.0, // index 1 → 0
    2.0, // index 2 → 1
    5.0,
    18.0, // SetMerchantCompareItem inner-loop bound
    100.0,
    1024.0, // SetSpell spell-id range upper bound (0x400)
    0.0,    // → -1.0 (rejected downstream as <0, but the FP is still computed)
    1.5,    // fractional → 0.5 (truncated downstream)
    2.5,
    0.9999999999,
    -1.0, // → -2.0
    -0.5,
    1e15,               // large but exactly representable
    1e16,               // beyond f64 integer-exactness onset region
    7.0,                // SetTradePlayerItem table bound
    1.0000000000000002, // 1 + 1 ulp → tiny positive
];

/// A synthesized stack frame: `(ebp, esp)` with room for the `call`/`ret` pushes the slice does.
struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80;
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000;
    Frame { ebp, esp }
}

/// Diff one `luabind_532` row's owned `fsub` (× −1.0 re-base). `slice` = `(start, halt)` where
/// `start` is the `call 0x6f3620` (lua_tonumber, patched to load our arg LIVE in st0) and `halt` is
/// the instruction after `call 0x40a2b0` (__ftol, patched to store st0 → f64 out-cell). `fsub_va` is
/// the owned `fsub` site, byte-verified == shared kernel before the diff (else circular). Diffs the
/// f64 `arg − 1.0` over `ARGS` + 4000 fuzz cases. Returns the assertion count.
fn diff_row(name: &str, slice: (u64, u64), fsub_va: u64) -> u32 {
    let Some(wow) = Wow::load() else {
        return 0;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // byte-verify the owned fsub is the shared ×(−1.0) kernel before trusting the diff.
    assert_eq!(
        emu.read(fsub_va, 6),
        ONE_FSUB_BYTES.to_vec(),
        "{name}: fsub @ {fsub_va:#x} != shared −1.0 kernel"
    );

    // Patch the two calls the slice spans:
    //   0x6f3620 lua_tonumber → `fld qword [in_cell]; ret`   (lands our f64 arg LIVE in st0)
    //   0x40a2b0 __ftol       → `fstp qword [out_cell]; ret` (captures the pre-trunc st0 = arg−1.0)
    let in_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let mut tonum = vec![0xddu8, 0x05]; // fld qword [in_cell]
    tonum.extend_from_slice(&(in_cell as u32).to_le_bytes());
    tonum.push(0xc3); // ret
    emu.write(0x6f3620, &tonum);
    let mut ftol = vec![0xddu8, 0x1d]; // fstp qword [out_cell]
    ftol.extend_from_slice(&(out_cell as u32).to_le_bytes());
    ftol.push(0xc3); // ret
    emu.write(0x40a2b0, &ftol);

    let (start, halt) = slice;
    let run = |emu: &mut Emu, arg: f64| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, 0x1234); // `this` (orchestration; untouched by the slice)
        emu.set_reg(RegisterX86::EDI, 0x1234);
        emu.write(in_cell, &arg.to_bits().to_le_bytes());
        emu.run(start, halt);
        f64::from_bits(u64::from_le_bytes(
            emu.read(out_cell, 8).try_into().unwrap(),
        ))
    };

    let mut total = 0u32;
    for &arg in &ARGS {
        assert_eq!(
            run(&mut emu, arg).to_bits(),
            rebase_index(arg).to_bits(),
            "{name} arg={arg} (bits {:#018x})",
            arg.to_bits()
        );
        total += 1;
    }
    // fuzz: HIGH bits → a wide spread of f64 args straddling the index domain (incl. fractional and
    // negative), exercising the same single `fsub` over many bit-patterns.
    let mut s = (fsub_va as u32).wrapping_mul(0x9e37_79b1).wrapping_add(1);
    for _ in 0..4000 {
        // High bits of the LCG: 0..=0xFFFFFF gives a signed fractional argument in about [-128, 16384).
        let hi = next(&mut s) >> 8;
        let arg = f64::from(hi as i32) / 1024.0 - 128.0;
        assert_eq!(
            run(&mut emu, arg).to_bits(),
            rebase_index(arg).to_bits(),
            "{name} fuzz arg={arg}"
        );
        total += 1;
    }
    total
}

/// Row 0x532d10 SetSpell — call 0x6f3620; fsub [0x8015b8]=1.0; call 0x40a2b0. fsub @0x532dd9.
#[test]
fn luabind_532d10_set_spell() {
    let n = diff_row("luabind_532d10_set_spell", (0x532dd4, 0x532de4), 0x532dd9);
    if n > 0 {
        eprintln!("luabind_532d10_set_spell: {n} bit-exact (lua_tonumber − 1.0 index re-base)");
    }
}

/// Row 0x533470 SetLootItem — call 0x6f3620; fsub [0x8015b8]=1.0; call 0x40a2b0. fsub @0x533541.
#[test]
fn luabind_533470_set_loot_item() {
    let n = diff_row(
        "luabind_533470_set_loot_item",
        (0x53353c, 0x53354c),
        0x533541,
    );
    if n > 0 {
        eprintln!("luabind_533470_set_loot_item: {n} bit-exact (lua_tonumber − 1.0 index re-base)");
    }
}

/// Row 0x534080 SetMerchantItem — call 0x6f3620; fsub [0x8015b8]=1.0; call 0x40a2b0. fsub @0x534159.
#[test]
fn luabind_534080_set_merchant_item() {
    let n = diff_row(
        "luabind_534080_set_merchant_item",
        (0x534154, 0x534164),
        0x534159,
    );
    if n > 0 {
        eprintln!(
            "luabind_534080_set_merchant_item: {n} bit-exact (lua_tonumber − 1.0 index re-base)"
        );
    }
}

/// Row 0x5341e0 SetTradePlayerItem — call 0x6f3620; fsub [0x8015b8]=1.0; call 0x40a2b0. fsub @0x5342b1.
#[test]
fn luabind_5341e0_set_trade_player_item() {
    let n = diff_row(
        "luabind_5341e0_set_trade_player_item",
        (0x5342ac, 0x5342bc),
        0x5342b1,
    );
    if n > 0 {
        eprintln!(
            "luabind_5341e0_set_trade_player_item: {n} bit-exact (lua_tonumber − 1.0 index re-base)"
        );
    }
}

/// Row 0x534410 SetTradeTargetItem — call 0x6f3620; fsub [0x8015b8]=1.0; call 0x40a2b0. fsub @0x5344e1.
#[test]
fn luabind_534410_set_trade_target_item() {
    let n = diff_row(
        "luabind_534410_set_trade_target_item",
        (0x5344dc, 0x5344ec),
        0x5344e1,
    );
    if n > 0 {
        eprintln!(
            "luabind_534410_set_trade_target_item: {n} bit-exact (lua_tonumber − 1.0 index re-base)"
        );
    }
}

/// Row 0x536080 SetMerchantCompareItem — call 0x6f3620; fsub [0x8015b8]=1.0; call 0x40a2b0.
/// fsub @0x536166. (The optional second arg re-base @0x53621e/0x536223 is INTEGER `dec eax` — no FP.)
#[test]
fn luabind_536080_set_merchant_compare_item() {
    let n = diff_row(
        "luabind_536080_set_merchant_compare_item",
        (0x536161, 0x536171),
        0x536166,
    );
    if n > 0 {
        eprintln!(
            "luabind_536080_set_merchant_compare_item: {n} bit-exact (lua_tonumber − 1.0 index re-base)"
        );
    }
}

//! Bit-exact tests of the TSLink relocation, the TSGrowableArray capacity policy and the
//! render-slot region-base table against `WoW.exe`. The container functions realloc through callees
//! that are not modelled (`0x646320`, `0x47b530`); those tests use a slice that halts at
//! the instruction that passes the computed size or capacity, just before the realloc, and read the
//! result register. The constant-table test slices the constant block and reads the 16 dwords
//! from `[this+0x144 .. +0x180]`.
use benilla_re_charactermodel as cm;
use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// A stack base well inside the mapped harness stack (`0x7000_0000..+0x10_0000`), with room below for the
/// pushes a sliced fn makes before its capture point. The `push ebp; mov ebp,esp` prologue makes
/// `[ebp+8] == mem[S+4]` and `[ebp+0xc] == mem[S+8]` for an entry `ESP == S`.
const SLICE_ESP: u64 = 0x7000_0000 + 0x8_0000;

/// Plant the entry stack frame for a `run`-sliced `__thiscall`/`__stdcall` fn: set `ECX=this`, point
/// `ESP` at `SLICE_ESP`, and write the stack args where `[ebp+8]`, `[ebp+0xc]`, … will land.
fn slice_setup(emu: &mut Emu, this: u32, stack_args: &[u32]) {
    emu.set_reg(RegisterX86::ECX, u64::from(this));
    emu.set_reg(RegisterX86::ESP, SLICE_ESP);
    emu.write_u32(SLICE_ESP, 0); // return-address slot (unused by the slice)
    for (i, &a) in stack_args.iter().enumerate() {
        emu.write_u32(SLICE_ESP + 4 + 4 * i as u64, a);
    }
}

/// `0x47b6c0` TSLink relocation resolver — `__thiscall` (`ecx`=this) + 1 stack arg (`ret 4`).
/// `a=[this+4]`: `(a&1||a==0)?a&~1 : arg>=0 ? a+arg : a + (this - [[this]+4])`. The negative-arg branch
/// reads `[this]` then `[[this]+4]`, so set up a chain node.
#[test]
fn tslink_reloc() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40) as u32;
    let node = emu.alloc(0x40) as u32; // [this] points here; [node+4] is the inner reloc base
    let mut total = 0u32;
    // a values: odd (bit0 set), zero, and even-nonzero (the only case where the arg branches matter).
    for a in [0u32, 1, 3, 0x1001, 0x1000, 0x2000, 0xdead_beef, 0xdead_bee0] {
        emu.write_u32(u64::from(this) + 4, a);
        for inner4 in [0u32, 0x10, 0x800, 0x4000_0000, 0xffff_fff0] {
            emu.write_u32(u64::from(this), node); // [this]
            emu.write_u32(u64::from(node) + 4, inner4); // [[this]+4]
            for arg in [
                0u32,
                1,
                0x20,
                0x7fff_ffff,
                0x8000_0000,
                0xffff_ffff,
                0xffff_fffc,
            ] {
                let bin = emu.call(0x47b6c0, this, 0, &[arg]);
                let exp = cm::container::tslink_reloc(this, a, arg, node, inner4);
                assert_eq!(
                    bin, exp,
                    "tslink_reloc this={this:#x} a={a:#x} arg={arg:#x} inner4={inner4:#x}"
                );
                total += 1;
            }
        }
    }
    eprintln!("tslink_reloc: {total} bit-exact");
}

/// `0x47bab0` TSGrowableArray::SetCount — grow-path SIZE. Set `[this+4]=0` (cur size), `[this]=0`
/// (capacity), `[this+0xc]=0` (stored cap) so any `count>0` takes the grow path; slice
/// `0x47bab0 .. 0x47bb18` (the `push esi`, before the `0x47bd80` realloc) and read `ESI` = the rounded
/// size. (`count==0` would take the shrink path, so the sweep starts at 1.)
#[test]
fn ts_setcount_grow() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40) as u32;
    let mut total = 0u32;
    // Sweep across the pow2-floor regime (<0x15) and the pinned-0x15 regime (>=0x15).
    for count in [
        1u32, 2, 3, 4, 5, 7, 8, 9, 0x10, 0x13, 0x14, 0x15, 0x16, 0x20, 0x2a, 0x40, 0x63, 0x100,
        0x1000,
    ] {
        emu.write_u32(u64::from(this), 0); // [this] capacity = 0
        emu.write_u32(u64::from(this) + 4, 0); // [this+4] current size = 0
        emu.write_u32(u64::from(this) + 0xc, 0); // [this+0xc] stored cap = 0
        slice_setup(&mut emu, this, &[count]);
        emu.run(0x47bab0, 0x47bb18);
        let bin_size = emu.get_reg(RegisterX86::ESI) as u32;
        let (_cap, exp_size) = cm::container::ts_setcount_grow(count);
        assert_eq!(bin_size, exp_size, "ts_setcount_grow count={count:#x}");
        total += 1;
    }
    eprintln!("ts_setcount_grow: {total} bit-exact");
}

/// `0x47bbf0` TSGrowableArray::Add-by-N — grow-path total/size. `total = [this+4] + extra`; set
/// `[this]=0` (capacity) and `[this+0xc]=0` (stored cap) so the grow path runs. Slice
/// `0x47bbf0 .. 0x47bc55` (where `esi` is final, before the `0x47b530` TSList loop + `0x646320`
/// realloc) and read `ESI` = the final size. `arg1` toggles the cap-rounding (when 0, `esi == total`).
#[test]
fn ts_add_n_size() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40) as u32;
    let mut total = 0u32;
    for old in [0u32, 1, 5, 0x10, 0x14, 0x40, 0x100] {
        for extra in [1u32, 2, 3, 5, 8, 0x14, 0x15, 0x20, 0x80, 0x1000] {
            for arg1 in [0u32, 1] {
                emu.write_u32(u64::from(this), 0); // [this] capacity = 0 → grow path
                emu.write_u32(u64::from(this) + 4, old); // [this+4] current size
                emu.write_u32(u64::from(this) + 0xc, 0); // [this+0xc] stored cap = 0
                slice_setup(&mut emu, this, &[extra, arg1]);
                emu.run(0x47bbf0, 0x47bc55);
                let bin_size = emu.get_reg(RegisterX86::ESI) as u32;
                let exp = cm::container::ts_add_n_size(old, extra, arg1 != 0);
                assert_eq!(
                    bin_size, exp,
                    "ts_add_n_size old={old:#x} extra={extra:#x} arg1={arg1}"
                );
                total += 1;
            }
        }
    }
    eprintln!("ts_add_n_size: {total} bit-exact");
}

/// `0x47bd80` TSGrowableArray::Grow — realloc byte size + stored count. Set `[this+4]=0` so any
/// `newcount>=0` skips the `0x47bd30`/`0x47b4f0` destroy loop; slice `0x47bd80 .. 0x47bdd9` (the
/// `0x646320` realloc `call`) and read `ESI` = `12*newcount` (the byte size) + `[this]` = `newcount`.
#[test]
fn ts_grow_bytesize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40) as u32;
    let mut total = 0u32;
    for newcount in [
        0u32, 1, 2, 3, 5, 0x10, 0x15, 0x40, 0x100, 0x1000, 0x10000, 0x15555556,
    ] {
        emu.write_u32(u64::from(this) + 4, 0); // [this+4] cur size = 0 → skip destroy loop
        emu.write_u32(u64::from(this) + 8, 0); // [this+8] data ptr (unread on this path)
        slice_setup(&mut emu, this, &[newcount]);
        emu.run(0x47bd80, 0x47bdd9);
        let bin_size = emu.get_reg(RegisterX86::ESI) as u32;
        let bin_count = emu.read_u32(u64::from(this)); // [this] = newcount (stored at 0x47bdd7)
        assert_eq!(
            bin_size,
            cm::container::ts_grow_bytesize(newcount),
            "ts_grow_bytesize size newcount={newcount:#x}"
        );
        assert_eq!(
            bin_count, newcount,
            "ts_grow_bytesize count newcount={newcount:#x}"
        );
        total += 1;
    }
    eprintln!("ts_grow_bytesize: {total} bit-exact");
}

/// `0x476810` (constructor): the 16 render-slot region-base constants written to `[this+0x144 ..
/// +0x180]` (step 4). The test slices the constant block (`begin .. lea edi,[esi+0x188]`, before the
/// sub-array constructor `0x47a900`, which is not modelled) and reads the 16 dwords back.
#[test]
fn region_base_table() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x800) as u32;
    let exp = cm::container::region_base_table();
    // The end of the constant block is the `lea edi,[esi+0x188]` that starts the sub-array constructors.
    let (begin, end) = (0x476810u64, 0x4768e1u64);
    // The table region is poisoned first, so a missed store diverges instead of matching zero by chance.
    for i in 0..16u64 {
        emu.write_u32(u64::from(this) + 0x144 + 4 * i, 0xbad0_0000 + i as u32);
    }
    slice_setup(&mut emu, this, &[]);
    emu.run(begin, end);
    for (i, &want) in exp.iter().enumerate() {
        let got = emu.read_u32(u64::from(this) + 0x144 + 4 * i as u64);
        assert_eq!(got, want, "0x476810 region base [+{:#x}]", 0x144 + 4 * i);
    }
    eprintln!("region_base_table: 16 bit-exact");
}

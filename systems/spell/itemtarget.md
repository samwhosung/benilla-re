# spell: itemtarget

The check a clicked item passes before it becomes the target of the pending spell: the enchant effects, the confirm events and the prospecting effect.

While a spell awaits a target and its flag word wants an item (`targeting`), a click on an inventory or bag
item goes to the item validator `0x495d60` (`GameUI.cpp`, owned by `ui`), not straight to the binder. It
checks the item against each of the pending spell's three effects, may raise a confirm event instead of
binding, and otherwise binds the item through `0x6e5b40` (`targeting`, Binding a clicked object). The weapon
auto-pick of the cast arm binds through the same binder without this check (`cast`, Arm candidates).

Roots: `0x6e5b40`, `0x6e3760`

## Item click validator

`0x495d60(suppress, item guid)` resolves the item (mask 2), the pending spell (`0x6e3d10`) and its record;
a missing item or record returns without a report. It walks the three effects once each. An enchant effect
(53 or 54) runs the enchant checks, effect 127 the prospecting checks, any other effect nothing. When the
walk ends it binds the item and returns. Its three callers are the paper doll click `0x4c76fb`, the bag
click `0x4f9c6d` and the Lua `BindEnchant`, which passes 1 for `suppress`. The popup answer `ReplaceEnchant`
(`0x48d300`) binds the stashed item with `0x6e5b40` directly, without calling it again.

Addresses: `0x495d60`, `0x495d95`, `0x495da7`, `0x495dd4`, `0x495de9`, `0x495dee`, `0x495f36`, `0x495f39`, `0x49603d`, `0x496047`, `0x496058`, `0x4c76fb`, `0x4f9c6d`, `0x48d2e0`, `0x48d2f2`, `0x48d300`, `0x48d327`, `0x6e5b40`, `0x6e3d10`, `0x468460`
Confidence: verified
Proof: one
Test: none

`0x495d95` calls `0x468460` with mask 2 and the guid pair at `[ebx+8]`; `0x495da7` reads the pending
spell. The loop counter `[ebp-0x10]` and cursor `[ebp-0x8]` start at 0 and `Effect[0]` (`0x495dd4
lea eax,[esi+0xf4]`); `0x496047 cmp eax,3; jb 0x495de4`. `0x495de9` and `0x495dee` compare the effect
with 0x35 and 0x36, `0x495f39` with 0x7f. The image holds three calls of this function (`0x48d2f2`,
`0x4c76fb`, `0x4f9c6d`) and no dword of its address; the control is the same scan finding the vtable
slot `0x764360` once. `0x48d2ed mov ecx,1` is the suppress argument. `0x6e5b40` has six calls
(`0x48d327`, `0x4935d5`, `0x496058`, `0x5f35cf`, `0x6e53c3`, `0x6e5400`) and two tail jumps (`0x6e5b2d`,
`0x6ea274`).

## Enchant effect checks

An item whose template flags have bit 13 (0x2000) ends the validator silently. For an enchant effect, a spell
with an `EquippedItemSubClassMask` (`+0xec`) needs the item's class to equal `+0xe8` and its subclass bit in
the mask, and one with an inventory type mask (`+0xf0`) the item's type bit; a miss reports reason 0x0a. Then
an enchantment row (from `EffectMiscValue`) with flag bit 0 raises `BIND_ENCHANT` (402) and stops, when the item
is not bound, `suppress` is 0 and its inventory type is nonzero. The item's existing enchantment of the same
kind (effect 53 permanent, 54 temporary) with a new row raises `REPLACE_ENCHANT` (403) with the two names.

Addresses: `0x495df7`, `0x495e05`, `0x495e07`, `0x495e0a`, `0x495e10`, `0x495e22`, `0x495e31`, `0x495e4d`, `0x495e5a`, `0x495e79`, `0x495ea3`, `0x495eac`, `0x495ebf`, `0x495ecc`, `0x495ed6`, `0x495eec`, `0x495f1c`, `0x496068`, `0x496077`, `0x496087`, `0x4960a3`, `0x4960b3`, `0x4960e4`, `0x5d9f30`, `0x5d9f90`, `0x5d9ff0`, `0x5da2a0`, `0x5da2c0`, `0x6e1a00`, `0x703e50`, `0x703f50`
Confidence: verified
Proof: one
Test: none

`0x495e05` is the item's flags accessor (vtable slot 0, `0x5da2a0`); `0x495e07 test ah,0x20` is bit 13. The
class compare is `0x495e22`; the subclass `shl edx,cl` test is `0x495e45`, the inventory type's `0x495e6e`;
each miss goes to `0x496068`, which calls `0x6e1a00` with the in-flight spell, `dl` 0xa
(`0x496077`). `0x495e79` loads `[Effect+0xb4]`, `EffectMisc` at `+0x1a8`, and the
SpellItemEnchantment.dbc row (`tables/dbc_stores.tsv`); `0x495ea3 test [row+0x5c],1`, `0x495eac` calls the
bound test and `0x495ebf` the inventory type. `0x496087` stores the guid at `0xb4e3c0` and `0xb4e3c4` and
signals 402 (`0x4960a3`). `0x495ed6` makes `esi` 0 for effect 53 and 1 for 54; the item's enchantment slot is
`[fields + esi·12 + 0x40]` (`0x495eec`). `0x4960b3` stores the guid again and signals 403 with the format
`%s%s` and the rows' names at `+0x34` of the locale column (`0x4960e4`).

## Prospecting effect checks

Effect 127 requires the item's template flags to have bit 18 (0x40000), else the validator reports 0x0d.
With the item record cached it then needs the player to know the record's required skill (`+0x40`) and reach its
rank (`+0x44`), else it reports 0x90 with the skill and rank; the rank is the skill's value plus its permanent
bonus. Last the item's stack count must reach the effect's computed amount (`0x6e3760`), else it reports 0x84
with the item entry and the count. A record not yet cached skips the skill test, and the lookup, given no callback, sends no query.

Addresses: `0x495f42`, `0x495f4e`, `0x495f50`, `0x495f55`, `0x495f71`, `0x495f76`, `0x495f82`, `0x495fab`, `0x495fb4`, `0x495fe8`, `0x495ff3`, `0x495ffb`, `0x496000`, `0x496003`, `0x496009`, `0x49601d`, `0x49602e`, `0x496031`, `0x4960f7`, `0x496106`, `0x496116`, `0x49612c`, `0x49613c`, `0x496152`, `0x55ba30`, `0x5ea3f0`, `0x6e3760`, `0xc0e2a0`
Confidence: verified
Proof: one
Test: none

`0x495f50 test eax,0x40000` on the flags; `0x4960f7` reports 0x0d (`0x496106`). `0x495f71` calls `0x55ba30` on
the item cache `0xc0e2a0` with a null callback and no query; a null result jumps to `0x496009`. The skill
index is `0x5ea3f0([rec+0x40])`; `-1` (`0x495fb4 jl 0x496116`) reports 0x90. The sum reads the 16-bit value
at `[player+0xe68] + 12i + 0x84c` and, when positive, adds the 16-bit permanent bonus at `+0x852`
(`0x495fe8`, `0x495ff3`, `0x495ffb`); `0x496000 cmp eax,[rec+0x44]; jl 0x496119` reports 0x90 (`0x49612c`). The
count test is `0x49602e cmp [stats+0x20],eax` against the amount (`0x49601d`), reporting 0x84 at `0x496152`. The
three reasons have no other local source: of the 63 calls of `0x6e1a00` (`reasons`), `0x496106`, `0x49612c`
and `0x496152` are the only ones that load 0x0d, 0x90 and 0x84.

# ui: reactive spells and the book reset

The spells the interface retests for usability, and the reset that empties the spell book. A learned
spell whose record has `AttributesEx4` bit 0x200 enters the reactive list when `AddSpellToBook` runs
(`ui/spellbook`). The usable refresh `0x4b31c0` retests the list after the action bar's flags and raises
`SPELL_ACTIVE` for a spell that became usable. The reset `0x4b23a0` empties the book, its tabs and the opener, form,
dispel and reactive lists, when the interface starts and when the active player arrives.

The code sits among the spell book's: the reset `0x4b23a0` after its start-up and shutdown functions
(`0x4b21c0`, `0x4b22e0`), the refresh `0x4b31c0` among its event functions (`0x4b31b0`, `0x4b3250`).
Twelve functions of the target, control, combo point, shapeshift, unit-field and cooldown code call the
refresh.

Roots: `0x4b23a0`, `0x4b31c0`

## Reactive spell list

The reactive list at `0xb71148` holds 8-byte entries `{spell id, usable}`, one per name among the
learned spells whose record has `AttributesEx4` (`+0x28`) bit 0x200. `AddSpellToBook` tests the bit for
every learned spell, before its `Attributes` and `CastUI` gates. An entry with the same name (`SStrCmp`
of the locale name, case counted) takes the new id only when `ParseSpellRank` of the new record is
strictly higher, keeping its `usable` word; a new name is appended as `{id, 0}`. Outside `AddSpellToBook`,
only `FireUsableEvents` reads its entries (`Usable refresh`). No code removes an entry: unlearning
(`0x4b2c50`) leaves it, and the reset (`Resetting the book`) and the exit callback `0x4b2190` free the
whole list.

Addresses: `0x4b25b0`, `0x4b2705`, `0x4b2708`, `0x4b270b`, `0x4b2720`, `0x4b275f`, `0x4b2776`, `0x4b2792`, `0x4b279e`, `0x4b27a9`, `0x4b27b3`, `0x4b27c7`, `0x4b2828`, `0x4b2838`, `0x4b2844`, `0x4b2911`, `0x4b3c30`, `0x64a480`, `0x4b4fe0`, `0x4b4fe6`, `0x4b4feb`, `0x4b4ff9`, `0x4b5000`, `0x4b5009`, `0x4b5020`, `0x4b5040`, `0xb71148`, `0xb7114c`, `0xb71150`, `0xb71154`, `0xb71104`, `0x4b2c50`, `0x4b2190`, `0x4b21b9`, `0x4b31c0`
Confidence: verified
Proof: one
Test: none

`0x4b2705 mov ecx,[eax+0x28]` and `0x4b2708 test ch,2` guard the block; `0x4b270b je` skips it. The
walk `0x4b2720` to `0x4b2774` takes each entry's record from `[0xc0d788][id]` (bound `0xc0d78c`) and
compares the names at `+0x1e0 + 4·[0xc0e080]` with `0x64a480(a, b, 0x7fffffff)`. Zero goes to
`0x4b2828`, which ranks the held record, then the new one with `0x4b3c30`; `0x4b283a jbe` keeps the
entry unless the new rank is greater, and `0x4b2844` stores the id alone. No match reaches `0x4b2776`:
a count past the capacity grows the array through `0x4b4fe0`, `0x4b5020` and `0x4b5040`, then
`0x4b27b3` to `0x4b27c7` store `{id, 0}`. `0x4b4fe0` at `0x4b4fe6` sends 32 and up to `0x4b5009`, which
stores 0x20 at `[ecx+0xc]` and returns it; below, `0x4b4feb` to `0x4b4ff9` clear the lowest set bit
until one is left, and `0x4b5000` returns 1 for 0. Census of the four dwords `0xb71148` to `0xb71154`
(`tools/callers.py`; control `0xb71104`, whose removal in `0x4b2c50` it finds): the static initializer
and its exit callback (it frees the block at `0x4b21b9`), `0x4b23a0`, `AddSpellToBook` and
`FireUsableEvents`. `Spell.dbc` column 10 has the bit in
26 of 22357 records, of seven names: Mongoose Bite, Execute, Revenge, Overpower, Riposte, Counterattack
and Hammer of Wrath.

## Usable refresh

`FireUsableEvents` (`0x4b31c0`) runs `0x4e5c00` (`ACTIONBAR_UPDATE_USABLE` when an action slot changed),
then retests every reactive entry with `IsSpellUsable` and stores the answer. An entry that went from 0 to
1 (a new entry starts at 0) raises `COMBAT_TEXT_UPDATE` with `SPELL_ACTIVE` and the spell's name, in list order; a fall to 0 is
stored silently. It then signals `SPELL_UPDATE_USABLE` (263) on every path, and reads no form-bar entry.
It has no timer; twelve functions call it:

| caller | site |
|---|---|
| `CGGameUI::SelectTarget`, `CGGameUI::ReleaseTarget` | `0x493892`, `0x493a69` |
| `SetPlayerControl` | `0x4958fc`, `0x495910` (tail jumps) |
| `OnComboPointsChanged`, `OnShapeshiftFormChanged` | `0x5ddff0`, `0x5de011` |
| `CGUnit_C::OnActivePlayerFlagsChanged` | `0x5ff50e`, `0x5ff51b` |
| `CGUnit_C::OnBytes1Byte3Changed` | `0x5ff83a` |
| `CGUnit_C::OnHealthFieldChanged`, `OnPowerFieldChanged`, `OnAuraStateFieldChanged` | `0x604848`, `0x604876`, `0x6048b6` |
| `StartGlobalCooldown`, `StartOrRemoveSpellCooldown` | `0x6e2e85`, `0x6e3076` |

Addresses: `0x4b31c0`, `0x4b31c5`, `0x4b31d7`, `0x4b31fd`, `0x4b320b`, `0x4b3221`, `0x4b3226`, `0x4b3231`, `0x4b323d`, `0x4b3241`, `0x4b3246`, `0x4e5c00`, `0x6e3d60`, `0x6269b0`, `0x862920`, `0xb7114c`, `0xb71150`, `0x493540`, `0x493910`, `0x4958e0`, `0x5ddff0`, `0x5de010`, `0x5ff4e0`, `0x5ff800`, `0x6046f0`, `0x604860`, `0x604890`, `0x6e2de0`, `0x6e3050`
Confidence: verified
Proof: one
Test: none

`0x4b31c5 call 0x4e5c00`. The loop `0x4b31d7` to `0x4b323d` walks the 8-byte entries of `[0xb71150]`, count
`[0xb7114c]`; `0x4b31fd` calls `0x6e3d60` on the record, discarding `noPower`. `0x4b320b cmp esi,[edx+ebx*8+4]`
skips an unchanged answer, `0x4b3213` skips the event for 0, and `0x4b3231` stores the answer. For a rise
`0x4b3221 mov ecx,0x26` and `0x4b3226 call 0x6269b0` pass the name at `+0x1e0 + 4·[0xc0e080]`; index 0x26
of the table `0x862920` is `SPELL_ACTIVE`. `0x4b3246` signals 0x107. Census of `0x4b31c0`
(`tools/callers.py`; controls `0x4b31b0`, 12 sites, and `0x4b2bb0`, a stored address): 14 `call` and `jmp`
sites in these functions, no stored address. The five combat text signals have 68 call sites, none
stored, each loading its type into `ecx` with a `mov` of an immediate (`xor ecx,ecx` at `0x626b67`, type
0); 0x26 occurs once, at `0x4b3221`.

## Resetting the book

`0x4b23a0` empties the player's book and its lists, and signals nothing. It
clears the known-spell bitset in place when `0xb710f0` is nonzero, the 1024 book slots and the book
count, frees the tab nodes and their array and zeroes the tab list and the tab count, zeroes the four
effect-spell globals, clears the indexed spell array in place, then frees and zeroes the opener list,
the form list, the dispel types and the reactive list. The pet book and the spell-history record that
`AddSpellToBook` adds for `AttributesEx2` & 0x8100 (`0x6e2c60`, `0x4b27ef`) stay. `InitSpellBook` calls
it at both of its exits and `CGPlayer_C::OnActiveArrival` calls it before it learns the player's
initial spells; no other code does.

Addresses: `0x4b23a0`, `0x4b21c0`, `0x4b2244`, `0x4b22d3`, `0x4b27ef`, `0x6e2c60`, `0x5dea50`, `0x5dea86`, `0xb710f0`, `0xb710f4`, `0xb710fc`, `0xb700f0`, `0xb7116c`, `0xb71158`, `0xb71160`, `0xb71170`, `0xb71130`, `0xb70098`, `0xb700e4`, `0xb700e8`, `0xb700a4`, `0xb700ac`, `0xb700b0`, `0xb71100`, `0xb71104`, `0xb71108`, `0xb71110`, `0xb71148`, `0x4b4b90`, `0x4088f0`, `0x646430`
Confidence: verified
Proof: one
Test: none

In address order: `0x4b23a0` to `0x4b23bb` `rep stosd` the `[0xb710f8]` dwords at `[0xb710fc]`;
`0x4b23c4` the 0x400 dwords at `0xb700f0`; `0x4b23d4` stores 0 in `0xb7116c`; `0x4b23e0` to
`0x4b2402` free each of the `[0xb7115c]` nodes at `[0xb71160]`, `0x4b2409` the array (`0x4b4b90`), and
`0x4b2415` to `0x4b2427` zero the header and `0xb71170`; `0x4b242d` to `0x4b243f` store 0 in the four
globals; `0x4b2447` to `0x4b2451` zero `[0xb700a8]` dwords at `[0xb700ac]` from the top down;
`0x4b2458` frees the opener list and `0x4b2464` to `0x4b2470` zero it; the form, dispel and reactive
blocks are freed at `0x4b2481`, `0x4b24aa` and `0x4b24d3` when non-null, their three dwords zeroed at
`0x4b248d`, `0x4b24b6` and `0x4b24d9`. `0x4b21c0` sets `0xb710f0` and `0xb700a4` from
`[0xc0d78c] + 1` and `[0xc0db4c] + 1`. Census of `0x4b23a0`: three `call`s (`0x4b2244`, `0x4b22d3`,
`0x5dea86`), no `jmp`, no stored address; control `0x4b2bb0` finds its stored one.

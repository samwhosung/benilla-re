# ui: GameTooltip, the widget

How a `GameTooltip` is anchored to its owner, how lines are added, and how it fades. What each
`Set…` method puts into the tooltip for a game object belongs to the docs of those objects.

A `CGameTooltip` (`Tooltip.cpp`) is a frame whose fields include the owner `+0x314`, the anchor
mode `+0x318`, the line count `+0x31c` and the line limit `+0x320`, the fading flag `+0x3b4` and
time left `+0x3b8`, and the owner offsets `+0x3c4` and `+0x3c8`.

Roots: `0x5368a0`, `0x530b20`

## SetOwner and the anchor modes

`SetOwner(owner, anchor, x, y)` clears the tooltip, restores full alpha and stores the owner, the
mode and the offsets, which it converts from script units. The anchor names give the modes and the
point of the tooltip set to a point of the owner, offset by x and y:

| name | mode | tooltip point | owner point |
|---|---|---|---|
| `ANCHOR_LEFT`, and an absent or unknown name | 0 | `BOTTOMRIGHT` | `TOPLEFT` |
| `ANCHOR_RIGHT` | 1 | `BOTTOMLEFT` | `TOPRIGHT` |
| `ANCHOR_BOTTOMLEFT` | 2 | `TOPRIGHT` | `BOTTOMLEFT` |
| `ANCHOR_BOTTOMRIGHT` | 3 | `TOPLEFT` | `BOTTOMRIGHT` |
| `ANCHOR_TOPLEFT` | 4 | `BOTTOMLEFT` | `TOPLEFT` |
| `ANCHOR_TOPRIGHT` | 5 | `BOTTOMRIGHT` | `TOPRIGHT` |
| `ANCHOR_CURSOR` | 6 | (next entry) | |
| `ANCHOR_NONE` | 7 | all points cleared | |
| `ANCHOR_PRESERVE` | 8 | points kept | |

Addresses: `0x5310d0`, `0x53120d`, `0x53123a`, `0x531333`, `0x531379`, `0x5313b2`, `0x5313cf`, `0x52ffe0`, `0x52ffe6`, `0x52fff4`, `0x53000c`, `0x530031`, `0x530037`, `0x52fe90`, `0x52fead`, `0x52febd`, `0x52fec2`, `0x52fed6`, `0x52ff12`
Confidence: verified
Proof: one
Test: `ui::luabind_530_setters`

`0x5310d0` stages mode 0 (`0x53120d`) and compares the name with the nine strings in turn
(`0x53123a` to `0x531333`); arguments 4 and 5 go through `0x41ae60` (`0x531379`, `0x5313b2`).
`0x52ffe0` calls `0x530050` (`0x52ffe6`) and slot 0x28 with 255 (`0x52fff4`), stores the owner
(`0x53000c`) and, with an owner, the mode (`0x530031`), then `0x52fe90(1)` (`0x530037`). That
returns for mode 8 (`0x52fead`), clears the points (`0x52fec2`) and, for modes 0 to 5, jumps through
the table at 0x52ffbc (`0x52fed6`) to `0x767c70(point, owner, owner point, x, y, 1)` (`0x52ff12`).

Bit-exact coverage: `ui::luabind_530_setters` covers `0x531367` to `0x531388`, `0x5313a0` to `0x5313cf` of `0x5310d0`.

## Anchoring again

`0x52fe90(clear)` applies the anchor of the owner and mode. It does nothing without an owner or in mode 8.
It clears the tooltip's points unless `clear` is 0 and the mode is 7, so a tooltip whose points a script set
by hand keeps them when its size changes. Modes 0 to 5 then set the one point of the table; modes 6 and 7 set
none (mode 6 is placed on every update). `SetOwner` first resets the fade state and the alpha, and stores
mode 7 before it looks at the owner, so an absent owner leaves the mode at 7 and anchors nothing.

Addresses: `0x52fe90`, `0x52fe96`, `0x52fe9e`, `0x52fea4`, `0x52feaa`, `0x52feb6`, `0x52feba`, `0x52febd`, `0x52fec2`, `0x52fec7`, `0x52fed0`, `0x52fed6`, `0x52ffe0`, `0x52fff4`, `0x530002`, `0x53000c`, `0x530012`, `0x530028`, `0x530037`, `0x530a49`, `0x767ed0`
Confidence: verified
Proof: one
Test: none

`0x52fe9e` and `0x52fead` leave for a null owner and mode 8. `0x52feb6 test ecx,ecx; jne 0x52febf` clears for a
nonzero argument; for 0 `0x52feba cmp eax,0x7; je 0x52fec7` skips `0x767ed0` (`0x52fec2`). The jump table is
reached only for modes up to 5 (`0x52fed0`, `0x52fed6`). `0x52ffe0` sets the alpha 255 (`0x52fff4`), clears
`+0x3b4` (`0x530002`), stores the owner (`0x53000c`) and mode 7 (`0x530012`) and, with an owner, the given mode
and the call with 1 (`0x530028`, `0x530037`). The size pass calls it with 0 (`0x530a49`).

## Following the cursor

In mode 6, each update sets the tooltip's `BOTTOM` to the screen's `BOTTOMLEFT` at the last mouse
position, divided by the tooltip's scale; `SetOwner`'s offsets play no part.

Addresses: `0x530b20`, `0x530b2d`, `0x530b32`, `0x530b4f`, `0x530b63`, `0x530b6b`, `0x530b97`
Confidence: verified
Proof: one
Test: `ui::cursor_fade_530b20`

`0x530b20` (slot 14) calls `0x76b2c0` (`0x530b2d`) and, with `+0x318` equal to 6 (`0x530b32`),
reads the top's `+0x1118` and `+0x111c` (`0x530b4f`), converts them with `0x41ad80` (`0x530b63`),
multiplies by `1/+0x7c` (`0x530b6b`) and calls `0x767c70(7, [0xcf0bd8], 6, x, y, 1)` (`0x530b97`).

## Lines

A line is added only while the tooltip has a free line slot, and only when its left or right text
is nonempty: `AddLine("")` adds nothing. A line with a right text never wraps.

Addresses: `0x531630`, `0x53182b`, `0x5303b0`, `0x5303c2`, `0x530270`, `0x53027d`, `0x530284`, `0x530291`, `0x5302a9`
Confidence: verified
Proof: one
Test: `ui::luabind_530_add_line`

`AddLine` (`0x531630`) calls `0x5303b0` (`0x53182b`), which calls `0x530270(left, 0, …)`
(`0x5303c2`). `0x530270` zeroes the wrap argument when the right text is nonempty (`0x53027d`,
`0x530284`), returns when `+0x31c` equals `+0x320` (`0x530291`) and when both texts are empty
(`0x5302a9`).

Bit-exact coverage: `ui::luabind_530_add_line` covers `0x531709` to `0x53181a` of `0x531630`.

## Fading out

`FadeOut` hides a cursor-anchored tooltip at once. Any other fades over 2 seconds: full alpha for
the first second, then `trunc(left · 255)` over the last; when the time runs out the tooltip is
hidden and its alpha restored.

Addresses: `0x531f00`, `0x530ae0`, `0x530ae3`, `0x530aee`, `0x530b00`, `0x530b10`, `0x808e7c`, `0x530baa`, `0x530bca`, `0x530bd8`, `0x530be9`, `0x530bf6`, `0x530c05`, `0x530c18`, `0x530c2c`
Confidence: verified
Proof: one
Test: none

`0x530ae0` calls slot 0x84 for mode 6 (`0x530ae3`, `0x530aee`), else sets `+0x3b4` (`0x530b00`) and
stores 2.0 from `0x808e7c` at `+0x3b8` (`0x530b10`). `0x530b20` subtracts the frame time
(`0x530baa`); at or below 0 (`0x530bca`) it calls slot 0x28 with 255 (`0x530bd8`) and slot 0x84
(`0x530be9`); else it compares the time with 2.0 · 0.5 (`0x530bf6`, `0x530c05`), divides it by
that when below (`0x530c18`), and passes `trunc(· 255)` to slot 0x28 (`0x530c2c`).

## Lua methods

`GameTooltip` has 46 methods at `0x854198`, registered by `0x5368a0`, from `AddFontStrings` to
`SetLootRollItem`.

Addresses: `0x854198`, `0x5368a0`, `0x530c40`, `0x530e50`, `0x530f10`, `0x530fe0`, `0x5310d0`, `0x5313e0`, `0x531560`, `0x531630`, `0x531840`, `0x531b90`, `0x531e30`, `0x531f00`, `0x531fd0`, `0x5322a0`, `0x532730`, `0x532920`, `0x532b00`, `0x532c50`, `0x532d10`, `0x532ee0`, `0x533470`, `0x533610`, `0x533760`, `0x5338b0`, `0x533ac0`, `0x533cd0`, `0x533e90`, `0x534080`, `0x5341e0`, `0x534410`, `0x534620`, `0x5349b0`, `0x534ac0`, `0x534e30`, `0x535170`, `0x5353b0`, `0x5354c0`, `0x535700`, `0x535810`, `0x535ad0`, `0x535bb0`, `0x535c60`, `0x535d70`, `0x536080`, `0x536300`, `0x5364a0`
Confidence: verified
Proof: one
Test: `ui::luabind_530_add_double_line`, `ui::luabind_530_set_text`, `ui::luabind_530_setters`, `ui::luabind_532d10_set_spell`, `ui::luabind_533470_set_loot_item`, `ui::luabind_534080_set_merchant_item`, `ui::luabind_5341e0_set_trade_player_item`, `ui::luabind_534410_set_trade_target_item`

The table is 46 pairs of a name and a function, `AddFontStrings` (`0x530c40`) to `SetLootRollItem`
(`0x5364a0`); `0x5368a0` calls `0x701d80` with 0x2e and the hash 0xc0cf20.

Bit-exact coverage: `ui::luabind_530_add_double_line` covers `0x531927` to `0x531b69` of `0x531840`; `ui::luabind_530_set_text` covers `0x531c5e` to `0x531d64`, `0x531d69` to `0x531ddc` of `0x531b90`; `ui::luabind_530_setters` covers `0x530fa8` to `0x530fc5` of `0x530f10`; `ui::luabind_532d10_set_spell` covers `0x532dd4` to `0x532ddf` of `0x532d10`; `ui::luabind_533470_set_loot_item` covers `0x53353c` to `0x533547` of `0x533470`; `ui::luabind_534080_set_merchant_item` covers `0x534154` to `0x53415f` of `0x534080`; `ui::luabind_5341e0_set_trade_player_item` covers `0x5342ac` to `0x5342b7` of `0x5341e0`; `ui::luabind_534410_set_trade_target_item` covers `0x5344dc` to `0x5344e7` of `0x534410`.

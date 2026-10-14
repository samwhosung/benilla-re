# ui: the skills list and its verbs

How the skills window's list (`SkillInfo.cpp`, `0x4d2ab0` to `0x4d4090`) is built from the player's skill slots, grouped by
category, ordered and collapsed, and what the Lua functions that read it, stage skill-up points and buy skill tiers do. The
skill field block and the events that rebuild the list are `objectlayer/playerfields` ("Skill changes"); the point counters are
`ui/talents`.

Roots: `0x4d2cb0`, `0x4d2ed0`, `0x4d3070`, `0x4d3610`, `0x4d32d0`, `0x4d3480`, `0x4d3d40`, `0x4d3e50`, `0x4d3ab0`

## Skill slots

The player's field block holds 128 skill slots of 12 bytes from `+0x848`: the skill line id (u16), the tier step (u16 at `+2`), the
rank (u16 at `+4`), the maximum rank (u16 at `+6`), a signed temporary modifier (s16 at `+8`) and a permanent bonus (u16 at
`+0xa`). The Lua functions add the bonus to the rank and the maximum when these are above 0, and read the active player's slots only.

Addresses: `0x4d2d26`, `0x4d2db4`, `0x4d3396`, `0x4d33a8`, `0x4d33db`, `0x4d33ea`, `0x4d389e`, `0x4d3925`, `0x4d3813`
Confidence: verified
Proof: one
Test: none

`0x4d2d26` reads the skill line id at `+0x848` and `0x4d2db4` the rank at `+0x84c`. `0x4d33db` reads the maximum at `+0x84e`, `0x4d33ea` the bonus at `+0x852`, `0x4d389e` the tier step at `+0x84a` and `0x4d3925` the signed modifier at `+0x850`.

## Building the list

`0x4d2cb0` rebuilds the list from the slots. A slot adds a row when its skill line and category are known and the player's race
and class row (`FindSkillRaceClassInfo`) exists with flag 2 clear. A skill with no rank is listed only for a row with flag 1 or,
with flag 4, when the player's level reaches the row's level. A row is five dwords: the skill line id, the category, the visible
flag, the unlearned flag and the staged ups. Each new category gets a header record (category, collapsed flag) and a header row
with skill id 0. It then sorts and signals SKILL_LINES_CHANGED (265).

Addresses: `0x4d2cb0`, `0x4d2d60`, `0x4d2d80`, `0x4d2d9c`, `0x4d2da2`, `0x4d2da8`, `0x4d2dd7`, `0x4d2dee`, `0x4d2e1c`, `0x4d2e2e`, `0x4d2e75`, `0x4d2e83`, `0x4d2eb7`, `0x4d2ab0`, `0x6ddf90`, `0xb7318c`, `0xb73170`, `0xb731b8`, `0xb731bc`
Confidence: verified
Proof: one
Test: none

`0x4d2ce2` to `0x4d2cfb` clear the counters first. `0x4d2d9f` tests flag 2 (skip) and `0x4d2da2` flag 1; `0x4d2dc5` tests flag 4 and `0x4d2dd7` compares the player's level (`+0x70`) with the row's `+0x14`. `0x4d2e1c` stores the unlearned flag (rank 0). `0x4d2e75` to `0x4d2e9d` make the header row. Rows are allocated by `0x4d2ab0`.

## Order and visibility

`0x4d2ed0` sets each header's collapsed flag from the expanded mask (0xb731c8; a clear bit is collapsed), then marks each row visible or
not, counts the visible rows (0xb731c0, what `GetNumSkillLines` returns) and sorts. A header row is always visible. A skill row is
hidden when its category is collapsed or, with the hide flag at 0xb731cc set (nothing sets it), when the skill is unlearned. Categories
sort by the `+0x28` order value of their category row. Rows sort visible before hidden, by category position, the header row first, learned before
unlearned, then by the skill line's name, case-insensitively.

Addresses: `0x4d2ed0`, `0x4d2f0c`, `0x4d2f74`, `0x4d2f8d`, `0x4d2fa0`, `0x4d2fc8`, `0x4d2fe2`, `0x4d2fec`, `0x4d3000`, `0x4d3038`, `0x4d3070`, `0x4d3095`, `0x4d30f0`, `0x4d3130`, `0x4d318d`, `0x4d3198`
Confidence: verified
Proof: one
Test: none

`0x4d2f0c` stores the collapsed flag, `0x4d2f74` reads the hide flag and `0x4d2fa0` lowers the visible count. `0x4d3000` compares the order values with `setge`, so equal orders compare as greater. `0x4d3095` and `0x4d3198` handle hidden rows, `0x4d30f0` the category position, `0x4d3130` the unlearned flag and `0x4d318d` the names.

## Reading a line

`GetSkillLineInfo(row)` returns 12 values for a header row: its category name, 1, 1 when expanded else nil, then 0, 0, 0, 0, nil,
nil, nil, 0, 0. For a skill row it returns 13: the name, nil, nil, the rank, the staged ups, the modifier, the maximum, the abandon
flag, the tier value, the next rank cost, the minimum level, the cost type plus 1 and the skill line's description. A row past the
list gives 13 empty values (nil, nil, nil, 0, 0, 0, 0, nil, nil, nil, 0, 0, nil).

Addresses: `0x4d3610`, `0x4d36e1`, `0x4d36f1`, `0x4d370f`, `0x4d3768`, `0x4d3a20`, `0x4d3a2a`, `0x4d3aa4`, `0x4d35e0`, `0x4d31c0`
Confidence: verified
Proof: one
Test: none

`0x4d36f1` calls `0x4d31c0`, which gives the collapsed flag of the row's header, and `0x4d3768` returns 12. `0x4d3a20` returns 13 and `0x4d3a2a` pushes the empty set.

## The values of a skill row

A skill whose race and class row has flag 0x400 shows a maximum of 1 and a rank of at most 1. It is abandonable when the slot's tier
step is not 0 and the row has flag 0x20. The tier value is shown when the row has flag 8, or flag 4 with step 0, and is nonzero. The next rank
cost is `GetSkillCostForRank` for the staged ups plus the rank plus 1, nil when 0.

Addresses: `0x4d38b1`, `0x4d38cb`, `0x4d38f6`, `0x4d3943`, `0x4d395d`, `0x4d397a`, `0x4d398d`, `0x4d39c1`, `0x4d39ef`, `0x4d3a06`, `0x6de110`, `0x6de1c0`
Confidence: verified
Proof: one
Test: none

`0x4d38b1` tests bit 0x400 of the row flags, `0x4d395d` flag 0x20, `0x4d397a` flags 8 and 4 and `0x4d398d` calls `GetSkillTierValue`. `0x4d39c1` calls `GetSkillCostForRank` for the staged ups plus the rank plus 1.

## Staged skill ups

`AddSkillUp(row)` stages one more rank on a skill row when the staged ups plus the rank are below the maximum and the staged
cost plus the cost of the next rank stays within the player's points (`+0xe4c`), and signals SKILL_LINES_CHANGED. `RemoveSkillUp(row)`
drops one staged rank and returns its cost. `GetAdjustedSkillPoints()` returns the points less the staged cost. `AcceptSkillUps()`
sends opcode 0x220 with the number of skills and each skill's id and staged count, then clears the staging; `CancelSkillUps()` clears it and signals.

Addresses: `0x4d3c30`, `0x4d3c67`, `0x4d32d0`, `0x4d3409`, `0x4d3413`, `0x4d344b`, `0x4d345b`, `0x4d3c70`, `0x4d3480`, `0x4d34cb`, `0x4d357c`, `0x4d358a`, `0x4d358d`, `0x4d3cb0`, `0x4d3d0f`, `0x4d3d40`, `0x4d3d58`, `0x4d3dbc`, `0x4d3dc2`, `0x4d3df3`, `0x4d3df8`, `0x4d3e30`, `0x4d35b0`, `0xb731d0`
Confidence: verified
Proof: one
Test: none

`0x4d3409` compares the rank plus staged ups with the maximum (`jge` leaves) and `0x4d344b` the staged cost plus the next cost with the player's points. `0x4d345b` increments the staged ups. `0x4d34cb` leaves for a row with none staged. `0x4d3d58` pushes 0x220 and `0x4d3dc2` writes each id and `0x4d3dce` its count. `0x4d35b0` clears every row's staged count and the cost.

## Buying a tier and abandoning

`BuySkillTier(row)` sends opcode 0x21f with the skill line id when the skill's tier value for the slot's step is nonzero and not above
the player's points, and the row has flag 8 or, with step 0, flag 4. `AbandonSkill(row)` sends opcode 0x202 with the row's skill line id for any row in the list. Both raise
`Usage: <name>(index)` for a non-number.

Addresses: `0x4d3e50`, `0x4d3f5f`, `0x4d3f62`, `0x4d3f76`, `0x4d3faf`, `0x4d3fbc`, `0x4d3fe4`, `0x4d3ab0`, `0x4d3b08`, `0x4d3b35`, `0x4d3b40`
Confidence: verified
Proof: one
Test: none

`0x4d3f62` tests row flag 8, `0x4d3f66` the step and `0x4d3f6e` flag 4. `0x4d3faf` compares the tier value with the points. `0x4d3fbc` pushes 0x21f and `0x4d3b08` pushes 0x202; both send through `ClientServices::Send`.

## Headers and the selected skill

`CollapseSkillHeader(row)` and `ExpandSkillHeader(row)` clear or set the expanded bit of the header of the row's category, or
the whole mask (0, or all ones) for a row number below 1, then re-sort. `SetSelectedSkill(row)` stores the row's skill id (0 for a header row or a row past the list) and
`GetSelectedSkill()` returns the 1-based position of that skill among the visible rows, 0 when none.

Addresses: `0x4d3b70`, `0x4d3ba7`, `0x4d3bbc`, `0x4d3bd0`, `0x4d3c07`, `0x4d3c1c`, `0x4d3210`, `0x4d3249`, `0x4d3270`, `0x4d32a9`, `0x4d4020`, `0x4d4065`, `0x4d406a`, `0x4d4073`, `0x4d4090`, `0x4d40b6`, `0xb731c4`, `0xb731c8`
Confidence: verified
Proof: one
Test: none

`0x4d3249` clears the header's bit and `0x4d32a9` sets it, each then calls `0x4d2ed0`. `0x4d40b6` compares each visible row's skill id with the selected id.

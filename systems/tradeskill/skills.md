# tradeskill: skills

The skills tab (`SkillInfo.cpp`): a list of the player's skills under category headers, its collapse and selection state, and the Lua functions that read it. The list is built from the 128 skill slots of the player object. Two groups of functions act on the server: abandoning a skill, and spending skill points on steps. The slot layout is `objectlayer/playerfields`.

Roots: none, the function rows of this file belong to `ui`

## List state

The skill list keeps a row array and a category array, each a capacity, a count and a data pointer, and seven words of state.

| address | holds |
|---|---|
| `0xb73184` | the row array: capacity, count `0xb73188`, data `0xb7318c` |
| `0xb73168` | the category array: capacity, count `0xb7316c`, data `0xb73170` |
| `0xb731b8` | the count of rows, headers included |
| `0xb731bc` | the count of categories |
| `0xb731c0` | the count of visible rows |
| `0xb731c4` | the selected skill line, 0 for none |
| `0xb731c8` | the collapse mask: a clear bit collapses the category at that position |
| `0xb731cc` | a flag that would hide untrained skills; nothing sets it |
| `0xb731d0` | the skill points the unaccepted skill-ups cost |

A row is `0x14` bytes: the skill line (0 for a header), the category, the visible flag, the untrained flag and the points chosen. A category is `0xc` bytes: the category id and its collapsed flag.

Addresses: `0xb73184`, `0xb7318c`, `0xb73188`, `0xb73168`, `0xb73170`, `0xb7316c`, `0xb731b8`, `0xb731bc`, `0xb731c0`, `0xb731c4`, `0xb731c8`, `0xb731cc`, `0xb731d0`, `0x4d2ab0`, `0x48f5e5`, `0x4d2b09`, `0x4d2b93`, `0x4d2b0e`, `0x4d2b98`, `0x4d2b3d`, `0x4d2b40`, `0x4d2c00`, `0x4911eb`, `0x4d2f74`
Confidence: verified
Proof: one
Test: none

`0x4d2ab0` sizes the arrays when the game starts (`0x48f5e5`): one row for each of the 128 skill slots plus one for each category, and one category record for each category. It allocates `0x14` and `0xc` bytes under the file's path (`0x4d2b09`, `0x4d2b93`; the sizes are pushed at `0x4d2b0e` and `0x4d2b98`), and starts every row with a skill line of 0, a category of 0 and visible
(`0x4d2b40`). `0x4d2c00` frees the records at shutdown (`0x4911eb`). `0x4d2f74` is the only read of `0xb731cc`.

## Rebuilding the list

`0x4d2cb0` rebuilds the list from the local player and does nothing for another player object. It resets the counts, the selection and the points, and expands every category. It walks the 128 skill slots and keeps a skill when its SkillLine row, its race-class row (`0x6ddf90`) and its category row exist and the race-class flags lack bit `0x2`. A skill with rank 0 stays only when flag bit `0x1` is set,
or when bit `0x4` is set and the player's level reaches the row's level (`+0x14`). Each kept skill becomes a row; the first skill of a category also adds the category and a header row. Last it orders the list and raises `SKILL_LINES_CHANGED`.

Addresses: `0x4d2cb0`, `0x4d2ce2`, `0x4d2cec`, `0x4d2cf1`, `0x4d2cf6`, `0x4d2cfb`, `0x4d2d04`, `0x4d2d11`, `0x4d2d26`, `0x4d2d60`, `0x4d2d9c`, `0x4d2da2`, `0x4d2da8`, `0x4d2dc5`, `0x4d2dd7`, `0x4d2e1c`, `0x4d2e46`, `0x4d2e64`, `0x4d2e83`, `0x4d2ea9`, `0x4d2eb7`, `0x4d2c90`, `0x4909f1`, `0x5de2a7`, `0x5de45c`, `0x5de49a`, `0x6ddf90`
Confidence: verified
Proof: one
Test: none

`0x4d2ce2` to `0x4d2cfb` reset the mask, the counts and the points. `0x4d2d04` to `0x4d2d26` read the skill word of the slot, only when the slot belongs to the local guid (`0x4d2d11`). `0x4d2d60` finds the race-class row. `0x4d2d9c` loads the flags, `0x4d2da2` drops a skill with bit `0x2`, `0x4d2da8` tests bit `0x1`, and `0x4d2dc5` and `0x4d2dd7` test bit `0x4` and the level. `0x4d2e1c` stores the untrained flag of the row (rank 0).
`0x4d2e46` to `0x4d2e83` add the category and the header row. The loop ends at `0x4d2ea9` after 128 slots (`0x600` bytes) and `0x4d2eb7` orders. The world entry (`0x4909f1` through `0x4d2c90`) and the skill value, maximum and temporary bonus watchers (`0x5de2a7`, `0x5de45c`, `0x5de49a`) call it.

## Order and visibility

`0x4d2ed0` orders the list and sets the visible flags. A category is collapsed when its bit in `0xb731c8` is clear, with the bit taken from its position in the category array. A header row is always visible; a skill row is visible unless its category is collapsed. The visible count starts as the row count and loses one for each hidden row. The categories sort by the sort field (`+0x28`) of the SkillLineCategory row, ascending.
The rows sort with `0x4d3070`: the visible rows first, then by category position, the header first in its category, trained skills before untrained ones, and the localized skill name case-blind.

Addresses: `0x4d2ed0`, `0x4d2ee2`, `0x4d2f0c`, `0x4d2f3e`, `0x4d2f74`, `0x4d2f8d`, `0x4d2fa0`, `0x4d2fc8`, `0x4d2fe2`, `0x4d2fec`, `0x4d3000`, `0x4d3038`, `0x4d303b`, `0x4d3070`, `0x4d3095`, `0x4d30cc`, `0x4d30f0`, `0x4d310f`, `0x4d311c`, `0x4d3130`, `0x4d318d`, `0x64a4c0`, `0x703e50`
Confidence: verified
Proof: one
Test: none

`0x4d2ee2` copies the row count to the visible count. `0x4d2f0c` writes each category's collapsed flag from the mask. `0x4d2f3e` marks a header row visible; `0x4d2f74` to `0x4d2f8d` decide a skill row; `0x4d2fa0` decrements the count. `0x4d2fc8` sorts the categories with `0x4d3000` and `0x4d2fe2` the rows. The category comparator reads the sort field at `0x4d3038` and `0x4d303b` and returns 1 for an equal pair in either order. In the row comparator,
`0x4d3095` orders by visibility, `0x4d30cc` finds the two category positions, `0x4d30f0` compares them, `0x4d310f` and `0x4d311c` put a header first, `0x4d3130` compares the untrained flags and `0x4d318d` the names (`0x64a4c0`). `0x4d2fec` raises event 265.

## Skill rows

`GetNumSkillLines` (`0x4d35e0`) returns the visible count. `GetSkillLineInfo` (`0x4d3610`) returns 12 values for a header row and 13 for a skill row. A header gives the category name, 1, 1 when the category is expanded or nil when it is collapsed, then four zeros, three nils and two zeros. A skill row gives the name, nil, nil, the rank, the points chosen, the modifier, the maximum, whether the skill can be abandoned,
the step cost, the rank cost, the level, the cost type plus 1 and the description. An index out of range gives the 13-value tuple nil, nil, nil, 0, 0, 0, 0, nil, nil, nil, 0, 0, nil, which includes index 0.

Addresses: `0x4d35e0`, `0x4d3610`, `0x4d3676`, `0x4d367a`, `0x4d369e`, `0x4d36d6`, `0x4d36e1`, `0x4d36f1`, `0x4d3768`, `0x4d37b3`, `0x4d3806`, `0x4d3828`, `0x4d3874`, `0x4d38b1`, `0x4d38bc`, `0x4d38db`, `0x4d392d`, `0x4d3943`, `0x4d3956`, `0x4d395d`, `0x4d3973`, `0x4d397d`, `0x4d398d`, `0x4d39c1`, `0x4d39e4`, `0x4d39f4`, `0x4d3a0b`, `0x4d3a2a`, `0x4d31c0`, `0x6de1c0`, `0x6de110`
Confidence: verified
Proof: one
Test: none

`0x4d3676` bounds the index against the row count and `0x4d367a` takes the failure path `0x4d3a2a`, which pushes the 13 values. `0x4d369e` handles a header: `0x4d36d6` pushes the category name and `0x4d36f1` asks `0x4d31c0` whether the category is collapsed. A skill row's rank (`0x4d3828`), maximum (`0x4d3874`) and modifier (`0x4d392d`) are words of the slot, local player only. The rank and maximum add the permanent bonus word when above 0.
`0x4d38b1` tests flag bit `0x400` of the race-class row: the maximum becomes 1 (`0x4d38bc`) and a rank above 1 becomes 1. `0x4d3956` and `0x4d395d` test the slot's step word (`+0x84a`) and flag bit `0x20`; the abandon flag is 1 only when both are set. `0x4d397d` tests flag bits `0x8` and `0x4`; `0x4d398d` calls the step value `0x6de1c0`, and `0x4d39c1` the rank cost `0x6de110` for the rank plus the points plus 1. `0x4d39f4` pushes the cost type plus 1 and `0x4d3a0b` the description.

## Selection

`SetSelectedSkill` (`0x4d4020`) stores the skill line of the row at the index, or 0 for a header, an index of 0 or below, or one past the last row; it checks no further. `GetSelectedSkill` (`0x4d4090`) returns the 1-based position of the first visible row whose skill line equals the selection, or 0 when none matches or none is selected.

Addresses: `0x4d4020`, `0xb731c4`, `0x4d405d`, `0x4d406a`, `0x4d4073`, `0x4d4090`, `0x4d409f`, `0x4d40b6`, `0x4d40bf`, `0x4d40c2`
Confidence: verified
Proof: one
Test: none

`0x4d405d` compares the index with the row count (`jae`), `0x4d406a` stores the row's skill line and `0x4d4073` stores 0. The scan of `0x4d4090` runs over the visible count only (`0x4d409f`), compares the skill line at `0x4d40b6`, and `0x4d40bf` converts "none" to -1 before the increment at `0x4d40c2`. A header row stores its skill line, which is 0.

## Collapse and expand

`CollapseSkillHeader` (`0x4d3b70`) and `ExpandSkillHeader` (`0x4d3bd0`) take a row index. An index of 0 or below sets the mask to 0 or to all ones; otherwise the category of the row at the index, header or skill, has its bit cleared or set. Both re-order the list, which raises `SKILL_LINES_CHANGED`; an index past the last row does nothing.

Addresses: `0x4d3b70`, `0x4d3ba7`, `0x4d3bb1`, `0x4d3bba`, `0x4d3bbc`, `0x4d3bd0`, `0x4d3c07`, `0x4d3c11`, `0x4d3c1a`, `0x4d3c1c`, `0x4d3210`, `0x4d3270`, `0x4d3259`, `0x4d32b9`
Confidence: verified
Proof: one
Test: none

`0x4d3ba7` and `0x4d3c07` store the two masks and call `0x4d2ed0` (`0x4d3bb1`, `0x4d3c11`). `0x4d3bbc` and `0x4d3c1c` call the two bit functions `0x4d3210` and `0x4d3270`, which return at once for an index past the row count, find the category of the row, change the bit of its position (`0x4d3259`, `0x4d32b9`) and jump to `0x4d2ed0`.

## Abandon

`AbandonSkill` (`0x4d3ab0`) sends message `0x202` with the skill line of the row at the index. It removes nothing locally and checks no flag; a header row sends the skill line 0, and an index of 0 or below (the unsigned compare `jae` at `0x4d3af7`) or past the last row sends nothing.

Addresses: `0x4d3ab0`, `0x4d3af5`, `0x4d3af7`, `0x4d3b08`, `0x4d3b2f`, `0x4d3b35`, `0x4d3b40`, `0x418190`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

`0x4d3af5` bounds the index against the row count, `0x4d3b08` pushes the opcode `0x202`, `0x4d3b2f` and `0x4d3b35` append the opcode and the skill line to the data store (`0x418190`), and `0x4d3b40` sends the message (`0x5ab630`). The Lua function returns no value.

## Skill points

`AddSkillUp` (`0x4d3c30`) adds one point to a skill when the rank plus the points chosen is below the maximum and the next rank's cost fits in the points left; `RemoveSkillUp` (`0x4d3c70`) takes one back. `GetAdjustedSkillPoints` (`0x4d3cb0`) returns the points (`+0xe4c`) minus those chosen. `CancelSkillUps` (`0x4d3e30`) clears every choice.
`AcceptSkillUps` (`0x4d3d40`) sends nothing when no points are chosen, and otherwise sends message `0x220`: a count, then the skill line and points of each visible row with points, and clears the choices. `BuySkillTier` (`0x4d3e50`) sends message `0x21f` with the skill line when the skill allows a step (flag bit `0x8`, or a step word of 0 with bit `0x4`) and the step value is nonzero and within the player's points.

Addresses: `0x4d3c30`, `0x4d3d46`, `0x4d32d0`, `0x4d3409`, `0x4d3413`, `0x4d344b`, `0x4d345b`, `0x4d3c70`, `0x4d3480`, `0x4d357a`, `0x4d358a`, `0x4d358d`, `0x4d3cb0`, `0x4d3d0f`, `0x4d3d40`, `0x4d3d58`, `0x4d3da0`, `0x4d3dbe`, `0x4d3dcb`, `0x4d3de8`, `0x4d3df3`, `0x4d3df8`, `0x4d3e30`, `0x4d35b0`, `0x4d3e50`, `0x4d3f5f`, `0x4d3f76`, `0x4d3faf`, `0x4d3fbc`, `0x4d3fef`, `0x418190`, `0x4181f0`, `0x417dc0`, `0x5ab630`, `0x6de110`, `0x6de1c0`
Confidence: verified
Proof: one
Test: none

`0x4d3d46` to `0x4d3d50` return without sending or clearing when the chosen cost `0xb731d0` is 0. `0x4d3409` loads the points chosen, `0x4d3413` asks `0x6de110` for the rank cost, `0x4d344b` tests the budget and `0x4d345b` raises the count. `0x4d3480` mirrors it: `0x4d357a` takes the cost of the current rank and `0x4d358a` and `0x4d358d` lower the count and the total. `0x4d3d0f` subtracts the total. `0x4d3d58` pushes `0x220`; the loop at `0x4d3da0` runs over the visible count and writes each pair (`0x4d3dbe`, `0x4d3dcb`),
so rows hidden by a collapsed category are not sent. `0x4d3de8` patches the count in place, `0x4d3df3` sends and `0x4d3df8` clears the choices through `0x4d35b0`. `0x4d3f5f` to `0x4d3f76` test the step flags and fetch the step value, `0x4d3faf` compares it with the points, `0x4d3fbc` pushes `0x21f` and `0x4d3fef` sends.

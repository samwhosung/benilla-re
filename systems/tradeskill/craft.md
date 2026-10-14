# tradeskill: craft

The Lua functions of the Craft window (`CraftInfo.cpp`): the window of Beast Training, Disguise and Enchanting. The list it reads is built and sorted in `ui/craft`. Most of these functions read a row of that list the way their profession-window twins in `bindings` do. A craft type is the window's kind: 1 Beast Training, 2 Disguise, 3 Enchanting.

Roots: none, the function rows of this file belong to `ui`

## Open, close and selection

`0x4f6000` toggles the window for a craft type: the type that is open closes it, and any other type is stored, the selection and the collapse mask reset, the list is built (`0x4f60c0`) and `CRAFT_SHOW` is raised. `CloseCraft` (`0x4f6c00`) sets the type to 0 and raises `CRAFT_CLOSE` through `0x4f5fe0`. `SelectCraft` (`0x4f70f0`) stores the spell id of the row, or 0 for a header or an index out of range, and raises nothing.
`GetCraftSelectionIndex` (`0x4f7130`) returns the position of the selected spell plus 1, so 0 for none. `GetNumCrafts` (`0x4f6e70`) returns the displayed count.

Addresses: `0x4f6000`, `0x4f6006`, `0x4f6017`, `0x4f601c`, `0x4f6022`, `0x4f602c`, `0x4f6036`, `0x4f6040`, `0x4f6c00`, `0x4f5fe0`, `0x4f5fef`, `0x4f70f0`, `0x4f6050`, `0x4f7130`, `0x4f6080`, `0x4f6e70`, `0xbdcfc8`, `0x4f7127`, `0x4f7137`, `0x4f60c0`
Confidence: verified
Proof: one
Test: none

`0x4f6006` compares the argument with the open type; `0x4f6017` raises the close event. `0x4f601c` to `0x4f602c` store the type, clear the selection and set the mask to all ones, `0x4f6036` builds the list and `0x4f6040` raises the show event. `0x4f5fef` raises `CRAFT_CLOSE`. The select function `0x4f6050` is called at `0x4f7127`, the index finder `0x4f6080` at `0x4f7137`.

## Name and skill line

`GetCraftName` (`0x4f6c10`) returns the localized name of the spell the player stores for the open craft type, or nil. `GetCraftButtonToken` (`0x4f6c90`) returns the word at the open type in the table `0x84d0f4` with no bound check. `GetCraftDisplaySkillLine` (`0x4f6cb0`) answers only for type 3: the name of the skill line of the first spell in the player's list of that type, the player's rank in the line
and the maximum, each plus the permanent bonus when above 0; any failure gives nil, 0 and 0. `GetCraftSkillLine` (`0x4f7210`) returns the name of the skill line in the group field (`+4`) of the row, or nil.

Addresses: `0x4f6c10`, `0x4f6c3b`, `0x4f6c5f`, `0x4f6c90`, `0x84d0f4`, `0x4f6cb0`, `0x4f6ce3`, `0x4f6cf4`, `0x4f6d14`, `0x4f6d21`, `0x4f6d52`, `0x4f6d61`, `0x4f6d99`, `0x4f6da4`, `0x4f6e08`, `0x4f6e17`, `0x4f6e40`, `0x4f7210`, `0x4f725c`, `0x4f727e`, `0x5ea360`, `0x5ea330`, `0x60c7c0`, `0x5ea3f0`
Confidence: verified
Proof: one
Test: none

`0x4f6c3b` asks the player for the spell stored for the open type and `0x4f6c5f` reads the spell's name. The table `0x84d0f4` holds `USE`, `TRAIN`, `DISGUISE` and `ENSCRIBE`. `0x4f6ce3` fails unless the type is 3; `0x4f6cf4` fetches the list, `0x4f6d14` finds the ability row of its first spell and `0x4f6d21` takes its skill line (`+4`). `0x4f6d52` finds the slot, `0x4f6d99` reads the rank word and `0x4f6da4` adds the permanent bonus; `0x4f6e08` and `0x4f6e17` do so for the maximum.
A player with no slot for the line gives the slot -1 (`0x4f6d52`), which `0x4f6d99` and `0x4f6da4` do not test, so the rank and the bonus are read from the words 12 bytes below the first slot.
`0x4f6e40` is the failure triple. `0x4f725c` reads the row's group field and `0x4f727e` the line's name.

## Craft rows

`GetCraftInfo` (`0x4f6e90`) returns seven values for an index. A header row gives the skill line name, nil, `header`, 0, 1 when the group is not collapsed or nil when it is, 0 and 0. A recipe row gives the spell name, its rank text, the difficulty word of the row (`none`, `optimal`, `medium`, `easy`, `trivial` or `used` from the table `0x807e00`), the available count, nil, and a cost and a required level.
An index out of range gives three nils, then 0, nil, 0 and 0.

Addresses: `0x4f6e90`, `0x4f6f2c`, `0x4f6f38`, `0x4f6f4f`, `0x4f6f56`, `0x4f6f60`, `0x4f6f9b`, `0x4f6fae`, `0x4f6fbc`, `0x4f6fcf`, `0x4f6fdf`, `0x4f70a4`, `0x4f70c6`, `0x4f70cb`, `0x4f6b80`, `0x807e00`, `0x846f64`
Confidence: verified
Proof: one
Test: none

`0x4f6f2c` pushes the header's name, `0x4f6f38` the word `header` (`0x846f64`) and `0x4f6f4f` asks `0x4f6b80` whether the group is collapsed. `0x4f6f9b` and `0x4f6fae` push the spell name (`+0x1e0`) and the rank text (`+0x204`). `0x4f6fbc` indexes the 32-byte entries of `0x807e00` by the row's bucket (`+0xc`) and `0x4f6fcf` pushes the available count (`+8`). `0x4f6fdf` pushes the nil. `0x4f70a4` is the failure path.
The cost and the level are in the next entry.

## Craft difficulty

A craft row's difficulty bucket comes from the ability row of the spell for the local player (`0x60c7c0`), with high at `+0x28` and low at `+0x2c`. A missing row or a high of 0 leaves it 0. A zero low becomes high - 25, and mid is (high + low) / 2 truncated toward zero. The player's skill total in the line (`0x5ea520`) gives 1 below low, 2 below mid, 3 below high and 4 otherwise. A row still at 0 becomes 5, `used`, when the first learn effect of the spell names a spell the player knows, or, for an effect aimed at the pet, one the pet knows or has a higher rank of.

Addresses: `0x4f61a8`, `0x4f61dd`, `0x4f61ea`, `0x4f61f5`, `0x4f61fc`, `0x4f6203`, `0x4f620e`, `0x4f6215`, `0x4f6222`, `0x4f6233`, `0x4f623f`, `0x4f6248`, `0x4f6255`, `0x4f6267`, `0x4f62f7`, `0x4f6310`, `0x4f6323`, `0x4f6347`, `0x4f6360`, `0x4f6369`, `0x4f6389`, `0x4f63a1`, `0x4f63b7`, `0x4f63cc`, `0x5ea520`, `0x60c740`, `0x60c8d0`
Confidence: verified
Proof: one
Test: none

`0x4f61a8` finds the ability row, `0x4f61dd` clears the bucket and `0x4f61ea` skips a zero high. `0x4f61fc` to `0x4f6203` keep high - 25 whenever it is not zero (`sbb`), which is negative when high is below 25. `0x4f620e` reads the skill total: the rank, plus the permanent bonus when the rank is above 0, plus the signed modifier, never below 0. The signed compares at `0x4f6215`, `0x4f6233` and `0x4f6248` choose the bucket stored at `0x4f6222`, `0x4f623f`, `0x4f6255` and `0x4f6267`. `0x4f62f7` leaves a row whose bucket is not 0.
`0x4f6310` finds the first effect 36; `0x4f6323` tests its two target fields (`+0x148`, `+0x154`) for 5. `0x4f6347` asks whether the player knows the learned spell and `0x4f6360` stores 5. For a pet target `0x4f6369` to `0x4f6389` fetch the unit whose guid the player's unit fields hold at `+8`, `0x4f63a1` and `0x4f63b7` ask for the spell or a higher rank, and `0x4f63cc` stores 5. The profession window reads the raw rank instead (`recipes`).

## Training cost and level

For a learn effect the last two values of `GetCraftInfo` come from the pet. In each of the three effect slots, an effect 36 aimed at the pet (target field `+0x54` equal to 5) or an effect 57 names a learned spell (`+0xc0`); the sixth value is `+0x38` of that spell's ability row for the pet unit and the seventh is the level `0x60c840` gives, the greater of the spell's own level (`+0x74`) and the ability's required level.
The last slot that has such an effect wins. With no pet, or no such effect, both are 0.

Addresses: `0x4f6fe6`, `0x4f6fff`, `0x4f7010`, `0x4f701c`, `0x4f7027`, `0x4f702e`, `0x4f7039`, `0x4f7042`, `0x4f704b`, `0x4f705f`, `0x4f7078`, `0x4f7088`, `0x60c7c0`, `0x60c840`, `0xb714a0`, `0x4f7006`
Confidence: verified
Proof: one
Test: none

`0x4f6fe6` loads the pet guid (`0xb714a0`) and `0x4f6fff` to `0x4f7010` fetch the unit of type 8 with the path `0x84d368` and line `0x22b`. `0x4f7027` loops the three effects from `+0xf4`; `0x4f702e` and `0x4f7034` test effect 36 with target 5 and effect 57. `0x4f7042` finds the ability row and `0x4f704b` takes `+0x38`; `0x4f705f` asks `0x60c840` for the required level. `0x4f7078` and `0x4f7088` push the two numbers.

## Reagents and links

`GetCraftNumReagents` (`0x4f7420`), `GetCraftReagentInfo` (`0x4f74d0`) and `GetCraftReagentItemLink` (`0x4f7730`) run the same code as `GetTradeSkillNumReagents`, `GetTradeSkillReagentInfo` and `GetTradeSkillReagentItemLink` of `bindings` on the craft rows; the item request of the info function carries the callback `0x4f7710`, which rebuilds the craft list. `GetCraftItemLink` (`0x4f72a0`) returns an enchant link for a spell whose cast-UI field (`+0xc`) is 3:
`|cffffffff|Henchant:<spell id>|h[<spell name>]|h|r`. For any other spell it scans the three effect slots for effect 24 and links the first created item whose template is cached, in the form of the item links of `bindings`; with none it returns no value.

Addresses: `0x4f7420`, `0x4f74d0`, `0x4f7730`, `0x4f7710`, `0x4f72a0`, `0x4f7330`, `0x4f7347`, `0x4f7386`, `0x4f73a4`, `0x4f73f6`, `0x4f60c0`, `0x84d410`, `0x4ff510`, `0x4ff5c0`, `0x4ff800`
Confidence: verified
Proof: one
Test: none

The three reagent functions match their twins instruction for instruction once addresses are masked, apart from the arrays (`0xbdcf78`, `0xbdcfc0`) and the callback. `0x4f7710` calls `0x4f60c0` when the record arrives. `0x4f7330` compares the cast-UI field with 3 and `0x4f7347` pushes the format `0x84d410`. `0x4f7386` tests each effect slot for effect 24 (`0x18`); `0x4f73a4` requests the template with no callback and `0x4f73f6` builds the link.

## Spell focus and description

`GetCraftSpellFocus` (`0x4f78b0`) returns the same flat list as `GetTradeSkillTools` of `bindings`: the spell focus object with the constant flag 1, then each totem item with a cached template and the flag 1 or nil, with the callback `0x4f7710`; it returns twice the pair count. `GetCraftDescription` (`0x4f7a90`) returns the description of the spell (`+0x228`) with its dollar tokens expanded
when it is not empty. When it is empty, it uses the description of the first learned spell of a learn effect (36 or 57) and, with none, returns nil. The expansion takes the pet flag from the craft type for the spell itself and from the effect for the learned spell.

Addresses: `0x4f78b0`, `0x4f7994`, `0x4f79b1`, `0x4f79e5`, `0x4f7a0a`, `0x4f7a3c`, `0x4f7a85`, `0x4f7a90`, `0x4f7b1b`, `0x4f7b21`, `0x4f7b31`, `0x4f7b53`, `0x4f7b71`, `0x4f7b82`, `0x4f7b87`, `0x4f7bc1`, `0x4f7be6`, `0x5075f0`, `0x4f7bae`
Confidence: verified
Proof: one
Test: none

`0x4f7994` to `0x4f79b1` push the focus name and the immediate 1. `0x4f79e5` starts the totem loop, `0x4f7a0a` requests the item with the callback, `0x4f7a3c` asks the inventory and `0x4f7a85` doubles the pair count. `0x4f7b21` loads the description and `0x4f7b31` loads the craft type; `0x4f7b53` expands the text (`0x5075f0`) with the pet flag set for type 1. `0x4f7b71` scans the effects for 36 or 57 (`0x4f7b82`, `0x4f7b87`);
`0x4f7bc1` loads the effect and `0x4f7be6` expands with the pet flag set for effect 57. `0x4f7bae` pushes nil when nothing is found.

## Collapse and cast

`CollapseCraftSkillLine` (`0x4f7c10`) and `ExpandCraftSkillLine` (`0x4f7c90`) take the index of a header row. An index of 0 or below collapses or expands every group by setting the collapse mask `0x84d0f0` to 0 or to all ones. Otherwise the row must be a header row, and its group's bit is cleared or set; any other row raises `Bad skill line in CollapseCraftSkillLine` or `Bad skill line in ExpandCraftSkillLine`.
A clear bit means the group is collapsed. Each change re-sorts the list and raises `CRAFT_UPDATE`. `DoCraft` (`0x4f7d10`) casts the spell of the row once with the active player as caster, with no repeat and no check of the row type.

Addresses: `0x4f7c10`, `0x4f7c47`, `0x4f7c49`, `0x4f7c54`, `0x4f7c5b`, `0x4f7c71`, `0x4f7c83`, `0x4f7c90`, `0x4f7cc7`, `0x4f7cca`, `0x4f7cd5`, `0x4f7cdc`, `0x4f7cf2`, `0x4f7d02`, `0x4f7d10`, `0x4f7d64`, `0x4f6b30`, `0x4f6be0`, `0x4f6630`, `0x4f6bf0`, `0x84d0f0`, `0x6e5a90`
Confidence: verified
Proof: one
Test: none

`0x4f7c47` stores mask 0 through `0x4f6be0` and `0x4f7cc7` all ones. `0x4f7c54` and `0x4f7cd5` call `0x4f6b30`, which returns the header position of a header row and -1 otherwise; `0x4f7c5b` and `0x4f7cdc` raise on a negative result. `0x4f7c71` to `0x4f7c83` clear the bit and `0x4f7cf2` to `0x4f7d02` set it. `0x4f6be0` stores the mask, calls the recount and sort `0x4f6630` and raises the update at `0x4f6bf0`.
`0x4f7d64` calls the cast with the spell id, a zero item and a zero target.

# tradeskill: overview

The client side of the profession windows. Three object files of the interface hold it: `TradeSkillFrame.cpp`, the window every
profession opens except Beast Training, Disguise and Enchanting; `CraftInfo.cpp`, the window for those three; and `SkillInfo.cpp`,
the skills tab. The windows are Lua frames. The engine builds their lists from the player's known spells and answers the Lua
functions that read the lists and the few that act on them. Opening a window sends nothing, a craft is one cast, and an unlearn is
one message.

The docs split by topic: `recipes` (the profession list: records, difficulty, available counts, filters, order), `bindings` (the
profession window's Lua functions), `itemstats` (the lines of its item tooltip), `cast` (DoTradeSkill and the repeat count), `craft`
(the Craft window's Lua functions) and `skills` (the skill list). This doc holds the object files, their static objects and the events.
The function rows of these files belong to `ui`; the docs cite them by address.

Roots: none, the function rows of these files belong to `ui`

## Object files

Each file opens its run in the initializer table with a +∞ thunk, and the next file opens with its own. All three runs have 26 slots.

| file | extent | path | run slots |
|---|---|---|---|
| `TradeSkillFrame.cpp` | `0x4fc650` to the end of `0x5009c0` | `0x84e270` | `0x829428` to `0x82948c` |
| `CraftInfo.cpp` | `0x4f5de0` to the end of `0x4f8420` | `0x84d368` | `0x8292a0` to `0x829304` |
| `SkillInfo.cpp` | `0x4d29b0` to the end of `0x4d46c0` | `0x849e84` | `0x828b98` to `0x828bfc` |

Addresses: `0x4fc650`, `0x5009c0`, `0x84e270`, `0x829428`, `0x82948c`, `0x4f5de0`, `0x4f8420`, `0x84d368`, `0x8292a0`, `0x829304`, `0x4d29b0`, `0x4d46c0`, `0x849e84`, `0x828b98`, `0x828bfc`, `0x827000`, `0x4fcb57`, `0x4fcdc7`, `0x4fcfd7`, `0x4f7006`, `0x4d2b09`, `0x5009e0`, `0x4f8440`, `0x4d46f0`, `0x829490`, `0x829308`, `0x828c00`
Confidence: verified
Proof: one
Test: none

The path of each file is passed to the allocator by its list builders: `0x4fcb57`, `0x4fcdc7` and `0x4fcfd7` for the profession list, `0x4f7006` for the craft
list and `0x4d2b09` for the skill list. The next file in each run begins with its +∞ thunk: `0x5009e0` (slot `0x829490`), `0x4f8440` (slot `0x829308`) and
`0x4d46f0` (slot `0x828c00`). The three extents hold the code, the registration and unregistration functions of the Lua tables and the
static initializers; the static initializers of `CraftInfo.cpp` and `TradeSkillFrame.cpp` sit between and after their code, and those of `SkillInfo.cpp`
follow its last Lua function.

## Static objects

Each run holds the constants every file takes from the shared headers (`models/overview#shared-header-constants`): +∞, 2π, 1/2π, the use count of a shared
object, the squares 10², 11.111111², 5.5555553², 10², 5², 10², 30², 120², 20², 10², 0.5², 3² and 100², the float 5.0, then 50², 100², 40² and 1.5², then 10² and
0.5/10. The last two slots build the file's two empty lists, a row array and a group array. The use count slot adds 1 to `0xc2a3c4` and calls `0x5b71c0` on the first user; its exit
function subtracts 1 and calls `0x5b71d0` on the last.

Addresses: `0x4fc660`, `0x4ff3d0`, `0x4ff3f0`, `0x500370`, `0x500380`, `0x5003a0`, `0x5003b0`, `0x5003d0`, `0x5006f0`, `0x500990`, `0x5009c0`, `0x4fc670`, `0x4fc6e0`, `0x4f5de0`, `0x4f7dd0`, `0x4f7df0`, `0x4f7e10`, `0x4f7e20`, `0x4f7e50`, `0x4f5e00`, `0x4f5e70`, `0x4d29b0`, `0x4d4130`, `0x4d4150`, `0x4d42b0`, `0x4d29d0`, `0x4d2a40`, `0x5b71c0`, `0x5b71d0`, `0xc2a3c4`, `0x4d29c0`, `0x4d29e0`, `0x4d2a00`, `0x4d2a10`, `0x4d2a50`, `0x4d2a70`, `0x4d2a80`, `0x4d4120`, `0x4d4140`, `0x4d42c0`, `0x4d42e0`, `0x4d42f0`, `0x4d4300`, `0x4d4310`, `0x4d4330`, `0x4d4340`, `0x4d4360`, `0x4d4370`, `0x4d4390`, `0x4d43a0`, `0x4d43c0`, `0x4d43d0`, `0x4d43f0`, `0x4d4400`, `0x4d4420`, `0x4d4430`, `0x4d4450`, `0x4d4460`, `0x4d4480`, `0x4d4490`, `0x4d44b0`, `0x4d44c0`, `0x4d44e0`, `0x4d44f0`, `0x4d4510`, `0x4d4520`, `0x4d4540`, `0x4d4550`, `0x4d4570`, `0x4d4580`, `0x4d4590`, `0x4d45a0`, `0x4d45c0`, `0x4d45d0`, `0x4d45f0`, `0x4d4600`, `0x4d4620`, `0x4d4630`, `0x4d4690`, `0x4d46a0`, `0x4d46c0`, `0x4d46d0`, `0x4f5df0`, `0x4f5e10`, `0x4f5e30`, `0x4f5e40`, `0x4f5e80`, `0x4f5ea0`, `0x4f5eb0`, `0x4f7dc0`, `0x4f7de0`, `0x4f7e40`, `0x4f7e60`, `0x4f7e70`, `0x4f7f80`, `0x4f7f90`, `0x4f7fb0`, `0x4f7fc0`, `0x4f7fe0`, `0x4f7ff0`, `0x4f8010`, `0x4f8020`, `0x4f8040`, `0x4f8050`, `0x4f8070`, `0x4f8080`, `0x4f80a0`, `0x4f80b0`, `0x4f80d0`, `0x4f80e0`, `0x4f8100`, `0x4f8110`, `0x4f8130`, `0x4f8140`, `0x4f8160`, `0x4f8170`, `0x4f8190`, `0x4f81a0`, `0x4f81c0`, `0x4f81d0`, `0x4f81e0`, `0x4f81f0`, `0x4f8210`, `0x4f8220`, `0x4f8240`, `0x4f8250`, `0x4f8270`, `0x4f8280`, `0x4f83e0`, `0x4f83f0`, `0x4f8410`, `0x4f8420`, `0x4fc650`, `0x4fc680`, `0x4fc6a0`, `0x4fc6b0`, `0x4fc6f0`, `0x4fc710`, `0x4fc720`, `0x4ff3c0`, `0x4ff3e0`, `0x5003c0`, `0x5003f0`, `0x500400`, `0x500420`, `0x500430`, `0x500450`, `0x500460`, `0x500480`, `0x500490`, `0x5004b0`, `0x5004c0`, `0x5004e0`, `0x5004f0`, `0x500510`, `0x500520`, `0x500540`, `0x500550`, `0x500570`, `0x500580`, `0x5005a0`, `0x5005b0`, `0x5005d0`, `0x5005e0`, `0x500600`, `0x500610`, `0x500630`, `0x500640`, `0x500650`, `0x500660`, `0x500680`, `0x500690`, `0x5006b0`, `0x5006c0`, `0x5006e0`, `0x500980`, `0x5009b0`
Confidence: verified
Proof: one
Test: none

The +∞ slot of `TradeSkillFrame.cpp` runs `0x4fc660`, which copies +∞ into `0xbddfd8`; nothing reads that global. The 2π and 1/2π slots are `0x4ff3d0` and `0x4ff3f0`, in
the middle of the Lua functions. The use count is `0x500370` with the body `0x500380`, the exit registration `0x5003a0` and the exit function `0x5003b0`. The squares run from `0x5003d0` to
`0x5006f0`, and the extra 10² and 0.5/10 are `0x500990` and `0x5009c0`. `CraftInfo.cpp` has the same slots with its use count at `0x4f7e10` (body `0x4f7e20`, exit function `0x4f7e50`) and its arrays at `0x4f5e00` and `0x4f5e70`; `SkillInfo.cpp`
has its use count at `0x4d42b0` and its arrays at `0x4d29d0` and `0x4d2a40`.

## Lua function tables

Each file registers its Lua functions from a table of name and handler pairs. `TradeSkillFrame.cpp` has 26 pairs at `0x84ddd0`, registered by `0x500320` and unregistered
by `0x500350`; `CraftInfo.cpp` has 19 at `0x84d108` (`0x4f7d70` registers, `0x4f7da0` unregisters); `SkillInfo.cpp` has 13 at `0x849cf8` (`0x4d40e0`, `0x4d4100`). The game interface
calls the registrations at `0x4902f6`, `0x490319` and `0x490341` and the unregistrations at `0x490d8a`, `0x490dad` and `0x490dd0`. A handler takes the Lua state and returns the count of values it pushed.
A handler with arguments tests the first with `0x6f34d0` and, on a miss, pushes its usage string and calls the raising function `0x6f4940`.

Addresses: `0x84ddd0`, `0x500320`, `0x500350`, `0x84d108`, `0x4f7d70`, `0x4f7da0`, `0x849cf8`, `0x4d40e0`, `0x4d4100`, `0x4902f6`, `0x490319`, `0x490341`, `0x490d8a`, `0x490dad`, `0x490dd0`, `0x6f4940`, `0x6f34d0`, `0x500323`, `0x4f7d73`, `0x4d40e3`, `0x500337`, `0x4f7d87`, `0x4d40f7`, `0x704120`
Confidence: verified
Proof: one
Test: none

Each registration loop steps eight bytes through its table (`0x500323`, `0x4f7d73`, `0x4d40e3`) and calls `0x704120` with the name and the handler; the bounds are `0xd0`, `0x98` and `0x68`
bytes (`0x500337`, `0x4f7d87`, `0x4d40f7`). The usage string of a handler names the handler in all but two cases: `GetCraftIcon` pushes `Usage: GetTradeSkillIcon(index)`, and
`GetTradeSkillReagentItemLink` pushes `Usage: GetTradeReagentSkillItemLink(index, reagentIndex)`.

## Events

The windows tell the interface about changes through the events of `tables/events.tsv`.

| event | id | raised by |
|---|---|---|
| `TRADE_SKILL_SHOW` | 313 | `0x4fc99f`, an open that finds recipes |
| `TRADE_SKILL_UPDATE` | 314 | `0x4fd164` and the filter setters `0x4fd725`, `0x4fd745`, `0x4fd765` |
| `TRADE_SKILL_CLOSE` | 315 | `0x4fc884` and `0x4fc930` |
| `UPDATE_TRADESKILL_RECAST` | 502 | `0x4fd781`, `0x4fd7a8`, `0x4fd7e1` |
| `CRAFT_SHOW` | 359 | `0x4f6040` |
| `CRAFT_UPDATE` | 360 | `0x4f661a`, `0x4f6bf0` |
| `CRAFT_CLOSE` | 361 | `0x4f5fef`, `0x4f6017` |
| `SKILL_LINES_CHANGED` | 265 | `0x4d2ff2`, `0x4d346f`, `0x4d3598`, `0x4d3e3a` |

Addresses: `0x4fc99f`, `0x4fd164`, `0x4fd725`, `0x4fd745`, `0x4fd765`, `0x4fc884`, `0x4fc930`, `0x4fd781`, `0x4fd7a8`, `0x4fd7e1`, `0x4f6040`, `0x4f661a`, `0x4f6bf0`, `0x4f5fef`, `0x4f6017`, `0x4d2ff2`, `0x4d346f`, `0x4d3598`, `0x4d3e3a`, `0x703e50`
Confidence: verified
Proof: one
Test: none

Each site loads the event id into `ecx` and calls or jumps to the signal function `0x703e50`; `tables/event_sites.tsv` lists them. A list rebuild raises UPDATE and an open raises SHOW only when the list has rows. Nothing in these files raises an event for a cast result: the
spell code raises RECAST through the functions of `cast`.

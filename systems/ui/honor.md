# ui: the PvP honor functions

The Lua functions that report a player's PvP standing: the kill and honor figures (`GetPVPSessionStats` and four more),
the rank functions (`UnitPVPRank`, `GetPVPRankInfo`, `GetPVPRankProgress`), the inspected player's bar and the name
with its honor title (`UnitPVPName`). They are `ScriptEvents.cpp` bindings, `0x51a4b0` to `0x51ab30` and `0x5172b0`. What
is cached for an inspected player is `ui/honorinspect`; the credit message and the rank badges are `ui/honorcredit`.

Roots: `0x51a4b0`, `0x51a550`, `0x51a620`, `0x51a6c0`, `0x51a7c0`, `0x51a8a0`, `0x51a930`, `0x51aa90`, `0x51ab00`, `0x5172b0`

## Honor fields

Each honor function reads the player block, the pointer at `+0xe68` of a player object. A pair is two 16-bit halves,
read low half first; every figure widens unsigned into a Lua number. `UnitPVPRank` reads the named unit's own block;
the stat functions and `GetPVPRankProgress` read the active player's.

| player block offset | read as |
|---|---|
| `+0x1f` | the current honor rank, a byte |
| `+0x102b` | the highest honor rank, a byte |
| `+0x1098` | session pair |
| `+0x109c` | yesterday pair |
| `+0x10a0` | last-week pair |
| `+0x10a4` | this-week figure, low half only |
| `+0x10a8`, `+0x10b4`, `+0x10b8` | this-week, yesterday and last-week contribution |
| `+0x10ac`, `+0x10b0` | the two lifetime counts, 32 bits each |
| `+0x10bc` | last-week standing |
| `+0x10c0` | the rank bar byte |

Addresses: `0x51a4df`, `0x51a4ff`, `0x51a581`, `0x51a5a1`, `0x51a5c1`, `0x51a651`, `0x51a671`, `0x51a6f5`, `0x51a715`, `0x51a735`, `0x51a75b`, `0x51a7f1`, `0x51a817`, `0x51a83d`, `0x51a8f5`, `0x51aabc`
Confidence: verified
Proof: one
Test: none

The halves load with `movzx` and `fild` of a dword; a 32-bit value is stored with a zero upper dword and loaded with
`fild` of a qword (`0x51a5c7` to `0x51a5d1`), so it is unsigned. No memory operand in the image names the displacement
`0x10a6`, the high half of the this-week figure. The two rank bytes are different fields: the current rank is public
and `+0x102b` is the lifetime best.

## Absent player results

When the active player object is not found, the stat functions and `GetPVPRankProgress` push the number 0 once for each
value they return and return that count: 2, 3, 2, 4 and 3 for the five stat functions, 1 for the progress. None returns
nil. The player is the object of the active player GUID under type mask 0x10.

Addresses: `0x51a524`, `0x51a5ec`, `0x51a69c`, `0x51a786`, `0x51a869`, `0x51aae6`, `0x468550`, `0x468460`
Confidence: verified
Proof: one
Test: none

Each function starts with the same lookup (`push 0x90`, `call 0x468550`, then `0x468460` with `ecx = 0x10`) and branches
to its absent tail when the result is null (`0x51a4d7`, `0x51a579`, `0x51a649`, `0x51a6e9`, `0x51a7e9`, `0x51aab4`). The
tail pushes `0` twice per value (`0x51a524` to `0x51a535`). `0x51a869` falls into the push at `0x51a881` that a rank below
5 also uses.

## PvP stat functions

`GetPVPSessionStats()` returns the session pair. `GetPVPYesterdayStats()` returns the yesterday pair and yesterday
contribution. `GetPVPThisWeekStats()` returns the this-week figure and this-week contribution: two values, never three.
`GetPVPLastWeekStats()` returns the last-week pair, last-week contribution and last-week standing.
`GetPVPLifetimeStats()` returns the two lifetime counts and the highest rank, which it replaces by 0 when below 5.

Addresses: `0x51a4b0`, `0x51a550`, `0x51a620`, `0x51a6c0`, `0x51a7c0`, `0x51a843`, `0x51a847`, `0x51a849`, `0x51a881`
Confidence: verified
Proof: one
Test: none

The order of pushes is the order of the loads in each function. The lifetime rank test is `cmp al,5; jb` (`0x51a843`,
`0x51a847`), unsigned, jumping to the shared zero push; a rank of 1 to 4, the four lowest titles, therefore reads 0. The
functions take no arguments and do not check the count.

## Unit PvP rank

`UnitPVPRank(unit)` returns the honor rank of a player unit. A non-string argument raises
`Usage: UnitPVPRank("unit")`. The token is resolved to a unit object; for a player object the result is the byte at
`+0x1f` of that object's own block, for anything else (a creature, an unresolvable token) the number 0, never nil. It is
the current rank, not the highest.

Addresses: `0x51a8a0`, `0x51a8b1`, `0x51a8d8`, `0x51a8dd`, `0x51a8e1`, `0x51a8ed`, `0x51a8f5`, `0x51a916`, `0x515940`, `0x515970`
Confidence: verified
Proof: one
Test: none

`0x515940` looks the GUID of the token up (`0x515970`) and fetches the unit by type mask 8 (`0x515957`). The player test
shifts the object's type word right 4 and tests bit 0 (`0x51a8e7`, `0x51a8ea`). The function does not cache the rank
and has no case for the local player.

## PvP rank info

`GetPVPRankInfo(rank [, unit-or-side])` returns the title of a rank and its badge number. A non-number rank raises
`Usage: GetPVPRankInfo(rank [, unit])`. A rank outside 1 to 18 returns `nil, 0`. The side is a number, used as given, or a
unit token whose player's side is taken (0 for a token that is not a player), or the active player's side when the argument
is absent. The title is the text `PVP_RANK_<rank>_<side>`; a missing or empty text returns `nil, 0`. The badge number is
rank minus 5 for ranks 1 to 4 and rank minus 4 for 5 to 18.

Addresses: `0x51a930`, `0x51a940`, `0x51a96c`, `0x51a97c`, `0x51a998`, `0x51a9af`, `0x51a9c4`, `0x51a9e9`, `0x51a9ee`, `0x51a9f0`, `0x51a9f5`, `0x51aa07`, `0x51aa17`, `0x51aa1c`, `0x51aa20`, `0x51aa31`, `0x51aa38`, `0x51aa55`, `0x51aa72`, `0x5efe00`, `0x703bf0`
Confidence: verified
Proof: one
Test: none

The rank is truncated by `__ftol` (`0x51a96c`) and bounded signed: `cmp edi,1; jl` and `cmp edi,0x12; jg`
(`0x51a9f0`, `0x51a9f5`), so rank 19, the leader rank the credit message can carry, is refused here. A numeric second
argument skips unit resolution (`0x51a98c`, `0x51a996` to `0x51a9ee`); a string is a token (`0x51a9af`); `0x5efe00` gives
0, 1 or -1 (`objectlayer/interaction#other-units`), and -1 forms a key no text matches. The key is `snprintf` into 0x20
bytes (`0x51aa07`) and looked up as a script text with gender 0 (`0x51aa0f push 0`), the default form for every unit, so
the title never takes the female variant here. Ranks 1 to 4 give badge numbers -4 to -1 (`0x51aa38`).

## Rank progress

`GetPVPRankProgress()` returns the rank bar byte of the active player times the float constant 0.003921569 (bits
0x3b808081), and 0 for an absent player. `GetInspectPVPRankProgress()` does the same to the byte cached from the inspect
reply, with no player lookup. Neither clamps. The product is not the byte over 255.

Addresses: `0x51aa90`, `0x51aabc`, `0x51aace`, `0x51ab00`, `0x51ab04`, `0x51ab14`
Confidence: verified
Proof: one
Test: `ui::luabind_51aa90_get_pvp_rank_progress`, `ui::luabind_51ab00_get_inspect_pvp_rank_progress`

Both bodies run `fild` of the byte, `fmul` by the float constant (`0x51aace`, `0x51ab14`) and push the result as
a double. The constant is the float `0x3b808081`, which is not exactly 1/255, so a division gives different results for
all but byte 0. `GetInspectPVPRankProgress` reads the cell `0xb71f84` (`0x51ab04`), 0 until a reply arrives and the old
value after the target changes until the next reply (`ui/honorinspect#inspect-honor-slot`).

Bit-exact coverage: `ui::luabind_51aa90_get_pvp_rank_progress` covers `0x51aac6` to `0x51aad7` of `0x51aa90`; `ui::luabind_51ab00_get_inspect_pvp_rank_progress` covers `0x51aace` to `0x51aad7` of `0x51aa90`; `ui::luabind_51ab00_get_inspect_pvp_rank_progress` covers `0x51ab0e` to `0x51ab1d` of `0x51ab00`.

## PvP name

`UnitPVPName(unit)` returns a unit's name with its honor title: the object's name built by the decorator `0x609370` with
the title flag on. A non-string argument raises `Usage: UnitPVPName("unit")`. A token that resolves to a GUID with no
object in the manager returns nil. A token that resolves to no GUID pushes nothing and returns 1, which makes the result
the top of the stack, the last argument.

Addresses: `0x5172b0`, `0x5172c1`, `0x5172ed`, `0x5172f6`, `0x517309`, `0x517324`, `0x517343`, `0x51733c`, `0x609370`, `0x6f61aa`, `0x6f61b4`
Confidence: verified
Proof: one
Test: none

`0x5172f6 or ecx,edx; je 0x517343` skips the `lua_pushnil` at `0x51733c` and loads the result count 1 (`0x517343`). The
VM takes the results as the top count values: after the call it computes `L->top - count * 16` (`0x6f61aa shl eax,4`,
`0x6f61b4 sub eax,edx`). Nothing was pushed, so the one result is the argument. The decorator and its rank text are
`objectlayer/unitnames#unit-names`.

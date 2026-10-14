# ui: group, guild, raid and instance verbs

The Lua functions of `GameUI.cpp` and `RaidInfo.cpp` that invite, remove and promote people, answer invitations, and reset
or inspect instances, plus the small verbs that send one message: the play time request, the random roll, standing and
sitting, the PvP toggle and the cinematic switches. Each is a thin call: it checks its argument, resolves a token or a
name, and sends or calls once. The wire records are in `objectlayer/party`, the raid roster in `ui/raid`, and the name
match behind the `ByName` verbs in `ui/targeting`.

Roots: `0x48a3b0`, `0x48a420`, `0x48a510`, `0x48a580`, `0x48a610`, `0x48a7c0`, `0x48a830`, `0x48aef0`, `0x4bb820`,
`0x4bb990`, `0x4bbb00`, `0x48a6b0`, `0x4a1850`

## Inviting and removing by token

`InviteToParty("unit")`, `UninviteFromParty("unit")` and `PromoteToPartyLeader("unit")` turn the token into a GUID through
`GetGUIDFromName` and call `InviteByGuid`, `UninviteByGuid` and `SendGroupSetLeader` on the active player, when the player
and a nonzero GUID exist; otherwise nothing happens. `UninviteFromRaid(index)` does the same for the raid roster entry
index−1, using the first two dwords of the entry as the GUID. The first three raise `Usage: InviteToParty("unit")` and its
siblings for a non-string argument, the raid one `Usage: UninviteFromRaid(index)` for a non-number.

Addresses: `0x48a3b0`, `0x48a3ed`, `0x48a400`, `0x48a510`, `0x48a54d`, `0x48a55f`, `0x48a580`, `0x48a5c0`, `0x48a5cf`, `0x48a5ed`, `0x48a7c0`, `0x48a7fd`, `0x48a810`, `0x5e9440`, `0x5e9510`, `0x5e9640`, `0x515970`, `0xb713e0`, `0xb712a8`
Confidence: verified
Proof: one
Test: none

`0x48a3f2` and `0x48a3f6` require the player object and a GUID with either word set. `0x48a5c0` loads the roster count
`0xb713e0` and `0x48a5c7` compares `index-1` with it unsigned, so 0 and negative indexes select nothing. `0x48a5cf` reads
the roster pointer from `0xb712a8`.

## Inviting and removing by name

`InviteByName("name")` shows `ERR_INVITE_SELF` when the name equals the player's own, ignoring case; otherwise it sends
message 0x6e with the name. `UninviteByName("name")` sends 0x75 with it. `PromoteByName("name")` finds the best player of
that name (mask 0x10, mode 2) and sends 0x78 with the GUID, or nothing when none matches. The three raise
`Usage: InviteByName("name")` and its siblings for a non-string argument.

Addresses: `0x48a420`, `0x48a46b`, `0x48a473`, `0x48a47a`, `0x48a483`, `0x48a4b4`, `0x48a4c8`, `0x48a610`, `0x48a649`, `0x48a669`, `0x48a830`, `0x48a86a`, `0x48a8a0`, `0x609210`, `0x496720`, `0x418430`, `0x418370`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

`0x48a473` fetches the player's name with `GetUnitName`, `0x48a47a` compares with `SStrCmpI`, and `0x48a485` shows message
id 0x3b. Each send builds a `CDataStore` on the stack, writes the opcode with `PutUInt32` and the name with `PutCString`
(`PutGuid` for promotion) and calls `ClientServices::Send`.

## Answering invitations

`AcceptGroup()`, `DeclineGroup()`, `AcceptGuild()`, `DeclineGuild()` call `SendGroupAccept`, `SendGroupDecline`,
`SendGuildAccept` and `SendGuildDecline` on the active player, `AcceptResurrect()` and `DeclineResurrect()` call
`SendResurrectResponse` with 1 and 0, `RepopMe()` calls `SendRepopRequest`, and `LeaveParty()` calls `SendGroupDisband`.
Each does nothing without the player object, takes no argument and returns no value.

Addresses: `0x48aa80`, `0x48aab0`, `0x48aae0`, `0x48ab10`, `0x48a9a0`, `0x48a9d0`, `0x48a970`, `0x4e9180`, `0x48aa9f`, `0x48aacf`, `0x48aaff`, `0x48ab2f`, `0x48a9bf`, `0x48a9ef`, `0x48a991`, `0x5e96c0`, `0x5e9720`, `0x5e9870`, `0x5e98e0`, `0x5ded80`, `0x5e0ae0`, `0x5e9770`
Confidence: verified
Proof: one
Test: none

Each wrapper loads the player with `ClntObjMgrObjectPtr(mask 0x10, active player)`, tests it, and calls the named function
once: at `0x48aa9f`, `0x48aacf`, `0x48aaff`, `0x48ab2f`, `0x48a9bf` (with 1), `0x48a9ef` (with 0) and `0x48a991`. The call
`SendGroupDisband` of `LeaveParty` is read through the row of `0x4e9180` only.

## Guild verbs

Eight verbs send one guild message with a name. `GuildInviteByName`, `GuildUninviteByName`, `GuildPromoteByName`,
`GuildDemoteByName` and `GuildSetLeaderByName` write the name, `GuildSetMOTD("text")` the text; `GuildLeave()`,
`GuildDisband()` and `GuildInfo()` send the bare opcode. An empty or non-string invite name shows message 0x132; the others
raise `Usage: Guild<Name>("name")`. `GuildSetLeaderByName` sends no name for an empty string or the word `target`
(any case), leaving the choice to the server.

| verb | opcode | verb | opcode |
|---|---|---|---|
| `GuildInfo` | 0x87 | `GuildLeave` | 0x8d |
| `GuildInviteByName` | 0x82 | `GuildUninviteByName` | 0x8e |
| `GuildPromoteByName` | 0x8b | `GuildDisband` | 0x8f |
| `GuildDemoteByName` | 0x8c | `GuildSetLeaderByName` | 0x90 |
| `GuildSetMOTD` | 0x91 | | |

Addresses: `0x48aef0`, `0x48af22`, `0x48af93`, `0x48afb0`, `0x48b050`, `0x48b0f0`, `0x48b190`, `0x48b1e5`, `0x48b270`, `0x48b330`, `0x48b3a0`, `0x48b410`
Confidence: verified
Proof: one
Test: none

`0x48af22` pushes 0x82 and `0x48af93` shows message 0x132 for a missing or empty name. `0x48b1e1` tests the first byte and
`0x48b1e5` compares with `target` using `SStrCmpI` and length limit `0x7fffffff`; either match skips the `PutCString` at
`0x48b215`. The other verbs push their opcode and write the string argument as in the table.

## The raid roster verbs

`SetRaidRosterSelection(index)` stores the GUID of roster entry index−1 as the selection (`0xb713e8`), or 0 when the index is
outside the roster. `GetRaidRosterSelection()` returns the selected entry's 1-based index. `IsRaidOfficer()` returns 1 when
the player's own roster entry has a rank above 0, else nil. `SetRaidSubgroup(index, subgroup)` and
`SwapRaidSubgroup(index1, index2)` send message 0x27e with a name and the subgroup minus 1, and 0x280 with two names; they
do nothing for a missing entry, an entry already in that group, or a name not yet cached. `SetRaidSubgroup` raises its usage
error unless the subgroup is 1 to 8. `ConvertToRaid()` sends 0x28e for a party leader; `PromoteToAssistant(name)`
and `DemoteAssistant(name)` send 0x28f with the best player's GUID and the byte 1 or 0.

Addresses: `0x4bb820`, `0x4bb85f`, `0x4bb87b`, `0x4bb890`, `0x4bb897`, `0x4bb910`, `0x4bb968`, `0x4bb990`, `0x4bb9fe`, `0x4bba1f`, `0x4bba3c`, `0x4bba5e`, `0x4bba6e`, `0x4bbaa5`, `0x4bbb00`, `0x4bbb9f`, `0x4bbbfd`, `0x4bbc90`, `0x4bbca5`, `0x4bbcc5`, `0x4bbd20`, `0x4bbd6b`, `0x4bbd78`, `0x4bbdf0`, `0xb713e8`, `0x4bae90`
Confidence: verified
Proof: one
Test: none

`0x4bb87b` zeroes the selection for an out-of-range index. `0x4bb968` tests the rank dword at `+0xc` of the roster entry.
`0x4bb9fe` compares subgroup−1 with 8 unsigned. `0x4bba3c` compares the entry's subgroup at `+8`. `0x4bbbfd` and `0x4bba6e`
push the opcodes; the name comes from `DBCache::NameCache::GetRecord`. `0x4bbca5` compares the stored leader GUID with the
player's and requires a party. `0x4bbdf0` is the demotion twin of `0x4bbd20`, pushing 0 at its byte.

## Instances

`ResetInstances()` sends message 0x31d. `RequestRaidInfo()` sends 0x2cd. `GetNumSavedInstances()` returns the count at
`0xb6e5e8`; `GetSavedInstanceInfo(index)` returns, for entry index−1, the map's name in the current locale, a dword and a
second number, and for an invalid index nil, 0 and nil. `IsInInstance()` returns two values: 1 or nil by the map type
(nonzero is an instance), and the type name `none`, `party`, `raid` or `pvp`; for a map with no record it returns 2 without pushing anything. `CanShowResetInstances()` returns 1 only when the
server has sent the ownership flag, the player is not in a type-1 map, and the last type-1 map was left no more than 90000
seconds ago.

Addresses: `0x48a6b0`, `0x4a1850`, `0x4a18f0`, `0x4a1920`, `0x4a1982`, `0x4a1997`, `0x48a750`, `0x48a77b`, `0x48a79d`, `0x48a720`, `0x495c90`, `0x495cb9`, `0x495ce6`, `0x495d00`, `0x495d10`, `0x495d3f`, `0x495d50`, `0x49e670`, `0x49e6c0`, `0x49e470`, `0xb6e5e8`, `0xb4fdf4`, `0xb4e378`, `0xb4e374`, `0xb4e370`, `0xb4e37c`, `0xc0daa8`, `0xc0daac`, `0x4294c0`
Confidence: verified
Proof: one
Test: none

`0x495d10(now, map)`, run when a map is entered, stores the new map at `0xb4e378` and, when the previous map's record
type (`+8`) is 1, its id at `0xb4e374` and the time at `0xb4e370`; `0x49e470` clears both through `0x495d00`. `0x49e6c0` stores a handler word at
`0xb4e37c` (`0x495d50`). `0x495c90` tests it, the current map's type (`0x495cb9`) and the age against 0x15f90 (`0x495ce6`).
The type names are the pointer table at `0x83de58`. `0x4a1920` indexes 12-byte entries at `0xb4fdf4`.

## Small sends

| verb | effect |
|---|---|
| `RequestTimePlayed()` | sends 0x1cc |
| `TogglePVP()` | sends 0x253 |
| `OpeningCinematic()` | sends 0xf9 |
| `RandomRoll(min, max)` | sends 0x1fb with both numbers when 0 ≤ min ≤ max ≤ 1000000 |
| `SitOrStand()` | asks to stand (0) when the stand state byte is nonzero, else to sit (1) |
| `InCinematic()` | 1 while the cinematic flag is set, else nil |
| `StopCinematic()` | stops the narration and schedules the cinematic's end |
| `HasKey()` | 1 when the player's bags hold a key-family item, else nil |
| `GetDamageBonusStat()` | the class record's dword at `+8` plus 1, or 0 |

Addresses: `0x48a900`, `0x48d700`, `0x48c8c0`, `0x48c7b0`, `0x48c80b`, `0x48c822`, `0x48c82a`, `0x48c8a1`, `0x48b920`, `0x48b947`, `0x48b951`, `0x48b95b`, `0x48c930`, `0x48b970`, `0x48f050`, `0x48ae90`, `0x48aebc`, `0x6223a0`, `0x48b520`, `0x48b54c`, `0x5ed430`, `0xb4e310`, `0xc0def4`
Confidence: verified
Proof: one
Test: none

`RandomRoll` needs both arguments, each a string or a number (`0x6f3510` accepts Lua types 3 and 4; the tests are at `0x48c7c5`, `0x48c7d9`), so a single argument raises
`Usage: RandomRoll("max") or RandomRoll("min", "max")` although that text offers the one-argument form, and both zero returns without a message. `0x48c80b` to `0x48c82a` reject
a negative minimum, a maximum under the minimum and a maximum over 1000000. `0x48b947` reads the stand state byte at
descriptor `+0x210`. `HasKey` calls `FindItemByBagFamily(9, 0x4f)` on the player's bag block (`+0x1d38`). `0x48b54c` reads the
class byte at descriptor `+0x79`, indexes the store `0xc0def4` and pushes the record's dword at `+8` plus 1.

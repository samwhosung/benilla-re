# ui: the party, its leader and its members' records

`PartyFrame.cpp` keeps the four other party members, the group leader cell that the raid shares, and a record of each member that
outlives the member's object. This doc is the ui side: the tables, the leader functions, how a member's record is created and refreshed, and
the Lua getters that fall back to the record. The group messages and the record's fields are `objectlayer/party`; the
looking-for-group half of the file is `lfg`.

Roots: `0x4e8250`, `0x4e82d0`, `0x4e84a0`, `0x4e85d0`, `0x4e9100`, `0x4e9130`, `0x516210`

## Tables and the party functions

`0xbc6f48` holds the GUIDs of the four other party members, 8 bytes each, 0 for an empty slot; the local player is not among them. The member records
`0xbc70b0` are 0x148 bytes each, one per slot. `GetNumPartyMembers()` pushes the count of nonzero slots (0 to 4). `GetPartyMember(n)` takes n from 1
to 4 (`dec eax; cmp eax,4; jae`), raises `Usage: GetPartyMember(1-4)` for anything else, and pushes the number 1 when slot n-1 holds a GUID, else nil.
The count and the slot reader are `0x4e86d0` and `0x4e81a0`.

Addresses: `0x4e86d0`, `0x4e9050`, `0x4e9090`, `0x4e90b2`, `0x4e90b3`, `0x4e90ba`, `0x4e90e4`, `0x4e81a0`, `0x4e81b0`, `0x4e8820`, `0x4e8860`, `0x496400`, `0x496420`, `0xbc6f48`, `0xbc70b0`
Confidence: verified
Proof: one
Test: none

`0x4e8820(&guid)` returns the record of the slot holding a GUID, or 0; `0x4e8860(&guid)` returns the pet part (`+0x80`) of the online member
record whose pet GUID matches. `0x496400` tries `0x4e8820` and then the raid record `0x4bb0f0`; `0x496420` tries `0x4e8860` and then the raid pet record `0x4bb130`. The index
bound of `0x4e81a0` is unsigned, so a missing digit suffix (index -1) is out of range.

## The group leader

The leader GUID is `0xbc75f8`, one cell for party and raid. `0x4e8250(guid)` stores it when it differs from the stored one, resets the leader index `0x84c150` to -1, scans the four
slots and stores the slot number found (0 to 3), and signals `PARTY_LEADER_CHANGED`; an unchanged GUID returns without an event. `0x4e84a0(notify)` releases every member's
unit token flags, empties the slots, zeroes the leader cell and sets the index to 0, and signals `PARTY_MEMBERS_CHANGED` when `notify` is set. `IsPartyLeader()` pushes 1 when the cell is nonzero and is the
active player's GUID, else nil. `GetPartyLeaderIndex()` pushes `0x84c150 + 1`.

Addresses: `0x4e8250`, `0x4e8256`, `0x4e826e`, `0x4e827a`, `0x4e8288`, `0x4e82b8`, `0x4e82bd`, `0x4e82c8`, `0x4e84a0`, `0x4e85a9`, `0x4e85b5`, `0x4e85c0`, `0x4e9130`, `0x4e9141`, `0x4e9100`, `0x4e9104`, `0x4e7c93`, `0xbc75f8`, `0x84c150`
Confidence: verified
Proof: one
Test: none

`0x84c150` is -1 at start-up, -1 after a leader that is not one of the four, the slot index for a party member. The player is never in the four slots, so the cell is -1 and `GetPartyLeaderIndex()` pushes 0 for the player. `0x4e84a0` leaves it 0 with no group, and the
`0x4e8250(0, 0)` that follows it in the uninvite and disband handlers returns before its -1 store because the cell is already 0, so
`GetPartyLeaderIndex()` answers 1 (not 0) for a player in no group, but only after a group list, uninvite or disband has run `0x4e84a0`; before that it answers 0 (`0x4e7c9f`). `0x4e8250` signals event 219 on both of its non-returning paths; the early return fires none.

## UnitIsPartyLeader

`UnitIsPartyLeader(unit)` has no usage check. It resolves the token, then answers 1 when the unit's player object (type mask 0x10) has `PLAYER_FLAGS` bit 0x1
set, or when the token's GUID equals the leader cell; else nil. One value on both returning paths, and the number 1, never a boolean. A token no prefix
matches raises `Unknown unit name: %s` (`unittokens#the-unit-token`); a missing or non-string argument gives GUID 0, so with no group, where the cell is 0 too,
it answers 1.

Addresses: `0x516210`, `0x51621a`, `0x516221`, `0x51623b`, `0x51624a`, `0x516250`, `0x516258`, `0x516260`, `0x516266`, `0x516274`, `0x51627d`
Confidence: verified
Proof: one
Test: none

The flag leg is `mov eax,[obj+0xe68]; test byte [eax+8],1`, the one byte test of bit 0x1 among the 55 places that load the player block and then read its `+8` word; the next two bits of `PLAYER_FLAGS` are away (0x2) and do-not-disturb (0x4). It answers only for a player object that is held; a
pet, creature or totem falls to the GUID compare, as does a party or raid member whose object is not held (a `party3` out of range still resolves to its slot GUID). The GUID leg has no zero guard,
unlike `IsPartyLeader`, so `UnitIsPartyLeader()` is 1 outside a group and nil inside one.

## A member's record

`0x4e82d0(online, record, guid)` runs for each accepted member of a group list. It ignores a zero GUID and a full table; otherwise it stores the GUID in the first empty slot and fills the
record from a copy of the caller's old one (`rep movs`, 0x52 dwords), or with zeros and health, maximum, power and maximum power set to 1. It sets bit 0 of
the status byte from `online`. A member whose player object exists gets the party token flag 0x10 (0x20 on its pet) and a name refresh; with no object and no
old record the function sends opcode 0x27f with the member's GUID. It then queries the name cache; its callback signals `PARTY_MEMBERS_CHANGED`.

Addresses: `0x4e82d0`, `0x4e82f3`, `0x4e8312`, `0x4e8335`, `0x4e833d`, `0x4e836e`, `0x4e838d`, `0x4e839e`, `0x4e83d6`, `0x4e83dd`, `0x4e83e4`, `0x4e83f1`, `0x4e845f`, `0x4e8472`, `0x4e8490`, `0x60c4a0`, `0x607080`, `0x55f080`
Confidence: verified
Proof: one
Test: none

The placeholder stores are the words at `+0x0a`, `+0x0c`, `+0x0e` and `+0x10` (`0x4e8342` to `0x4e8357`). The request is sent only on the path where the player object is missing and the old record is null (`0x4e83e4`); a resynced
member keeps its cached record and sends nothing. `0x4e8490` signals event 218. When the member has no object and no old record the record stays at the placeholder until a stats message arrives.

## Asking for stats

`0x4e85d0(player, mode, withPet)` finds the player's slot by GUID (-1 when missing) and returns the slot plus 1. With mode 0 it copies the live descriptor into the record (`BuildPartyRecord`) and sends opcode 0x27f with the
member's GUID; with any other mode it signals `PARTY_MEMBER_ENABLE` with the slot number. Mode 0 runs when a party member's player object is deactivated, whether it is destroyed or goes out of range, from the player's
release method (`objectlayer/players`), whose caller then signals `PARTY_MEMBER_DISABLE`; mode 1 runs when the object is created or reactivated. There is no periodic request.

Addresses: `0x4e85d0`, `0x4e85e5`, `0x4e8608`, `0x4e8630`, `0x4e8646`, `0x4e865d`, `0x4e86a8`, `0x4e86ac`, `0x4bab6e`, `0x4bae27`, `0x4badc0`, `0x5f0880`, `0x5e9aa0`, `0x5debe0`, `0x5dec90`
Confidence: verified
Proof: one
Test: none

The opcode 0x27f senders are four: `0x4e83f1`, `0x4e8646` and their raid twins `0x4bab6e` (the roster rebuild for a raid member with no object) and `0x4bae27` (the raid hook `0x4badc0`). The mode-0 callers are the player release virtual (`0x5e9aa0`)
and the destructor when the object was not deactivated; mode 1 is called by the create path `0x5debe0` and the reactivation virtual `0x5dec90`. The census of pushes of 0x27f finds these four and a line number.
Events: `PARTY_MEMBER_ENABLE` is 220 and `PARTY_MEMBER_DISABLE` 221.

## Reading a member without its object

The Lua getters read the unit's descriptor while its object is held, and otherwise the record found by `0x496400` (party, then raid), then the pet record from `0x496420`. A getter
that finds nothing gives 0, or nil for a predicate. Record fields: `+0x08` status (1 online, 4 dead, 8 ghost), `+0x09` power type, u16 words `+0x0a` health, `+0x0c` maximum, `+0x0e` power, `+0x10` maximum power, `+0x12` level;
the pet block is the same layout at `+0x80`, its fields at `+0xd8` (power type), `+0xdc`, `+0xde`, `+0xe0`, `+0xe2`.

| function | held object | record | pet record |
|---|---|---|---|
| `UnitHealth` | `+0x40` clamped at 0; 0 when dynamic flags `+0x224` bit 0x20 | `+0x0a` | `+0x5c` |
| `UnitHealthMax` | `+0x58` | `+0x0c` | `+0x5e` |
| `UnitMana` | `+0x44 + 4*type`, divided by the type's display divisor; 0 with the same flag | `+0x0e` | `+0x60` |
| `UnitManaMax` | `+0x5c + 4*type`, divided | `+0x10` | `+0x62` |
| `UnitPowerType` | `+0x7b` byte | `+0x09` | `+0x58` |
| `UnitLevel` | its own function (`unitlevels#unitlevel`) | `+0x12` | none |
| `UnitIsConnected` | always 1 | `+0x08` bit 1, else nil | 1 when found |
| `UnitIsDead` | health at or below 0, or the dynamic flag | `+0x08` bit 4 | none |
| `UnitIsGhost` | player type and `PLAYER_FLAGS` ghost bit | `+0x08` bit 8 | none |
| `UnitIsDeadOrGhost` | `0x605f30` or the dynamic flag | `+0x08` bits 4 and 8 | none |

Addresses: `0x5174d0`, `0x5175b0`, `0x517670`, `0x5177e0`, `0x517940`, `0x517fc0`, `0x517d50`, `0x517ac0`, `0x517b90`, `0x517c70`, `0x517568`, `0x517571`, `0x51757a`, `0x517586`, `0x517630`, `0x517744`, `0x517748`, `0x51774f`, `0x5178af`, `0x5179d2`, `0x517dd3`, `0x517df4`, `0x517b5d`, `0x517c32`, `0x517d16`, `0x6e7130`
Confidence: verified
Proof: one
Test: none

The mana getters divide the record's words by `0x6e7130(type)` exactly as the live path does (`1` for a negative type, else the table entry). The same chain serves `UnitName`'s pet leg, `UnitBuff`, `UnitDebuff`, `SetPortraitTexture` and the tooltips.
`UnitIsConnected` for a pet record only tests that the pet was found (`test eax,eax` before the shared `jmp 0x517dd7`), and `0x4e8860` finds a pet only while the owner's online bit is set. The predicates push the literal double 1.0 (`push 0x3ff00000; push 0`) or nil, one value.

# ui: sharing a quest with the party

The party quest share: the two functions that gate and send a share, the sharer's answers and the
nine lines they become, what the client answers on its own, how a shared offer differs from an NPC's, the
confirmation a quest can ask for, and the leash that closes an offer panel. `IsUnitOnQuest`, which
tells whether a party member holds a quest, is here too. The quest list is `questlog`.

The code is `QuestLog.cpp` and `QuestFrame.cpp`: `GetQuestLogPushable` `0x4e12b0`, `QuestLogPushQuest`
`0x4e1340`, `IsUnitOnQuest` `0x4dfe10`, the offer display `0x5dbd50`, the decline reporter `0x501130`
and `ConfirmAcceptQuest` `0x5021b0`; the object-layer handlers `0x5e5eb0` and `0x5e38c0` carry two of
the messages.

Roots: `0x4e12b0`, `0x4e1340`, `0x4dfe10`, `0x501130`, `0x5021b0`, `0x5019a0`

## Whether a quest can be shared

`GetQuestLogPushable()` takes no argument and returns the number 1 or nil. It is 1 when a quest is selected
(`0xbb7480` is non-zero), its template is cached, and bit 0x8 of the template's flags word (`+0x38`) is set.
A cache miss gives nil and sends no query. `QuestLogPushQuest()` repeats those three tests and
also needs a local player and a first party member (`0xbc6f48`/`0xbc6f4c` not both 0), then calls `0x5eaed0` with the
quest id and returns no values.

Addresses: `0x4e12b0`, `0x4e12c6`, `0x4e12d3`, `0x4e12f3`, `0x4e12fa`, `0x4e12fc`, `0x4e1302`, `0x4e131d`, `0x4e1340`, `0x4e135a`, `0x4e139a`, `0x4e139e`, `0x4e13a0`, `0x4e13a4`, `0x4e13a6`, `0x4e13ac`, `0x4e13b2`, `0x4e13b7`, `0x5eaed0`, `0x6dbf60`, `0x6dc002`, `0x4df65b`, `0xbc6f48`
Confidence: verified
Proof: one
Test: none

The template's field 15 lands at `+0x38` (the deserializer `0x6dbf60` reads fifteen consecutive u32s, the last at
`0x6dc002`). Five instructions touch `+0x38`: the read, its serializer, `0x4df65b` (`and edi,6`) and the two
`test byte [eax+0x38],8` of these functions. The predicate reads no party state; the send function adds the party test.
`0x5eaed0` writes opcode 0x19d (`push 0x19d`) and the quest id as u32 and nothing else, with one caller (`0x4e13b7`), and 0x19d has
no other sender in the image.

## The sharer's answers

`MSG_QUEST_PUSH_RESULT` (opcode 0x276) arrives as `u64 sharerGuid`, `u8 result`. The handler (the arm `0x5e4781`
of `0x5e38c0`) looks the sharer's name up in the name cache, shows nothing at all when it is not there or
when the result is above 8, and otherwise shows a system chat line from the message catalogue, ids
0x181 to 0x189 for results 0 to 8: success, invalid, accepted, declined, too far, busy, log full, on quest and
already done (the `ERR_QUEST_PUSH_*_S` strings), each with the name. They are chat lines of type 10, not error text.

Addresses: `0x5e38c0`, `0x5e46b3`, `0x5e4781`, `0x5e478a`, `0x5e4795`, `0x5e47b1`, `0x5e47b6`, `0x5e47bd`, `0x5e47d3`, `0x5e47d6`, `0x5e47dc`, `0x5e47e3`, `0x5e48d3`, `0x5e48e1`, `0x55f080`, `0x496720`, `0xc0e228`
Confidence: verified
Proof: one
Test: none

The arm reads the GUID with `0x4190b0` and the byte with `0x418cb0`; a name-cache miss returns at once with
no query (`0x55f080` with callback 0). `cmp ecx,8; ja` is the range test. The
nine arms are 30 bytes each: `push name; push id; call 0x496720`. The ids follow the result exactly.
`tables/messages.tsv` rows 385 to 393 are these, kind 0, sound `NONE`, chat type 10.

## What the client answers itself

The client sends `MSG_QUEST_PUSH_RESULT` with only two results, both literals: 3 (declined) and 5 (busy).
A decline is sent by the reporter `0x501130(keep, report)` when the open offer was made by a player
object and the caller asks for a report; closing the panel, declining it and the leash all ask. The
quest accept does not. A share that arrives while another quest panel is open is refused with 5 by the
offer display and the panel is not opened. The out-of-range check also reports 3, never 4.

Addresses: `0x501130`, `0x501136`, `0x501149`, `0x501151`, `0x501159`, `0x501171`, `0x501173`, `0x5011aa`, `0x5011af`, `0x5011bf`, `0x501217`, `0x501a17`, `0x5014b1`, `0x4933da`, `0x500bc7`, `0x5013d3`, `0x5dbf2b`, `0x5dbf3d`, `0x5dbf56`, `0x5dbf85`, `0x5dbfbb`, `0xbe0810`, `0xbe0814`
Confidence: verified
Proof: one
Test: none

`0x501130` returns when the dialog's source GUID (`0xbe0810`, `0xbe0814`) is 0, when the report flag is 0, or when the GUID
does not resolve to a player (mask 0x10); it then writes opcode 0x276 (`push 0x276`), the GUID as u64 and `3`
as u8, clears the GUID and fires `QUEST_FINISHED` (id 304). The callers with the report flag are `CloseQuest`
(`0x501a17`), the decline (`0x5014b1`), the leash (`0x4933da`) and the leave-world cleanup (`0x500bc7`); the accept
passes 0 (`xor edx,edx`, `0x5013c5`). The only image references to 0x276 as an operand are the two sends
(`0x501173`, `0x5dbf56`), the register and the unregister.

## A shared offer and an NPC's

`SMSG_QUESTGIVER_QUEST_DETAILS` (opcode 0x188, handler `0x5e59b0`) reads `u64 sourceGuid`, `u32 questId`
and the title, then the texts, rewards and a trailing `u32`. It tells a shared offer from an NPC's by the
source object's type: the player bit. A player source skips the NPC-only step and takes the busy
path when a dialog is open; the trailing `u32` is kept at `0xbe0824` and only the decline reads it,
to skip re-opening a gossip menu. No Lua function reads it.

Addresses: `0x5e59b0`, `0x5e5a22`, `0x5dbd50`, `0x5dbd69`, `0x5dbd74`, `0x5dbd87`, `0x5dbefc`, `0x5dbf12`, `0x5dbf18`, `0x5dbf26`, `0x5dbf37`, `0x5dbf48`, `0x500ef0`, `0x50101a`, `0x5014a2`, `0x60bb80`, `0xbe0824`
Confidence: verified
Proof: one
Test: none

`0x5dbd50` resolves the GUID with mask 8, which a player object also carries, and tests bit 4 of its type word
(`shr edx,4; test dl,1`, `0x5dbf18`, again `0x5dbf2b`). `0xbe0824` has one reader, `0x5014a2` in the decline. The
cell is also written by the progress panel, whose meaning then differs.

## Accepting and declining

`AcceptQuest()` runs `0x501380`: with a panel of kind 1, a local player and the acted latch `0xbe0844` clear it sends
opcode 0x189 (`CMSG_QUESTGIVER_ACCEPT_QUEST`: `u64 guid`, `u32 questId`), sets the latch and closes the panel,
without any share answer. `DeclineQuest()` runs `0x5013f0`: when the latch is set it does nothing; for a unit
with the gossip flag it sends opcode 0x17b (`CMSG_GOSSIP_HELLO`, the guid), for an item, a player or an offer
with `0xbe0824` set it reports the decline as above, for a game object it calls the object's interact slot, and
for a plain unit it sends opcode 0x184 (`CMSG_QUESTGIVER_HELLO`, the guid), which reopens the giver's list.

Addresses: `0x501d20`, `0x501380`, `0x5013c5`, `0x501d30`, `0x5013f0`, `0x5013fe`, `0x50142c`, `0x501474`, `0x5014aa`, `0x5014d0`, `0x5014e4`, `0x5014ec`, `0x50151b`, `0x5df4d0`, `0x5df4d9`, `0x5eac10`, `0xbe0818`, `0xbe0844`
Confidence: verified
Proof: one
Test: none

The latch `0xbe0844` is set by each acting branch of the decline and by the accept. The gossip leg is
`[[obj+0x110]+0x234] & 1` (`0x501474`). Both functions push no Lua value.

## The confirmation request

`SMSG_QUEST_CONFIRM_ACCEPT` (opcode 0x19c, `0x5e5eb0`) is `u32 questId`, a C string title (at most 0x400)
and `u64 senderGuid`. The handler stores the quest id in `0xbe0840` at once, then, when the sender's name is
in the name cache, fires `QUEST_ACCEPT_CONFIRM` (id 346) with the name and the title as two strings; with
the name missing it fires nothing. `ConfirmAcceptQuest()` sends opcode 0x19b with whatever `0xbe0840` holds,
with no test. Nothing clears that cell, not a decline, a close, a party change or entering the world.

Addresses: `0x5e5eb0`, `0x5e5eb9`, `0x5e5ecb`, `0x5e5ede`, `0x5e5ee9`, `0x5e5efb`, `0x5019a0`, `0x5019c7`, `0x5019e9`, `0x5019f0`, `0x5019fe`, `0x5021b0`, `0x5021b9`, `0x5021db`, `0x5021e0`, `0x5021e9`, `0x5021f4`, `0x500af0`, `0xbe0840`
Confidence: verified
Proof: one
Test: none

Two instructions reference `0xbe0840`: the store `0x5019c7` and the read `0x5021e0`. The world-enter reset `0x500af0`
is straight-line stores and does not touch it. The event helper is called with the format `%s%s`.

## The offer leash

A latched interaction closes when the player walks too far from its object. The per-frame check `0x493230` reads the
squared range `0xb4e2c0` and, for any object but an item, measures the centre-to-centre distance squared
in the order `(dx*dx + dz*dz) + dy*dy`; a distance above the range closes the panel, which for a player-sourced
offer reports 3 and for an NPC sends nothing. The opener stores its own range; when the object is a player
the range is replaced by `(10 + 4)^2 = 196` (14 yards), so a shared quest has a 14-yard leash.

Addresses: `0x493230`, `0x49324e`, `0x493254`, `0x493276`, `0x493298`, `0x4932a0`, `0x4932b3`, `0x4932f2`, `0x493303`, `0x493305`, `0x493310`, `0x4933bf`, `0x4933d3`, `0x4818c0`, `0x481790`, `0x4930d0`, `0x493126`, `0x493156`, `0x49315e`, `0x493171`, `0xb4e2c0`, `0xb4e2d0`, `0x8044b0`, `0x80306c`
Confidence: verified
Proof: one
Test: none

`0x4930d0(guid, range2)` stores the range at `0x493126` and, when bit 4 of the object's type is set
(`0x493156`), overwrites it with `[0x8044b0] + [0x80306c]` (10.0 and 4.0) squared (`0x493171`). The check
returns early for a range of 0 or less (`fcomp` against 0.0, `0x493254`), closes for a missing, released
(`0x614ea0`) or absent object, and keeps the panel when the squared distance is at most the range
(`test ah,0x41; jne`, `0x493303`). It is called once per frame from `0x4818c0` in `0x481790`. A quest giver's range is
(50/9)^2 = 30.864195, 5.5555553 yards. The walk-away path calls `0x501130` directly, so it never reopens
a gossip menu.

## IsUnitOnQuest

`IsUnitOnQuest(index, unit)` takes a 1-based quest list row (a number) and a unit token (a string or
number; either type failing raises `Usage: IsUnitOnQuest(index, "unit")`) and returns the number 1 or nil. It
is 1 only when the row is a quest, the unit is the player or one of the four party members, the unit's
object is present as a player and not released, and the quest id is in one of the unit's 20 quest slots.
Otherwise nil.

Addresses: `0x4dfe10`, `0x4dfe17`, `0x4dfe2b`, `0x4dfe48`, `0x4dfe4d`, `0x4dfe52`, `0x4dfe55`, `0x4dfe5c`, `0x4dfe67`, `0x4dfe6e`, `0x4dfe7c`, `0x4dfe8b`, `0x4dfe9c`, `0x4dfea9`, `0x4dfeb2`, `0x4dfecf`, `0x4dfed9`, `0x4dfeec`, `0x4dff06`, `0x4df150`, `0x515970`, `0x4e7f70`, `0x468460`, `0x614ea0`
Confidence: verified
Proof: one
Test: none

The row to id step is `0x4df150(index-1)`; an id of 0 returns nil before the unit is resolved (`0x4dfe5c`). The
token goes through `0x515970`, which raises `Unknown unit name: %s` for an unknown token. `0x4e7f70` accepts the
active player's GUID or one of the four party GUIDs at `0xbc6f48` (stride 8, `cmp eax,0x20`), so `target`,
`mouseover` or `raidN` pass only when they name one. The loop compares the id with `[slot]` for byte offsets 0 to 0xe4
in steps of 12 (`0x4dfeb2` to `0x4dfecf`), pushing `1.0` at `0x4dfeec`.

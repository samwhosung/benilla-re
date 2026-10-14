# ui: duels

How the client answers a duel challenge and what the four duel verbs do (`DuelInfo.cpp`, `0x4d4710` to `0x4d4cf0`). The duel
spell is cast through the spell code (`spell/cast`); the flag object and the arena are the server's.

Roots: `0x4d4710`, `0x4d49d0`, `0x4d4ae0`, `0x4d4b20`, `0x4d4ba0`, `0x4d4c40`, `0x4d4c90`, `0x4d4830`, `0x4d48b0`

## Messages

`0x4d4710` installs six handlers: `SMSG_DUEL_REQUESTED` (0x167), `SMSG_DUEL_OUTOFBOUNDS` (0x168), `SMSG_DUEL_INBOUNDS` (0x169),
`SMSG_DUEL_COUNTDOWN` (0x2b7), `SMSG_DUEL_COMPLETE` (0x16a) and `SMSG_DUEL_WINNER` (0x16b); `0x4d4780` clears them. Out of bounds
and in bounds signal DUEL_OUTOFBOUNDS (381) and DUEL_INBOUNDS (382) and nothing else.

Addresses: `0x4d4710`, `0x4d4717`, `0x4d4728`, `0x4d4739`, `0x4d474a`, `0x4d4756`, `0x4d476c`, `0x4d4780`, `0x4d4aa0`, `0x4d4ac0`, `0x5ab650`, `0x5ab670`
Confidence: verified
Proof: one
Test: none

`0x4d4717`, `0x4d4728`, `0x4d4739` and `0x4d474a` load the opcodes 0x167, 0x168, 0x169 and 0x2b7 before each `SetMessageHandler` call; `0x4d4756` loads the complete handler and `0x4d476c` opcode 0x16b. `0x4d4aa0` and `0x4d4ac0` are `mov ecx,0x17d` and `mov ecx,0x17e` followed by the signal.

## A challenge

`0x4d49d0` reads two GUIDs: the first (the flag object) is kept as the current duel at `0xb73240`, the second is the challenger.
When the challenger is the active player it shows `ERR_DUEL_REQUESTED` (309) and accepts at once. When the challenger is on the
ignore list it cancels and clears the duel. Otherwise it signals DUEL_REQUESTED (380) with the challenger's name, when the player
object is known. `AcceptDuel()` sends `CMSG_DUEL_ACCEPTED` (0x16c) and `CancelDuel()` `CMSG_DUEL_CANCELLED` (0x16d), each with the
stored flag GUID and without a check that a duel exists.

Addresses: `0x4d49d0`, `0x4d49e1`, `0x4d49ec`, `0x4d4a05`, `0x4d4a12`, `0x4d4a2a`, `0x4d4a33`, `0x4d4a78`, `0x4d4a83`, `0x4d4830`, `0x4d4839`, `0x4d48b0`, `0x4d48b9`, `0x4d4ce0`, `0x4d4cf0`, `0xb73240`
Confidence: verified
Proof: one
Test: none

`0x4d49e1` and `0x4d49ec` read the two GUIDs. `0x4d4a05` pushes error 0x135 and `0x4d4a12` calls the accept sender `0x4d4830`; `0x4d4a2a` tests `FriendList::IsIgnored` and `0x4d4a33` calls the cancel sender `0x4d48b0`. The senders push 0x16c at `0x4d4839` and 0x16d at `0x4d48b9`. `0x4d4a78` fetches the name and `0x4d4a83` pushes event 0x17c.

## Countdown and result

`SMSG_DUEL_COUNTDOWN` carries the time in milliseconds; the handler stores it divided by 1000 as the seconds and starts the
`ProcessCountdown` timer: each second it prints `DUEL_COUNTDOWN` with the remaining seconds as a system line in the chat and re-arms
itself until the count is 0. `SMSG_DUEL_COMPLETE` ends a duel that is current: a flag of 0 shows `ERR_DUEL_CANCELLED` (310);
DUEL_FINISHED (383) is signalled, the flag GUID is cleared and the countdown timer is killed. `SMSG_DUEL_WINNER` prints
`DUEL_WINNER_KNOCKOUT` for a flag of 0, `DUEL_WINNER_RETREAT` otherwise, with the two names, as a system line.

Addresses: `0x4d4ae0`, `0x4d4aef`, `0x4d4af4`, `0x4d4afa`, `0x4d4b04`, `0x4d4930`, `0x4d4949`, `0x4d4982`, `0x4d4987`, `0x4d4996`, `0x4d49a0`, `0x4d4b20`, `0x4d4b35`, `0x4d4b44`, `0x4d4b51`, `0x4d4b79`, `0x4d4ba0`, `0x4d4be5`, `0x4d4bec`, `0x4d4c2a`, `0x4d47d0`, `0x401520`, `0x401550`, `0xb73248`
Confidence: verified
Proof: one
Test: none

`0x4d4af4` multiplies by 0x10624dd3 and `0x4d4afa` shifts right by 6, which is division by 1000. `0x4d4949` fetches the global string and `0x4d4982` adds the chat line with type 10; `0x4d4996` re-arms with 1000 ms. `0x4d4b35` tests the current duel, `0x4d4b44` pushes error 0x136 for a flag of 0 and `0x4d4b51` loads event 0x17f. `0x4d4be5` and `0x4d4bec` pick the winner string.

## Starting a duel

`StartDuel("name")` finds the player whose name begins with the text (`ClosestObjectMatch` over players) and casts the duel spell on
that GUID; with no match or a non-string it does nothing. `StartDuelUnit("unit")` does the same for a unit token and raises
`Usage: StartDuelUnit("unit")` for a non-string. The duel spell is the id the spell book keeps when a learned spell's first effect is 0x53.

Addresses: `0x4d4c40`, `0x4d4c4d`, `0x4d4c6d`, `0x4d4c7a`, `0x4d4c90`, `0x4d4cad`, `0x4d4caf`, `0x4d4cbc`, `0x4d4810`, `0x4d4823`, `0x493aa0`, `0x515970`, `0x6e5a90`
Confidence: verified
Proof: one
Test: none

`0x4d4c6d` calls `ClosestObjectMatch` with kind mask 0x10 and `0x4d4caf` `GetGUIDFromName`. Both call `0x4d4810`, which passes the spell id from `0xb71130` and the GUID to `CastSpellAsActivePlayer` (`0x4d4823`). The id is written by the spell book add routine when the spell's first effect (`+0xf4`) is 0x53.

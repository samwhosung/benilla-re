# ui: guild charters and petitions

The interface side of the guild charter: the module state behind the petition window, the Lua functions that read and
change it, the charter shop, and the events. The server messages that fill the state are `objectlayer/party#petitions`;
the tabard functions registered beside these are `ui/tabard`. This is `PetitionInfo.cpp` (`0x4f3e20` to `0x4f5240`).

Roots: `0x4f3f60`, `0x4f40b0`, `0x4f41c0`, `0x4f43c0`, `0x4f43d0`, `0x4f44e0`, `0x4f4510`, `0x4f45e0`, `0x4f46d0`, `0x4f4790`, `0x4f4930`, `0x4f4fb0`, `0x4f5010`, `0x4f5050`, `0x4f5160`, `0x4f5220`, `0x4f5230`, `0x4f5260`, `0x4f52d0`

## Petition state

The window reads a few cells. The open petition's item GUID is `0xbdce10`, its id `0xbdce18`, and a latch that is 1 while a
signature is outstanding `0xbdce1c`. The signature count is `0xbdce20` (signers only, not the owner), the number of signer
names still being fetched `0xbdce24`, and the cached petition record `0xbdce28`. The signers are an array of 16-byte
entries (GUID in the first eight bytes and the trailing dword of the wire entry); its base is `0xbdcdcc`. The guild
registrar's NPC GUID is `0xbdceb0`, and the first entry of the vendor's charter list is kept as five dwords from
`0xbdce44`: its index, the charter item, the display id, the cost (`0xbdce50`) and the flags.

Addresses: `0x4f3f60`, `0x4f3ef0`, `0x4f40b0`, `0x4f4e40`, `0x4f4e60`, `0x4f4f60`, `0x4f4fb0`, `0x4f4ffc`, `0xbdce10`, `0xbdce18`, `0xbdce1c`, `0xbdce20`, `0xbdce24`, `0xbdce28`, `0xbdcdc4`, `0xbdcdcc`, `0xbdce44`, `0xbdceb0`, `0xc0e0fc`
Confidence: verified
Proof: one
Test: none

The signer array has minimum capacity 0x10 (`0x4f4105`) and is allocated under the tag `__AUPetitionSignerInfo__`. The five
dwords are `rep movsd` from the showlist entry (`0x4f4ffc`); `0x4f4f60`, called from the world-enter step, zeroes
them, and `0x4f3ef0` zeroes the count and the pending-name counter. The cached record is from the petition cache
`0xc0e0fc`.

## Petition info

`GetPetitionInfo()` always returns six values: the type (`charter` when bit 0 of the record's flags at `+0x1110` is set,
else `petition`), the title, the body text, the signature limit, the owner's name when cached (else nil), and 1 when the
player owns the petition (else nil). With no petition cached it returns `nil, nil, nil, 0, nil, nil`. The limit is the
record's signed word at `+0x1118`, the title is the string at `+0x10` and the body the string at `+0x110`.

Addresses: `0x4f43d0`, `0x4f43eb`, `0x4f43fb`, `0x4f440e`, `0x4f4413`, `0x4f441d`, `0x4f442a`, `0x4f4459`, `0x4f4472`, `0x4f448e`, `0x4f44a1`, `0x4f44b4`, `0x4f44c6`, `0x55f080`, `0x468550`
Confidence: verified
Proof: one
Test: none

The owner name comes from the name cache `0xc0e228` with the owner GUID at `+0x8`; the ownership test compares the same
GUID with the active player's. The limit is the field `CanSignPetition` compares the count with (`0x4f4634`).

## Signatures

`GetNumPetitionNames()` returns the signature count as an unsigned number. `GetPetitionNameInfo(index)` returns one value:
the signer's name when cached, else nil, and nil for an index outside 1 to the count. A non-number raises `Usage:
GetPetitionNameInfo(index)`. `CanSignPetition()` returns 1 or nil: nil for a charter when the player is in a guild or the
list is full, nil for the owner and for a player already listed, and 1 otherwise, including with no petition open.

Addresses: `0x4f44e0`, `0x4f4510`, `0x4f4531`, `0x4f4569`, `0x4f456a`, `0x4f45a1`, `0x4f45e0`, `0x4f45f7`, `0x4f45f9`, `0x4f4621`, `0x4f4634`, `0x4f4643`, `0x4f4666`, `0x4f469c`, `0x4f46ad`
Confidence: verified
Proof: one
Test: none

The index test is `dec eax; cmp eax,[0xbdce20]; jae`, so 0 and negatives fail. The refusals of `CanSignPetition` need the
charter bit for the guild and limit tests (`0x4f45f9`); with no cached record the function jumps to the signer scan
(`0x4f45f7 je 0x4f4655`), and that array is empty because `0x4f3ef0` clears the count when a petition closes.

## Signing and offering

`SignPetition([n])` does nothing without an open petition; otherwise it sends opcode 0x1c0 with the item GUID and the
byte `n` (default 1) and sets the in-flight latch. `OfferPetition()` offers the open petition to the current target
(the selection, not the interaction target). It does nothing without a petition, a target or a player object; a target
that does not resolve shows `ERR_OUT_OF_RANGE`; a non-player target is silent; the player itself shows
`ERR_PETITION_CREATOR`; a target that is not allied shows `ERR_GUILD_NOT_ALLIED`; one in a guild shows
`ERR_ALREADY_IN_GUILD_S` with its name. Otherwise it sends 0x1c3 with the item GUID and the target's GUID and shows
`ERR_PETITION_OFFERED_S` at once.

Addresses: `0x4f46d0`, `0x4f46e2`, `0x4f4711`, `0x4f4740`, `0x4f4749`, `0x4f4754`, `0x4f475d`, `0x4f4790`, `0x4f47ac`, `0x4f47bd`, `0x4f47e4`, `0x4f4810`, `0x4f4831`, `0x4f4839`, `0x4f485b`, `0x4f4879`, `0x4f489c`, `0x4f48dc`, `0x4f48e7`, `0x4f48fa`, `0x606ba0`, `0x609210`, `0xb4e2d8`
Confidence: verified
Proof: one
Test: none

The target cell pair is `0xb4e2d8` and `0xb4e2dc`. The guard order is the order of the addresses above; the offered message is
pushed after the send, with no reply awaited. The sign function sends no bytes at all without the GUID (`0x4f470d je`).

## Renaming

`RenamePetition(name)` sends opcode 0x2c1 with the item GUID and the name when a petition is open and the name passes the
guild name check; otherwise it does nothing. A non-string argument raises the text `Usage(RenamePetition("name")`,
which has a bracket where the others have `Usage:`.

Addresses: `0x4f4930`, `0x4f493f`, `0x4f494e`, `0x4f4987`, `0x4f4992`, `0x4f49c1`, `0x4f49ca`, `0x4f49d5`, `0x4f5160`
Confidence: verified
Proof: one
Test: none

The check is the validator `0x4f5160` of the next entry. The packet is the opcode, the item GUID (`0x418370`) and the name
(`0x418430`).

## Guild name check

`0x4f5160(name)` returns 1 only for a name the locale-aware checker `0x6c9b70` accepts (code 13). Every other code shows a
message and returns 0: 0 `ERR_GUILD_ENTER_NAME`, 1 `ERR_GUILD_NAME_TOO_SHORT`, 4 `ERR_GUILD_NAME_MIXED_LANGUAGES`,
5 `ERR_GUILD_NAME_PROFANE`, 6 `ERR_GUILD_NAME_RESERVED`, 10 `ERR_GUILD_NAME_INVALID_SPACE`, 11
`ERR_GUILD_NAME_NAME_CONSECUTIVE_SPACES`, and 2, 3, 7, 8, 9 or above 11 `ERR_GUILD_NAME_INVALID`.

Addresses: `0x4f5160`, `0x4f5162`, `0x4f5168`, `0x4f516d`, `0x4f5178`, `0x4f517d`, `0x6c9b70`, `0x496720`
Confidence: verified
Proof: one
Test: none

The code is passed through `cmp eax,0xd` and `cmp eax,0xb; ja`, then a 12-entry jump table that follows the function. The
message ids are 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x7b and 0x73.

## Closing

`ClosePetition()` is the replace-or-close routine `0x4f3f60` with no petition. When a petition was open, no signature is
in flight, a record is cached and the player is not its owner, it sends the decline (opcode 0x1c2) first; it then
signals `PETITION_CLOSED` and clears the record and the latch. `CloseGuildRegistrar()` does nothing without a registrar
GUID; otherwise it clears the interaction target for it, zeroes the GUID and signals `GUILD_REGISTRAR_CLOSED`. It sends
nothing of its own. The target clear (`0x4f5024`, `0x4931c0`) runs the dispatcher `0x493310`, which calls the close
again while the GUID is still set (the zeroing is `0x4f502b`), so `GUILD_REGISTRAR_CLOSED` fires twice when the NPC is
still the interaction target.

Addresses: `0x4f43c0`, `0x4f3f60`, `0x4f3fb5`, `0x4f3ff3`, `0x4f4016`, `0x4f402c`, `0x4f4078`, `0x4f5220`, `0x4f5010`, `0x4f5024`, `0x4f503a`, `0x4931c0`, `0x493310`
Confidence: verified
Proof: one
Test: none

The decline is `Put32(0x1c2)` at `0x4f3fb5` and the send at `0x4f3ff3`. `0x4f5010` tail-jumps to the signaller with 0x17b. The
close path `0x4931c0` builds no packet; it hands a matching GUID to the interaction-clear dispatcher `0x493310`, which
zeroes the target and calls the close of the first window holding that GUID: an else-if chain (trade, `0xbc3fa0`,
gossip, and on to the stable `0xb71b48`) that jumps to `0x493513` after the first match.

## Charter shop

`GetGuildCharterCost()` returns the cost of the first charter in the vendor's list, in copper, as an unsigned number; it
is 0 before a list arrives. `BuyGuildCharter(name)` returns 1 when the name passes the guild name check and nil otherwise,
and a non-string raises `Usage: BuyGuildCharter(guildName)`. The purchase itself (`0x4f5050`) then refuses silently with no
name, no registrar or no player, shows `ERR_ALREADY_IN_GUILD` for a player in a guild and `ERR_NOT_ENOUGH_MONEY` when
the coinage is below the cost; otherwise it sends the petition record with only the name filled in, for the registrar
GUID. `TurnInGuildCharter()` hands the active player to the charter search of `objectlayer/party#petitions`.

Addresses: `0x4f5230`, `0x4f5236`, `0x4f5040`, `0x4f5260`, `0x4f5268`, `0x4f5294`, `0x4f529f`, `0x4f52bc`, `0x4f5050`, `0x4f5064`, `0x4f507e`, `0x4f50a3`, `0x4f50b4`, `0x4f50ed`, `0x4f5138`, `0x4f5153`, `0x4f52d0`, `0x4f52f1`, `0x5eec70`, `0x5ef2b0`, `0xbdce50`
Confidence: verified
Proof: one
Test: none

`0x4f5040` is `mov eax,[0xbdce50]; ret`. The cost is read with `fild qword` with the high dword zero, so it is unsigned
(`0x4f5245`); the purchase compares it with `PLAYER_FIELD_COINAGE` (`[player+0xe68]+0xf70`). `BuyGuildCharter` pushes 1.0
or nil after the validator (`0x4f5294`). The record is cleared with `rep stos` (`0x4f511b`), the name copied to `+0x10`
(`0x4f5138`), and `0x5eec70` sends it with the registrar GUID `0xbdceb0`, not the interaction pair.

## Petition events

Three events belong to the petition window and one to the registrar; none carries an argument. `GUILD_REGISTRAR_SHOW`
(378) is signalled by the list opener, `GUILD_REGISTRAR_CLOSED` (379) by the registrar close. `PETITION_SHOW` (390) waits
for both the record and every signer name: it is signalled when the list is stored and nothing is pending, when one signer
is appended with nothing pending, when the last pending name resolves with the record cached, and when the record
arrives with nothing pending. `PETITION_CLOSED`
(391) is signalled when a petition is replaced or closed.

Addresses: `0x4f5003`, `0x4f503a`, `0x4f402c`, `0x4f41b4`, `0x4f430c`, `0x4f433f`, `0x4f43ad`, `0x4f419b`, `0x4f4090`, `0x4f4320`, `0x4f4350`, `0x703e50`
Confidence: verified
Proof: one
Test: none

The deferral is `0x4f419b` to `0x4f41ad`: the event needs `[0xbdce24] == 0` and `[0xbdce28] != 0`. The callback `0x4f4090` runs
`0x4f4350`, which fetches the record of the open petition id from the cache; `0x4f4320` decrements the pending counter and
signals only on the step to zero (`0x4f432f jne`), so the event comes once however many names were outstanding. Other signal sites are
`objectlayer/party#petitions` (`0x5ef037`, `0x5ef16b`, `0x5ef29c`).

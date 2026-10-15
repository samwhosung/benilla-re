# net: friends

The friend and ignore lists, the `/who` query and the who results.

`FriendList.cpp` holds one object, `0xc28168`, of `0x728` bytes, built once at start-up. It keeps 50 friend slots and 25 ignore guids, filled by server packets, and the list of who results the `/who` reply fills. The Lua functions that read them (`GetFriendInfo`, `GetWhoInfo`, `SortWho` and the rest) are in the same file and are registered as 19 pairs of name and function in the table `0x85d830`. Names of players come from the name cache (`0xc0e228`): a slot or ignore that waits for a name is counted in `+0x640` and `+0x718`, and the lists sort and signal their update event when the count reaches zero.

Roots: `0x5adc50`, `0x5ad000`, `0x5ad060`, `0x5ad260`, `0x5ad3b0`, `0x5ad690`, `0x5ad6e0`, `0x5ad870`, `0x5ad890`, `0x5ae5a0`, `0x5ae670`, `0x5ae810`, `0x5ae490`, `0x5ae510`, `0x5ae550`, `0x5ae5f0`, `0x5ae630`

## Friend list object

The object is cleared by `0x5aca10`. Friend slot `k` is 32 bytes at `+0x20·k`: byte `+0` online, byte `+1` status flags (2 is away, 4 is busy), `+4` the name string pointer, `+8` the guid, `+0x10` level, `+0x14` class id, `+0x18` area id. `+0x640` counts friend names pending, `+0x648` is the selected friend's guid, the ignore guids are 8 bytes each from `+0x650`, `+0x718` counts ignore names pending and `+0x720` is the selected ignore's guid. `0x5adc50` creates the object once, registers handlers for opcodes `0x63`, `0x65`, `0x1fe`, `0x67`, `0x68` and `0x6b`, registers the console command `whois`, and sets the who sort chain and counts.

Addresses: `0x5adc50`, `0x5aca10`, `0x85d830`, `0x5adc7b`, `0x5adc98`, `0x5adca8`, `0x5adcb8`, `0x5adcc8`, `0x5adcd8`, `0x5adce8`, `0x5adcfd`, `0xc28168`
Confidence: verified
Proof: one
Test: none

`0x5adc50` returns when `0xc28168` is set; it allocates `0x728` (`0x5adc6b`) and calls `0x5aca10`. The handlers are `0x5adf60` (`0x63`), `0x5adda0` (`0x65`), `0x5adde0` (`0x1fe`), `0x5add80` (`0x67`), `0x5add30` (`0x68`) and `0x5ae2b0` (`0x6b`), each through `0x5ab650`; `0x5adee0` is the handler of `whois` (`0x5adcfd`, `0x63f9e0`). `0x5adc50` has one caller, `0x401666`, and no stored address. A scan of `call` and `jmp` for `0x5adc50` and of the image for its address as a dword finds that one site; the control is `0x5ab1a0`, which finds its four.

## Friend list packet

`SMSG_FRIEND_LIST` (`0x67`) replaces the friends. `0x5ae350` frees every slot's name and clears the pointers, then reads a count byte and, for each record up to 50: the guid, a status byte, and, when the status is not 0, three dwords stored at `+0x18`, `+0x10` and `+0x14`: area, level, class. A status of 0 stores zeros there. The slot's online byte is 1 for a non-zero status, and `+1` holds the status. Each name comes from the name cache; a name not cached adds to `+0x640`. When nothing is pending at the end the friends sort and `FRIENDLIST_UPDATE` fires (`0x5ae850`).

Addresses: `0x5ae350`, `0x5add80`, `0x5ae37e`, `0x5ae3df`, `0x5ae3ee`, `0x5ae438`, `0x5ae475`, `0x5ae850`, `0x5ace70`
Confidence: verified
Proof: one
Test: `net::friends_read_list`

`0x5ae366` to `0x5ae384` loop over 50 slots stepping `0x20` from `+4`, the name pointer, free (`0x646430`) and zero it. `0x5ae393` reads the count; `0x5ae3a7` `cmp [ebp+0xb],0x32; jae` stops at 50 records. The reads are `0x4190b0`, `0x418cb0` then `0x418e30` three times (`0x5ae3f6`, `0x5ae401`, `0x5ae40c`). `0x5ae438` is `0x55f080` with the callback `0x5ace70` and the message id `0x1d1`; a non-null result is copied with `0x64a620`, a null one increments `+0x640` (`0x5ae459`). `0x5ae47b` tests `+0x640`.

The pending count is not reset: a lookup still unanswered from an earlier list keeps it above zero, and the sort then waits for that answer. Records past the 50th are left unread. A packet shorter than its count leaves the reads on stale stack; the test reads whole packets.

## Ignore list packet

`SMSG_IGNORE_LIST` (`0x6b`) replaces the ignores: `0x5ae2b0` passes it to `0x5af530` when the friend list exists. `0x5af530` zeroes the 25 guids and reads a count byte and that many guids, with no cap. Each is stored by `0x5af5d0` without a lookup, then looked up with the callback `0x5acf40`; a miss adds to `+0x718`, which is not reset first. With nothing pending at the end the ignores sort (`0x5ae910`). `0x5af5d0(guid, look_up)` stores the guid in the first slot whose guid is 0, or does nothing when all 25 are taken; with `look_up` set it also looks the name up, counts a miss and sorts when nothing is pending. A guid past the 25th is dropped but still looked up and counted.

Addresses: `0x5ae2b0`, `0x5af530`, `0x5af5d0`, `0x5af548`, `0x5af578`, `0x5af59a`, `0x5af5a3`, `0x5af5e0`, `0x5af5eb`, `0x5af60c`, `0x5af627`, `0x5ae910`, `0x5acf40`
Confidence: verified
Proof: one
Test: `net::friends_ignore_list`

`0x5af548` is `rep stos` of `0x32` dwords from `+0x650`. The loop compares its index with the count byte (`0x5af5ad` to `0x5af5b0`) and nothing else. `0x5af578` calls `0x5af5d0` with a zero flag; `0x5af59a` is `0x55f080` with `0x5acf40`, the id `0x1d1` and a copy of the guid on the stack; `0x5af5a3` increments `+0x718`. `0x5af5e0` to `0x5af5ee` test each guid's two dwords for zero, 25 times (`cmp eax,0x19`), and return on none. `0x5af60c` tests the flag, `0x5af627` looks up, and `0x5af636` tests `+0x718` only on that path.

## Pending names

`0x5aeb70` and `0x5aeb90` are the name cache callbacks' last step for a friend and an ignore. Each takes one from its pending count, `+0x640` or `+0x718`, when that is not 0, and sorts its list when the count reaches 0 (`0x5ae850`, `0x5ae910`). A count of 0 stays 0 and nothing sorts.

Addresses: `0x5aeb70`, `0x5aeb90`, `0x5ae850`, `0x5ae910`, `0x5aceb6`, `0x5acf97`
Confidence: verified
Proof: one
Test: `net::friends_pending_counts`

`0x5aeb76` `test eax,eax; je` returns on 0; `0x5aeb7a` decrements and stores; `0x5aeb85` tail-jumps to `0x5ae850` when the result is 0. `0x5aeb90` is the same on `+0x718` with `0x5ae910`. The callers are `0x5ace70` (`0x5aceb6`) and `0x5acf40` (`0x5acf97`).

## Friend order

`0x5ae850` sorts the 50 slots with `qsort` and the comparator `0x5ae870`. A slot with guid 0 goes last, two such are equal. Then, when the online bytes differ, the one that is 0 goes last; two different non-zero bytes give `-1` both ways, but the byte is only written 0 or 1. Then the names compare without case through `0x64a4c0` with no bound (Strings without case); a missing name goes last, two missing are equal.

Addresses: `0x5ae850`, `0x5ae870`, `0x5ae884`, `0x5ae8b5`, `0x5ae8ce`, `0x5ae8e6`, `0x5ae8f4`
Confidence: verified
Proof: one
Test: `net::friends_compare`

`0x5ae876` to `0x5ae8b2` test the two guids for zero. `0x5ae8b5` compares the bytes at `+0`; on a difference `0x5ae8f4` returns `(a == 0) * 2 - 1`. `0x5ae8b9` to `0x5ae8f3` handle the names at `+4`, calling `0x64a4c0` with `0x7fffffff` at `0x5ae8ce`.

## Strings without case

`SStrCmpI` (`0x64a4c0`, `(a, b, n)`) calls `__strnicmp` (`0x414310`) for two non-null strings, and otherwise sets error `0x57` and returns the first pointer less the second. `__strnicmp` runs its C locale path, since the locale word `0x883b08` stays 0: `A` to `Z` fold to lower case, the first pair that differs decides `-1` or 1 unsigned, and a NUL in either string or the bound ends the compare; a bound of 0 is equal.

Addresses: `0x64a4c0`, `0x64a4d5`, `0x414310`, `0x41432d`, `0x41434a`, `0x414365`, `0x410c71`, `0x4109d6`
Confidence: verified
Proof: one
Test: `net::friends_compare`

`0x64a4c7` and `0x64a4cf` test the pointers and `0x64a4d5` calls `0x64e850` with `0x57`. In `0x414310`, `0x41432d` tests `[0x883b08]`; `0x41434a` to `0x41435c` add `0x20` to a byte from `0x41` to `0x5a`; `0x414365` to `0x41437a` turn the last pair into 0, `-1` or 1 from an unsigned compare. The one write to the locale words, `0x410c71` in `__setlocale_set_cat` (`0x410c1c`), is reached only from `_setlocale` (`0x4109d6`, the calls at `0x410a25`, `0x410b1e`, `0x410bb3`), and a scan of `call` and `jmp` and of the image for `0x4109d6` as a dword finds nothing.

## Friend status result

`SMSG_FRIEND_STATUS` (`0x68`) is a result byte and a guid, read by `0x5add30` and passed to `0x5acab0` with the packet; the handler returns 1 even when there is no friend list. The result selects an action and a game error `0x496720` shows. Results 2, 3, 5, 6, 7 and 8 name the player, so the message waits for a cache lookup.

| result | action on the slot | game error |
|---|---|---|
| 0, 4 | none | `ERR_FRIEND_NOT_FOUND` |
| 1 | none | `ERR_FRIEND_LIST_FULL` |
| 2 | read status byte, area, level, class into the guid's slot, mark it online | `ERR_FRIEND_ONLINE_SS` |
| 3 | mark offline | `ERR_FRIEND_OFFLINE_S` |
| 5 | clear the slot | `ERR_FRIEND_REMOVED_S` |
| 6, 7 | find or claim a slot; 6 reads the four fields as 2 does, then 6 marks online, 7 offline | `ERR_FRIEND_ADDED_S` |
| 8 | find or claim a slot | `ERR_FRIEND_ALREADY_S` |
| 9, 10 | none | `ERR_FRIEND_SELF`, `ERR_FRIEND_WRONG_FACTION` |
| 11, 12, 13 | none | `ERR_IGNORE_FULL`, `ERR_IGNORE_SELF`, `ERR_IGNORE_NOT_FOUND` |
| 14, 15, 16 | 15 adds the guid to the ignores, 16 removes it | `ERR_IGNORE_ALREADY_S`, `ERR_IGNORE_ADDED_S`, `ERR_IGNORE_REMOVED_S` |
| 17, 18 and up | none | `ERR_IGNORE_AMBIGUOUS`, `ERR_FRIEND_ERROR` |

Addresses: `0x5add30`, `0x5acab0`, `0x5ae850`, `0x5ae910`, `0x5acac9`, `0x5acacf`, `0x5ace1c`, `0x5aea00`, `0x5aea60`, `0x5aeac0`, `0x5aeb30`, `0x5af650`, `0x5ace70`, `0x5acf40`, `0x5aeb70`, `0x5acdbd`, `0x496720`
Confidence: verified
Proof: one
Test: `net::friends_status`

`0x5acab9` `cmp eax,0x11; ja 0x5acd85` and `jmp [eax*4+0x5ace1c]` (18 entries). The ids passed to `0x496720` are `0x108`, `0x104`, `0x106`, `0x107`, `0x10a`, `0x105`, `0x10c`, `0x10d`, `0x109`, `0x10e`, `0x10f`, `0x110`, `0x111`, `0x112`, `0x113`, `0x114` and, past 17, `0x10b`; `tables/messages.tsv` names them from id 259. The id `0x103`, `ERR_FRIEND_DB_ERROR`, is never passed. A result that looks up a name calls `0x55f080` (`0x5acb97`): a cached name sets the name for result 6 or 7 (`0x5acdc3`), sorts the friends or ignores and shows the error with the name twice; a miss adds to `+0x640` or `+0x718` and the callback `0x5ace70` or `0x5acf40` finishes later: a found name is stored and the error shown unless the id is `0x1d1`; a name not found shows `ERR_FRIEND_ERROR` and sends the remove packet (`0x5ae760`, `0x5af4b0`) for the guid.

## Friend slot claim

Results 6, 7 and 8 find or claim the guid's slot (`0x5aea60`): the slot holding the guid, else the first empty one, which takes the guid and an online byte of 0. The scan stops at that empty slot. On a full list without the guid it writes the guid and a zero online byte as slot `-1`, in the 32 bytes before the object, and returns `-1`, so result 6 stores no fields. Result 2 instead matches the guid against all 50 slots, empty ones included. Result 16 (`0x5af650`) zeroes the first ignore holding the guid and sorts the ignores when nothing is pending.

Addresses: `0x5aea60`, `0x5aea81`, `0x5aea92`, `0x5acb47`, `0x5acc20`, `0x5af650`, `0x5af691`
Confidence: verified
Proof: one
Test: `net::friends_status`

`0x5aea66` starts the index at `-1` (`or ebx,0xffffffff`); the loop returns a match at once (`0x5aea7f`) and takes an empty slot at `0x5aea81` to `0x5aea90`. Past 50 it falls to `0x5aea92`, which stores `+0xc`, `+8` and the byte `+0` at `index·0x20 + this` with the index still `-1`. `0x5acb47` `test eax,eax; jl` skips the fields. `0x5acc20` to `0x5acc36` compare all 50 guids with no stop at an empty one. `0x5af691` tests `+0x718` before `0x5ae910`.

## Friend script functions

`GetNumFriends` (`0x5ad000`) returns the count of slots before the first empty one, 0 without a list. `GetSelectedFriend` (`0x5ad260`) returns the selected friend's 1-based index, 0 for none. `GetFriendInfo(index)` (`0x5ad060`) returns six values for a 1-based index: name, level, class name, area name, 1 or nil for online, and a status text. The text is `CHAT_FLAG_DND` for status flag 4, `CHAT_FLAG_AFK` for 2 and `""` otherwise, and only for an online slot. A class or area with no row is `UNKNOWN`; the area name is the parent area's when the area row has a valid non-zero parent. An index outside 1 to 51 returns `UNKNOWN`, 1, `UNKNOWN`, `UNKNOWN`, nil and `""`; index 51 reads the fields that follow the slots.

Addresses: `0x5ad000`, `0x5ad060`, `0x5ad260`, `0x5ae4b0`, `0x5ae490`, `0x5ae510`, `0x5ae810`, `0x5ad0b1`, `0x5ad15c`, `0x5ad17b`
Confidence: verified
Proof: one
Test: none

`0x5ae490` and `0x5ae510` read the guid dwords from `+8` in steps of `0x20` over 50; the first stops at the first slot whose guid is 0, the second compares the guid with `+0x648` and returns `-1` when that is zero or absent. `0x5ae4b0` returns `index·0x20 + this` for an index up to 50 inclusive, else null; index 50 lies on `+0x640`. `0x5ad0b1` calls it with the index less 1, which for 0 or a negative number is a large unsigned value. `0x5ad15c` tests the online byte; `0x5ad17b` tests flag bits 4 then 2. `0x5ae810` compares a guid with all 50 slots, so an empty slot matches the guid 0.

## Friend requests

The Lua `AddFriend` (`0x5ad290`, the call at `0x5ad2c6`) calls `FriendList::AddFriend(name)` (`0x5ae670`), which marks tutorial 21 seen and sends `CMSG_ADD_FRIEND` (`0x69`) with the name through `0x5ab630`. `ShowFriends` (`0x5ad340`) sends `CMSG_FRIEND_LIST` (`0x66`) with no body.

Addresses: `0x5ad290`, `0x5ae670`, `0x5ad340`, `0x4b54c0`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

`0x5ae677` is `mov ecx,0x15; call 0x4b54c0`; `0x5ae683` puts `0x69` with `0x418190`, `0x5ae6ae` the name with `0x418430`, and `0x5ae6b9` sends. `0x5ad349` puts `0x66` and `0x5ad373` sends. The binding table `0x85d830` names `0x5ad340` `ShowFriends`.

## Ignore list functions

`0x5ae550` counts the ignore guids before the first empty one, of 25. `0x5ae5a0(guid)` returns 1 for a non-zero guid among the 25, else 0. `0x5ae5f0(index)` stores the guid of ignore slot `index` as the selection; an index above 25 stores zero. Index 25 is accepted and reads the eight bytes at `+0x718`, the pending count and the dword after it. `0x5ae630` returns the slot holding the selected guid, `-1` when there is no selection or no match.

Addresses: `0x5ae550`, `0x5ae5a0`, `0x5ae5f0`, `0x5ae630`, `0x5ae5f6`
Confidence: verified
Proof: one
Test: none

`0x5ae5f6` `cmp eax,0x19; jbe` accepts 0 to 25; the load is `[ecx+eax*8+0x650]`, which for 25 is `+0x718`. The loops step `0x8` and stop at `0x19` (25) entries.

## Who query tags

`SendWho(filter)` calls `0x5aebb0` when the friend list exists. The filter is copied up to 128 bytes and split at white space; a token in double quotes may hold spaces. A token that begins, without case, with the localized tag `WHO_TAG_NAME`, `WHO_TAG_GUILD`, `WHO_TAG_ZONE`, `WHO_TAG_RACE` or `WHO_TAG_CLASS` sets that part of the query. Name and guild take the text after the tag, up to 47 characters. The race and class masks start all ones; the first such tag zeroes its mask, and every row whose localized name contains the text adds the bit `1 << id`. A zone tag collects up to 10 area rows with no parent whose name contains the text, or the id 0 when none does.

Addresses: `0x5ad3b0`, `0x5aebb0`, `0x5aec90`, `0x5aecf9`, `0x5aedc1`, `0x5aedc8`, `0x5aee08`, `0x5aee59`, `0x5aef06`, `0x5aeeb9`, `0x5aef68`
Confidence: verified
Proof: one
Test: none

`0x5aebbb` sets both masks to `-1`. `0x5aec90` `cmp eax,0x22` toggles the quote state; a character that is not white space, or any character inside quotes, is copied to the token. A tag test is `0x64a4c0` over the tag's length (`0x5aecf9`, `0x5aed38`, `0x5aed77`, `0x5aee3b`, `0x5aeeed`); name and guild text go to 48-byte buffers with `0x64a5a0` (`0x5aed0a`, `0x5aed49`). The zone loop skips a row whose `+8` is non-zero (`0x5aedc1`), appends at most 10 (`0x5aedc8`) and on none stores 0 and a count of 1 (`0x5aee08`). `0x5aee59` and `0x5aef06` zero a mask that is still `-1`. Race and class rows are `0x74` and `0x44` bytes apart (`0x5aeeb9`, `0x5aef68`).

## Who query packet

A token that begins with a digit is a level range: the number is the minimum and the maximum, a `-` after it sets the maximum to 100, and a number after that dash replaces it. A token that begins with `-` is a range from 0 only when a digit follows. Levels are not clamped; the defaults are 0 and 100. Any other token is a search term, up to four; the scan stops after the fourth. The packet is `CMSG_WHO` (`0x62`): minimum, maximum, name and guild C strings, race mask, class mask, the zone count (0 to 10) with the zone ids, then the term count with the C strings

Addresses: `0x5aebb0`, `0x5aef7d`, `0x5aefc4`, `0x5aefd9`, `0x5af02d`, `0x5af0a4`, `0x5af168`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

The minimum, maximum, masks, zone count, each zone id and the term count are all u32: 4-byte puts through `0x418190` and `0x4181f0` (`0x5af0c3` to `0x5af141`). `0x5aebd7` and `0x5aebda` store the defaults 0 and `0x64`. At `0x5aef7d` `iswdigit` on the first character selects the range; `0x64ac60` converts it to both values; the loop at `0x5aefa5` skips digits, and `0x5aefc4` stores 100 after a dash. `0x5aefd9` tests a digit with `_isdigit`; a token that starts with `-` and passes reaches `0x5aeff2` and the maximum, otherwise `0x5af015` keeps it as a term. A digit-start token is a range even when text follows its digits. Terms are 128-byte slots; `0x5af02d` `cmp eax,4; jae` ends the scan. The packet is built at `0x5af0a4` and sent at `0x5af168`.

## Who list reply

`SMSG_WHO` (`0x63`) is two dwords, the count and the server's total, then per record the name (up to 48 bytes), the guild (up to 96), and four dwords: level, class, race, zone. The first 50 records are kept in the array `0xc281b8`, stride `0xa0`: name `+0`, guild `+0x30`, level `+0x90`, race `+0x94`, class `+0x98`, zone `+0x9c`. The kept count `0xc2a120` is the count capped at 50 and `0xc2a124` the total. The array is sorted by the chain (Who sort). The reply then fires `WHO_LIST_UPDATE` when `SetWhoToUI` is on or the raw count is more than 3, and otherwise prints each record to chat as it is read, then a `WHO_NUM_RESULTS` line with the raw count.

Addresses: `0x5adf60`, `0x5adfca`, `0x5ae045`, `0x5ae0a1`, `0x5ae0e2`, `0x5ae13c`, `0x5ae141`, `0x5ae160`, `0x5ad870`, `0x5ad690`, `0xc2a120`, `0xc2a124`, `0xc2a128`, `0xc2a12c`, `0xc281b8`
Confidence: verified
Proof: one
Test: none

`0x5adf74` and `0x5adf7f` read the count and total with `0x418fb0`; `0x5adf87` caps the count at `0x32`. With `SetWhoToUI` (`0xc2a12c`, set at `0x5ad87c`) clear, the threshold is the dword at `+0x28` of the object `0xc2a128` points to, else 3 (`0x5adfc1`). Nothing writes `0xc2a128`: a scan of the image for its address finds only the load at `0x5adfb3`, so the threshold is 3, and `cmp eax,ecx; jl` (`0x5adfca`) takes the event leg when it is below the raw count. The record loop reads `0x4191b0` (caps `0x30`, `0x60`) and four `0x418e30`; it stores while the offset is below `0x1f40` (`0x5ae045`) but prints every record. `qsort` is at `0x5ae0e2`, after the loop. `0x5ae160` formats the name twice, the level, race, class, the guild when it is not empty, and the zone with `WHO_LIST_FORMAT` or `WHO_LIST_GUILD_FORMAT`, then adds the text to chat with type 10 (`0x49a870`); a name with no row is `UNKNOWN`. `0x5ae0f6` passes the raw count to `WHO_NUM_RESULTS`.

## Who results function

`GetWhoInfo(index)` returns the six values name, guild, level, race, class and zone for a 1-based index into the array. Race, class and zone are the localized names, or `UNKNOWN` when the id has no row. An index that is 0 or more than the kept count plus one returns nil, nil, 0, nil, nil, nil; an index equal to the kept count plus one is accepted and reads the next slot of the array.

Addresses: `0x5ad6e0`, `0x5ad71b`, `0x5ad71f`, `0x5ad782`, `0x5ad7c5`, `0x5ad816`, `0x5ad836`
Confidence: verified
Proof: one
Test: none

`0x5ad70f` converts the argument with `__ftol`; `0x5ad71a` decrements it and `0x5ad71b` `cmp eax,[0xc2a120]; ja 0x5ad836` is an unsigned compare, so the index `count + 1` passes. The slot is `index·0xa0 + 0xc281b8`. Each of the three names has a branch to `UNKNOWN` through `0x703bf0` (`0x5ad782`, `0x5ad7c5`, `0x5ad816`). `0x5ad836` pushes nil, nil, the number 0, nil, nil, nil.

## Who sort keys

`SortWho(type)` maps the text, without case, to a key: `name` 4, `level` 1, `class` 2, `group` 3, `race` 5, `zone` 0, `guild` 6, and any other text to 4. It moves that key to the front of a seven-slot chain of key and direction pairs. The direction flips only when the key was already first; otherwise it keeps the direction it had and the earlier slots shift down. It then sorts the kept results with `qsort` and fires `WHO_LIST_UPDATE` before it returns. The chain starts as keys 0 to 6 with direction 0, set once at start-up.

Addresses: `0x5ad890`, `0x5ad8bb`, `0x5ad982`, `0x5ad99e`, `0x5ad9ab`, `0x5ad9c3`, `0x5ad9d6`, `0x5ad9ed`, `0x5add04`, `0xc2817c`, `0xc28180`
Confidence: verified
Proof: one
Test: none

`0x5ad8bb` sets the default key 4; the seven `0x64a4c0` compares run in the order name, level, class, group, race, zone, guild. `0x5ad982` searches the chain; `0x5ad99e` to `0x5ad9a7` flips the direction only for slot 0; `0x5ad9ab` to `0x5ad9c1` shift; `0x5ad9c3` and `0x5ad9c9` store slot 0. `0x5ad9d6` passes the comparator `0x5ada00` to `0x73f727` over `0xc281b8` with the kept count and `0xa0`. `0x5ad9ed` is `mov ecx,0x184; call 0x703e50`. `0x5add04` stores `key[i] = i` and `dir[i] = 0` for 7 slots.

## Who comparator

The comparator `0x5ada00` walks the chain and returns the first non-zero comparison, negated for a slot whose direction is set, and 0 when all seven tie. At direction 0 the earlier record is the one with the earlier name or guild (without case), the earlier localized zone, class or race name, or the lower level. An id with no row in the class, race or zone table, on either record, ties on that key and the walk goes on to the next slot. Key 3 never orders anything.

Addresses: `0x5ada00`, `0x5ada40`, `0x5ada58`, `0x5ada7b`, `0x5adadf`, `0x5adb40`, `0x5adb99`, `0x5adbb2`, `0x5adbc4`, `0x5adbd8`
Confidence: verified
Proof: one
Test: `net::friends_who_compare`

`0x5ada40` jumps through `0x5adbd8` on the key. The string keys call `0x64a4c0` on the texts at `+0` and `+0x30` with the bound `0x7fffffff`. Level compares the dwords at `+0x90` (`0x5ada58`): equal ties, otherwise `setge` gives 1 or `-1`. Class, race and zone use `ChrClasses` (`0xc0def4`, name `+0x14`), `ChrRaces` (`0xc0dee0`, `+0x44`) and `AreaTable` (`0xc0e048`, `+0x2c`) rows: a negative id, an id above the table's last, or a null row gives 0 for that side (`0x5ada7b`, `0x5adadf`, `0x5adb40`) and `0x5adbb2` goes to the next slot. `0x5adbc4` negates the result when the slot's direction is non-zero.

## Reverse whois

`SMSG_RWHOIS` (`0x1fe`) is a u32 count: `-1` writes `RWhoIs failed` and 0 writes `Not found` to the console. Otherwise each of the `count` accounts is a C string of up to 64 bytes written as `Account: <name>`, a u32 character count, a u32 index, and that many character names (up to 48 bytes), each written on a line of its own; the character whose position equals the index is written in console colour 4 and the others in colour 0. The counts and the index are zeroed before each read, so a short packet reads 0; a name that is cut short, or that fills its buffer before its NUL, is written empty.

Addresses: `0x5adde0`, `0x5ade03`, `0x5ade23`, `0x5ade66`, `0x5adeb6`, `0x63cd00`, `0x63cd50`
Confidence: verified
Proof: one
Test: `net::friends_reverse_whois`

The first read is `0x418eb0`; the tests are `cmp eax,-1` (`0x5addfe`) and `cmp eax,esi` against 0 (`0x5ade1f`). The account is `0x4191b0` with cap `0x40` and goes to `0x63cd50` (`Account: %s`); the count and index are two `0x418e30` reads; each name is `0x4191b0` with cap `0x30` to `0x63cd00` with the colour `(index != i ? 0 : -1) & 4` (`0x5adea5` to `0x5adead`). `0x5addf3`, `0x5ade74` and `0x5ade82` zero the dwords before their reads, and `0x4191b0` writes 0 at the buffer's start when its read ends past the store (`systems/net/messages.md`).

# net: friends

The friend and ignore lists. The `/who` query and its results, kept in the same object file, are in `who`.

`FriendList.cpp` holds one object, `0xc28168`, of `0x728` bytes, built at each world entry and freed at teardown. It keeps 50 friend slots and 25 ignore guids, filled by server packets, and the list of who results the `/who` reply fills. The Lua functions that read them (`GetFriendInfo`, `GetWhoInfo`, `SortWho` and the rest) are in the same file and are registered as 19 pairs of name and function in the table `0x85d830`. Names of players come from the name cache (`0xc0e228`): a slot or ignore that waits for a name is counted in `+0x640` and `+0x718`, and the lists sort and signal their update event when the count reaches zero.

Roots: `0x5adc50`, `0x5ad000`, `0x5ad060`, `0x5ad260`, `0x5ae5a0`, `0x5ae670`, `0x5ae810`, `0x5ae490`, `0x5ae510`, `0x5ae550`, `0x5ae5f0`, `0x5ae630`

## Friend list object

The object is cleared by `0x5aca10`. Friend slot `k` is 32 bytes at `+0x20·k`: byte `+0` online, byte `+1` status flags (2 is away, 4 is busy), `+4` the name string pointer, `+8` the guid, `+0x10` level, `+0x14` class id, `+0x18` area id. `+0x640` counts friend names pending, `+0x648` is the selected friend's guid, the ignore guids are 8 bytes each from `+0x650`, `+0x718` counts ignore names pending and `+0x720` is the selected ignore's guid. `0x5adc50` creates the object, registers handlers for opcodes `0x63`, `0x65`, `0x1fe`, `0x67`, `0x68` and `0x6b` and the console command `whois`, and sets the who sort chain and counts. `0x5ae2d0` removes them, destroys the object and zeroes `0xc28168`; the two run at world entry and teardown.

Addresses: `0x5adc50`, `0x5aca10`, `0x85d830`, `0x5adc7b`, `0x5adc98`, `0x5adca8`, `0x5adcb8`, `0x5adcc8`, `0x5adcd8`, `0x5adce8`, `0x5adcfd`, `0xc28168`, `0x5ae2d0`, `0x5ae340`, `0x401666`, `0x401fc6`
Confidence: verified
Proof: one
Test: none

`0x5adc50` returns when `0xc28168` is set; it allocates `0x728` (`0x5adc6b`) and calls `0x5aca10`. The handlers are `0x5adf60` (`0x63`), `0x5adda0` (`0x65`), `0x5adde0` (`0x1fe`), `0x5add80` (`0x67`), `0x5add30` (`0x68`) and `0x5ae2b0` (`0x6b`), each through `0x5ab650`; `0x5adee0` is the handler of `whois` (`0x5adcfd`, `0x63f9e0`). `0x5adc50` has one caller, `0x401666`, and no stored address; `0x5ae2d0` has one, `0x401fc6`, and `0x5ae340` is its store of 0 to `0xc28168`. A scan of `call` and `jmp` for `0x5adc50` and of the image for its address as a dword finds that one site; the control is `0x5ab1a0`, which finds its four.

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

`SMSG_FRIEND_STATUS` (`0x68`) is a result byte and a guid, read by `0x5add30` and passed to `0x5acab0` with the packet; the handler returns 1 even when there is no friend list. The result selects an action and a game error `0x496720` shows. Results 2, 3, 5, 6, 7 and 8 name a friend and 14, 15 and 16 an ignored player, so their message waits for a cache lookup, finished by `0x5ace70` for a friend and `0x5acf40` for an ignore (`0x5acb7d`, `0x5acd1e`, `0x5acd42`, `0x5acd64`).

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

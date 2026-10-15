# net: who

The `/who` query, its results and the reverse whois reply.

They live in `FriendList.cpp` beside the friend and ignore lists (`friends`). `SendWho(filter)` turns a filter into a `CMSG_WHO` packet; the server's `SMSG_WHO` reply fills an array of 50 results that `GetWhoInfo` reads and `SortWho` orders; `SMSG_RWHOIS` writes accounts and their characters to the console.

Roots: `0x5ad3b0`, `0x5ad690`, `0x5ad6e0`, `0x5ad870`, `0x5ad890`

## Who query tags

`SendWho(filter)` calls `0x5aebb0` when the friend list exists. The filter is split at white space (Who filter copy); a token in double quotes may hold spaces. A token that begins, without case, with the localized tag `WHO_TAG_NAME`, `WHO_TAG_GUILD`, `WHO_TAG_ZONE`, `WHO_TAG_RACE` or `WHO_TAG_CLASS` sets that part of the query. Name and guild take the text after the tag, up to 47 characters. The race and class masks start all ones; the first such tag zeroes its mask, and every row whose localized name contains the text adds the bit `1 << (id mod 32)`. A zone tag collects up to 10 area rows with no parent whose name contains the text, or the id 0 when none does.

Addresses: `0x5ad3b0`, `0x5aebb0`, `0x5aec90`, `0x5aecf9`, `0x5aedc1`, `0x5aedc8`, `0x5aee08`, `0x5aee59`, `0x5aef06`, `0x5aeeb9`, `0x5aef68`
Confidence: verified
Proof: one
Test: `net::who_query_packet`

`0x5aebbb` sets both masks to `-1`. `0x5aec90` `cmp eax,0x22` toggles the quote state; a character that is not white space, or any character inside quotes, is copied to the token. A tag test is `0x64a4c0` over the tag's length (`0x5aecf9`, `0x5aed38`, `0x5aed77`, `0x5aee3b`, `0x5aeeed`); name and guild text go to 48-byte buffers with `0x64a5a0` (`0x5aed0a`, `0x5aed49`). The zone loop skips a row whose `+8` is non-zero (`0x5aedc1`), appends at most 10 (`0x5aedc8`) and on none stores 0 and a count of 1 (`0x5aee08`). `0x5aee59` and `0x5aef06` zero a mask that is still `-1`. Race and class rows are `0x74` and `0x44` bytes apart (`0x5aeeb9`, `0x5aef68`), and `shl edx,cl` (`0x5aeeab`) takes the id mod 32. The search `0x64b540` (`ecx` the row's name, `edx` the text) returns the first place in the name where `0x64a4c0` over the text's length is 0, or null for an empty name, no match or a null argument; an empty text matches any other name.

## Who filter copy

The filter is copied up to 127 bytes and a space is appended, so the last token ends. Double quotes toggle, are dropped, and keep white space in a token; a token still open at the end is dropped. The copy's 128 bytes are followed on the stack by the name buffer. A filter of 127 bytes or more puts the space in the copy's last byte and its NUL in the name's first, so a name a tag sets earlier in the filter runs on from the copy and its words are parsed too: with `n-` as `WHO_TAG_NAME`, from `"n-a b"` the word `a` becomes a term, while `b`, which no space ends, is dropped.

Addresses: `0x5aebc8`, `0x5aebf2`, `0x5aec12`, `0x5aec1a`, `0x5aec90`, `0x5aecda`, `0x5af07c`
Confidence: verified
Proof: one
Test: `net::who_query_packet`

`0x5aebf2` is `SStrCopy` into `[ebp-0x128]` with the bound `0x80`, which keeps 127 bytes and a NUL. `0x5aec12` and `0x5aec1a` write the space and a NUL at the copy's length; at 127 the NUL lands at `[ebp-0xa8]`, the name buffer (`0x5aebe4` zeroes its first byte; `0x5aed14` copies a name there). `0x5aec90` `cmp eax,0x22` toggles the quote flag and steps past the quote. `0x5af07c` stores a token byte at `[edi+esi]` with no bound; `0x5aecda` ends the token with a NUL at a white space outside quotes. The scan ends when `0x41aab0` returns `-1` at a NUL (`0x5af099`).

## Who filter decoding

The copy is read as UTF-8, one code point at a time. A code point goes to the token as every byte the decoder read, so a broken sequence takes the byte that broke it, a space after a lone lead byte included, and a NUL inside a sequence ends the scan. White space is `iswspace` of the code point's low 16 bits: `0x09` to `0x0d` and the space from the C table, nothing from `0x80` to `0xff` (so not `U+00A0`), and from `0x100` on what Windows' `GetStringTypeW` says, such as `U+3000`. The overlong `C0 A0` and `U+10020` are spaces too.

Addresses: `0x41aab0`, `0x5af06f`, `0x5aec2b`, `0x5af03d`, `0x5aecb8`, `0x74006f`, `0x7458e4`, `0x413ed3`, `0x8824a0`, `0x831724`
Confidence: verified
Proof: one
Test: `net::who_query_packet`

`0x41aab0` takes up to five continuations by the lead's high bits with no overlong check; a stray continuation, or a byte that is not `10xxxxxx`, gives `0x80000000`, and the count it stores at `[ebp-0x10]` includes that byte. `0x5af06f` to `0x5af083` copy that many bytes. The skips at `0x5aec2b` and `0x5af03d` and the test at `0x5aecb8` push the code point to `0x74006f`, `_iswctype` (`0x7458e4`) with `_SPACE`, which reads a 16-bit unit: `0xffff` is 0, below `0x100` the table `_ctype` (`0x831724`, read at its second entry), else `___crtGetStringTypeW` (`0x413ed3`), which calls the system through `0x8824a0`. The table has no bit for `0x80` to `0xff`. Its pointers (`0x83171c`, `0x831720`), `__mb_cur_max` (`0x831710`) and the decimal point (`0x831714`) are written only by the category initializers `0x41237d` and `0x411eff`, named only in `__lc_category` (`0x831bd8`), which only `_setlocale` (`0x4109d6`) and `0x410c1c` read; nothing calls or names `_setlocale`, so they keep their initial values.

## Who level digits

A byte of `0x80` or more begins no level range. After a range's digits, or a leading `-`, the bytes `0xf7`, `0xfb` and `0xfd` pass the digit test, and the maximum becomes 0, the value of no digits.

Addresses: `0x5aef7d`, `0x740050`, `0x5aefd9`, `0x73fc0f`, `0x831710`, `0x83171c`, `0x831720`, `0x831714`
Confidence: verified
Proof: one
Test: `net::who_query_packet`

`0x5aef7d` passes the first byte to `iswdigit` (`0x740050`) sign-extended to 16 bits by `movsx ax`, as does the loop at `0x5aefa5`: a high byte becomes `U+FF80` to `U+FFFF`, none of them a decimal digit in Unicode, and `U+FFFF` is 0 without asking. `0x5aefd9` passes the next byte, sign-extended, to `_isdigit` (`0x73fc0f`), which with `__mb_cur_max` 1 (`0x831710`) reads the table entry at the byte, so a high byte reads the 128 bytes before `_ctype`'s second entry. Of those, the `_DIGIT` bit is set only in `0x26`, the low byte of `_pctype` (`0x83171c`) and `_pwctype` (`0x831720`), read for `0xfb` and `0xfd`, and in the decimal point `.` (`0x831714`), read for `0xf7`. `0x64ac60` reads no digit in any of them.

## Who query packet

A token that begins with a digit is a level range: the number is the minimum and the maximum, a `-` after it sets the maximum to 100, and a number after that dash replaces it. A token that begins with `-` sets only the maximum, and only when a digit follows; the minimum keeps 0 or an earlier range's. Levels are not clamped; the defaults are 0 and 100. Any other token is a search term, up to four; the scan stops after the fourth. The packet is `CMSG_WHO` (`0x62`): minimum, maximum, name and guild C strings, race mask, class mask, the zone count (0 to 10) with the zone ids, then the term count with the C strings.

Addresses: `0x5aebb0`, `0x5aef7d`, `0x5aefc4`, `0x5aefd9`, `0x5af02d`, `0x5af0a4`, `0x5af168`, `0x5ab630`
Confidence: verified
Proof: one
Test: `net::who_query_packet`

The minimum, maximum, masks, zone count, each zone id and the term count are all u32: 4-byte puts through `0x418190` and `0x4181f0` (`0x5af0c3` to `0x5af141`). `0x5aebd7` and `0x5aebda` store the defaults 0 and `0x64`. At `0x5aef7d` `iswdigit` on the first character selects the range; `0x64ac60` converts it to both values; the loop at `0x5aefa5` skips digits, and `0x5aefc4` stores 100 after a dash. `0x5aefd9` tests a digit with `_isdigit`; a token that starts with `-` and passes reaches `0x5aeff2` and the maximum, otherwise `0x5af015` keeps it as a term. A digit-start token is a range even when text follows its digits. `0x64ac60` reads an optional `-` and then decimal digits in wrapping 32-bit arithmetic, 0 when no digit follows. Terms are 128-byte slots; `0x5af02d` `cmp eax,4; jae` ends the scan. The packet is built at `0x5af0a4` and sent at `0x5af168`.

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

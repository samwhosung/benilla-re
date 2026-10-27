# dbcache: records

The decoders of the twelve record types the caches hold: each type's `Read(this = record, stream)`, which fills a record from the stream in a fixed order, and the layouts they make. The caches that call them, the receive and lookup code and the `.wdb` files are in `dbcache/overview`.

Roots: `0x7c8120`, `0x7c87c0`, `0x7c8dc0`, `0x7c9640`, `0x7c9fa0`, `0x569d00`

## Record decoders

Each type has a `Read(this = record, stream)` that fills a record in a fixed order, the key having been read by `Receive`. The saved record sizes are creature `0x34`,
game object `0x7c`, item name 4, item `0x1d4`, NPC text `0x140`, guild `0x2f8`, quest `0x18dc`, page text `0x404`, pet name `0x58`, petition `0x13c8`, name `0x148`
and item text `0x1f40`. A string is either read into the record, or duplicated with `SStrDupA` and stored as a pointer; an empty one after the first in a run of names is
duplicated from the previous slot.

Addresses: `0x7c8120`, `0x7c87c0`, `0x7c8dc0`, `0x7c9640`, `0x7c9fa0`, `0x62f260`, `0x6dbf60`, `0x7cabd0`, `0x6dc290`, `0x7c7a30`, `0x6dc1f0`, `0x569d00`, `0x64a620`
Confidence: verified
Proof: two
Test: `dbcache::dbcache_creature`, `dbcache::dbcache_gameobject`, `dbcache::dbcache_itemstats`, `dbcache::dbcache_npctext`, `dbcache::dbcache_item_name`, `dbcache::dbcache_pet_name`, `dbcache::dbcache_page_text`, `dbcache::dbcache_name`, `dbcache::dbcache_guild`, `dbcache::dbcache_quest`, `dbcache::dbcache_petition`, `dbcache::dbcache_item_text`

The tests run each decoder whole against the exe's own code: `0x7c8120` creature, `0x7c87c0` game object, `0x7c8dc0` item name, `0x7c9640` item, `0x7c9fa0` NPC text, `0x62f260` guild,
`0x6dbf60` quest, `0x7cabd0` page text, `0x6dc290` pet name, `0x7c7a30` petition, `0x6dc1f0` name, and the item text inside `0x569d00`. The sizes are the pushes of the saving
functions (`0x5567e8` and the like); for the name, guild, pet name and petition caches those saves never write.

## Creature and game object

Creature (`0x7c8120`): four name pointers at `+0`, `+4`, `+8`, `+0xc`; the subname pointer at `+0x10`, always duplicated; seven dwords at `+0x14` to `+0x2c`; two bytes stored as
0 or 1 at `+0x30` and `+0x31`. Game object (`0x7c87c0`): dwords at `+0` and `+4`; five string pointers at `+8` to `+0x18`; 24 dwords at `+0x1c` to `+0x78`.

Addresses: `0x7c8120`, `0x7c87c0`
Confidence: verified
Proof: two
Test: `dbcache::dbcache_creature`, `dbcache::dbcache_gameobject`

Read orders as the decoders make them, in the stream's order. The first four names are read in a loop; the fifth string is read apart.

## Item and NPC text

Item (`0x7c9640`, `0x1d4` bytes): dwords at `+0`, `+4`; four string pointers `+8` to `+0x14`; 20 dwords `+0x18` to `+0x64`; two arrays of ten at `+0x68` and `+0x90`, read as
pairs; five groups of a float at `+0xb8…`, a float at `+0xcc…` and a dword at `+0xe0…`; seven dwords from `+0xf4`; the rest to `+0x1d0`, with a description pointer at `+0x198`, not written when empty.
NPC text (`0x7c9fa0`, `0x140`): eight times: a float at `+i·4`, two string pointers at `+0x20 + i·8` and `+0x24 + i·8`, a dword at `+0x60 + i·4`, then three emote pairs at `+0x80` and `+0xe0` strides.

Addresses: `0x7c9640`, `0x7c9fa0`, `0x7c98d6`, `0x7c98ee`, `0x55bef6`, `0x7ca023`
Confidence: verified
Proof: two
Test: `dbcache::dbcache_itemstats`, `dbcache::dbcache_npctext`

The item description is duplicated only when non-empty, and an empty one skips the store (`0x7c98d6` past `0x7c98ee`), so `+0x198` keeps its value: 0 in a fresh node, the old pointer when the receive decodes into a node again (`0x55bef6`). Each NPC text string is duplicated only when non-empty; otherwise the pointer is 0 (`0x7ca023`).

## Guild and quest

Guild (`0x62f260`, `0x2f8`): a dword at `+0`; the name, 96 bytes at `+4`; ten rank names of 64 bytes from `+0x64`; five dwords at `+0x2e4` to `+0x2f4`. Quest (`0x6dbf60`, `0x18dc`): 15 dwords `+0`
to `+0x38`; four pairs, six pairs and a run of dwords with two floats at `+0x90`, `+0x94`; Title at `+0x9c`, Objectives `+0x29c`, Details `+0xa9c`, EndText `+0x129c`; four arrays of four objective dwords
at `+0x149c`, `+0x14ac`, `+0x14bc` and `+0x14cc`, read interleaved; four objective texts of 256 bytes from `+0x14dc`.

Addresses: `0x62f260`, `0x6dbf60`, `0x4191b0`, `0x6dc097`, `0x6dc0e3`, `0x6dc0f0`, `0x62f273`, `0x62f290`, `0x4192aa`, `0x4192be`, `0x5552b0`
Confidence: verified
Proof: two
Test: `dbcache::dbcache_guild`, `dbcache::dbcache_quest`

The quest strings are read without a bound (`0x7fffffff`, from `0x6dc097`), so a field's size is the layout. The objective loop (`0x6dc0f0` to `0x6dc123`) runs four times with `ebx` at `+0x14ac` (`0x6dc0e3`) and reads at `ebx − 0x10`, `ebx`, `ebx + 0x10` and `ebx + 0x20`, then adds 4 to `ebx`: the arrays start at `+0x149c`, `+0x14ac`, `+0x14bc` and `+0x14cc`. The guild decoder bounds its name at `0x60` (`0x62f273`) and each rank at `0x40` (`0x62f290`): when `0x4191b0` copies that many bytes without a NUL it sets the stream past its end (`0x4192aa` to `0x4192ae`) and empties the string (`0x4192be`), so a guild name of 0x60 characters or more reads as empty, later reads fail and the handler refuses (`0x5552b0`).

## Quest record dwords

`+0x24` is the id of the quest that follows; `+0x34` is the item the quest hands out; `+0x8c` is a column nothing reads.

Addresses: `0x5dc4d7`, `0x5dc6d7`, `0x5dc751`, `0x4e00ce`, `0x4e04fa`, `0x6dc05f`, `0x562286`, `0x562da9`, `0x6dbe5c`
Confidence: inferred
Proof: none
Test: none

`+0x24` is only tested for zero: `0x5dc4d7` and `0x5dc751`, in the quest-complete readers `0x5dc400` and `0x5dc710`, print message `0x8a` with the title (`+0x9c`) when it is 0, and `0x5dc6d7` turns it into 0 or 1. Where the install's `questcache.wdb` has it set, the value is another quest's id. `+0x34` is compared with each required item id (`0x4e00ce` in `GetNumQuestLeaderBoards`, `0x4e04fa` in `GetQuestLogLeaderBoard`): a required item equal to it is not counted as an objective line. `+0x8c` is read by the decoder (`0x6dc05f`), zeroed by the record clear (`0x562286`), moved by the node copy (`0x562da9`) and put by the file writer (`0x6dbe5c`); no instruction in the 42 functions that call `0x562a40`, nor in the code around them (`0x4de510` to `0x4e1600`, `0x4e8c80` to `0x4e9bd0`, `0x5dc400` to `0x5ddf30`, `0x5e5ad0` to `0x5e5ea0`), reads it. The control, `+0x24`, shows three reads in the same ranges.

## Pages, pets and item texts

Page text (`0x7cabd0`, `0x404`): a string of up to `0x400` bytes at `+0`, a dword at `+0x400`. Pet name (`0x6dc290`, `0x58`): a dword at `+0x50`, a string of up to `0x50` bytes at `+0`, a dword at `+0x54`. Item name
(`0x7c8dc0`): one string, duplicated, at `+0`. Item text: a key with its top bit set refuses; otherwise a string of up to 8,000 bytes at `+0`, and the key stored at `+0x1f40` with a valid byte at `+0x1f44`.

Addresses: `0x7cabd0`, `0x6dc290`, `0x7c8dc0`, `0x569d00`
Confidence: verified
Proof: two
Test: `dbcache::dbcache_page_text`, `dbcache::dbcache_pet_name`, `dbcache::dbcache_item_name`, `dbcache::dbcache_item_text`

The item text is decoded inside the receive function `0x569d00`.

## Names and petitions

Name (`0x6dc1f0`): a guid at `+0x130`, a name of up to `0x30` bytes at `+0`, a server name of up to `0x100` at `+0x30`, then dwords at `+0x138`, `+0x13c`, `+0x140`. Petition (`0x7c7a30`): a dword at `+0`; a guid at `+8`;
strings at `+0x10` and `+0x110`; eight dwords from `+0x1110`; a 16-bit value at `+0x1130`; dwords at `+0x1134`, `+0x1138`; a count at `+0x13bc` and that many 64-byte strings from `+0x113c`; a dword at `+0x13c0`.

Addresses: `0x6dc1f0`, `0x7c7a30`
Confidence: verified
Proof: two
Test: `dbcache::dbcache_name`, `dbcache::dbcache_petition`

The petition's count is a signed dword; the loop runs zero times for 0 or less.

# ui: add-on record lookups

Three lookups of `AddOns.cpp` that answer one field of an add-on's record by its folder name, for
the glue's `GetAddOnInfo` and `LaunchAddOnURL` and the in-game `GetAddOnInfo`: the notes, the URL and
the update flag. The records, the hash that holds them and the reply that writes the URL and the flag
are `ui/addons`.

Roots: `0x51e050`, `0x51e180`, `0x51e3e0`

## Finding a record by name

Each lookup hashes the name with `SStrHashHT` and walks the bucket `hash & mask` of the add-on hash,
taking the first record whose stored hash equals it and whose folder name `+0x14` equals the name
ignoring ASCII case. A bucket mask of −1 (no buckets), an empty bucket or no match answers null, 0 or
false. `0x51e050` then looks `Notes` up the same way in the record's metadata hash and answers its
value, or null. `0x51e180` answers the URL `+0x30` when it is not null, not empty and the record's
security `+0x24` is not 2 (banned), else null. `0x51e3e0` answers the byte `+0xc0`, set when the
realm's reply gave a revision above the installed one.

Addresses: `0x51e050`, `0x51e054`, `0x51e067`, `0x51e079`, `0x51e07e`, `0x51e0a2`, `0x51e0b0`, `0x51e0d0`, `0x51e0d5`, `0x51e0e7`, `0x51e0ff`, `0x51e13b`, `0x51e16c`, `0x51e180`, `0x51e1fc`, `0x51e203`, `0x51e208`, `0x51e20e`, `0x51e3e0`, `0x51e45c`, `0x51e468`, `0xbe1b7c`, `0xbe1b84`, `0x8537f4`, `0x64b3f0`, `0x64a4c0`, `0x51db0d`
Confidence: verified
Proof: one
Test: `ui::addon_notes_lookup_difftest`, `ui::addon_url_lookup_difftest`, `ui::addon_update_flag_lookup_difftest`

`0x51e054` tests the mask `0xbe1b84` for −1; `0x51e067` hashes the name, `0x51e079` masks it and
`0x51e07e` reads the bucket's head at `+8` of its 12 bytes in `0xbe1b7c`; a tagged or null head ends
the walk. `0x51e0a2` compares `[rec+0]` with the hash and `0x51e0b0` calls `SStrCmpI([rec+0x14],
name, 0x7fffffff)`; a miss follows the link at the bucket's offset `+0` into the record
(`0x51e0d0`). The metadata walk is the same over `+0xb4` with the mask `+0xbc` (`0x51e0d5`,
`0x51e0ff`), the key `Notes` at `0x8537f4` (`0x51e0e7`, `0x51e13b`) and the value `+0x18`
(`0x51e16c`). `0x51e1fc` loads `+0x30`, `0x51e203` tests its first byte, `0x51e208` compares `+0x24`
with 2 and `0x51e20e` clears `eax`. `0x51e45c` loads `+0xc0` into `al`, and a miss clears `al` alone
(`0x51e468`), so the answer is a byte. The writer of `+0xc0` is the reply's `0x51db0d`; the URL comes
from the add-on's `.url` file at registration or the reply (`ui/addons`, Secure add-ons). The tests
run the three whole over hashes built in memory, with `SStrHashHT` and `SStrCmpI` run from the image.

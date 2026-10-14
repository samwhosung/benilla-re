# ui: the player token before the player object

The interface loads and runs addon updates before the local player object exists. In that window four functions still answer
for `"player"` from a record the character screen filled in, and the rest answer from objects that are not there. This doc is the
record, its readers and what each unit function answers in the window.

Roots: `0x517020`, `0x517e90`, `0x518200`, `0x518350`

## The selected character record

`0xc27d80` holds a 0x110-byte copy of the selected character's entry from the character list. Fields: `+0x08` the name (NUL-terminated), `+0x100` the race byte, `+0x101` the class byte,
`+0x102` the sex byte, `+0x108` the level byte (nothing reads it). It sits in the zeroed tail of the data section, so it is empty at start-up. `0x5abd90(src)` is the one
runtime writer: `rep movsd` of 0x44 dwords. Its one caller is `CGlueMgr::EnterWorld` (`0x46b60f`), reached from the glue script `EnterWorld`, after the character, realm and connection checks and before the
loading screen is raised. Nothing zeroes the record afterwards, so it keeps the last selected character through a logout.

Addresses: `0x5abd90`, `0x5abd94`, `0x5abd99`, `0x5abd9e`, `0x5abdc0`, `0x5abdd0`, `0x5abde0`, `0x5abdf0`, `0x5abe00`, `0x46b500`, `0x46b60f`, `0x46d3c0`, `0xc27d80`, `0xc27d88`
Confidence: verified
Proof: one
Test: none

The accessors return the name pointer or null for an empty first byte (`neg al; sbb eax,eax; and eax,0xc27d88`), and a byte that the callers widen themselves. `0x46b60f` is inside the stretch of
`EnterWorld` after the guards (`0x46b50d`, `0x46b53e`, `0x46b559`, `0x46b56c`) with no branch out, and the screen state 8 that starts the interface load is stored last, so the copy
always precedes the interface. The 19 callers of `0x5abdc0` read it only. The three words at `+0x44` to `+0x4c` are zeroed by the object's static initialiser.

## The four readers

`UnitName`, `UnitSex`, `UnitRace` and `UnitClass` compare their token with the whole string `player` (`SStrCmpI`, `0x7fffffff`) and, on a match, answer from the record without the resolver or an object.
`UnitName` pushes the name and then nil (two values; the realm slot is always nil here). `UnitSex` pushes `{2, 3, 1, 6}[sex byte]` from the table `0x808be4`, with no bound. `UnitRace` and
`UnitClass` push the localised name and the token of the ChrRaces or ChrClasses row of the byte, or `nil, nil` when the id is out of range or its row is missing. The record wins over the object for these
four forever.

Addresses: `0x517020`, `0x51706a`, `0x517076`, `0x517083`, `0x51708a`, `0x51708c`, `0x51709c`, `0x517e90`, `0x517ef0`, `0x517ef9`, `0x517f06`, `0x518200`, `0x518350`, `0x5182f9`, `0x518449`, `0x808be4`, `0x847894`
Confidence: verified
Proof: one
Test: none

The race and class bounds pass for index 0 and the nil comes from the null row: the id tables are indexed by id and slot 0 is never filled, as are ids 6 and 10 of ChrClasses. With the empty record `UnitName("player")` is `nil, nil`
and `UnitSex("player")` is the number 2. A name of an empty record is nil because `lua_pushstring` of null tail-jumps to `lua_pushnil`. `UnitRace` pushes the name at `+0x44 + 4*locale` and the file token at `+0x3c`; `UnitClass` the name at `+0x14 + 4*locale` and the upper-case token at `+0x38`. The locale index is
the word `0xc0e080`, whose one writer is `0x402562` at start-up.

## Race and class of other units

For any other token `UnitRace` and `UnitClass` read the byte of a held unit object (`[[obj+0x110]+0x78]` race, `+0x79` class) and, for an unheld one, the name cache record's `+0x138` and `+0x140`, queried with a null
callback so no name request is sent. The same ChrRaces and ChrClasses lookup then gives the two values, or two nils on any miss. Both raise `Usage: UnitRace("unit")` and `Usage: UnitClass("unit")` for a
nil or missing argument, before anything else; `UnitRace("target")` with no target gives `nil, nil`.

Addresses: `0x518200`, `0x518225`, `0x518260`, `0x51826e`, `0x518301`, `0x518307`, `0x51830c`, `0x518329`, `0x518330`, `0x518350`, `0x5183b0`, `0x51844b`, `0x518457`, `0x51845c`, `0x518479`, `0x518480`, `0x55f080`, `0xc0dee0`, `0xc0dee4`, `0xc0def4`, `0xc0def8`
Confidence: verified
Proof: one
Test: none

Both functions return 2 values on every returning path (`mov eax,2` at `0x518316` and `0x51833c`, `0x518466` and `0x51848c`). The row pointers come from the id arrays at `0xc0dee0` and `0xc0def4`.

## What else answers in the window

With the record present and no player object, the active player GUID pair of the object manager is still 0 (`0x468550` returns it as stored), so `player` resolves to GUID 0 (the
resolver reads the GUID from the object's descriptor, not from the manager). `UnitExists("player")` pushes nil: the object lookup misses, the roster fallback `0x491900` rejects a zero GUID
in its first three instructions (`0x4e80aa`, `0x4bafda`), and the nil path pushes. `UnitLevel("player")` pushes the number 0, since the object lookup and both record
lookups miss. Without the zero guards the compare of 0 with the still-zero active player GUID would hold and `UnitExists("player")` would be 1.

Addresses: `0x515fb0`, `0x515fd9`, `0x515fe8`, `0x515ff8`, `0x516001`, `0x516012`, `0x491900`, `0x4e80a0`, `0x4e80aa`, `0x4bafd0`, `0x4bafda`, `0x517fc0`, `0x518110`, `0x51813e`, `0x464870`, `0x46487b`
Confidence: verified
Proof: one
Test: none

The window exists because the update loop runs an addon's `OnUpdate` in the same iteration that builds the interface, and the player object arrives with a later packet. `UnitIsVisible` is the same presence test as
`UnitExists` without the roster fallback (`unitinfo#unitisvisible`).

## Friend and cooperation fallbacks

`UnitIsFriend` and `UnitCanCooperate` take two tokens. When either object is missing, or the reaction is below 4, each side that is exactly the string `player` hands the other side's GUID to `0x491900`: friendly when that GUID is
the player's own, its charm or summon, or on the party or raid roster. This needs no object, and it is not the record.

Addresses: `0x516930`, `0x5169dc`, `0x5169e1`, `0x5169f3`, `0x5169fc`, `0x516a04`, `0x516a16`, `0x516a1f`, `0x516a70`, `0x516b20`, `0x516b43`
Confidence: verified
Proof: one
Test: none

The reaction test is `0x6061e0(a, b)` and `cmp eax,4; jge` (`0x5169d7`, `0x5169da`). The `player` compares at `0x5169e1`, `0x516a04`, `0x516b20` and `0x516b43` are whole-string compares.

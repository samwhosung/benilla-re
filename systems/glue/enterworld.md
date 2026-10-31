# glue: entering and leaving the world

What the glue does around world entry and return: the remembered character, the event ids, the
sends and the links.

`EnterWorld` (`0x46b500`) and the `Idle` arm of state 8 start the world; the session's logout and
refusal handlers rebuild the glue and send the next character list request. This doc holds what the
flow doc does not: the persisted selection, the glue's own event id space, the add-on event, the
entry camera, what the two phases wait on, the realmd link, the return to character select and
the character name check. The screen states are `flow`; the sends are `net`.

Roots: `0x46b500`, `0x46c2c0`, `0x46cc60`, `0x4724d0`, `0x46b340`

## Last character index

The client remembers the last character as one global 0-based roster index in the CVar
`lastCharacterIndex` (default "0", record at `0x882674`). `EnterWorld` formats `0x83856c` with `%d`,
sets the CVar and saves the config file before it sets state 8, with no way out between the set and
the save. The restore (`0x4724d0`, from `Idle` in state 3) selects the CVar's integer, which
`0x472740` clamps to 0 when negative or not below the roster count. The file keeps no line for a
value equal to the default, so a first character is remembered as an absent key. Nothing keys the
index to a realm or an account: the file is one global `WTF\Config.wtf`.

Addresses: `0x46b500`, `0x46b5ce`, `0x46b5e0`, `0x46b5fa`, `0x46b6f6`, `0x402d93`, `0x402db1`, `0x882674`, `0x63df50`, `0x63d980`, `0x63da21`, `0x63da43`, `0x63da51`, `0x63ddaa`, `0x63d395`, `0x402370`, `0x63d380`, `0x46e777`, `0x4753fb`, `0x4724d0`, `0x472550`, `0x472563`, `0x472740`, `0x5aabe0`
Confidence: verified
Proof: one
Test: none

`0x402db1` stores the registration of `lastCharacterIndex` (`0x402d93`) in `0x882674`. Between
`0x46b5ff` and `0x46b6f6` every jump stays inside, and no `ret` lies there. `SaveConfig` writes a
`SET %s "%s"` line only when the record's value differs, ignoring case, from its default
at `+0x30` (`0x63da43`, `0x63da51`); the flag test at `0x63da21` always passes because
`CVar::Register` ORs bit 0 into every record (`0x63ddaa`). `0x402370` passes `Config.wtf` to
`0x63d380`, which stores it (`0x63d395`); `SaveConfig` opens `WTF\` plus that name. The
roster is in wire order: `0x5aabe0` walks it forward at stride `0x110`, and the glue's only `qsort`
calls are the realm sort `0x46e777` and the name generator `0x4753fb`.

## Selection after create and delete

After a character is created, the last roster entry is selected; after one is deleted, the remembered
index is. A create reports `SELECT_LAST_CHARACTER` (12) and then asks for `charselect`; the script
only sets a flag, the roster refresh restores the remembered index, and the next
`CHARACTER_LIST_UPDATE` makes `UpdateCharacterList` consume the flag and select the last entry, so
the flag wins. A delete fires `SELECT_FIRST_CHARACTER` (13), which selects entry 1 at once, but the
frame then requests the roster again (`0x46c1a7`), and the restore of the remembered index replaces
that selection.

Addresses: `0x46c12e`, `0x46c133`, `0x46c138`, `0x46c19d`, `0x46c1a2`, `0x46c1a7`, `0x46b880`, `0x472556`, `0x472563`, `0x472568`, `0x47256d`
Confidence: verified
Proof: one
Test: none

`0x46c133` fires 12 (`mov ecx,0xc`) and `0x46c13d` calls `0x46b840` with `charselect`; `0x46c1a2`
fires 13. `CharacterSelect.lua` sets `selectLast` in the event handler and `UpdateCharacterList` reads it
after the roster is rebuilt: the flag selects the last entry, otherwise the stored `selectedIndex`
(1 when 0 or past the count). `0x472563` calls `0x472740` with the CVar's value, then `0x47256d` fires
`CHARACTER_LIST_UPDATE`.

## Glue event id space

The glue screen's 26 event ids are an array of names stored by `0x46cc60` at `0xb41e70`, four
bytes each, and `0x703d90` rebuilds the script event registry `0xceef68`, 16 bytes a record, from
whichever table it is given, so an id means one event in the glue and another in the world. Id 8 is
`UPDATE_SELECTED_CHARACTER`. In the glue band the event fire `0x703e50` has 21 sites: 16 calls and 5
tail jumps (`0x46c52e`, `0x46e9cb`, `0x46ea1a`, `0x47256d`, `0x4725e1`).

Addresses: `0x46cc60`, `0x46a885`, `0x46a88a`, `0x46a88f`, `0x703d90`, `0x703e09`, `0x703e34`, `0x703e50`, `0x703f50`, `0xceef68`, `0xb41e70`, `0x46c52e`
Confidence: verified
Proof: one
Test: none

`0x703e09 mov eax,[edx+esi*4]` reads the input table at stride 4, and `0x703e34 add ebx,0x10` steps
the destination registry, whose record size `shl edi,4` in `0x703f50` confirms. `0x46c52e` is the
tail jump that fires ids 14 and 15 (`Screenshot`'s callback `0x46c520`). The census of `call` and
`jmp` to `0x703e50` over `0x46a400` to `0x476020` lists `0x46aa34`, `0x46b48b`, `0x46bac8`, `0x46bf91`,
`0x46c08f`, `0x46c0dd`, `0x46c129`, `0x46c133`, `0x46c1a2`, `0x46c62e`, `0x46e73d`, `0x46ec26`,
`0x472732`, `0x47461e`, `0x474b94`, `0x474c81` as calls.

## Add-on list event

`ADDON_LIST_UPDATE` (id 6) fires once per `SMSG_ADDON_INFO` (opcode 0x2ef), after the body is applied
to the add-on records, and nowhere else. The handler `0x5b4260` calls the session's listener, slot
`+0x0c` of its vtable, which is `0x5ab2f0` (its cell is `0x80a400`), which signals event 6. The auth response
(0x1ee) calls the listener's slots `+0x14` and `+0x8`, never `+0x0c`.

Addresses: `0x5ab2f0`, `0x5b4260`, `0x5b4282`, `0x5b4290`, `0x5b3f7f`, `0x5b3f8f`, `0x80a400`, `0x51da70`
Confidence: verified
Proof: one
Test: none

`0x5b3f7f cmp edx,0x2ef` dispatches to `0x5b4260`, which reads the listener at `[svc+0x1adc]`, calls
the no-op hook in slot `+0x14`, `0x51da70` on the body, then `call [vt+0xc]` (`0x5b4290`) with nothing
between. The listener vtable's first four cells are `0x5ab2d0`, `0x5ab2e0`, `0x5ab280`, `0x5ab2f0`. Of the
15 reads of `[reg+0x1adc]`, the only call through slot `+0x0c` is `0x5b4290`; `0x5b41b0` calls
slots `+0x14` (`0x5b41ce`) and `+0x8` (`0x5b424a`).

## Level-0 entry eye

A record whose wire level byte is 0 is entered at the opening eye of its race's intro cinematic.
The parser sets the record's `+0x10a` and raises the level to 1 for such a record; phase 1 calls
`0x46c2c0(race, &position)` only when `+0x10a` is set. The function walks `ChrRaces +0x40` to
`CinematicSequences +0x08` to a `CinematicCamera` row, loads that row's model synchronously, applies
the row's origin and facing, and writes camera 0's eye (property 7) over the three position floats;
it returns nothing used and leaves the map alone, so the record's own map opens at that eye.

Addresses: `0x46c203`, `0x46c20b`, `0x46c214`, `0x46c2c0`, `0x46c2f8`, `0x46c320`, `0x46c348`, `0x46c357`, `0x46c36c`, `0x46c3e6`, `0x46c40d`, `0x46c47c`, `0x46c49d`, `0x46c4cf`, `0x46c4fa`, `0x5b44f9`, `0x7103d0`, `0x7abf80`
Confidence: verified
Proof: one
Test: none

Each lookup bounds its id by the store's maximum and tests the row for null; any failure returns 0 at
`0x46c4e6` with the position untouched. `0x46c36c call 0x7103d0` with arguments 1 and 0 makes an
unloaded model wait for its read; a model that is still not loaded returns 0. `0x46c3e6` and
`0x46c40d` build translation by `+0x0c` to `+0x14` and rotation by `+0x18` about (0, 0, 1);
`0x46c4cf` takes camera 0 and `0x46c4fa` calls `0x7abf80(camera, 7, &position)`. `0x46c219`
follows, ignoring the result.

## World entry waits

Phase 1 of state 8 returns with the terrain around the entry point already loaded: `0x401570` runs
the map open `0x66fbe0` (`0x4017cd`) around the position, with the loading screen's progress callback
installed, loads the interface files, and sets the in-world flag `0x882734` last. Phase 2 waits for
the asynchronous file reader to be idle: nothing being read, nothing submitted and not started,
and nothing completed and awaiting its callback. The reader serves textures, terrain tiles, model
files and world-model files, so the wait covers every file queued between the phases.

Addresses: `0x401570`, `0x4017cd`, `0x4017e5`, `0x66fbe0`, `0x443e20`, `0x443e2b`, `0x443e34`, `0x443e41`, `0x443620`, `0x882734`, `0x46c247`
Confidence: verified
Proof: one
Test: none

`0x443e20` takes the reader's lock, returns 1 when the request being serviced is set (`0x443e2b`),
when the submitted list holds a node (`0x443e34`, bit 0 clear and nonzero), or when the completed
list does (`0x443e41`), else 0. `0x401570` calls
`0x443620` before and after `0x66fbe0`, `0x48f480` for the interface and `0x4818f0` for the camera.
The map open's forced synchronous read is `terrain`'s.

## Waiting for one read

`AsyncFileReadWait(read)` returns at once for a read whose done byte `+0x1c` is set. Otherwise it
records the read as the one awaited and, unless it is the read in progress, its byte `+0x1d` is 0
or its byte `+0x28` is set, moves it to the head of the queue `0x885688`. It then drains finished
reads and sleeps 1 ms until the drain clears the awaited read. A depth count is held across the wait.
Texture, map object, model and terrain loads call it.

Addresses: `0x443bd0`, `0x443bd3`, `0x443be3`, `0x443bf5`, `0x443bfb`, `0x443c07`, `0x443c12`, `0x443c1d`, `0x443c93`, `0x443cd2`, `0x443ce3`, `0x443cf4`, `0x443d07`, `0x443d1e`, `0x443eda`, `0x443ee2`, `0x885684`, `0x885688`, `0x8856d0`, `0x8856e0`
Confidence: verified
Proof: one
Test: none

`0x443bd3` increments `0x8856e0` and `0x443d1e` decrements it. `0x443be3` tests `+0x1c` under the
lock and leaves through `SCritSect::Leave`. `0x443bf5` compares the read with `0x8856d0` and `0x443bfb`
stores it in `0x885684`. `0x443c1d` unlinks the read's link `+0x20`, the queue's link offset, and
`0x443c93` to `0x443cd2` link it after the terminator. `0x443ce3` prints `AsyncFileReadWait()` to the
empty print; `0x443cf4` calls `0x443e70(0, 0)` and `0x443d07` `OsSleep(1)`. In `0x443e70`, `0x443eda`
compares a finished read with `0x885684` and `0x443ee2` clears it. The callers are `0x44ad50`, `0x6985d0`,
`0x6a4a50`, `0x6a4a90`, `0x706900` and `0x710520`.

## Starting the reader

`0x443300` starts the asynchronous reader: it registers the drain `0x443e70`, clears the progress state, stores the
starting thread's property context (`0x41db60`) in `0x885680`, resets the stop event `0x8856dc` and creates the thread
`AsyncFileLoader` running `0x443360`, its handle in `0x885694`. The work event `0x885698` is auto-reset, so one wait
consumes one set; each queued read sets it (`0x443ae0`) and so does the drain at its entry (`0x443e77`). The stop
event is manual-reset and is set by the shutdown `0x443520`, which also sets the work event and waits without a limit
for the thread's handle.

Addresses: `0x443300`, `0x443351`, `0x443520`, `0x443ae0`, `0x443bc5`, `0x443e70`, `0x443e77`, `0x4431b0`, `0x4431f0`, `0x41db60`, `0x659a70`, `0x659ac0`, `0x885680`, `0x885694`, `0x885698`, `0x8856dc`
Confidence: verified
Proof: one
Test: none

`0x4431f0` constructs the work event with both flags 0 and `0x4431b0` the stop event with manual-reset 1 and initial 0
(`SEvent::SEvent` passes them to `CreateEventA`, `0x659a81` to `0x659a87`). `0x443331` calls `0x41db60`, `0x443340`
`SEvent::Reset`, and `0x443351` passes `0x443360` to `SThread::Create` (`0x659ac0`) with the handle address
`0x885694` and the name `AsyncFileLoader`. `0x443bc5` is the tail jump of the queueing function to `SEvent::Set`.
`0x443527` and `0x44352c` set the stop and work events and `0x443538` waits on the thread with the timeout −1. The one
reference to `0x443360` is `0x443351` (`tools/callers.py`, control `0x4baee0`).

## The reader loop

The thread `0x443360` adopts the starting thread's property context, then loops. It polls the stop event with a 0
timeout and returns 0, after setting the work event, when it is set. It waits 500 ms on the work event: a timeout goes
back to the poll, a signal to the queue. Under the lock `0x88569c` it takes the first read of the queue `0x885688`,
unlinks it and stores it in `0x8856d0`. An empty queue goes back to the poll. After each read it sleeps 1 ms and takes
the next without waiting, so the stop event is looked at only when the queue is empty or a wait times out.

Addresses: `0x443360`, `0x443370`, `0x443384`, `0x443390`, `0x44339f`, `0x4433b0`, `0x4433ba`, `0x4433bf`, `0x4433c7`, `0x443407`, `0x4434d7`, `0x4434dc`, `0x4434eb`, `0x443500`, `0x41db50`, `0x6599c0`, `0x6579b0`, `0x885680`, `0x885688`, `0x88569c`, `0x8856d0`
Confidence: verified
Proof: one
Test: none

`SEventWait` is `WaitForSingleObject(handle, timeout)`. `0x443369` loads `0x885680` for `SetThreadPropContext`
(`0x443370`). The poll `0x443377` to `0x443384` pushes 0 and leaves to `0x443500` on a result of 0. `0x443390` pushes
`0x1f4` and waits on `0x885698`; `0x44339f cmp eax,0x102` goes to the poll again at `0x4434eb`. `0x4433b0` enters the
lock, `0x4433ba` loads the queue's first link and `0x4433bf`, `0x4433c7` test the end bit and null. The unlink is
`0x4433cf` to `0x4433ff` and the in-progress store `0x443407`. `0x4434d7` is `OsSleep(1)` and `0x4434dc` jumps back
to `0x4433b0`.

## Reading a file

For each read the thread calls `SFile::Read(file +0, buffer +4, size +8)` up to 10 times, with `OsSleep(1)` after each
failure; after the tenth failure it copies the file name into a local buffer that nothing reads. It enters the read's own
lock (`+0x18`) around the tries when that is set. The read is queued as finished either way: under the queue lock it is
unlinked, appended to the finished queue `0x8856c4`, cleared from `0x8856d0`, and its byte `+0x28` is set. A file that
cannot be read is therefore delivered unread, as done.

Addresses: `0x443360`, `0x443419`, `0x443423`, `0x44342b`, `0x443431`, `0x443436`, `0x44343f`, `0x443444`, `0x443445`, `0x443456`, `0x443462`, `0x44346c`, `0x443471`, `0x4434a8`, `0x4434be`, `0x4434c9`, `0x429d50`, `0x648460`, `0x6487b0`, `0x8856c4`, `0x8856d0`
Confidence: verified
Proof: one
Test: none

`0x443419` enters the read's lock and `0x443462` leaves it. `0x44342b` to `0x443431` push the six arguments of
`SFile::Read` (three zeros last), `0x443436` tests the result, `0x44343f` is `OsSleep(1)` and `0x443444`, `0x443445`
count `edi` down from 10. `0x443456` calls `SFileGetFileName` into `[ebp-0x200]`, never read again. `0x44346c`
re-enters the queue lock; `0x443471` to `0x4434b9` move the read to the finished queue by its link offset
(`0x8856c4`), `0x4434be` clears the in-progress word and `0x4434c9` sets `+0x28`. A read is a `CAsyncObject` of 0x2c bytes,
allocated by `0x4438b0`.

## Waiting for every read

`0x443d30` blocks until the asynchronous reader is idle. It first adds up the pending counts the
registered counters of the array `0x885658` report into a total, then, while a load is pending, services the
reader, reports progress through the callback `0x885668` when one is set, as
`(total − c) / total` clamped to [0, 1], `c` being the dword `0x88567c` at that moment, which starts at
the total, and sleeps 1 ms. It clears the callback when done.
Its two diagnostic lines go to the empty print.

Addresses: `0x443d30`, `0x443d55`, `0x443d72`, `0x443d84`, `0x443d9f`, `0x443da8`, `0x443dc1`, `0x443ddf`, `0x443de6`, `0x443df9`, `0x885658`, `0x885668`, `0x88567c`, `0x8856d8`
Confidence: verified
Proof: one
Test: none

`0x443d55` calls each counter of the array, its count at `+0x4` and its data at `+0x8`, and the sum is
stored in `0x88567c` too. The loop tests `IsAsyncLoadPending` (`0x443d72`) and calls `0x443e70(0, 0)`
(`0x443d84`). `0x443d9f` is `fild` of `total − [0x88567c]` divided by the total; `0x443da8` stores 0 for a
value below 0 (`C0`) and `0x443dc1` stores 1.0 for one above 1.0, while NaN passes both. `0x443ddf`
calls the callback with `ecx` from `0x8856d8`, and `0x443de6` is `OsSleep(1)`. `0x443df9` zeroes `0x885668`.

## Realmd link at entry

`EnterWorld` closes the realmd link with `0x5b3320` (`0x46b70c`, asynchronous), and nothing in the
world dials it again. The dial `0x5bc3e0` has two callers: the realm list re-dial `0x5b2200`, whose
one caller `0x5ab840` has one caller, `0x46b902` in `0x46b8d0`, and the account login dial. Only the
glue calls `0x46b8d0` (Change Realm, a cancelled character list request and the no-realm leg of the
roster restore), and `0x5b2200` sets the logon object's reconnect flag `+0xc`, so the re-dial sends a
reconnect challenge on the kept session.

Addresses: `0x46b70c`, `0x5b3320`, `0x5bc3e0`, `0x5b2200`, `0x5b2219`, `0x5b3022`, `0x5ab840`, `0x5ab86a`, `0x46b902`, `0x46b8d0`, `0x46ecfe`, `0x46c073`, `0x47254b`, `0x5b2134`
Confidence: verified
Proof: one
Test: none

Census of `call` and `jmp` targets: `0x5bc3e0` two, `0x5b2200` one, `0x5ab840` one, `0x46b8d0` three,
none a jump table or stored address (the second caller of the dial, `0x5b3022`, is the account
login `0x5b2fb0`, whose one caller is `0x5ab586`). `0x5ab840` picks `0x5b2200` when the logon
object's `+0x5` is clear. State 0 holds for the whole world session, so no arm that calls `0x46b8d0`
runs there.

## Return to character select

A logout and a refused character login both return to character select on the same world link and
send `CMSG_CHAR_ENUM` there, on direct-call evidence with no realmd logon and no new world
authentication. The logout
reply handler `0x5aaeb0` calls `0x401ee0(1, 1, 0)`, which unregisters ten world handlers, shuts the
world down, rebuilds the glue on a fresh Lua state (`0x46a7b0(1)`) and signals `SET_GLUE_SCREEN
"charselect"`; `CharacterSelect_OnShow` then asks for the list. The refusal handler `0x5aad70` does
the same with stack argument 1, which stores state 9 first, so the list request waits for the `OKAY`
dialog's own click.

Addresses: `0x5aaeb0`, `0x5aad70`, `0x5aaeca`, `0x5aad8d`, `0x401ee0`, `0x401ee6`, `0x402064`, `0x402069`, `0x402075`, `0x402090`, `0x46a7b0`, `0x46b840`, `0x46b880`, `0x5aabb0`, `0x5b4690`, `0x5b4620`, `0x5aae20`, `0x5b4700`, `0x46c5f8`, `0x46d380`, `0x5aae08`
Confidence: verified
Proof: one
Test: none

`0x5b4620` calls vtable slot `+0x50` of the session (`0x80a398`), which is `0x5aaeb0`; slot `+0x4c` is
`0x5aad70`. `0x401ee0` returns at `0x401ef3` unless the in-world flag is set; `edx` (the second
argument) gates the rebuild (`0x402062`), the stack argument the state-9 store (`0x402075`), and `ecx`
picks `charselect` or the other screen (`0x402089`): the logout and refusal pass 1, the disconnect
handler `0x46c5f8` passes 0. The refusal result byte 1 to 6 maps to status `0x3e`, `0x3f`, `0x40`,
`0x42`, `0x43`, `0x44` and any other to `0x41` (`0x5aae08`). `0x46b880` returns at once in state 9.
A closure over direct calls and jumps from either handler (3,414 functions) reaches `0x5ab1a0`, `0x5b5370`,
`0x5b3320`, `0x5ab800`, `0x5ab840`, `0x5b2200`, `0x5bc3e0`, `0x5b2fb0` and `0x5ab4b0` in none; indirect calls are not
followed. The only `0x418190` calls with an immediate opcode 0x3d (`CMSG_PLAYER_LOGIN`) and 0x37
(`CMSG_CHAR_ENUM`) are in `0x5b4700` (`0x5b472b`, caller `0x5aae53` in `0x5aae20`, whose caller is `0x46c272`) and
`0x5b4690` (`0x5b46bb`, by a jump from `0x5aabd4`); 266 of the 733 calls take their opcode from elsewhere.

## Character name check

The name check `0x5abd50` returns the validator's code plus 0x45, an index in the status table:
`0x6c9ae0` runs `0x6c96c0` with an empty extra-character list and all three word lists on, then
rejects a name longer than 12 units (charset 1), 6 (charset 3) or 8 (other charsets) with code 2.
The validator returns 0 (no name), 1 (one unit), 3 (a unit outside the charset, space and
apostrophe included), 6 (a hit in any of three word lists), 9 (three equal units in a row), 0xc (a
conversion overflow or a charset index outside 0 to 7) or 0xd (accepted). It never returns 4
(mixed languages) or 5 (profane); a profane hit reports reserved.

Addresses: `0x5abd50`, `0x6c9ae0`, `0x6c9b30`, `0x6c96c0`, `0x6c9b04`, `0x65b350`, `0x6c9a4f`, `0x6c9a5d`, `0x6c98c1`, `0x6c9833`, `0x71fc70`
Confidence: verified
Proof: one
Test: none

The returns of `0x6c96c0` (`ret 0x18`), by the `mov eax` before each: 0 for a null name, 3
(`0x6c9827`), 0 or 1 by length (`0x6c98ac setne`), 8, 0xb, 9, 7, 0xa, 0xd, 6, 0xc. 7, 8, 0xa and 0xb need
a nonempty extra list: with `edx` 0 a space or apostrophe fails the charset test and reports 3. The
three word-list loops call `0x71fc70` over three tables of 16-byte records; a hit reaches
`0x6c9a4f`. The charsets are the next entry's. `0x6c9b30` is `0x6c9ae0` without the
charset 3 cap (`0x6c9b51` to `0x6c9b66`); no `call`, `jmp` or stored dword names it (the same scan finds `0x5abd58` for `0x6c9ae0`).

## Name charsets

A name is held to the charset of its first unit, the primary or secondary of `0x6c9a70(locale)`. A
region `0x86c9e0` of 1 to 5 picks its pair at `0x86c9e4`: (1, 1), (2, 4), (1, 1), (3, 1), (3, 1).
Otherwise the primary is the locale's entry at `0x86ca0c`, 1, 2, 1, 1, 3, 3, 1, 0, with entry 0 for a
zero entry or an index outside 0 to 7, and the secondary is 4 for koKR (index 1), else 1.
`0x6c9dd0(unit, set)` tests a UTF-16 unit against a set; a set not listed takes none:

| set | units |
|---|---|
| 1 | `IsLatin1Letter` (`0x6c9c60`) |
| 2 | the 2,350 syllables at `0x86ca30` (`0x6c9d50`) |
| 3 | U+4E00 to U+9FFF, U+3400 to U+4DFF, U+F900 to U+FAFF (`0x6c9d80`) |
| 4 | A to Z, a to z (`0x6c9ce0`) |

Addresses: `0x6c9a70`, `0x6c9a78`, `0x6c9a82`, `0x6c9aa1`, `0x6c9ab7`, `0x6c9abe`, `0x6c9ac8`, `0x6c9dd0`, `0x6c9c60`, `0x6c9d50`, `0x6c9d60`, `0x6c9d80`, `0x6c9dc0`, `0x6c9ce0`, `0x86c9e0`, `0x86c9e4`, `0x86ca0c`, `0x86ca30`, `0x6c971e`, `0x6c91b8`
Confidence: verified
Proof: one
Test: `glue::name_charset_pair_difftest`, `glue::name_charset_units_difftest`

`0x6c9a78` and `0x6c9a80` bound the region with `jl` and `jg`; `0x6c9a82` reads `[region·8 + 0x86c9dc]`
for the primary and `[region·8 + 0x86c9e0]` for the secondary. Otherwise `0x6c9aa1` to `0x6c9ab5` reset
the index to 0, and `0x6c9abe` to `0x6c9acb` compute `(((index − 1) ≠ 0 ? −1 : 0) & −3) + 4`. The one
caller, `0x6c971e`, has already refused an index outside 0 to 7. The region is 1 in the file; its only
writer, `0x6c91b8`, stores the `Wow.ini` region at start-up (`dbc#wowini-values`). `0x6c9dd0` compares
`edx` with 1 to 4 and tail-jumps to `0x6c9c60`, `0x6c9d50`, `0x6c9dc0` (a jump to `0x6c9d80`) and
`0x6c9ce0`. `0x6c9d50` bounds the unit, then compares it with each word from `0x86ca30` while the offset
is below 0x125c (`0x6c9d60`); the words ascend and are EUC-KR's rows 0xB0 to 0xC8, the Hangul set of KS X 1001.
The tests run each function whole, the predicates over all 65,536 units.

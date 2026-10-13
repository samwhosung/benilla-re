# ui: the script layer and events

How a C++ object becomes a Lua table, how frames register for named events, how an event or a
script handler is delivered, and the Lua state those run in.

`FrameScript.cpp` (`0x701940` to `0x705e20`) is the bridge between the interface and the Lua 5.0
state in `0xceef74`. It holds the script object every frame and region begins with, the table of
event names and their listener lists, the functions that run a handler with `this`, `event` and
`arg1` to `argN` set, the creation and reset of the state, and helpers the rest of the client uses
to register functions and read or write globals. The same object file holds a base library of
Lua functions (`setglobal` to `debugstack`, from 0x702770 to 0x703760), which this doc does
not cover. The run `0x705e30` to `0x706430` after it registers the frame-type method tables and
the global script functions `CreateFrame`, `CreateFont`, `GetNumFrames` and `EnumerateFrames`.

Roots: `0x7019c0`, `0x7039e0`, `0x703b80`, `0x703ba0`, `0x703bf0`, `0x703cf0`, `0x703d90`, `0x703e50`, `0x703f50`, `0x7040d0`, `0x704120`, `0x704160`, `0x702140`, `0x702280`, `0x7023e0`, `0x7024f0`, `0x702590`, `0x7025c0`, `0x702670`, `0x702690`, `0x7026f0`, `0x702710`, `0x701ae0`, `0x701b10`, `0x701bd0`, `0x701cd0`, `0x701d80`, `0x702000`, `0x706350`, `0x7063d0`

## The script object

The script object is 0x24 bytes with the vtable `0x811f4c`: a count of holders of its Lua table
(`+0x4`), the registry reference of that table (`+0x8`, −2 when there is none), its `OnEvent` slot
(`+0xc`), and an array of the event indices it listens to (capacity `+0x14`, count `+0x18`, data
`+0x1c`, grow step `+0x20`). Its Lua table is made on first use: a table whose element 0 is the
object as light userdata, with the metatable the global `__framescript_meta` holds. Releasing the
last holder clears element 0 and the metatable, and frees the reference.

Addresses: `0x701ae0`, `0x701b10`, `0x701b40`, `0x701bd0`, `0x701bde`, `0x701bf3`, `0x701bfc`, `0x701c0d`, `0x701c2c`, `0x701c3d`, `0x701cd0`, `0x811f4c`, `0x8729ec`
Confidence: verified
Proof: one
Test: none

`0x701ae0` stores −2 at `+0x8` and zeroes the rest under `0x811f4c`. `0x701bd0` makes the table
when `+0x4` is 0: new table, `pushnumber(0)`, the object as light userdata (`0x701bfc`), the set
into the table, the global `__framescript_meta` (`0x701c0d`) as metatable (`0x701c2c`), and a
registry reference stored at `+0x8` (`0x701c3d`); then it counts one holder. `0x701cd0` counts one
down and at zero stores nil at element 0 and as metatable, frees the reference and stores −2.
`0x701b40` unregisters every event, releases `OnEvent` and frees the index array.

## The event table

The event names are an array of 0x10-byte records at `0xceef68`, counted by `0xceef64`: the name,
and a list of listener nodes, each holding one script object. Installing a new table destroys the
old records with their listeners, allocates the new array and copies each name. An event is
signalled by its index into this array.

Addresses: `0x703d90`, `0x7053b0`, `0x705530`, `0x701a40`, `0x7019b0`, `0x7019c0`, `0xceef60`, `0xceef64`, `0xceef68`, `0x872990`
Confidence: verified
Proof: one
Test: none

`0x703d90(names, count)` calls `0x701a40` on each old record, frees the array tagged
`.?AVFrameScript_EventObject@@`, zeroes the three globals, grows the array through `0x7053b0` and
stores `SStrDupA` of each non-empty name at `+0x0` of record `i·0x10`. `0x7053b0` builds each new
record as a name and an empty list (`+0x8` terminator, `+0xc` first link). The static initializer
`0x7019b0` calls `0x7019c0`, which zeroes the globals, and registers its exit handler.

## Registering for an event

Registering an object for an event name compares the name, ignoring case, with every record. For
a match where the object is already a listener it answers 1 at once; otherwise it appends a
listener node to the record's list and the record's index to the object's array. An unknown name
changes nothing. Unregistering walks the object's array from the end, freeing its node in each
record of that name and removing the index. Registering for all events appends the object to
every record it is not already in; unregistering from all frees every node of the object and
empties its array.

Addresses: `0x702140`, `0x702181`, `0x7021a8`, `0x7021bf`, `0x702280`, `0x7023e0`, `0x7024f0`, `0x7052d0`, `0x705570`, `0x872a08`
Confidence: verified
Proof: one
Test: none

`0x702140` loops over `[0xceef64]` records calling `SStrCmpI` (`0x702181`), scans the record's
list for `[node+0x8] == object` (`0x7021a8`), and calls `0x7052d0(2, 0, 0)` (`0x7021bf`), which
allocates a node tagged `.?AUEVENTLISTENERNODE@@` and links it before the list's terminator, the
tail. The index array grows through `0x429340`. `0x702280` unlinks with
`0x705570` and frees each node whose object matches; `0x7023e0` and `0x7024f0` are the all-events
forms.

## Signalling an event

Signalling event `i` with no arguments does nothing when its list is empty. Otherwise it saves the
global `event`, sets it to the event's name, and for each listener in list order runs the
listener's `OnEvent` script when it has one, reading the next node before the call. It then
restores the saved `event`. The form with arguments runs each listener's `OnEvent` through the
argument runner instead, which also sets `arg1` onward.

Addresses: `0x703e50`, `0x703e56`, `0x703e5e`, `0x703e7a`, `0x703e99`, `0x703ea4`, `0x703ec8`, `0x703ee8`, `0x703eee`, `0x703efb`, `0x703f04`, `0x703f3d`, `0x703f50`, `0x84b648`
Confidence: verified
Proof: one
Test: none

`0x703e50` is `fastcall(index)`: `shl ecx,0x4` into `[0xceef68]` (`0x703e56`) and the first link
`+0xc` (`0x703e5e`). It reads `_G["event"]` (`0x703e7a`) into a registry reference
(`0x703e99`), sets `_G["event"]` to the record's name (`0x703ea4` to `0x703ec8`), then per node
loads `+0x4` before the call (`0x703ee8`) and calls `0x702690` on `object + 0xc` when its first
dword is set (`0x703efb`). It pushes the saved value back, stores it as `event`, and frees the
reference (`0x703f3d`). `0x703f50` does the same with `0x702710(object, object + 0xc, format,
arguments)`.

## Running a handler

A script slot is a registry reference to a compiled function and an execution-context word. Running
a slot sets the current context `0xceeac0` to the slot's word while a nesting count is above 0,
calls the function and restores the context. The runner saves the global `this`, makes it the
object's table, calls the function protected under the error handler, and restores `this`. The
argument runner first reads a format: each `%d`, `%u`, `%f` or `%s`, at most 19, becomes the
global `arg1`, `arg2` and so on, each saved before and restored after the call.

Addresses: `0x702690`, `0x702698`, `0x7026a8`, `0x7026ad`, `0x7026b7`, `0x7026c5`, `0x7026d6`, `0x7026f0`, `0x702710`, `0x704d50`, `0x704de5`, `0x704e16`, `0x704e2b`, `0x704e4f`, `0x704e79`, `0x704ea7`, `0x704ef0`, `0x704f10`, `0x704f73`, `0x704f98`, `0x7052a4`, `0x7052b8`, `0xceeac0`, `0xceeac4`, `0x8722c8`, `0x8722dc`, `0x872e64`
Confidence: verified
Proof: one
Test: none

`0x702690` increments `[0xceeac4]` and, when the result is not 0 (`je` at `0x7026a8` on the
increment's flags), stores `[slot+4]` in `0xceeac0` (`0x7026ad`); after `0x704d50` (`0x7026b7`)
it restores it when the count is not 0, decrements and clamps at 0 (`0x7026d6`). `0x704d50`
saves `_G["this"]` (`0x704de5`), makes the table with `0x701bd0(0)` when `+0x4` is 0
(`0x704e16`), stores it as `this` (`0x704e2b`), pushes the error handler `[0x8722c8]`
(`0x704e4f`) and the function, `lua_pcall(0, 0)` (`0x704e79`), and restores `this` (`0x704ea7`).
`0x704f10` stops at the 19th argument (`cmp [ebp-4],0x13; jge`, `0x704f73`) and switches on the
letter through `0x7052b8` and `0x7052a4`; the name buffer `0x8722dc` is `arg` and the count.

## Compiling a script element

Setting a slot from text releases the slot's function, and with non-empty text compiles the body
as a function named `<frame name>:<handler>`, storing its reference and a context word of 0. A
handler name the object has no slot for fails.

Addresses: `0x7025c0`, `0x702670`, `0x704c70`, `0x704cb0`, `0x872a28`
Confidence: verified
Proof: one
Test: none

`0x7025c0` gets the slot through slot 0xc of the object, calls `0x702670` (which calls `0x704cb0`
when the reference is set), formats `"%s:%s"` (`0x872a28`) from slot 4's name, or `<unnamed>`, and
the handler name, and stores `0x704c70`'s reference at the slot with 0 beside it. `0x704c70`
compiles a buffer and answers a registry reference, or −1 when it fails to compile.

## The Lua state

Opening the state creates it, keeps the registry reference of the error handler in `0x8722c8` and
of a second value in `0x8722d4`, makes 36 table stores in a loop, calls four library openers, and
runs a final setup through `0x704cd0`. Resetting closes the state when
one exists and opens it again; closing stores 0 in `0xceef74`. `0x7040d0` answers the state.

Addresses: `0x7039e0`, `0x703ab2`, `0x703b80`, `0x703ba0`, `0x7040d0`, `0x703bc0`, `0xceef74`, `0x8722c8`, `0x8722cc`, `0x8722d4`, `0x811e28`
Confidence: inferred
Proof: none
Test: none

`0x7039e0` stores the new state in `0xceef74`, stores −1 in `0x8722cc`, takes the two references
and loops in steps of 8 below 0x120; the base library table `0x811e28` is pushed at `0x703ab2`.
`0x703b80` tests `0xceef74` before `0x703ba0` and a jump to `0x7039e0`. The openers it calls were
not read.

## Functions and globals

Registering a C function sets the global of its name to a closure of it with no upvalues, and
unregistering sets it to nil. The client reads and writes globals through small helpers: a number,
a string or nil, each through the current state.

Addresses: `0x704120`, `0x704160`, `0x7041a0`, `0x7041f0`, `0x704250`, `0x7042a0`, `0x704300`, `0x704350`, `0x7043b0`, `0x7040e0`
Confidence: inferred
Proof: none
Test: none

`0x704120` pushes the closure, the name, inserts and sets into the globals; `0x704160` pushes nil
instead. `0x7041a0` and `0x704250` set a number global from an integer or a float, `0x7041f0` and
`0x7042a0` read one, `0x704300` sets a string or nil, `0x704350` reads a string, `0x7043b0` sets
nil. `0x7040e0` raises a Lua error with a formatted message.

## Text, counts and running code

`FrameScript_GetText` answers the text of a global string, choosing a variant by a count and a
gender when given; `0x703cf0` calls the Lua function `GetPluralIndex` with a count and answers its
result minus one. `0x704ae0` compiles and runs a buffer, reporting an error through a status
object; `0x704bc0` reads a file and runs it; `0x704cd0` runs a string; `0x704cf0` calls a function
reference.

Addresses: `0x703bf0`, `0x703cf0`, `0x704ae0`, `0x704bc0`, `0x704cd0`, `0x704cf0`, `0x872cd0`
Confidence: inferred
Proof: none
Test: none

`0x703bf0` with no gender and a negative count reads the global through `0x704350`, else calls the
global with the two values and answers the string result or `""`. `0x703cf0` pushes `0x872cd0` and
calls it with one argument. The run helpers load the buffer and call it protected under the error
handler `[0x8722c8]`.

## Saved variables

Writing the saved variables serializes a global by type: nil, booleans, finite numbers with
`%.16g`, strings quoted with `\`, `"`, newline and NUL escaped, and tables recursively with one tab
per level, skipping functions, NaN and infinite numbers and keys of other types with a comment.

Addresses: `0x7043f0`, `0x704480`, `0xceeb60`
Confidence: inferred
Proof: none
Test: none

`0x704480` switches on the value's type through 0x704a40 and builds each line in the 1024-byte
buffer `0xceeb60`, flushing it to the output after each line; the strings `%.16g`,
`nil, -- skipped %s` and the skipped-entry comments decide the cases. `0x7043f0` writes `name = `
and the value.

## Method tables of the frame types

A frame type's Lua methods are a name-to-function table installed into a hash table of the type
the first time it is needed; a method call finds the object's type table through the metatable.
`0x706350` registers the global script functions and the method tables of seventeen frame types,
and `0x7063d0` removes them.

Addresses: `0x701d80`, `0x702000`, `0x705730`, `0x7056a0`, `0x705b30`, `0x705ac0`, `0x706350`, `0x7063d0`, `0x705f00`, `0x705f60`, `0x706040`, `0x7060b0`
Confidence: inferred
Proof: none
Test: none

`0x701d80(table, count, hash)` hashes each name with `SStrHashHT`, adds a node, stores
`SStrDupA` of the name and a registry reference of the function; `0x702000` finds a name and pushes
its function. `0x7056a0` and `0x705730` grow the hash. `0x705f00`, `0x705f60`, `0x706040` and
`0x7060b0` are `GetNumFrames`, `EnumerateFrames`, `CreateFont` and `CreateFrame`, by the error
strings each formats.

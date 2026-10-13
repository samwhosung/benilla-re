# ui: the script layer and events

How a C++ object becomes a Lua table, how frames register for named events, how an event or a
script handler is delivered, and the Lua state those run in.

`FrameScript.cpp` (`0x701940` to `0x705e20`) is the bridge between the interface and the Lua 5.0
state in `0xceef74`. It holds the script object every frame and region begins with, the table of
event names and their listener lists, the functions that run a handler with `this`, `event` and
`arg1` to `argN` set, and helpers the rest of the client uses to read or write globals. The state
itself, its base library (`setglobal` to `debugstack`, from 0x702770 to 0x703760) and the contract
of C functions are in `luastate`. The run `0x705e30` to `0x706430` after it registers the
frame-type method tables and the global script functions `CreateFrame`, `CreateFont`,
`GetNumFrames` and `EnumerateFrames`.

Roots: `0x7019c0`, `0x703bf0`, `0x703cf0`, `0x703d90`, `0x703e50`, `0x703f50`, `0x702140`, `0x702280`, `0x7023e0`, `0x7024f0`, `0x702590`, `0x7025c0`, `0x702670`, `0x702690`, `0x7026f0`, `0x702710`, `0x701ae0`, `0x701b10`, `0x701bd0`, `0x701cd0`, `0x701d80`, `0x702000`, `0x706350`, `0x7063d0`

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

## Listener order

Listeners of an event run in the order they registered: registering appends at the tail of the
event's list, and signalling walks from the head. Registering an object already on the list
changes nothing, so it keeps its place; unregistering frees its node at once, so registering it
again puts it last. Registering for all events appends the object to every list it is not on, so
per-event and all-event registrations interleave by time. Nothing else orders a list.

Addresses: `0x702140`, `0x7021a8`, `0x702264`, `0x7021bf`, `0x7052d0`, `0x705359`, `0x705364`, `0x705368`, `0x702280`, `0x702309`, `0x702318`, `0x7023e0`, `0x703e50`, `0x703ecd`
Confidence: verified
Proof: one
Test: none

`0x7052d0` with its first argument 2 (`0x7021bf`) links the node after the record's tail cursor
`[rec+0x8]` (`0x705359` to `0x705364`) and makes it the new tail (`0x705368`); the first node of an
empty list is written into the head `[rec+0xc]`, which no later append changes. The walk starts at
`[rec+0xc]` (`0x703ecd`) and follows `+0x4`. A match at `0x7021a8` returns 1 at `0x702264` before
any insert. `0x702280` unlinks (`0x702309`) and frees the node (`0x702318`). `0x7023e0` runs the
same append per record.

## Changes during a signal

The walk reads each node's successor before running its listener and reads nothing else ahead.
A listener that unregisters itself is safe. One that unregisters the next listener leaves the walk
holding a freed node, which it then reads: a fault of the client. An object registered during the
signal is reached unless the running listener is the last. A signal raised inside a listener runs
to its end first, and the outer `event`, `this` and arguments are restored after it. An error ends
only its own listener: the error handler runs and the walk goes on.

Addresses: `0x703ee8`, `0x703f00`, `0x702318`, `0x704e79`, `0x704e80`, `0x704e8e`, `0x705199`
Confidence: verified
Proof: one
Test: none

`0x703ee8` loads `[node+0x4]` into `edi` before the call and `0x703f00` continues from it; a tail
node's successor is the tagged end, so a node appended after it is not seen. `0x702318` frees with
no deferral. After `lua_pcall` (`0x704e79`, `0x705199`) a nonzero result pops the message
(`0x704e80`) and `0x704e8e` pops the handler; both runners return normally.

## Handler arguments

The argument runner sets `arg1` onward before `this` and restores `this` before them. `%d` gives a
signed integer, `%u` an unsigned one and `%f` a double, each as a number; `%s` gives a string, or
nil for a null pointer. Any other letter after `%`, and any other character, is skipped. Only the
arguments the format names are set: a handler reads a higher `argN` as whatever that global holds,
nil unless an enclosing signal set it.

Addresses: `0x704f10`, `0x704f7e`, `0x704f89`, `0x704fa6`, `0x704fbe`, `0x704fe3`, `0x704ffb`, `0x705045`, `0x705100`, `0x7051be`, `0x705212`, `0x7052a4`, `0x7052b8`, `0x6f3890`
Confidence: verified
Proof: one
Test: none

`0x704f7e` tests for `%` and `0x704f89` indexes `0x7052b8` by the letter minus `d`: `d` to case 0
(`fild dword`, `0x704fa6`), `f` to 1 (`fld qword`, `0x704fe3`), `s` to 2 (`0x6f3890`, which
pushes nil for null, `0x704ffb`), `u` to 3 (the dword zero-extended, `0x704fbe`), every other
letter to `0x705095`. Each value is set as `argN` after its old value is referenced (`0x705045`);
`this` follows (`0x705100`). After the call `this` is restored (`0x7051be`), then `arg1` to `argN`
in order (`0x705212`).

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

## Reading and writing globals

The client reads and writes globals through small helpers, each on the current state with
`lua_gettable` and `lua_settable` on the globals: `0x704250` sets a global to a single-precision
float, `0x704300` to a string or, for a null or empty one, nil, and `0x704350` reads one as text
when it is a string or number, answering whether it is one. `0x7040e0` raises a Lua error with a
formatted message prefixed by the calling position, as `luaL_error` does. The integer and nil
forms are unreached (`README`).

Addresses: `0x704250`, `0x70425c`, `0x704300`, `0x70430d`, `0x70431f`, `0x704350`, `0x704378`, `0x7040e0`, `0x7040f2`, `0x704113`, `0x7041a0`, `0x7041f0`, `0x7042a0`, `0x7043b0`
Confidence: verified
Proof: one
Test: none

`0x704250` loads its argument with `fld dword` (`0x70425c`) before `lua_pushnumber`. `0x704300`
tests the pointer and its first byte (`0x70430d`) and pushes nil otherwise (`0x70431f`). `0x704350`
tests `lua_isstring` (`0x704378`) before `lua_tostring` into its out argument. `0x7040e0` calls
`luaL_where(1)` (`0x7040f2`), formats, concatenates two and calls `lua_error` (`0x704113`).

## Text, counts and running code

`FrameScript_GetText(key, count, gender)` with no gender and a negative count answers the global
`key` when it is a string or number, else `""`; otherwise it calls the Lua function `GetText` with
the key, the gender or nil and the count or nil, under the error handler, and answers its string
result or `""`. `0x703cf0(count)` calls `GetPluralIndex(count)` and answers the result truncated
minus one, or 0 on an error or a non-number. `0x704cd0` runs a string as a chunk under the error
handler (`framexml`, `Loading one file`, for the runner); `0x704cf0` calls a function reference the
same way, discarding an error after the handler.

Addresses: `0x703bf0`, `0x703c0d`, `0x703c12`, `0x703c4d`, `0x703cad`, `0x703cb7`, `0x703cf0`, `0x703d47`, `0x703d6f`, `0x704ae0`, `0x704bc0`, `0x704cd0`, `0x704ce0`, `0x704cf0`, `0x704d22`, `0x872cc8`, `0x872cd0`
Confidence: verified
Proof: one
Test: none

`0x703c0d` takes the fast path for gender 0 and a negative count to `0x704350` (`0x703c12`). The
other path pushes the handler, `GetText` (`0x703c4d`), the key, nil or the gender and nil or the
count, then `lua_pcall(3, 1, handler)` (`0x703cad`) and reads a string or number result
(`0x703cb7`). `0x703cf0` calls `lua_pcall(1, 1, −3)` (`0x703d47`) and decrements `__ftol` of a
number result (`0x703d6f`). `0x704cd0` passes the string, its length and the name to `0x704ae0`
(`0x704ce0`); `0x704cf0` pushes the handler and the reference and calls `lua_pcall(0, 0, −2)`
(`0x704d22`).

## Creating a frame from Lua

`CreateFrame(type, name, parent, inherits)` raises its usage unless `type` is a string or number and
`parent` is absent, nil or a table. `name` and `inherits` are read as strings, a number becoming
one, anything else absent; a string `inherits` must name a template, else it raises. The frame is
created from a node of that type with those attributes. With a `parent` table the node also gets an
empty `parent`, so the table is the parent; without one, it is the first `parent` the `inherits`
chain gives, an empty one ending the walk with none. `CreateTexture` and `CreateFontString` read
`inherits` only when it is a string, the font string trying a font object first; their layer
defaults to `ARTWORK`.

Addresses: `0x7060b0`, `0x7060d5`, `0x7060e9`, `0x70610f`, `0x706121`, `0x70613f`, `0x706158`, `0x7061ab`, `0x7061cb`, `0x7061dd`, `0x7061ed`, `0x706211`, `0x706232`, `0x70623b`, `0x70628b`, `0x7062a0`, `0x7062af`, `0x773a20`, `0x773ae0`, `0x773b12`, `0x773c30`, `0x773d25`, `0x773d39`, `0x773d47`, `0x6f3690`, `0x6f7c80`
Confidence: verified
Proof: one
Test: none

`0x7060d5` tests `lua_isstring(1)`; `0x7060e9` to `0x70610f` accept type −1, 0 or 5 for argument 3.
`lua_tostring` reads 1, 2 and 4 (`0x706121`, `0x70613f`); for a number it converts the slot in place
(`0x6f7c80`), so `0x7061cb`'s later `lua_type(4) == 4` holds for it and `0x7061dd` looks it up,
raising at `0x7061ed`. A table parent is tested at `0x7061ab`. `0x706211` builds the node, adding
`parent` with `""` (`0x70623b`) only when a parent is given (`0x706232`, `test edi,edi; je`);
without it `0x6ee280` walks `inherits` for a `parent` (`0x6ee390` to `0x6ee3c2`); `0x70628b` creates
and `0x7062a0` raises on null, an unknown type; `0x7062af` destroys the status, discarding the
load's messages. `0x773ae0` stages 2; `0x773b12` and `0x773d25` compare `lua_type(4)` with 4;
`0x773d39` tries `0x783870` before `0x773d47`.

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

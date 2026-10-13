# core: events

The event bus every system plugs into. A context holds 29 handler lists, one per event category;
code registers a handler on a category of the current thread's context, and the library calls
the handlers of a category, highest priority first, whenever it dispatches that category.

The loop dispatches the context categories (`core/loop`): 7 at start, 6, 5 (frame time) and 0x11
each tick, 3 and 4 at teardown. The window input dispatches the input categories
(`core/input`): 1 and 2, 8 to 0x10, 0x1b and 0x1c. Nothing in the library dispatches 0, or 0x12
to 0x1a. The same object files hold the held-key and held-button state the dispatch keeps, the
deferred message queue, the timers and the observer classes.

Roots: `0x41fc90`, `0x41fca0`, `0x41fd80`, `0x41fd90`, `0x41f8f0`, `0x41fe40`, `0x4200a0`, `0x4203e0`, `0x423510`, `0x427180`, `0x428fb0`, `0x4291f0`, `0x4292e0`, `0x429320`, `0x429340`, the vtable slots `0x422970`, `0x4229e0`, `0x422a50`, `0x423240`, `0x4251e0`, `0x4251f0`, `0x426190`, `0x4261c0`, `0x4261f0`, `0x426220`, `0x426250`, `0x426280`, and the static initializers `0x4231d0`, `0x423870`, `0x423890`, `0x424410`, `0x424430`, `0x424460`, `0x424480`, `0x4252c0`, `0x4252e0`, `0x425380`, `0x4253a0`, `0x426140`, `0x426160`, `0x4262c0`, `0x4262e0`, `0x426340`, `0x4263a0`, `0x427ba0`, `0x427bc0`, `0x428700`, `0x428f60`, `0x428f80`

## Handler records

A handler is a 0x18-byte record in its category's list: a link, the function, its argument, a
float priority, and a skip flag at `+0x14`. A list's head is 12 bytes at `+0x5c + 12·category`
of the context: the link offset 0, then the end link. The function is called as
`fastcall(payload, argument)` and returns nonzero to let the dispatch go on.

Addresses: `0x424f00`, `0x422d50`, `0x832de4`
Confidence: verified
Proof: two
Test: `core::event_dispatch_order_matches_binary`

`0x424f00` allocates 0x18 bytes tagged `.?AUEvtHandler@@` (`0x424f30`) and stores the function at
`+0x08`, the argument at `+0x0c`, the priority at `+0x10` and 0 at `+0x14` (`0x424f4d` to
`0x424f59`). The constructor sets each head's link offset to 0 and its end link to point at itself,
marked by bit 0 (`0x422d50`, `0x422160`). The call through `+0x08` is at `0x4246ad`, with `ecx` the
payload and `edx` the argument. The test runs `0x422d50`, `0x424f00` and `0x4245b0` whole, with the
handlers stubbed.

## Handler order

A new handler goes before the first record whose priority is not greater than its own and
whose skip flag is clear; with none, at the end. So a list runs from the highest priority to the
lowest, and among equal priorities the newest runs first. A NaN on either side compares as not
greater, so a NaN handler goes before the first unskipped record, and a NaN record stops every
later handler from passing it.

Addresses: `0x424f70`, `0x424f73`, `0x424f78`, `0x424f7d`, `0x424fcd`, `0x424fe9`
Confidence: verified
Proof: two
Test: `core::event_dispatch_order_matches_binary`

The walk loads the record's priority and `fcomp`s the new one (`0x424f70`, `0x424f73`); `test
ah,0x41; je` advances when neither C0 nor C3 is set, that is when the record is greater
(`0x424f78`). Equal sets C3 and unordered sets all three, so both stop there, unless `+0x14` is
nonzero (`0x424f7d`). The new record is linked before the stopping record, or before the end link
(`0x424fcd` to `0x424fe9`). The test runs `0x424f00` and `0x4245b0` whole.

## Dispatch

`0x4245b0` first updates the held-key and held-button state for the category, which can turn
a key-down into a repeat (`Held keys and repeats`). It then calls the category's handlers from
the head of the list, skipping records whose skip flag is set, and stops at the first handler
that returns 0. Handlers may add or remove records during the walk: a removed record not yet
reached is never called, and a record added after the current one, by priority, is called in the
same walk. Nothing is called while a fatal error is being reported, or for a null context.

Addresses: `0x4245b0`, `0x424710`, `0x4245e0`, `0x424637`, `0x424660`, `0x4246a0`, `0x4246ad`, `0x4246b0`, `0x425220`, `0x425250`, `0x425290`, `0x425310`, `0x425340`
Confidence: verified
Proof: two
Test: `core::event_dispatch_order_matches_binary`, `core::demux_route_matches_binary`

The dispatch is `fastcall(context, category)` with the payload pushed, `ret 4`. `0x424710` gets a
pointer to the category (`0x4245d6`), and the list is chosen after it (`0x4245f8`). `0x4245e0` calls
Storm's `SErrIsDisplayingError`, which reads the fatal-error flag. The walk keeps a cursor record on
the stack with its skip flag set, linked after the end link and moved past each record before it is
called (`0x424660` to `0x42469d`), so the record called may unlink itself and a new record stops on
no cursor. `0x4246a0` skips a set flag; `0x4246b0` stops on zero. `0x425220`, `0x425290` and
`0x425310` find a record's link, and `0x425250` and `0x425340` unlink one.
`core::event_dispatch_order_matches_binary` runs `0x4245b0` whole with `0x424710` stubbed, and
`core::demux_route_matches_binary` runs `0x424710` whole.

## Registration calls

`EventRegisterEx` (`0x41fca0`) takes a category, a function, an argument and a priority, and
registers on the calling thread's context; `EventRegister` (`0x41fc90`) uses argument 0 and
priority 0.0. A category outside 0 to 28 or a null function raises error 0x57. `EventUnregisterEx`
(`0x41fd90`) removes every unskipped record matching a category, function and argument, each test
enabled by flag bits 1, 2 and 4; `EventUnregister` (`0x41fd80`) matches all three with argument 0.

Addresses: `0x41fca0`, `0x41fc90`, `0x41fd90`, `0x41fd80`, `0x425000`, `0x801354`, `0xcf7700`, `0x41db90`
Confidence: verified
Proof: two
Test: none

The range test is `jl`, `cmp 0x1d; jge` and a zero test on `edx` (`0x41fcb0` to `0x41fcb9`). The
context is found by the handle `0x41db90(0)` returns for the thread, in bucket `handle & 7` of
`0xcf7700`, walked by `+0x08` and matched on `+0x0c` under the bucket's read lock (`0x41fcca` to
`0x41fd1f`); no context means a silent return. `0x41fc90` pushes `[0x801354]` (0.0) and 0.
`0x425000` runs the 29 lists from `+0x64`; flag 1 skips other categories, 2 compares the function,
4 the argument, and a match is unlinked and freed at once (`0x425039` to `0x4250a1`). `0x41fd80`
passes flags `0xffffffff`.

## Held keys and repeats

Each context keeps a list of held keys. A key-down (category 8) removes the key from the list,
dispatches as category 0xa (repeat) instead if it was there, and appends it. A key-up (9) removes
it. Category 2, the window's activation change, empties the list. `EventIsKeyDown` (`0x41f8f0`)
answers whether a key code is on the calling thread's list. The Windows repeat count plays no
part.

Addresses: `0x424710`, `0x424780`, `0x424770`, `0x424810`, `0x4248a5`, `0x4248b3`, `0x424790`, `0x41f8f0`, `0x424510`, `0x832dac`
Confidence: verified
Proof: two
Test: `core::demux_route_matches_binary`

`0x424710` indexes the byte map `0x424780` by `category − 2` (`ja` for anything outside 2 to 0xe)
into the table `0x424770`: 2 runs `0x424790`, 8 and 9 run `0x424810` with the payload's first dword,
0xb and 0xe run `0x424920`, the rest nothing. In `0x424810` every node with that key is unlinked and
freed, remembering a find; for category 8 a find stores 0xa through the category pointer
(`0x4248b3`), and a 12-byte node tagged `.?AUEvtKeyDown@@` is appended. The list is `+0x1c8` to
`+0x1d0`, under the context lock. `0x424510` walks it for a match. The test runs `0x424710` whole
with the three functions it routes to stubbed.

## Held buttons

Each context also keeps a mask of held mouse buttons at `+0x1c4`: a button-down (0xb) ORs in the
payload's button bit, a button-up (0xe) clears it, and category 2 clears the whole mask. Its one
reader is unreached, so the mask has no effect in this build.

Addresses: `0x424920`, `0x424756`, `0x42479b`
Confidence: verified
Proof: two
Test: none

`0x424756` passes `category == 0xb` and the payload's second dword; `0x424920` ORs or clears
with it. `0x42479b` zeroes the mask. A census of `+0x1c4` in the library finds these, the
constructor's store of 0 at `0x422216`, and one reader, `0x42459e` in the unreached `0x424590`.

## Deferred messages

Each tick, and at teardown, `0x424ad0` takes the context's message queue under its lock and
dispatches each message's category with its payload, freeing it after. The only function that
queues one, `0x424c00`, is called only by `0x41fa80`, which nothing reaches, so in this build the
queue is always empty.

Addresses: `0x424ad0`, `0x424a70`, `0x424c00`, `0x424cd0`, `0x41fa80`, `0x424490`, `0x4244d0`, `0x424500`, `0x883f64`, `0x883f7c`, `0x4251e0`, `0x4251f0`, `0x760210`, `0x42449e`
Confidence: verified
Proof: one
Test: none

A message is a size, a link, the category at `+0x0c` and a copy of the payload at `+0x10`
(`0x424c32`, `0x424c48`). `0x424cd0` takes it from the pool `0x883f64` when `max(size, 4) + 0x10`
is at most `0x883f7c`, 0 in the file and 0x100 from `0x42449e`, so for payloads up to 240
bytes, and otherwise through the pool's slot `0x760210`, which calls `SMemAlloc`. The drain swaps the queue at `+0x1b8` for an empty one (`0x424b2d` to `0x424b69`)
and calls `0x4245b0(category = +0x0c, payload = +0x10)` (`0x424b7f` to `0x424b8a`). `0x424490`
builds the pool and `0x4244d0`/`0x424500` destroy it.

## Timers

`EventSetTimer` (`0x4200a0`) takes a delay in milliseconds, a function and an argument, and
returns a timer id for the current context. The timer is due at `now + delay`; when the loop's
timer phase finds it due, it is removed and its function called once as
`fastcall(&{delay · 0.001f, now}, argument)`. Due timers run in due-time order and the phase reads
the clock once. `EventKillTimer` (`0x4203e0`) clears a timer's functions, and dead timers are
dropped from the heap's top.

Addresses: `0x4200a0`, `0x4203e0`, `0x428b30`, `0x428510`, `0x428720`, `0x428820`, `0x428ee0`, `0x4290c0`, `0x832f2c`
Confidence: verified
Proof: one
Test: `core::evttimer_ticks_to_seconds_matches_binary`

`0x428b30` reuses an id from the free list or takes the next (at least 1), allocates a 0x38-byte
record, and stores `due = now + delay` at `+0x10` (`0x428c09`, `0x428c14`), the delay as
`fild QWORD` of the unsigned delay times `0.001f` at `+0x14` (`0x428c4e` to `0x428c6b`), the
functions at `+0x18` and `+0x20` and their arguments, then sift-inserts it in the heap at
`+0x1f4`. `0x428510` reads the clock once (`0x42854e`), stops at a root with `due − now > 0`
signed (`0x4285f2`), and calls with the lock released (`0x428659`). The test runs `0x428c4e` to `0x428c6e`.

## Observer classes

The observer and registration classes (`CObserver.cpp`, `CInputObserver.cpp`, `EventReg`) are
unreached: no live code installs their vtables, so their slots and everything those reach never
run. What the interface uses from these files is the event vtables from `0x8013cc` to
`0x8013f4`, which it stores from its own code; their slots are `0x41f3f0` and the deleting
destructors listed here.

Addresses: `0x8013b0`, `0x801404`, `0x801420`, `0x801430`, `0x8013cc`, `0x8013f4`, `0x426190`, `0x4261c0`, `0x4261f0`, `0x426220`, `0x426250`, `0x426280`, `0x425430`, `0x426450`, `0x4265b0`, `0x41f3f0`
Confidence: verified
Proof: one
Test: none

The input observer vtable `0x8013b0` is stored only at `0x42545d`, in the unreached `0x425430`.
The observer vtable `0x801404` is stored only by `0x426450`, which only the destructor slots
`0x4254a0` and `0x4283f0` of those two vtables call. The registration vtables `0x801420` and
`0x801430` are stored by `0x4265b0`, `0x427990` and `0x428470`, reached only from those slots.
The interface stores `0x8013cc` to `0x8013f4` (`0x765e0e`, `0x765f43`, `0x7660a7`); each holds
`0x41f3f0` and one of `0x426190` to `0x426280`, which reset the vtable and free. The class names
are the type tags at `0x426414` to `0x42643a`.

## Shared arrays

Growable arrays of dwords from these object files are used across the client: set the count,
add elements, reallocate, free. Their capacity rounds up to a multiple of a quantum: the largest
power of two not above the size for sizes below 64, else 64, and 1 for size 0.

Addresses: `0x4292e0`, `0x429320`, `0x4291f0`, `0x428fb0`, `0x429340`, `0x423510`, `0x427180`, `0x832e00`
Confidence: verified
Proof: one
Test: none

`0x4292e0` repeats `x & (x − 1)` while nonzero and keeps the last nonzero value: the highest set
bit (`0x4292eb` to `0x4292f9`); 64 and up store 64 as the fixed quantum at `+0x0c` (`0x429309`).
`0x429320` rounds its first argument up to a multiple of its second with `div`. The arrays are
tagged `.I` (`0x832e00`); `0x423510` frees one. `0x427180` returns 1 for a null or bit-0-marked
link, else 0. The other three were read for their calls only.

## Static initializers

The bus, message, observer and timer object files run 22 static initializers: the +∞, 2π and
1/2π stores of each file, the context table and lock again, the cursor rectangle cleared, the
message pool, and the three registration pools.

Addresses: `0x827378`, `0x4231d0`, `0x423230`, `0x423870`, `0x423880`, `0x423890`, `0x4238a0`, `0x4238d0`, `0x4238e0`, `0x424410`, `0x424420`, `0x424430`, `0x424440`, `0x424460`, `0x424470`, `0x424480`, `0x4244c0`, `0x4252c0`, `0x4252d0`, `0x4252e0`, `0x4252f0`, `0x425380`, `0x425390`, `0x4253a0`, `0x4253b0`, `0x4253c0`, `0x4253d0`, `0x426140`, `0x426150`, `0x426160`, `0x426170`, `0x4262c0`, `0x4262d0`, `0x4262e0`, `0x4262f0`, `0x426310`, `0x426320`, `0x426340`, `0x426350`, `0x426370`, `0x426380`, `0x4263a0`, `0x4263b0`, `0x4263d0`, `0x4263e0`, `0x427ba0`, `0x427bb0`, `0x427bc0`, `0x427bd0`, `0x428700`, `0x428710`, `0x428f60`, `0x428f70`, `0x428f80`, `0x428f90`
Confidence: verified
Proof: one
Test: none

They sit in the table from `0x827378` through `0x8273cc`, and the same shape census as
`core/loop#static-initializers` matched each. `0x4231d0` repeats `0x420850` under the same guard
bit, so it builds nothing and only registers the teardown `0x4208c0` again, which its own guard
bit runs once. `0x4238a0` zeroes the cursor rectangle `0x883f48` to `0x883f54`.
`0x4253b0` clears the input observer pointer `0x883f94`. `0x4262f0`, `0x426350` and `0x4263b0`
build pools at `0x883fd0`, `0x883ffc` and `0x883fa0` with their locks.

## Unreached functions

The `Event` calls without a caller (context queries, posting, input queries, timer variants), the
message posting and handler marking they lead to, the cursor rectangle setters, the input
observer's converters and its attach and detach pairs, and the observer and registration classes
(`Observer classes`): nothing reaches any of them.

Addresses: `0x4254a0`, `0x4262b0`, `0x426450`, `0x426470`, `0x4265b0`, `0x4266c0`, `0x426850`, `0x426870`, `0x4269b0`, `0x4269e0`, `0x426c20`, `0x426c60`, `0x426d30`, `0x426d70`, `0x426ec0`, `0x426f90`, `0x4270b0`, `0x4271a0`, `0x427290`, `0x427340`, `0x427460`, `0x427550`, `0x427680`, `0x4276c0`, `0x4276e0`, `0x4276f0`, `0x427720`, `0x427750`, `0x427810`, `0x4278d0`, `0x427990`, `0x427a50`, `0x427a80`, `0x427ac0`, `0x427af0`, `0x427b30`, `0x427b60`, `0x427bf0`, `0x427c10`, `0x427ca0`, `0x427d30`, `0x427d80`, `0x427f70`, `0x427fa0`, `0x428050`, `0x428060`, `0x4281a0`, `0x4282e0`, `0x4283f0`, `0x428420`, `0x428470`, `0x425430`, `0x424c00`, `0x424cd0`, `0x41f630`, `0x41f6f0`, `0x41f7a0`, `0x41f840`, `0x41fa80`, `0x41fb90`, `0x41fe50`, `0x41ff00`, `0x41ffd0`, `0x420240`, `0x420310`, `0x420490`, `0x4206a0`, `0x420740`, `0x4242b0`, `0x4242f0`, `0x424370`, `0x424590`, `0x424950`, `0x4249b0`, `0x424d70`, `0x4250d0`, `0x4254d0`, `0x425520`, `0x425570`, `0x4255c0`, `0x425610`, `0x425690`, `0x425710`, `0x425790`, `0x425810`, `0x425890`, `0x425910`, `0x425960`, `0x4259a0`, `0x4259e0`, `0x425a30`, `0x425a70`, `0x425ac0`, `0x425b00`, `0x425b50`, `0x425b90`, `0x425be0`, `0x425c20`, `0x425c70`, `0x425cb0`, `0x425d00`, `0x425d40`, `0x425d90`, `0x425dd0`, `0x425e20`, `0x425e60`, `0x425eb0`, `0x425ef0`, `0x425f40`, `0x425f80`, `0x425fd0`, `0x426010`, `0x426060`, `0x4260a0`, `0x4260f0`, `0x426130`, `0x426a10`, `0x426af0`, `0x426bd0`, `0x426bf0`, `0x4273e0`, `0x427420`, `0x428790`, `0x428960`, `0x428d00`, `0x420170`
Confidence: verified
Proof: one
Test: none

The census of `core/loop#unreached-functions` finds no `e8`/`e9` and no dword at any alignment
for any of them outside this list, the import tables, and the vtables of `Observer classes`.
`0x41fa80`'s only dword match is unaligned, inside a float table. `0x420170`, the timer variant
that sets the second function form, is called only at `0x40153f`, in the unreferenced `0x401530`. `0x425430`, which builds the input observer, is called 78 times,
all from the attach and detach pairs here. The converters `0x425610` to `0x425890` are referenced
only by those pairs; the interface's live converter is `0x766f70`, which calls the same scaling
`0x41ad80` at `0x766fb1`.

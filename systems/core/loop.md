# core: the frame loop

The engine's event library runs the client: a scheduler that sleeps until a context is due, then
runs one tick of it. A tick fans out to the rest of the client through event categories
(`events`), and its clock is the millisecond clock (`time`).

The library is the object files from `0x41f550` through `0x429340`: the public `Event` calls,
the scheduler (`EvtSched.cpp`), contexts, handler lists and messages, the input bridge, the
observers and the timers (`EvtTimer.cpp`). Its state is the slot table at `0x883f18`, the
shutdown event `0x883f24`, and one context of 0x214 bytes. In this build `WinMain` starts it with
one thread and creates one context, so the scheduler always runs that one context on the main
thread, and the heap and load balancing never choose between two.

Roots: `0x41f550`, `0x41f5e0`, `0x41f5c0`, `0x41f5a0`, `0x41f6e0`, `0x41f9a0`, `0x41f9b0`, and the
static initializers `0x41f580`, `0x4207e0`, `0x420800`, `0x420850`, `0x4209b0`, `0x4209d0`,
`0x420a10`, `0x420b00`, `0x420b60`, `0x420ba0`, `0x422c40`, `0x422c60`

## Startup and shutdown

`WinMain`'s body calls the library four times. `0x41f550` with one thread and polling off sets up
one scheduler slot; `0x41f5e0` creates the interactive context, with the start handler `0x402840`
on category 7 and the stop handler `0x403640` on category 4, each at priority 1000.0; `0x41f5c0`
runs the loop on the calling thread until shutdown; `0x41f5a0` frees the library. No other thread
runs a loop.

Addresses: `0x41f550`, `0x4219d0`, `0x41f5e0`, `0x41f5c0`, `0x420be0`, `0x41f5a0`, `0x421cb0`, `0x883f0c`, `0x883f10`, `0x883f20`, `0x423910`, `0x4244f0`, `0x4253f0`, `0x426400`, `0x4238f0`, `0x424500`, `0x425400`, `0x426410`, `0x402120`, `0x402350`, `0x403bc0`
Confidence: verified
Proof: two
Test: none

The calls are at `0x4027f8` (`ecx` 1, `edx` 0), `0x402825` (`ecx` 1, `edx` `0x402840`, pushes 0, 0,
`0x403640`), `0x4021e4` and `0x403bd3`. `0x41f550` calls three empty module setups and the input
setup `0x423910`, raises the thread count to 1, and tail-jumps to `0x4219d0`, which stores `edx` in
`0x883f0c`, rounds the thread count up to a power of two (one) into `0x883f10`, builds one critical
section per slot and the zeroed slot table, creates slot 0 through `0x421080` into `0x883f20`, and
starts `count − 1` threads at `0x420c00` (`0x421b5b`): none. `0x41f5c0` jumps to `0x420be0`, which
sets the start gate `0x883ec4` and calls `0x420c00(1)`. `0x41f5a0` calls `0x421cb0`, which frees the
slots and thread records, then the input, message, observer and registration modules.

## Contexts

A context is a 0x214-byte object: a lock at `+0x10`, run state `+0x2c`, a heap node `+0x30`
whose key `+0x3c` is the due time, the last tick time `+0x40`, flags `+0x44`, the idle time
`+0x48` and `+0x4c`, a load cost `+0x50` and `+0x54`, 29 handler lists from `+0x5c`, and the
message, input, timer and thread state after them. For the client's context the constructor
stores: run state 0, last tick = now, flags 2 (interactive), idle time 1 ms in both fields, cost
1000 in both, and gives it a handle in the context table.

Addresses: `0x422090`, `0x4222a0`, `0x41f600`, `0x422d50`, `0x422f80`, `0x422f90`, `0x422fa0`, `0x422fb0`, `0x422960`, `0x422990`, `0x4229b0`, `0x883ef0`, `0xcf7700`, `0x4224f0`
Confidence: verified
Proof: two
Test: none

`0x422090` allocates 0x214 bytes (`0x422113`). The idle time is the second stack argument, raised to
1 when below 1 (`0x4220a3` to `0x4220af`), stored at `+0x48` and `+0x4c` (`0x422207`). The cost is
`neg; sbb; and 0x3e7; inc` on `ecx` (`0x4220f7` to `0x422112`): 1000 for an interactive context, 1
otherwise, at `+0x50` and `+0x54`. Flags are 2 for `ecx` nonzero (`0x4221f7` to `0x422204`), the bit
`0x4224f0` returns; `+0x40` is the clock (`0x4221fb`). An interactive context increments `0x883ef0`.
The handler in `edx` goes on category 7 and the first stack argument on category 4, both with
`0x447a0000` (1000.0) and argument 0 (`0x422256` to `0x422280`). `0x4222a0` then picks the
least-loaded slot, assigns the next free handle from `0xcf76f8`, links the context into the 8-bucket
table `0xcf7700` by `handle & 7`, sets its due time to the clock, inserts it in the slot's heap,
sets the slot's event and adds its cost to the slot's load. `0x41f600`, the variant with idle time
500 for a non-interactive context, has no caller.

## The loop

`0x420c00` waits for the start gate, names its thread `Engine <id>` and registers it. Each
iteration then restores the thread priority saved at startup, leaves the loop when the shutdown
event is set, takes the context due soonest, and sleeps on the slot's event for the time until
that context is due, or forever when none is. When the sleep ends by timeout, it runs one tick
(`One tick`); when it is woken early, it only reschedules. A context in run state 1 is torn down
instead of rescheduled. On exit it releases its slot, which tears down every context left in it.

Addresses: `0x420c00`, `0x6599c0`, `0x421430`, `0x421560`, `0x420cc8`, `0x420cf8`, `0x420d7e`, `0x4211d0`, `0x883ec4`, `0x883ec0`, `0x883f24`, `0x437060`, `0x4371e0`
Confidence: verified
Proof: two
Test: none

`0x6599c0` is `WaitForSingleObject` on the handle at `ecx`. The gate wait is infinite on
`0x883ec4` (`0x420c2d`). The exit test waits 0 ms on `0x883f24` and leaves on a zero return,
`WAIT_OBJECT_0` (`0x420c6f`, `0x420c76`). The sleep is `due − now` from `0x421430`'s context,
0 when negative (`0x420c97`, `jns`), −1 (infinite) with no context, on the event `0x421560`
returns (`0x420cc8`). Only a result of `0x102`, `WAIT_TIMEOUT`, runs the tick (`0x420cf8`). With
`0x883f0c` set, the loop would pump through `0x43f050` and force `0x102`; `WinMain` passes 0.
The run-state test at `0x420d7e` sends a state-1 context to `0x421370` and `0x420e60`.

## One tick

A tick runs, in order: the start dispatch (category 7, first tick only); the due timers; for an
interactive context, the queued window input, which may request shutdown; category 6; the deferred
messages; the frame time (category 5); and category 0x11. Before the first call the context's handle
is made the thread's current context and the start time is read; after the last, the current context
is cleared and the context rescheduled. Categories 6, 5 and 0x11 are skipped once the context is in
run state 1.

Addresses: `0x420d05`, `0x420d0c`, `0x420d23`, `0x420d4e`, `0x420d55`, `0x420d5c`, `0x420d63`, `0x420d6a`, `0x420e20`, `0x428510`, `0x423920`, `0x420ff0`, `0x424ad0`, `0x420f70`, `0x421030`, `0x4374d0`, `0x437560`, `0x41db50`, `0x41dbb0`, `0x41db90`, `0x436db0`
Confidence: verified
Proof: two
Test: none

The calls follow one another from `0x420d05` to `0x420d6a`. `0x423920` runs only when flag 2 is set
(`0x420d11`); its out-flag set means the window asked to quit, and then run state 0 becomes 1 under
the context lock and `0x422030` sets the shutdown event (`0x420d2f` to `0x420d4e`). `0x420ff0`
dispatches category 6 with a null payload unless run state is 1. `0x421030` dispatches category 0x11
only when flag 4 is set, clearing it first; `0x420f70` sets flag 4 on an interactive context, so the
client gets 0x11 every tick. The current context is set by `0x41db50(+0x208)` and `0x41dbb0(0,
+0x0c)` (`0x420cd3` to `0x420ce3`), which `0x41db90(0)` reads back, and cleared by `0x41db50(0)` at
`0x420dec`. The calls of `0x4374d0` (`0x420cee`) and `0x437560` (`0x420de5`) pass `+0x20c`, which is
0 here: the constructor fills it only when bit 0 of its third argument is set (`0x4220b6`), and
`WinMain` passes 0 (`0x402812`), so both return at once.

## Next due time

After a tick the next due time is the tick's start time `s` plus a wait `w`. With flag 4 still
set, `w` is 0. Otherwise `w` starts as the time to the earliest timer (0 when overdue, `0xffffffff`
with none), or as the idle time `+0x48` when the two idle fields differ, and is then lowered to
`max(0, last + idle − s)`, compared unsigned, with `last` the time the frame time was taken. For
the client (idle 1 ms) the next tick is due about 1 ms after the last frame time, sooner if a
timer is due.

Addresses: `0x420da5`, `0x420db6`, `0x420dc0`, `0x420dc7`, `0x420dd3`, `0x420dd9`, `0x420dfd`, `0x428720`, `0x421570`
Confidence: verified
Proof: two
Test: none

`0x420da5` tests flag 4. `0x428720` returns the earliest timer's due time minus `s`, zeroed when
negative by `sets dl; dec edx; and` (`0x428765` to `0x42876e`), or −1 for none. `0x420dc0` replaces
it with `+0x48` when `+0x48 ≠ +0x4c`. `eax = +0x40 − s + +0x48` is zeroed when negative the same way
(`0x420dc7` to `0x420dd7`), and `cmp edi,eax; jb` keeps the smaller as unsigned (`0x420dd9`).
`0x421570` receives `s + w` and the cost `+0x54` (`0x420df1` to `0x420dfd`). It stores the due time
in `+0x3c` and re-sifts on a change (`0x421590`), sets `+0x58` when the cost moved by at least
`+0x50 >> 3` (`0x4215b4` to `0x4215bc`), and puts the context back in a slot's heap. Only the path
that moves it to another slot sets that slot's event (`0x421732`); the same-slot path does not
(`0x42164a`, `0x42177e` to `0x421826`). The load balancing is not read here.

## Frame time

Every tick of a running context dispatches category 5 with a pointer to a float and a dword: the
seconds since the previous frame time, then the clock value. The float is the signed 32-bit
difference of clock values in milliseconds times `0.001f`, rounded once to a float. The clock
value read becomes the new `+0x40`. On the first
tick the difference counts from the start dispatch, which also stamps `+0x40`.

Addresses: `0x420f70`, `0x420f9b`, `0x420faa`, `0x420fb6`, `0x420fc7`, `0x420fd5`, `0x801360`
Confidence: verified
Proof: two
Test: `core::dt_seconds_matches_binary`

`0x420f9b` reads the clock; `sub ecx,edi` takes `now − [+0x40]` (`0x420fa5`), `fild DWORD` loads it
signed, and `fmul DWORD [0x801360]` multiplies by the float `0x3a83126f` at 53-bit precision. `fstp
DWORD` (`0x420fc7`) rounds the product to a float, so the result is the 53-bit product rounded once,
not `delta as f32 * 0.001f`; the clock value follows it (`0x420fd2`). The store of `now` to `+0x40`
is at `0x420fb3`. The whole function returns early for run state 1 (`0x420f8a`, `0x420f99`), so a
closing context gets no frame time and keeps its old `+0x40`. The test runs `0x420faa` to
`0x420fca`.

## Start and stop

Category 7 is dispatched once per context: on its first tick, or at teardown if it never ticked.
A context closes when its run state becomes 1: by `0x41f9b0` on a handle (the current context
for null), or by the window asking to quit. At the end of that tick the loop unloads it and
`0x420e60` dispatches category 3, sets run state 2, drains its messages, dispatches category 4,
unbinds it and deletes it. When the last interactive context goes, `0x422030` sets the shutdown
event, and the loop exits at its next test.

Addresses: `0x420e20`, `0x420e2e`, `0x420e60`, `0x41f9a0`, `0x41f9b0`, `0x41fa44`, `0x41f6e0`, `0x421370`, `0x422030`, `0x423240`, `0x801364`
Confidence: verified
Proof: two
Test: none

`0x420e20` returns 0 when flag 1 is set; otherwise it sets it, stamps `+0x40` and dispatches 7
(`0x420e2e` to `0x420e45`). A census of stores to `+0x44` in the library finds three, at `0x420e31`,
`0x420fc1` and `0x421067`, plus the constructor's `0x422204`: none clears flag 1. In `0x420e60`:
`0x420e70` starts an unstarted context, `0x420e86` decrements `0x883ef0` for an interactive one and
calls `0x422030` at zero, then 3 (`0x420e9e`), `+0x2c = 2` (`0x420eaf`), `0x424ad0`, 4 (`0x420ec7`),
and the virtual destructor `0x423240`, slot 0 of `0x801364` (`0x420f3e`). `0x41f9b0` finds the
context by handle and stores 1 only over 0 (`0x41fa3d` to `0x41fa44`). `0x41f6e0` returns the
current context's handle. `0x421370` takes the context's cost off its slot's load.

## Shutdown event

`0x422030` sets the shutdown event `0x883f24` and the event of every slot, so a sleeping loop
wakes, finds the shutdown event set and leaves. Three paths call it: an accepted quit from the
window input, the last interactive context's teardown, and a window close the confirm-close
query allows, which it does only when no callback is installed.

Addresses: `0x422030`, `0x41f5d0`, `0x423ca0`, `0x883f24`
Confidence: verified
Proof: one
Test: none

`0x422030` sets `0x883f24` (`0x422035`), then, unless polling, sets `+0x20` of each slot in
`0x883f18` under `0x883ef4` (`0x42206f`). Its callers are `0x420d4e`, `0x420e8f`, `0x422660` (in
the unreached `0x422540`) and the jump at `0x41f5d0`, which `0x423ca0` reaches from the window
switch's close case. The client installs a confirm-close callback in its start handler, so a
window close takes the callback's path instead (`core/input#close-and-quit`).

## Scheduler heap

Each slot keeps its contexts in a binary min-heap ordered by due time, compared as
`(int32)(a − b) ≤ 0`, so due times may wrap. Taking the root moves the last element down,
choosing the right child when it is not later than the left. Timers use the same heap code and
comparator.

Addresses: `0x421430`, `0x422970`, `0x421860`, `0x421f40`, `0x421840`, `0x428ee0`, `0x801368`
Confidence: verified
Proof: one
Test: `core::heap_key_le_matches_binary`, `core::heap_pop_min_matches_binary`, `core::heap_resift_matches_binary`

`0x422970` subtracts the other node's `+0xc` from its own and returns `setle` (`0x422979`,
`0x422980`); it is the only slot of the node vtable `0x801368`. In `0x421430` the right child is
taken when `call [vtable]` on it returns 1 against the left (`0x4214ec`, `0x4214f2`), and the sift
stops when the moving element compares not later than the child (`0x421506`). `0x421860` removes an
element and re-inserts it after a key change, `0x421f40` removes at an index, `0x428ee0` inserts,
and `0x421840` refreshes a node's owner and index. The tests run `0x422970`, `0x421430` and
`0x421860` whole.

## Slots and threads

The scheduler's other machinery serves more than one thread or slot, which this build never
has: slot creation and release, load accounting and migration, thread records and teardown. Its
growable arrays round their capacity up to a multiple of a quantum: the largest power of two not
above the size for sizes below 64, else 64, and 1 for size 0.

Addresses: `0x421080`, `0x4211d0`, `0x421370`, `0x421570`, `0x422720`, `0x422850`, `0x422b50`, `0x422da0`, `0x422e90`, `0x422ff0`, `0x423030`, `0x423050`, `0x4230e0`, `0x423120`, `0x423140`, `0x422ac0`, `0x4229c0`, `0x4229e0`, `0x422a50`, `0x422c90`, `0x422d80`, `0x422fc0`, `0x4233d0`, `0x4234d0`, `0x420830`, `0x420980`, `0x421f00`, `0x423530`, `0x4235c0`, `0x423650`, `0x423670`, `0x4236d0`, `0x423740`, `0x4237a0`, `0x4237e0`
Confidence: inferred
Proof: none
Test: none

`0x421f00`, a thread entry that pumps until its event is set, is started only when polling. The
context destructor clears its lists through the helpers listed. The quantum loop is `x & (x − 1)`
repeated until zero, keeping the last nonzero value (`0x422ff3` to `0x423010`): the highest set bit.
The rest was read only for its calls and stores; the derivations of the multi-slot paths are not
kept.

## Static initializers

The scheduler's object files run 12 static initializers from the C runtime table. Each file
copies +∞ into a global and stores 2π and 1/2π as floats, as every object file using the shared
maths header does. The others build the context table `0xcf7700`, the slot list `0x883ee8`, the
thread array `0x883ecc`, the slot lock `0x883ef4`, and the events `0x883ec4` and `0x883f24`, each
registering a matching destructor with `_atexit`.

Addresses: `0x827348`, `0x41f580`, `0x41f590`, `0x4207e0`, `0x4207f0`, `0x420800`, `0x420810`, `0x420850`, `0x4208b0`, `0x4208c0`, `0x4209b0`, `0x4209c0`, `0x4209d0`, `0x4209e0`, `0x4209f0`, `0x420a00`, `0x420a10`, `0x420a20`, `0x420a50`, `0x420a60`, `0x420b00`, `0x420b10`, `0x420b30`, `0x420b40`, `0x420b60`, `0x420b70`, `0x420b80`, `0x420b90`, `0x420ba0`, `0x420bb0`, `0x420bc0`, `0x420bd0`, `0x422c40`, `0x422c50`, `0x422c60`, `0x422c70`
Confidence: verified
Proof: one
Test: none

The slots from `0x827348` through `0x827374` hold them in table order. A shape census over the
library matched each body: `mov eax,[+∞]; mov [g],eax; ret`, `fld [π]; fadd st,st; fstp [g];
ret`, `fld 1.0; fdiv [g2π]; fstp [g]; ret`, or `call init; jmp atexit-thunk`. `0x420850` builds
the table's lock and eight bucket locks and lists under the guard bit `0xcf76cc`; `0x4208c0`
unlinks every entry and destroys them. `0x420a20` makes `0x883ee8` an empty list. `0x420b70` and
`0x420bb0` create the two events as manual-reset, initially unset.

## Unreached functions

These have no call, jump or stored address outside unreached code: the non-interactive context
creator, a worker thread's registration, a second loop body and its exit, and an array teardown.

Addresses: `0x41f600`, `0x422500`, `0x422540`, `0x422710`, `0x421c30`
Confidence: verified
Proof: one
Test: none

The census is two scans over the whole image: every `e8`/`e9` whose target is one of them, and every
dword equal to one of their addresses at any byte alignment, outside the import tables. Both find
nothing outside this list; the control is `0x420c00`, whose address is the immediate of the
`mov ecx` at `0x421b5b` and the target of the call at `0x420bec`; both appear. `0x422540` repeats
the tick of `0x420c00` with its own calls of `0x428510`, `0x423920` and `0x420f70`, and nothing calls
it; `0x422500` only names and registers its thread and returns (`0x42253b`).
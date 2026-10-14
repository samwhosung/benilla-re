# core: scheduler

The data below the frame loop (`core/loop`): the slots that hold contexts, how a context is
inserted, balanced and destroyed, the context table, and the array and list helpers the event
library shares. A slot is an `EvtThread` of 0x38 bytes. It serves the loop threads that run it and
holds a binary min-heap of contexts ordered by due time. This build runs one loop thread, so there
is one slot, and the code that moves a context between slots finds no other slot to move it to.
The code is the event library's scheduler file and its neighbours (`0x41f550` to `0x429340`). The
context table is eight buckets by handle, each with a read-write lock and a list.

Roots: `0x41f550`, `0x41f5a0`, `0x41f5e0`, `0x41f9b0`, `0x420c00`, `0x422970`, `0x4229e0`, `0x422a50`, `0x423240`

## Slot acquire and release

`AcquireSlot` (`0x421080`) returns the index of the slot a loop thread serves. Under the slot lock
`0x883ef4` it takes the first empty table entry, else the entry with the fewest threads (`+0x0c`,
the lowest index on a tie). An empty entry gets a new slot: an auto-reset event, initially unset,
at `+0x20`, an empty heap with element offset 0x30, and the last place in the slot list. The
thread count rises by 1. `ReleaseSlot` (`0x4211d0`) lowers it. At 0 it clears the table entry,
empties the heap, frees the slot, then sets the run state of each removed context to 1 when it is
0 and tears it down with `0x420e60` on the calling thread, the last one removed first.

Addresses: `0x421080`, `0x4211d0`, `0x422ac0`, `0x421f40`, `0x422e90`, `0x659a70`, `0x883ee8`, `0x883ef4`, `0x883f10`, `0x883f14`, `0x883f18`
Confidence: verified
Proof: one
Test: none

The scan from `0x4210b0` starts with the pick equal to the slot count. An index becomes the pick
when it is the first or its thread count is below the pick's (`jae` at `0x4210c4` skips equal
counts), and the scan stops at a null entry (`0x4210ca`). `0x421100` calls `0x659a70` with both
stack arguments 0: its manual-reset and initial-state flags. The heap fields are four zero dwords
and 0x30 (`0x421109` to `0x421114`), and `0x421155` links the slot at the end of the list, before the sentinel.
`0x4211ac` adds the thread. Client fault: when the allocation fails, `esi` stays 0 (`0x4210f4`
jumps to `0x421124`), and the stores at `0x42116b` to `0x421177` write through it, to the null page. In `0x4211d0`, `dec [ebx+0x0c]` (`0x42120d`) gates the rest. The loop
at `0x421254` takes the root out with `0x421f40(heap, 0)` until the count at `+0x28` is 0 and
appends each context to a local array grown by `0x422e90`; `0x4212b6` to `0x4212d2` destroy and
free the slot. The array is walked from its end (`0x4212f4`): lock, run state 0 to 1 (`0x42130d`),
the current context set as the loop does, then `0x420e60` (`0x42133d`).

## Polling mode

`0x883f0c` is the polling flag, stored by `0x4219d0` from its second argument. The application init `0x402350` passes 0 (`0x4027f1`)
and nothing else writes it. Set, the loop sleeps through `0x43f050` instead of on the slot's event,
100 ms when no context is due, and takes the timeout result without waiting; `0x422030` sets only
the shutdown event; and `0x421cb0` starts one helper thread per processor at `0x421f00` while it
joins the loop threads. `0x421f00` takes a pointer to an event and, until a zero-timeout wait on
it succeeds, calls `0x43f050(100)`.

Addresses: `0x883f0c`, `0x4219d0`, `0x420c00`, `0x422030`, `0x421cb0`, `0x421f00`, `0x43f050`, `0x429d20`
Confidence: verified
Proof: one
Test: none

The reads of the flag are `0x420c9d`, `0x42203a`, `0x421cd3` and `0x42259b` (the unreached worker
loop); the one write is `0x4219de`. At `0x420ca6` an infinite wait becomes 100 (`0x420cab`),
`0x420cb2` calls `0x43f050` and `0x420cb7` loads `0x102`. `0x421ceb` takes the processor count from
`0x429d20` (`GetSystemInfo`, 1 for a zero count); the thread is made at `0x421d5d` with `0x421f00`
and a local manual-reset event; `0x421f14` calls `0x43f050` with 100.

## Load accounting

`0x832cc0` holds -1 at rest and counts the scheduler calls in flight: `AcquireSlot`,
`ReleaseSlot`, `0x421370`, `0x421570` and `0x4222a0` increment it on entry and decrement it on
exit, and the balancing in `0x421570` runs only when its own increment returns 0. A slot keeps a
cost sum (`+0x10`), a context count (`+0x18`) and a mean (`+0x14`, sum / count, 0 when empty).
`RemoveContextLoad` (`0x421370`) takes a context's cost `+0x50` off its slot and recomputes the
mean. To each other slot whose mean is nonzero and whose sum is at least this slot's plus its mean
it adds (that sum - this sum) / that mean to its credit `+0x1c`, capped at its context count.

Addresses: `0x421370`, `0x832cc0`, `0x65ad30`, `0x65ad40`, `0x883eec`, `0x4224d5`
Confidence: verified
Proof: one
Test: none

`0x65ad30` is `lock xadd` of +1 returning the new value, `0x65ad40` the same with -1; the dword
at `0x832cc0` is `ff ff ff ff` in the file. The calls sit at `0x42108c`, `0x4211c2`, `0x4211ee`,
`0x4212e8`, `0x421375`, `0x421424`, `0x4215c4`, `0x421779`, `0x4222b5` and `0x4224d5`, and `0x4215c9` skips the balancing
unless the result is 0. `0x4213a0` to `0x4213b8` hold the sum, count and mean. The walk starts at
`0x883eec`, skips this slot (`0x4213d9`) and a zero mean (`0x4213e2`), and `cmp eax,ebx; jb`
(`0x4213ed`) skips a slot whose sum is below this sum plus its mean. The division is at `0x4213f5` and the cap
at `0x421401` to `0x421408`.

## Context insert

`0x4222a0` places a new context. It picks the slot with the lowest cost sum (the first on a tie),
gives a context without a handle the next value of `0xcf76f8` (0 is skipped, and after a wrap any
value still in the table), files it at the end of bucket `handle & 7`, sets its due time to the
clock, inserts it in the slot's heap, sets the slot's event, and adds the cost `+0x50` to the
slot's sum and 1 to its count With no slot it destroys the context through its virtual destructor
and returns 0.

Addresses: `0x4222a0`, `0x422c90`, `0x420830`, `0x658890`, `0x658b50`, `0xcf76e0`, `0xcf76f8`, `0xcf76fc`, `0xcf7700`, `0xcf7760`, `0x421840`
Confidence: verified
Proof: one
Test: none

The slot's mean is recomputed as sum / count (`0x4224b0`). The bucket locks are `0xcf7700` and the
lists `0xcf7760`; a lookup locks with mode 0, an insert with mode 1. The slot scan is `0x4222c4` to `0x4222f6`; `jae` at `0x4222ea` keeps the first of equal sums. The
counter is incremented at `0x42231a` under the lock `0xcf76e0`; reaching 0 sets `0xcf76fc`
(`0x422322`) and repeats, and once it is set `0x42232e` checks the bucket (mode 0 at `0x42234c`,
the walk by `0x420830`, `+0x0c` compared at `0x422370`). The insert is mode 1 at `0x4223bc` and
`0x422c90` with mode 2 and no reference, which links before the list's end. The due time goes
through the heap node key and `0x421860` when it differs (`0x4223f0` to `0x422402`). The sift-up
is `0x422421` to `0x422473`, the event `0x422492`, the accounting `0x422497` to `0x4224b5`, and the
no-slot path `0x4224ba`.

## Reschedule and migrate

`RescheduleContext` (`0x421570`; slot, context, due, cost) stores the due time, re-sifting the
heap node when it changed, and the cost in `+0x54`, setting `+0x58` to whether it differs from
`+0x50` by at least `+0x50 >> 3`. When no other scheduler call is in flight and the slot's credit
or `+0x58` is set, it swaps `+0x50` for `+0x54` in the slot's sum and looks for a slot whose sum
plus this cost is below the current slot's. Found, the context moves there (accounts follow, a
nonzero credit drops by 1) and that slot's event is set. Else a set
`+0x58` clears with the mean recomputed, and the credit clears when `+0x50` is at most the mean. The
context enters a heap.

Addresses: `0x421570`, `0x421860`, `0x422720`, `0x422b50`, `0x659aa0`
Confidence: verified
Proof: one
Test: none

`0x421585` to `0x421598` store `due` through the node and call `0x421860`. `0x42159b` to `0x4215bc`
compute the flag (skipped when `+0x54` already equals the cost): `cmp edx,eax; sbb; inc`. `0x4215e4` to `0x4215f0` skip the block when credit
and flag are both 0, and `0x4215fd` to `0x421609` swap the cost. The scan `0x42160c` to `0x421646`
keeps the lowest `sum + cost`, a candidate needing to be lower (`jae` at `0x42163d`). Staying is
`0x42173f` to `0x421763`; moving is `0x42164a` to `0x421699` for the accounting and `0x4216a8` to
`0x421732` for the sift-in and `0x659aa0`. The same-slot insert is `0x42177e` to `0x421826`, room
made by `0x422b50` (`0x4217b9`). In this build no other slot exists, so the scan keeps the slot.

## Growable arrays

Growable arrays are four dwords: capacity, count, storage and quantum. A heap array adds a fifth,
the offset from an element pointer to its heap node. Capacity rounds up to a multiple of the
quantum: 64 for sizes of 64 and above, stored at `+0x0c`, and below 64 the highest power of two not
above the size, 1 for size 0, not stored. Growth reallocates, falling back to allocate, copy and
free. `0x422720(index, zero)` raises the count to index + 1, zeroing the new entries on request;
`0x422850(n)` sets the count to n and grows only when n exceeds both count and capacity;
`0x422b50(add, round)` raises the capacity for count + add and leaves the count.

Addresses: `0x422720`, `0x422850`, `0x422b50`, `0x883ecc`, `0x422da0`, `0x422e90`, `0x422ff0`, `0x423030`, `0x423050`, `0x4230e0`, `0x423120`, `0x423140`, `0x422960`, `0x422990`, `0x4229b0`, `0x4229c0`, `0x646320`
Confidence: verified
Proof: one
Test: none

The quantum loop is `lea ecx,[eax-1]; and ecx,eax` repeated while nonzero (`0x422760` to
`0x422767`), which clears low set bits until one remains. `0x422775` stores 64, and a stored
quantum is used for every later size (`0x42274a`). The round-up is
the `div` at `0x422785` with `ecx - edx` added. `0x4227ad` calls `0x646320` with flags 0x10, and
`0x4227c2` the allocator when that fails. `0x422da0` and `0x422e90` repeat `0x422b50` for the
thread array (tag `.PAVSThread@@`) and the context array (tag `.PAUEvtContext@@`);
`0x422ff0` and `0x4230e0` are the quantum rule, `0x423030` and `0x423120` the round-up, and
`0x423050` and `0x423140` set the capacity exactly, keeping the smaller of capacity and count.
`0x422960` and `0x4229b0` zero four dwords, `0x422990` zeroes four and stores its argument as the
fifth, and `0x4229c0` frees the thread array's storage.

## Intrusive lists

Event-library lists link their elements in place. A head is 12 bytes: the link's offset in the
element, then a sentinel link. A link is two dwords: a pointer to the previous link and the next
element, with bit 0 set on a next pointer that names the sentinel. An unlinked link is zeros.
Unlinking writes the next link's previous pointer and the previous link's next pointer. A helper
turns a link into the link of the element after it: the sentinel at the end, else the next element
plus the offset. `0x422c90(node, mode, reference)` unlinks the node, then links it after the
reference when mode is 1 and before it otherwise; no reference means the sentinel.

Addresses: `0x422c90`, `0x422d50`, `0x422d80`, `0x422f80`, `0x422f90`, `0x422fa0`, `0x422fb0`, `0x422fc0`, `0x420830`, `0x420980`, `0x425220`, `0x425250`, `0x425290`, `0x425310`, `0x425340`, `0x4237a0`
Confidence: verified
Proof: one
Test: none

`0x422f80` zeroes a link. `0x422f90` stores the sentinel's previous pointer as its own address and
its next as that address with bit 0 set; `0x422fa0` and `0x422fb0` are the same code. `0x4237a0` is the unlink: it writes `[next
link] = previous` and `[previous + 4] = next` (`0x4237c3`, `0x4237ca`), then zeroes the link. The
next link is the masked pointer when bit 0 is set or the pointer is null (`0x4237b3`), else the
pointer plus the distance from the previous link's element (`0x4237b8` to `0x4237c0`).
`0x422fc0`, `0x420980`, `0x425220`, `0x425290` and `0x425310` are that computation with the offset in
the argument, a negative one meaning derive it from the link and its owner; `0x425250` and `0x425340`
are the unlink. `0x420830` returns the link at `+4` of the element it is given and `0x422d80` its
argument, each the sentinel link for 0. In `0x422c90`, mode 1 is the `dec esi; je` at `0x422ceb`.

## Context fields

The fields of a context outside the loop's own: `+0x04` and `+0x08` are the context's link in its
table bucket (`+0x04` the previous link, `+0x08` the next context), `+0x0c` the handle the bucket
walk compares, `+0x208` the thread property array, `+0x20c` the name object, and `+0x210` bit 1 of
the constructor's third argument. The constructor stores 0 at `+0x28`, and nothing else in the
event library touches that word of a context. The array comes from `0x41db30` (`core/threads`) and
the name object from `0x437300` only when bit 0 of the third argument is set; the application init `0x402350` passes 0 (`0x402812`),
so `+0x20c` and `+0x210` are 0.

Addresses: `0x422090`, `0x41db30`, `0x437300`, `0x420e60`, `0x4221e9`, `0x422227`, `0x422230`, `0x422236`
Confidence: verified
Proof: one
Test: none

The constructor stores the array at `0x422227`, the name object at `0x422230` (0 unless `test bl,1`
at `0x4220b6` fell through) and `(argument >> 1) & 1` at `0x422236`. The bucket walks read `+0x08`
for the next context and compare `+0x0c` (`0x420f1c`, `0x420f21`). A scan of the instructions with
displacement `0x28` between `0x41f550` and `0x429340` finds the slot's heap count (`0x421243`,
`0x42145a`, `0x4216ad`, `0x4217a9`, `0x42241b`, `0x422ac3`, `0x422adf`), the timer record's fields
(`0x428662`, `0x428a99`, `0x428c74`, `0x428e47`), an unreached store (`0x426d64`), and the
constructor's store (`0x4221e9`).

## Context destruction

The deleting destructor `0x423240` frees a context's parts and then, when bit 0 of its argument is
set, the context. In order: every handler record of the 29 lists (`0x423670` then free), the
messages (`0x4236d0`), the held keys (`0x423740`), the timer records from index 1 (each with its
heap node removed by `0x4233d0`), the property array (`0x41db70`), the name object (`0x437410`),
the timer heap and arrays, the held-key and message list heads (`0x4235c0` on `+0x1c8`, then `0x423530` on `+0x1b8`), the 29 handler list heads through `0x4044b0`, its
own heap node (removed from a heap if still in one), the lock, and the context table link
(`0x4237a0`). The vtables it installs are in `Context vtables`.

Addresses: `0x423240`, `0x423670`, `0x4236d0`, `0x423740`, `0x4233d0`, `0x4234d0`, `0x423510`, `0x423530`, `0x4235c0`, `0x423650`, `0x4237e0`, `0x4044b0`, `0x41db70`, `0x437410`, `0x6579a0`
Confidence: verified
Proof: one
Test: none

`0x423248` stores `0x801364`; the handler loop is `0x423260` to `0x423292` (29 lists from `+0x64`),
the message loop `0x423294`, the key loop `0x4232c0`, the timer loop `0x4232e9` (index from 1,
vtable `0x801374` stored at `0x423308` before `0x4233d0`). `0x423343` to `0x42336e` clear the
rest, `0x423379` to `0x423386` call `0x4044b0` with `0x4237e0`, element size 12 and count 29, and
`0x42338e` stores `0x801374` in the node at `+0x30` before removing it. `0x4233a4` stores
`0x801370`, `0x4233aa` unlinks, and `0x4233af` frees on bit 0. `0x4233d0` removes a heap node: the
last element replaces it and sifts down, the comparator called through the node's vtable.

## Context vtables

Five single-slot vtables lie in a row from `0x801364`: the context's deleting destructor
`0x423240`, the heap-node comparator `0x422970`, then `0x4229e0`, `0x422a50` and `0x40a2d7`. The
constructor stores `0x80136c` at `+0x00` and `0x801364` once built; the destructor stores
`0x801364` and ends with `0x801370`. `0x4229e0` and `0x422a50` are the deleting destructors of the
two base layers and differ only in the free call's displacement: each stores `0x801370`, unlinks
the link at `+4`, frees `this` when bit 0 of the argument is set and returns it. `0x40a2d7` ends
the process with runtime error 25, a pure virtual call.

Addresses: `0x801364`, `0x801368`, `0x80136c`, `0x801370`, `0x801374`, `0x4229e0`, `0x422a50`, `0x40a2d7`, `0x422132`, `0x4221e3`
Confidence: verified
Proof: one
Test: none

The constructor stores at `0x422132` and `0x4221e3` and the heap node's `0x801368` at `0x422146`.
The vtable dwords are read from the file. A scan for `0x4229e0` and `0x422a50` as a call or jump
target and as a dword in the image finds only the vtable slots `0x80136c` and `0x801370`, with
`0x420c00` as the control. The constructor between `0x422132` and `0x4221e3` and the destructor from
`0x4233a4` make no virtual call, so nothing runs through those slots.

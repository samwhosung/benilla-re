# core: process entry and threads

How the process reaches the frame loop, and the thread layer the loop leans on. The entry point
runs two start-up calls and the C runtime, which calls `WinMain`; the application body brings the
clock, the event library and the interface up, runs the loop on the main thread, and takes them
down. The thread layer is `OsCall.cpp`: a record per registered thread under a TLS slot, an
optional name object per context, and the thread property array of `Prop.cpp`, whose element 0
holds the current context. Core owns none of those files, so the entries give the loop's side of
each call: its order and arguments. This build starts the library with one thread, creates one
context, and never creates a name object.

Roots: `0x41f550`, `0x41f5a0`, `0x41f5c0`, `0x41f5e0`, `0x420be0`, `0x420c00`, `0x41f6e0`

## Process entry

The PE entry point `0x401000` calls `0x645010`, then jumps to the C runtime start-up `0x4098c0`.
`0x645010` starts Storm: `0x65aa20` initializes five critical sections once, and `0x646280` runs
`SMemInitialize` once. The start-up reads the OS version, runs the runtime's set-up, takes the
command line and the startup info, and calls `WinMain(GetModuleHandleA(0), 0, command line, show)`
at `0x40999b`, `show` being the startup info's `wShowWindow` when its flag bit 0 is set, else 10,
and exits with the return value. `WinMain` (`0x404100`, `ret 0x10`) clears the x87 status, sets
the control word with `__control87(0x9001f, -1)`, calls `0x42a1c0`, and then the application body
`0x402120`.

Addresses: `0x401000`, `0x645010`, `0x65aa20`, `0x646280`, `0x4098c0`, `0x404100`, `0x409bf2`, `0x409c06`, `0x42a1c0`, `0x402120`
Confidence: verified
Proof: one
Test: none

The entry is `call 0x645010; jmp 0x4098c0` (ten bytes). `0x645010` is `call 0x65aa20; jmp 0x646280`.
`0x65aa20` tests a guard dword, calls `InitializeCriticalSection` on four adjacent 24-byte objects
and on a fifth, and sets the guard. `0x646280` tests its own guard, formats the string
`SMemInitialize()` into a 4 KB buffer and passes it to `OutputDebugStringA` through `0x667440`, and
constructs the allocator object. The `WinMain` call's arguments are pushed at `0x40998e` to
`0x40999a`, with `GetModuleHandleA` at `0x409994`. `0x404100` is `call __clearfp; push -1; push 0x9001f; call
__control87; call 0x42a1c0; call 0x402120; ret 0x10`.

## Process access list

`0x42a1c0` replaces the access list of the running process. It builds a list with a deny entry for
Everyone, mask 0xfa, and an allow entry for the current user, mask 0x100701, and sets it as the
process's list, marked protected from inheritance. It returns 1 when every step succeeds. `WinMain`
ignores the result.

Addresses: `0x42a1c0`
Confidence: verified
Proof: one
Test: none

The Everyone SID comes from `AllocateAndInitializeSid` with the authority bytes `00 00 00 00 00 01`
and one zero sub-authority (`0x42a1dd`, `0x42a20a`). The user SID is the first field of the
`TOKEN_USER` that `GetTokenInformation` returns for the process token (`OpenProcessToken` with
access 8, `0x42a21f`; the buffer is a stack block of up to 0x400 bytes, `0x42a242`). `InitializeAcl`
takes 0x200 bytes and revision 2 (`0x42a273` to `0x42a281`); `AddAccessDeniedAce` (`0x42a28f`) and
`AddAccessAllowedAce` (`0x42a2aa`) follow; `SetSecurityInfo` takes object type 6 and flags
0x80000004 (`0x42a2cf`). A zero return sets the result (`0x42a2e1`). The handle and the SID are
released at `0x42a2ec` and `0x42a2fa`.

## Application body

`0x402120` runs the start-up steps, creates the time manager, then the application init `0x402350`.
When that returns nonzero it runs the event loop `0x41f5c0` and then the teardown `0x403bc0`. It
ends with `0x64cbf0` and returns 0. The init calls the library's `0x41f550` and creates the
interactive context; the teardown destroys the time manager before it frees the library.

Addresses: `0x402120`, `0x402350`, `0x403bc0`, `0x429780`, `0x429790`, `0x41f550`, `0x41f5a0`, `0x41f5c0`, `0x41f5e0`, `0x404080`, `0x404090`
Confidence: verified
Proof: one
Test: none

The order in `0x402120`: Storm's `0x64cbe0` and `0x64df70`, two calls to `0x64ec80` with ids 10 and
11 and a 4-byte value 1 (`0x402141`, `0x402151`), `0x429a50`, `0x402210`, the time manager
`0x429780` (`0x402160`), a read of the value `SendErrorLogs` by `0x64c650` (`0x402178`) saved as 1
by `0x64c9a0` when the read fails (`0x402190`), the application name `World of WarCraft (build
5875)` passed to `0x64e870` (`0x402195`), then `0x404080`, which returns 1 (`0x4021d2`). Init is
called at `0x4021db`; its result gates the loop (`0x4021e4`) and teardown (`0x4021e9`). In `0x403bc0`, `0x429790` (`0x403bce`)
comes before `0x41f5a0` (`0x403bd3`). The init's calls are `0x41f550` with `ecx` 1 and `edx` 0
(`0x4027f8`) and `0x41f5e0` (`0x402825`).

## Engine thread records

`0x437060(name)` registers the calling thread. Under the thread lock it counts the registration
and allocates the TLS slot for the first; a record already in the slot is unlinked and freed. The
new 0x189c-byte record holds the thread id at `+0x08`, a duplicate of the thread handle at `+0x0c`
and the text `name [id]` at `+0x181c`, and goes in the TLS slot and at the end of the record list.
`0x4371e0` undoes it: it empties the slot, zeroes the bound object's `+0x08`, closes the handle,
unlinks and frees the record, and frees the slot when the count reaches 0.

Addresses: `0x437060`, `0x4371e0`, `0x436d60`, `0x436d70`, `0x436d80`, `0x436db0`, `0x64bbd0`, `0x884ecc`, `0x884ef0`, `0x884ef8`, `0x884f00`
Confidence: verified
Proof: one
Test: none

The loop registers as `Engine <thread id in hex>` after the start gate opens and unregisters after
its last iteration. Client fault: when the record allocation at `0x437112` fails, `esi` is 0 and
`0x437145` stores the thread id to `[esi+0x08]`, a write to the null page. The lock is the dword `0x884ecc`: while it is null, `0x437071` to `0x437082` spin on
`InterlockedExchange` of a flag, create a critical section in static storage, store its address
there and clear the flag. The count is bumped at `0x4370b8` to `0x4370c2`, with `TlsAlloc` (`0x436d60`) at
`0x4370c9`. The allocation is `0x6462e0(0x189c, source file, line 0xe5, 8)`; the duplicate is
`DuplicateHandle(process, GetCurrentThread(), process, +0x0c, 0, 0, 2)` (`0x43712f` to `0x437148`);
the name is `SStrPrintf(+0x181c, 0x80, "%s [%u]", name, id)` (`0x437163` to `0x437174`). `0x436d80`
is `TlsGetValue` that raises error 0x57 when `GetLastError` is set. The unregister path is
`0x4371e0` to `0x4372cc`. The loop's name is `Engine %x` (`0x420c32` to `0x420c4e`).

## Context binding

A context's `+0x20c` holds a name object or 0. `0x4374d0(object)` ties the calling thread's record
to it: under the thread lock it stores the record in the object's `+0x08` and the object in the
record's `+0x14`. `0x437560(object)` clears both. Both return at once for a null object, with no
registration, or with no record for the thread. The loop passes `+0x20c` every iteration and at
teardown. The constructor builds the object only when bit 0 of its third argument is set, named
`Context: interactive = %u, idleTime = %u`; the application init `0x402350` passes 0 (`0x402812`), so these calls do nothing.
`0x437300` builds it; `0x437410` frees it.

Addresses: `0x4374d0`, `0x437560`, `0x437300`, `0x437410`, `0x422090`, `0x884f0c`
Confidence: verified
Proof: one
Test: none

`0x437300` builds the object at 0x45a8 bytes, name at `+0x4528`, `+0x0c` set to -1, at the end of the
list `0x884f0c`. `0x4374d9` to `0x4374f0` return for a zero count or object or a missing record; `0x437544` and
`0x437547` are the two stores. `0x437591` to `0x43759d` let the unbind proceed when either side is
set, and `0x4375f4` to `0x4375fb` clear both. The constructor's test is `test bl,1` at `0x4220b6`
and the format push at `0x4220c7`; `0x4220e6` calls `0x437300`. `0x437300` clears the first 8 bytes of
each of 1024 entries of 16 bytes from `+0x18` (`0x437329` to `0x43733b`). `0x437410` clears the record's `+0x14` when
the object's `+0x08` is set (`0x437473`).

## Thread properties

Each thread that runs a context carries a property array of 19 dwords. The TLS slot `0x883e68` holds
its address. `0x41db50(array)` stores the pointer in the slot, null clearing it; `0x41db60` returns
it; `0x41db90(i)` returns element i of the calling thread's array, 0 without one; `0x41dbb0(i,
value)` stores element i. `0x41db30` allocates an array and `0x41db70` frees one. The loop makes a
context current by storing its array (`+0x208`) in the slot and its handle in element 0, and clears
the slot after the tick; `EventGetCurrentContext` (`0x41f6e0`) returns element 0. `0x41dae0` creates
the TLS slot and `0x41db00` frees it.

Addresses: `0x41db30`, `0x41db50`, `0x41db60`, `0x41db70`, `0x41db90`, `0x41dbb0`, `0x41dae0`, `0x41db00`, `0x41f6e0`, `0x883e68`
Confidence: verified
Proof: one
Test: none

`0x41db30` is `SMemAlloc(0x4c, source file, line 0x2f, 8)`. `0x41db50` moves `ecx` to `edx`, loads the
slot index and jumps to `0x436db0`, which is `TlsSetValue`. `0x41db90` reads the array through
`0x436d80` and indexes it by `ecx * 4` (`0x41dba2`). `0x41dae0` is `TlsAlloc`, store, and
`TlsSetValue(index, 0)`. The context constructor stores `0x41db30`'s result at `+0x208`
(`0x422227`); the loop stores the array and handle at `0x420cd3` to `0x420ce3` and clears at
`0x420dea`.

## Host calls

The event library reaches the OS through thin wrappers. `0x6579b0`, `0x6579c0`, `0x6579a0` and
`0x657990` call `EnterCriticalSection`, `LeaveCriticalSection`, `DeleteCriticalSection` and
`InitializeCriticalSection` on `ecx`. `0x659a70` makes an event at `*ecx`, `CreateEventA(0, [esp+4],
[esp+8], 0)`; `0x659930` zeroes `*ecx`; `0x659aa0`, `0x659ab0` and `0x6599c0(timeout)` call
`SetEvent`, `ResetEvent` and `WaitForSingleObject` on `*ecx`; `0x659960` closes the handle and
zeroes it. `0x65ad30` and `0x65ad40` add and subtract 1 atomically and return the new value;
`0x65ad70` exchanges. `0x64bbd0` is `GetCurrentThreadId`, `0x64bbf0` and `0x64bc00` get and set the
priority of the current thread, `0x429d50` is `Sleep`, and `0x43f050(ms)` runs the network object's
wait when one exists and `Sleep` otherwise.

Addresses: `0x6579a0`, `0x6579b0`, `0x6579c0`, `0x657990`, `0x659a70`, `0x659930`, `0x659aa0`, `0x659ab0`, `0x6599c0`, `0x659960`, `0x65ad30`, `0x65ad40`, `0x65ad70`, `0x64bbd0`, `0x64bbf0`, `0x64bc00`, `0x429d50`, `0x43f050`, `0x439120`
Confidence: verified
Proof: one
Test: none

Each is a few instructions that call the import of that name. `0x659a70` pushes `0, [ebp+0xc],
[ebp+8], 0` (`0x659a81` to `0x659a87`). `0x65ad30` is `mov eax,1; lock xadd [ecx],eax; inc eax` and
`0x65ad40` the same with -1 and `dec`; `0x65ad70` is `lock xchg [ecx],eax` with `edx` moved in.
`0x43f050` tests a global network object pointer, calling `0x439120` with it as `this` when set
(`0x43f05c`) and `Sleep` otherwise (`0x43f062`). In `0x439120` the object's `+0x0c` flag selects
`Sleep` too (`0x439123`).

## Allocator and error calls

`SMemAlloc(size, name, line, flags)` (`0x6462e0`) asks the allocator object for `size` bytes tagged
with a name and line, and zero-fills them when bit 3 of the flags is set. `SMemFree(pointer, name,
line, flags)` (`0x646430`) frees through the allocator and returns 1. `SStrPrintf(buffer, size,
format, ...)` (`0x64a7f0`) raises error 0x57 and returns 0 for a null buffer or format, else
formats through `0x64a820`. `SErrSetLastError(code)` (`0x64e850`) stores the code in a global and
calls `SetLastError`. `0x658890(mode)` and `0x658b50(mode)` enter and leave the read-write lock at
`ecx` through `0x6588a0` and `0x658b60` with the mode in `edx`.

Addresses: `0x6462e0`, `0x646430`, `0x645640`, `0x645790`, `0x64a7f0`, `0x64a820`, `0x64e850`, `0x658890`, `0x658b50`, `0x6588a0`, `0x658b60`
Confidence: verified
Proof: one
Test: none

`0x6462f0` loads the allocator object and `0x6462f5` calls its allocate (`0x645640`) with size,
name and line; `0x6462fc` tests flag bit 3 and `rep stos` clears the block (`0x646308` to
`0x646313`). `0x646444` calls the allocator's free (`0x645790`). `0x64a7f6` to `0x64a801` test the
buffer and format and push 0x57; `0x64a814` calls `0x64a820` with the argument list at `ebp+0x14`.
`0x64e857` stores the code before the `SetLastError` call. `0x658896` and `0x658b56` load the
mode from the stack into `edx` and call the lock bodies.

## String conversion table

`0x64a670` runs `0x64a680` once, setting a guard. `0x64a680` fills a table of 20 rows of 10
doubles, row r (0 to 19) column k (0 to 9) holding `k * 10^-(r+1)`, with the power taken by the C
runtime's `pow` intrinsic `0x73f940` on 10.0 and the negative exponent. Storm's string-to-double
conversion reads it for fractional digits.

Addresses: `0x64a670`, `0x64a680`, `0x73f940`, `0xc52560`, `0xc52ba0`
Confidence: verified
Proof: one
Test: none

`0x64a68c` sets the destination to `0xc52560`. The exponent counts down from -1 and the loop ends
when it passes -20 (`0x64a6c7`, `jg`). `fild` of the exponent and `fld` of the double 10.0 feed
`0x73f940` (`0x64a694` to `0x64a6a2`); the inner loop `fild k; fmul st,st(1); fstp qword` runs 10
times per row (`0x64a6b0` to `0x64a6c2`). The table ends at the guard `0xc52ba0`, 1600 bytes on.

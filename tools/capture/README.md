# tools/capture: measuring the running client

Two rigs that read the 1.12.1 client from inside its own process while it runs, and a launcher
that starts the client under Wine with one of them, with an OpenGL tracer, or with nothing.

- The **player rig** samples the active player's movement state (position, facing, flags, fall,
  collision size) at a fixed rate.
- The **hook rig** logs every call of the client functions you name: the object in `ecx`, the
  stack arguments, the result in `eax`, `edx` and `st0`, and for a call on the active player's
  mover, its position and flags before and after.

A capture is an observation (`METHOD.md`, The binary decides). It can refute a reading or choose
between two, and it shows what a run did; the mechanism still comes from the bytes. Nothing a
rig writes is committed (`CONTRIBUTING.md`, Setup).

## Files

| file | what it is |
|---|---|
| `rig.h`, `rig.c` | the shared core: the build check, fault-safe reads, output files, the clock, the active player lookup |
| `player.c` | the player rig |
| `hook.c` | the hook rig |
| `imm32_def.py` | writes the forwarders that make a rig a complete `imm32.dll` |
| `build.sh` | builds everything into `build/` |
| `launch.sh` | runs the client under Wine, with a rig or without; runs the tests |
| `forwardtest.c` | checks a built rig loads in `imm32`'s place, forwards, and refuses any process but the client |
| `hooktest.c` | runs the hook mechanism against functions of its own |
| `probe.c` | tells whether another process can read the running client |

## How a rig gets into the client

`WoW.exe` imports nine functions from `IMM32.dll` (its import table: `ImmAssociateContext`,
`ImmAssociateContextEx`, `ImmGetCandidateListA`, `ImmGetCompositionStringA`, `ImmGetContext`,
`ImmGetConversionStatus`, `ImmNotifyIME`, `ImmReleaseContext`, `ImmSetConversionStatus`), so the
loader maps `imm32.dll` into the process before the client runs. A rig is a DLL named
`imm32.dll` placed beside `WoW.exe`: its `DllMain` starts one thread that runs the rig, and every
export of the real `imm32.dll` is forwarded to a copy of it named `imm32_orig.dll`, placed beside
it. The forwarding covers the whole export list, not only the client's nine imports, because the
system's `user32` loads other `imm32` functions on demand and a missing export ends the process.
`imm32_def.py` writes that list from the real DLL's export directory, so it matches whatever Wine
or Windows supplies it.

Under Wine two settings make the loader take the rig instead of its own `imm32`: the DLL override
`imm32=n,b` (native first), and `imm32` taken out of the prefix's
`HKLM\System\CurrentControlSet\Control\Session Manager\KnownDLLs`, which loads from the system
directory regardless of overrides. `launch.sh` sets both; `forwardtest` fails with either one
missing. `launch.sh restore` puts the KnownDLLs value back. Elsewhere, run `forwardtest` from a
directory holding the rig and `imm32_orig.dll` to see whether the system takes the local copy.

A rig touches no file of the client. It acts only when the image at `0x400000` is `WoW.exe`
1.12.1.5875 by its PE header (`TimeDateStamp` `0x4510b6db`, `SizeOfImage` `0x906000`; the exe has
no relocations, so it always loads there), and otherwise logs why and stops. Every read of client
memory goes through `ReadProcessMemory` on the rig's own process, which fails instead of faulting
when the game frees a page under it.

## Building

```
tools/capture/build.sh
```

It needs the 32-bit MinGW-w64 compiler `i686-w64-mingw32-gcc` (`brew install mingw-w64`,
`sudo apt install gcc-mingw-w64-i686`), Python 3, and the 32-bit `imm32.dll` of the system the
client runs on: `IMM32_DLL=<path>`, else the script looks in `lib/wine/i386-windows/` beside the
`wine` it finds (`WINE`, else the `PATH`). On Windows it is `C:\Windows\SysWOW64\imm32.dll`.
Everything lands in `tools/capture/build/`, which git ignores; the script fails unless each rig
forwards every export the generated `.def` lists.

## Testing a build

```
WINE=<wine loader> WINEPREFIX=<prefix> tools/capture/launch.sh test
```

Without the client, in the Wine the client runs in:

- `forwardtest`, once per rig, imports `imm32` as the client does and passes when the `imm32.dll`
  loaded is the rig, when the client's nine imports and `ImmSetCompositionFontW` resolve into
  `imm32_orig.dll`, and when the rig's log, written during this run, says the build check
  refused the process.
- `hooktest` runs a test build of the hook rig, which skips the build check, against three of its
  own functions, and passes when `calls.csv` holds each call's arguments and result, the four
  calls of a recursive function in the right order with their own return addresses, and the
  `st0` of a function that returns a float; and when that float reaches the caller and the x87
  control word `0x023f` is unchanged, ignoring reserved bit 6.

The hook rig's log also says whether the system carries the x87 registers through an exception.
Some translation layers do not (hooktest shows it), and the rig is built for that: see The hook
rig.

## Running the client

```
WOW_DIR=<install> WINE=<wine loader> WINEPREFIX=<prefix> tools/capture/launch.sh player
```

| setting | meaning | default |
|---|---|---|
| `WOW_DIR` | the client's install directory | the repo's `WoW/` |
| `WOW_RUN` | what to run there: `WoW.exe`, or the loader your setup starts it with | `WoW.exe` |
| `WINE` | the Wine loader | `wine` |
| `WINESERVER` | its wineserver | beside `WINE`, else `wineserver` |
| `WINEPREFIX` | the prefix, read by Wine | Wine's own |
| `CAPTURE_OUT` | where the rig writes, on the host | `tools/capture/out/` (ignored by git) |
| `APITRACE_DLL` | for `trace`: apitrace's 32-bit Windows `opengl32.dll` | none |

The modes are `plain` (nothing added), `player`, `hook`, `trace`, `test` and `restore`. For a
rig, the launcher copies it and `imm32_orig.dll` into `WOW_DIR`, refusing to overwrite a file of
that name that is not one of its own, passes `CAPTURE_OUT` to the rig as `BENILLA_CAPTURE_DIR`,
runs the client, waits until every process of the prefix exits (`wineserver -w`, so a loader that
returns at once is fine), and removes both files. The rig writes `<rig>.log` beside its output;
check it first: it names the build check, the player it found, what it armed and how much it
wrote.

## The player rig

`player.csv` holds one row per sample at `BENILLA_CAPTURE_HZ` (default 60), from the rig's own
thread. The game thread writes the fields while the rig reads them, so a row is a sample of a
live struct, not one frame's state: two fields of a row can come from adjacent frames. To see
what one call decided, use the hook rig.

The rig finds the active player the way the client finds an object, by reads alone, and never
calls client code from its thread. It reads the manager at `[0xb41414]` (null outside the world)
and the active player's GUID at its `+0xc0`, which `0x468550` returns
([objectlayer/manager](../../systems/objectlayer/manager.md#accessors-and-enumeration)); walks the
live table as `0x464890` does: the bucket `[+0x1c] + 12·(lo & [+0x24])`, the chain's link offset
in the bucket's first dword and its first object in the third, matching `+0x18` and `+0x30` with
the low GUID dword and `+0x34` with the high
([objectlayer/manager](../../systems/objectlayer/manager.md#finding-an-object)); and takes the
unit's CMovement from `+0x118`, requiring it to equal the unit plus `0x9a8`
([objectlayer/units](../../systems/objectlayer/units.md#the-unit-object)). It looks again every
half second, so a teleport or relog that builds a new player object is followed.

| column | CMovement field |
|---|---|
| `t_ms` | milliseconds since the rig loaded |
| `guid` | the active player's GUID, hex |
| `x`, `y`, `z` | `+0x10` position, in transport space while a transport GUID is set |
| `facing`, `pitch` | `+0x1c`, `+0x20`, radians |
| `transport` | `+0x38` transport GUID, hex, 0 for none |
| `flags` | `+0x40` movement flags, hex |
| `fall_ms`, `fall_z`, `fall_speed` | `+0x78` ms on the fall arc, `+0x7c` its start height, `+0xa0` its launch speed (down-positive) |
| `speed` | `+0x84` current speed |
| `radius`, `height`, `scale` | `+0xb0`, `+0xb4`, `+0xb8` collision radius, height and scale ratio |

Each field's meaning and derivation is its row in `symbols/fields.tsv`
([collision/kinematics](../../systems/collision/kinematics.md#anchors-and-elapsed-time),
[collision/falling](../../systems/collision/falling.md#arc-fields-and-constants),
[collision/mover](../../systems/collision/mover.md#collision-size)). `fall_speed` is the speed the
arc started with, constant through a fall, not the current vertical speed.

## The hook rig

```
BENILLA_CAPTURE_HOOKS=0x636100:4 BENILLA_CAPTURE_FILTER=mover tools/capture/launch.sh hook
```

`BENILLA_CAPTURE_HOOKS` lists up to 16 functions as `address:args`, comma-separated, where args
is how many stack dwords to log (0 to 8). The rig arms them once the build check passes. Each
address must be in `.text` and begin with `push ebp` (`0x55`), which the rig checks; that it is a
function's entry is the caller's job: take it from `symbols/functions.tsv`.
`BENILLA_CAPTURE_FILTER=mover` keeps only calls whose `ecx` is the active player's CMovement
(found as the player rig finds it) and adds that mover's position and flags before and after.

How a call is caught. The rig writes `int3` over the function's first byte. At the breakpoint a
vectored exception handler records the call, replaces its return address with a stub's, performs
the `push ebp` the breakpoint replaced, and resumes at the second instruction; the breakpoint
stays for the next call. The function returns into the stub without any exception: the stub
copies `st0` when the register is in use, records `eax` and `edx`, and jumps to the real return
address with every register, the flags and the x87 environment as the function left them. Each
thread keeps its own stack of open calls, so nested and recursive calls pair correctly. Neither
the handler nor the stub uses the x87 or writes the file; a finished call goes into a ring of
8192 that the rig's thread writes out every 100 ms, and the log counts any call dropped from a
full ring.

The return takes no exception because some translation layers lose every x87 register across
one, and a function's float result is live at its return. At a compiled function's entry the x87
stack is empty by the calling convention, so the entry breakpoint loses nothing there; a function
that takes its arguments on the x87 stack, as some C runtime helpers do, must not be hooked on
such a system. The log says which kind of system the rig runs on.

Every hooked call costs an exception and a trip through the handler, so a function called many
times a frame slows the client and changes its timing; hook the narrowest function that answers
the question.

`calls.csv` holds one row per finished call, written when it returns, so an inner call's row
comes before its caller's; `seq` is the order calls began.

| column | value |
|---|---|
| `seq` | the call's number, in the order calls entered |
| `t_in_ms`, `t_out_ms` | entry and return, milliseconds since the rig loaded |
| `thread` | the calling thread's id |
| `function` | the hooked address |
| `ecx` | `ecx` at entry: `this` for a `__thiscall` |
| `return_to` | the caller's return address |
| `eax`, `edx` | at return, hex |
| `st0` | at return when the register is in use, as a decimal; empty otherwise |
| `a0` to `a7` | the stack dwords at entry, hex; a float argument is its bits, a pointer is the pointer |
| `pre_x` to `post_flags` | with the mover filter, the mover's `+0x10` position and `+0x40` flags at entry and return |

For example `0x636100` is `CMovement::ProbeStepUp(dir, nx, ny, nz)`, `ret 0x10`, which returns 0
when a sweep fails, 1 when blocked and 2 when a step over the face is possible
([collision/walking](../../systems/collision/walking.md#step-up-probe)). With
`0x636100:4`, `a0` is the pointer to the direction and `a1` to `a3` the face normal's bits; for
a row read with `csv.DictReader`:

```python
import struct
nx, ny, nz = (struct.unpack("<f", bytes.fromhex(row[k])[::-1])[0] for k in ("a1", "a2", "a3"))
```

## The OpenGL trace

`launch.sh trace` puts apitrace's `opengl32.dll` (`APITRACE_DLL`) beside the client with the
override `opengl32=n,b`, and after the client exits moves every `.trace` written during the run
into `CAPTURE_OUT`. The client draws through the API the CVar `gxApi` names, `direct3d` by default
(`tables/cvars.tsv`). Under Wine a Direct3D client's calls reach `opengl32` only as Wine's
translation of them, so such a trace is read through that layer (`METHOD.md`, Checklist:
observations); `SET gxApi "opengl"` in `WTF/Config.wtf` makes the trace the client's own OpenGL
calls. Quit the client from its menu: a killed process leaves no finished trace.

## Reading from outside

`probe.exe`, run in the client's Wine prefix or Windows login while it runs, opens the process
named `WoW.exe` (or its argument) with read rights and reads the first bytes of the image. `MZ`
means an ordinary outside reader works and a rig is not needed for reads; a denied open or read
means a layer around the client refuses other processes, and the rigs, which read from inside,
are the way in.

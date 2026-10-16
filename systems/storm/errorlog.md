# storm: error logs and crash dumps

When Storm reports a fatal error it writes a text log and a crash dump. The client gives the log a
title line and a header callback that adds its own state, and, while the registry value
`SendErrorLogs` is set, registers a callback that starts `WowError.exe` on the log. The log writer
and the dump writer are Storm's code; the header and the launcher are the client's, in
`Client.cpp`.

Roots: `0x64e600`, `0x64e870`, `0x64e8a0`, `0x64e8d0`, `0x66c9c0`

## Client registration

`ClientMain` reads `SendErrorLogs` under `World of Warcraft\Client`, and when the value is missing
writes it as 1. It sets the log title to `World of WarCraft (build 5875)` and the header callback
to `0x403ca0`, and, when the value is not 0, adds `0x404000` to Storm's error callbacks. That
callback takes the path of the last log written and runs `WowError.exe` with the command line
`WowError.exe <path>`; it ignores its five arguments and returns 1, and with no log written it runs
nothing.

Addresses: `0x402178`, `0x402190`, `0x40219a`, `0x4021a4`, `0x4021b6`, `0x64e870`, `0x64e8a0`, `0x64e600`, `0x64e6c9`, `0x404000`, `0x404015`, `0x404040`, `0x404077`, `0x64e8d0`, `0x64ba90`
Confidence: verified
Proof: one
Test: none

`0x402178` calls `0x64c650` with the key, the value name and 0; a zero answer makes `0x402190` call
`0x64c9a0` with 1. `0x40219a` passes the title to `0x64e870`, which copies at most 0x80 bytes to
`0xc53d48` under Storm's error lock; `0x4021a4` passes `0x403ca0` to `0x64e8a0`, which stores it in
`0xc53dc8`; `0x4021b6` calls `0x64e600`, which links a 12-byte record onto the list `0xc53d00` and
stores the callback in its `+0x8` (`0x64e6c9`). `0x404015` calls `0x64e8d0`, which copies `0xc53dd0`
out under the lock and answers 0 when it is empty. `0x404040` formats `%s %s` and `0x404077` calls
`0x64ba90`, the client's wrapper of `CreateProcessW`.

## The log file

`0x64daa0` creates a `.txt` log named from the local time, writes the title, then the error text.
A header callback, when set, gets a 5200-byte buffer with its first byte 0, and any text it leaves
is written. It records the file's path for
`0x64e8d0` and returns the file. The client's header is `WoWBuild: 5875`; then, when the object
manager is valid and the active player can be read, `Local Zone: <area>`, a line
`<label>: <name>, <guid>, (<x>,<y>,<z>)` for each of `Local Player`, `Attacking Target`,
`Current Object Track`, `Interact Target`, `Locked Target`, `Last Enemy Target` and `Cursor Item`
that exists, and, with any add-on listed, `Add Ons: ` and each loaded add-on's name followed by a
space.

Addresses: `0x64daa0`, `0x64dabb`, `0x64dad8`, `0x64db06`, `0x64db0f`, `0x64db1b`, `0x64db32`, `0x64db4a`, `0x64db60`, `0xc53dc8`, `0xc53dd0`, `0x403ca0`, `0x403cb8`, `0x403cc2`, `0x403d14`, `0x403e54`, `0x403e83`, `0x403ed0`, `0x403f70`, `0x403fc6`
Confidence: verified
Proof: one
Test: none

`0x64dabb` calls `GetLocalTime` when no time is given and `0x64dad8` builds the name with the extension
`txt` through `0x64d960`. `0x64db06` writes the title through `0x66baa0` and `0x64db0f` the text through
`0x64dc00`. `0x64db1b` tests `0xc53dc8`, `0x64db32` calls it with `(buffer, 0x1450)`, and `0x64db4a`
writes when `strlen` is not 0. `0x64db60` copies the path to `0xc53dd0`. In `0x403ca0`, `0x403cb8` prints
the build 0x16f3 and `0x403cc2` tests `ClntObjMgrIsValid`; `0x403d14` writes the zone through
`0x403ed0`, each object line goes through `0x403f70` with the format at `0x403fc6`, and `0x403e54` to
`0x403e83` walk the add-ons with `IsAddOnLoadedByName`. `0x403f70` writes nothing for an object that
`0x403f50` rejects.

## Crash dump

`0x66c9c0(file, exception, count, strings)` writes a minidump from a thread of its own when no
other report writer holds their shared busy counter, after making sure `psapi.dll` is loaded, waits for the thread and returns
`MiniDumpWriteDump`'s answer, or 0. The thread calls it for the current process with the type 0x40,
the exception and the current thread's id when an exception is given, and each non-null string, up
to 16, as a user stream of type `0x1000 + i` holding the string and its terminator. The register
section of the text report, `x86 Registers`, prints `EAX` to `ESI`, `EDI` to `FLG` and the six
segment registers from a thread context.

Addresses: `0x66c9c0`, `0x66c9d7`, `0x66ca02`, `0x66ca2a`, `0x66ca39`, `0x66ca60`, `0x66ca74`, `0x66cabb`, `0x66cac0`, `0x66cb0c`, `0x66cb20`, `0x66c900`, `0x66c90f`, `0x66c940`, `0x66bf60`
Confidence: verified
Proof: one
Test: none

`0x66c9d7` increments `0xc5a430`, which `0x66baa0` and `0x66bd50` take the same way, and only a
result of 1 goes on; `0x66ca4e` releases it; `0x66ca02` and `0x66ca11` find or load
`psapi.dll`; `0x66ca2a` creates the thread on `0x66ca60` and `0x66ca39` waits forever. The thread calls
`0x66b370` on the table `0xc5a3f0`, which loads `dbghelp.dll` and stores, after the module and a use count, twelve
`Sym` entry points and `MiniDumpWriteDump` at `+0x38`; `0x66cabb` stops the
streams at 16 and `0x66cac0` numbers them; `0x66cb0c` pushes 0x40 and `0x66cb20` calls the entry. The
result is stored at the record's `+0xc`. `0x66c900(print, argument, context)` prints the title through
`0x66bf60` and reads `Eax` at `+0xb0`, `Ebx` `+0xa4`, `Ecx` `+0xac`, `Edx` `+0xa8` and `Esi` `+0xa0`
for its first line (`0x66c940`).

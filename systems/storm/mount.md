# storm: mounting the archives

The client mounts its MPQ archives once, early in start-up, and resolves every asset name through one
priority-ordered list of them. The mounter is `0x403740`; the resolver is `0x6549a0`. `format.md` has the archive
format and the name hash.

Roots: `0x6549a0`, `0x6477a0`, `0x6477c0`, `0x648dd0`, `0x648ef0`

## Patch archives

`0x403740` collects `Data\patch-?.MPQ` through the operating system's wildcard (one character for `?`, so
`patch-10.MPQ` does not match, and hidden files are skipped), sorts the matches case-insensitively descending,
then adds `Data\patch.MPQ`, and opens the array from its end. The opening order is `patch.MPQ`, `patch-2`,
`patch-3`, and so on upward, from priority `0x40`, one more for each open that succeeds; a patch that fails to open
is skipped silently. Only the `Data` directory is searched.

Addresses: `0x403740`, `0x42ad10`, `0x4039b0`, `0x403ae0`, `0x41a350`, `0x41a4b0`, `0x648dd0`
Confidence: verified
Proof: one
Test: none

`0x42ad10` passes `Data\` plus the pattern to `FindFirstFileW` and `FindNextFileW` (`0x42ad7b`, `0x42ae09`), calls the
callback for each match unless it is hidden and the hidden flag is 0 (`0x42ade9` to `0x42adef`), and does not filter
directories. `0x4039b0` formats `Data\%s` (`0x4039c0`), copies it into the array and returns 0. The comparator
`0x403ae0` is `neg(strnicmp(a, b))`; the selection sort `0x41a4b0`, used up to 8 elements (`0x41a38d`), leaves the
element the comparator ranks greatest last, which is the alphabetically smallest, so the array is descending;
the quicksort branch for more is not read here. The open loop reads `[edx+edi*4−4]` (`0x4037eb`) and counts up
`ebx` (`0x4037fe`) only on success.

## Base and speech archives

`speech2.MPQ` opens next, if present, at the next priority. Then each patch that opened gets a child record for
the prefix `speech2` when a real `speech2.MPQ` exists. Then the ten base archives, `model`, `texture`, `terrain`,
`wmo`, `sound`, `misc`, `interface`, `fonts`, `speech`, `dbc` (`.MPQ`), at the fixed priorities `0x3f` down to `0x36`
whether or not each opens. A base or speech2 name is tried under `Data\` and then `..\Data\`; a missing base archive
logs `Failed to open archive %s: %s`, a missing speech2 or patch nothing. A base or speech2 archive that is present but fails its
header or table checks is a fatal error; a patch that does is skipped.

Addresses: `0x403740`, `0x403b00`, `0x4022d0`, `0x402320`, `0x648e80`, `0x655a40`, `0x82e12c`
Confidence: verified
Proof: one
Test: none

`0x4038b5` to `0x403912` walk the ten-entry table at `0x82e12c`, with the priority in `[ebp-8]` starting at `0x3f`
(`0x40389f`) and decremented for each entry (`0x403908`). `0x403b00` formats `%s%s` of each root (`0x82e124`) and the
name through `0x402320` and tries `0x648dd0`, twice (`0x403b78`); on failure it reports error `0x26` as `0x85100083`,
and `0x8510006c` and `0x85100083` go to `SErrDisplayAppFatal2` (`0x403b6f`). The child record is `0x648e80`, which
`0x655a40` completes (`0x403886`). `0x4022d0` tests which root holds a file (`0x42a4e0`) and returns 0, 1 or 2.

## base.MPQ and the speech setting

`Data\base.MPQ` is opened at priority `0x64`, `telemetry.dat` read from it, and the archive closed again, all before
`0x403740` runs, so a file only in `base.MPQ` is not found for the rest of the run. The CVar
`disableOptionalSpeech` (default `0`) is registered right after the mount; when its value is not 0 at start-up the
`speech2.MPQ` mount is closed. The setting is read once.

Addresses: `0x5aa2d0`, `0x402350`, `0x63db90`, `0x648ef0`
Confidence: verified
Proof: one
Test: none

`0x402361` calls `0x5aa2d0` and `0x402366` the mount. The open is `0x648dd0("Data\base.MPQ", 0x64, …)`
(`0x5aa2ed`), the read `telemetry.dat`, the close `0x648ef0` (`0x5aa35c`). The CVar registers at `0x40239d`, its value
is tested at `0x4023a2` (`[eax+0x28]`), and the closing loop `0x4023c0` to `0x402478` closes the recorded index and
compacts the handle array. Only the standalone speech2's index is recorded (`0x40382b`).

## Archive list

An opened archive is linked into the list at `0xc53fe0`, kept in descending priority order: the new record goes
before the first record whose priority is not above its own. A record's priority is at `+0x148`. The resolver can recurse
into an overlay chain at `+0x170` of an archive record; the record is zero-filled at allocation and the only
writers of that field, in `0x652780`, have no caller, so no archive has a chain and the recursion never runs.
Patches override base archives by priority alone.

Addresses: `0x655bf0`, `0x656012`, `0x655690`, `0x652780`, `0xc53fe0`
Confidence: verified
Proof: one
Test: none

`0x6559bb` stores the priority, `0x655958` allocates with the zero flag (`push 8`); the insert loop is `0x656012` to
`0x65603a`, `cmp [eax+0x148],ecx; jle` (`0x656025`, `0x65602b`), with a copy in `0x655a40` (`0x655b60`). `0x652780`
stores `+0x170` and has no `e8`, `e9` or dword reference in the image; the control, `0x648dd0`, has 3 callers.

## Name resolution

`0x6549a0` walks the list from its head. In each archive it probes the hash table (`format.md#hash-lookup`); a
hit ends the walk, and only a miss goes on to the next archive. So a name resolves to the highest priority archive
that has it: every patch beats every base archive, a patch that sorts after another beats it, and `speech2.MPQ` and its
children beat the patches. A speech2 child probes its parent patch for the name prefixed `speech2\`. A hit on a
block whose flags carry `0x02000000` ends the walk with result 3 and error 2: the file is not found, and a lower
archive's copy is not used.

Addresses: `0x6549a0`, `0x654cff`, `0x656690`, `0x654aea`, `0x654b7d`, `0x654d3a`, `0x654d53`
Confidence: verified
Proof: one
Test: none

`0x654aa4` loads the head, `0x654ce7` the next record. A hit reads the in-memory bit `+0xb & 1` (`0x654cff` to
`0x654d0a`), which `format.md#hash-and-block-tables` sets for deleted blocks, and returns 3 (`0x654d5c`) with
`SErrSetLastError(2)` (`0x654d63`); the file open `0x656690` fails on 0 and 3 (`0x6566f1` to `0x656700`). A name the archives do not hold can still resolve to a loose disk file: when flag bits 0 or 1 are set and the
archive has no `+0x144` set, `0x654b59` to `0x654b76` probe the disk path with `0x654dd0`, and a hit (`0x654b7d`) copies the path to
the caller's buffer (`0x654d3a` to `0x654d4e`) and returns 2 (`0x654d53`). Otherwise
`0x653140` fetches the block, and a clear `0x80000000` gives error `0x3ee` (`0x654d20` to `0x654d33`). The child's
parent and prefix are `+0x174` and `+0x178` (`0x654aea`, `0x654b03`); the joiner is `0x833184`.

## Loose files

A name that starts with two backslashes, has a colon in its second place, or is opened while `0xc5251d` is set,
goes to the disk-path builder `0x648180`. `0xc5251d` has one writer, in `0x646fa0`, which nothing calls. Any other name
reaches the archive walk with the disk probe of `0x6549a0` (`0x654b59`) disabled: the callers of `0x654920` clear
flag bits 0 and 1 when the byte `0x865b44` is 0, which the start-up `0x402210` sets (`0x40221a`) before any open.

Addresses: `0x647e60`, `0x648180`, `0x646fa0`, `0x64811f`, `0x648c20`, `0x402210`, `0x654b59`
Confidence: verified
Proof: one
Test: none

`0x647e9a` to `0x647ea9` test the doubled backslash and the colon; `0x647e8d` loads `0xc5251d`. `0x648116` to `0x64811f`
and `0x64840c` to `0x648415` clear the bits when `0x865b44` is 0, and `0x6548d0` gives the third caller `0x6548b0` flag 0 while an archive is open; the callers that pass flag 1, `0x78c39f`, `0x7a8860`,
`0x7a88db` and `0x7a8b86`, first go through a path-alias hash table at `0xc521e8`, whose writers are not read here.

## Start-up order

The start-up routine `0x402350` runs, in order: the mount, the console command registry, the CVar machinery, the
CVar `disableOptionalSpeech`, the command line, `Wow.ini` for the country and language, the opening and parsing of
`Startup_Strings.dbc`, the console device, the event library, the clock, the random seed `0x44ea80` and the start
handler `0x402840`. The handler runs the object heaps, the font and texture caches and the M2 cache, then `0x402ad0`,
which opens with the system message state, text drawing, the client's console commands, the `DBFilesClient` tables, the
sound interface, the script engine, the interface types, the network handlers and the world, and ends by
posting event 6.

Addresses: `0x402350`, `0x402840`, `0x402ad0`, `0x44ea80`, `0x41f5e0`, `0x41fc90`
Confidence: verified
Proof: one
Test: none

The calls in order, with their sites: `0x403740` (`0x402366`), `0x6400e0`, `0x63d380`, `0x63d820`, `0x63db90`,
`Wow.ini` (`0x4442c0` to `0x445900`), `0x6477a0` on `Startup_Strings.dbc` (`0x402586`), `0x63a230`, `0x41f550`, `0x42c940`,
`0x42c010` (`0x402802`), `0x44ea80` (`0x40280d`), `0x41f5e0` (`0x402825`); in `0x402840`: `0x463910`, `0x442170`, `0x63b460`,
`0x443300`, `0x448bd0`, eight CVars, `0x706dd0`, `0x402a00`, `0x402ad0`; in `0x402ad0`: `0x44cd10`, `0x409040`, `0x402ba0`, `0x53f4f0`,
`0x408000`, `0x456fe0`, `0x7039e0`, `0x6ef070`, `0x46cc60`, `0x51ab30`, `0x61f5a0`, `0x476020`, `0x5ab1d0`, `0x554ff0`, `0x66f6c0`, `0x46a400`, `0x46b840`,
`0x5142a0`, `0x50b6f0`, `0x6ec0e0`, `0x6c91a0`, and the jump to `0x41fc90`.

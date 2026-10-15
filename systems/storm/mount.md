# storm: mounting the archives

The client mounts its MPQ archives once, early in start-up, into one priority-ordered list. An open
resolves its name in `0x647e60`: when the caller's flags ask for it, first in an index of the loose files on
disk below the install folder and its `Data` folder, then in the archive list, whose walk is `0x6549a0`. The
mounter is `0x403740`; the index's builder is `0x646ea0`. `format.md` has the archive format and the name hash.

Roots: `0x6549a0`, `0x6477a0`, `0x6477c0`, `0x648dd0`, `0x648ef0`, `0x648a10`, `0x648a30`, `0x646fc0`,
`0x648be0`, `0x648c20`, `0x6488b0`, `0x648920`, `0x6466c0`, `0x6467a0`

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

`0x647e60(name, flags)` adds the bits of `0x865b40`, which start-up sets to 0 (`0x648be0(0)`) before anything
opens. A name that starts with two backslashes, has a colon second, or is opened while `0xc5251d` is set goes to
the disk-path builder `0x648180`; the one writer of `0xc5251d`, `0x646fa0`, has no caller. Any other name builds
the loose-file index if it is not built, asks it when flag bit 0 or 1 is set, and only on a miss walks the
archives (`0x654920`) with bits 0 and 1 cleared, since start-up clears `0x865b44` (`0x648c20`). So the walk's own
disk probe (`0x654b59`) never runs.

Addresses: `0x647e60`, `0x647e72`, `0x647ebc`, `0x647ec1`, `0x648116`, `0x64811f`, `0x648180`, `0x646fa0`, `0x648be0`, `0x648c20`, `0x402210`, `0x865b40`, `0x865b44`, `0xc5251d`, `0x654b59`
Confidence: verified
Proof: two
Test: `storm::resolve_matches_index`

`0x647e72` is `or ebx,[0x865b40]`; `0x648be0` stores 0 there, then 1 for argument bit 0 and ORs 2 for bit 1
(`0x648be8` to `0x648c02`); `0x402210` calls it with 0 after `0x648c20` (`0x40221a`, `0x40221f`), and `0x402120`
calls `0x402210` (`0x40215b`) before `0x402350` (`0x4021db`). `0x647e8d` loads `0xc5251d`; `0x647e9a` to `0x647ea9`
test the backslashes and the colon. `0x647eb3` tests the built byte `0xc5251c` and `0x647ebc` calls `0x646ea0`
whatever the flags; `0x647ec1` `test bl,3` sends 0 straight to `0x648116`. `0x648116` to `0x64811f` clear the bits
when `0x865b44` is 0, and `0x64840c` does the same in `0x648180`. The test runs `0x647e60` whole with the
callees named in its header stubbed.

## Loose-file index

`0x646ea0` walks the base path `0xc52418`, or `.` when it does not list, then that string with `0xc520c8` (`Data\`)
appended (`.Data\` in the fallback, so `Data` is then not walked), and marks the index built. `0x646910` skips
`.`, `..` and folders whose path below the walked folder is a prefix of `Data\` (case-sensitive), walks the
others, and enters every regular file: the key is its path below the walked folder, the value its full path. A last extension `.bz` or
`.MPQ` (any case) is cut from the key, as kind 1 or 2; any other file is kind 0. A key's first entry stays: the
base folder's before `Data\`'s, then listing order. A third folder, `0xc52310`, stays empty.

Addresses: `0x646ea0`, `0x646910`, `0x646d90`, `0x6674a0`, `0x667580`, `0x6676a0`, `0x6488b0`, `0x648920`, `0x6489a0`, `0x6466d0`, `0xc521cc`, `0xc521e8`, `0xc521f0`, `0xc52418`, `0xc520c8`, `0xc52310`, `0xc5251c`
Confidence: verified
Proof: two
Test: `storm::resolve_matches_index`, `storm::index_falls_back_to_dot`

`0x646eb1` and `0x646ec2` open the listing; `0x646ed8` copies the base path to `0xc521f8`; `0x646f13` appends `0xc520c8`;
`0x646f41` tests `0xc52310`, whose one writer `0x6489a0` has no caller; `0x646f8a` stores 1 in `0xc5251c`. In `0x646910`: `0x6469c0` and `0x6469d8` compare `.` and
`..` with `SStrCmp`; `0x646a04` stats; `0x646a14` tests `0x4000` and `0x646a6a` `0x8000` of `st_mode` (`+6`); `0x646a2f`
is `SStrCmp(rel, 0xc520c8, strlen(rel))`; `0x646a58` recurses. `0x646a78` finds the path's last dot; `0x646aab` and
`0x646bc7` compare `.bz` and `.MPQ` with `SStrCmpI`, `0x646ab8` and `0x646bd4` cut there; `0x646bad`, `0x646ca5` and
`0x646d4a` store kinds 1, 2 and 0 at `+0x1c`, the value at `+0x18`, the key at `+0x14`; a key already present frees
the new value (`0x646d66`). `0x6488b0` stores the base path with a trailing backslash; start-up passes it the
command-line value of `0x419c20(1)`, or the exe's folder (`0x42b2e0`: `GetModuleFileNameW` cut after its last
backslash) when that is empty (`0x402226` to `0x402255`). `0x648920` stores `Data\` (`0x40228c`, `0x4022a7`).
Folders are listed through `FindFirstFileW` and `FindNextFileW`, since Storm's wide option `0xc53f14` is set at
start-up (`0x402141`), and entries are stated through `_wstat` or `_stat`.

## Index lifetime

The index lives from the first open that reaches `0x647e60` until `0x646fc0` empties it and clears the built byte;
the next open builds it again. The client calls `0x646fc0` when it has written a saved-variables file
(`0x491865`, `0x51fa31`) or a character's `AddOns.txt` (`0x51f09d`), or moved loose interface folders aside
(`0x51fbe3`); Storm's shutdown `0x648d10` empties it too. A file added in between is not found until the next
build; one removed in between is found and fails to open. `0x647040`, which nothing calls, empties the index and
marks it built.

Addresses: `0x646fc0`, `0x647040`, `0x648d10`, `0x491730`, `0x51ef20`, `0x51f650`, `0x51fb50`, `0xc5251c`, `0xc521cc`, `0x6466c0`, `0x6466d0`, `0x646790`, `0x6467a0`
Confidence: verified
Proof: two
Test: none

`0x646fc0` unlinks (`0x646fe5`) and deletes (`0x64701d`) every node and stores 0 in `0xc5251c` (`0x64702f`); `0x647040` is the same
loop ending in 1 (`0x6470af`); `0x648d10` ends with the loop and 0 (`0x648d5e` to `0x648dc2`). Each caller tests a
written or moved count before the call (`0x491863`, `0x51f09b`, `0x51fa2f`). The census of `e8`, `e9` and stored
dwords finds these four callers of `0x646fc0`, none of `0x647040` and one of `0x648d10` (`0x64cc09`); the control,
`0x648dd0`, has its 3. The table `0xc521cc` is built empty by the static initializer `0x6466c0` (body `0x6466d0`,
mask 0xffffffff at `0x646782`) and freed at exit by `0x6467a0`, which `0x646790` registers with `_atexit`.

## Index lookup and open

A probe finds an entry whose `SStrHashHT` hash and key equal the name's, the key compared by `SStrCmpI`; the hash
reads `/` as `\` and the compare does not, so a name with `/` never matches. Flag bit 1 first probes the part after the last backslash, a file directly in the base folder or
`Data\`; then bit 0 or 1 probes the whole name. A hit returns the full path and the kind. `SFile::OpenEx` opens
kind 0 with `fopen(path, "rb")`, kind 1 the same way and checks `BZ00` in its 24-byte header, kind 2 as an
archive holding the name's last component. A failed open returns 0 without the archives. A third probe never
changes the result.

Addresses: `0x647e60`, `0x647ed7`, `0x647f37`, `0x647f89`, `0x64802c`, `0x648110`, `0x64b3f0`, `0x64a4c0`, `0x6477c0`, `0x64783d`, `0x647865`, `0x6478d5`, `0x647964`, `0x647a63`, `0x647a94`, `0x647d9c`
Confidence: verified
Proof: two
Test: `storm::resolve_matches_index`, `storm::index_hit_has_no_fallback`

The bucket is `hash & [0xc521f0]` (`0x647f07`); a bucket's first word, 4, places an entry's chain links at `+0x4`
and `+0x8`, and the table's list of all entries links at `+0xc` and `+0x10`. `0x647ed7` is `SStrChrR(name, '\')`; `0x647f37` and `0x647fdb` compare the key at `+0x14`; a hit copies `+0x18`
(`0x647f68`) and stores `+0x1c` as the kind (`0x647f75`). `0x64803b` compares `0xc521f8` with `0xc52418`; the third
probe (`0x64802c`), of the base path joined to the name, runs only when they differ, and its hit falls through to `0x648110` and the archive walk, which overwrites the kind on a hit and returns 0 on a
miss. `0x6477fc` calls `0x647e60`; `0x64783d` jumps through `0x647d9c` (kinds 0 to 4: `0x647844`, `0x647894`,
`0x647a30`, `0x647ab2`, `0x647b45`). A null `FILE` sends kind 0 to `0x647c64` (`0x647865`) and kind 1 to
`0x6478d5`; a wrong magic sends kind 1 to `0x647981` (`0x647964`); a failed archive or file open sends kind 2 to
`0x647c64` (`0x647a63`, `0x647a94`). Each returns 0 without calling `0x654920`. The tests run `0x647e60` and
`0x6477c0` with the file system and the archive walk stubbed, 822 name and flag cases on each of two installs against `LooseIndex`.

## Which opens ask the index

An open asks the index only with flag bit 0 or 1. `SFile::Open`, `0x6485f0` and `0x648a10` pass 0, and so do the DBC tables,
the terrain files, the minimap catalog, movie subtitles, `VideoHardware` and the telemetry read. These pass 1 or 3:

| entry | flags | opens |
|---|---|---|
| `SFile::Load` for interface and settings files | 1 | `.toc`, `.xml`, `.lua`, bindings, saved variables, `AddOns.txt`, `Config.wtf` |
| `0x648a30` from the add-on loader and `CGGameUI::Initialize` | 1 | existence of an add-on's `.toc`, bindings and saved variables, and of `Interface\FrameXML\Bindings.xml` |
| `0x5d0060`, a font face | 3 | the font file: its bare name first, then its path |
| `CSimpleTexture::SetTexture`, `Minimap` texture methods, model frame `ReplaceTexture` | 1 | interface textures through `0x449d90` |
| `0x449610` from the cursor code | 1 | the cursors and the dragged picture |
| `CreateModelFromPath` | flags bit 2 | the `.m2` and its textures |
| the sound callbacks `0x7a8770`, `0x7a8ab0` | 1 | every file FMOD opens |
| `ReadMovieAvi` | 1 | the movie `.avi` |
| `GetPortraitAlphaMask`, `SetPortraitTexture` through `0x5a3660` and `0x5a7b40` | 1 | the portrait masks and portraits |
| the texture updates `0x448840` and `0x44a260`, and `0x4493a0` | bit 0x2 of the texture's `+0x110` | a texture's re-read, by its first open's flag |

The texture cache `0x47ac10` calls `0x449610` with 0; the other callers of `0x449d90` and the tabard emblems'
`0x5a7b40` pass 0.

Addresses: `0x6477a0`, `0x6485f0`, `0x648620`, `0x648a30`, `0x648a10`, `0x48fff9`, `0x52594f`, `0x525a00`, `0x525e37`, `0x5033eb`, `0x449418`, `0x5d0121`, `0x449d90`, `0x449e20`, `0x770281`, `0x4ee5e4`, `0x76d03d`, `0x449610`, `0x707368`, `0x706afd`, `0x71d4fa`, `0x71da72`, `0x7a45e4`, `0x7a8860`, `0x78c39f`, `0x47ac30`
Confidence: verified
Proof: two
Test: none

`0x6477a0` and `0x6485f0` push 0 as the flags. The census of `e8` and `e9` to the open entries finds 156 calls of
`SFile::Open`, 11 of `0x6477c0`, 23 of `0x648620`, 6 of `0x6485f0`, 7 of `0x648a30` and 1 of `0x648a10`, with no stored address;
the flags of each were read at the call, and through `0x5a7b40` and `0x5a3660`, which pass their caller's. Bit 0x2
of `+0x110` is set by the three texture opens (`0x4496a8`, `0x44a1d0`, `0x44a3d7`) and passed by `0x4488e4` and
`0x44a2d6`; `0x4491f0` gives it to `0x4493a0`'s probe (`0x449418`), and `0x4490c0` is reached only from `0x4499d0`,
which nothing calls. `0x449d90` passes its third stack argument as `edx` (`0x449e20`), which
`0x44a310` and `0x5a3660` turn into flag `edx != 0`. `0x707368` passes `flags & 4` to `0x706afd`; `0x71d4fa` keeps
it as bit 1 of the shared model's `+8`, which `0x71da64` pushes for its textures. `0x7a45e4` registers the
sound callbacks with `FSOUND_File_SetCallbacks`. Model creations whose flags come from a call before the push
were not traced one by one.

## Hash-table string hash

`SStrHashHT` (`0x64b3f0`) copies at most the first 1023 bytes of a string, raising `a` to `z` by 0x20 and turning
`/` into `\`, and returns `bjhash(copy, length, 0)`. `bjhash` (`0x64b1d0`) is Bob Jenkins' `lookup2`: `a` and `b`
start at 0x9e3779b9 and `c` at the initial value; each 12-byte block adds three little-endian words and mixes;
then `c` gains the length, the last 0 to 11 bytes are added (`c` from its second byte up), and a last mix returns `c`.

Addresses: `0x64b3f0`, `0x64b1d0`, `0x64b3c0`
Confidence: verified
Proof: two
Test: `storm::str_hash_ht`

`0x64b411` stops the copy at 0x3ff bytes; `0x64b419` to `0x64b42d` fold the case and the slash; `0x64b440` pushes the
initial value 0 with the length in `edx`. `0x64b1df` loads 0x9e3779b9; the block loop runs `len / 12` times
(`0xaaaaaaab`, `0x64b1f2`); `0x64b2e6` adds the length; `0x64b2ee` jumps through the table `0x64b3c0` for the tail.
The shifts of the mix are 13, 8, 13, 12, 16, 5, 3, 10, 15. The test runs `0x64b3f0` whole, `bjhash` inside it,
over every length to 40, the 1023-byte cap and a fuzz.

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

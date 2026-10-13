# ui: add-ons and saved variables

How the client finds add-ons, reads their toc directives, decides which may load, loads them with
their dependencies and saved variables, keeps each character's enable state, and writes the saved
variables back.

`AddOns.cpp` (`WoW\Source\UIUtil`, from 0x51c740 to the add-on list's helpers ending at 0x522170)
owns the registry: a hash of records by folder name, with a list in registration order at
`0xbe1b64`, and a sorted array of names at `0xbe1b94` that Lua indexes. The glue's add-on functions
are in `glue/script`; the in-game ones, the flat `SavedVariables.lua` and the shutdown that writes
everything are in `GameUI.cpp`. Files of an add-on load through `framexml`'s toc runner.

Roots: `0x51c740`, `0x51c910`, `0x51d7b0`, `0x51f240`, `0x51f600`, `0x51f650`, `0x51ebe0`, `0x51ef20`, `0x51ea20`, `0x51e470`, `0x51e780`, `0x51da70`, `0x51d910`, `0x51deb0`, `0x4913b0`, `0x491730`, `0x48e350`, `0x48e390`, `0x48e530`, `0x48e5e0`, `0x48e690`, `0x48e720`, `0x48e760`, `0x48e7f0`, `0x48e830`, `0x48e840`, `0x48e8e0`, `0x48e980`, `0x4884e0`

## Finding add-ons

The glue's `DefaultServerLogin` resets the registry and scans `Interface\AddOns\` twice. First,
every name in the mounted archives' `(listfile)`s that begins with that prefix and ends in `.toc`
registers the folder name before its first separator, in listfile order. Then every loose
subfolder not beginning with `.` registers its name, in the order the file system lists them. A
name already registered, ignoring case, is skipped, and a folder without a readable
`<Name>\<Name>.toc` registers nothing. Records join the list at its tail.

Addresses: `0x46b084`, `0x51c740`, `0x51c743`, `0x51c760`, `0x51c777`, `0x51c78f`, `0x51c910`, `0x51c932`, `0x51c986`, `0x51d7b0`, `0x51d7b4`, `0x51d7bc`, `0x51c9b0`, `0x51ca10`, `0x51ca62`, `0x51ca69`, `0x51caf5`, `0x521ad0`, `0x401470`, `0x4014fe`, `0x648fb0`, `0x42ad10`, `0x853724`, `0x853738`, `0x853840`
Confidence: verified
Proof: one
Test: none

`0x46b084` tail-jumps to `0x51c740`, which resets (`0x51fa40`, `0x51c743`) and scans. `0x51c777`
walks the archives through `0x401470`, whose adapter `0x4014f0` compares the prefix ignoring case
(`0x4014fe`) over the lines of `(listfile)` read by `0x648fb0`; `0x51c78f` walks the folder through
`0x42ad10` with `*`. `0x51c910` keeps a name whose text from its last `.` is `.toc` (`0x51c932`),
cuts it at the first `\` or `/` and registers it (`0x51c986`). `0x51d7b0` requires the directory
attribute 0x10 (`0x51d7b4`) and a first character other than `.` (`0x51d7bc`). In `0x51c9b0` a name
match returns at `0x51ca10`; the toc is read at `0x51ca62`, failure returning at `0x51ca69`;
`0x51caf5` calls `0x521ad0` on `0xbe1b64` with 2, the tail append.

## Toc directives

A toc's lines that begin `##` set the record's fields; other lines are the file list, which only the
toc runner reads. After the `##` and any spaces, each name is matched as a prefix, ignoring case, in
this order:

| directive | stored |
|---|---|
| `Interface:` | the number, `+0x1c` |
| `Title`, `Notes`, `Author`, `Version`, `X-` | metadata, below |
| `Revision:` | the number, `+0x20` |
| `OptionalDep` | names split at spaces and commas, `+0x38` |
| `RequiredDep`, `Dep` | the same, `+0x48` |
| `LoadWith:` | the same, `+0x58` |
| `SavedVariables:` | the same, `+0x68` |
| `SavedVariablesPerCharacter:` | the same, `+0x78` |
| `DefaultState:` | `enabled` 1, `disabled` 0, else unchanged, `+0x2b` |
| `LoadOnDemand:` | true or false, `+0x2c` |
| `Secure:` | true or false, `+0x28` |

Addresses: `0x51cc22`, `0x51cc2e`, `0x51cc69`, `0x51cc8e`, `0x51cca6`, `0x51cd18`, `0x51cd5f`, `0x51ce6e`, `0x51ce8c`, `0x51ceaa`, `0x51cfbe`, `0x51d0be`, `0x51d1be`, `0x51d204`, `0x51d223`, `0x51d23c`, `0x51d27c`, `0x51d295`, `0x51d2cc`, `0x51ca75`, `0x51cbf9`, `0x520550`, `0x5205b9`
Confidence: verified
Proof: one
Test: none

In `0x51c9b0`, `0x51cc22` and `0x51cc2e` require `#` twice. The compares run in the table's order
with the prefix's own length (`0x51cc69` onward); the lists tokenize with `" ,"` and append
`SStrDupA` copies. `DefaultState` stores 1 for `enabled` (`0x51d204`) and `al`, 0, for `disabled`
(`0x51d223`); `LoadOnDemand` and `Secure` store `setne` of `0x6f1b30`, true for a first character 1
to 9, `t`, `T`, `y` or `Y`, or for `on` or `enabled` ignoring case. The toc's BOM is skipped
(`0x51ca75`) and `0x51cbf9` stores its count of non-`#` lines at `+0x34`. The constructor `0x520550`
zeroes the record and stores 1 in `+0x24` and `+0x2b` (`0x5205b9`), so an add-on without
`DefaultState` is enabled by default.

## Add-on metadata

Before the directives are read the record's `Title` is its folder name. A `Title`, `Notes`,
`Author`, `Version` or `X-` line stores its value, after the colon and spaces, under its name up to
the colon with trailing spaces removed; a later line of the same name replaces it. A name ending in
`-` and a locale code (`enUS`, `koKR`, `frFR`, `deDE`, `zhCN`, `zhTW`, `esES`, `xxYY`) is kept,
under the name before the dash, only when that code is the client's locale. `GetAddOnMetadata(addon,
name)` answers a stored value, or nil.

Addresses: `0x51cbb3`, `0x51cbf3`, `0x51d3c6`, `0x51d580`, `0x51d5a0`, `0x51d5c2`, `0x51d5e1`, `0x51d600`, `0x51d79c`, `0x8558a4`, `0xc0e080`, `0x48e530`, `0x48e5b5`, `0x51e220`
Confidence: verified
Proof: one
Test: none

`0x51cbb3` to `0x51cbf3` add the key `Title` with the folder name. `0x51d580` keeps the position
after the last `-` before the colon (`0x51d5a0`), compares four characters with each of the eight
codes at `0x8558a4` (`0x51d5c2`) and, on a match, returns unless the index equals `[0xc0e080]`
(`0x51d5e1`); `0x51d600` trims spaces. The value replaces any old one (`0x51d79c`).
`GetAddOnMetadata` (`0x48e530`) pushes `0x51e220`'s answer (`0x48e5b5`).

## The order Lua sees

Lua's add-on indexes run over a separate array of names, built when the realm server's add-on
reply arrives: every record not hidden by that reply, sorted by its `Title` ignoring case, the
folder name standing in for a missing title. The sort is the C runtime's `qsort`, which keeps no
order between equal titles. Until the reply arrives the count is 0. An index is truncated, reduced
by 1 and compared unsigned with the count.

Addresses: `0x51da70`, `0x51dc30`, `0x51dc4f`, `0x51dcac`, `0x51dcd1`, `0x51deb0`, `0x51df20`, `0x51def0`, `0x51df00`, `0x5b4282`, `0xbe1b90`, `0xbe1b94`
Confidence: verified
Proof: one
Test: none

`0x51da70`'s tail `0x51dc30` zeroes `0xbe1b90`, walks the list, skips a record whose `+0x29` is set
(`0x51dc4f`), appends `+0x14` (`0x51dcac`) and calls `_qsort` with `0x51deb0` (`0x51dcd1`).
`0x51deb0` compares `0x51df20`'s `Title` for each name, or the name when 0, with `SStrCmpI`.
`0x51da70` has one caller, `0x5b4282`, in the add-on reply's handler `0x5b4260`. `0x51def0` answers
the count and `0x51df00` the name or 0 for an index not below it.

## Secure add-ons

An add-on that declares `Secure` takes its key from `<Name>.pub` and its URL from `<Name>.url` in
its folder, and is named to the realm server in the auth packet's add-on block. The server's reply,
read for these add-ons in list order, can replace the key and URL, which the client then writes back
to those files, and gives a state: 0 bans the add-on, 2 hides it from the list, and 1 or 2 has its
toc checked against its signature. A good signature makes the add-on secure (`+0x24` 0) and stores
the digest its files must hash to; a mismatched one marks it corrupt. Every other add-on stays
insecure (`+0x24` 1).

Addresses: `0x51d3f4`, `0x51d3fd`, `0x51d404`, `0x51d910`, `0x51da70`, `0x51daa5`, `0x51dac4`, `0x51daee`, `0x51daf5`, `0x51db08`, `0x51db0d`, `0x51db34`, `0x51db6d`, `0x51db77`, `0x51db84`, `0x51dbeb`, `0x51dbfb`, `0x51dc04`, `0x51dc0a`, `0x51dce0`, `0x51dde0`, `0x6f10f0`
Confidence: verified
Proof: one
Test: none

At registration `0x51d3f4` tests `+0x28` and calls `0x51d410` and `0x51d4c0`, which read the two
files into `+0xc1` (0x101 bytes) and `+0x30`. `0x51d910` builds the block from the same records
(next entry). `0x51da70` reads, per record with `+0x28` (`0x51daa5`), a state byte, a key flag
(`0x51dac4`) and, when set, a byte and a 256-byte key (`0x51daee`) written by `0x51dce0`
(`0x51daf5`), and a revision whose excess over `+0x20` sets `+0xc0` (`0x51db08`, `0x51db0d`); then a
URL flag and string (`0x51db34`) written by `0x51dde0` (`0x51db6d`). It stores 1 in `+0x24`
(`0x51db77`); state 2 sets `+0x29` (`0x51db84`), state 0 stores 2 (`0x51dc0a`); `0x6f10f0`'s 2 sets
`+0x2a` (`0x51dc04`) and 3 stores 0 (`0x51dbfb`), with the digest at `+0x1c2`.

## The auth packet's add-on block

With any add-on registered, the auth packet gets a u32, the size of the block before compression,
and then the block compressed with zlib at its default level. The block holds, for each add-on that
declares `Secure`, in list order: the folder name as a NUL-terminated string; a byte, the first of
the 0x101-byte `.pub` file or the one the server's reply gives, 0 without either; a u32, the key's
CRC-32 when that byte is nonzero, whatever its value, else 0; and a u32, the CRC-32 of the URL's
text, or 0 for none or an empty one. An empty registry adds nothing; one with no `Secure` add-on
adds the size 0 and an empty stream.

Addresses: `0x51d910`, `0x51d917`, `0x51d924`, `0x51d961`, `0x51d96f`, `0x51d980`, `0x51d98d`, `0x51d99e`, `0x51d9ab`, `0x51d9b5`, `0x51d9ca`, `0x51d9d7`, `0x51d4a0`, `0x52060f`, `0x51da02`, `0x51da0f`, `0x51da20`, `0x51da33`, `0x51da3f`, `0x4181f0`, `0x4182b0`, `0x418c10`, `0x6628d0`, `0x662860`, `0x6605a0`, `0xbe1b64`
Confidence: verified
Proof: one
Test: none

`0x51d917` reads the list head `0xbe1b6c` and `0x51d924` returns on an empty list before writing.
The records go into a local store: per record with `+0x28` (`0x51d961`), `PutCString` of `+0x14`
(`0x51d96f`), the byte `+0xc1` (`0x51d980`), and `0x4182b0` of `0x6628d0(+0xc2, 0x100, 0, 7)` when
`+0xc1` is nonzero (`0x51d98d`, `0x51d99e`, `0x51d9ab`), else 0. `0x51d4a0` stores the file's first
byte there, and the constructor zeroes it (`0x52060f`); then a nonempty `+0x30` (`0x51d9b5`)
gives `0x6628d0(url, SStrLen(url), 0, 7)` (`0x51d9ca`, `0x51d9d7`). `0x6628d0` calls `0x662860`,
whose flags 1 and 4 start at 0xffffffff and invert the result over the table `0x866bd8`, standard
CRC-32. The packet then gets the store's size (`0x4181f0`, `0x51da02`); `0x51da0f` takes the bytes,
`0x51da20` allocates the size plus 0x20 on the stack, `0x6605a0` compresses with level −1
(`0x51da33`) and `0x418c10` appends the result (`0x51da3f`).

## Whether an add-on may load

The load check answers false with a reason: `MISSING` for an unknown name; `DISABLED` when the
character's enable state is 0; `BANNED`; `CORRUPT`; `INTERFACE_VERSION` when `Interface` is not
11200 and the CVar `checkAddonVersion` (default `1`) is nonzero; a dependency's own failure, as a
second reason shown with `DEP_`; and, when called for a load on demand, `NOT_DEMAND_LOADED` unless
the add-on declares `LoadOnDemand`. An add-on already being checked, through a dependency cycle,
passes. The reason `INSECURE` exists but the check never gives it.

Addresses: `0x51e780`, `0x51e812`, `0x51e824`, `0x51e832`, `0x51e841`, `0x51e85b`, `0x51e876`, `0x51e889`, `0x51e8ae`, `0x51e8d5`, `0x51e909`, `0x51e915`, `0x51d7d0`, `0x51f180`, `0x402c3b`, `0x82e99c`, `0x85365c`, `0x51e930`
Confidence: verified
Proof: one
Test: none

Codes go to the first out argument: 1 (miss), 2 when `0x51e470(name, character, 1)` is 0
(`0x51e824`, `0x51e832`), 3 when `+0x24` is 2 (`0x51e841`), 4 when `+0x2a` is set (`0x51e85b`), 7
when `+0x1c` differs from `0x51d7d0`'s 0x2bc0 and `0x51f180` reads the CVar nonzero (`0x51e889`), 6
(`0x51e915`). A set `+0x2d` returns true at once (`0x51e812`); `0x51e8ae` sets it while each
required dependency not yet loaded is checked recursively with the second out argument as its reason
(`0x51e8d5`). `0x402c3b`, in `0x402ba0`, registers the CVar. `0x51e930` names codes from `0x85365c`,
or `DEP_%s` with the second.

## Loading an add-on

`0x51f240(name, demand, status)` answers true at once for a loaded add-on. Otherwise, when the load
check passes, it marks the add-on loaded, loads its optional dependencies, then its required ones,
unmarking it and failing if one fails; runs its toc's files and its folder's `Bindings.xml` under
its context word (its name when insecure, 0 when secure) with an MD5 over them; runs its account
and then per-character saved variables; for a secure add-on quits with code 10 unless the MD5
matches its digest; signals `ADDON_LOADED` with its name; and then loads each add-on that declared
`LoadWith` it.

Addresses: `0x51f240`, `0x51f2d6`, `0x51f2fa`, `0x51f313`, `0x51f32d`, `0x51f350`, `0x51f5ee`, `0x51f387`, `0x51f3fa`, `0x51f443`, `0x51f4b5`, `0x51f53b`, `0x51f54e`, `0x51f565`, `0x51f56e`, `0x51f5ad`, `0x51f5d3`, `0x51c7c0`, `0x51c854`
Confidence: verified
Proof: one
Test: none

`0x51f2d6` tests `+0x18`; `0x51f2fa` calls `0x51e780`; `0x51f313` stores 1 before the loops over
`+0x3c`/`+0x40` (`0x51f32d`, results unused) and `+0x4c`/`+0x50` (`0x51f350`), whose failure
stores 0 (`0x51f5ee`). `0x51f387` sets the context. `0x51f3fa` runs the toc with the MD5 context and
`0x51f443` the bindings file when it exists. `0x51f4b5` and `0x51f53b` run the saved variables
(next entries) with no MD5. For `+0x24` 0, `0x51f565` compares the digest with `+0x1c2` and
`0x51f56e` calls `0x401560(10)`. `0x51f5ad` signals 0x1ad with `%s`; `0x51f5d3` loads `+0x90`'s
records, which the scan's last pass appends to each record named in another's `LoadWith` list
(`0x51c7c0`, `0x51c854`).

## Load order and cycles

At the interface's build the add-ons not loaded on demand load in registration order, each pulling
its dependencies in first, before the flat saved variables and `VARIABLES_LOADED`. Because an
add-on is marked loaded before its dependencies, a cycle of required dependencies loads every
member once: with A requiring B and B requiring A, loading A loads B's files and signals B's
`ADDON_LOADED` first, then A's. `IsAddOnLoaded` is already true while an add-on's own files run.

Addresses: `0x51f600`, `0x51f61e`, `0x51f62b`, `0x4900a3`, `0x4900b2`, `0x51f313`, `0x51f2d6`, `0x51e812`
Confidence: verified
Proof: one
Test: none

`0x51f600` walks the list from `0xbe1b6c` and calls `0x51f240` with `dl` 0 for each record whose
`+0x2c` is 0 (`0x51f61e`, `0x51f62b`); its one caller is `0x4900a3`, before `0x4900b2`. In a cycle
B's check of A passes at `0x51e812`, B's load of A returns at the loaded test `0x51f2d6`, and B's
files then run before A's.

## Enable state

Each character on the realm has a node of add-on names with an enabled flag, read from
`WTF\Account\<account>\<realm>\<character>\AddOns.txt` or, when that is missing,
`WTF\Account\<account>\AddOns.txt`, whose use marks the node to be written. A line is `Name: value`;
a value of `disabled`, ignoring case, is 0 and anything else 1. A last line survives only when two
separators follow it, as CR LF does; one ending in a lone LF or CR is dropped. Writing rewrites each
changed node's file as `Name: enabled` or `Name: disabled` with CR LF. The glue's roster builds the
nodes, and the in-game `ResetDisabledAddOns` rereads the current character's. The glue's
`SaveAddOns` also writes.

Addresses: `0x51ebe0`, `0x51ebf9`, `0x51ec1a`, `0x51ecde`, `0x51ed1c`, `0x51ed49`, `0x51ed6a`, `0x51ed72`, `0x51edd3`, `0x51ee69`, `0x51ef20`, `0x51ef5e`, `0x51ef8a`, `0x51efb6`, `0x51f044`, `0x853928`, `0x853948`, `0x85392c`, `0x853968`, `0x64af66`, `0x46d990`, `0x4723c0`, `0x4724d2`, `0x48e837`
Confidence: verified
Proof: one
Test: none

`0x51ebe0(character)` reloads every node for null (`0x51ec1a`), else finds or creates (`0x51ee69`)
the node and clears it. `0x51ecde` formats `0x853948`; when it fails to read, `0x51ed1c` formats
`0x85392c` and `0x51ed49` marks the node. `0x51ed6a` tokenizes at CR, LF and `"` and `0x51ed72`
stops when the rest after the token is empty, before using it; the tokenizer steps past exactly one
separator (`0x64af66`); `0x51edd3` compares with `disabled`. `0x51ef20` writes nodes marked
(`0x51ef5e`) to `0x838708` plus `\AddOns.txt` (`0x51ef8a`, `0x51efb6`), each entry with `0x853968`,
`%s: %s\r\n` (`0x51f044`). The roster's `0x4723c0` loads one node per character, after `0x4724d2`
drops them all; glue's `0x46d990` calls `0x51ef20`; `0x48e830` passes `0x5abdc0`'s name
(`0x48e837`).

## Reading the enable state

`0x51e470(addon, character, fallback)` answers 2 enabled, 1 mixed or 0 disabled from the nodes it
considers: the named character's, or all for null. A node with an entry counts its flag. A node
without one counts only with `fallback`: then the add-on's state over all nodes counts as enabled
when it is 2, or when it is 1 and `DefaultState` is enabled. With nothing counted the answer is
`DefaultState`, 2 or 0; otherwise 0 when none is enabled, 2 when all are, else 1. Setting a state
creates or changes the entry and marks the node; the in-game `EnableAddOn` and `DisableAddOn` set 2
and 0 for the current character, and `EnableAllAddOns` and `DisableAllAddOns` for every add-on.

Addresses: `0x51e470`, `0x51e542`, `0x51e5ca`, `0x51e5dc`, `0x51e5f0`, `0x51e600`, `0x51e607`, `0x51e61c`, `0x51e623`, `0x51e64b`, `0x51ea20`, `0x51eb62`, `0x51ebb9`, `0x51ebbc`, `0x48e6fc`, `0x48e7cc`
Confidence: verified
Proof: one
Test: none

For a named character `0x51e542` skips other nodes and the walk ends after its node. A hit adds its
byte to the count of entries (`0x51e5ca`). With no entry and the flag (`0x51e5dc`), `0x51e5f0`
recurses with null and 0: an answer of 1 tests `+0x2b` (`0x51e600`), 2 counts (`0x51e607`). With
none counted `0x51e623` answers `+0x2b ? 2 : 0`; else `0x51e64b` computes `enabled < total ? 1 : 2`.
`0x51ea20` inserts a missing entry (`0x51eb62`), stores the flag and marks the node (`0x51ebb9`,
`0x51ebbc`). `0x48e6fc` (in `0x48e690`) and `0x48e7cc` (in `0x48e760`) pass 2 and 0 with
`0x5abdc0`'s name; `0x48e720` and `0x48e7f0` pass 2 (`0x48e730`) and 0 (`0x48e800`) for each index
below `0x51def0`.

## The in-game add-on functions

Each takes an index or a name. A number or numeric string is an index, out of range raising
`AddOn index must be in the range of 1 to %d`; another string is a name, ignoring case, which may
match nothing; anything else raises the function's usage. `GetAddOnInfo` answers seven values: the
name; the title and notes or nil; enabled for the current character as 1 or nil; loadable, 1 when
loaded, else the on-demand check as 1 or nil; its reason or nil; and `SECURE`, `INSECURE` or
`BANNED`. `LoadAddOn` loads on demand and answers 1, nil or nil and the reason, `UNKNOWN_ERROR` when
the check has none. `IsAddOnLoaded` and `IsAddOnLoadOnDemand` answer 1 or nil, `GetNumAddOns` the
count.

Addresses: `0x48e390`, `0x48e405`, `0x48e44b`, `0x48e46b`, `0x48e4a5`, `0x48e4e6`, `0x48e4fd`, `0x48e50c`, `0x48e980`, `0x48ea90`, `0x48eafe`, `0x48eb3a`, `0x48e840`, `0x48e865`, `0x48e89d`, `0x48e8c7`, `0x48e8e0`, `0x48e5e0`, `0x51e350`, `0x51e660`, `0x51e6f0`, `0x51e980`, `0x837d70`, `0x842ebc`, `0x85367c`
Confidence: verified
Proof: one
Test: none

`IsAddOnLoadOnDemand` (`0x48e840`) is the pattern: `lua_isnumber(1)`, `__ftol`, `dec`, `0x51df00`
(`0x48e865`) or the range error; `lua_isstring(1)` and the name; `0x51e660` (`+0x2c`, `0x48e89d`)
pushes 1 or nil; else the usage (`0x48e8c7`). `GetAddOnInfo` pushes the name (`0x48e405`), calls
`0x51e470` with flag 1 (`0x48e44b`), `0x51e6f0` (`0x48e46b`), `0x51e780` with `dl` 1 (`0x48e4a5`),
`0x51e930` (`0x48e4e6`) and `0x51e980` (`0x48e4fd`), returning 7 (`0x48e50c`). `LoadAddOn` calls
`0x51f240` with `dl` 1 (`0x48ea90`) and on failure `0x51e780` (`0x48eafe`), pushing `0x842ebc` when
both codes are 0 (`0x48eb3a`). `GetAddOnDependencies` pushes `0x51e350`'s list `+0x48`. `0x48e350`
pushes `0x51def0`'s count (`0x48e373`).

## Saved variables, read

An add-on's account file `WTF\Account\<account>\SavedVariables\<Name>.lua` and then its character
file `WTF\Account\<account>\<realm>\<character>\SavedVariables\<Name>.lua`, or the realm-less
`WTF\Account\<account>\<character>\SavedVariables\<Name>.lua` when that is missing, run as Lua
after the add-on's own files and before its `ADDON_LOADED`, so their values replace what the files
set. They run in the shared state under the add-on's context word, and an error in one goes to the
error handler with nothing logged. The flat `WTF\Account\<account>\SavedVariables.lua` is not run:
each line `name = value` sets a string, from a quoted value, nil for an empty one, or a number
read as a single-precision float; other lines are ignored. `VARIABLES_LOADED` follows, file or not.

Addresses: `0x51f491`, `0x51f4a2`, `0x51f4b5`, `0x51f4df`, `0x51f518`, `0x51f53b`, `0x8539d4`, `0x8539a8`, `0x853980`, `0x4913b0`, `0x4913c4`, `0x4913e6`, `0x49144a`, `0x4914bf`, `0x4914ef`, `0x491534`, `0x491541`, `0x49156a`, `0x491580`
Confidence: verified
Proof: one
Test: none

`0x51f491` formats `0x8539d4`, `0x51f4a2` tests existence and `0x51f4b5` runs it with no MD5 and no
status; `0x51f4df` and `0x51f518` format the two character paths and `0x51f53b` runs the one
found. `0x4913b0` builds the path (`0x4913c4`), reads it (`0x4913e6`) and splits it at CR and LF;
the name ends at a space or `=` (`0x49144a`). A value beginning `"` (`0x4914bf`) is cut at its
last `"` and set with `0x704300` (`0x4914ef`); one beginning with a digit, or `-` or `.` then a
digit, goes through `atof` and `fstp dword` (`0x491534`) to `0x704250` (`0x491541`). `0x49156a`
signals 0x1ae on every path.

## Saved variables, written

The saved-variables files are written only in the interface's shutdown, after
`PLAYER_LEAVING_WORLD` and `PLAYER_LOGOUT`: the flat file, then each loaded add-on's files.
`AddOns.txt` follows them there, and the glue's `SaveAddOns` writes it too. For each loaded
add-on its account file and character file are rewritten from its declared names, an existing file
first renamed to `.bak`, each name read from the globals with the context word 0; with no names the
file is deleted. The realm-less character file is deleted. A written add-on is then marked not
loaded. The flat file holds the names `RegisterForSave` collected, which raises unless called with
context 0.

Addresses: `0x490bd0`, `0x490c20`, `0x490c2a`, `0x490c7e`, `0x490c83`, `0x490c88`, `0x51f650`, `0x51f711`, `0x51f74b`, `0x51f794`, `0x51f7c5`, `0x51f7e1`, `0x51f7fc`, `0x51f859`, `0x51f8eb`, `0x51f937`, `0x51f94f`, `0x51f9ac`, `0x51f9f7`, `0x51fa03`, `0x491730`, `0x4917a1`, `0x491812`, `0x4884e0`, `0x48851c`, `0x4915d0`, `0x8421a8`
Confidence: verified
Proof: one
Test: none

`0x490bd0` calls `0x490a80` (`0x490c20`), signals `PLAYER_LOGOUT` (`0x490c2a`), then `0x491730`
(`0x490c7e`), `0x51f650` (`0x490c83`) and `0x51ef20` (`0x490c88`). `0x51f650` skips records
with `+0x18` 0 (`0x51f711`); a nonzero `+0x6c` (`0x51f74b`) renames (`0x51f794`), creates
(`0x51f7c5`), stores context 0 (`0x51f7e1`) and serializes each name (`0x51f7fc`); zero deletes
(`0x51f859`). The character file repeats this with `+0x7c` (`0x51f8eb` to `0x51f9ac`); `0x51f9f7`
deletes the realm-less file and `0x51fa03` clears `+0x18`. `0x491730` renames (`0x4917a1`) and
serializes each registered name (`0x491812`); `RegisterForSave` adds a name with `0x4915d0`
(`0x48851c`).

## The saved variables format

Each variable is CR LF, its name, ` = ` and its value; the flat and account files end with CR LF,
a character file with CR alone. Nil, booleans and finite numbers (`%.16g`) are written as such, NaN
and infinities as `nil --[[ nan ]]` and `nil --[[ inf ]]`. Strings are quoted, escaping only `\`,
`"`, LF as `\n` and NUL as `\000`. Functions, userdata and threads are written as `nil` with a
comment.

Addresses: `0x7043f0`, `0x704422`, `0x704448`, `0x704455`, `0x51f80d`, `0x491846`, `0x51f960`, `0x704480`, `0x704a40`, `0x7044e9`, `0x704507`, `0x70452c`, `0x704560`, `0x70457f`, `0x704a5c`, `0x704a6c`, `0x70459e`, `0x7049a9`, `0x7049f0`
Confidence: verified
Proof: one
Test: none

`0x7043f0` writes `0x82edfc` (2 bytes), the name and `0x872ce0`, then calls `0x704480` at depth 1,
building text in `0xceeb60`;
`0x51f80d`, `0x491846` and `0x51f960` pass 2, 2 and 1 bytes for the end. `0x704480` switches on
the type through `0x704a40`; numbers test `_isnan` (`0x7044e9`) and `_finite` (`0x704507`) before
`%.16g` (`0x70452c`); strings open with `"` (`0x704560`), map bytes up to `\` through `0x704a6c`
and `0x704a5c` (`0x70457f`) and flush at 1,016 bytes (`0x70459e`). Functions are named through
`lua_getinfo` (`0x7049a9`); other types take `0x7049f0`.

## Saved tables

A table is written as `{` and CR LF, each entry one tab deeper as `[key] = value,` and CR LF, then
`}` at the parent's indent, in `next` order. A table already open on the current path is
`nil --[[ skipped recursive table ]]`, so a table reached twice by different paths is written
twice. A frame's table is its name, unquoted. Entries keyed by NaN, an infinity, a table, a
function or userdata are skipped with a comment.

Addresses: `0x704607`, `0x704613`, `0x704642`, `0x704670`, `0x7046bc`, `0x7046e7`, `0x704727`, `0x704744`, `0x704766`, `0x70488f`, `0x7048a3`, `0x704928`, `0x70497b`, `0x8722d4`
Confidence: verified
Proof: one
Test: none

A table first reads element 0 as a light userdata and writes `GetName` (`0x704613`, `0x704642`).
Otherwise the light userdata whose reference is `0x8722d4` marks it as a key (`0x704670`,
`0x7046bc`) until `0x70497b` removes the mark on the way out. `0x7046e7` writes the brace; the
indent grows by a tab per depth up to 128 (`0x704727`); `lua_next` walks the entries (`0x704744`).
Keys dispatch at `0x704766`, a light userdata equal to the mark being dropped (`0x70488f`) and
others reaching the skip comment (`0x7048a3`); `0x704928` writes the closing brace.

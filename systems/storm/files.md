# storm: loose files and zip archives

How Storm's file open reaches a file on disk and what it does with an archive name that is not an
MPQ. `SFile::OpenEx` (`storm/bz`) hands a loose name to `0x647db0`, which opens it with the C
runtime in binary mode, through the wide calls on Windows 2000 and later. `SFileOpenArchiveShim`
opens a name ending `.mpq` as an MPQ and any other name as a zip archive, read by `SZip.cpp`; the
archive listing walks a zip's entries.

Roots: `0x647db0`, `0x646e50`, `0x648990`, `0x648dd0`, `0x648fb0`, `0x668b30`

## Opening a loose file

`0x647db0(name, mode, wideMode)` opens a disk file. With Storm option 11 clear it is `fopen(name,
mode)`. With it set, the UTF-8 name is converted to UTF-16 into 260 units; on Windows 2000 or later
(`GetVersionExA`'s major version 5 or more, read once) it is `_wfopen(wide, wideMode)`, else
`WideCharToMultiByte` turns the units into 260 ANSI bytes, `_` for what it cannot map, and `fopen`
opens those; a result of 0 or less fails. Callers pass `"rb"`, `L"rb"`. The conversion's
result is untested: 260 units or more leave the wide buffer unterminated. The count passed back is the
bytes read, not the units written, so a non-ASCII name also converts its NUL and stale units after it,
failing past 260 bytes.

Addresses: `0x647db0`, `0x647db9`, `0x647dc5`, `0x647dc8`, `0x647ddd`, `0x647de2`, `0x647df6`, `0x647e21`, `0x647e29`, `0x647e44`, `0x647844`, `0x647894`, `0x646e50`, `0x646e5a`, `0x646e6b`, `0x646e75`, `0x646e82`, `0x646e90`, `0x646650`, `0x64667d`, `0x646689`, `0x6466b0`, `0xc53f14`, `0xc520a0`, `0xc52414`, `0x8824c0`, `0x4155bb`
Confidence: verified
Proof: one
Test: none

`0x647db9` tests `[0xc53f14]`; `0x647ddd` calls `SUniConvertUTF8to16(buffer, 0x104, name, 0x7fffffff, 0, &count)`, and `0x647de2`
calls `0x646e50` straight after; `count` is the sixth argument, `read`, pushed at `0x647dc5` and `0x647dc8`. `0x647df6` is
`_wfopen` with the stack argument; `0x647e21` calls `[0x8824c0]` with code page 0, flags 0, the units and `count`, a 0x104-byte buffer and the default `_` (`0x8003d8`); `0x647e29 jg` keeps a
positive length, and `0x647e44` is `fopen` with `edx`. `0x647844` and `0x647894` push `0x865bf0` (`L"rb"`) and load
`0x85f398` (`"rb"`). `0x646e50` tests bit 0 of `0xc520a0` (`0x646e5a`); the first call sets it, runs `0x646650` on
`0xc52414` (`0x646e6b`) and registers the empty `0x646e90` (`0x646e75`), and every call returns the byte through
`0x6466b0` (`0x646e82`). `0x646650` calls `GetVersionExA` on a 0x94-byte block (`0x64667d`) and stores
`major >= 5` (`0x646689`, `cmp 5; sbb; inc`), 0 when the call fails. `0x8824c0` starts at `0x4155bb`, which binds
`WideCharToMultiByte` from `kernel32.dll` on the first call.

## The data path accessor

`0x648990` returns the address of the data path `0xc520c8`, `Data\` after start-up. The macro icon
list formats it in front of its folder prefix.

Addresses: `0x648990`, `0x4f00c3`, `0xc520c8`
Confidence: verified
Proof: one
Test: none

The body is `mov eax,0xc520c8; ret`. Its one caller, `0x4f00c3` in `BuildMacroIconList`, passes the result with the
prefix to `SStrPrintf` and the format `%s%s` (`0x82e424`).

## Zip archive listing

`SFileOpenArchiveShim` makes an 8-byte record: type 0 and an MPQ handle for a `.mpq` name, any case,
else type 1 and a zip handle from `0x668360`, which `fopen`s the file and reads its central directory,
found by the first end record `PK\5\6` in the last 0x10016 bytes. Each record whose local header
matches it and whose method is 0 or 8 becomes an entry of a table Storm keeps for every zip, the zip at
`+0x18` and the name, `/` turned to `\`, at `+0x1c`; another record fails the open. The listing
`0x648fb0` sends type 1 to `0x668b30(zip, callback, data)`, which calls `callback(name, data)` for each
entry of that zip in the order read until a call returns 0.

Addresses: `0x648dd0`, `0x648e38`, `0x648e42`, `0x648fb0`, `0x648fc3`, `0x648fd4`, `0x668360`, `0x6683de`, `0x6683ed`, `0x667bd0`, `0x667c10`, `0x667c54`, `0x667c96`, `0x667da0`, `0x667e20`, `0x667ed6`, `0x667f21`, `0x667f51`, `0x668076`, `0x6680d9`, `0x667e00`, `0x668197`, `0x66819a`, `0x668247`, `0x668b30`, `0x668b5c`, `0x668b66`, `0x668b72`, `0xc5a334`, `0xc5a338`, `0xc5a340`, `0x80fd6c`
Confidence: verified
Proof: one
Test: none

`0x648e0f` compares the extension with `.mpq` by `SStrCmpI`; no extension or another goes to `0x648e38`, which stores
type 1 and the result of `0x668360` (`0x648e42`) at `+4`. `0x648fc3` dispatches on the type and `0x648fd4` passes `+4`.
`0x668360` allocates 0x110 bytes, opens with `0x667bd0` (`fopen` with `"rb"`), finds the end record with `0x667c10`
(`0x667c54` caps the search at 0x10016 bytes from the end; `0x667c96` matches `0x80fd6c`) and reads the directory
through `0x667da0`, which runs `0x667e20` once per record the end record counts. `0x667e20` reads the 46-byte
record and tests `PK\1\2` (`0x667ed6`), reads and slashes the name (`0x667f21`), keeps method 0 or 8 (`0x667f51`),
reads the local header and tests `PK\3\4` (`0x668076`), compares its name (`0x6680d9`), stores the zip at `0x668197` and copies the name at
`0x66819a`, and links the entry into the list `0xc5a338` with mode 2, the tail (`0x668247`, the same link as
`0x56d0e0` with the list's link offset). `0x667840` builds the table `0xc5a334` with that list's link offset 0xc and
its head at `0xc5a340`. `0x668b30` walks from the head, compares `+0x18` with the zip (`0x668b5c`), calls the callback
with `ecx` the name and `edx` the data (`0x668b66`) and follows `[entry + 0x10]` (`0x668b72`); it returns 1 (`0x668b85`).

# storm: text logs

Storm's text logs, which the client writes `Logs\FrameXML.log`, `Logs\GlueXML.log` and its other
logs through. `SLogCreate` makes a log record, `SLogVWrite` and `SLogWrite` add a line, and the
file behind a record opens at its first line. A record is `0x10124` bytes from `VirtualAlloc`: its
id at `+0`, the path at `+8`, the file handle at `+0x10c` (−1 until opened), the create flags at
`+0x110`, the write position at `+0x114`, the position last sent to the debugger at `+0x118`, the
indent at `+0x11c`, a time-stamp flag at `+0x120` and a 0x10000-byte line buffer from `+0x124`.

Roots: `0x65a0c0`, `0x65ac20`, `0x65ace0`

## Opening a log file

`SLogCreate(name, flags, &id)` keeps the path and the flags and opens nothing unless flag bit 0 is
set; flag bit 1 replaces the name with an empty string and clears bit 0. A record whose handle is −1
opens its file before its first line is written, through `0x65a1c0(path, &handle, flags)`: a null or
empty path stores −1 and fails; otherwise the path's folders are made and `CreateFileA` opens it for
writing, shared for reading and writing, as `OPEN_ALWAYS` at the end of the file when flag bit 2 is
set and as `CREATE_ALWAYS`, emptied, when it is clear. A failed open clears the path's first byte, so
the record never tries again.

Addresses: `0x65a0c0`, `0x65a0dc`, `0x65a0f4`, `0x65a104`, `0x65a1c0`, `0x65a1ce`, `0x65a1e3`, `0x65a1ed`, `0x65a1f5`, `0x65a208`, `0x65a217`, `0x65a238`, `0x65a930`, `0x65a940`, `0x65a94e`, `0x65a957`, `0x659f70`, `0x65a3e0`, `0x65a260`
Confidence: verified
Proof: one
Test: none

`0x65a0dc` tests bit 1 and stores the empty string `0x882748`; `0x65a0f4` tests bit 0 and `0x65a104` opens at once. The
record comes from `0x659f70(id, 1)`, whose `VirtualAlloc` asks for `0x10124` bytes (`0x659ffa`) and sets the handle to −1.
`0x65a1ce` and `0x65a1d3` refuse the path; `0x65a3e0` puts a path with neither `:` second nor a backslash in the log
folder (not read further); `0x65a1ed` makes `(flags >> 2) & 1`; `0x65a1f5` calls `0x65a260`, which makes the folders.
`0x65a208` turns the bit into the disposition, `lea eax,[eax+eax*1+0x2]`: 2 or 4. `0x65a217` calls `CreateFileA` with
`0x40000000` and share 3; `0x65a238` calls `SetFilePointer(handle, 0, 0, 2)` for bit 2. `PrepareLog` (`0x65a930`) skips
an open handle (`0x65a940`), opens with the path at `+8` and the flags at `+0x110` (`0x65a94e`), and on failure stores
0 in `+8` (`0x65a957`).

## Writing a log line

`SLogVWrite(id, format, args)` finds the record, opens its file if it has to, and builds the line in
the buffer: the time stamp, the indent, the text from `vsprintf`, and `"\r\n"`. The stamp is
`"%u/%u %02u:%02u:%02u.%03u  "` of month, day, hour, minute, second and millisecond from
`GetLocalTime`, made again only when `GetTickCount` has changed since the last stamp of any log. The
indent is `+0x11c` spaces, at most 128. Storm option 5 sends the line to `OutputDebugStringA`. A
buffer at `0xc000` bytes or more is flushed. Nothing bounds the text: a line that carries the position
past `0x10000` writes past the record.

Addresses: `0x65ac20`, `0x65ac2c`, `0x65ac3d`, `0x65ac51`, `0x65ac58`, `0x65ac73`, `0x65ac89`, `0x65ac96`, `0x65aca0`, `0x65acb2`, `0x65acbe`, `0x65accd`, `0x65a840`, `0x65a854`, `0x65a85a`, `0x65a860`, `0x65a871`, `0x65a89f`, `0x65a8ce`, `0x65a8dd`, `0x65a8f4`, `0x65a7b0`, `0x65a7bd`, `0x65a810`, `0x659f10`, `0x866aa0`, `0x82edfc`, `0xc53f04`
Confidence: verified
Proof: one
Test: none

`0x65ac2c` finds the record, `0x65ac3d` calls `PrepareLog` and skips the line when it fails. `OutputTime` (`0x65a840`) runs
with `edx` 1 (`0x65ac51`); it writes nothing when `+0x120` is 0 (`0x65a854`; `SLogCreate` stores 1 at `0x65a174`). It
compares `GetTickCount` (`0x65a85a`) with the cached tick `0xc5a174` (`0x65a860`) and, on a change, formats
`SYSTEMTIME` words 1, 3, 4, 5, 6 and 7 with `wsprintfA` into `0xc5a100` (`0x65a871`, `0x65a89f`); `edx` 1 copies the
stamp (`0x65a8dd`), 0 writes as many spaces (`0x65a8f4`). `OutputIndent` (`0x65a7b0`) caps the indent at `0x80`
(`0x65a7bd`). `0x65ac73` calls `_vsprintf` at `+0x124 + position`, `0x65ac89` adds its `SStrLen`, and `OutputReturn`
(`0x65a810`) copies `"\r\n"` and its NUL from `0x82edfc`. `0x65aca0` tests option 5 and `0x65acb2` calls
`OutputDebugStringA`; `0x65acbe` compares with `0xc000` and `0x65accd` calls `FlushLog` (`0x659f10`).

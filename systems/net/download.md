# net: download

The file a patch download writes: a `.partial` file that ends in a 40-byte trailer, so a broken download resumes, and an MD5 check when the last byte arrives.

The code sits in the net run after the account data code. The glue screens drive it (`0x473c4e` to `0x474c71`); the file objects come from Storm's file layer (`0x662910`), and their slots read, write, give and set the size.

Roots: `0x5b03f0`, `0x5b0440`, `0x5b04d0`, `0x5b0550`, `0x5b05f0`, `0x5b0ab0`, `0x5b0b20`, `0x5b0b60`, `0x5b0bc0`, `0x5b0ce0`

## Download file object

`0x5b03f0` clears the object: `+0` the file object, `+0x134` whether the file is the `.partial`, `+0x138` the state, `+0x13c` the error and `+0x144` the bytes fed, then writes the kept trailer at `+8`: the magic `DInf` (`0x44496e66`), a zero dword, the expected MD5 at `+0x10`, the bytes received at `+0x20` and the total at `+0x28`. The final path is at `+0x30`, `0x104` bytes. The state is 2 when complete and 3 when failed, with error 1 for a write or create, 2 for a resize, which stores the MiB it wanted at `+0x148`, and 3 for an abort.

Addresses: `0x5b03f0`, `0x5b0418`, `0x5b0426`, `0x5b0b60`, `0x5b0b20`, `0x5b0859`
Confidence: verified
Proof: one
Test: `net::download_feed`, `net::download_open`

`0x5b03fe` to `0x5b0418` zero `+0`, `+0x134`, `+0x138`, `+0x13c`, `+0x140` and `+0x144`; `0x5b0423` zeroes the ten trailer dwords and `0x5b0426` stores the magic. `0x5b0b60` returns `+0x138`. `0x5b0b20` stores 3 and error 3 under the lock `0xc2a1a0`. `0x5b0859` stores the MiB count.

## Opening a download

`0x5b05f0(path, digest, total, &offset)` releases the file it held, keeps the total and the MD5, copies the path, and sets the offset to 0. An existing `.partial` whose trailer reads resumes: the offset is the bytes received and it returns 1, or, when complete, sets state 2 and returns 0. Otherwise an existing final file that is complete sets state 2, gives the offset `+0x20` already held, and returns 0. Otherwise it creates the `.partial`, sized to the total's low dword plus a trailer, and returns 1; a failed create or resize sets state 3 and returns 0.

Addresses: `0x5b05f0`, `0x5b0600`, `0x5b0669`, `0x5b06a8`, `0x5b06bc`, `0x5b06ea`, `0x5b0770`, `0x5b07ea`, `0x5b082a`, `0x5b083c`, `0x85dee4`
Confidence: verified
Proof: one
Test: `net::download_open`

`0x5b0600` calls `0x5b0550`. `0x5b0669` formats `%s.partial` (`0x85dee4`); `0x5b06a8` opens it with access `0xc0000000` and disposition 3 once `0x42a4e0` finds it. `0x5b06bc` reads the trailer; on a complete one `0x5b06ea` returns `al` 0. On failure the file object is released (`0x5b070a` to `0x5b0729`). `0x5b0770` tests the final file, which is released either way, and `0x5b078b` clears the result. `0x5b07ea` creates with disposition 2; `0x5b082a` is slot 5 with the low dword plus `0x28` and a high dword of 0; `0x5b083c` shifts the 64-bit size plus `0x28` right by 20 and adds 1.

## Download trailer

`0x5b09b0` accepts a file one trailer longer than the total when its last 40 bytes hold the magic, a zero dword, the kept MD5 and the kept total. It keeps that trailer, and calls it complete when the trailer says every byte arrived and the data's MD5 matches; a mismatch then fails. A file exactly the total long is accepted, and complete, when its MD5 matches (`0x5b0970`). `0x5b0b70` writes the kept trailer at the total's low dword; a failed write sets state 3, error 1. `0x5b0880` hashes the file in 32 KiB reads, or gives zeros when a read fails, over `start` to `end - start`: the start is subtracted twice, harmless since every caller passes 0.

Addresses: `0x5b09b0`, `0x5b09d7`, `0x5b0a06`, `0x5b0a11`, `0x5b0a32`, `0x5b0a46`, `0x5b0a6e`, `0x5b0a85`, `0x5b0970`, `0x5b0920`, `0x5b0b70`, `0x5b0880`, `0x5b089d`, `0x5b08a9`, `0x5b08f9`
Confidence: verified
Proof: one
Test: `net::download_read_trailer`, `net::download_digest_range`, `net::download_feed`

`0x5b09d7` adds `0x28` to the total with carry; `0x5b0a06` reads 40 bytes at the size less `0x28`. `0x5b0a11` tests the magic, `0x5b0a21` the zero dword, `0x5b0a32` the MD5 and `0x5b0a39` the total; `0x5b0a46` compares the trailer's count with its total and calls `0x5b0920`, and `0x5b0a6e` copies the ten dwords to `+8`. `0x5b0a85` handles a file of exactly the total. In `0x5b0880`, `0x5b089d` takes `end - start` as the bound and the loop starts at `start`; `0x5b08a9` caps a read at `0x8000` and passes a high dword of 0, and `0x5b08f9` zeroes the digest on a failed read. The callers `0x5b0942` and `0x5b0c90` push a start of 0.

## Feeding a download

`0x5b0bc0(data, size)` writes the data at the received count's low dword, then adds the size to the count and to `+0x144`. Past the total it sets state 3, error 1; otherwise, at the total or once `+0x144` reaches 1 MiB, it rewrites the trailer. Nothing but `0x5b03f0` resets `+0x144`, so after the first MiB every feed rewrites it. At the total the MD5 sets state 2 or 3. Without a file, or when the write fails, it returns 0 and sets state 3; else 1. `0x5b0ce0` returns the count over the total as a float, both read as unsigned 64-bit at the x87's 53-bit precision, or 0 for a total of 0.

Addresses: `0x5b0bc0`, `0x5b0bfa`, `0x5b0c1e`, `0x5b0c40`, `0x5b0c56`, `0x5b0c90`, `0x5b0ca6`, `0x5b0ccd`, `0x5b0ce0`, `0x5b0d09`, `0x5b0d4b`, `0x5b0d51`
Confidence: verified
Proof: one
Test: `net::download_feed`, `net::download_progress`

`0x5b0bfa` is slot 2 with the count's low dword and a high dword of 0. `0x5b0c1e` adds to `+0x144`, which only `0x5b0418` writes besides; `0x5b0c40` stores 3 and 1; `0x5b0c56` compares `+0x144` with 1 MiB and the count with the total before `0x5b0b70`. `0x5b0c90` hashes from 0 to the total and `0x5b0ca6` stores 2 on a match. `0x5b0ccd` stores 3 when the result is 0. In `0x5b0ce0`, `0x5b0d09` to `0x5b0d43` load each value as its low 63 bits plus the top bit's `2^63`, `0x5b0d4b` divides, and `0x5b0d51` loads the 0.0 at `0x7ffd74`.

## Releasing a download

`0x5b0450(delete, commit)` takes the lock `0xc2a1a0`, drops one reference from the file object and from each object its `+4` chains to (`0x5b04d0`), deleting each whose count `+8` reaches 0 through slot 0 with 1, and clears `+0`. It then deletes the file (`0x5b04f0`), or with `commit`, for a `.partial`, renames it to the final path (`0x5b0570`), which sets state 3, error 1 when the rename fails. `0x5b0550(delete)` is the release without a commit and `0x5b0440` without either.

Addresses: `0x5b0450`, `0x5b0466`, `0x5b047f`, `0x5b0494`, `0x5b04b1`, `0x5b04d0`, `0x5b04d4`, `0x5b04e1`, `0x5b0550`, `0x5b0440`, `0x5b04f0`, `0x5b0570`, `0x5b05be`
Confidence: verified
Proof: one
Test: `net::download_release`, `net::download_release_chain`

`0x5b0466` to `0x5b0477` decrement the file object's `+8` and call `[[ecx]]` with 1 at zero; `0x5b047f` passes its `+4` to `0x5b04d0`, which reads the next link at `0x5b04d4` before the release at `0x5b04e1`. `0x5b0494` and `0x5b04b1` are the two paths. `0x5b0570` builds the `.partial` name, deletes an existing final file, and moves with `0x42a820` (`0x5b05be`).

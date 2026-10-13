#!/usr/bin/env python3
"""tables/dbc_stores.tsv: the client's database stores, one per DBC file.

StaticDBLoadAll (0x53f8b0) first loads five stores inline, each block ending in

    mov dword [<store> + 0x10], 1

the store's loaded flag; such a store's loader is StaticDBLoadAll and its load site that store,
and the block's file name comes from the first callee that names one. From 0x540321 to 0x540f00
the sequence is a flat run of

    mov ecx, <store>; call <loader>

ecx is the thiscall `this`, so each store is paired with the loader it precedes. A loader
reaches its file name through a thunk; the generator scans the loader's first 0x100 bytes and the
same span of each direct callee within its first 0x260 bytes for a dword that points at a
`DBFilesClient\\...` string in the image, and reads the string.

A store is 0x14 bytes, fixed by every store's static initializer and by the WowClientDB<T>::Load
template body:

    +0x00  records   +0x04  count   +0x08  idmap   +0x0c  maxrow   +0x10  loaded

maxrow is rowCount-1 and -1 until the store loads, which is why consumers guard it with a signed
compare. idmap[k] is the file's k-th row, not the row whose id column is k; the two coincide only
for a file whose ids run 0..n-1 in row order. Load writes the loop counter into an appended
dword after each record, so the in-memory stride is the record size plus four.
"""

import bisect
import struct
import sys

from lib import Disasm, Image, addr, note, parse_args, write_table

LOAD_ALL = 0x53F8B0
LOAD_SEQ = (0x540321, 0x540F00)


def main():
    args = parse_args(__doc__, disasm=True)
    img, dis = Image(args.exe), Disasm(args.disasm)
    addrs = [t[0] for t in dis.ins]

    def dbc_name_at(va):
        if not (0x400000 < va < 0xD00000):
            return None
        s = img.bytes(va, 96)
        if s.startswith(b'DBFilesClient\\'):
            return s[:s.index(b'\0')].decode('latin1')
        return None

    def direct_calls(fn, span=0x260):
        lo = bisect.bisect_left(addrs, fn)
        out = []
        for a, _, mn, ops in dis.ins[lo:]:
            if a >= fn + span:
                break
            if mn == 'call' and ops.startswith('0x'):
                out.append(int(ops, 16))
        return out

    def filename_of(loader):
        for fn in [loader] + direct_calls(loader):
            for off in range(0x100):
                n = dbc_name_at(struct.unpack_from('<I', img.mem, fn - img.base + off)[0])
                if n:
                    return n
        return None

    pairs, pend = [], None
    block = []
    lo = bisect.bisect_left(addrs, LOAD_ALL)
    for a, _, mn, ops in dis.ins[lo:]:
        if a >= LOAD_SEQ[0]:
            break
        if mn == 'call' and ops.startswith('0x'):
            block.append(int(ops, 16))
        elif mn == 'mov' and ops.startswith('DWORD PTR ds:0x') and ops.endswith(',0x1'):
            flag = int(ops.split('ds:')[1].split(',')[0], 16)
            name = next((n for n in map(filename_of, block) if n), None)
            pairs.append((flag - 0x10, LOAD_ALL, a, name))
            block = []

    lo = bisect.bisect_left(addrs, LOAD_SEQ[0])
    for a, _, mn, ops in dis.ins[lo:]:
        if a >= LOAD_SEQ[1]:
            break
        if mn == 'mov' and ops.startswith('ecx,0x'):
            pend = int(ops.split('0x')[1], 16)
        elif mn == 'call' and pend is not None:
            if ops.startswith('0x'):
                pairs.append((pend, int(ops, 16), a, None))
            pend = None
        elif mn in ('call', 'jmp'):
            pend = None

    seen, rows, unnamed = set(), [], 0
    for store, loader, site, name in pairs:
        if store in seen:
            continue
        seen.add(store)
        name = name or filename_of(loader)
        unnamed += name is None
        rows.append((addr(store), addr(loader), addr(site), name or '?'))
    rows.sort()
    note('%d stores, %d without a file name' % (len(rows), unnamed))
    write_table(['store', 'loader', 'load_site', 'file'], rows, args.out)
    return 1 if unnamed else 0


if __name__ == '__main__':
    sys.exit(main())

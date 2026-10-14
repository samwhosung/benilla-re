#!/usr/bin/env python3
"""Function clones: the function rows whose bytes are identical once the addresses in them are
masked.

    tools/clones.py [ADDRESS ...] [--system NAME] [--min-size N]

Each row's body is its extent in symbols/functions.tsv. Inside it, the bytes that hold an address
are masked and everything else is kept as it is:

  - the rel32 of a `call`, `jmp` or conditional jump to an absolute target (a clone at another
    address, or one calling another function of the same shape, differs only there);
  - every four-byte operand whose value is an address in the image (a global, a string, a vtable,
    a jump table), found by the value the listing prints in the instruction's operands;
  - in the bytes of the extent the listing leaves out (a jump table), every aligned dword that is
    an address in the image.

Opcodes, ModRM bytes, short displacements and immediates, and four-byte constants that are not
addresses stay as they are, so a function that reads another field or another constant is not a
clone. Two rows are clones when their masked bodies are the same length and the same bytes.

A cluster is printed with its count, its size, `raw` when the unmasked bytes are identical too,
and each member with its row. A masked clone can call a different function or read a different
global from its siblings: the shape is shared, each target is read from the member's own bytes.
A function whose shape no other row shares is in no cluster, so this finds shared shapes; it
proves nothing about a function it does not list.

With addresses, only the clusters that hold one of them are printed (and a line for each address
in none). `--system` keeps the rows of that system; `--min-size` drops bodies smaller than N
bytes (default 1). Options: --exe PATH, --disasm PATH.
"""

import hashlib
import re
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / 'tables'))
from lib import Disasm, Functions, Image, addr, parse_args  # noqa: E402

HEX = re.compile(r'0x([0-9a-f]+)')
MASK = b'\xee\xee\xee\xee'


def masked(img, dis, row):
    start, size = row['address'], row['size']
    raw = img.bytes(start, size)
    out = bytearray(raw)
    in_image = img.in_image
    covered = bytearray(size)
    k = dis.at.get(start)
    while k is not None and k < len(dis.ins):
        a, n, mn, ops = dis.ins[k]
        if a >= start + size:
            break
        lo, hi = a - start, min(a - start + n, size)
        for i in range(lo, hi):
            covered[i] = 1
        body = raw[lo:hi]
        values = [int(v, 16) for v in HEX.findall(ops)]
        if (mn in ('call', 'jmp') or mn.startswith('j')) and values and len(body) >= 5:
            target = values[0]
            for j in range(1, len(body) - 3):
                rel = struct.unpack_from('<i', body, j)[0]
                if (a + j + 4 + rel) & 0xFFFFFFFF == target:
                    out[lo + j:lo + j + 4] = MASK
                    break
        for v in values:
            if in_image(v):
                needle = struct.pack('<I', v)
                j = body.find(needle)
                while j >= 0:
                    out[lo + j:lo + j + 4] = MASK
                    j = body.find(needle, j + 1)
        k += 1
    for i in range(0, size - 3):
        if not covered[i] and (start + i) % 4 == 0:
            if in_image(struct.unpack_from('<I', raw, i)[0]):
                out[i:i + 4] = MASK
    return raw, bytes(out)


def main():
    def extra(ap):
        ap.add_argument('addresses', nargs='*')
        ap.add_argument('--system')
        ap.add_argument('--min-size', type=int, default=1)
    args = parse_args(__doc__, disasm=True, extra=extra)
    img, dis, rows = Image(args.exe), Disasm(args.disasm), Functions()
    chosen = [r for r in rows.rows if r['size'] >= args.min_size
              and (args.system is None or r['system'] == args.system)]
    clusters, raws = {}, {}
    for r in chosen:
        raw, body = masked(img, dis, r)
        raws[r['address']] = raw
        key = (len(body), hashlib.sha1(body).hexdigest())
        clusters.setdefault(key, []).append(r)
    multi = sorted((v for v in clusters.values() if len(v) >= 2),
                   key=lambda v: (-v[0]['size'], -len(v), v[0]['address']))
    wanted = {int(a, 16) for a in args.addresses}
    shown = set()
    for i, members in enumerate(multi):
        here = {m['address'] for m in members}
        if wanted and not (wanted & here):
            continue
        shown |= here
        raw = all(raws[m['address']] == raws[members[0]['address']] for m in members[1:])
        print('cluster %d: %d functions of %d bytes%s' % (i, len(members), members[0]['size'],
                                                         ', raw' if raw else ''))
        for m in members:
            print('  %s' % Functions.label(m))
    for a in sorted(wanted - shown):
        print('%s is in no cluster%s' % (addr(a), '' if a in rows.at else ' (no function row)'))
    print('%d rows, %d clusters of two or more holding %d rows' % (
        len(chosen), len(multi), sum(len(v) for v in multi)), file=sys.stderr)
    return 0


if __name__ == '__main__':
    sys.exit(main())

#!/usr/bin/env python3
"""tables/dbc_schemas.tsv: the record layout of every client database the client reads,
as each record type's Read function schedules it.

Each DBC record type T has a `T::Read(ctx, stringBlockBase)` that reads the record column by
column through the typed reader 0x648460 (fastcall-style stack call: ctx, destination, size, and
three zeros pushed right to left). A function is found by its error message, the string
"Error reading <T>Rec" that it loads before reporting a failed read. From the call schedule:

    record_size   the sum of the read sizes, in bytes
    field_count   the sum of size/4 for a read of four bytes or more, 1 for a smaller read
                  (a few tables pack byte columns)
    string_cols   the field indices read into a stack temporary rather than into the record.
                  Those are string-block offsets: on success each is rewritten as
                  stringBlockBase + offset (the loop of `add reg, base` after the error check,
                  where base is the Read function's second argument [ebp+0xc]). Read treats
                  a localized string's eight columns like any other, so the list does not
                  mark one; tables/README.md gives the runs of eight that are not one.
    fixups        the count of those adds, an independent count of the same columns

The two counts agree on all 151 tables. The scalar columns are untyped four-byte cells here: whether
one is an integer or a float is the consumer's, not Read's.
"""

import bisect
import re
import sys

from lib import Disasm, Image, addr, note, parse_args, write_table

READER = 0x648460
BASE_ARG = 'DWORD PTR [ebp+0xc]'


def main():
    args = parse_args(__doc__, disasm=True)
    img, dis = Image(args.exe), Disasm(args.disasm)
    ins = dis.ins
    mem = bytes(img.mem)

    types = []
    for m in re.finditer(rb'Error reading ([A-Za-z0-9_]+Rec)\x00', mem):
        types.append((m.group(1).decode(), img.base + m.start()))

    # instruction index of each load of an error string: an instruction with that immediate
    want = {va: name for name, va in types}
    loads = {}
    for k, (a, _, mn, ops) in enumerate(ins):
        mm = re.search(r',(0x[0-9a-f]+)$', ops) if mn in ('mov', 'push') else None
        if mn == 'push':
            mm = re.fullmatch(r'(0x[0-9a-f]+)', ops)
        if mm:
            v = int(mm.group(1), 16)
            if v in want:
                loads.setdefault(want[v], []).append(k)

    rows, warn = [], 0
    for name, va in sorted(types):
        ks = loads.get(name)
        if not ks or len(ks) != 1:
            note('%s: %s loads of the error string' % (name, len(ks or [])))
            warn += 1
            continue
        k = ks[0]
        j = k
        while j > 0 and not (ins[j][2] == 'push' and ins[j][3] == 'ebp'
                             and ins[j + 1][3].startswith('ebp,esp')):
            j -= 1
        reads = []        # (size, is_temp)
        for c in range(j, k):
            if ins[c][2] == 'call' and ins[c][3] == '0x%x' % READER:
                pushes = []
                for p in range(c - 1, max(j, c - 20), -1):
                    if ins[p][2] == 'push':
                        pushes.append((p, ins[p][3]))
                        if len(pushes) == 3:
                            break
                (_, size), (pd, dest) = pushes[2], pushes[1]
                size = int(size, 16)
                temp = False
                if dest in ('eax', 'ecx', 'edx', 'ebx', 'esi', 'edi'):
                    for q in range(pd - 1, max(j, pd - 8), -1):
                        m = re.match(r'%s,\[(\w+)([+-]0x[0-9a-f]+)?\]$' % dest, ins[q][3]) \
                            if ins[q][2] == 'lea' else None
                        if m:
                            temp = m.group(1) == 'ebp'
                            break
                reads.append((size, temp))
        record_size = sum(s for s, _ in reads)
        field_count = sum(s // 4 if s >= 4 else 1 for s, _ in reads)
        cols, f = [], 0
        for s, temp in reads:
            if temp:
                cols.append(f)
            f += s // 4 if s >= 4 else 1
        # the fixup adds: after the error check, the first load of the base argument, then
        # every `add X, <that register>` up to the first return
        fixups = 0
        reg = None
        for n, (a, _, mn, ops) in enumerate(ins[k:k + 400]):
            if n and mn == 'push' and ops == 'ebp' and ins[k + n + 1][3].startswith('ebp,esp'):
                break                       # the next function
            if reg is None:
                m = re.fullmatch(r'(e\w\w),%s' % re.escape(BASE_ARG), ops)
                if mn == 'mov' and m:
                    reg = m.group(1)
                continue
            if mn.startswith('ret'):
                break
            m = re.fullmatch(r'.+,(e\w\w)', ops)
            if mn == 'add' and m and m.group(1) == reg:
                fixups += 1
        rows.append((name[:-3], field_count, record_size,
                     ','.join(str(c) for c in cols), fixups))
    rows.sort(key=lambda r: r[0])
    note('%d record types, %d warnings' % (len(rows), warn))
    write_table(['file', 'field_count', 'record_size', 'string_cols', 'fixups'], rows, args.out)
    return 1 if warn else 0


if __name__ == '__main__':
    sys.exit(main())

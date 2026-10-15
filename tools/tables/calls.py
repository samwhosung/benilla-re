#!/usr/bin/env python3
"""tables/calls.tsv: the direct calls and tail jumps out of each function.

A function row's extent is [address, address + size). Each `call`, `jmp` or conditional jump in
that extent whose operand is a bare address, and whose address is another function row's start,
is one callee. A `jmp` to a row's start is a tail call. The generator lists each function's
callees once, in address order, joined with `|`, and skips a function with none. A call through a
register or a memory slot, and a row reached only by a stored address, are not in the table. The
callers of a function are the rows whose list holds it.
"""

import re

from lib import Disasm, Functions, addr, note, parse_args, write_table

TARGET = re.compile(r'0x[0-9a-f]+')


def main():
    args = parse_args(__doc__, disasm=True)
    dis, fns = Disasm(args.disasm), Functions()
    rows, edges = [], 0
    for row in fns.rows:
        start, end = row['address'], row['address'] + row['size']
        k = dis.at.get(start)
        found = set()
        while k is not None and k < len(dis.ins) and dis.ins[k][0] < end:
            _, _, mn, ops = dis.ins[k]
            k += 1
            if (mn in ('call', 'jmp') or mn.startswith('j')) and TARGET.fullmatch(ops):
                t = int(ops, 16)
                if t in fns.at and t != start:
                    found.add(t)
        if found:
            rows.append([addr(start), '|'.join(addr(t) for t in sorted(found))])
            edges += len(found)
    note('%d functions with callees, %d edges' % (len(rows), edges))
    write_table(['function', 'callees'], rows, args.out)


main()

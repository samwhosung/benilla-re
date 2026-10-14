#!/usr/bin/env python3
"""tables/unit_gates.tsv: how each of the 83 unit functions checks its arguments.

The table at 0x850438 holds 83 name and function pairs, the Unit* functions and a few player
state readers. A function that checks an argument calls `lua_isstring` (0x6f3510) or
`lua_isnumber` (0x6f34d0) on its stack index (`mov edx,N` just before) and, when the check fails,
`luaL_error` (0x6f4940) with a `Usage:` string; luaL_error does not return. A function with no such
call takes a missing, nil, boolean or table argument without an error: a unit token argument then
reads as NULL, which the token resolver (0x515970) turns into GUID 0. The generator reads the
instructions of each function (its extent is the function row's size) and lists the checked
arguments, `1:string` for `lua_isstring` and `1:number` for `lua_isnumber`, and every `Usage:` string
the function pushes. `lua_isstring` accepts a number as well as a string, so a number passes the
check and then fails in the resolver as an unknown unit name.
"""

import csv

from lib import Disasm, Image, ROOT, addr, note, parse_args, write_table

TABLE, COUNT = 0x850438, 83
ISSTRING, ISNUMBER, ERROR = 0x6F3510, 0x6F34D0, 0x6F4940


def sizes():
    out = {}
    with open(ROOT / 'symbols' / 'functions.tsv', newline='') as f:
        for row in csv.DictReader(f, delimiter='\t'):
            out[int(row['address'], 16)] = int(row['size'] or 0)
    return out


def main():
    args = parse_args(__doc__, disasm=True)
    img, dis = Image(args.exe), Disasm(args.disasm)
    size = sizes()
    rows = []
    for i in range(COUNT):
        name = img.cstr(img.u32(TABLE + 8 * i), 48)
        fn = img.u32(TABLE + 8 * i + 4)
        end = fn + size.get(fn, 0)
        k = dis.at[fn]
        gates, usage = [], []
        last_edx = None
        while k < len(dis.ins) and dis.ins[k][0] < end:
            a, sz, mn, ops = dis.ins[k]
            if mn == 'mov' and ops.startswith('edx,0x'):
                last_edx = int(ops[4:], 16)
            elif mn == 'call' and ops.startswith('0x'):
                t = int(ops, 16)
                if t in (ISSTRING, ISNUMBER) and last_edx is not None:
                    g = '%d:%s' % (last_edx, 'string' if t == ISSTRING else 'number')
                    if g not in gates:
                        gates.append(g)
            elif mn == 'push' and ops.startswith('0x8'):
                s = img.cstr(int(ops, 16), 96)
                if s and s.startswith('Usage:') and s not in usage:
                    usage.append(s)
            k += 1
        rows.append([name, addr(fn), ','.join(gates), ' | '.join(usage)])
    note('%d unit functions, %d with a checked argument' % (len(rows), sum(bool(r[2]) for r in rows)))
    write_table(['name', 'fn', 'checked', 'usage'], rows, args.out)


if __name__ == '__main__':
    main()

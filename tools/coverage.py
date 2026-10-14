#!/usr/bin/env python3
"""Which instructions of each function the bit-exact tests ran, and which they never reached.

    BENILLA_RE_COVERAGE_OUT=$PWD/dumps/coverage.txt cargo test --workspace   (or -p a crate)
    tools/coverage.py dumps/coverage.txt [ADDRESS ...] [--system NAME] [--all]

The path is absolute because each test runs in its crate's directory. The harness appends, so
remove the file before a fresh run.

With BENILLA_RE_COVERAGE_OUT set, the harness (crates/difftest) appends the address of every
block the emulator runs, once, and for code a test copies into scratch memory it records the
.text addresses those bytes came from. A test of a slice says which instructions it covers
(METHOD.md, The harness); this reads that record back against the listing.

An instruction ran when the record holds an address inside it, or when an instruction before it
in the same function ran and nothing between ends a block: the emulator runs a block from its
first instruction through the first jump, call or return, so each later instruction of the block
ran too. A call's return lands at a new block, which the record holds when it ran.

For each function it prints how many of its instructions ran and each run of instructions none
of the tests reached, with its first instruction and how many of its instructions compute
(arithmetic, logic, shifts, floating point; the census's sense of `primitive`). A function the
record does not touch at all is `not run`, which says that no test of this run reached it, not
that none exists. By default it reports the rows whose `test` column names a test; with
addresses or `--system`, those rows; `--all` lists the fully covered rows too. Option:
--disasm PATH.
"""

import bisect
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / 'tables'))
from lib import Disasm, Functions, addr, parse_args  # noqa: E402

ENDS = re.compile(r'^(j[a-z]+|call|ret|retn|retf|iret|iretd|int|int3|into|loop[a-z]*|hlt|ud2|'
                  r'syscall|sysenter)$')
INTEGER = {'add', 'adc', 'sub', 'sbb', 'imul', 'mul', 'idiv', 'div', 'neg', 'inc', 'dec', 'and',
           'or', 'xor', 'not', 'shl', 'shr', 'sal', 'sar', 'shld', 'shrd', 'rol', 'ror', 'rcl', 'rcr',
           'bswap'}
X87_NOT_COMPUTING = ('fld', 'fst', 'fxch', 'fild', 'fist', 'fbld', 'fbst', 'fcom', 'fucom', 'ficom',
                     'ftst', 'fxam', 'ffree', 'finit', 'fninit', 'fwait', 'fnop', 'fclex', 'fnclex',
                     'fnst', 'fincstp', 'fdecstp', 'fcmov', 'fnsave', 'frstor', 'fldenv')
SSE = re.compile(r'^((add|sub|mul|div|sqrt|min|max|rcp|rsqrt|and|andn|or|xor)[sp][sd]|cvt\w+|'
                 r'p(add|sub|mul|and|or|xor|sll|srl|sra)\w*|pf\w+)$')


def computes(mn):
    """Whether an instruction computes on values: integer arithmetic, logic and shifts, x87
    arithmetic, SSE, MMX and 3DNow arithmetic and conversions."""
    if mn in INTEGER or SSE.match(mn):
        return True
    return mn.startswith('f') and not mn.startswith(X87_NOT_COMPUTING)


def main():
    def extra(ap):
        ap.add_argument('coverage')
        ap.add_argument('addresses', nargs='*')
        ap.add_argument('--system')
        ap.add_argument('--all', action='store_true')
    args = parse_args(__doc__, disasm=True, extra=extra)
    points = []
    for line in Path(args.coverage).read_text().splitlines():
        line = line.strip()
        if line.startswith('0x'):
            points.append(int(line, 16))
    if not points:
        sys.exit('coverage: %s holds no address; run the tests with BENILLA_RE_COVERAGE_OUT set'
                 % args.coverage)
    points.sort()
    dis, rows = Disasm(args.disasm), Functions()
    if args.addresses:
        chosen = [rows.at[int(a, 16)] for a in args.addresses if int(a, 16) in rows.at]
        missing = [a for a in args.addresses if int(a, 16) not in rows.at]
        for a in missing:
            print('%s has no function row' % a)
    elif args.system:
        chosen = [r for r in rows.rows if r['system'] == args.system]
    else:
        chosen = [r for r in rows.rows if r['test']]

    def hit(lo, hi):
        i = bisect.bisect_left(points, lo)
        return i < len(points) and points[i] < hi

    totals = [0, 0, 0]
    for row in chosen:
        start, end = row['address'], row['address'] + row['size']
        k = dis.at.get(start)
        ins = []
        while k is not None and k < len(dis.ins) and dis.ins[k][0] < end:
            ins.append(dis.ins[k])
            k += 1
        if not ins:
            continue
        ran, live = [], False
        for a, n, mn, ops in ins:
            if hit(a, min(a + n, end)):
                live = True
            ran.append(live)
            if ENDS.match(mn):
                live = False
        done = sum(ran)
        totals[0] += 1
        totals[1] += len(ins)
        totals[2] += done
        if done == len(ins) and not args.all:
            continue
        if done == 0:
            print('%s: not run (%d instructions)' % (Functions.label(row), len(ins)))
            continue
        print('%s: %d of %d instructions ran' % (Functions.label(row), done, len(ins)))
        j = 0
        while j < len(ins):
            if ran[j]:
                j += 1
                continue
            m = j
            while m < len(ins) and not ran[m]:
                m += 1
            gap = ins[j:m]
            busy = sum(1 for _, _, mn, _ in gap if computes(mn))
            print('  never ran %s to %s: %d instructions, %d compute; first `%s %s`' % (
                addr(gap[0][0]), addr(gap[-1][0]), len(gap), busy, gap[0][2], gap[0][3]))
            j = m
    print('%d functions, %d of %d instructions ran' % (totals[0], totals[2], totals[1]),
          file=sys.stderr)
    return 0


if __name__ == '__main__':
    sys.exit(main())

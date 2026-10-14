#!/usr/bin/env python3
"""A system's closure, walked from its roots, against what systems/README.md asks of it.

    tools/closure.py SYSTEM [--list]
    tools/closure.py ADDRESS [ADDRESS ...] [--list]

The closure of a system is every function reachable from its roots by calls, tail jumps, vtable
slots and stored addresses, stopping at functions another system owns (systems/README.md,
Roots). Given a system, the roots are the addresses on the `Roots:` lines of its docs; given
addresses, those are the roots and the system is the first root's.

From each function it follows, in the listing (dumps/disasm.txt):

  call, jmp   every direct `call`, `jmp` or conditional jump to another row's start;
  address     every row start an instruction names as an operand (a handler pushed or stored);
  slot        when an operand names data whose aligned dwords are row starts (a vtable or a
              handler table), each of those dwords up to the first that is not one.

A function of another system is listed once as a boundary and not followed. What the walk cannot
see is outside it: a call through a register or a slot it did not reach by an operand, a table
indexed from a computed base. A function the system owns that the walk never reaches is reachable
some other way or not at all; `cargo xtask census --why` says which.

It prints each function of the system in the closure that falls short (no class, an inferred
meaning, a `primitive` with no test), the boundary functions with `--list`, and a summary.
Options: --exe PATH, --disasm PATH.
"""

import re
import sys
from collections import deque
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / 'tables'))
from lib import ROOT, Disasm, Functions, Image, addr, parse_args  # noqa: E402

HEX = re.compile(r'0x([0-9a-f]+)')
ROOTS = re.compile(r'^Roots:(.*)$', re.M)


def roots_of(system):
    docs = [ROOT / 'systems' / (system + '.md')] + sorted((ROOT / 'systems' / system).glob('*.md'))
    out = []
    for doc in docs:
        if doc.is_file():
            for line in ROOTS.findall(doc.read_text(encoding='utf-8')):
                out += [int(v, 16) for v in re.findall(r'`0x([0-9a-f]+)`', line)]
    return out


def edges(img, dis, rows, row):
    """(target, kind, site) for each reference out of a function row."""
    start, end = row['address'], row['address'] + row['size']
    k = dis.at.get(start)
    while k is not None and k < len(dis.ins) and dis.ins[k][0] < end:
        a, _, mn, ops = dis.ins[k]
        k += 1
        for v in (int(x, 16) for x in HEX.findall(ops)):
            if v in rows.at and v != start:
                kind = mn if mn in ('call', 'jmp') else ('jmp' if mn.startswith('j') else 'address')
                yield v, kind, a
            elif v not in rows.at and img.section_of(v) in ('.rdata', '.data') and v % 4 == 0:
                slot = v
                while img.in_image(slot + 3) and img.u32(slot) in rows.at:
                    yield img.u32(slot), 'slot', a
                    slot += 4


def main():
    def extra(ap):
        ap.add_argument('what', nargs='+')
        ap.add_argument('--list', action='store_true')
    args = parse_args(__doc__, disasm=True, extra=extra)
    img, dis, rows = Image(args.exe), Disasm(args.disasm), Functions()
    if args.what[0].startswith('0x'):
        roots = [int(a, 16) for a in args.what]
        missing = [r for r in roots if r not in rows.at]
        if missing:
            sys.exit('closure: no function row at %s' % ', '.join(addr(r) for r in missing))
        system = rows.at[roots[0]]['system']
    else:
        system = args.what[0]
        roots = roots_of(system)
        if not roots:
            sys.exit('closure: no Roots: line in systems/%s' % system)
    seen, boundary, queue = {}, {}, deque()
    for r in roots:
        if r not in rows.at:
            print('root %s has no function row' % addr(r))
            continue
        seen[r] = None
        queue.append(r)
    while queue:
        f = queue.popleft()
        for t, kind, site in edges(img, dis, rows, rows.at[f]):
            if t in seen or t in boundary:
                continue
            if rows.at[t]['system'] != system:
                boundary[t] = (kind, site)
                continue
            seen[t] = (kind, site)
            queue.append(t)
    short = {'no class': [], 'inferred': [], 'untested primitive': []}
    for f in sorted(seen):
        row = rows.at[f]
        how = seen[f]
        via = 'root' if how is None else '%s at %s' % (how[0], addr(how[1]))
        if not row['class']:
            short['no class'].append((row, via))
        elif row['class'] == 'primitive' and not row['test']:
            short['untested primitive'].append((row, via))
        if row['confidence'] != 'verified':
            short['inferred'].append((row, via))
    for what, items in short.items():
        for row, via in items:
            print('%-18s %s  (%s)' % (what, Functions.label(row), via))
    if args.list:
        for t in sorted(boundary):
            kind, site = boundary[t]
            print('boundary %s  (%s at %s)' % (Functions.label(rows.at[t]), kind, addr(site)))
    owned = sum(1 for r in rows.rows if r['system'] == system)
    print('%s: %d roots, %d functions in the closure of %d the system owns, %d boundary functions; '
          '%d without a class, %d not verified, %d primitives without a test' % (
              system, len(roots), len(seen), owned, len(boundary), len(short['no class']),
              len(short['inferred']), len(short['untested primitive'])), file=sys.stderr)
    return 0


if __name__ == '__main__':
    sys.exit(main())

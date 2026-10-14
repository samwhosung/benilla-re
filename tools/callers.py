#!/usr/bin/env python3
"""The census of every reference to an address, with controls (METHOD.md, Completeness and
Checklist: proof by absence).

    tools/callers.py ADDRESS [ADDRESS ...] --control ADDRESS [ADDRESS ...]

For each address it runs the two scans METHOD.md names and prints one line per site:

  direct  every `call`, `jmp` or conditional jump in the listing (dumps/disasm.txt) whose target
          is the address, short forms included, each with the function row that holds it. An `e8`
          or `e9` byte in .text whose rel32 lands on the address but that starts no instruction of
          the listing is printed apart, as `rejected`, with the reason: such a byte is inside
          another instruction or in data, and is no call.
  stored  every little-endian occurrence of the address as a dword, at any alignment, in every
          section of the image. In .text it names the instruction that holds the dword, or says
          that none does (a table inside code); in data it gives the alignment. Two kinds are
          printed as `rejected`: a dword inside the import directory's tables, whose dwords are
          relative addresses of import names, never pointers; and a dword that starts an
          instruction of the listing, whose first byte is an opcode, so it is no operand.

The address may be a function or a global: for a global the stored scan is the census of every
instruction that names it by its absolute address (a load, a store, a `push`, a `lea`); an access
through a pointer computed into a register is outside both scans, as METHOD.md warns.

A zero is evidence only beside a control that the same scans find, so the tool refuses to run
without `--control`, runs each control in the same pass, and says for each arm whether a control
exercised it. Pick a control with a known direct call and one with a known stored address.

Options: --exe PATH, --disasm PATH (as for the table generators, tools/tables/lib.py).
"""

import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / 'tables'))
from lib import Disasm, Functions, Image, addr, parse_args  # noqa: E402

BRANCHES = ('call', 'jmp')


def direct(dis, target):
    """The listing's call, jmp and jcc instructions whose operand is target."""
    want = '0x%x' % target
    return [(a, mn) for a, _, mn, ops in dis.ins
            if (mn in BRANCHES or mn.startswith('j')) and ops == want]


def rel32(img, dis, target):
    """Every e8 or e9 byte in .text whose rel32 lands on target and that starts no instruction."""
    out = []
    for site in img.call_sites(target):
        if site not in dis.at:
            out.append(site)
    return out


def stored(img, target):
    """Every little-endian occurrence of target, at any alignment, in every section."""
    needle = struct.pack('<I', target)
    out = []
    for name, (lo, hi, _) in img.sections.items():
        mem = img.mem
        i = mem.find(needle, lo - img.base, hi - img.base)
        while i >= 0:
            out.append((img.base + i, name))
            i = mem.find(needle, i + 1, hi - img.base)
    return out


def holder(dis, site):
    """The listing instruction whose bytes hold the dword at site, or None."""
    k = None
    lo, hi = 0, len(dis.ins)
    while lo < hi:
        mid = (lo + hi) // 2
        if dis.ins[mid][0] <= site:
            k, lo = mid, mid + 1
        else:
            hi = mid
    if k is None:
        return None
    a, size, mn, ops = dis.ins[k]
    if a <= site and site + 4 <= a + size:
        return a, mn, ops
    return None


def opens(dis, site):
    """Whether site is where an instruction of the listing starts: a dword there begins with the
    opcode and is no operand."""
    return site in dis.at


def census(img, dis, rows, imports, target, title):
    print('%s %s' % (title, Functions.label(rows.at.get(target)) if target in rows.at
                     else addr(target)))
    hits = direct(dis, target)
    for site, mn in hits:
        print('  direct  %-6s %s  in %s' % (mn, addr(site), Functions.label(rows.holding(site))))
    for site in rel32(img, dis, target):
        print('  rejected e8/e9 %s  starts no instruction of the listing (inside one, or data)'
              % addr(site))
    n_stored = 0
    for site, section in stored(img, target):
        if any(lo <= site < hi for lo, hi in imports):
            print('  rejected dword %s  %s import table: a relative address of a name'
                  % (addr(site), section))
            continue
        if section == '.text' and opens(dis, site):
            _, _, mn, ops = dis.ins[dis.at[site]]
            print('  rejected dword %s  starts the instruction `%s %s`: an opcode, no operand'
                  % (addr(site), mn, ops))
            continue
        n_stored += 1
        if section == '.text':
            h = holder(dis, site)
            if h:
                print('  stored  %s  operand of `%s %s` at %s  in %s'
                      % (addr(site), h[1], h[2], addr(h[0]), Functions.label(rows.holding(site))))
            else:
                print('  stored  %s  .text, inside no instruction of the listing (a table)  in %s'
                      % (addr(site), Functions.label(rows.holding(site))))
        else:
            print('  stored  %s  %s, %s' % (addr(site), section,
                                             'aligned' if site % 4 == 0 else
                                             'offset %d from alignment' % (site % 4)))
    print('  %d direct, %d stored' % (len(hits), n_stored))
    return len(hits), n_stored


def main():
    argv = sys.argv[1:]
    if '--control' not in argv:
        sys.exit('callers: a zero is evidence only beside a control the same scans find; pass '
                 '--control ADDRESS (see the header)')
    k = argv.index('--control')
    head, controls = argv[:k], []
    j = k + 1
    while j < len(argv) and not argv[j].startswith('-'):
        controls.append(int(argv[j], 16))
        j += 1
    rest = argv[j:]
    targets = [int(a, 16) for a in head if not a.startswith('-')]
    opts = [a for a in head if a.startswith('-')]
    sys.argv = [sys.argv[0]] + opts + rest
    args = parse_args(__doc__, disasm=True)
    if not targets or not controls:
        sys.exit('usage: tools/callers.py ADDRESS [ADDRESS ...] --control ADDRESS [ADDRESS ...]')
    img, dis, rows = Image(args.exe), Disasm(args.disasm), Functions()
    imports = img.import_ranges()
    for t in targets:
        census(img, dis, rows, imports, t, 'target')
        print()
    arms = [False, False]
    for c in controls:
        d, s = census(img, dis, rows, imports, c, 'control')
        arms[0] |= d > 0
        arms[1] |= s > 0
        print()
    print('controls: the direct scan %s; the stored scan %s' % (
        'found sites' if arms[0] else 'found NONE, so a direct zero above is no evidence',
        'found sites' if arms[1] else 'found NONE, so a stored zero above is no evidence'))
    return 0 if all(arms) else 1


if __name__ == '__main__':
    sys.exit(main())

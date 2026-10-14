#!/usr/bin/env python3
"""The compiler's own type names in the exe, and the code that uses each.

    tools/class-tags.py [NAME ...] [--system NAME]

MSVC writes a type's decorated name, `.?AVCCamera@@` for a class, `.?AU...` for a struct,
`.PAV...` for a pointer to one, wherever the source asks for it by `typeid`. In this client that
is mostly the allocator: the storage allocation call takes the type's name as its tag, so a
`push` of a tag string sits beside the allocation of an object of that type, and the function
that holds it builds or grows one. Each name is the compiler's, never ours, which makes it the
strongest naming lead the exe holds for a struct; it is still a lead, and the bytes decide what
the allocation is.

For every decorated name it prints the string's address, the name undecorated (`NTempest::C3Vector`,
`CCamera *`; a template keeps its decorated form) and each place that holds the string's address:
the instruction and the function row in .text, the section elsewhere. It also counts the
complete run-time type records (a TypeDescriptor whose name is the string, a complete object
locator pointing at it, a vtable slot before the locator), which exist only for classes compiled
with run-time type information; it names each class that has one.

With names (a substring of the decorated or the undecorated name), only matching types are
printed; with `--system`, only the uses inside functions of that system. Options: --exe PATH,
--disasm PATH.
"""

import re
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / 'tables'))
from lib import Disasm, Functions, Image, addr, parse_args  # noqa: E402

DECORATED = re.compile(rb'\.(\?A[TUVW]|P[AB][TUVW])[^\x00]{1,240}@@\x00')


def undecorate(text):
    """`.?AVC3Vector@NTempest@@` -> `NTempest::C3Vector`; `.PAVSThread@@` -> `SThread *`."""
    head, body = text[:4], text[4:-2]
    if head.startswith('.?AW'):
        body = body[1:]  # an enum's underlying-type digit
    name = body if '?' in body else '::'.join(reversed(body.split('@')))
    if head.startswith('.PA'):
        return name + ' *'
    if head.startswith('.PB'):
        return 'const ' + name + ' *'
    return name


def instruction_at(dis, site):
    lo, hi, k = 0, len(dis.ins), None
    while lo < hi:
        mid = (lo + hi) // 2
        if dis.ins[mid][0] <= site:
            k, lo = mid, mid + 1
        else:
            hi = mid
    if k is not None:
        a, n, mn, ops = dis.ins[k]
        if a < site and site + 4 <= a + n:
            return a, mn, ops
    return None


def main():
    def extra(ap):
        ap.add_argument('names', nargs='*')
        ap.add_argument('--system')
    args = parse_args(__doc__, disasm=True, extra=extra)
    img, dis, rows = Image(args.exe), Disasm(args.disasm), Functions()
    mem = bytes(img.mem)
    tags = []
    for m in DECORATED.finditer(mem):
        text = m.group()[:-1].decode('latin1')
        tags.append((img.base + m.start(), text, undecorate(text)))
    # Run-time type records: TypeDescriptor = {vftable, spare, name}, so the name is at +8; a
    # complete object locator is {signature 0, offset, cd offset, TypeDescriptor, hierarchy}.
    rtti = []
    for va, text, name in tags:
        td = va - 8
        needle = struct.pack('<I', td)
        i = mem.find(needle)
        while i >= 0:
            col = img.base + i - 12
            if i >= 12 and img.u32(col) == 0 and img.in_image(img.u32(col + 16)):
                if mem.find(struct.pack('<I', col)) >= 0:
                    rtti.append(name)
            i = mem.find(needle, i + 1)
    shown = 0
    for va, text, name in tags:
        if args.names and not any(n in text or n in name for n in args.names):
            continue
        needle = struct.pack('<I', va)
        uses = []
        i = mem.find(needle)
        while i >= 0:
            site = img.base + i
            section = img.section_of(site)
            if section == '.text':
                row = rows.holding(site)
                if args.system is None or (row and row['system'] == args.system):
                    ins = instruction_at(dis, site)
                    if ins is None and dis.at.get(site) is not None:
                        i = mem.find(needle, i + 1)
                        continue  # the dword opens an instruction: its opcode, no operand
                    what = ('`%s %s` at %s' % (ins[1], ins[2], addr(ins[0])) if ins
                            else 'inside no instruction')
                    uses.append('  %s in %s' % (what, Functions.label(row)))
            elif args.system is None:
                uses.append('  %s %s' % (section, addr(site)))
            i = mem.find(needle, i + 1)
        if args.system is not None and not uses:
            continue
        shown += 1
        print('%s %s %s' % (addr(va), text, name))
        for u in uses:
            print(u)
    print('%d decorated names, %d printed; complete run-time type records for %d: %s' % (
        len(tags), shown, len(rtti), ', '.join(sorted(set(rtti))) or 'none'), file=sys.stderr)
    return 0


if __name__ == '__main__':
    sys.exit(main())

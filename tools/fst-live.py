#!/usr/bin/env python3
"""The places where a float is stored and then used at full precision (METHOD.md, Checklist:
reading the bytes, on `fst`).

    tools/fst-live.py [ADDRESS ...] [--system NAME]

`fst DWORD PTR [m]` stores st0 rounded to 32 bits and leaves st0 on the stack at the precision it
had, 53 bits under the client's control word. Code after it that uses st0 reads the unrounded
value, while code that reloads [m] reads the rounded one; a reimplementation that keeps one f32
gets the first wrong by up to half an f32 ulp. Where the use is a comparison the difference can
change a branch exactly at a threshold, which only a probe at the threshold sees.

For every `fst DWORD PTR` in the chosen function rows it follows that value on the x87 stack
(each load pushes it down, each pop lifts it, `fxch` moves it) until the first instruction that
uses it, and prints the store, the use and its kind:

  compare     an `fcom`, `fucom`, `ficom` or `ftst` that reads the unrounded value;
  clamp       a compare `fcom [K]` whose taken arm drops the value and loads the same [K]
              (`fstp st(0)`, `fld [K]`): the result is max or min of the value and K, and since
              rounding keeps order and K is a float, its rounded result is the same from either
              precision, so the use is listed apart;
  arithmetic  any other x87 instruction that reads it.

Past a conditional jump it follows the fall-through path only. It stops without a use at a pop of
the value, a call, a return or an unconditional jump, or after twelve instructions; those stores
print nothing, so a use on a jump's taken path is not found. Without addresses or `--system` it reads every
row. Option: --disasm PATH.
"""

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / 'tables'))
from lib import Disasm, Functions, addr, parse_args  # noqa: E402

REG = re.compile(r'st(?:\((\d)\))?(?![a-z])')
PUSHES = ('fld', 'fild', 'fbld')
COMPARES = ('fcom', 'fucom', 'ficom', 'ftst')
STORES = ('fst', 'fist', 'fbst')
STATE = ('fnstsw', 'fstsw', 'fnstcw', 'fstcw', 'fldcw', 'fwait', 'fnclex', 'fclex', 'fninit',
         'finit', 'fnop', 'fnsave', 'fsave', 'frstor', 'fldenv', 'fnstenv', 'fstenv', 'fincstp',
         'fdecstp')
WINDOW = 12


def regs(ops):
    """The x87 registers an operand list names, as stack indexes."""
    return [int(n) if n else 0 for n in REG.findall(ops)]


def pops(mn):
    if mn in ('fcompp', 'fucompp'):
        return 2
    return 1 if (mn.endswith('p') and mn not in ('fcmovnp',)) else 0


def follow(dis, k, end):
    """From the `fst` at index k, the first use of its value: (index, kind) or None."""
    pos = 0
    for j in range(k + 1, min(k + 1 + WINDOW, len(dis.ins))):
        a, _, mn, ops = dis.ins[j]
        if a >= end or mn in ('call', 'ret', 'jmp'):
            return None
        if not mn.startswith('f') or mn in STATE:
            continue
        named = regs(ops)
        if mn == 'ffree':
            if pos in named:
                return None
            continue
        if mn == 'fxch':
            other = named[0] if named else 1
            pos = other if pos == 0 else (0 if pos == other else pos)
            continue
        if mn.startswith(PUSHES):
            pos += 1
            continue
        if mn.startswith(STORES):
            if (pos == 0 and pops(mn)) or (named and pos in named):
                return None
            pos -= pops(mn)
            continue
        reads = pos in named if named else pos == 0
        if mn in ('faddp', 'fmulp', 'fsubp', 'fsubrp', 'fdivp', 'fdivrp') and not named:
            reads = pos in (0, 1)
        if reads:
            if mn.startswith(COMPARES):
                return j, 'clamp' if clamp(dis, j) else 'compare'
            return j, 'arithmetic'
        pos -= pops(mn)
        if pos < 0:
            return None
    return None


def clamp(dis, j):
    """`fcom [K]`, then in its arm `fstp st(0)` and `fld [K]`: the value is replaced by K."""
    _, _, mn, ops = dis.ins[j]
    if mn != 'fcom' or 'PTR' not in ops:
        return False
    for m in range(j + 1, min(j + 11, len(dis.ins) - 1)):
        _, _, mn2, ops2 = dis.ins[m]
        if mn2 in ('call', 'ret'):
            return False
        if mn2 == 'fstp' and ops2.replace(' ', '') in ('st(0)', 'st'):
            _, _, mn3, ops3 = dis.ins[m + 1]
            return mn3 == 'fld' and ops3 == ops
    return False


def main():
    def extra(ap):
        ap.add_argument('addresses', nargs='*')
        ap.add_argument('--system')
    args = parse_args(__doc__, disasm=True, extra=extra)
    dis, rows = Disasm(args.disasm), Functions()
    if args.addresses:
        chosen = [rows.at[int(a, 16)] for a in args.addresses if int(a, 16) in rows.at]
        for a in args.addresses:
            if int(a, 16) not in rows.at:
                print('%s has no function row' % a)
    elif args.system:
        chosen = [r for r in rows.rows if r['system'] == args.system]
    else:
        chosen = rows.rows
    kinds = {'compare': 0, 'clamp': 0, 'arithmetic': 0}
    for row in chosen:
        end = row['address'] + row['size']
        k = dis.at.get(row['address'])
        lines = []
        while k is not None and k < len(dis.ins) and dis.ins[k][0] < end:
            a, _, mn, ops = dis.ins[k]
            if mn == 'fst' and ops.startswith('DWORD PTR'):
                use = follow(dis, k, end)
                if use:
                    j, kind = use
                    kinds[kind] += 1
                    b, _, mn2, ops2 = dis.ins[j]
                    lines.append('  %-10s `fst %s` at %s, then `%s %s` at %s' % (
                        kind, ops, addr(a), mn2, ops2, addr(b)))
            k += 1
        if lines:
            print(Functions.label(row))
            print('\n'.join(lines))
    print('%d rows; %d compare, %d clamp, %d arithmetic' % (
        len(chosen), kinds['compare'], kinds['clamp'], kinds['arithmetic']), file=sys.stderr)
    return 0


if __name__ == '__main__':
    sys.exit(main())

"""Synthetic second builds of WoW.exe 5875 for testing `cargo xtask match`.

usage: python3 -I builds.py <repo> <outdir> [scenario ...]
Writes <outdir>/<scenario>.exe and <outdir>/<scenario>.truth for each scenario named (all when
none): one line per function row, its 5875 address and the address it has in the build, or
'none' when the function is gone. Each scenario draws from its own random sequence, so a build
is the same whichever others are made.
"""
import json, random, re, struct, sys, os

repo, outdir = sys.argv[1], sys.argv[2]
os.makedirs(outdir, exist_ok=True)
exe = open(os.path.join(repo, 'WoW/WoW.exe'), 'rb').read()
pe = struct.unpack_from('<I', exe, 0x3c)[0]
opt = struct.unpack_from('<H', exe, pe + 20)[0]
sec = pe + 24 + opt  # first section header = .text
VS, VA, RS, RO = struct.unpack_from('<IIII', exe, sec + 8)
BASE = 0x400000
TSTART = BASE + VA
text = exe[RO:RO + VS]
TEND = TSTART + VS
ROOM = RS  # raw bytes available (next section VA = VA + 0x3fe000 here)
assert VA + RS <= 0x3ff000

sigs = []
for line in open(os.path.join(repo, 'tables/signatures.tsv')).read().splitlines()[1:]:
    a, u, p, _size, _body, c = line.split('\t')
    calls = [tuple(int(x, 16) for x in s.split(':')) for s in c.split(' ') if s]
    sigs.append((int(a, 16), u == 'yes', p, calls))
rows = [s[0] for s in sigs]
rowset = set(rows)
sizes = {}
for line in open(os.path.join(repo, 'symbols/functions.tsv')).read().splitlines():
    pass  # sizes from row spacing below
for i, a in enumerate(rows):
    sizes[a] = (rows[i + 1] if i + 1 < len(rows) else TEND) - a

# every rel32 branch in .text from the listing
rel = []  # (src, pos_of_rel32, end_of_insn, target)
pat = re.compile(r'^\s*([0-9a-f]+):\s+(\S+)\s+0x([0-9a-f]+)$')
for line in open(os.path.join(repo, 'dumps/disasm.txt')):
    m = pat.match(line)
    if not m:
        continue
    mn = m.group(2)
    if not (mn == 'call' or mn.startswith('j')):
        continue
    src = int(m.group(1), 16)
    o = src - TSTART
    if o < 0 or o + 5 > len(text):
        continue
    b0 = text[o]
    if b0 in (0xe8, 0xe9):
        rel.append((src, src + 1, src + 5, int(m.group(3), 16)))
    elif b0 == 0x0f and (text[o + 1] & 0xf0) == 0x80:
        rel.append((src, src + 2, src + 6, int(m.group(3), 16)))
print('rel32 sites', len(rel), file=sys.stderr)


def layout(items):
    """items: list of ('seg', start, end) | ('pad', bytes). Returns new text, map(old->new)
    for primary copies, and list of (newstart, oldstart, oldend) for every placed segment."""
    out = bytearray()
    placed = []
    for it in items:
        if it[0] == 'seg':
            placed.append((TSTART + len(out), it[1], it[2]))
            out += text[it[1] - TSTART:it[2] - TSTART]
        else:
            out += it[1]
    primary = {}
    segs = []
    for ns, s, e in placed:
        if (s, e) not in primary:
            primary[(s, e)] = ns
            segs.append((s, e, ns))
    segs.sort()
    starts = [s for s, _, _ in segs]
    import bisect

    def mp(a):
        i = bisect.bisect_right(starts, a) - 1
        if i < 0:
            return a
        s, e, ns = segs[i]
        if a < e or (a == e and i == len(segs) - 1):
            return ns + (a - s)
        return a
    # fix rel32 in every placed copy
    relsorted = sorted(rel)
    srcs = [r[0] for r in relsorted]
    for ns, s, e in placed:
        lo = bisect.bisect_left(srcs, s)
        hi = bisect.bisect_left(srcs, e)
        for src, p, end, tgt in relsorted[lo:hi]:
            if end > e:
                continue
            nend = ns + (end - s)
            npos = ns + (p - s)
            if TSTART <= tgt < TEND:
                ntgt = mp(tgt)
            else:
                ntgt = tgt
            v = (ntgt - nend) & 0xffffffff
            struct.pack_into('<I', out, npos - TSTART, v)
    return bytes(out), mp


def write(name, newtext, truth):
    assert len(newtext) <= ROOM, (name, len(newtext), ROOM)
    b = bytearray(exe)
    b[RO:RO + RS] = newtext + b'\0' * (RS - len(newtext))
    struct.pack_into('<I', b, sec + 8, len(newtext))
    open(os.path.join(outdir, name + '.exe'), 'wb').write(b)
    with open(os.path.join(outdir, name + '.truth'), 'w') as f:
        for a in rows:
            t = truth.get(a, 'none')
            f.write('%x\t%s\n' % (a, t if isinstance(t, str) else '%x' % t))


def whole():
    return [('seg', TSTART, TEND)]


def split_at(points):
    pts = sorted(set(p for p in points if TSTART < p < TEND))
    b = [TSTART] + pts + [TEND]
    return [('seg', b[i], b[i + 1]) for i in range(len(b) - 1)]


wanted = sys.argv[3:]


def scenario(name):
    """Whether to make `name`, and the random sequence for it."""
    global rng
    rng = random.Random('builds-' + name)
    return not wanted or name in wanted


rng = None

if scenario('base'):
    write('base', text, {a: a for a in rows})

if scenario('shift'):
    # shift all of .text by an odd amount
    k = 0x33
    t, mp = layout([('pad', b'\xcc' * k)] + whole())
    t = t[:len(t)]
    write('shift', t, {a: mp(a) for a in rows})

if scenario('padding'):
    # odd paddings between many functions
    cuts = sorted(rng.sample(rows[1:], 180))
    items = []
    prev = TSTART
    budget = ROOM - VS
    for c in cuts:
        items.append(('seg', prev, c))
        n = rng.choice([1, 2, 3, 5, 7])
        if n > budget:
            n = 0
        budget -= n
        if n:
            items.append(('pad', bytes([rng.choice([0xcc, 0x90])]) * n))
        prev = c
    items.append(('seg', prev, TEND))
    t, mp = layout(items)
    write('padding', t, {a: mp(a) for a in rows})

if scenario('reorder'):
    # swap 60 pairs of adjacent rows (reorder neighbours)
    cand = [i for i in range(1, len(rows) - 2) if sizes[rows[i]] < 4096 and sizes[rows[i + 1]] < 4096]
    pick = []
    used = set()
    for i in rng.sample(cand, 400):
        if i in used or i + 1 in used or i - 1 in used or i + 2 in used:
            continue
        used |= {i, i + 1}
        pick.append(i)
        if len(pick) == 60:
            break
    pick.sort()
    items = []
    prev = TSTART
    for i in pick:
        a, b, c = rows[i], rows[i + 1], rows[i + 2]
        items.append(('seg', prev, a))
        items.append(('seg', b, c))
        items.append(('seg', a, b))
        prev = c
    items.append(('seg', prev, TEND))
    t, mp = layout(items)
    truth = {a: mp(a) for a in rows}
    write('reorder', t, truth)

if scenario('duplicate'):
    # duplicate small functions: a copy right after the original, and one at the end of .text
    small = [a for a in rows if 6 <= sizes[a] <= 40]
    dups = sorted(rng.sample(small, 12))
    pts = []
    for a in dups:
        pts += [a, a + sizes[a]]
    items = []
    for it in split_at(pts):
        items.append(it)
        if it[1] in dups[:6] and it[2] == it[1] + sizes[it[1]]:
            items.append(('seg', it[1], it[2]))  # copy right after
    for a in dups[6:]:
        items.append(('seg', a, a + sizes[a]))
    t, mp = layout(items)
    truth = {a: mp(a) for a in rows}
    write('duplicate', t, truth)
    json.dump([hex(a) for a in dups], open(os.path.join(outdir, 'duplicate.dups'), 'w'))

if scenario('midcall'):
    # callers whose call lands in the middle of a function
    # for 200 random call sites to row X, point the rel32 into the middle of another row
    # (X's own body or a random one); the callee is then not changed, only the caller.
    t = bytearray(text)
    sites = [(src, p, end, tg) for src, p, end, tg in rel if tg in rowset]
    chosen = rng.sample(sites, 200)
    for src, p, end, tg in chosen:
        other = rng.choice(rows)
        off = rng.randrange(1, max(2, sizes[other]))
        struct.pack_into('<I', t, p - TSTART, (other + off - end) & 0xffffffff)
    write('midcall', bytes(t), {a: a for a in rows})

if scenario('thunkswap'):
    # realistic: a call to a thunk now goes to another thunk (same 1-fixed-byte pattern)
    patof = {s[0]: s[2] for s in sigs}
    thunks = [a for a in rows if patof[a] == 'e9 ?? ?? ?? ??']
    t = bytearray(text)
    tsites = [(src, p, end, tg) for src, p, end, tg in rel if tg in rowset and patof[tg] == 'e9 ?? ?? ?? ??']
    chosen = rng.sample(tsites, min(200, len(tsites)))
    for src, p, end, tg in chosen:
        other = rng.choice(thunks)
        struct.pack_into('<I', t, p - TSTART, (other - end) & 0xffffffff)
    write('thunkswap', bytes(t), {a: a for a in rows})

if scenario('removed'):
    # functions removed: 300 non-unique rows' bodies replaced by int3 (the function is gone)
    t = bytearray(text)
    nonu = [s[0] for s in sigs if not s[1] and sizes[s[0]] < 256]
    gone = rng.sample(nonu, 300)
    for a in gone:
        t[a - TSTART:a - TSTART + sizes[a]] = b'\xcc' * sizes[a]
    truth = {a: a for a in rows}
    for a in gone:
        truth[a] = 'none'
    write('removed', bytes(t), truth)

if scenario('calleeprologue'):
    # a callee changed so that a caller's call lands mid-function: the callee's body moves
    # down by its prologue (padding inserted at its start), every caller fixed to the old start+0
    # i.e. callers now land in the middle of the padded function. Emulated: insert 3 bytes of
    # 'push ebp; mov ebp,esp'-like filler ahead of 100 callee rows but do NOT fix callers.
    cuts = sorted(rng.sample([a for a in rows if any(True for _ in [0])], 100))
    items = []
    prev = TSTART
    for c in cuts:
        items.append(('seg', prev, c))
        items.append(('pad', b'\x90\x90\x90'))
        prev = c
    items.append(('seg', prev, TEND))
    t, mp = layout(items)
    truth = {a: mp(a) for a in rows}
    # now break callers of the cut rows: point them at the padding start (old mapping - 3) + 1
    t = bytearray(t)
    cutset = set(cuts)
    for src, p, end, tg in rel:
        if tg in cutset:
            nend = mp(end)
            struct.pack_into('<I', t, mp(p) - TSTART, (mp(tg) - 2 - nend) & 0xffffffff)
    write('calleeprologue', bytes(t), truth)

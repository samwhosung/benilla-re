//! The vertex programs of `Shaders\Vertex\Model2.bls` (`systems/models/shared.md#model-vertex-programs`),
//! read from the install: the file's two program sets, and which of the vertex constants c6 to c9
//! each program reads, by the stage digits of its index (`0x71de50`).

mod common;

use benilla_re_difftest::data_dir;
use common::mount;

const PROGRAMS: usize = 1800;

fn rd(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// The records of the set whose table entry is `level`: the three header dwords, the code.
fn records(file: &[u8], level: usize) -> Vec<(u32, u32, u32, &[u8])> {
    let mut at = rd(file, 0x0c + 4 * level) as usize;
    let mut out = Vec::new();
    for _ in 0..PROGRAMS {
        let size = rd(file, at + 12) as usize;
        out.push((
            rd(file, at),
            rd(file, at + 4),
            rd(file, at + 8),
            &file[at + 16..at + 16 + size],
        ));
        at += 16 + ((size + 3) & !3);
    }
    out
}

/// The stage digits of a program index: the id is `5·(3·(3·Z + t0) + t1) + d`.
fn digits(index: usize) -> (usize, usize) {
    let q = index / 5;
    ((q / 3) % 3, q % 3)
}

/// The constants among c6 to c9 a Direct3D program reads as sources (bit `n - 6`), and the base
/// registers of its relative-addressed constants.
fn d3d_reads(code: &[u8]) -> (u8, Vec<u32>, Vec<u32>) {
    let word = |i: usize| rd(code, 4 * i);
    let (mut mask, mut relative, mut defs) = (0u8, Vec::new(), Vec::new());
    let mut p = 1;
    loop {
        let t = word(p);
        if t == 0xffff {
            assert_eq!(4 * (p + 1), code.len(), "the end token closes the record");
            break;
        }
        let (op, len) = (t & 0xffff, ((t >> 24) & 0xf) as usize);
        if op == 0xfffe {
            p += 1 + ((t >> 16) & 0x7fff) as usize;
            continue;
        }
        if op == 81 {
            defs.push(word(p + 1) & 0x7ff);
        } else if op != 31 {
            for i in 2..=len {
                let x = word(p + i);
                let kind = ((x >> 28) & 7) | ((x >> 8) & 0x18);
                if kind != 2 {
                    continue;
                }
                if x & 0x2000 != 0 {
                    relative.push(x & 0x7ff);
                } else if (6..=9).contains(&(x & 0x7ff)) {
                    mask |= 1 << ((x & 0x7ff) - 6);
                }
            }
        }
        p += 1 + len;
    }
    (mask, relative, defs)
}

/// The constants among c6 to c9 an ARB program reads: the `program.local` indexes its declarations
/// bind, followed through the uses of their names (an indexed use names one, a bare use all), and
/// the arrays it addresses relatively.
fn arb_reads(text: &str) -> (u8, Vec<String>) {
    let mut names: Vec<(String, u32, u32)> = Vec::new();
    let mut mask = 0u8;
    let mut relative = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("PARAM ") {
            if let Some(at) = rest.find("program.local[") {
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
                let range = &rest[at + 14..rest[at..].find(']').unwrap() + at];
                let (lo, hi) = range.split_once("..").unwrap_or((range, range));
                names.push((name, lo.parse().unwrap(), hi.parse().unwrap()));
            }
            continue;
        }
        if line.starts_with("ATTRIB") || line.starts_with("TEMP") || line.starts_with('!') {
            continue;
        }
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !(bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
                continue;
            }
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let Some((_, lo, hi)) = names.iter().find(|n| n.0 == line[start..i]) else {
                continue;
            };
            let used: Vec<u32> = if bytes.get(i) == Some(&b'[') {
                let close = line[i..].find(']').unwrap() + i;
                match line[i + 1..close].parse::<u32>() {
                    Ok(n) => vec![lo + n],
                    Err(_) => {
                        relative.push(line[start..i].to_string());
                        (*lo..=*hi).collect()
                    }
                }
            } else {
                (*lo..=*hi).collect()
            };
            for n in used {
                if (6..=9).contains(&n) {
                    mask |= 1 << (n - 6);
                }
            }
        }
    }
    (mask, relative)
}

/// What the stage digits predict: stage 0's transform reads c6 and c7, stage 1's c8 and c9.
fn expected(t0: usize, t1: usize) -> u8 {
    (if t0 == 1 { 0b0011 } else { 0 }) | (if t1 == 1 { 0b1100 } else { 0 })
}

#[test]
fn model2_program_constants() {
    let Some(dir) = data_dir() else {
        return;
    };
    let mut chain = mount(&dir, "misc.MPQ");
    let file = chain
        .read_file("Shaders\\Vertex\\Model2.bls")
        .expect("the install has Model2.bls");

    // The header: the magic, the version, the program count, and the table of 6 level offsets,
    // of which levels 1 (Direct3D) and 2 (ARB) are set and the second set ends the file.
    assert_eq!(rd(&file, 0), 0x4758_5653);
    assert_eq!(rd(&file, 4), 0x0001_0003);
    assert_eq!(rd(&file, 8) as usize, PROGRAMS);
    let table: Vec<u32> = (0..6).map(|i| rd(&file, 0x0c + 4 * i)).collect();
    assert_eq!(table[0], 0);
    assert!(table[1] != 0 && table[2] > table[1] && table[3..].iter().all(|&o| o == 0));

    let d3d = records(&file, 1);
    let arb = records(&file, 2);
    let end = arb
        .last()
        .map(|r| r.3.as_ptr() as usize + r.3.len())
        .unwrap();
    assert_eq!(
        (end - file.as_ptr() as usize + 3) & !3,
        file.len(),
        "the second set ends the file"
    );

    let mut sets = std::collections::BTreeMap::new();
    for (i, (a, b, c, code)) in d3d.iter().enumerate() {
        assert_eq!((*a, *b, *c, rd(code, 0)), (0, 0, 2, 0xfffe_0200), "D3D {i}");
        let (mask, relative, defs) = d3d_reads(code);
        let (t0, t1) = digits(i);
        assert_eq!(mask, expected(t0, t1), "D3D program {i}");
        assert!(relative.iter().all(|r| (31..=33).contains(r)), "D3D {i}");
        assert_eq!(defs, [0], "D3D {i} defines only c0");
        *sets.entry((t0, t1, mask)).or_insert(0usize) += 1;
    }
    for (i, (a, b, c, code)) in arb.iter().enumerate() {
        assert_eq!((*a, *b, *c), (0, 0, 0), "ARB {i}");
        let text = std::str::from_utf8(code).expect("ARB text");
        assert!(
            text.starts_with("!!ARBvp1.0\n") && text.ends_with("END\n"),
            "ARB {i}"
        );
        let (mask, relative) = arb_reads(text);
        let (t0, t1) = digits(i);
        assert_eq!(mask, expected(t0, t1), "ARB program {i}");
        assert!(relative.iter().all(|r| r == "c31"), "ARB {i}");
    }
    // Nine digit pairs, 200 programs each (40 combinations of the other digits, 5 lowest digits).
    assert_eq!(sets.len(), 9);
    assert!(sets.values().all(|&n| n == PROGRAMS / 9));
}

/// The fields of a program index: the id is `5·(3·(3·Z + t0) + t1) + d` with `Z = G + 2·L + 4·B +
/// 20·P`, where `d` is the point-light count the scene adds (`0x707cf6`).
#[derive(Clone, Copy, Debug)]
struct Id {
    d: usize,
    g: bool,
    lit: bool,
    pixel: bool,
}

fn id(index: usize) -> Id {
    let z = index / 45;
    Id {
        d: index % 5,
        g: z & 1 != 0,
        lit: z & 2 != 0,
        pixel: z / 20 != 0,
    }
}

/// A register token: its kind (0 temporary, 1 input, 2 constant, 4 rasterizer output, 5 colour
/// output), index and, for a destination, the write mask, for a source the swizzle byte.
#[derive(Clone, Copy, Debug)]
struct Reg {
    kind: u32,
    n: u32,
    bits: u32,
}

struct Inst {
    op: u32,
    dst: Reg,
    src: Vec<Reg>,
}

const MOV: u32 = 1;
const ADD: u32 = 2;
const MAD: u32 = 4;
const MIN: u32 = 10;
const MAX: u32 = 11;

/// The instructions of a Direct3D program (without `dcl`, `def` and comments) and the value of
/// its `def c0`.
fn d3d_insts(code: &[u8]) -> (Vec<Inst>, [f32; 4]) {
    let word = |i: usize| rd(code, 4 * i);
    let reg = |x: u32, dst: bool| Reg {
        kind: ((x >> 28) & 7) | ((x >> 8) & 0x18),
        n: x & 0x7ff,
        bits: if dst {
            (x >> 16) & 0xf
        } else {
            (x >> 16) & 0xff
        },
    };
    let (mut out, mut c0) = (Vec::new(), [0.0; 4]);
    let mut p = 1;
    loop {
        let t = word(p);
        if t == 0xffff {
            break;
        }
        let (op, len) = (t & 0xffff, ((t >> 24) & 0xf) as usize);
        if op == 0xfffe {
            p += 1 + ((t >> 16) & 0x7fff) as usize;
            continue;
        }
        if op == 81 {
            assert_eq!(word(p + 1) & 0x7ff, 0, "def sets only c0");
            for (k, v) in c0.iter_mut().enumerate() {
                *v = f32::from_bits(word(p + 2 + k));
            }
        } else if op != 31 {
            out.push(Inst {
                op,
                dst: reg(word(p + 1), true),
                src: (2..=len).map(|i| reg(word(p + i), false)).collect(),
            });
        }
        p += 1 + len;
    }
    (out, c0)
}

/// The last instruction before `before` that writes temporary `n` under write-mask bit `bit`
/// (1 x, 2 y, 4 z, 8 w).
fn writer(insts: &[Inst], before: usize, n: u32, bit: u32) -> Option<usize> {
    (0..before)
        .rev()
        .find(|&i| insts[i].dst.kind == 0 && insts[i].dst.n == n && insts[i].dst.bits & bit != 0)
}

/// The constants c10 to c30 a program reads, as a bit set over `n - 10`.
fn read_constants(insts: &[Inst]) -> u32 {
    let mut set = 0;
    for s in insts.iter().flat_map(|i| i.src.iter()) {
        if s.kind == 2 && (10..=30).contains(&s.n) {
            set |= 1 << (s.n - 10);
        }
    }
    set
}

/// The constants c10 to c30 a program of this id reads (`models/shared#light-constants`). With
/// `d` lights a lit program reads their colours c17.., positions c21.. and the attenuation c25 to
/// c27; a program without the pixel digit also reads the harmonic vectors c10 to c16 when lit and
/// the model colours c28 and c29; the fog pair c30 when fog.
fn expected_constants(id: Id) -> u32 {
    let range = |lo: u32, n: u32| ((1u32 << n) - 1) << (lo - 10);
    let mut set = 0;
    if !id.pixel {
        set |= range(28, 2);
        if id.lit {
            set |= range(10, 7);
        }
    }
    if id.lit && id.d > 0 {
        set |= range(17, id.d as u32) | range(21, id.d as u32) | range(25, 3);
    }
    if id.g {
        set |= range(30, 1);
    }
    set
}

#[test]
fn model2_lit_programs_d3d() {
    let Some(dir) = data_dir() else {
        return;
    };
    let mut chain = mount(&dir, "misc.MPQ");
    let file = chain
        .read_file("Shaders\\Vertex\\Model2.bls")
        .expect("the install has Model2.bls");
    let d3d = records(&file, 1);
    for (i, (_, _, _, code)) in d3d.iter().enumerate() {
        let id = id(i);
        let (insts, c0) = d3d_insts(code);
        // c0 holds 0, 1 and one more value, in an order that varies by program, so a c0 operand is
        // read by value.
        let val = |r: &Reg| {
            assert_eq!((r.kind, r.n), (2, 0), "D3D {i}: a c0 operand");
            assert!(
                r.bits == (r.bits & 3) * 0x55,
                "D3D {i}: a replicated swizzle"
            );
            c0[(r.bits & 3) as usize]
        };
        let colour: Vec<usize> = (0..insts.len())
            .filter(|&k| insts[k].dst.kind == 5 && insts[k].dst.n == 0)
            .collect();
        let fog: Vec<usize> = (0..insts.len())
            .filter(|&k| insts[k].dst.kind == 4 && insts[k].dst.n == 1)
            .collect();
        assert_eq!(fog.len(), 1, "D3D {i} writes oFog once");
        let f = &insts[fog[0]];
        if id.g {
            // oFog = min(max(z·c30.x + c30.y, 0), 1).
            assert_eq!((f.op, val(&f.src[1])), (MIN, 1.0), "D3D {i}");
            let m = writer(&insts, fog[0], f.src[0].n, 1 << (f.src[0].bits & 3)).unwrap();
            assert_eq!((insts[m].op, val(&insts[m].src[1])), (MAX, 0.0), "D3D {i}");
            let a = &insts[writer(
                &insts,
                m,
                insts[m].src[0].n,
                1 << (insts[m].src[0].bits & 3),
            )
            .unwrap()];
            let s = &a.src;
            assert_eq!(a.op, MAD, "D3D {i}");
            assert_eq!(
                (s[0].kind, s[0].bits, s[1].n, s[1].bits, s[2].n, s[2].bits),
                (0, 0xaa, 30, 0x00, 30, 0x55),
                "D3D {i}: z·c30.x + c30.y"
            );
        } else {
            // No fog: oFog = 0.
            assert_eq!((f.op, val(&f.src[0])), (MOV, 0.0), "D3D {i}");
        }
        assert_eq!(
            read_constants(&insts),
            expected_constants(id),
            "D3D {i} constants {id:?}"
        );
        // Fog's z is the third bone matrix row dotted with the position.
        if id.g {
            let m = writer(&insts, fog[0], insts[fog[0]].src[0].n, 8).unwrap();
            let mx = &insts[m];
            let a = writer(&insts, m, mx.src[0].n, 8).unwrap();
            let z = &insts[writer(
                &insts,
                a,
                insts[a].src[0].n,
                1 << (insts[a].src[0].bits & 3),
            )
            .unwrap()];
            // The row is c33, or a0-relative from c33 in a skinned program, whose address token
            // follows (kind 3).
            // A skinned program blends the row into a temporary first.
            assert!(
                (z.op, z.src[0].kind, z.src[0].n) == (9, 2, 33) || (z.op, z.src[0].kind) == (9, 0),
                "D3D {i} z"
            );
            assert!(matches!(z.src[1].kind, 1 | 3), "D3D {i} z");
        }
        if id.pixel {
            // A pixel-shader program writes no oD0 and no c28 or c29: oT0 carries the light term
            // (1 unlit, 0 lit at d = 0, else the clamped sum of the lights with no harmonic
            // terms), oT1 the normalized normal, and the stage coordinates move to oT2 and oT3.
            assert!(
                colour.is_empty(),
                "D3D {i}: a pixel-shader program writes no oD0"
            );
            let out = |n: u32| -> Vec<usize> {
                (0..insts.len())
                    .filter(|&k| insts[k].dst.kind == 6 && insts[k].dst.n == n)
                    .collect()
            };
            let (t0, t1) = (out(0), out(1));
            assert_eq!((t0.len(), t1.len()), (1, 1), "D3D {i}");
            assert!(
                !out(2).is_empty() && !out(3).is_empty(),
                "D3D {i}: stages in oT2, oT3"
            );
            let o = &insts[t0[0]];
            if !id.lit {
                assert_eq!((o.op, o.dst.bits, val(&o.src[0])), (MOV, 7, 1.0), "D3D {i}");
            } else if id.d == 0 {
                assert_eq!((o.op, o.dst.bits, val(&o.src[0])), (MOV, 7, 0.0), "D3D {i}");
            } else {
                let x = writer(&insts, t0[0], o.src[0].n, 1).unwrap();
                assert_eq!((o.op, val(&o.src[1])), (MIN, 1.0), "D3D {i}");
                assert_eq!((insts[x].op, val(&insts[x].src[1])), (MAX, 0.0), "D3D {i}");
            }
            let n = &insts[t1[0]];
            assert_eq!(n.dst.bits, 7, "D3D {i}");
            // `nrm`, or a `rsq` and a `mul` in a skinned program.
            if n.op == MOV {
                let nrm = &insts[writer(&insts, t1[0], n.src[0].n, 1).unwrap()];
                assert_eq!(nrm.op, 36, "D3D {i}: oT1 is a normalized normal");
            } else {
                assert!(n.op == 5 && insts.iter().any(|x| x.op == 7), "D3D {i}");
            }
            continue;
        }
        assert_eq!(colour.len(), 1, "D3D {i} writes oD0 once");
        let k = colour[0];
        let o = &insts[k];
        // The first operand is a copy of c28.
        let model = |r: &Reg| {
            let w = &insts[writer(&insts, k, r.n, 1).unwrap()];
            assert_eq!(
                (w.op, w.src[0].kind, w.src[0].n, w.dst.bits),
                (MOV, 2, 28, 0xf),
                "D3D {i}"
            );
        };
        if !id.lit {
            // oD0 = c28 + c29.
            assert_eq!((o.op, o.src[1].kind, o.src[1].n), (ADD, 2, 29), "D3D {i}");
            model(&o.src[0]);
            continue;
        }
        // oD0 = c28 · E + c29, with E.w = 1.
        assert_eq!((o.op, o.src[2].kind, o.src[2].n), (MAD, 2, 29), "D3D {i}");
        model(&o.src[0]);
        let e = o.src[1].n;
        let wi = writer(&insts, k, e, 8).unwrap();
        let w = &insts[wi];
        assert_eq!(
            (w.op, w.dst.bits, val(&w.src[0])),
            (MOV, 8, 1.0),
            "D3D {i}: E.w = 1"
        );
        let x = writer(&insts, wi, e, 1).unwrap();
        if id.d == 0 {
            // No point light: the harmonic sum, with no clamp.
            let m = &insts[x];
            assert_eq!((m.op, m.dst.bits, m.src[0].n), (MAD, 7, 16), "D3D {i}");
        } else {
            // E = min(max(E, 0), 1): max with 0, then min with 1.
            let mn = &insts[x];
            let mx = &insts[writer(&insts, x, mn.src[0].n, 1).unwrap()];
            assert_eq!(
                (mn.op, mn.dst.bits, val(&mn.src[1])),
                (MIN, 7, 1.0),
                "D3D {i}"
            );
            assert_eq!(
                (mx.op, mx.dst.bits, val(&mx.src[1])),
                (MAX, 7, 0.0),
                "D3D {i}"
            );
        }
    }
    // The unlit programs do not depend on the light count.
    for i in (0..PROGRAMS).filter(|&i| !id(i).lit && id(i).d > 0) {
        assert_eq!(d3d[i].3, d3d[i - id(i).d].3, "D3D {i} unlit");
    }
}

/// The op, destination and operands of an ARB instruction line.
fn arb_line(line: &str) -> Option<(&str, &str, Vec<&str>)> {
    let line = line.trim().trim_end_matches(';');
    let (op, rest) = line.split_once(' ')?;
    if matches!(op, "PARAM" | "ATTRIB" | "TEMP") {
        return None;
    }
    let mut parts = rest.split(", ");
    let dst = parts.next()?;
    Some((op, dst, parts.collect()))
}

#[test]
fn model2_lit_programs_arb() {
    let Some(dir) = data_dir() else {
        return;
    };
    let mut chain = mount(&dir, "misc.MPQ");
    let file = chain
        .read_file("Shaders\\Vertex\\Model2.bls")
        .expect("the install has Model2.bls");
    let arb = records(&file, 2);
    for (i, (_, _, _, code)) in arb.iter().enumerate() {
        let id = id(i);
        let text = std::str::from_utf8(code).unwrap();
        // c0 is 255.002, 0, 1 and one more value, in an order that varies by program.
        let decl = text
            .lines()
            .find(|l| l.starts_with("PARAM c0 = {"))
            .unwrap();
        let c0: Vec<f32> = decl[decl.find('{').unwrap() + 1..decl.find('}').unwrap()]
            .split(',')
            .map(|v| v.trim().parse().unwrap())
            .collect();
        assert_eq!((c0.len(), c0[0]), (4, 255.002), "ARB {i}");
        let val = |t: &str| -> f32 {
            let comp = t
                .strip_prefix("c0.")
                .unwrap_or_else(|| panic!("ARB {i}: {t}"));
            c0["xyzw".find(comp).unwrap()]
        };
        let body: Vec<(&str, &str, Vec<&str>)> = text.lines().filter_map(arb_line).collect();
        // Fog: the view-space depth, or 0 when the material has no fog.
        let fog: Vec<_> = body.iter().filter(|l| l.1 == "result.fogcoord.x").collect();
        assert_eq!(fog.len(), 1, "ARB {i}");
        assert_eq!(fog[0].0, "MOV", "ARB {i}");
        if id.g {
            // The view-space z: the third matrix row dotted with the vertex.
            let (reg, comp) = fog[0].2[0].split_once('.').unwrap();
            let src = (0..body.len())
                .rev()
                .find(|&j| {
                    body[j].1.starts_with(reg)
                        && body[j]
                            .1
                            .split_once('.')
                            .is_some_and(|d| d.1.contains(comp))
                        && body[j].1.len() <= reg.len() + 5
                        && j < body
                            .iter()
                            .position(|l| l.1 == "result.fogcoord.x")
                            .unwrap()
                })
                .unwrap();
            assert_eq!(body[src].0, "DP4", "ARB {i} fog is the z row dot");
        } else {
            assert_eq!(val(fog[0].2[0]), 0.0, "ARB {i} no fog");
        }
        let colour: Vec<usize> = (0..body.len())
            .filter(|&k| body[k].1 == "result.color.front.primary")
            .collect();
        // The constant arrays read: the indexes used outside the declarations.
        let reads = |name: &str| -> Vec<u32> {
            let mut v: Vec<u32> = body
                .iter()
                .flat_map(|l| std::iter::once(l.1).chain(l.2.iter().copied()))
                .filter_map(|t| t.strip_prefix(name)?.strip_prefix('['))
                .filter_map(|t| t.split(']').next()?.parse().ok())
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        let d = id.d as u32;
        let lights: Vec<u32> = if id.lit && d > 0 {
            (0..d).chain(4..4 + d).chain(8..11).collect()
        } else {
            Vec::new()
        };
        assert_eq!(reads("c17"), lights, "ARB {i} light constants {id:?}");
        // The attenuation components used are the first `d`: c17[8..10] swizzles.
        let mut comps = std::collections::BTreeSet::new();
        for l in &body {
            for t in l.2.iter().chain(std::iter::once(&l.1)) {
                for n in ["c17[8]", "c17[9]", "c17[10]"] {
                    if *t == n {
                        comps.extend("xyzw".chars());
                    } else if let Some(sw) = t.strip_prefix(n).and_then(|r| r.strip_prefix('.')) {
                        comps.extend(sw.chars());
                    }
                }
            }
        }
        let want: std::collections::BTreeSet<char> =
            "xyzw".chars().take(if id.lit { id.d } else { 0 }).collect();
        assert_eq!(comps, want, "ARB {i} attenuation components");
        // Fog's z is the third bone matrix row dotted with the position.
        if id.g {
            let f = body
                .iter()
                .position(|l| l.1 == "result.fogcoord.x")
                .unwrap();
            let (reg, comp) = body[f].2[0].split_once('.').unwrap();
            let z = (0..f)
                .rev()
                .find(|&j| {
                    body[j]
                        .1
                        .split_once('.')
                        .is_some_and(|(r, c)| r == reg && c.contains(comp))
                })
                .unwrap();
            let row = body[z].2[0];
            // A skinned program first blends the row into a temporary from `c31[A0.x + 2 ..]`.
            let direct = row == "c31[2]" || row == "c31[A0.x + 2]";
            assert!(
                direct || text.contains("c31[A0.x + 2]"),
                "ARB {i} fog row {row}"
            );
            assert!(direct || !row.starts_with("c31"), "ARB {i} fog row {row}");
        }
        if id.pixel {
            // No colour, no c10 to c16, c28 or c29: texcoord 0 carries the light term (1 unlit, 0
            // lit at d = 0, else the clamped sum of the lights), texcoord 1 the normalized
            // normal, and the stage coordinates move to texcoords 2 and 3.
            assert!(
                colour.is_empty(),
                "ARB {i}: a pixel-shader program writes no colour"
            );
            let find = |name: &str| body.iter().position(|l| l.1.starts_with(name));
            for n in 0..4 {
                assert!(
                    find(&format!("result.texcoord[{n}]")).is_some(),
                    "ARB {i} texcoord {n}"
                );
            }
            let t0 = find("result.texcoord[0].xyz").unwrap();
            let (op, args) = (body[t0].0, &body[t0].2);
            if !id.lit {
                assert_eq!((op, val(args[0])), ("MOV", 1.0), "ARB {i}");
            } else if id.d == 0 {
                assert_eq!((op, val(args[0])), ("MOV", 0.0), "ARB {i}");
            } else {
                assert_eq!((op, val(args[0])), ("MAX", 0.0), "ARB {i}");
                assert_eq!(
                    (body[t0 - 1].0, val(body[t0 - 1].2[0])),
                    ("MIN", 1.0),
                    "ARB {i}"
                );
            }
            let t1 = find("result.texcoord[1].xyz").unwrap();
            assert!(matches!(body[t1].0, "MOV" | "MUL"), "ARB {i}");
            assert!(text.contains("RSQ "), "ARB {i}: the normal is normalized");
            assert_eq!(reads("c10").len() + reads("c28").len(), 0, "ARB {i}");
            continue;
        }
        let harmonics: Vec<u32> = if id.lit { (0..7).collect() } else { Vec::new() };
        assert_eq!(reads("c10"), harmonics, "ARB {i}");
        assert_eq!(reads("c28"), [0, 1], "ARB {i}");
        assert_eq!(colour.len(), 1, "ARB {i} writes the colour once");
        let k = colour[0];
        let (op, _, args) = &body[k];
        let model = &body[k - 1];
        assert_eq!((model.0, model.2[0]), ("MOV", "c28[0]"), "ARB {i}");
        assert_eq!(args[0], model.1, "ARB {i}");
        if !id.lit {
            assert_eq!(
                (*op, args[1]),
                ("ADD", "c28[1]"),
                "ARB {i}: oD0 = c28 + c29"
            );
            continue;
        }
        assert_eq!(
            (*op, args[2]),
            ("MAD", "c28[1]"),
            "ARB {i}: oD0 = c28·E + c29"
        );
        let e = args[1];
        let w = (0..k)
            .rev()
            .find(|&j| body[j].1 == format!("{e}.w"))
            .unwrap();
        assert_eq!(
            (body[w].0, val(body[w].2[0])),
            ("MOV", 1.0),
            "ARB {i}: E.w = 1"
        );
        let last_xyz = (0..k)
            .rev()
            .find(|&j| body[j].1 == format!("{e}.xyz"))
            .unwrap();
        if id.d == 0 {
            assert_eq!(
                body[last_xyz].0, "MAD",
                "ARB {i}: the harmonic sum, unclamped"
            );
            assert!(body.iter().all(|l| l.0 != "MIN" && l.0 != "MAX"), "ARB {i}");
        } else {
            // min(·, 1), then max(·, 0) into E, on the two lines in a row.
            let (mx, mn) = (&body[last_xyz], &body[last_xyz - 1]);
            assert_eq!((mx.0, val(mx.2[0])), ("MAX", 0.0), "ARB {i}");
            assert_eq!((mn.0, val(mn.2[0])), ("MIN", 1.0), "ARB {i}");
            assert_eq!(
                mx.2[1].split('.').next(),
                mn.1.split('.').next(),
                "ARB {i}: max reads the min"
            );
        }
    }
    for i in (0..PROGRAMS).filter(|&i| !id(i).lit && id(i).d > 0) {
        assert_eq!(arb[i].3, arb[i - id(i).d].3, "ARB {i} unlit");
    }
}

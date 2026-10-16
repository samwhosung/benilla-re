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

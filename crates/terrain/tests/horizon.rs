//! The horizon buffer against `AddHorizonOccluder` (`0x681b50`), `TestHorizonSphere` (`0x686000`)
//! and `TestHorizonBox` (`0x686180`), `systems/terrain/cull.md#horizon-buffer`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::horizon;
use common::assert_f32;

/// The render flags; bit 0x20 lets the functions run.
const RENDER_FLAG: u64 = 0xc7b2a4;
/// The value the functions run only within ±0.9 of.
const CFF4: u64 = 0xc7cff4;
/// The view matrix.
const VIEW_MAT: u64 = 0xc7b700;
/// The matrix the sphere test takes its radius through.
const RADIUS_MAT: u64 = 0xc7d280;
/// The 320 columns.
const HORIZON_BUF: u64 = 0xc7b750;

/// `[0x810180]`: the occluder's z gate, 2.7777777 (`0x4031c71c`).
const Z_GATE_INSERT: f32 = f32::from_bits(0x4031_c71c);

/// Opens the render-state gates.
fn open_gates(emu: &mut Emu) {
    emu.write_u32(RENDER_FLAG & !3, {
        let lane = (RENDER_FLAG & 3) as u32;
        0x20u32 << (lane * 8)
    });
    emu.write_u32(CFF4, 0.0f32.to_bits());
}

/// Write a 16-f32 matrix at `va`.
fn write_mat(emu: &mut Emu, va: u64, m: &[f32; 16]) {
    for (i, &v) in m.iter().enumerate() {
        emu.write_u32(va + i as u64 * 4, v.to_bits());
    }
}

/// Write the 320-f32 buffer.
fn write_buf(emu: &mut Emu, buf: &[f32; 320]) {
    for (i, &v) in buf.iter().enumerate() {
        emu.write_u32(HORIZON_BUF + i as u64 * 4, v.to_bits());
    }
}

/// Read all 320 buffer columns.
fn read_buf(emu: &Emu) -> [f32; 320] {
    let mut out = [0.0f32; 320];
    for (i, o) in out.iter_mut().enumerate() {
        *o = emu.read_f32(HORIZON_BUF + i as u64 * 4);
    }
    out
}

fn assert_buf(binary: &[f32; 320], ours: &[f32; 320], ctx: &str) {
    for (i, (b, o)) in binary.iter().zip(ours.iter()).enumerate() {
        assert_f32(*b, *o, &format!("{ctx} col[{i}]"));
    }
}

/// Finite camera-space coordinates: bands of view-space values and the values at the z gates (2.7777777
/// and 50.0); the non-finite inputs are `horizon_non_finite_probes`. The high bits pick each band.
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A finite coordinate from one of five bands.
    fn coord(&mut self) -> f32 {
        let r = self.next();
        match (r >> 8) % 5 {
            0 => {
                // values at the gates and the column centre
                const EDGES: [f32; 12] = [
                    0.0,
                    -0.0,
                    1.0,
                    -1.0,
                    Z_GATE_INSERT,
                    2.7,
                    50.0,
                    49.9,
                    50.1,
                    2.5,
                    -2.5,
                    100.0,
                ];
                EDGES[((self.next() >> 8) as usize) % EDGES.len()]
            }
            1 => {
                // [1.5, 4.0), about the occluder gate
                let u = (self.next() >> 8) as f32 / (1u32 << 24) as f32;
                1.5 + u * 2.5
            }
            2 => {
                // [-30, 30)
                let u = (self.next() >> 8) as f32 / (1u32 << 24) as f32;
                u * 60.0 - 30.0
            }
            3 => {
                // [40, 120), about the 50.0 gate
                let u = (self.next() >> 8) as f32 / (1u32 << 24) as f32;
                40.0 + u * 80.0
            }
            _ => {
                // [-3, 3)
                let u = (self.next() >> 8) as f32 / (1u32 << 24) as f32;
                u * 6.0 - 3.0
            }
        }
    }
    /// A view-space z, mostly above the occluder gate.
    fn z(&mut self) -> f32 {
        let r = self.next();
        match (r >> 8) % 4 {
            0 => self.coord(),
            _ => {
                // [2.0, 80.0)
                let u = (self.next() >> 8) as f32 / (1u32 << 24) as f32;
                2.0 + u * 78.0
            }
        }
    }
}

/// A view matrix near the identity with a tilt, so the columns vary; `m[4j+i]` is row `i`, column
/// `j`, with z biased by 2.0.
fn sample_view() -> [f32; 16] {
    [
        1.0, 0.0, 0.1, 0.0, // col 0
        0.2, 1.0, 0.0, 0.0, // col 1
        0.0, 0.15, 1.0, 0.0, // col 2
        0.3, -0.2, 2.0, 0.0, // col 3
    ]
}

/// The matrix `(w, w, 0)` goes through in the sphere test: half the radius, no translation.
fn sample_radius_mat() -> [f32; 16] {
    [
        0.5, 0.0, 0.0, 0.0, // col 0
        0.0, 0.5, 0.0, 0.0, // col 1
        0.0, 0.0, 0.0, 0.0, // col 2
        0.0, 0.0, 0.0, 0.0, // col 3
    ]
}

/// The values that test the gates and the column `fistp`: an unordered compare passes every gate,
/// and a value outside `i32` stores `0x80000000`. Each case puts one into one coordinate.
const BAD: [f32; 6] = [
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    1.0e30,
    -1.0e30,
    3.0e9,
];

/// `AddHorizonOccluder` (`0x681b50`) on strips of 2 to 9 vertices about the 2.7777777 gate: the
/// gates, the smaller `y/z`, the span either way, the clamps and the raise.
#[test]
fn horizon_insert() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    open_gates(&mut emu);
    let view = sample_view();
    write_mat(&mut emu, VIEW_MAT, &view);

    // 16 vertices 12 bytes apart, and the strip's `i32` indices into them
    let vbuf = emu.alloc(0x400);
    let ibuf = emu.alloc(0x100);
    let offbuf = emu.alloc(0x10);

    let mut s = Lcg(0x1234_5678);
    let mut checked = 0u32;
    let mut both_pass = 0u32;
    let mut any_raise = 0u32;
    for case in 0..3000usize {
        let n_pool = 16usize;
        let mut verts: Vec<[f32; 3]> = Vec::with_capacity(n_pool);
        for _ in 0..n_pool {
            verts.push([s.coord(), s.coord(), s.z()]);
        }
        for (i, v) in verts.iter().enumerate() {
            for (c, &f) in v.iter().enumerate() {
                emu.write_u32(vbuf + (i * 3 + c) as u64 * 4, f.to_bits());
            }
        }
        let offset = [s.coord() * 0.1, s.coord() * 0.1, s.z() * 0.0];
        for (c, &f) in offset.iter().enumerate() {
            emu.write_u32(offbuf + c as u64 * 4, f.to_bits());
        }
        // 2 to 9 vertices in order, as a strip walks them
        let count = 2 + (case % 8);
        let mut indices: Vec<i32> = Vec::with_capacity(count);
        for k in 0..count {
            indices.push(((k + (case % 7)) % n_pool) as i32);
        }
        for (k, &ix) in indices.iter().enumerate() {
            emu.write_u32(ibuf + k as u64 * 4, ix as u32);
        }

        // a skyline some edges raise and some do not
        let mut buf = [0.0f32; 320];
        for (j, b) in buf.iter_mut().enumerate() {
            *b = ((j as f32) * 0.001 - 0.15) * (((case + j) % 3) as f32);
        }
        let pre = buf;
        write_buf(&mut emu, &buf);

        // ecx = the vertices, edx = the indices, then the count and the offset
        emu.call(
            0x681b50,
            vbuf as u32,
            ibuf as u32,
            &[count as u32, offbuf as u32],
        );
        let bin = read_buf(&emu);

        horizon::horizon_insert(&verts, &indices, &offset, &view, &mut buf);
        assert_buf(&bin, &buf, &format!("horizon_insert case {case}"));
        // a column raised
        if buf
            .iter()
            .zip(pre.iter())
            .any(|(a, b)| a.to_bits() != b.to_bits())
        {
            any_raise += 1;
        }

        // edges with both ends past the gate
        let proj: Vec<[f32; 3]> = indices
            .iter()
            .map(|&ix| {
                let v = verts[ix as usize];
                let tmp = [
                    (f64::from(v[0]) + f64::from(offset[0])) as f32,
                    (f64::from(v[1]) + f64::from(offset[1])) as f32,
                    (f64::from(v[2]) + f64::from(offset[2])) as f32,
                ];
                let p = benilla_re_cmath::transform::transform_point(&tmp, &view);
                [p[0], p[1], p[2]]
            })
            .collect();
        for e in 0..proj.len().saturating_sub(1) {
            if proj[e][2] >= Z_GATE_INSERT && proj[e + 1][2] >= Z_GATE_INSERT {
                both_pass += 1;
            }
        }
        checked += 1;
    }
    assert!(
        both_pass > 0,
        "no edge ever passed the z-gate — domain too cold"
    );
    assert!(any_raise > 0, "buffer never raised — domain too cold");
    eprintln!(
        "horizon_insert: {checked} strips bit-exact vs WoW.exe @ 0x681b50 \
         (z-gate-pass edges {both_pass}, cases that raised the buffer {any_raise})"
    );
}

/// `TestHorizonSphere` (`0x686000`) on centres in the camera window, radii below and above 2^-22
/// and flags 0 and 8.
#[test]
fn horizon_span_test() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    open_gates(&mut emu);
    let view = sample_view();
    let radius_mat = sample_radius_mat();
    write_mat(&mut emu, VIEW_MAT, &view);
    write_mat(&mut emu, RADIUS_MAT, &radius_mat);
    let cbuf = emu.alloc(0x10); // the centre

    let mut s = Lcg(0xdead_cafe);
    let mut checked = 0u32;
    let mut ret2 = 0u32;
    let mut ret0 = 0u32;
    let mut bypass = 0u32;
    for case in 0..4000usize {
        let center = [s.coord(), s.coord(), s.z()];
        for (c, &f) in center.iter().enumerate() {
            emu.write_u32(cbuf + c as u64 * 4, f.to_bits());
        }
        // radii in a band, some below the 2^-22 gate
        let w = match (s.next() >> 8) % 4 {
            0 => 1e-9, // below the gate
            1 => 0.5 + ((s.next() >> 8) as f32 / (1u32 << 24) as f32) * 3.0,
            2 => 5.0 + ((s.next() >> 8) as f32 / (1u32 << 24) as f32) * 20.0,
            _ => -((s.next() >> 8) as f32 / (1u32 << 24) as f32) * 4.0 - 0.5, // negative radius
        };
        let flags = if (s.next() >> 8) & 1 == 0 { 0u32 } else { 8u32 };
        if flags & 8 != 0 {
            bypass += 1;
        }

        // a ramp that hides some spheres
        let mut buf = [0.0f32; 320];
        for (j, b) in buf.iter_mut().enumerate() {
            *b = (j as f32) * 0.01 - 1.5 + ((case % 5) as f32) * 0.3;
        }
        write_buf(&mut emu, &buf);

        // ecx = the centre, edx = the flags, the radius on the stack
        let bin = emu.call(0x686000, cbuf as u32, flags, &[w.to_bits()]);

        let ours = horizon::horizon_span_test(&center, w, flags, &view, &radius_mat, &buf);
        assert_eq!(
            bin, ours,
            "horizon_span_test case {case}: binary={bin} ours={ours} (center={center:?} w={w} flags={flags})"
        );
        match ours {
            2 => ret2 += 1,
            0 => ret0 += 1,
            other => panic!("unexpected return {other}"),
        }
        checked += 1;
    }
    assert!(
        ret2 > 0 && ret0 > 0,
        "one return code never seen (2:{ret2} 0:{ret0})"
    );
    assert!(bypass > 0, "flags&8 bypass never exercised");
    eprintln!(
        "horizon_span_test: {checked} spheres bit-exact vs WoW.exe @ 0x686000 \
         (ret2 {ret2}, ret0 {ret0}, gate-bypass {bypass})"
    );
}

/// `TestHorizonBox` (`0x686180`) on boxes in the camera window and flags 0 and 8.
#[test]
fn horizon_box_test() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    open_gates(&mut emu);
    let view = sample_view();
    write_mat(&mut emu, VIEW_MAT, &view);
    let bbuf = emu.alloc(0x20); // min then max

    let mut s = Lcg(0xfeed_face);
    let mut checked = 0u32;
    let mut ret2 = 0u32;
    let mut ret0 = 0u32;
    let mut bypass = 0u32;
    for case in 0..4000usize {
        // a box, z mostly above the gate
        let mn = [s.coord(), s.coord(), s.z()];
        let ext = [
            (s.next() >> 8) as f32 / (1u32 << 24) as f32 * 8.0,
            (s.next() >> 8) as f32 / (1u32 << 24) as f32 * 8.0,
            (s.next() >> 8) as f32 / (1u32 << 24) as f32 * 30.0,
        ];
        let mx = [mn[0] + ext[0], mn[1] + ext[1], mn[2] + ext[2]];
        let box6 = [mn[0], mn[1], mn[2], mx[0], mx[1], mx[2]];
        for (c, &f) in box6.iter().enumerate() {
            emu.write_u32(bbuf + c as u64 * 4, f.to_bits());
        }
        let flags = if (s.next() >> 8) & 1 == 0 { 0u32 } else { 8u32 };
        if flags & 8 != 0 {
            bypass += 1;
        }

        let mut buf = [0.0f32; 320];
        for (j, b) in buf.iter_mut().enumerate() {
            *b = (j as f32) * 0.008 - 1.2 + ((case % 4) as f32) * 0.25;
        }
        write_buf(&mut emu, &buf);

        // ecx = the box, edx = the flags
        let bin = emu.call(0x686180, bbuf as u32, flags, &[]);

        let ours = horizon::horizon_box_test(&box6, flags, &view, &buf);
        assert_eq!(
            bin, ours,
            "horizon_box_test case {case}: binary={bin} ours={ours} (box={box6:?} flags={flags})"
        );
        match ours {
            2 => ret2 += 1,
            0 => ret0 += 1,
            other => panic!("unexpected return {other}"),
        }
        checked += 1;
    }
    assert!(
        ret2 > 0 && ret0 > 0,
        "one return code never seen (2:{ret2} 0:{ret0})"
    );
    assert!(bypass > 0, "flags&8 bypass never exercised");
    eprintln!(
        "horizon_box_test: {checked} boxes bit-exact vs WoW.exe @ 0x686180 \
         (ret2 {ret2}, ret0 {ret0}, gate-bypass {bypass})"
    );
}

#[test]
fn horizon_non_finite_probes() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    open_gates(&mut emu);
    let view = sample_view();
    let radius_mat = sample_radius_mat();
    write_mat(&mut emu, VIEW_MAT, &view);
    write_mat(&mut emu, RADIUS_MAT, &radius_mat);

    // Skylines: a ramp (some spheres occluded), a ramp with a NaN column, all-NaN, all -1e6 (the
    // reset value: nothing occludes).
    let ramp = {
        let mut b = [0.0f32; 320];
        for (j, v) in b.iter_mut().enumerate() {
            *v = (j as f32) * 0.01 - 1.5;
        }
        b
    };
    let mut ramp_nan = ramp;
    ramp_nan[160] = f32::NAN;
    let all_nan = [f32::NAN; 320];
    let cleared = [-1_000_000.0f32; 320];
    let bufs: [(&str, [f32; 320]); 4] = [
        ("ramp", ramp),
        ("ramp+nan", ramp_nan),
        ("all-nan", all_nan),
        ("cleared", cleared),
    ];

    // 0x686000: the span test
    let cbuf = emu.alloc(0x10);
    let base_center = [0.5f32, 0.25, 75.0];
    let mut n_span = 0u32;
    let mut span_ret = [0u32; 3];
    for (bname, buf) in &bufs {
        write_buf(&mut emu, buf);
        // each coordinate of the centre and the radius replaced in turn
        let mut cases: Vec<([f32; 3], f32)> = vec![(base_center, 5.0)];
        for &bad in &BAD {
            for k in 0..3 {
                let mut c = base_center;
                c[k] = bad;
                cases.push((c, 5.0));
            }
            cases.push((base_center, bad));
        }
        for (center, w) in cases {
            for flags in [0u32, 8u32] {
                for (c, &f) in center.iter().enumerate() {
                    emu.write_u32(cbuf + c as u64 * 4, f.to_bits());
                }
                let bin = emu.call(0x686000, cbuf as u32, flags, &[w.to_bits()]);
                let ours = horizon::horizon_span_test(&center, w, flags, &view, &radius_mat, buf);
                assert_eq!(
                    bin, ours,
                    "span probe [{bname}]: binary={bin} ours={ours} (center={center:?} w={w} flags={flags})"
                );
                span_ret[ours as usize] += 1;
                n_span += 1;
            }
        }
    }

    // 0x686180: the box test
    let bbuf = emu.alloc(0x20);
    let base_box = [-1.0f32, -1.0, 60.0, 1.0, 1.0, 90.0];
    let mut n_box = 0u32;
    let mut box_ret = [0u32; 3];
    for (bname, buf) in &bufs {
        write_buf(&mut emu, buf);
        let mut cases: Vec<[f32; 6]> = vec![base_box];
        for &bad in &BAD {
            for k in 0..6 {
                let mut b = base_box;
                b[k] = bad;
                cases.push(b);
            }
        }
        for box6 in cases {
            for flags in [0u32, 8u32] {
                for (c, &f) in box6.iter().enumerate() {
                    emu.write_u32(bbuf + c as u64 * 4, f.to_bits());
                }
                let bin = emu.call(0x686180, bbuf as u32, flags, &[]);
                let ours = horizon::horizon_box_test(&box6, flags, &view, buf);
                assert_eq!(
                    bin, ours,
                    "box probe [{bname}]: binary={bin} ours={ours} (box={box6:?} flags={flags})"
                );
                box_ret[ours as usize] += 1;
                n_box += 1;
            }
        }
    }

    // 0x681b50: the strip insert
    let vbuf = emu.alloc(0x100);
    let ibuf = emu.alloc(0x40);
    let offbuf = emu.alloc(0x10);
    let base_verts = [
        [-2.0f32, -1.0, 10.0],
        [0.0, 0.5, 12.0],
        [2.0, -0.5, 14.0],
        [4.0, 1.0, 9.0],
    ];
    let offset = [0.0f32, 0.0, 0.0];
    for (c, &f) in offset.iter().enumerate() {
        emu.write_u32(offbuf + c as u64 * 4, f.to_bits());
    }
    let indices = [0i32, 1, 2, 3];
    for (k, &ix) in indices.iter().enumerate() {
        emu.write_u32(ibuf + k as u64 * 4, ix as u32);
    }
    let mut n_ins = 0u32;
    for start in [-0.3f32, 0.4] {
        let mut cases: Vec<Vec<[f32; 3]>> = vec![base_verts.to_vec()];
        for &bad in &BAD {
            for v in 0..4 {
                for k in 0..3 {
                    let mut vs = base_verts.to_vec();
                    vs[v][k] = bad;
                    cases.push(vs);
                }
            }
        }
        for verts in cases {
            for (i, v) in verts.iter().enumerate() {
                for (c, &f) in v.iter().enumerate() {
                    emu.write_u32(vbuf + (i * 3 + c) as u64 * 4, f.to_bits());
                }
            }
            let mut buf = [start; 320];
            write_buf(&mut emu, &buf);
            emu.call(0x681b50, vbuf as u32, ibuf as u32, &[4, offbuf as u32]);
            let bin = read_buf(&emu);
            horizon::horizon_insert(&verts, &indices, &offset, &view, &mut buf);
            assert_buf(
                &bin,
                &buf,
                &format!("insert probe start={start} verts={verts:?}"),
            );
            n_ins += 1;
        }
    }

    eprintln!(
        "horizon_non_finite_probes: span {n_span} (ret0 {} ret2 {}), box {n_box} (ret0 {} ret2 {}), \
         insert {n_ins} bit-exact vs WoW.exe",
        span_ret[0], span_ret[2], box_ret[0], box_ret[2]
    );
}

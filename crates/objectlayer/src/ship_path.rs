//! The moving transport's path, whole: `0x5f4cc0` builds it from the `TaxiPathNode` rows when the
//! object's record arrives, and `0x5f50a0` places the ship on it at a time
//! (`systems/objectlayer/transports.md`, The ship path, Leg control points, Moving the ship).
//!
//! A leg is a Catmull-Rom curve over its nodes as they are, mode 1 (cubic), measured by arc length:
//! `n` nodes give `n − 3` segments, and the curve runs from node 1 to node `n − 2`. The kernels are
//! the movement spline file's (`benilla_re_spline`); this module strings them together the way the
//! two functions call them. The x87 runs at 53 bits: an operation rounds to `f64` and a store to
//! `f32`.

use benilla_re_spline::eval::{
    arc_length, eval_point_cubic, eval_point_quad, knot_sum_n, knot_sum_nm1, seg_locate,
};

use crate::gameobject::transport_speed_easing;
use crate::leg_timing::{arc_time_ms, arc_time_ms_final};

/// The Catmull-Rom basis at `0xb05e10`, which `0x453fa0` writes at start-up: four rows of four
/// coefficients, highest power first.
pub const CATMULL_ROM: [f32; 16] = [
    -0.5, 1.0, -0.5, 0.0, //
    1.5, -2.5, 0.0, 1.0, //
    -1.5, 2.0, 0.5, 0.0, //
    0.5, -0.5, 0.0, 0.0,
];

/// Its first derivative at `0xb05dd8` (`0x454080`), rows of three, as `0x454250` lays it out on the
/// stack in rows of four: the fourth word of rows 0 to 2 is 0 and of row 3 is 1.0, which degree 2
/// never reads.
pub const CATMULL_ROM_DERIVATIVE: [f32; 16] = [
    -1.5, 2.0, -0.5, 0.0, //
    4.5, -5.0, 0.0, 0.0, //
    -4.5, 4.0, 0.5, 0.0, //
    1.5, -1.0, 0.0, 1.0,
];

/// The chord step of `0x453760`, the float at `0x8029c8`.
const ARC_STEP: f32 = 0.05;
/// `0x8029d4`, the length below which `0x454580` and `0x5f8cb0` leave a vector undivided.
const SHORT: f64 = 2.384_185_791_015_625e-7;
/// `0x8029d0`, the derivative length below which `0x454580` writes no tangent.
const SHORT_DERIVATIVE: f32 = 0.01;
/// `0x801360`, the float 0.001 that turns ms into seconds.
const MS: f32 = 0.001;

/// A `TaxiPathNode` row as the builder reads it: the path `+4`, the map `+0xc`, the position
/// `+0x10`, the flags `+0x1c` and the delay in seconds `+0x20`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TaxiNode {
    pub path: u32,
    pub map: u32,
    pub position: [f32; 3],
    pub flags: u32,
    pub delay: u32,
}

/// One entry of a leg's stop table, 12 bytes at `leg+0x44`: the time it is reached in the period
/// (ms), its distance along the leg, and its wait (ms).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stop {
    pub time: u32,
    pub distance: f32,
    pub wait: u32,
}

/// One leg, 0x4c bytes: the map `+0`, the curve `+4` (its control points, segment lengths and total
/// length `+8`), the start `+0x30`, the travel time `+0x34`, the end `+0x38`, and the stop table
/// `+0x40`, `+0x44`, whose last entry is the leg's end.
#[derive(Clone, Debug, PartialEq)]
pub struct Leg {
    pub map: u32,
    pub points: Vec<[f32; 3]>,
    pub lengths: Vec<f32>,
    pub length: f32,
    pub start: u32,
    pub travel: u32,
    pub end: u32,
    pub stops: Vec<Stop>,
}

/// The path at handler `+0x20`: the speed `+0x18`, the acceleration `+0x20`, the legs and the
/// period `+0x1c` (ms).
#[derive(Clone, Debug, PartialEq)]
pub struct ShipPath {
    pub speed: f32,
    pub accel: f32,
    pub legs: Vec<Leg>,
    pub period: u32,
}

/// A data column made a float as the builder does: `fild` of the zero-extended qword, `fstp dword`.
#[must_use]
pub fn column_float(v: u32) -> f32 {
    f64::from(v) as f32
}

/// The control points of segment `i`, `P[i]` to `P[i + 3]`.
fn quad(points: &[[f32; 3]], i: usize) -> [[f32; 3]; 4] {
    [points[i], points[i + 1], points[i + 2], points[i + 3]]
}

/// `0x5f4cc0` up to the transport list: the nodes are the run of rows from the first with `path`,
/// in store order, ending at the first row of another path or the store's end. A leg opens at the
/// first node, at a node on another map than the leg's and after a node with flag 0x1; a node with
/// flag 0x2 that is not its leg's first is a stop of `delay · 1000` ms. Each leg is timed by
/// [`time_leg`], then laid end to end: its start is the total so far, the total gains its travel
/// and the waits of the last leg's stops, its stops' times gain the start, and its end is the
/// total. The total is the period.
///
/// # Panics
///
/// On a leg the client leaves partly unwritten (`time_leg`).
#[must_use]
pub fn build(nodes: &[TaxiNode], path: u32, speed: u32, accel: u32) -> ShipPath {
    let speed = column_float(speed);
    let accel = column_float(accel);
    let mut legs: Vec<Leg> = Vec::new();
    let mut points: Vec<[f32; 3]> = Vec::new();
    let mut stops: Vec<(u32, u32)> = Vec::new();
    if let Some(first) = nodes.iter().position(|n| n.path == path) {
        let mut map: Option<u32> = None;
        let mut after_flag1 = false;
        for node in nodes[first..].iter().take_while(|n| n.path == path) {
            // 0x5f4dd5: no leg, a map change or the previous node's bit 0x1 opens a leg.
            if map != Some(node.map) || after_flag1 {
                if let Some(m) = map {
                    legs.push(time_leg(m, &points, &stops, speed, accel));
                }
                map = Some(node.map);
                points.clear();
                stops.clear();
            }
            // 0x5f4e37: a stop needs a node before it in the leg.
            if node.flags & 2 != 0 && !points.is_empty() {
                stops.push((points.len() as u32, node.delay.wrapping_mul(1000)));
            }
            points.push(node.position);
            after_flag1 = node.flags & 1 != 0;
        }
        if let Some(m) = map {
            legs.push(time_leg(m, &points, &stops, speed, accel));
        }
    }
    // 0x5f4ebe: the stop buffer holds the last leg's stops for every leg's end.
    let waits = stops.iter().fold(0u32, |w, s| w.wrapping_add(s.1));
    let mut total = 0u32;
    for leg in &mut legs {
        leg.start = total;
        total = total.wrapping_add(leg.travel).wrapping_add(waits);
        for s in &mut leg.stops {
            s.time = s.time.wrapping_add(leg.start);
        }
        leg.end = total;
    }
    ShipPath {
        speed,
        accel,
        legs,
        period: total,
    }
}

/// `0x5f9120(points, stops, leg)`: the curve over `points` (`SetSplinePath`), its segment lengths
/// as 20 chords each stored as floats and its total as a float of their 53-bit sum, then the stop
/// table. Stop `j` at node `k` lies at the 53-bit sum of the first `k − 1` lengths; each span is
/// timed on its own by [`arc_time_ms`] (the first one-ended, the rest two-ended), the last from the
/// final stop to the leg's end by [`arc_time_ms_final`]. An entry's time is the travel so far plus
/// the waits before it; the end entry's distance is the length and its wait 0.
///
/// # Panics
///
/// For a leg of 3 nodes or fewer, whose lengths the client never writes, and for a stop on a leg's
/// last node, which leaves entries of the table unwritten.
#[must_use]
pub fn time_leg(map: u32, points: &[[f32; 3]], stops: &[(u32, u32)], v: f32, a: f32) -> Leg {
    let n = points.len();
    assert!(n > 3, "a leg of {n} nodes keeps an unwritten length");
    let lengths: Vec<f32> = (0..n - 3)
        .map(|i| {
            let p = quad(points, i);
            arc_length(ARC_STEP, |t| eval_point_cubic(&CATMULL_ROM, &p, t)) as f32
        })
        .collect();
    let length = knot_sum_n(0.0, &lengths, (n - 3) as u32) as f32;
    let mut table = vec![
        Stop {
            time: 0,
            distance: 0.0,
            wait: 0,
        };
        stops.len() + 1
    ];
    let mut travel = 0u32;
    let mut last = 0.0f32;
    let mut waited = 0u32;
    let mut j = 0usize;
    while j < stops.len() {
        let (k, wait) = stops[j];
        assert!(
            (k as usize) < n - 1,
            "a stop on a leg's last node leaves entries unwritten"
        );
        let len = knot_sum_nm1(0.0, &lengths, k);
        travel = travel.wrapping_add(arc_time_ms(len, last, v, a, j == 0) as u32);
        last = len as f32;
        table[j] = Stop {
            time: travel.wrapping_add(waited),
            distance: last,
            wait,
        };
        waited = waited.wrapping_add(wait);
        j += 1;
    }
    travel = travel.wrapping_add(arc_time_ms_final(length, last, v, a, j == 0) as u32);
    table[j] = Stop {
        time: travel.wrapping_add(waited),
        distance: length,
        wait: 0,
    };
    Leg {
        map,
        points: points.to_vec(),
        lengths,
        length,
        start: 0,
        travel,
        end: 0,
        stops: table,
    }
}

/// The point and tangent `0x454580` writes into a frame whose tangent row holds `tangent` before
/// the call: the segment and local parameter by arc length, the cubic point, the unit chord
/// `P[i + 2] − P[i + 1]` and the derivative. The derivative, divided by its length, is the tangent
/// when the length is at least 0.01 and its dot product with the chord at least 0.5; below 0.5 the
/// chord is; a shorter derivative writes no tangent.
#[must_use]
pub fn curve_frame(leg: &Leg, t: f32, tangent: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let n = leg.points.len();
    let (i, u) = seg_locate(leg.length, &leg.lengths, t, (n - 3) as u32);
    let p = quad(&leg.points, i as usize);
    let position = eval_point_cubic(&CATMULL_ROM, &p, u);
    let a = p[1];
    let b = p[2];
    let mut chord = [0, 1, 2].map(|k| (f64::from(b[k]) - f64::from(a[k])) as f32);
    let [cx, cy, cz] = chord.map(f64::from);
    let clen = ((cx * cx + cy * cy) + cz * cz).sqrt();
    // 0x454606: `test ah,5; jnp` skips only a length below 2^-22; equal and NaN divide.
    if clen.abs() >= SHORT || clen.is_nan() {
        let r = 1.0 / f64::from(clen as f32);
        chord = chord.map(|c| (f64::from(c) * r) as f32);
    }
    let d = eval_point_quad(&CATMULL_ROM_DERIVATIVE, &p, u);
    let [dx, dy, dz] = d.map(f64::from);
    let dlen = ((dx * dx + dy * dy) + dz * dz).sqrt();
    if dlen.abs() < f64::from(SHORT_DERIVATIVE) {
        return (position, tangent);
    }
    let inv = 1.0 / dlen;
    let d = d.map(|c| (f64::from(c) * inv) as f32);
    let dot = (f64::from(d[2]) * f64::from(chord[2]) + f64::from(d[1]) * f64::from(chord[1]))
        + f64::from(chord[0]) * f64::from(d[0]);
    // 0x4546b4: `test ah,5; jp` takes the derivative for 0.5 and above and for NaN.
    let tangent = if dot < 0.5 { chord } else { d };
    (position, tangent)
}

/// Where the tick puts the ship: the leg it is on (the leg count when none), that leg's map, the
/// position, the facing and the animation (0 at a stop, else the speed curve's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipPose {
    pub leg: u32,
    pub map: Option<u32>,
    pub position: [f32; 3],
    pub facing: f32,
    pub anim: u32,
}

/// `0x5f8cb0`: a vector of two divided by its length `√(x² + y²)` unless that is below 2^-22.
fn normalize2(v: [f32; 2]) -> [f32; 2] {
    let (x, y) = (f64::from(v[0]), f64::from(v[1]));
    let len = (x * x + y * y).sqrt();
    if len.abs() < SHORT {
        return v;
    }
    let r = 1.0 / len;
    [(r * x) as f32, (r * y) as f32]
}

/// `0x5f50a0` at `now` with the timetable offset `offset` (game object `+0x278`), up to the map
/// test: `t = (now + offset) mod period` (1 for a period of 0), the first leg whose end is after
/// `t`, then within it the stop whose wait holds `t` or the span after the last stop passed. A span
/// runs the speed curve on its time and `t`'s part of it in seconds, accelerating unless it is the
/// first and decelerating unless it is the last, from the distance of the stop it starts at. The
/// distance over the leg's length, clamped to [0, 1] with NaN as 1, places the frame; the heading
/// is the frame's tangent negated in x and y and normalised, and the facing its `atan2`.
///
/// The facing passes through `fpatan` (`0x5f528e`), which this computes as the host's `atan2`.
#[must_use]
pub fn pose(path: &ShipPath, now: u32, offset: u32) -> ShipPose {
    let period = if path.period == 0 { 1 } else { path.period };
    let t = now.wrapping_add(offset) % period;
    let identity = [1.0f32, 0.0, 0.0];
    let Some(li) = path.legs.iter().position(|l| t < l.end) else {
        let h = normalize2([-identity[0], -identity[1]]);
        return ShipPose {
            leg: path.legs.len() as u32,
            map: None,
            position: [0.0; 3],
            facing: f64::from(h[1]).atan2(f64::from(h[0])) as f32,
            anim: 0,
        };
    };
    let leg = &path.legs[li];
    let m = leg.stops.len();
    let mut span_start = leg.start;
    let mut base = 0.0f32;
    let mut anim = 0u32;
    let mut j = 0usize;
    let mut at_stop: Option<f32> = None;
    while j + 1 < m {
        let e = leg.stops[j];
        if t < e.time {
            break;
        }
        if t - e.time < e.wait {
            at_stop = Some(e.distance);
            break;
        }
        span_start = e.time.wrapping_add(e.wait);
        base = e.distance;
        j += 1;
    }
    let dist = if let Some(d) = at_stop {
        f64::from(d)
    } else {
        let span = leg.stops[j].time.wrapping_sub(span_start);
        let span_s = (f64::from(span) * f64::from(MS)) as f32;
        let t_s = (f64::from(t.wrapping_sub(span_start)) * f64::from(MS)) as f32;
        let (d, a) =
            transport_speed_easing(path.speed, path.accel, t_s, span_s, j != 0, j + 1 != m);
        anim = a;
        d + f64::from(base)
    };
    let x = dist / f64::from(leg.length);
    let f = if x < 0.0 {
        0.0
    } else if x < 1.0 {
        x as f32
    } else {
        1.0
    };
    let (position, tangent) = curve_frame(leg, f, identity);
    let h = normalize2([-tangent[0], -tangent[1]]);
    ShipPose {
        leg: li as u32,
        map: Some(leg.map),
        position,
        facing: f64::from(h[1]).atan2(f64::from(h[0])) as f32,
        anim,
    }
}

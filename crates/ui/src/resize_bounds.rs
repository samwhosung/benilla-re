//! Dragging a frame's grip (`systems/ui/resizebounds.md#the-drag`, `#the-clamp`): the one place
//! the resize bounds are applied, `CLayoutFrame::ApplyDragDelta`.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and each `fst`/`fstp dword` is an
//! `as f32`.

/// What `CLayoutFrame::ApplyDragDelta` (`0x768710`) hands the resize `0x768430`, or does instead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DragResolve {
    /// The arguments of `0x768430(this, arg0, cx, cy, dx, dy)`: the new size `(cx, cy)` and the
    /// drag delta `(dx, dy)` left over, each stored as `f32`. This is every grip but the centre one.
    Apply { cx: f32, cy: f32, dx: f32, dy: f32 },
    /// Grip 4, the centre, with a child whose slot `+0xc` answers true: the child's `+4` and `+8`
    /// words become `child + drag / scale`, each stored as `f32`; the resize is not called and the
    /// function returns 1.
    ChildAccum { c4: f32, c8: f32 },
}

/// `CLayoutFrame::ApplyDragDelta` (`0x768710`): applies the cursor difference `(dx, dy)` to the
/// frame's size `(cx0, cy0)`, the results of the vtable getters `+0x1c` and `+0x20` (each stored
/// as `f32`), for the grip `id`, the anchor point 0 to 8. `scale` is `this+0x58`. `min_w`, `max_w`,
/// `min_h` and `max_h` are the bounds at `+0x60`, `+0x68`, `+0x5c` and `+0x64`.
///
/// Each grip changes one or both sizes; some use `inv = 1.0 / scale` and multiply, some divide by
/// `scale`, and the two round differently, so each is written as the binary has it. Each size is
/// clamped to its bounds, a bound of 0 meaning none. A grip on the left or top edge (a rebate)
/// takes the amount cut off from the difference, and a grip on the right or bottom edge (a pin)
/// moves the size to the bound and leaves the difference. The differences a grip does not carry are
/// stored as +0.0.
///
/// | id | cx             | x clamp | cy             | y clamp | dx out  | dy out  |
/// |----|----------------|---------|----------------|---------|---------|---------|
/// | 0  | cx0 - dx*inv   | rebate  | cy0 + dy*inv   | rebate  | rebated | rebated |
/// | 1  | cx0            |         | cy0 + dy/scale | rebate  | 0       | rebated |
/// | 2  | cx0 + dx*inv   | pin     | cy0 + dy*inv   | rebate  | 0       | rebated |
/// | 3  | cx0 - dx/scale | rebate  | cy0            |         | rebated | 0       |
/// | 4  | the child accumulates the drag (see [`DragResolve::ChildAccum`]) |  |  |  |  |  |
/// | 5  | cx0 + dx/scale | pin     | cy0            |         | 0       | 0       |
/// | 6  | cx0 - dx*inv   | rebate  | cy0 - dy*inv   | pin     | rebated | 0       |
/// | 7  | cx0            |         | cy0 - dy/scale | pin     | 0       | 0       |
/// | 8  | cx0 + dx*inv   | pin     | cy0 - dy*inv   | pin     | 0       | 0       |
///
/// The x rebate is `dx - (min - c) * scale` at the minimum and `dx + (c - max) * scale` at the
/// maximum; the y rebate has the signs reversed, `dy + (min - c) * scale` and `dy - (c - max) *
/// scale` (`fsubr` and `fadd` swap between the legs). The tests are `c < min` and `c > max`
/// (`fld c; fcomp bound`), so a bound above the other loses to the maximum. An id above 8 returns
/// `cx0, cy0, dx, dy` unchanged.
///
/// Grip 4 reads the child at `this+4`; if it is not null and its slot `+0xc` answers nonzero, the
/// result is [`DragResolve::ChildAccum`] (`child_accum` carries the child's `+4` and `+8`); a null
/// child or a false answer falls to the unchanged result, as `child_accum = None`.
#[allow(clippy::too_many_arguments)]
pub fn apply_drag_delta(
    cx0: f32,
    cy0: f32,
    dx: f32,
    dy: f32,
    id: u32,
    scale: f32,
    min_w: f32,
    max_w: f32,
    min_h: f32,
    max_h: f32,
    child_accum: Option<(f32, f32)>,
) -> DragResolve {
    let s = f64::from(scale);
    let (cx0f, cy0f, dxf, dyf) = (f64::from(cx0), f64::from(cy0), f64::from(dx), f64::from(dy));
    match id {
        0 => {
            let inv = 1.0 / s;
            let cx = (cx0f - dxf * inv) as f32;
            let (cx, dx) = clamp_x(cx, dx, min_w, max_w, s, true);
            let cy = (cy0f + dyf * inv) as f32;
            let (cy, dy) = clamp_y(cy, dy, min_h, max_h, s, true);
            DragResolve::Apply { cx, cy, dx, dy }
        }
        1 => {
            let cy = (cy0f + dyf / s) as f32;
            let (cy, dy) = clamp_y(cy, dy, min_h, max_h, s, true);
            DragResolve::Apply {
                cx: cx0,
                cy,
                dx: 0.0,
                dy,
            }
        }
        2 => {
            let inv = 1.0 / s;
            let cx = (cx0f + dxf * inv) as f32;
            let (cx, _) = clamp_x(cx, dx, min_w, max_w, s, false);
            let cy = (cy0f + dyf * inv) as f32;
            let (cy, dy) = clamp_y(cy, dy, min_h, max_h, s, true);
            DragResolve::Apply {
                cx,
                cy,
                dx: 0.0,
                dy,
            }
        }
        3 => {
            let cx = (cx0f - dxf / s) as f32;
            let (cx, dx) = clamp_x(cx, dx, min_w, max_w, s, true);
            DragResolve::Apply {
                cx,
                cy: cy0,
                dx,
                dy: 0.0,
            }
        }
        4 => match child_accum {
            Some((c4, c8)) => DragResolve::ChildAccum {
                c4: (f64::from(c4) + dxf / s) as f32,
                c8: (f64::from(c8) + dyf / s) as f32,
            },
            None => DragResolve::Apply {
                cx: cx0,
                cy: cy0,
                dx,
                dy,
            },
        },
        5 => {
            let cx = (cx0f + dxf / s) as f32;
            let (cx, _) = clamp_x(cx, dx, min_w, max_w, s, false);
            DragResolve::Apply {
                cx,
                cy: cy0,
                dx: 0.0,
                dy: 0.0,
            }
        }
        6 => {
            let inv = 1.0 / s;
            let cx = (cx0f - dxf * inv) as f32;
            let (cx, dx) = clamp_x(cx, dx, min_w, max_w, s, true);
            let cy = (cy0f - dyf * inv) as f32;
            let (cy, _) = clamp_y(cy, dy, min_h, max_h, s, false);
            DragResolve::Apply {
                cx,
                cy,
                dx,
                dy: 0.0,
            }
        }
        7 => {
            let cy = (cy0f - dyf / s) as f32;
            let (cy, _) = clamp_y(cy, dy, min_h, max_h, s, false);
            DragResolve::Apply {
                cx: cx0,
                cy,
                dx: 0.0,
                dy: 0.0,
            }
        }
        8 => {
            let inv = 1.0 / s;
            let cx = (cx0f + dxf * inv) as f32;
            let (cx, _) = clamp_x(cx, dx, min_w, max_w, s, false);
            let cy = (cy0f - dyf * inv) as f32;
            let (cy, _) = clamp_y(cy, dy, min_h, max_h, s, false);
            DragResolve::Apply {
                cx,
                cy,
                dx: 0.0,
                dy: 0.0,
            }
        }
        // `ja default`: the size and the difference pass through.
        _ => DragResolve::Apply {
            cx: cx0,
            cy: cy0,
            dx,
            dy,
        },
    }
}

/// The x clamp of `ApplyDragDelta`: `c` is raised to `min` when `c < min`, then lowered to `max`
/// when `c > max`, a bound of 0 meaning none. With `rebate` the difference is also adjusted by the
/// amount cut off, `dx -= (min - c) * scale` or `dx += (c - max) * scale`, each stored as `f32`.
/// Returns `(c as f32, dx)`.
fn clamp_x(c0: f32, d0: f32, min: f32, max: f32, scale: f64, rebate: bool) -> (f32, f32) {
    let mut c = f64::from(c0);
    let mut d = d0;
    if min != 0.0 && c < f64::from(min) {
        if rebate {
            d = (f64::from(d) - (f64::from(min) - c) * scale) as f32;
        }
        c = f64::from(min);
    }
    if max != 0.0 && c > f64::from(max) {
        if rebate {
            d = (f64::from(d) + (c - f64::from(max)) * scale) as f32;
        }
        c = f64::from(max);
    }
    (c as f32, d)
}

/// The y clamp: as [`clamp_x`] with the signs of the rebate reversed, `dy += (min - c) * scale`
/// and `dy -= (c - max) * scale`.
fn clamp_y(c0: f32, d0: f32, min: f32, max: f32, scale: f64, rebate: bool) -> (f32, f32) {
    let mut c = f64::from(c0);
    let mut d = d0;
    if min != 0.0 && c < f64::from(min) {
        if rebate {
            d = (f64::from(d) + (f64::from(min) - c) * scale) as f32;
        }
        c = f64::from(min);
    }
    if max != 0.0 && c > f64::from(max) {
        if rebate {
            d = (f64::from(d) - (c - f64::from(max)) * scale) as f32;
        }
        c = f64::from(max);
    }
    (c as f32, d)
}

/// The grip that `CSimpleTop::StartMovingOrSizingAt` (`0x765320`, `__thiscall`, `ret 0x14`, the
/// arguments `frame, kind, cur_x, cur_y, size_flag`; `input#moving-and-sizing`) picks from where
/// the cursor is in the frame, and passes on to `0x7652b0`. It is the anchor point id, 0 to 8, of
/// the cell of a three by three split that holds the cursor: 0 top left, 1 top, 2 top right, 3 left,
/// 4 centre, 5 right, 6 bottom left, 7 bottom, 8 bottom right. `arg4` is the size flag: when it
/// is 0 the function jumps to 4 (`0x765464`) without reading the rectangle. Otherwise the frame's
/// rectangle comes from `0x768310`, `0x7680e0` and `0x768320` as `rect` = `[bottom, left, top,
/// right]` (`[ebp-0x18]` to `[ebp-0xc]`); if `0x768320` returns 0 the function returns without
/// choosing, which this does not model.
///
/// With `dx = cur_x - left` and `q_x = (right - left) * 0.25` at register precision, and `dy =
/// (cur_y - bottom) as f32` (`fstp [ebp-4]`) and `q_y = ((top - bottom) * 0.25) as f32` (`fstp
/// [ebp+0x18]`), reloaded as `f32` for every compare, with the products by 3.0 (`[0x80297c]`) at
/// register precision:
///
/// - the column is 0 when `dx < q_x`, 1 when `dx < 3 * q_x`, else 2;
/// - the band counts from the bottom: 2 (the bottom row) when `dy < q_y`, 1 when `3 * q_y > dy`,
///   else 0 (the top row), so an equal or unordered value falls to the top (`test ah, 0x41` read
///   by its polarity);
/// - the point is `col + 3 * band`.
pub fn grip_region(arg4: u32, cur_x: f32, cur_y: f32, rect: [f32; 4]) -> u32 {
    if arg4 == 0 {
        return 4;
    }
    let [bottom, left, top, right] = rect.map(f64::from);
    let dx = f64::from(cur_x) - left;
    let dy = (f64::from(cur_y) - bottom) as f32;
    let q_x = (right - left) * 0.25;
    let q_y = ((top - bottom) * 0.25) as f32;
    let dyf = f64::from(dy);
    let q_yf = f64::from(q_y);
    let col: u32 = if dx < q_x {
        0
    } else if dx < 3.0 * q_x {
        1
    } else {
        2
    };
    let band: u32 = if dyf < q_yf {
        2
    } else if 3.0 * q_yf > dyf {
        1
    } else {
        0
    };
    col + 3 * band
}

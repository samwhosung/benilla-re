//! The span-time solve of `0x5f9120`, which times a transport leg (`systems/objectlayer/
//! transports.md#leg-timing`). A span of length `seg = length - last` takes the time of a constant
//! acceleration `a` up to the speed `v`; the slice tests cover the per-stop arithmetic at
//! `0x5f91af` to `0x5f924f` and the end of the leg at `0x5f92bd` to `0x5f9335`.
//!
//! The x87 runs at 53 bits. `ta = float((1/a) * v)` and `da = (v * 0.5) * ta` are computed once:
//! `fst [ebp-8]` stores `float(da)` and keeps the 53-bit `da` on the stack, so each compare reads
//! the live `da` and each formula reads the stored float.

#[inline]
fn g(x: f32) -> f64 {
    f64::from(x)
}

/// The start of a span's time: `(1/a)` at 53 bits, the stored `ta`, and the live and stored `da`.
struct Accel {
    recip: f64,
    ta: f32,
    da_live: f64,
    da: f32,
}

fn accel(speed: f32, accel: f32) -> Accel {
    let recip = 1.0 / g(accel); // fld 1.0; fdiv [esi+0x20]
    let ta = (recip * g(speed)) as f32; // fmul [esi+0x18]; fstp dword
    let da_live = (g(speed) * 0.5) * g(ta); // fld [esi+0x18]; fmul 0.5; fmul ta
    Accel {
        recip,
        ta,
        da_live,
        da: da_live as f32, // fst dword
    }
}

/// The span's time in ms at a stop, `0x5f91af` to `0x5f924f`. `length` is the cumulative length to
/// the stop (`0x453300`'s result), `last` the previous one, `speed` and `accel` are `[esi+0x18]`
/// and `[esi+0x20]`, and `first` is `edi == 0`, the first span of the leg.
pub fn arc_time_ms(length: f32, last: f32, speed: f32, accel_in: f32, first: bool) -> i32 {
    let seg = g(length) - g(last); // fsubr st,st(1)
    let a = accel(speed, accel_in);
    // fcomp and fcompp then `test ah,5; jp`: jp is taken for a >= or unordered compare, which
    // takes the square root.
    let t: f64 = if first {
        if a.da_live < seg {
            (seg - g(a.da)) / g(speed) + g(a.ta)
        } else {
            (seg * a.recip + seg * a.recip).sqrt() // fmulp; fadd st,st; fsqrt
        }
    } else if a.da_live < seg * 0.5 {
        (seg - (g(a.da) + g(a.da))) / g(speed) + (g(a.ta) + g(a.ta))
    } else {
        let r = (seg * a.recip).sqrt(); // fmulp; fsqrt
        r + r // fadd st,st
    };
    round_ms(t)
}

/// The last span of the leg, `0x5f92bd` to `0x5f9335`: `length` is a float in memory, and `first`
/// (`edi == 0`) is a leg with no stop, `seg / v`. Otherwise it is the first-span formula.
pub fn arc_time_ms_final(length: f32, last: f32, speed: f32, accel_in: f32, first: bool) -> i32 {
    let seg = g(length) - g(last);
    let t: f64 = if first {
        seg / g(speed) // fdiv [esi+0x18]
    } else {
        let a = accel(speed, accel_in);
        if a.da_live < seg {
            (seg - g(a.da)) / g(speed) + g(a.ta)
        } else {
            (seg * a.recip + seg * a.recip).sqrt()
        }
    };
    round_ms(t)
}

/// `t * 1000.0`, then `+ 0.5` for a positive product and `- 0.5` otherwise (`fcom 0.0; test
/// ah,0x41; jne`, so zero, a negative and NaN subtract), then `__ftol`.
#[inline]
fn round_ms(t: f64) -> i32 {
    let scaled = t * 1000.0;
    let adj = if scaled > 0.0 {
        scaled + 0.5
    } else {
        scaled - 0.5
    };
    crate::x87::ftol(adj)
}

//! The static initializers of the network object files (`systems/net/layout.md#static-initializers`):
//! the constants of the shared maths header each file copies, the reference count those files
//! share, and the two list heads.
//!
//! [`INITIALIZERS`] lists every body the C runtime's initializer table (`0x827000`) reaches in
//! these files that computes a value, in table order, which is the order the client runs them:
//! a [`Init::Quotient`] can read the result of a [`Init::Double`] above it.

/// What one initializer body does. Addresses are the client's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Init {
    /// `fld [src]; fmul [src]; fstp [dst]`: the `f32` at `src` squared and rounded once to `f32`.
    Square { src: u32, dst: u32 },
    /// `fld [num]; fdiv [den]; fstp [dst]`: the quotient of two `f32`, rounded to 53 bits, then
    /// to `f32`.
    Quotient { num: u32, den: u32, dst: u32 },
    /// `fld [src]; fadd st,st; fstp [dst]`: twice the `f32` at `src`.
    Double { src: u32, dst: u32 },
    /// Adds 1 to the dword at `counter`, and when the result is 1 tail-calls `0x5b71c0`, an empty
    /// function.
    CountUnit { counter: u32 },
    /// An empty list head at `head`: `+0` 0, `+4` its own address, `+8` that address with bit 0
    /// set.
    ListHead { head: u32 },
}

/// What an initializer does to memory: the dwords it stores, in order, and whether it calls the
/// empty function its counter guards.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InitEffect {
    pub stores: Vec<(u32, u32)>,
    pub calls_empty: bool,
}

/// `x * x` with the x87 at 53-bit precision: the product of two `f32` is exact in 53 bits, so
/// the one rounding is the store to `f32`.
pub fn init_square(x: f32) -> f32 {
    (f64::from(x) * f64::from(x)) as f32
}

/// `a / b`, rounded to 53 bits by `fdiv` and then to `f32` by `fstp`.
pub fn init_quotient(a: f32, b: f32) -> f32 {
    (f64::from(a) / f64::from(b)) as f32
}

/// `x + x`, exact in 53 bits, rounded to `f32` by the store.
pub fn init_double(x: f32) -> f32 {
    (f64::from(x) + f64::from(x)) as f32
}

impl Init {
    /// The effect of the body on a memory whose dwords `read` returns.
    pub fn apply(self, read: impl Fn(u32) -> u32) -> InitEffect {
        let f = |va: u32| f32::from_bits(read(va));
        match self {
            Init::Square { src, dst } => InitEffect {
                stores: vec![(dst, init_square(f(src)).to_bits())],
                calls_empty: false,
            },
            Init::Quotient { num, den, dst } => InitEffect {
                stores: vec![(dst, init_quotient(f(num), f(den)).to_bits())],
                calls_empty: false,
            },
            Init::Double { src, dst } => InitEffect {
                stores: vec![(dst, init_double(f(src)).to_bits())],
                calls_empty: false,
            },
            Init::CountUnit { counter } => {
                let n = read(counter).wrapping_add(1);
                InitEffect {
                    stores: vec![(counter, n)],
                    calls_empty: n == 1,
                }
            }
            Init::ListHead { head } => InitEffect {
                stores: vec![(head, 0), (head + 4, head + 4), (head + 8, (head + 4) | 1)],
                calls_empty: false,
            },
        }
    }
}

/// The counter every [`Init::CountUnit`] and [`RELEASES`] function shares.
pub const SHARED_COUNTER: u32 = 0xc2a3c4;

/// The exit functions that undo [`Init::CountUnit`]: `dec [0xc2a3c4]`, and when the result is 0 a
/// tail call to `0x5b71d0`, an empty function. Each is registered with the C runtime's exit list
/// by the thunk of its file's count.
pub const RELEASES: &[u32] = &[
    0x536f60, 0x538170, 0x5aa260, 0x5aca00, 0x5b0380, 0x5b1140, 0x5b16d0, 0x5b1790, 0x5b35d0,
    0x5b4940, 0x5b7c90, 0x5bc9d0, 0x5bd5e0,
];

/// The effect of a [`RELEASES`] function on a counter holding `count`: the new count and whether
/// it calls the empty function.
pub fn release(count: u32) -> (u32, bool) {
    let n = count.wrapping_sub(1);
    (n, n == 0)
}

/// `0x5aa2b0`, registered for exit by `ClientServices.cpp`'s thunk: sets bit 0 of the byte at
/// `0xc27d50` and leaves the other bits.
pub const EXIT_FLAG: u32 = 0xc27d50;

/// The byte `0x5aa2b0` leaves at [`EXIT_FLAG`]: unchanged when bit 0 is set, else with bit 0 set.
pub fn set_exit_flag(flag: u8) -> u8 {
    flag | 1
}

/// Every computing initializer body of the network files, by body address, in the order of the
/// C runtime's initializer table.
pub const INITIALIZERS: &[(u32, Init)] = &[
    (
        0x536ee0,
        Init::Double {
            src: 0x808ffc,
            dst: 0xc0d49c,
        },
    ),
    (
        0x536f00,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc0d49c,
            dst: 0xc0d448,
        },
    ),
    (0x536f30, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x536f80,
        Init::Square {
            src: 0x809000,
            dst: 0xc0d494,
        },
    ),
    (
        0x536fb0,
        Init::Square {
            src: 0x809004,
            dst: 0xc0d4a8,
        },
    ),
    (
        0x536fe0,
        Init::Square {
            src: 0x809008,
            dst: 0xc0d44c,
        },
    ),
    (
        0x537010,
        Init::Square {
            src: 0x80900c,
            dst: 0xc0d480,
        },
    ),
    (
        0x537040,
        Init::Square {
            src: 0x809010,
            dst: 0xc0d4a4,
        },
    ),
    (
        0x537070,
        Init::Square {
            src: 0x809014,
            dst: 0xc0d428,
        },
    ),
    (
        0x5370a0,
        Init::Square {
            src: 0x809018,
            dst: 0xc0d4ac,
        },
    ),
    (
        0x5370d0,
        Init::Square {
            src: 0x80901c,
            dst: 0xc0d43c,
        },
    ),
    (
        0x537100,
        Init::Square {
            src: 0x809020,
            dst: 0xc0d458,
        },
    ),
    (
        0x537130,
        Init::Square {
            src: 0x809024,
            dst: 0xc0d444,
        },
    ),
    (
        0x537160,
        Init::Square {
            src: 0x809028,
            dst: 0xc0d484,
        },
    ),
    (
        0x537190,
        Init::Square {
            src: 0x80902c,
            dst: 0xc0d47c,
        },
    ),
    (
        0x5371c0,
        Init::Square {
            src: 0x809030,
            dst: 0xc0d498,
        },
    ),
    (
        0x537210,
        Init::Square {
            src: 0x809038,
            dst: 0xc0d478,
        },
    ),
    (
        0x537240,
        Init::Square {
            src: 0x80903c,
            dst: 0xc0d4a0,
        },
    ),
    (
        0x537270,
        Init::Square {
            src: 0x809040,
            dst: 0xc0d4b0,
        },
    ),
    (
        0x5372a0,
        Init::Square {
            src: 0x809034,
            dst: 0xc0d42c,
        },
    ),
    (
        0x5372d0,
        Init::Square {
            src: 0x80906c,
            dst: 0xc0d430,
        },
    ),
    (
        0x537300,
        Init::Quotient {
            num: 0x7ffa24,
            den: 0x80906c,
            dst: 0xc0d434,
        },
    ),
    (0x537330, Init::ListHead { head: 0xc0d488 }),
    (
        0x5380f0,
        Init::Double {
            src: 0x8090e0,
            dst: 0xc0d4f8,
        },
    ),
    (
        0x538110,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc0d4f8,
            dst: 0xc0d4d0,
        },
    ),
    (0x538140, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x538190,
        Init::Square {
            src: 0x8090e4,
            dst: 0xc0d4f0,
        },
    ),
    (
        0x5381c0,
        Init::Square {
            src: 0x8090e8,
            dst: 0xc0d504,
        },
    ),
    (
        0x5381f0,
        Init::Square {
            src: 0x8090ec,
            dst: 0xc0d4d4,
        },
    ),
    (
        0x538220,
        Init::Square {
            src: 0x8090f0,
            dst: 0xc0d4e8,
        },
    ),
    (
        0x538250,
        Init::Square {
            src: 0x8090f4,
            dst: 0xc0d500,
        },
    ),
    (
        0x538280,
        Init::Square {
            src: 0x8090f8,
            dst: 0xc0d4b8,
        },
    ),
    (
        0x5382b0,
        Init::Square {
            src: 0x8090fc,
            dst: 0xc0d508,
        },
    ),
    (
        0x5382e0,
        Init::Square {
            src: 0x809100,
            dst: 0xc0d4c4,
        },
    ),
    (
        0x538310,
        Init::Square {
            src: 0x809104,
            dst: 0xc0d4dc,
        },
    ),
    (
        0x538340,
        Init::Square {
            src: 0x809108,
            dst: 0xc0d4cc,
        },
    ),
    (
        0x538370,
        Init::Square {
            src: 0x80910c,
            dst: 0xc0d4ec,
        },
    ),
    (
        0x5383a0,
        Init::Square {
            src: 0x809110,
            dst: 0xc0d4e4,
        },
    ),
    (
        0x5383d0,
        Init::Square {
            src: 0x809114,
            dst: 0xc0d4f4,
        },
    ),
    (
        0x538420,
        Init::Square {
            src: 0x80911c,
            dst: 0xc0d4e0,
        },
    ),
    (
        0x538450,
        Init::Square {
            src: 0x809120,
            dst: 0xc0d4fc,
        },
    ),
    (
        0x538480,
        Init::Square {
            src: 0x809124,
            dst: 0xc0d50c,
        },
    ),
    (
        0x5384b0,
        Init::Square {
            src: 0x809118,
            dst: 0xc0d4bc,
        },
    ),
    (
        0x5a9e30,
        Init::Double {
            src: 0x80a2f0,
            dst: 0xc280fc,
        },
    ),
    (
        0x5a9e50,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc280fc,
            dst: 0xc27d68,
        },
    ),
    (
        0x5a9ee0,
        Init::Square {
            src: 0x80a2f4,
            dst: 0xc280f4,
        },
    ),
    (
        0x5a9f10,
        Init::Square {
            src: 0x80a2f8,
            dst: 0xc28108,
        },
    ),
    (
        0x5a9f40,
        Init::Square {
            src: 0x80a2fc,
            dst: 0xc27d6c,
        },
    ),
    (
        0x5a9f70,
        Init::Square {
            src: 0x80a300,
            dst: 0xc27eb0,
        },
    ),
    (
        0x5a9fa0,
        Init::Square {
            src: 0x80a304,
            dst: 0xc28104,
        },
    ),
    (
        0x5a9fd0,
        Init::Square {
            src: 0x80a308,
            dst: 0xc27d00,
        },
    ),
    (
        0x5aa000,
        Init::Square {
            src: 0x80a30c,
            dst: 0xc2810c,
        },
    ),
    (
        0x5aa030,
        Init::Square {
            src: 0x80a310,
            dst: 0xc27d5c,
        },
    ),
    (
        0x5aa060,
        Init::Square {
            src: 0x80a314,
            dst: 0xc27d74,
        },
    ),
    (
        0x5aa090,
        Init::Square {
            src: 0x80a318,
            dst: 0xc27d64,
        },
    ),
    (
        0x5aa0c0,
        Init::Square {
            src: 0x80a31c,
            dst: 0xc27eb4,
        },
    ),
    (
        0x5aa0f0,
        Init::Square {
            src: 0x80a320,
            dst: 0xc27d7c,
        },
    ),
    (
        0x5aa120,
        Init::Square {
            src: 0x80a324,
            dst: 0xc280f8,
        },
    ),
    (
        0x5aa170,
        Init::Square {
            src: 0x80a32c,
            dst: 0xc27d78,
        },
    ),
    (
        0x5aa1a0,
        Init::Square {
            src: 0x80a330,
            dst: 0xc28100,
        },
    ),
    (
        0x5aa1d0,
        Init::Square {
            src: 0x80a334,
            dst: 0xc28110,
        },
    ),
    (
        0x5aa200,
        Init::Square {
            src: 0x80a328,
            dst: 0xc27d48,
        },
    ),
    (0x5aa230, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x5a9e80,
        Init::Square {
            src: 0x80a338,
            dst: 0xc27d4c,
        },
    ),
    (
        0x5a9eb0,
        Init::Quotient {
            num: 0x7ffa24,
            den: 0x80a338,
            dst: 0xc27d54,
        },
    ),
    (
        0x5ac5d0,
        Init::Double {
            src: 0x80a430,
            dst: 0xc2a108,
        },
    ),
    (
        0x5ac5f0,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a108,
            dst: 0xc28158,
        },
    ),
    (
        0x5ac620,
        Init::Square {
            src: 0x80a434,
            dst: 0xc2a100,
        },
    ),
    (
        0x5ac650,
        Init::Square {
            src: 0x80a438,
            dst: 0xc2a114,
        },
    ),
    (
        0x5ac680,
        Init::Square {
            src: 0x80a43c,
            dst: 0xc2815c,
        },
    ),
    (
        0x5ac6b0,
        Init::Square {
            src: 0x80a440,
            dst: 0xc28174,
        },
    ),
    (
        0x5ac6e0,
        Init::Square {
            src: 0x80a444,
            dst: 0xc2a110,
        },
    ),
    (
        0x5ac710,
        Init::Square {
            src: 0x80a448,
            dst: 0xc28138,
        },
    ),
    (
        0x5ac740,
        Init::Square {
            src: 0x80a44c,
            dst: 0xc2a118,
        },
    ),
    (
        0x5ac770,
        Init::Square {
            src: 0x80a450,
            dst: 0xc2814c,
        },
    ),
    (
        0x5ac7a0,
        Init::Square {
            src: 0x80a454,
            dst: 0xc28164,
        },
    ),
    (
        0x5ac7d0,
        Init::Square {
            src: 0x80a458,
            dst: 0xc28154,
        },
    ),
    (
        0x5ac800,
        Init::Square {
            src: 0x80a45c,
            dst: 0xc28178,
        },
    ),
    (
        0x5ac830,
        Init::Square {
            src: 0x80a460,
            dst: 0xc28170,
        },
    ),
    (
        0x5ac860,
        Init::Square {
            src: 0x80a464,
            dst: 0xc2a104,
        },
    ),
    (
        0x5ac8b0,
        Init::Square {
            src: 0x80a46c,
            dst: 0xc2816c,
        },
    ),
    (
        0x5ac8e0,
        Init::Square {
            src: 0x80a470,
            dst: 0xc2a10c,
        },
    ),
    (
        0x5ac910,
        Init::Square {
            src: 0x80a474,
            dst: 0xc2a11c,
        },
    ),
    (
        0x5ac940,
        Init::Square {
            src: 0x80a468,
            dst: 0xc2813c,
        },
    ),
    (0x5ac9d0, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x5ac970,
        Init::Square {
            src: 0x80a4a0,
            dst: 0xc28140,
        },
    ),
    (
        0x5ac9a0,
        Init::Quotient {
            num: 0x7ffa24,
            den: 0x80a4a0,
            dst: 0xc28144,
        },
    ),
    (
        0x5affb0,
        Init::Double {
            src: 0x80a4a8,
            dst: 0xc2a170,
        },
    ),
    (
        0x5affd0,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a170,
            dst: 0xc2a148,
        },
    ),
    (
        0x5b0000,
        Init::Square {
            src: 0x80a4ac,
            dst: 0xc2a168,
        },
    ),
    (
        0x5b0030,
        Init::Square {
            src: 0x80a4b0,
            dst: 0xc2a17c,
        },
    ),
    (
        0x5b0060,
        Init::Square {
            src: 0x80a4b4,
            dst: 0xc2a14c,
        },
    ),
    (
        0x5b0090,
        Init::Square {
            src: 0x80a4b8,
            dst: 0xc2a160,
        },
    ),
    (
        0x5b00c0,
        Init::Square {
            src: 0x80a4bc,
            dst: 0xc2a178,
        },
    ),
    (
        0x5b00f0,
        Init::Square {
            src: 0x80a4c0,
            dst: 0xc2a130,
        },
    ),
    (
        0x5b0120,
        Init::Square {
            src: 0x80a4c4,
            dst: 0xc2a180,
        },
    ),
    (
        0x5b0150,
        Init::Square {
            src: 0x80a4c8,
            dst: 0xc2a13c,
        },
    ),
    (
        0x5b0180,
        Init::Square {
            src: 0x80a4cc,
            dst: 0xc2a154,
        },
    ),
    (
        0x5b01b0,
        Init::Square {
            src: 0x80a4d0,
            dst: 0xc2a144,
        },
    ),
    (
        0x5b01e0,
        Init::Square {
            src: 0x80a4d4,
            dst: 0xc2a164,
        },
    ),
    (
        0x5b0210,
        Init::Square {
            src: 0x80a4d8,
            dst: 0xc2a15c,
        },
    ),
    (
        0x5b0240,
        Init::Square {
            src: 0x80a4dc,
            dst: 0xc2a16c,
        },
    ),
    (
        0x5b0290,
        Init::Square {
            src: 0x80a4e4,
            dst: 0xc2a158,
        },
    ),
    (
        0x5b02c0,
        Init::Square {
            src: 0x80a4e8,
            dst: 0xc2a174,
        },
    ),
    (
        0x5b02f0,
        Init::Square {
            src: 0x80a4ec,
            dst: 0xc2a184,
        },
    ),
    (
        0x5b0320,
        Init::Square {
            src: 0x80a4e0,
            dst: 0xc2a134,
        },
    ),
    (0x5b0350, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x5b0d70,
        Init::Double {
            src: 0x80a4f4,
            dst: 0xc2a1e0,
        },
    ),
    (
        0x5b0d90,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a1e0,
            dst: 0xc2a1b8,
        },
    ),
    (
        0x5b0dc0,
        Init::Square {
            src: 0x80a4f8,
            dst: 0xc2a1d8,
        },
    ),
    (
        0x5b0df0,
        Init::Square {
            src: 0x80a4fc,
            dst: 0xc2a1ec,
        },
    ),
    (
        0x5b0e20,
        Init::Square {
            src: 0x80a500,
            dst: 0xc2a1bc,
        },
    ),
    (
        0x5b0e50,
        Init::Square {
            src: 0x80a504,
            dst: 0xc2a1d0,
        },
    ),
    (
        0x5b0e80,
        Init::Square {
            src: 0x80a508,
            dst: 0xc2a1e8,
        },
    ),
    (
        0x5b0eb0,
        Init::Square {
            src: 0x80a50c,
            dst: 0xc2a188,
        },
    ),
    (
        0x5b0ee0,
        Init::Square {
            src: 0x80a510,
            dst: 0xc2a1f0,
        },
    ),
    (
        0x5b0f10,
        Init::Square {
            src: 0x80a514,
            dst: 0xc2a194,
        },
    ),
    (
        0x5b0f40,
        Init::Square {
            src: 0x80a518,
            dst: 0xc2a1c4,
        },
    ),
    (
        0x5b0f70,
        Init::Square {
            src: 0x80a51c,
            dst: 0xc2a19c,
        },
    ),
    (
        0x5b0fa0,
        Init::Square {
            src: 0x80a520,
            dst: 0xc2a1d4,
        },
    ),
    (
        0x5b0fd0,
        Init::Square {
            src: 0x80a524,
            dst: 0xc2a1cc,
        },
    ),
    (
        0x5b1000,
        Init::Square {
            src: 0x80a528,
            dst: 0xc2a1dc,
        },
    ),
    (
        0x5b1050,
        Init::Square {
            src: 0x80a530,
            dst: 0xc2a1c8,
        },
    ),
    (
        0x5b1080,
        Init::Square {
            src: 0x80a534,
            dst: 0xc2a1e4,
        },
    ),
    (
        0x5b10b0,
        Init::Square {
            src: 0x80a538,
            dst: 0xc2a1f4,
        },
    ),
    (
        0x5b10e0,
        Init::Square {
            src: 0x80a52c,
            dst: 0xc2a18c,
        },
    ),
    (0x5b1110, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x5b1300,
        Init::Double {
            src: 0x80a540,
            dst: 0xc2a238,
        },
    ),
    (
        0x5b1320,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a238,
            dst: 0xc2a210,
        },
    ),
    (
        0x5b1350,
        Init::Square {
            src: 0x80a544,
            dst: 0xc2a230,
        },
    ),
    (
        0x5b1380,
        Init::Square {
            src: 0x80a548,
            dst: 0xc2a244,
        },
    ),
    (
        0x5b13b0,
        Init::Square {
            src: 0x80a54c,
            dst: 0xc2a214,
        },
    ),
    (
        0x5b13e0,
        Init::Square {
            src: 0x80a550,
            dst: 0xc2a228,
        },
    ),
    (
        0x5b1410,
        Init::Square {
            src: 0x80a554,
            dst: 0xc2a240,
        },
    ),
    (
        0x5b1440,
        Init::Square {
            src: 0x80a558,
            dst: 0xc2a1f8,
        },
    ),
    (
        0x5b1470,
        Init::Square {
            src: 0x80a55c,
            dst: 0xc2a248,
        },
    ),
    (
        0x5b14a0,
        Init::Square {
            src: 0x80a560,
            dst: 0xc2a204,
        },
    ),
    (
        0x5b14d0,
        Init::Square {
            src: 0x80a564,
            dst: 0xc2a21c,
        },
    ),
    (
        0x5b1500,
        Init::Square {
            src: 0x80a568,
            dst: 0xc2a20c,
        },
    ),
    (
        0x5b1530,
        Init::Square {
            src: 0x80a56c,
            dst: 0xc2a22c,
        },
    ),
    (
        0x5b1560,
        Init::Square {
            src: 0x80a570,
            dst: 0xc2a224,
        },
    ),
    (
        0x5b1590,
        Init::Square {
            src: 0x80a574,
            dst: 0xc2a234,
        },
    ),
    (
        0x5b15e0,
        Init::Square {
            src: 0x80a57c,
            dst: 0xc2a220,
        },
    ),
    (
        0x5b1610,
        Init::Square {
            src: 0x80a580,
            dst: 0xc2a23c,
        },
    ),
    (
        0x5b1640,
        Init::Square {
            src: 0x80a584,
            dst: 0xc2a24c,
        },
    ),
    (
        0x5b1670,
        Init::Square {
            src: 0x80a578,
            dst: 0xc2a1fc,
        },
    ),
    (0x5b16a0, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x5b1710,
        Init::Double {
            src: 0x80a58c,
            dst: 0xc2a290,
        },
    ),
    (
        0x5b1730,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a290,
            dst: 0xc2a268,
        },
    ),
    (0x5b1760, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x5b17b0,
        Init::Square {
            src: 0x80a590,
            dst: 0xc2a288,
        },
    ),
    (
        0x5b17e0,
        Init::Square {
            src: 0x80a594,
            dst: 0xc2a29c,
        },
    ),
    (
        0x5b1810,
        Init::Square {
            src: 0x80a598,
            dst: 0xc2a26c,
        },
    ),
    (
        0x5b1840,
        Init::Square {
            src: 0x80a59c,
            dst: 0xc2a280,
        },
    ),
    (
        0x5b1870,
        Init::Square {
            src: 0x80a5a0,
            dst: 0xc2a298,
        },
    ),
    (
        0x5b18a0,
        Init::Square {
            src: 0x80a5a4,
            dst: 0xc2a250,
        },
    ),
    (
        0x5b18d0,
        Init::Square {
            src: 0x80a5a8,
            dst: 0xc2a2a0,
        },
    ),
    (
        0x5b1900,
        Init::Square {
            src: 0x80a5ac,
            dst: 0xc2a25c,
        },
    ),
    (
        0x5b1930,
        Init::Square {
            src: 0x80a5b0,
            dst: 0xc2a274,
        },
    ),
    (
        0x5b1960,
        Init::Square {
            src: 0x80a5b4,
            dst: 0xc2a264,
        },
    ),
    (
        0x5b1990,
        Init::Square {
            src: 0x80a5b8,
            dst: 0xc2a284,
        },
    ),
    (
        0x5b19c0,
        Init::Square {
            src: 0x80a5bc,
            dst: 0xc2a27c,
        },
    ),
    (
        0x5b19f0,
        Init::Square {
            src: 0x80a5c0,
            dst: 0xc2a28c,
        },
    ),
    (
        0x5b1a40,
        Init::Square {
            src: 0x80a5c8,
            dst: 0xc2a278,
        },
    ),
    (
        0x5b1a70,
        Init::Square {
            src: 0x80a5cc,
            dst: 0xc2a294,
        },
    ),
    (
        0x5b1aa0,
        Init::Square {
            src: 0x80a5d0,
            dst: 0xc2a2a8,
        },
    ),
    (
        0x5b1ad0,
        Init::Square {
            src: 0x80a5c4,
            dst: 0xc2a254,
        },
    ),
    (
        0x5b3550,
        Init::Double {
            src: 0x80a610,
            dst: 0xc2a2f8,
        },
    ),
    (
        0x5b3570,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a2f8,
            dst: 0xc2a2c4,
        },
    ),
    (0x5b35a0, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x5b35f0,
        Init::Square {
            src: 0x80a614,
            dst: 0xc2a2f0,
        },
    ),
    (
        0x5b3620,
        Init::Square {
            src: 0x80a618,
            dst: 0xc2a304,
        },
    ),
    (
        0x5b3650,
        Init::Square {
            src: 0x80a61c,
            dst: 0xc2a2c8,
        },
    ),
    (
        0x5b3680,
        Init::Square {
            src: 0x80a620,
            dst: 0xc2a2e8,
        },
    ),
    (
        0x5b36b0,
        Init::Square {
            src: 0x80a624,
            dst: 0xc2a300,
        },
    ),
    (
        0x5b36e0,
        Init::Square {
            src: 0x80a628,
            dst: 0xc2a2ac,
        },
    ),
    (
        0x5b3710,
        Init::Square {
            src: 0x80a62c,
            dst: 0xc2a308,
        },
    ),
    (
        0x5b3740,
        Init::Square {
            src: 0x80a630,
            dst: 0xc2a2b8,
        },
    ),
    (
        0x5b3770,
        Init::Square {
            src: 0x80a634,
            dst: 0xc2a2dc,
        },
    ),
    (
        0x5b37a0,
        Init::Square {
            src: 0x80a638,
            dst: 0xc2a2c0,
        },
    ),
    (
        0x5b37d0,
        Init::Square {
            src: 0x80a63c,
            dst: 0xc2a2ec,
        },
    ),
    (
        0x5b3800,
        Init::Square {
            src: 0x80a640,
            dst: 0xc2a2e4,
        },
    ),
    (
        0x5b3830,
        Init::Square {
            src: 0x80a644,
            dst: 0xc2a2f4,
        },
    ),
    (
        0x5b3880,
        Init::Square {
            src: 0x80a64c,
            dst: 0xc2a2e0,
        },
    ),
    (
        0x5b38b0,
        Init::Square {
            src: 0x80a650,
            dst: 0xc2a2fc,
        },
    ),
    (
        0x5b38e0,
        Init::Square {
            src: 0x80a654,
            dst: 0xc2a310,
        },
    ),
    (
        0x5b3910,
        Init::Square {
            src: 0x80a648,
            dst: 0xc2a2b0,
        },
    ),
    (0x5b3940, Init::ListHead { head: 0xc2a2d0 }),
    (
        0x5b6fe0,
        Init::Double {
            src: 0x80a6b8,
            dst: 0xc2a394,
        },
    ),
    (
        0x5b7060,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a394,
            dst: 0xc2a344,
        },
    ),
    (0x5b4910, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x5b4960,
        Init::Square {
            src: 0x80a6bc,
            dst: 0xc2a38c,
        },
    ),
    (
        0x5b4990,
        Init::Square {
            src: 0x80a6c0,
            dst: 0xc2a3a0,
        },
    ),
    (
        0x5b49c0,
        Init::Square {
            src: 0x80a6c4,
            dst: 0xc2a348,
        },
    ),
    (
        0x5b49f0,
        Init::Square {
            src: 0x80a6c8,
            dst: 0xc2a384,
        },
    ),
    (
        0x5b4a20,
        Init::Square {
            src: 0x80a6cc,
            dst: 0xc2a39c,
        },
    ),
    (
        0x5b4a50,
        Init::Square {
            src: 0x80a6d0,
            dst: 0xc2a32c,
        },
    ),
    (
        0x5b4a80,
        Init::Square {
            src: 0x80a6d4,
            dst: 0xc2a3a4,
        },
    ),
    (
        0x5b4ab0,
        Init::Square {
            src: 0x80a6d8,
            dst: 0xc2a338,
        },
    ),
    (
        0x5b4ae0,
        Init::Square {
            src: 0x80a6dc,
            dst: 0xc2a350,
        },
    ),
    (
        0x5b4b10,
        Init::Square {
            src: 0x80a6e0,
            dst: 0xc2a340,
        },
    ),
    (
        0x5b4b40,
        Init::Square {
            src: 0x80a6e4,
            dst: 0xc2a388,
        },
    ),
    (
        0x5b4b70,
        Init::Square {
            src: 0x80a6e8,
            dst: 0xc2a380,
        },
    ),
    (
        0x5b4ba0,
        Init::Square {
            src: 0x80a6ec,
            dst: 0xc2a390,
        },
    ),
    (
        0x5b4bf0,
        Init::Square {
            src: 0x80a6f4,
            dst: 0xc2a37c,
        },
    ),
    (
        0x5b4c20,
        Init::Square {
            src: 0x80a6f8,
            dst: 0xc2a398,
        },
    ),
    (
        0x5b4c50,
        Init::Square {
            src: 0x80a6fc,
            dst: 0xc2a3a8,
        },
    ),
    (
        0x5b4c80,
        Init::Square {
            src: 0x80a6f0,
            dst: 0xc2a330,
        },
    ),
    (
        0x5b7ae0,
        Init::Double {
            src: 0x80a70c,
            dst: 0xc2a3d0,
        },
    ),
    (
        0x5b7b00,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a3d0,
            dst: 0xc2a3cc,
        },
    ),
    (
        0x5b7b50,
        Init::Double {
            src: 0x80a714,
            dst: 0xc2a3e8,
        },
    ),
    (
        0x5b7b70,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a3e8,
            dst: 0xc2a3e4,
        },
    ),
    (
        0x5b7c10,
        Init::Double {
            src: 0x80a71c,
            dst: 0xc2a470,
        },
    ),
    (
        0x5b7c30,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a470,
            dst: 0xc2a408,
        },
    ),
    (0x5b7c60, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x5b7cb0,
        Init::Square {
            src: 0x80a720,
            dst: 0xc2a468,
        },
    ),
    (
        0x5b7ce0,
        Init::Square {
            src: 0x80a724,
            dst: 0xc2a47c,
        },
    ),
    (
        0x5b7d10,
        Init::Square {
            src: 0x80a728,
            dst: 0xc2a40c,
        },
    ),
    (
        0x5b7d40,
        Init::Square {
            src: 0x80a72c,
            dst: 0xc2a420,
        },
    ),
    (
        0x5b7d70,
        Init::Square {
            src: 0x80a730,
            dst: 0xc2a478,
        },
    ),
    (
        0x5b7da0,
        Init::Square {
            src: 0x80a734,
            dst: 0xc2a3f0,
        },
    ),
    (
        0x5b7dd0,
        Init::Square {
            src: 0x80a738,
            dst: 0xc2a480,
        },
    ),
    (
        0x5b7e00,
        Init::Square {
            src: 0x80a73c,
            dst: 0xc2a3fc,
        },
    ),
    (
        0x5b7e30,
        Init::Square {
            src: 0x80a740,
            dst: 0xc2a414,
        },
    ),
    (
        0x5b7e60,
        Init::Square {
            src: 0x80a744,
            dst: 0xc2a404,
        },
    ),
    (
        0x5b7e90,
        Init::Square {
            src: 0x80a748,
            dst: 0xc2a424,
        },
    ),
    (
        0x5b7ec0,
        Init::Square {
            src: 0x80a74c,
            dst: 0xc2a41c,
        },
    ),
    (
        0x5b7ef0,
        Init::Square {
            src: 0x80a750,
            dst: 0xc2a46c,
        },
    ),
    (
        0x5b7f40,
        Init::Square {
            src: 0x80a758,
            dst: 0xc2a418,
        },
    ),
    (
        0x5b7f70,
        Init::Square {
            src: 0x80a75c,
            dst: 0xc2a474,
        },
    ),
    (
        0x5b7fa0,
        Init::Square {
            src: 0x80a760,
            dst: 0xc2a484,
        },
    ),
    (
        0x5b7fd0,
        Init::Square {
            src: 0x80a754,
            dst: 0xc2a3f4,
        },
    ),
    (
        0x5bd490,
        Init::Double {
            src: 0x80a804,
            dst: 0xc2a628,
        },
    ),
    (
        0x5bd4b0,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a628,
            dst: 0xc2a624,
        },
    ),
    (0x5bc9a0, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x5be1b0,
        Init::Double {
            src: 0x80a80c,
            dst: 0xc2a640,
        },
    ),
    (
        0x5be1d0,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a640,
            dst: 0xc2a63c,
        },
    ),
    (0x5bd5b0, Init::CountUnit { counter: 0xc2a3c4 }),
    (
        0x5be220,
        Init::Double {
            src: 0x80a814,
            dst: 0xc2a64c,
        },
    ),
    (
        0x5be240,
        Init::Quotient {
            num: 0x7ff9d8,
            den: 0xc2a64c,
            dst: 0xc2a648,
        },
    ),
];

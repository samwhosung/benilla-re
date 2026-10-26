//! The C runtime routines of the client whose output the player sees (`systems/crt/`): the
//! `printf` engine and its floating-point conversions, and the decimal digits of a `double` the
//! runtime makes in its own 12-byte real.
//!
//! The x87 is not used by these routines past a load and a store of the `double`: the digits are
//! made in integer arithmetic.

pub mod cvt;
pub mod fltout;
pub mod ld12;
pub mod output;

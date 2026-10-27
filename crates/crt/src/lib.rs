//! The C runtime routines of the client whose results the player sees or a script reads
//! (`systems/crt/`): the `printf` engine and its floating-point conversions, the decimal digits of
//! a `double` the runtime makes in its own 12-byte real, `strtod` and the 12-byte real it reads
//! text into, `sscanf`'s engine, `frexp`, `fmod` and the non-finite cases of `pow`, calendar time
//! and `strftime`, and the C locale's character classes.
//!
//! The x87 is not used by the number routines past a load and a store of the `double`: the digits
//! and the values are made in integer arithmetic.

pub mod ctype;
pub mod cvt;
pub mod fltout;
pub mod ld12;
pub mod math;
pub mod output;
pub mod scanf;
pub mod strtod;
pub mod time;

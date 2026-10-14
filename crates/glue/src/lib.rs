//! The arithmetic of the client's login, realm and character screens (`systems/glue/`): the screen
//! aspect, the ghost kit's tint and alpha, the select screen's pet scale, the facing conversions of
//! the script functions, the realm load bands and the cursor scale.
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with an `as f32` at each store. A value the client pushes to Lua with
//! `fstp qword` stays an `f64`.

#![forbid(unsafe_code)]

pub mod aspect;
pub mod customize;
pub mod init;
pub mod lua_scale;
pub mod realm;

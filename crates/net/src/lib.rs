//! The wire primitives of the client's network code (`systems/net/`): SHA-1, the size header and
//! the stream cipher of the world connection, the logon proofs and the random pool, and the SRP6
//! session. MD5 and CRC-32, which Storm supplies, are in `benilla-re-storm`.
//!
//! The big-number arithmetic of the SRP6 session is Storm's BigNumber in the exe; here it is
//! `num-bigint`, and the session test compares the two end to end.

#![forbid(unsafe_code)]

mod cipher;
mod digest;
mod framing;
mod login;
mod login_proof;
mod prng;
mod srp;
mod srp_helpers;
mod srp_session;

pub use cipher::*;
pub use digest::*;
pub use framing::*;
pub use login::*;
pub use login_proof::*;
pub use prng::*;
pub use srp::*;
pub use srp_helpers::*;
pub use srp_session::*;

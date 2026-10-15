//! The wire primitives of the client's network code (`systems/net/`): SHA-1, the size header and
//! the stream cipher of the world connection, the logon proofs and the random pool, and the SRP6
//! session. MD5 and CRC-32, which Storm supplies, are in `benilla-re-storm`.
//!
//! The big-number arithmetic of the SRP6 session is Storm's BigNumber in the exe; here it is
//! `num-bigint`, and the session test compares the two end to end.

#![forbid(unsafe_code)]

mod addresslist;
mod cipher;
mod connection;
mod datastore;
mod digest;
mod download;
mod framing;
mod friends;
mod grunt;
mod initializers;
mod login;
mod login_proof;
mod netclient;
mod prng;
mod realms;
mod srp;
mod srp_helpers;
mod srp_session;
mod telemetry;
mod who;

pub use addresslist::*;
pub use cipher::*;
pub use connection::*;
pub use datastore::*;
pub use digest::*;
pub use download::*;
pub use framing::*;
pub use friends::*;
pub use grunt::*;
pub use initializers::*;
pub use login::*;
pub use login_proof::*;
pub use netclient::*;
pub use prng::*;
pub use realms::*;
pub use srp::*;
pub use srp_helpers::*;
pub use srp_session::*;
pub use telemetry::*;
pub use who::*;

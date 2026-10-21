//! The record decoders of the client's server-streamed database caches (`DBCache.cpp` and the record
//! files `CreatureStats.cpp`, `GameObjectStats.cpp`, `ItemName.cpp`, `ItemStats.cpp` and `NPCText.cpp`).
//! Each decoder reads one `SMSG_*_QUERY_RESPONSE` record from a `CDataStore` and fills the cache's
//! `<Type>_C` struct, in the exe's read order and at its field offsets. The cache itself (the hash,
//! the node list and the `.wdb` files) is not here; [`callbacks`] models the callback lists. The
//! `CDataStore::Get*` reads belong to the network layer; [`datastore`] models them so a decoder can
//! be compared on its own.

pub mod callbacks;
pub mod datastore;
pub mod decode;
pub mod decode_inline;
pub mod decode_special;
pub mod decode_strptr;

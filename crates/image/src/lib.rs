//! The image readers and pixel-format converters of the client: the TGA decoder
//! (`Images\tga.cpp`), the paletted-to-RGB and DXT block converters of the BLP path, the mip-chain
//! geometry and the format-conversion matrix that turn texture-file bytes into the device's
//! preferred pixel format.
//!
//! Each function transcribes one client function. File input and output (the archive layer), the
//! heap, and the device format choice and upload (the graphics layer) are not transcribed; only the
//! integer codecs are.

#![forbid(unsafe_code)]

pub mod convert;
pub mod convertmatrix;
pub mod dxt;
pub mod mipalloc;
pub mod mipgeo;
pub mod palconv;
pub mod tga;

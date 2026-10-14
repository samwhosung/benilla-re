//! The lighting of the client (`systems/lighting/`): the time-of-day light, the sky, the fog, the
//! weather and the shadows. Each module names the functions it covers.
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with a final `as f32`. A value the client keeps in a register after an
//! `fst` stays wide.

#![forbid(unsafe_code)]

mod x87;

pub mod clouds;
pub mod dn7e_fp;
pub mod dn_angles;
pub mod dn_bands;
pub mod dn_color;
pub mod dn_color_table;
pub mod dn_init;
pub mod dn_setters;
pub mod dn_sky;
pub mod dn_sky_band;
pub mod dn_sky_blend;
pub mod dn_sky_fog;
pub mod dn_sky_mtx;
pub mod dn_sky_shadow;
pub mod dn_sky_uv;
pub mod dn_sky_vtx;
pub mod mist;
pub mod shadow_blob_fp;
pub mod sky_fade;
pub mod sky_leaves;
pub mod weather_init;
pub mod weather_kernels;
pub mod weather_scalars;
pub mod wx_driver;
pub mod wx_leaf_fp;
pub mod wx_mistrender;
pub mod wx_mistsand;
pub mod wx_raindrops;
pub mod wx_rainrender;
pub mod wx_rainspawn;
pub mod wx_sandspawn;
pub mod wx_snow;
pub mod wx_snowrender;
pub mod wx_snowspawn;

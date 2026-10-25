//! The texture layer's size and format decisions (`systems/gx/textures.md`): the device size
//! limit `0x448b60`, the BLP format chooser `0x44a790`, the device texture request gate
//! `0x448450` and the creation checks of `GxTexCreate` (`0x58ab10`).

/// The `GxTex` format a pixel type code maps to (`0x835258`, indexed by the code).
pub const TYPE_CODE_FORMAT: [u32; 8] = [5, 6, 1, 3, 2, 4, 0, 7];

/// `0x448b60`: halve `width` and `height` together until neither exceeds `limit`, adding one to
/// `dropped` for each halving and keeping each side at 1 or more. A limit of 0 is an error
/// (`0x448b8f`) and changes nothing.
pub fn limit_size(width: u32, height: u32, dropped: u32, limit: u32) -> Option<(u32, u32, u32)> {
    if limit == 0 {
        return None;
    }
    let (mut w, mut h, mut d) = (width, height, dropped);
    while w > limit || h > limit {
        w >>= 1;
        h >>= 1;
        d = d.wrapping_add(1);
        w = w.max(1);
        h = h.max(1);
    }
    Some((w, h, d))
}

/// What `0x44a790` stores: the pixel type code (the first output) and the `GxTex` format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatChoice {
    pub type_code: u32,
    pub format: u32,
}

/// `0x44a790(alpha_type, alpha_depth)` with the device's DXT1, DXT3 and DXT5 flags (`caps + 0x40`,
/// `+0x44`, `+0x48`). `None` is an alpha type of 6 or above 8, where it stores nothing.
pub fn choose_format(
    alpha_type: u32,
    alpha_depth: u32,
    dxt1: bool,
    dxt3: bool,
    dxt5: bool,
) -> Option<FormatChoice> {
    let c = |type_code, format| Some(FormatChoice { type_code, format });
    match alpha_type {
        0 => {
            if dxt1 {
                c(0, 5)
            } else if alpha_depth == 0 {
                c(5, 4)
            } else {
                c(3, 3)
            }
        }
        1 => {
            if dxt3 {
                c(1, 6)
            } else {
                c(4, 2)
            }
        }
        2 => c(2, 1),
        3 => c(2, 3),
        4 => c(2, 2),
        5 => c(2, 4),
        7 => {
            if dxt5 {
                c(7, 7)
            } else {
                c(4, 2)
            }
        }
        8 => match alpha_depth {
            0 => c(2, 4),
            1 => c(2, 3),
            4 => c(2, 2),
            _ => c(2, 1),
        },
        _ => None,
    }
}

/// What `0x448450` does with a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolRoute {
    /// Returns 0 at `0x448665`: no texture is made.
    Refused,
    /// Goes to `GxTexCreate` through `0x58ac40` (`0x44864b`).
    Create,
    /// Looks in pool list `class` for a free texture, and creates one when none matches.
    Pool(u32),
}

fn power_of_two_or_zero(v: u32) -> bool {
    v & v.wrapping_sub(1) == 0
}

/// The route of a `width` by `height` request for the device format `format` (`0x4484ae` to
/// `0x4484e8`). The format compares as a signed value.
pub fn pool_route(width: u32, height: u32, format: u32) -> PoolRoute {
    if !power_of_two_or_zero(width) || !power_of_two_or_zero(height) {
        return PoolRoute::Refused;
    }
    if width >= 1024 || height >= 1024 {
        return PoolRoute::Refused;
    }
    if width <= 16 || height <= 16 || (format as i32) > 7 {
        return PoolRoute::Create;
    }
    let wlog = (width >> 5).trailing_zeros();
    let hlog = (height >> 5).trailing_zeros();
    PoolRoute::Pool(wlog * 5 + hlog)
}

/// The device facts `GxTexCreate` reads for a 2D texture.
#[derive(Clone, Copy, Debug)]
pub struct CreateCaps {
    /// `caps + 0x50`: the 2D texture type is supported (it must equal 1).
    pub type_supported: u32,
    /// `caps + 0x60`: the largest side.
    pub max_side: u32,
    /// `caps + 0x28`: the device generates mip maps.
    pub generates_mips: u32,
    /// `caps + 0xa8`: the device takes filter mode 5.
    pub mode5: u32,
}

/// One request to `GxTexCreate` for texture type 0.
#[derive(Clone, Copy, Debug)]
pub struct CreateRequest {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub data_format: u32,
    pub flags: u32,
    pub has_callback: bool,
}

/// Whether `GxTexCreate` (`0x58ab10`) hands a type 0 request to the device (`0x58ac22`) and does
/// not fail with error `0x57`.
pub fn create_accepts(r: &CreateRequest, caps: &CreateCaps) -> bool {
    if caps.type_supported != 1 {
        return false;
    }
    if r.width < 8 || r.height < 8 {
        return false;
    }
    if r.width > caps.max_side || r.height > caps.max_side {
        return false;
    }
    if !power_of_two_or_zero(r.width) || !power_of_two_or_zero(r.height) {
        return false;
    }
    let format = r.format as i32;
    if format > 9 {
        return false;
    }
    if r.flags & 0x40 != 0 {
        if caps.generates_mips == 0 {
            return false;
        }
        if (5..=7).contains(&format) {
            return false;
        }
    }
    if r.flags & 7 == 5 && caps.mode5 == 0 {
        return false;
    }
    if (r.data_format as i32) > 9 {
        return false;
    }
    r.has_callback
}

//! The mip levels the Direct3D device creates and uploads for a texture (`systems/gx/textures.md`,
//! "Device texture upload"): `0x5a0880`.

/// What `0x5a0880` stores through its four output pointers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureLevels {
    /// The top kept level's width: the record's width shifted right by `first`.
    pub width: u32,
    /// The top kept level's height, likewise.
    pub height: u32,
    /// The first level uploaded: the device's skip count, held to `count - 1`.
    pub first: u32,
    /// One past the last level; `end - first` levels are created and uploaded.
    pub end: u32,
}

/// `0x5a0880(tex, &width, &height, &first, &end)` on a device whose skip count `[dev+0x31c]` is
/// `skip`, for a record of `width` by `height` (`+0x24`, `+0x28`) and flags (`+0x3c`).
///
/// A filter mode (the low three bits) of 0 or 1, or bit 6, with bit 5 clear keeps one level at full
/// size. Otherwise the chain has `floor(log2(max(width, height))) + 1` levels, `first` is `skip` held
/// to the last of them, both sides shift right by it, and bit 5 ends the chain at `first + 1`.
/// `None` is a chain of a record 0 by 0, where the count loop never ends.
pub fn texture_levels(width: u32, height: u32, flags: u32, skip: u32) -> Option<TextureLevels> {
    let mode = flags & 7;
    if (mode == 0 || mode == 1 || flags & 0x40 != 0) && flags & 0x20 == 0 {
        return Some(TextureLevels {
            width,
            height,
            first: 0,
            end: 1,
        });
    }
    let longest = width.max(height);
    if longest == 0 {
        return None;
    }
    let count = longest.ilog2() + 1;
    let first = skip.min(count - 1);
    let end = if flags & 0x20 != 0 { first + 1 } else { count };
    Some(TextureLevels {
        width: width >> first,
        height: height >> first,
        first,
        end,
    })
}

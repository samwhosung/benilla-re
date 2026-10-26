//! The text block bridge onto the font (`systems/font/api.md`, Text block fonts and Text block
//! measures): the size and flags `TextBlockGenerateFont` (`0x44d040`) hands the factory, and the
//! flag map of the two measures `0x44d670` and `0x44d960`.

/// `0x44d06f` to `0x44d08f`: the size over the screen scale `s_screenScaleY`, the `fdiv` at 53 bits
/// stored as a float, replaced by 1.0 when the unrounded quotient is 1.0 or more or NaN.
pub fn font_size(size: f32, scale_y: f32) -> f32 {
    let q = f64::from(size) / f64::from(scale_y);
    if q < 1.0 {
        q as f32
    } else {
        1.0
    }
}

/// `0x44d1fa` to `0x44d219`: the low byte's bit 0 gives font flag 1, bit 2 gives 8, bit 1 gives 2.
pub fn font_flags(flags: u32) -> u32 {
    let mut f = 0;
    if flags & 1 != 0 {
        f = 1;
    }
    if flags & 4 != 0 {
        f |= 8;
    }
    if flags & 2 != 0 {
        f |= 2;
    }
    f
}

/// The measures' map from a text block's flags to the font's, `0x44d6a4` to `0x44d701` and
/// `0x44d996` to `0x44d9f3`; only the low 16 bits are read.
pub fn measure_flags(flags: u32) -> u32 {
    const MAP: [(u32, u32); 10] = [
        (0x0100, 0x001),
        (0x0200, 0x004),
        (0x0400, 0x008),
        (0x0800, 0x010),
        (0x0040, 0x002),
        (0x1000, 0x040),
        (0x2000, 0x100),
        (0x4000, 0x200),
        (0x8000, 0x400),
        (0x0080, 0x800),
    ];
    MAP.iter()
        .filter(|&&(from, _)| flags & from != 0)
        .fold(0, |f, &(_, to)| f | to)
}

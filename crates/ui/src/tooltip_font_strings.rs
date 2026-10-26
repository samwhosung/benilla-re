//! `AddFontStrings`' core `0x529940` (`systems/ui/tooltip.md#adding-font-strings`): the line limit
//! rises by one, each line array is resized to it, and the caller's pair and a clear wrap flag
//! become the last line.

/// A line array as the tooltip keeps it: capacity, count and data (`+0`, `+4`, `+8`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineArray {
    pub capacity: u32,
    pub count: u32,
    pub data: Vec<u32>,
}

/// The three arrays and the limit `+0x320`: left strings `+0x324`, right strings `+0x330` and
/// wrap flags `+0x33c`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lines {
    pub limit: u32,
    pub left: LineArray,
    pub right: LineArray,
    pub wrap: LineArray,
}

/// The resizes `0x529940` asks for, in order: the array (0 left, 1 right, 2 wrap) and the
/// capacity. Each resize sets the capacity, as `0x536c80` and `0x4368c0` do; the count is then
/// the limit.
pub fn add_font_strings(t: &mut Lines, left: u32, right: u32) -> Vec<(u8, u32)> {
    t.limit = t.limit.wrapping_add(1);
    let n = t.limit;
    let mut resizes = Vec::new();
    for (i, a) in [&mut t.left, &mut t.right, &mut t.wrap]
        .into_iter()
        .enumerate()
    {
        if a.count != n {
            a.capacity = n;
            a.count = n;
            resizes.push((i as u8, n));
        }
    }
    let last = (n - 1) as usize;
    t.left.data[last] = left;
    t.right.data[last] = right;
    t.wrap.data[last] = 0;
    resizes
}

//! `0x789860`, the slider's thumb install (`systems/ui/slider.md#the-thumb-texture`).

/// A call the install makes, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThumbCall {
    /// The old thumb's deleting destructor, slot 0, with 1.
    Delete(u32),
    /// `CScriptRegion::SetRegionParent(texture; slider, layer, 1)`.
    Link { texture: u32, layer: u32 },
    /// `ClearAllPoints` on the texture's layout part, `texture + 0x24`.
    ClearPoints(u32),
}

/// The thumb and the flags after installing `incoming` on `layer` over `current`, and the calls
/// made. The same texture changes nothing; otherwise flag 1 is set whatever `incoming` is.
pub fn install_thumb(
    current: u32,
    flags: u32,
    incoming: u32,
    layer: u32,
) -> (u32, u32, Vec<ThumbCall>) {
    if incoming == current {
        return (current, flags, Vec::new());
    }
    let mut calls = Vec::new();
    if current != 0 {
        calls.push(ThumbCall::Delete(current));
    }
    if incoming != 0 {
        calls.push(ThumbCall::Link {
            texture: incoming,
            layer,
        });
        calls.push(ThumbCall::ClearPoints(incoming.wrapping_add(0x24)));
    }
    (incoming, flags | 1, calls)
}

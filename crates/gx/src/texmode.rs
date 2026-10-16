//! The Direct3D device's texture coordinate modes (`systems/gx/renderstate.md#texture-coordinate-modes`):
//! `CGxDeviceD3d::ApplyTexCoordMode` (`0x5a2b40`) for the modes whose matrix it does not take from
//! the view (0 and 3 to 6; modes 1 and 2 are not transcribed).

/// The top matrix of a stage's mode matrix stack (`dev + 0x1bd8 + 0x118·stage`): its 16 floats, the
/// level's flags word (bit 0 set: identity) and the stack's dirty byte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageMatrix {
    pub matrix: [f32; 16],
    pub flags: u32,
    pub dirty: bool,
}

/// What a mode puts in the stage's mode matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeMatrix {
    /// `0x591370`: the identity, written only when the level is not already marked identity.
    Identity,
    /// `0x5a2bb8` to `0x5a2c35`: the fixed reflection matrix.
    Reflection,
}

/// The reflection matrix mode 6 stores (`0x5a2bb8` to `0x5a2c35`), row-major: 0.5 on the first two
/// diagonal entries, 1 on the last two, 0.5 in the first two entries of the last row.
pub const REFLECTION_MATRIX: [f32; 16] = [
    0.5, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.5, 0.5, 0.0, 1.0,
];

/// The `TEXCOORDINDEX` value of a stage in a mode (`0x5a2b6b`, `0x5a2b92`, `0x5a2c4f`,
/// `0x5a2c60`): the stage number, or-ed with `0x20000` for modes 1 to 3, `0x30000` for modes 4 and
/// 6, `0x10000` for mode 5. A mode above 6 sets nothing.
pub fn texture_coordinate_index(stage: u32, mode: u32) -> Option<u32> {
    match mode {
        0 => Some(stage),
        1..=3 => Some(stage | 0x20000),
        4 | 6 => Some(stage | 0x30000),
        5 => Some(stage | 0x10000),
        _ => None,
    }
}

/// The matrix a mode stores: the reflection matrix for mode 6, the identity for modes 0, 3, 4, 5
/// and above 6; modes 1 and 2 (a copy of the view matrix) are `None`.
pub fn mode_matrix(mode: u32) -> Option<ModeMatrix> {
    match mode {
        1 | 2 => None,
        6 => Some(ModeMatrix::Reflection),
        _ => Some(ModeMatrix::Identity),
    }
}

/// `0x591370` and `0x591440` applied to the stage's top matrix: the identity leaves a level marked
/// identity as it is, else writes the identity matrix, marks the level (flags word 1) and the stack
/// dirty; the reflection matrix clears bit 0 of the flags word, marks the stack dirty and writes
/// the 16 floats.
pub fn store_mode_matrix(top: &mut StageMatrix, which: ModeMatrix) {
    match which {
        ModeMatrix::Identity => {
            if top.flags & 1 == 0 {
                top.matrix = [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ];
                top.dirty = true;
                top.flags = 1;
            }
        }
        ModeMatrix::Reflection => {
            top.dirty = true;
            top.flags &= !1;
            top.matrix = REFLECTION_MATRIX;
        }
    }
}

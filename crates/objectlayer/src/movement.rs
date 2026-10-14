//! The movement block of an object update and the movement status the client writes: the reader
//! `ReadMovementBlock` (`0x47f050`), the skip `SkipMovementBlock` (`0x47ede0`), the status size
//! `SkipMovementStatus` (`0x47eb10`) and the writer `CMovementStatus::Write` (`0x47ecb0`). The
//! status reader `0x47eba0` is inlined into the block reader as `decode_base`. The entries are
//! `systems/objectlayer/movement.md#movement-block`, `#skipping-a-movement-block` and
//! `#movement-status`.
//!
//! Movement flags that gate the layout: on a transport `0x02000000`, swimming `0x00200000`,
//! falling `0x00002000`, spline elevation `0x04000000`, spline enabled `0x00400000`. Spline
//! flags: final point `0x10000`, final target `0x20000`, final angle `0x40000`.

use crate::datastore::{DataStore, DataStoreW};

pub const FLAG_ONTRANSPORT: u32 = 0x0200_0000;
pub const FLAG_SWIMMING: u32 = 0x0020_0000;
pub const FLAG_FALLING: u32 = 0x0000_2000;
pub const FLAG_SPLINE_ELEVATION: u32 = 0x0400_0000;
pub const FLAG_SPLINE_ENABLED: u32 = 0x0040_0000;
pub const SPLINE_FINAL_POINT: u32 = 0x0001_0000;
pub const SPLINE_FINAL_TARGET: u32 = 0x0002_0000;
pub const SPLINE_FINAL_ANGLE: u32 = 0x0004_0000;

/// The in-memory movement block the reader fills, a `#[repr(C)]` mirror of the client's struct with
/// its offsets in the comments. Fields the wire does not carry under a flag set stay as the caller
/// left them, except the guid, which is zeroed off a transport, and the pitch, which is zeroed when
/// not swimming.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MovementInfo {
    pub time: u32,                     // +0x00 (read 2nd, after flags)
    pub _pad04: u32,                   // +0x04 (never wire-touched)
    pub transport_guid: [u32; 2], // +0x08 lo/hi (both zeroed when !ONTRANSPORT; [u32;2] keeps the struct align-4 like the binary)
    pub flags: u32,               // +0x10 (read 1st)
    pub transport_off: [f32; 4],  // +0x14..+0x20  x,y,z,o
    pub pos: [f32; 4],            // +0x24..+0x30  x,y,z,orient
    pub pitch: f32,               // +0x34 (zeroed when !SWIMMING)
    pub fall_time: u32,           // +0x38
    pub fall: [f32; 4],           // +0x3c..+0x48  velocity, sin, cos, xy-speed
    pub spline_elevation: f32,    // +0x4c
    pub speeds: [f32; 6],         // +0x50..+0x64  (decode-side only)
    pub spline_final_target: [u32; 2], // +0x68 lo/hi
    pub spline_final_point: [f32; 3], // +0x70..+0x78
    pub spline_final_angle: f32,  // +0x7c
    pub spline_flags: u32,        // +0x80
    pub spline_time_base: u32,    // +0x84 = tick_count - wire time-passed
    pub _pad88: u32,              // +0x88
    pub spline_duration: u32,     // +0x8c
    pub spline_id: u32,           // +0x90
    pub _pad94: [u32; 11],        // +0x94..+0xbc
    pub spline_dest: [f32; 3],    // +0xc0..+0xc8
}

/// Size of the struct image: the last field written ends at `+0xcc`.
pub const MOVEMENT_INFO_SIZE: usize = 0xcc;

const _: () = assert!(std::mem::size_of::<MovementInfo>() == MOVEMENT_INFO_SIZE);

/// The spline points the reader hands off: it skips `count * 0xc` bytes and passes the window and
/// the count to `SetSplinePath` (`0x453360`).
pub struct SplineHandoff<'a> {
    pub count: u32,
    pub raw_points: &'a [u8],
}

/// `0x47eb10`: reads the flags dword, then skips the status that follows by its flags, and returns
/// the flags. The length is `0x14` (time and position), `0x2c` on a transport (adding the guid and
/// the offset), plus 4 when swimming; then 4 for the fall time, or `0x14` with the fall vector when
/// falling; then 4 for the spline elevation.
pub fn movement_info_size(ds: &mut DataStore) -> u32 {
    let mut flags = 0u32;
    if let Some(v) = ds.get_u32() {
        flags = v;
    }
    let mut n: u32 = 0x14;
    if flags & FLAG_ONTRANSPORT != 0 {
        n = 0x2c;
    }
    if flags & FLAG_SWIMMING != 0 {
        n += 4;
    }
    let mut total = n + 4;
    if flags & FLAG_FALLING != 0 {
        total = n + 0x14;
    }
    if flags & FLAG_SPLINE_ELEVATION != 0 {
        total += 4;
    }
    ds.skip(total);
    flags
}

/// `0x47ede0`: skips the whole block: the status by the length above, the six speeds (`0x18`), and
/// with the spline enabled the sub-flags, the final point, target and angle they gate (`0xc`, 8 and
/// 4 bytes), three timing dwords, the count and `(count * 3 + 3) * 4` bytes of points and
/// destination. The count arithmetic wraps in 32 bits.
pub fn movement_block_skip(ds: &mut DataStore) {
    let flags = movement_info_size(ds);
    ds.skip(0x18);
    if flags & FLAG_SPLINE_ENABLED != 0 {
        let mut sub = 0u32;
        if let Some(v) = ds.get_u32() {
            sub = v;
        }
        let mut ext: u32 = 0;
        if sub & SPLINE_FINAL_POINT != 0 {
            ext = 0xc;
        }
        if sub & SPLINE_FINAL_TARGET != 0 {
            ext += 8;
        }
        if sub & SPLINE_FINAL_ANGLE != 0 {
            ext += 4;
        }
        ds.skip(ext + 0xc);
        let mut count = 0u32;
        if let Some(v) = ds.get_u32() {
            count = v;
        }
        ds.skip(count.wrapping_mul(3).wrapping_add(3).wrapping_mul(4));
    }
}

/// The status reader `CMovementStatus::Read` (`0x47eba0`): flags, time and position; on a transport
/// the guid and offset, otherwise both guid dwords zeroed; the pitch when swimming, otherwise
/// zeroed; the fall time; the fall vector when falling; the spline elevation when set.
fn decode_base(ds: &mut DataStore, mv: &mut MovementInfo) {
    if let Some(v) = ds.get_u32() {
        mv.flags = v;
    }
    if let Some(v) = ds.get_u32() {
        mv.time = v;
    }
    for i in 0..4 {
        if let Some(v) = ds.get_f32() {
            mv.pos[i] = v;
        }
    }
    if mv.flags & FLAG_ONTRANSPORT == 0 {
        mv.transport_guid = [0, 0];
    } else {
        if let Some(v) = ds.get_u64() {
            mv.transport_guid = [v as u32, (v >> 32) as u32];
        }
        for i in 0..4 {
            if let Some(v) = ds.get_f32() {
                mv.transport_off[i] = v;
            }
        }
    }
    if mv.flags & FLAG_SWIMMING == 0 {
        mv.pitch = 0.0;
    } else if let Some(v) = ds.get_f32() {
        mv.pitch = v;
    }
    if let Some(v) = ds.get_u32() {
        mv.fall_time = v;
    }
    if mv.flags & FLAG_FALLING != 0 {
        for i in 0..4 {
            if let Some(v) = ds.get_f32() {
                mv.fall[i] = v;
            }
        }
    }
    if mv.flags & FLAG_SPLINE_ELEVATION != 0 {
        if let Some(v) = ds.get_f32() {
            mv.spline_elevation = v;
        }
    }
}

/// `0x47f050`: the status, the six speeds and, with the spline enabled, the sub-flags into
/// `spline_flags`, the final point, target and angle they gate, `spline_time_base = tick_count −
/// time passed`, the duration, the id, the points handed off, and the destination.
pub fn movement_block_decode<'a>(
    ds: &mut DataStore<'a>,
    mv: &mut MovementInfo,
    tick_count: u32,
) -> Option<SplineHandoff<'a>> {
    decode_base(ds, mv);
    for i in 0..6 {
        if let Some(v) = ds.get_f32() {
            mv.speeds[i] = v;
        }
    }
    let mut handoff = None;
    if mv.flags & FLAG_SPLINE_ENABLED != 0 {
        if let Some(v) = ds.get_u32() {
            mv.spline_flags = v;
        }
        if mv.spline_flags & SPLINE_FINAL_POINT != 0 {
            for i in 0..3 {
                if let Some(v) = ds.get_f32() {
                    mv.spline_final_point[i] = v;
                }
            }
        }
        if mv.spline_flags & SPLINE_FINAL_TARGET != 0 {
            if let Some(v) = ds.get_u64() {
                mv.spline_final_target = [v as u32, (v >> 32) as u32];
            }
        }
        if mv.spline_flags & SPLINE_FINAL_ANGLE != 0 {
            if let Some(v) = ds.get_f32() {
                mv.spline_final_angle = v;
            }
        }
        let mut time_passed = 0u32;
        if let Some(v) = ds.get_u32() {
            time_passed = v;
        }
        mv.spline_time_base = tick_count.wrapping_sub(time_passed);
        if let Some(v) = ds.get_u32() {
            mv.spline_duration = v;
        }
        if let Some(v) = ds.get_u32() {
            mv.spline_id = v;
        }
        let mut count = 0u32;
        if let Some(v) = ds.get_u32() {
            count = v;
        }
        if count != 0 {
            if let Some(raw) = ds.skip(count.wrapping_mul(0xc)) {
                handoff = Some(SplineHandoff {
                    count,
                    raw_points: raw,
                });
            }
        }
        for i in 0..3 {
            if let Some(v) = ds.get_f32() {
                mv.spline_dest[i] = v;
            }
        }
    }
    handoff
}

/// `0x47ecb0`: writes the status in the reader's order and under its flags, with no speeds and no
/// spline.
pub fn movement_block_encode(w: &mut DataStoreW, mv: &MovementInfo) {
    w.put_u32(mv.flags);
    w.put_u32(mv.time);
    for i in 0..4 {
        w.put_f32(mv.pos[i]);
    }
    if mv.flags & FLAG_ONTRANSPORT != 0 {
        w.put_u64(u64::from(mv.transport_guid[0]) | (u64::from(mv.transport_guid[1]) << 32));
        for i in 0..4 {
            w.put_f32(mv.transport_off[i]);
        }
    }
    if mv.flags & FLAG_SWIMMING != 0 {
        w.put_f32(mv.pitch);
    }
    w.put_u32(mv.fall_time);
    if mv.flags & FLAG_FALLING != 0 {
        for i in 0..4 {
            w.put_f32(mv.fall[i]);
        }
    }
    if mv.flags & FLAG_SPLINE_ELEVATION != 0 {
        w.put_f32(mv.spline_elevation);
    }
}

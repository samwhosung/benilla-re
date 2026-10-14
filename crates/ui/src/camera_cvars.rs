//! Reimplements the camera angle conversions of `CheckPitchRange` (`0x50b380`), `CheckYawRange`
//! (`0x50b3c0`), `FlipCameraYaw` (`0x50b6a0`) and `CheckAngleRateRange` (`0x50c000`). The x87
//! runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the
//! tests in `tests/camera_cvars.rs` diff each against the exe.

/// `180/π` = 57.295780181884766 — `f32` `[0x803a78]` (rad→deg).
pub const RAD2DEG: f32 = f32::from_bits(0x4265_2ee1);
/// `π/180` = 0.01745329238474369 — `f32` `[0x7ffaac]` (deg→rad).
pub const DEG2RAD: f32 = f32::from_bits(0x3c8e_fa35);

/// `0x50b380` / `0x50b3c0` / `0x50c000` — the camera-angle-CVar setters' owned conversion:
/// `deg = rad · (180/π)` (`fld dword [g]; fmul dword [0x803a78]; fstp dword`, x87 `PC_53` then `→ f32`).
/// Each fn converts a radian range pair (the angle CVar's min/max) to degrees, then tail-calls the
/// clamp-setter `0x50b330`; this is the per-value owned arithmetic.
#[must_use]
pub fn cam_rad2deg(rad: f32) -> f32 {
    (f64::from(rad) * f64::from(RAD2DEG)) as f32
}

/// `0x50b6a0` `FlipCameraYaw` — the owned camera-yaw flip: `cam_yaw = arg·(π/180) + cam_yaw`
/// (`fld dword [arg]; fmul dword [0x7ffaac]; fadd dword [cam+0x100]; fstp dword [cam+0x100]`, x87
/// `PC_53` then `→ f32`). `arg` is the Lua degrees argument; `cam` is the current camera-yaw field.
#[must_use]
pub fn cam_flip_yaw(arg: f32, cam: f32) -> f32 {
    ((f64::from(arg) * f64::from(DEG2RAD)) + f64::from(cam)) as f32
}

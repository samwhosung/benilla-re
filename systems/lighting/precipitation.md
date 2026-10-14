# lighting: precipitation

The three weather effects, rain, snow and sand, and the mist each carries. An effect emits its
particles into packets of 6144, each packet replayed from a start time on the particle clock, and
draws them with a vertex shader or, without one, on the processor. The manager that creates the
effects and feeds them the grade is `weather`.

Roots: `0x674620`, `0x677420`, `0x679140`, `0x677380`, `0x6790b0`, `0x67a520`, `0x6773d0`, `0x6790f0`, `0x67a560`

## Particle clock

The particles run on a clock of 1024 ticks a second. Each world update adds the frame time to it
as round(t·1024 − 0.5) ticks, ties to even, so an exact odd t·1024 comes out one tick low;
particle times convert the same way, and back as ticks/1024. The
frame's tick count is kept beside the clock.

Addresses: `0x6745d0`, `0x6745e3`, `0x8680e8`, `0x674600`, `0x66fcc0`, `0x66fd26`, `0x66fd38`, `0x66fd33`, `0xc62970`, `0xc62974`, `0x7ffd68`, `0x8090d8`
Confidence: verified
Proof: one
Test: `lighting::weather_fixed_quantize_difftest`, `lighting::weather_tick_scale_difftest`

`0x6745d0(s)` multiplies by `[0x7ffd68]` (1024.0), subtracts `[0x8680e8]` (0.5, `0x6745e3`) and
`fistp`s, so a value at an
exact half rounds to even. `0x674600` loads the ticks as a 64-bit integer and multiplies by
`[0x8090d8]` (1/1024). `0x66fcc0` calls `0x6745d0` with its frame seconds (`0x66fd26`), adds the
result to `0xc62970` (`0x66fd38`) and stores it in `0xc62974` (`0x66fd33`).

## Effect objects

An effect starts with two bytes, uses-shaders and stopping, then its box as minimum and maximum
corners, its mist at `+0x1c`, and its packet lists. The shader byte is set, then cleared when a
vertex shader fails to load or a needed device capability is missing, or when `useWeatherShaders`
is 0. Rain's box is 130 × 130 × 75 and it loads `rain.bls` and `patter.bls`; snow's is
90 × 90 × 60 with `snowpoint.bls`; sand's 40 × 40 × 25 with `sand.bls`.

Addresses: `0x674620`, `0x6747fc`, `0x67480d`, `0x677420`, `0x679140`, `0x674830`, `0x6775d0`, `0x679290`, `0x67ba50`
Confidence: verified
Proof: one
Test: `lighting::weather_effect_ctor_674620_difftest`, `lighting::weather_effect_helper_677420_difftest`, `lighting::weather_effect_helper_679140_difftest`

`0x674620(130, 75)` stores the box from ±65 and ±37.5, rain at `+0xbc` and patter at `+0xc4`
shader handles, and clears byte 0 when either fails (`0x6747fc`) or the setting is 0
(`0x67480d`). Snow needs device capability `+0xd4`, sand `+0xcc`. `0x674830`, `0x6775d0` and
`0x679290` free the shaders, both packet lists and the mist; `0x67ba50` calls them.
The tests run `0x674620` from `0x6746fa` to `0x674757`; `0x677420` from `0x6774ce` to `0x677528`; `0x679140` from `0x6791ef` to `0x67924e`.

## Particle packets

A rain packet holds 6144 drops of 0x24 bytes: position, velocity, and birth and death in
seconds of fill time. After them come the count `+0x3600c`, the start and last ticks `+0x36010` and
`+0x36014`, the fill time `+0x36018` and the vertex buffer `+0x3602c`. A packet opens when
the first drop of a new batch is due: its start tick is now plus the time 6144 drops take at the
current rate. Every drop raises the last tick to the start plus its death plus the time its tail
takes to fall 2 yards. The packet closes, baking its vertex buffer, when it is full, its fill time
reaches P·(1/6144) for the rate's base P (`rain-rates`), or now passes its start.

Addresses: `0x6756a0`, `0x6758ac`, `0x6758ce`, `0x675944`, `0x675966`, `0x675976`, `0x67598c`, `0x675548`, `0x67554e`, `0x675568`, `0x675582`, `0x675686`, `0x675a6d`, `0x675a79`, `0x675a7f`, `0x675a8c`, `0x675a97`, `0x675aad`, `0x6752b0`, `0x6753c0`, `0x80ffc8`
Confidence: verified
Proof: one
Test: `lighting::wl_emit_count_clamp`, `lighting::wl_emit_phase_advance`, `lighting::wl_emit_phase_advance_trajectory`, `lighting::wl_emit_phase_inc`, `lighting::wl_rain_drop_t`, `lighting::wl_rain_patter_vec`, `lighting::wl_rain_wind_gate`, `lighting::wl_update_dt_accum`, `lighting::wl_update_interval_div`

`0x6758ac` opens a packet when `+0xb0` is 0, reusing a free one or allocating 0x36030 bytes
(`0x6758ce`). The start tick is `0x6745d0([0x80ffc8]/rate)` plus now (`0x675944` to `0x67598c`).
`0x675548` to `0x675582` take 2.0/vz from the death, convert and keep the larger. `0x675686` adds
the frame seconds to the fill time. The close tests are `0x675a6d`, `0x675a79` against `+0xb4`
and `0x675a8c`, the fill limit read at `0x675a7f`; `0x6752b0` writes three vertices a drop, each the position, velocity, corner
index in the colour's top byte, birth and death.
The tests run `0x6754a0` in 6 slices between `0x6754cc` and `0x675695`; `0x6756a0` from `0x675863` to `0x67588d` and from `0x675944` to `0x675976`.

## Emission budget

An effect that is not stopping emits round(min(6144 − count, q) − 0.5) drops a frame, with
q = min(frame seconds, 0.016667)·rate, and only when q is above 1. Below 60 frames a second the
budget shrinks with the frame rate. Each drop's birth is the fill time plus a random fraction of
the frame. The count rounds to even at a half, so a packet one short of full can stay there, and
an exact odd min(6144 − count, q) comes out one low.

Addresses: `0x675858`, `0x67585d`, `0x675863`, `0x675884`, `0x67588d`, `0x675a62`, `0x6754a0`, `0x6754cc`, `0x6754e9`, `0x6754ef`, `0x675505`, `0x67551c`, `0x675539`, `0x80ffcc`, `0x8015b8`, `0x8680e8`
Confidence: verified
Proof: one
Test: `lighting::wl_emit_count_clamp`, `lighting::wl_emit_phase_advance`, `lighting::wl_emit_phase_advance_trajectory`, `lighting::wl_emit_phase_inc`, `lighting::wl_rain_drop_t`, `lighting::wl_rain_patter_vec`, `lighting::wl_rain_wind_gate`, `lighting::wl_update_dt_accum`, `lighting::wl_update_interval_div`

`0x67585d` skips emission while byte `+0x1` is set. `0x675863` to `0x675884` take the smaller of
the frame time and `[0x80ffcc]` (0.016667, `0x3c88893b`) times `+0xcc`; `0x67588d` compares with the double 1.0 and
`test ah,0x41`. `0x6754a0(packet, patter, q)` takes 6144 less the count as an integer
(`0x6754cc`), keeps the smaller, subtracts `[0x8680e8]` (0.5, `0x6754e9`) and `fistp`s (`0x6754ef`). Each drop draws a
random float in [1, 2) from `s_clientRandom` (`0x675505`) for its birth (`0x675539`).
The tests run `0x6754a0` in 6 slices between `0x6754cc` and `0x675695`; `0x6756a0` from `0x675863` to `0x67588d` and from `0x675944` to `0x675976`.

## Packet replay

A packet draws only once now reaches its start tick, with the time (now − start)/1024; until then
it is invisible, so visible rain lags emission by one packet's span. A packet whose last tick has
passed returns to the free list. A packet spanning 60 s or more from start to last tick is
dropped with the message "Rain packet lifetime was too long". A stopping effect closes its open
packet at once and, while the density is at most 0.25, drops every packet that has not started.

Addresses: `0x6756b0`, `0x6756da`, `0x675717`, `0x67574a`, `0x67575a`, `0x675790`, `0x675795`, `0x6757db`, `0x6772f5`, `0x6772fd`, `0x67637d`, `0x80ff6c`
Confidence: verified
Proof: one
Test: `lighting::rain_drop_quad_difftest`, `lighting::wl_update_dt_accum`, `lighting::wl_update_interval_div`

`0x6756da` clears `+0xb0` when byte `+0x1` is set. `0x675717` retires when now is above the last
tick; `0x67574a` tests the manager's density against 0.25 and `0x67575a` now against the start.
`0x675790` converts last − start and `0x675795` compares with `[0x80ff6c]` (60.0). The shader
draw tests the start at `0x6772f5` and converts at `0x6772fd`; the processor draw at `0x67637d`.
The patter packets retire on their own last tick.
The tests run `0x6756a0` from `0x675863` to `0x67588d` and from `0x675944` to `0x675976`; `0x676160` from `0x676410` to `0x676932`.

## Rain rates

With x the grade the effect receives, rain emits K·P·x drops a second, P = 35000 with shaders and
6500 without, K the density setting, and closes a packet at a fill time of f32(P·(1/6144)) seconds,
about 5.70 s or 1.06 s. Its mist emits 2·max(x − 0.5, 0)·K·Q puffs a second, Q = 38 with shaders and 18
without, so the mist starts only above x = 0.5.

Addresses: `0x6749e0`, `0x6749ec`, `0x674a0d`, `0x674a13`, `0x674a19`, `0x675a7f`, `0x674a2f`, `0x674a62`, `0x67a980`, `0x80ffac`, `0x80ffa8`, `0x80ffa4`, `0x80ffa0`, `0x80ff9c`
Confidence: verified
Proof: one
Test: `lighting::rain_density_param_set_difftest`, `lighting::wl_update_dt_accum`, `lighting::wl_update_interval_div`

`0x6749e0(x)` stores x in `+0xd0`, picks `[0x80ffac]` or `[0x80ffa8]` by byte 0 (`0x6749ec`),
stores K·P·x in `+0xcc` (`0x674a0d`) and P·`[0x80ffa4]` (1/6144, `0x674a13`) in `+0xb4`
(`0x674a19`), which the close compares at `0x675a7f`. It takes
x − 0.5 (`0x674a2f`), keeps it when it is 0 or above, or NaN (`fcomp` of 0 against it, `test
ah,0x41`, `jne`), else takes 0, doubles it, multiplies by K and Q and passes it to
`0x67a980`, which stores the mist rate (`0x674a62`).
The tests run `0x6756a0` from `0x675863` to `0x67588d` and from `0x675944` to `0x675976`.

## Rain drops

A drop is seeded at a random point of the box's horizontal centre plane. Its velocity is
s·(sin a, cos a) across and vz down, with a = (0.2094395·x + 0.0523599)·(u − 0.5) − 1.57,
s = (2(u − 0.5) + 9.49)·x + 0.01 and vz = −2x·r − 4x − 28, u and r uniform in [0, 1). Its position
moves back by T = 37.5/|vz|, the time to fall half the box, onto the box's top. The position, not the velocity, is turned about the horizontal axis at heading
−90° by up to 65° as the speed reaches 18, then moved by 1.75 times the player's velocity and
by the camera.

Addresses: `0x674c50`, `0x674ccd`, `0x674d25`, `0x674d2b`, `0x674d33`, `0x674d45`, `0x674d78`, `0x674d84`, `0x674dc9`, `0x674dd8`, `0x674ded`, `0x674df6`, `0x674e05`, `0x674e53`, `0x674e75`, `0x674e94`, `0x674f36`, `0x674f45`, `0x674f54`, `0x674f82`, `0x674fd4`, `0x80ffc4`, `0x80ffc0`, `0x80ffbc`, `0x80ffb8`, `0x80ffb4`, `0x80ffb0`, `0x80ff80`, `0x8680f8`, `0x7bdd60`
Confidence: verified
Proof: one
Test: `lighting::rainspawn_angle_difftest`, `lighting::rainspawn_cam_global_difftest`, `lighting::rainspawn_cell_delta_difftest`, `lighting::rainspawn_collide_pos_difftest`, `lighting::rainspawn_collide_scale_difftest`, `lighting::rainspawn_collide_wind_difftest`, `lighting::rainspawn_final_delta_difftest`, `lighting::rainspawn_lifetime_difftest`, `lighting::rainspawn_pos_offset_difftest`, `lighting::rainspawn_pos_seed_difftest`, `lighting::rainspawn_residual_difftest`, `lighting::rainspawn_sincos_difftest`, `lighting::rainspawn_sky_time_difftest`, `lighting::rainspawn_vel_mag_difftest`, `lighting::rainspawn_vel_xy_difftest`, `lighting::rainspawn_vel_z_difftest`, `lighting::rainspawn_velmag_difftest`, `lighting::rainspawn_wind_apply_difftest`, `lighting::rainspawn_wind_delta_difftest`, `lighting::rainspawn_wind_factor_difftest`, `lighting::rainspawn_wind_scale2_difftest`, `lighting::rainspawn_world_scale_difftest`

`0x674ccd` multiplies the seed's height by 0. `fsincos` (`0x674d45`) stores the cosine at
`ebp−0x40` and the sine at `ebp−0x44`; the sine scales x. `0x674df6` to `0x674e05` form
−(75·0.5)/vz, and `0x674e53` stores the seed less T·V. `0x674e75` subtracts π/2 from the heading;
`0x674e94` scales the speed by 1/18 for `0x674ba0(0, [0x80ff80])` and `0x7bdd60` builds the
turn; `0x674f54` applies it to the position only. `0x674f82` scales `+0x50` by 1.75.
The tests run `0x674c50` in 22 slices between `0x674c5c` and `0x675251`.

## Drop landing

A drop is dead at birth, birth and death both 0, when the ground under its start is at or above
it. Otherwise it lands where the segment from its start to start + (1 + k)·T·V meets the ground,
k = 2, and its start is rewritten as hit − (|start − hit|/|V|)·V before its death is set to its birth
plus the distance to that point over its speed. With the unit
flag set or the transport's v² above 2 the segment is not tested: k = 1 and the end is the
landing. A miss is dead at birth.

Addresses: `0x675051`, `0x67505b`, `0x67505e`, `0x675069`, `0x675088`, `0x6750a1`, `0x6750a6`, `0x675143`, `0x67515c`, `0x675174`, `0x67517b`, `0x6751af`, `0x6751c8`, `0x6751d0`, `0x67520e`, `0x675221`, `0x67524a`, `0x675254`, `0x67526b`
Confidence: verified
Proof: one
Test: `lighting::rainspawn_angle_difftest`, `lighting::rainspawn_cam_global_difftest`, `lighting::rainspawn_cell_delta_difftest`, `lighting::rainspawn_collide_pos_difftest`, `lighting::rainspawn_collide_scale_difftest`, `lighting::rainspawn_collide_wind_difftest`, `lighting::rainspawn_final_delta_difftest`, `lighting::rainspawn_lifetime_difftest`, `lighting::rainspawn_pos_offset_difftest`, `lighting::rainspawn_pos_seed_difftest`, `lighting::rainspawn_residual_difftest`, `lighting::rainspawn_sincos_difftest`, `lighting::rainspawn_sky_time_difftest`, `lighting::rainspawn_vel_mag_difftest`, `lighting::rainspawn_vel_xy_difftest`, `lighting::rainspawn_vel_z_difftest`, `lighting::rainspawn_velmag_difftest`, `lighting::rainspawn_wind_apply_difftest`, `lighting::rainspawn_wind_delta_difftest`, `lighting::rainspawn_wind_factor_difftest`, `lighting::rainspawn_wind_scale2_difftest`, `lighting::rainspawn_world_scale_difftest`

`0x67505b` calls the grid's height slot at the start and `0x67505e` compares with its height.
`0x675088` stores k = 2.0 and `0x6750a6` 1.0, after `test ah,0x41` (`0x6750a1`, `0x67515c`) on v²
against 2.0, which counts a NaN as at most 2. The segment test is the grid's slot `+0x8`
(`0x675174`), skipped by `0x675143` and `0x67515c`, which copy the end into the hit
(`0x675254`). `0x6751af` to `0x67520e` store start − hit and f32(1/|V|) as floats, keep s = |start − hit|·(1/|V|)
on the x87 stack at 53 bits (`0x6751c8` to `0x6751d0`), and store hit − s·V as the start without
storing s·V; `0x675221` stores the birth, and `0x67524a` stores the new distance times f32(1/|V|)
plus the birth, computed at 53 bits. The grid's height is FLT_MAX outside it, which also kills the drop (`weather#ground-grid`).
The tests run `0x674c50` in 22 slices between `0x674c5c` and `0x675251`.

## Patter

A live drop that lands with the unit flag clear and the transport at most √2 (v² ≤ 2, NaN included)
leaves a patter,
a splash at its landing point that lives 0.25 s from its death. The patters fill their own packets
of 6144, opened with each drop packet and sharing its start tick.

Addresses: `0x675588`, `0x675593`, `0x67559e`, `0x6755ce`, `0x6755d3`, `0x675630`, `0x675280`, `0x675664`, `0x6759ae`, `0x675a48`, `0x8029b0`
Confidence: verified
Proof: one
Test: `lighting::patter_record_init_difftest`, `lighting::wl_emit_count_clamp`, `lighting::wl_emit_phase_advance`, `lighting::wl_emit_phase_advance_trajectory`, `lighting::wl_emit_phase_inc`, `lighting::wl_rain_drop_t`, `lighting::wl_rain_patter_vec`, `lighting::wl_rain_wind_gate`, `lighting::wl_update_dt_accum`, `lighting::wl_update_interval_div`

`0x675588` compares birth and death; `0x675593` skips a dead drop; `0x6755ce` tests v² with
`test ah,0x41`. `0x675630` calls
`0x675280(record, death, point)`, which stores the point and the times death and death +
`[0x8029b0]`. `0x675664` keeps the patter packet's last tick. `0x6759ae` allocates 0x24030 bytes,
6144 records of 0x18; `0x675a48` copies the drop packet's start tick.
The tests run `0x6754a0` in 6 slices between `0x6754cc` and `0x675695`; `0x6756a0` from `0x675863` to `0x67588d` and from `0x675944` to `0x675976`.

## Rain draw

With shaders each started packet draws its drops as triangles through `rain.bls` with additive
blend state 5: corners (y, z) of (−0.05, 0), (0.05, 0) and (0, 2), a tail length of 2 when the
player is still and 0 when moving, and a rotation leaning the streaks by up to 45° as the
player's horizontal speed reaches 30, about the axis across the motion. Patters draw as camera
quads through `patter.bls`, ±1/12 across and 1/6 up, with frames of 0.25 × 0.25 from the splash
texture. Without shaders the processor builds both each frame.

Addresses: `0x676e70`, `0x676e99`, `0x676eec`, `0x6772dc`, `0x677333`, `0x67735b`, `0x674a70`, `0x674bf0`, `0x6769d0`, `0x676160`, `0x675ac0`, `0x6763a0`, `0x677380`, `0x8680f0`, `0x8680f4`, `0x80ff78`, `0x80ff74`, `0x80e004`, `0x803568`
Confidence: verified
Proof: one
Test: `lighting::rain_drop_quad_difftest`, `lighting::rain_patter_matrix_difftest`, `lighting::weather_wind_tilt_difftest`, `lighting::wind_speed_gate_difftest`, `lighting::wx_rainrender_basis_difftest`, `lighting::wx_rainrender_quad_difftest`

`0x676e99` sets state 7 to 5. The constants are `c_motionBlurRotateMtx` from `0x674a70`,
`c_tailLength` from `0x674bf0`, `vtxCoordsY` ∓`[0x80ff78]` and 0, `vtxCoordsZ` 0, 0 and
`[0x80ff74]`, and `c_currentTime`. `0x674a70` turns by `0x674ba0(0, 45, |v|/30)` degrees about
(vy, −vx, 0)/|v|. `0x67735b` draws type 3 with 3·count indices. `0x6769d0` sets `posOffsets`,
`texCoords` and `texFrameSize`. `0x677380` updates, then draws drops, patters and mist.
The tests run `0x675ac0` from `0x675bfc` to `0x675c9b` and from `0x675db3` to `0x6760c3`; `0x676160` from `0x676410` to `0x676932`; `0x6769d0` from `0x676c37` to `0x676cb6`.

## Snow

Snow emits K·P·x flakes a second, P = 14000 with shaders and 1300 without, its mist
2·max(x − 0.5, 0)·K·Q puffs, Q = 48 or 24. A flake starts like a drop with
a = (u − 0.5)·(2π − 5.934·x) − 1.57, s = (u − 0.5 + 5.985)·x + 0.015 and
vz = −x·r − 3.5x − 2, and moves by 1.75 times the player's velocity and by the camera eye;
with the unit flag set, the update first lowers the eye's height by a blend of the box height.
With shaders the flakes draw as points of size up to
24 through `snowpoint.bls` with blend 2, `c_pointSize` (−0.02, 0, 14) and `c_zfade` (−1/6, 13/3).

Addresses: `0x6776c0`, `0x677750`, `0x6777fe`, `0x677811`, `0x677877`, `0x677883`, `0x6778c8`, `0x6778d7`, `0x6778ec`, `0x677a61`, `0x677aaf`, `0x677ff0`, `0x6781c4`, `0x6781e6`, `0x6781fc`, `0x67845f`, `0x67846f`, `0x6784d2`, `0x6785c3`, `0x678610`, `0x67863a`, `0x678960`, `0x80ffe0`, `0x80ffdc`, `0x80ffd8`, `0x80732c`, `0x80fff4`, `0x80fff0`, `0x80ffec`, `0x80ffe8`, `0x80ffe4`
Confidence: verified
Proof: one
Test: `lighting::snow_alpha_pack_difftest`, `lighting::snow_basis_difftest`, `lighting::snow_density_param_set_difftest`, `lighting::snow_exp2_arg_difftest`, `lighting::snow_exp2_blend_difftest`, `lighting::snow_field_lerp_difftest`, `lighting::snow_final_recip_difftest`, `lighting::snow_flake_center_difftest`, `lighting::snow_flake_gate_difftest`, `lighting::snow_min_select_scale_difftest`, `lighting::snow_normalize_scale_difftest`, `lighting::snow_record_distance_difftest`, `lighting::snow_tri_vertices_difftest`, `lighting::snow_wind_deflect_difftest`, `lighting::snowfl_add_base_difftest`, `lighting::snowfl_add_offset_difftest`, `lighting::snowfl_angle1_difftest`, `lighting::snowfl_cam_global_difftest`, `lighting::snowfl_cell_delta_difftest`, `lighting::snowfl_drift_difftest`, `lighting::snowfl_drift_vel_difftest`, `lighting::snowfl_lifetime_difftest`, `lighting::snowfl_mtx_scale_difftest`, `lighting::snowfl_phase_difftest`, `lighting::snowfl_pos_seed_difftest`, `lighting::snowfl_sincos_difftest`, `lighting::snowfl_sky_rot_difftest`, `lighting::snowfl_sub_drift_difftest`, `lighting::snowfl_vel_drift_scale_difftest`, `lighting::snowfl_vel_flag_scale_difftest`, `lighting::snowfl_vel_z_scale_difftest`, `lighting::snowfl_world_scale_difftest`

`0x6776c0` stores K·P·x in `+0xa0` and P/6144 in `+0x94`. The flake constants are read at
`0x6777fe` to `0x6778ec`; the point size is the smaller of 24 and the device's limit. The update
allocates 0x30030-byte packets of 6144 flakes of 0x20 bytes (`0x6784d2`) and closes at 6144
(`0x6785c3`). It copies the eye (`0x6781c4`) and the manager's `+0x50` (`0x6781e6`) for `0x677750`, which adds
1.75 times the latter and the eye (`0x677a61`, `0x677aaf`). The block from `0x6781fc` to `0x67846f`,
run with `+0x74` set, a player spline and its `+0x24` float b above 0,
subtracts the box height times a blend t from the eye's z (`0x67845f`). `0x678270` takes u =
clamp(((a − d/m) + 2350)/b, 0, 1), a the spline's `+0x20` float (`0x5fefe0`), d the eye's distance
from the manager's `+0x68` point, m its `+0x7c` and d/m 0 for |m| < 0.001. `0x453420` gives the
spline tangent at u, normalized and scaled by m, and w is its z; the block ends unless w < −10
(`0x67839f`). t = clamp((e^(−w/10) − e)/(e^2.25 − e), 0, 1) with the exponentials of `0x454d40`
(`0x678405`): 0 at w = −10 and 1 at −22.5. Landing is not read. The tests run slices of `0x677750`, `0x677ff0`, `0x678960`.

## Sand

Sand emits K·P·x grains a second, P = 32000 with shaders and 6000 without, and its mist K·Q·x
puffs, Q = 64 or 32, with no threshold. A grain starts at 0.85 to 1.0 of the box's length and
anywhere across and up it, with a = (u − 0.5)·0.349 − 1.57, a speed of (u − 0.5)·2/3 + 18.667
and a third term (u − 0.5)/6 + 0.833, all turned by the heading about the vertical. It draws as
points through `sand.bls`.

Addresses: `0x679380`, `0x6793f0`, `0x679467`, `0x67946d`, `0x6794fa`, `0x679543`, `0x679549`, `0x679595`, `0x67959b`, `0x6796ce`, `0x679b00`, `0x67a1b0`, `0x679e50`, `0x810010`, `0x81000c`, `0x808de4`, `0x806534`, `0x810024`, `0x810020`, `0x81001c`, `0x810018`
Confidence: inferred
Proof: none
Test: `lighting::mist_tex_scroll_difftest`, `lighting::sand_angle_difftest`, `lighting::sand_cam_global_difftest`, `lighting::sand_cell_delta_difftest`, `lighting::sand_clamp_difftest`, `lighting::sand_color_repack_difftest`, `lighting::sand_density_param_set_difftest`, `lighting::sand_lifetime_difftest`, `lighting::sand_negate_difftest`, `lighting::sand_pos_seed_difftest`, `lighting::sand_ramp_arg_difftest`, `lighting::sand_sincos_difftest`, `lighting::sand_speed_difftest`, `lighting::sand_transform_difftest`, `lighting::sand_vel_offset_difftest`, `lighting::sand_vel_scale_difftest`, `lighting::sand_vel_xy_difftest`, `lighting::sand_vel_z_difftest`, `lighting::sand_world_scale_difftest`, `lighting::wl_update_dt_accum`, `lighting::wl_update_interval_div`

The constants are read at the cited instructions; the grain's velocity assembly and its landing
are read from the decompiled body of `0x6793f0`, not settled at the bytes.
The tests run `0x6793f0` in 14 slices between `0x6793f6` and `0x679864`; `0x679b00` from `0x679c77` to `0x679ca1` and from `0x679da8` to `0x679dda`; `0x679e50` in 5 slices between `0x67a013` and `0x67a0eb`; `0x67a1b0` from `0x67a411` to `0x67a437`.

## Mist

Each effect's mist is 128 puffs, each a camera-facing quad of size 12 in the scene's fog colour.
A rate accumulator gains rate·frame seconds, capped at 128; a free puff spawns while it holds 1,
born at a random point of the frame. A puff moves at (u − 0.5)·spread + speed along
(u − 0.5)·0.349 − 1.57 turned by the heading: speed 5.0, 9.0 or 15.0 and spread 1.2, 3.0 or 4.5
for rain, snow and sand. It lives (u − 0.5)·0.3 + 2.7 s, fading in over its first 0.4 s and out
over its last 0.4 s, and is invisible within 6 yards of the camera, full beyond 18.

Addresses: `0x67a5b0`, `0x674684`, `0x677487`, `0x6791a7`, `0x67a990`, `0x67aa6b`, `0x67ac3a`, `0x67acef`, `0x67ad1d`, `0x67ad23`, `0x67ae20`, `0x67b14d`, `0x67b15d`, `0x67b24d`, `0x67b2be`, `0x67b46a`, `0x67b488`, `0x67b4a1`, `0x67b529`, `0x67b533`, `0x67b545`, `0x67b643`, `0x67ad60`, `0x67ad90`, `0x67add0`, `0x67b700`, `0x81002c`
Confidence: verified
Proof: one
Test: `lighting::mist_spawn_difftest`, `lighting::mistr_cell_lerp_difftest`, `lighting::mistr_cell_round_difftest`, `lighting::mistr_color_repack_difftest`, `lighting::mistr_corner_add_difftest`, `lighting::mistr_corner_basis_difftest`, `lighting::mistr_corner_dist_difftest`, `lighting::mistr_drift_clamp_difftest`, `lighting::mistr_fade_mag_difftest`, `lighting::mistr_half_scale_difftest`, `lighting::mistr_hash_count_difftest`, `lighting::mistr_life_span_difftest`, `lighting::mistr_node_accum_difftest`, `lighting::mistr_phase_advance_difftest`, `lighting::mistr_phase_wrap_difftest`, `lighting::mistr_ramp_product_difftest`, `lighting::mistr_spacing_recip_difftest`, `lighting::mistr_time_accum_difftest`, `lighting::weather_effect_ctor_674620_difftest`, `lighting::weather_effect_helper_677420_difftest`, `lighting::weather_effect_helper_679140_difftest`, `lighting::weather_ramp_eval_difftest`, `lighting::weather_ramp_eval_inv_difftest`, `lighting::weather_ramp_setup_difftest`

`0x67a5b0(12, box, texture, −1.57, 0.349, speed, spread, 128)`; the rain call is `0x674684`.
`0x67b14d` multiplies the frame seconds by `+0x20`; `0x67b24d` tests the accumulator against 1.0.
`0x67a990`, the table `0x81002c`'s one slot, places the puff at a box point less 1.5 times its
velocity from the camera (`0x67ac3a`) and sets the life (`0x67ad23`). `0x67ad60(a, b)` is the
ramp (b − v)/(b − a); fades use it with `+0x30` (0.4) and 6 and 18 (`0x67b529`). The colour is
light state `+0x70` (`0x67b46a`); state 7 is 2 (`0x67b643`).
The tests run `0x674620` from `0x6746fa` to `0x674757`; `0x677420` from `0x6774ce` to `0x677528`; `0x679140` from `0x6791ef` to `0x67924e`; `0x67ae20` in 20 slices between `0x67ae66` and `0x67b551`.

## Mist ground

A puff samples the ground along its path at spawn, up to 64 points, and its life is cut where the
ground ahead rises: a step of more than 1.0 three samples on, 0.75 two on or 0.5 one on. Each
frame the puff rises toward the sampled height plus half its size, by at most a quarter of its
size.

Addresses: `0x67a7a0`, `0x67a8e9`, `0x67a8fb`, `0x67a90d`, `0x67b427`, `0x67b43c`, `0x8012cc`
Confidence: inferred
Proof: none
Test: `lighting::mist_basis_count_difftest`, `lighting::mist_gate_diff_difftest`, `lighting::mist_node_accumulate_difftest`, `lighting::mist_node_spacing_difftest`, `lighting::mistr_cell_lerp_difftest`, `lighting::mistr_cell_round_difftest`, `lighting::mistr_color_repack_difftest`, `lighting::mistr_corner_add_difftest`, `lighting::mistr_corner_basis_difftest`, `lighting::mistr_corner_dist_difftest`, `lighting::mistr_drift_clamp_difftest`, `lighting::mistr_fade_mag_difftest`, `lighting::mistr_half_scale_difftest`, `lighting::mistr_hash_count_difftest`, `lighting::mistr_life_span_difftest`, `lighting::mistr_node_accum_difftest`, `lighting::mistr_phase_advance_difftest`, `lighting::mistr_phase_wrap_difftest`, `lighting::mistr_ramp_product_difftest`, `lighting::mistr_spacing_recip_difftest`, `lighting::mistr_time_accum_difftest`

The step constants are compared at `0x67a8e9`, `0x67a8fb` and `0x67a90d`; the sample spacing and
the rise are read from the decompiled bodies of `0x67a7a0` and `0x67ae20`.
The tests run `0x67a7a0` in 4 slices between `0x67a7f3` and `0x67a949`; `0x67ae20` in 20 slices between `0x67ae66` and `0x67b551`.

## Effect fade

A weather change that finds particles alive fades the old effect instead of replacing it: the
manager sets the effect's stopping byte and its mist's, so no new drops or puffs are born, the
open packet closes, packets that have not started are dropped, and puffs not yet born are freed.
The live count, packets still before their last tick plus puffs between birth and death, decides
when the new type is built.

Addresses: `0x67bfec`, `0x6773d0`, `0x6790f0`, `0x67a560`, `0x67b700`, `0x67b22a`, `0x67b25a`
Confidence: verified
Proof: one
Test: `lighting::mistr_cell_lerp_difftest`, `lighting::mistr_cell_round_difftest`, `lighting::mistr_color_repack_difftest`, `lighting::mistr_corner_add_difftest`, `lighting::mistr_corner_basis_difftest`, `lighting::mistr_corner_dist_difftest`, `lighting::mistr_drift_clamp_difftest`, `lighting::mistr_fade_mag_difftest`, `lighting::mistr_half_scale_difftest`, `lighting::mistr_hash_count_difftest`, `lighting::mistr_life_span_difftest`, `lighting::mistr_node_accum_difftest`, `lighting::mistr_phase_advance_difftest`, `lighting::mistr_phase_wrap_difftest`, `lighting::mistr_ramp_product_difftest`, `lighting::mistr_spacing_recip_difftest`, `lighting::mistr_time_accum_difftest`, `lighting::wx_67be40_owned_fp_difftest`

`0x6773d0` counts packets with a last tick above now and adds `0x67b700`'s count of puffs with
birth < now ≤ death. The mist tests its stopping byte `+0x3c`, effect `+0x58`, at `0x67b22a`
and `0x67b25a`.
The tests run `0x67ae20` in 20 slices between `0x67ae66` and `0x67b551`; `0x67be40` in 7 slices between `0x67be8a` and `0x67c117`.


//! The alpha and shadow decoders and the combined texture builders, each run whole on random
//! bytes: `0x6b0ac0` to `0x6b0da0`, and `0x6b03d0`, `0x6b0510`, `0x6b0770` and `0x6b08d0` with
//! every combination of absent layers and shadow.

use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// Random bytes for MCAL/MCSH inputs.
fn synth_bytes(n: usize, seed: u32) -> Vec<u8> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (s >> 11) as u8
        })
        .collect()
}

/// The output and input the decoder tests share; each runs the same 40 random inputs.
struct DecodeEnv<'a> {
    emu: Emu<'a>,
    out: u64,
    data: u64,
}

fn decode_env<'a>(wow: &Wow) -> DecodeEnv<'a> {
    let mut emu = Emu::new(wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let out = emu.alloc(4096 * 2);
    let data = emu.alloc(2048);
    DecodeEnv { emu, out, data }
}

fn read16(emu: &Emu, out: u64, n: usize) -> Vec<u16> {
    (0..n)
        .map(|i| emu.read_u32(out + i as u64 * 2) as u16)
        .collect()
}

/// The output and the layer record the combined texture tests share: three alpha layers and a
/// shadow, each present or absent in 16 combinations.
struct AlphaEnv<'a> {
    emu: Emu<'a>,
    out: u64,
    layers: u64,
    l1: u64,
    l2: u64,
    l3: u64,
    sh: u64,
}

/// The random alpha and shadow bytes both sides read, held apart from `AlphaEnv` so its borrows
/// stay immutable.
struct AlphaBlobs {
    d1: [u8; 2048],
    d2: [u8; 2048],
    d3: [u8; 2048],
    ds: [u8; 512],
}

fn alpha_env<'a>(wow: &Wow) -> (AlphaEnv<'a>, AlphaBlobs) {
    let mut emu = Emu::new(wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let out = emu.alloc(4096 * 2);
    let layers = emu.alloc(0x10); // the layer record: the alpha pointers at +4, +8 and +0xc
    let l1 = emu.alloc(2048);
    let l2 = emu.alloc(2048);
    let l3 = emu.alloc(2048);
    let sh = emu.alloc(512);
    let d1: [u8; 2048] = synth_bytes(2048, 0x1111_2222).try_into().unwrap();
    let d2: [u8; 2048] = synth_bytes(2048, 0x3333_4444).try_into().unwrap();
    let d3: [u8; 2048] = synth_bytes(2048, 0x5555_6666).try_into().unwrap();
    let ds: [u8; 512] = synth_bytes(512, 0x7777_8888).try_into().unwrap();
    emu.write(l1, &d1);
    emu.write(l2, &d2);
    emu.write(l3, &d3);
    emu.write(sh, &ds);
    (
        AlphaEnv {
            emu,
            out,
            layers,
            l1,
            l2,
            l3,
            sh,
        },
        AlphaBlobs { d1, d2, d3, ds },
    )
}

/// The shadow argument and the reimplementation's layer and shadow views for one mask.
type AlphaWire<'a> = (u32, [Option<&'a [u8; 2048]>; 3], Option<&'a [u8; 512]>);

/// Sets the layer record's pointers for `mask` and returns the shadow argument with the
/// reimplementation's views of the same inputs.
fn alpha_wire<'a>(
    env: &mut AlphaEnv<'_>,
    mask: u32,
    a1: &'a [u8; 2048],
    a2: &'a [u8; 2048],
    a3: &'a [u8; 2048],
    ash: &'a [u8; 512],
) -> AlphaWire<'a> {
    let p1 = mask & 1 != 0;
    let p2 = mask & 2 != 0;
    let p3 = mask & 4 != 0;
    let psh = mask & 8 != 0;
    env.emu
        .write_u32(env.layers + 4, if p1 { env.l1 as u32 } else { 0 });
    env.emu
        .write_u32(env.layers + 8, if p2 { env.l2 as u32 } else { 0 });
    env.emu
        .write_u32(env.layers + 0xc, if p3 { env.l3 as u32 } else { 0 });
    let shp = if psh { env.sh as u32 } else { 0 };
    let lay = [
        if p1 { Some(a1) } else { None },
        if p2 { Some(a2) } else { None },
        if p3 { Some(a3) } else { None },
    ];
    let shr = if psh { Some(ash) } else { None };
    (shp, lay, shr)
}

#[test]
fn mcal_decode64() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let DecodeEnv { mut emu, out, data } = decode_env(&wow);
    let mut checked = 0u32;
    for case in 0..40u32 {
        let mcal = synth_bytes(2048, 0xa11a_0000 + case);
        emu.write(data, &mcal);
        let mcal_a: &[u8; 2048] = mcal.as_slice().try_into().unwrap();
        emu.call(0x6b0ac0, out as u32, data as u32, &[]);
        let mut ours64 = [0u16; 4096];
        benilla_re_terrain::mcal_decode64(&mut ours64, mcal_a);
        assert_eq!(read16(&emu, out, 4096), ours64.to_vec(), "mcal_decode64");
        checked += 1;
    }
    eprintln!("mcal_decode64: {checked} decodes bit-exact vs WoW.exe @ 0x6b0ac0");
}

#[test]
fn mcal_decode32() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let DecodeEnv { mut emu, out, data } = decode_env(&wow);
    let mut checked = 0u32;
    for case in 0..40u32 {
        let mcal = synth_bytes(2048, 0xa11a_0000 + case);
        emu.write(data, &mcal);
        let mcal_a: &[u8; 2048] = mcal.as_slice().try_into().unwrap();
        emu.call(0x6b0b30, out as u32, data as u32, &[]);
        let mut ours32 = [0u16; 1024];
        benilla_re_terrain::mcal_decode32(&mut ours32, mcal_a);
        assert_eq!(read16(&emu, out, 1024), ours32.to_vec(), "mcal_decode32");
        checked += 1;
    }
    eprintln!("mcal_decode32: {checked} decodes bit-exact vs WoW.exe @ 0x6b0b30");
}

#[test]
fn mcal_decode63() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let DecodeEnv { mut emu, out, data } = decode_env(&wow);
    let mut checked = 0u32;
    for case in 0..40u32 {
        let mcal = synth_bytes(2048, 0xa11a_0000 + case);
        emu.write(data, &mcal);
        let mcal_a: &[u8; 2048] = mcal.as_slice().try_into().unwrap();
        emu.call(0x6b0bf0, out as u32, data as u32, &[]);
        let mut ours64 = [0u16; 4096];
        benilla_re_terrain::mcal_decode63(&mut ours64, mcal_a);
        assert_eq!(read16(&emu, out, 4096), ours64.to_vec(), "mcal_decode63");
        checked += 1;
    }
    eprintln!("mcal_decode63: {checked} decodes bit-exact vs WoW.exe @ 0x6b0bf0");
}

#[test]
fn mcal_decode32_63() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let DecodeEnv { mut emu, out, data } = decode_env(&wow);
    let mut checked = 0u32;
    for case in 0..40u32 {
        let mcal = synth_bytes(2048, 0xa11a_0000 + case);
        emu.write(data, &mcal);
        let mcal_a: &[u8; 2048] = mcal.as_slice().try_into().unwrap();
        emu.call(0x6b0c80, out as u32, data as u32, &[]);
        let mut ours32 = [0u16; 1024];
        benilla_re_terrain::mcal_decode32_63(&mut ours32, mcal_a);
        assert_eq!(read16(&emu, out, 1024), ours32.to_vec(), "mcal_decode32_63");
        checked += 1;
    }
    eprintln!("mcal_decode32_63: {checked} decodes bit-exact vs WoW.exe @ 0x6b0c80");
}

#[test]
fn mcsh_decode() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let DecodeEnv { mut emu, out, data } = decode_env(&wow);
    let mut checked = 0u32;
    for case in 0..40u32 {
        let mcal = synth_bytes(2048, 0xa11a_0000 + case);
        emu.write(data, &mcal);
        let mcsh_a: &[u8; 512] = mcal[..512].try_into().unwrap();
        emu.call(0x6b0d30, out as u32, data as u32, &[]);
        let mut ours64 = [0u16; 4096];
        benilla_re_terrain::mcsh_decode(&mut ours64, mcsh_a);
        assert_eq!(read16(&emu, out, 4096), ours64.to_vec(), "mcsh_decode");
        checked += 1;
    }
    eprintln!("mcsh_decode: {checked} decodes bit-exact vs WoW.exe @ 0x6b0d30");
}

#[test]
fn mcsh_decode32() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let DecodeEnv { mut emu, out, data } = decode_env(&wow);
    let mut checked = 0u32;
    for case in 0..40u32 {
        let mcal = synth_bytes(2048, 0xa11a_0000 + case);
        emu.write(data, &mcal);
        let mcsh_a: &[u8; 512] = mcal[..512].try_into().unwrap();
        emu.call(0x6b0da0, out as u32, data as u32, &[]);
        let mut ours32 = [0u16; 1024];
        benilla_re_terrain::mcsh_decode32(&mut ours32, mcsh_a);
        assert_eq!(read16(&emu, out, 1024), ours32.to_vec(), "mcsh_decode32");
        checked += 1;
    }
    eprintln!("mcsh_decode32: {checked} decodes bit-exact vs WoW.exe @ 0x6b0da0");
}

#[test]
fn alpha_texel_64() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut env, blobs) = alpha_env(&wow);
    let (a1, a2, a3, ash) = (&blobs.d1, &blobs.d2, &blobs.d3, &blobs.ds);
    let (out, layers) = (env.out, env.layers);
    let mut checked = 0u32;
    for mask in 0..16u32 {
        let (shp, lay, shr) = alpha_wire(&mut env, mask, a1, a2, a3, ash);
        env.emu.call(0x6b03d0, out as u32, layers as u32, &[shp]);
        let mut ours64 = [0u16; 4096];
        benilla_re_terrain::alpha_texel_64(&mut ours64, lay, shr);
        assert_eq!(
            read16(&env.emu, out, 4096),
            ours64.to_vec(),
            "alpha_texel_64 m={mask}"
        );
        checked += 1;
    }
    eprintln!("alpha_texel_64: {checked} combines bit-exact vs WoW.exe @ 0x6b03d0");
}

#[test]
fn alpha_texel_32_edge() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut env, blobs) = alpha_env(&wow);
    let (a1, a2, a3, ash) = (&blobs.d1, &blobs.d2, &blobs.d3, &blobs.ds);
    let (out, layers) = (env.out, env.layers);
    let mut checked = 0u32;
    for mask in 0..16u32 {
        let (shp, lay, shr) = alpha_wire(&mut env, mask, a1, a2, a3, ash);
        env.emu.call(0x6b0510, out as u32, layers as u32, &[shp]);
        let mut ours32 = [0u16; 1024];
        benilla_re_terrain::alpha_texel_32_edge(&mut ours32, lay, shr);
        assert_eq!(
            read16(&env.emu, out, 1024),
            ours32.to_vec(),
            "alpha_texel_32_edge m={mask}"
        );
        checked += 1;
    }
    eprintln!("alpha_texel_32_edge: {checked} combines bit-exact vs WoW.exe @ 0x6b0510");
}

#[test]
fn alpha_texel_dup() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut env, blobs) = alpha_env(&wow);
    let (a1, a2, a3, ash) = (&blobs.d1, &blobs.d2, &blobs.d3, &blobs.ds);
    let (out, layers) = (env.out, env.layers);
    let mut checked = 0u32;
    for mask in 0..16u32 {
        let (shp, lay, shr) = alpha_wire(&mut env, mask, a1, a2, a3, ash);
        env.emu.call(0x6b0770, out as u32, layers as u32, &[shp]);
        let mut ours64 = [0u16; 4096];
        benilla_re_terrain::alpha_texel_dup(&mut ours64, lay, shr);
        assert_eq!(
            read16(&env.emu, out, 4096),
            ours64.to_vec(),
            "alpha_texel_dup m={mask}"
        );
        checked += 1;
    }
    eprintln!("alpha_texel_dup: {checked} combines bit-exact vs WoW.exe @ 0x6b0770");
}

#[test]
fn alpha_texel_32() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut env, blobs) = alpha_env(&wow);
    let (a1, a2, a3, ash) = (&blobs.d1, &blobs.d2, &blobs.d3, &blobs.ds);
    let (out, layers) = (env.out, env.layers);
    let mut checked = 0u32;
    for mask in 0..16u32 {
        let (shp, lay, shr) = alpha_wire(&mut env, mask, a1, a2, a3, ash);
        env.emu.call(0x6b08d0, out as u32, layers as u32, &[shp]);
        let mut ours32 = [0u16; 1024];
        benilla_re_terrain::alpha_texel_32(&mut ours32, lay, shr);
        assert_eq!(
            read16(&env.emu, out, 1024),
            ours32.to_vec(),
            "alpha_texel_32 m={mask}"
        );
        checked += 1;
    }
    eprintln!("alpha_texel_32: {checked} combines bit-exact vs WoW.exe @ 0x6b08d0");
}

//! The use range each game object handler constructor leaves at `+0xc`, against the exe. Every
//! constructor of `HANDLER_CONSTRUCTORS` runs whole on a zeroed object, and the dword at `+0xc`
//! of the handler it builds is compared with `handler_use_range`. The type switch's jump table
//! (`0x5f76cc`) is read from the image and each case must call its constructor.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::gameobject::{handler_use_range, HANDLER_CONSTRUCTORS};

const TYPE_SWITCH_TABLE: u64 = 0x5f76cc;
const STATIC_HANDLER_CASE: u32 = 0x5f76a0;

fn rel_call_targets(e: &Emu, from: u32, len: u32) -> Vec<u32> {
    let code = e.read(u64::from(from), (len + 5) as usize);
    let mut out = Vec::new();
    for p in 0..len as usize {
        if code[p] == 0xe8 {
            let rel = i32::from_le_bytes(code[p + 1..p + 5].try_into().unwrap());
            out.push((from as i64 + p as i64 + 5 + i64::from(rel)) as u32);
        }
    }
    out
}

#[test]
fn go_handler_use_range_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for (t, ctor) in HANDLER_CONSTRUCTORS.iter().enumerate() {
        let case = e.read_u32(TYPE_SWITCH_TABLE + 4 * t as u64);
        match ctor {
            Some(c) => assert!(
                rel_call_targets(&e, case, 0x40).contains(c),
                "type {t}: case {case:#x} does not call {c:#x}"
            ),
            None => assert_eq!(case, STATIC_HANDLER_CASE, "type {t}"),
        }
    }

    let go = e.alloc(0x400);
    let fields = e.alloc(0x100);
    e.write_u32(go + 0x110, fields as u32);
    for (t, ctor) in HANDLER_CONSTRUCTORS.iter().enumerate() {
        let want = handler_use_range(t as u32);
        let Some(ctor) = ctor else {
            assert_eq!(want, None, "type {t}");
            continue;
        };
        let handler = e.alloc(0x100);
        e.write(handler, &[0xaa; 0x100]);
        let ret = e.call(u64::from(*ctor), handler as u32, 0, &[go as u32]);
        assert_eq!(
            u64::from(ret),
            handler,
            "type {t}: the constructor returns this"
        );
        assert_eq!(e.read_u32(handler + 4), go as u32, "type {t}: the object");
        let bin = f32::from_bits(e.read_u32(handler + 0xc));
        let ours = want.expect("a constructor has a range");
        assert_eq!(
            bin.to_bits(),
            ours.to_bits(),
            "type {t}: ctor {ctor:#x} binary={bin} ours={ours}"
        );
    }
    assert_eq!(handler_use_range(31), None);
    assert_eq!(handler_use_range(u32::MAX), None);
}

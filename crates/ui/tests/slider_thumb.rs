//! `0x789860`, the slider's thumb install, against the exe, run whole
//! (`systems/ui/slider.md#the-thumb-texture`). `CScriptRegion::SetRegionParent` (`0x77fd10`) and
//! `CLayoutFrame::ClearAllPointsThunk` (`0x767ed0`) are stubbed to log their object and
//! arguments, and each texture's vtable slot 0, its deleting destructor, points at a stub that
//! logs the texture and its argument.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::slider_thumb::{install_thumb, ThumbCall};
use std::cell::RefCell;
use std::rc::Rc;

const INSTALL: u64 = 0x789860;
const SET_REGION_PARENT: u64 = 0x77fd10;
const CLEAR_ALL_POINTS: u64 = 0x767ed0;

#[test]
fn slider_install_thumb_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // (call, ecx, args): the destructor logs [texture, 1], the link [texture, slider, layer, 1].
    let log: Rc<RefCell<Vec<ThumbCall>>> = Rc::default();
    let parents: Rc<RefCell<Vec<(u32, u32)>>> = Rc::default();
    let dtor = e.alloc(0x10);
    e.write(dtor, &[0xc2, 0x04, 0x00]);
    {
        let log = log.clone();
        e.stub_fn(dtor, 1, move |io| {
            assert_eq!(io.arg(0), 1, "the destructor frees");
            log.borrow_mut()
                .push(ThumbCall::Delete(io.reg(RegisterX86::ECX) as u32));
            0
        });
    }
    {
        let log = log.clone();
        let parents = parents.clone();
        e.stub_fn(SET_REGION_PARENT, 3, move |io| {
            parents.borrow_mut().push((io.arg(0), io.arg(2)));
            log.borrow_mut().push(ThumbCall::Link {
                texture: io.reg(RegisterX86::ECX) as u32,
                layer: io.arg(1),
            });
            0
        });
    }
    {
        let log = log.clone();
        e.stub_fn(CLEAR_ALL_POINTS, 0, move |io| {
            log.borrow_mut()
                .push(ThumbCall::ClearPoints(io.reg(RegisterX86::ECX) as u32));
            0
        });
    }
    let vtable = e.alloc(0x40);
    e.write_u32(vtable, dtor as u32);
    let textures: Vec<u32> = (0..3)
        .map(|_| {
            let t = e.alloc(0x100);
            e.write_u32(t, vtable as u32);
            t as u32
        })
        .collect();
    let slider = e.alloc(0x340);
    let choices = [0, textures[0], textures[1], textures[2]];
    for &current in &choices {
        for &incoming in &choices {
            for &layer in &[0u32, 2, 3, 4] {
                for &flags in &[0u32, 1, 2, 6, 0xffff_fffe, 0xffff_ffff] {
                    e.write_u32(slider + 0x328, current);
                    e.write_u32(slider + 0x314, flags);
                    log.borrow_mut().clear();
                    parents.borrow_mut().clear();
                    e.call(INSTALL, slider as u32, 0, &[incoming, layer]);
                    let (thumb, want_flags, calls) = install_thumb(current, flags, incoming, layer);
                    let what = format!(
                        "current {current:#x} incoming {incoming:#x} layer {layer} flags {flags:#x}"
                    );
                    assert_eq!(e.read_u32(slider + 0x328), thumb, "{what}: thumb");
                    assert_eq!(e.read_u32(slider + 0x314), want_flags, "{what}: flags");
                    assert_eq!(log.borrow().as_slice(), calls.as_slice(), "{what}: calls");
                    for &(parent, show) in parents.borrow().iter() {
                        assert_eq!((parent, show), (slider as u32, 1), "{what}: link");
                    }
                }
            }
        }
    }
}

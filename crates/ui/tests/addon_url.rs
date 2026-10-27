//! The URL file's parse, `0x51d500`, run whole against `benilla_re_ui::addon_url`
//! (`systems/ui/addons.md#reading-the-url-file`). `SFile::Load` (`0x648620`) answers the file the
//! test holds, a NUL after it as the exe's extra byte makes one, or a failure; `SFile::Unload`
//! (`0x648700`) and `SMemAlloc` (`0x6462e0`) are stubs. `SStrChr2`, `SStrDupA` and `SStrLen` run as
//! the exe's.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::addon_url::url_text;
use std::cell::Cell;
use std::rc::Rc;

const PARSE: u64 = 0x51d500;
const LOAD: u64 = 0x648620;
const UNLOAD: u64 = 0x648700;
const ALLOC: u64 = 0x6462e0;

const FILES: &[&[u8]] = &[
    b"",
    b"=",
    b"URL=",
    b"URL=x",
    b"URL=http://example.invalid/a?b=c \r\n",
    b"[InternetShortcut]\r\nURL=http://h/\r\n",
    b"no equals here",
    b"a=b=c  \n\n",
    b"=lead",
    b"URL=x\0=y",
    b"x\0URL=y",
    b"URL=   ",
    b"URL=\r\n\r\n \n",
    b"URL= a b \t",
    b"URL=a\tb\t",
    b"\r\n=\r\n",
    b"==",
    b"=\0",
];

#[test]
fn addon_url_parse_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let path = e.alloc(0x40);
    let out = e.alloc(8);
    let file = e.alloc(0x400);
    let arena = e.alloc(0x2000);
    e.stub_alloc(ALLOC, arena, 0x100, 4);
    e.stub(UNLOAD, 1, 1);
    let found: Rc<Cell<bool>> = Rc::default();
    {
        let found = found.clone();
        e.stub_fn(LOAD, 7, move |io| {
            if !found.get() {
                return 0;
            }
            let buffer = u64::from(io.arg(2));
            io.write_u32(buffer, file as u32);
            1
        });
    }
    // A file that does not load leaves the slot as it was.
    e.write(out, &[0; 8]);
    e.call(PARSE, path as u32, out as u32, &[]);
    assert_eq!(e.read_u32(out), 0);
    found.set(true);
    for (case, &content) in FILES.iter().enumerate() {
        // The buffer `SFile::Load` returns, with the extra zero byte.
        let mut buf = content.to_vec();
        buf.push(0);
        e.write(file, &buf);
        e.write(out, &[0; 8]);
        e.call(PARSE, path as u32, out as u32, &[]);
        let got_ptr = e.read_u32(out);
        let got = (got_ptr != 0).then(|| {
            let b = e.read(u64::from(got_ptr), 0xff);
            b[..b.iter().position(|&c| c == 0).unwrap()].to_vec()
        });
        assert_eq!(got, url_text(&buf), "case {case}: {content:?}");
    }
}

//! `cargo xtask listing`: writes the disassembly the findings cite, `dumps/disasm.txt`, as
//! `tools/disasm.sh` asks for it.
//!
//! A linear sweep of `.text` decodes the data inside code (a switch's jump table, say) as
//! instructions, and can stay out of step with the real instruction stream after it, hiding the
//! next function's entry inside a false instruction. So the decode restarts at every address the
//! code is known to start at: every function row and `call` target, every jump-table entry, and
//! the end of every jump table and byte index table, the tables being those the bodies of the rows
//! and the `call` targets index (`bodies::jump_tables`, `bodies::all_index_tables`). A table's
//! bytes are left out. The listing is GNU objdump's decode of each stretch between two such
//! points, `--start-address` to `--stop-address`, in address order, under the sweep's header:
//! where the sweep is already in step it is that stretch's decode, and objdump runs afresh only
//! where it is not. Finding the tables needs the listing, so the two are repeated until no table
//! is new.

use crate::bodies::{self, Image, Listing, Tables};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

/// The rounds of finding tables and decoding again after which a table still new is an error.
const ROUNDS: usize = 8;

/// One line of objdump's output: an instruction's address and the line as printed, or no
/// address for the `...` objdump prints over a run of zero bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub address: Option<u32>,
    pub text: String,
}

/// objdump's output split into its header, through the first `<label>:` line, and its lines.
/// Blank lines and later labels, which a decode from a start address prints, are dropped.
pub fn split(output: &str) -> (Vec<String>, Vec<Line>) {
    let mut header = Vec::new();
    let mut lines = Vec::new();
    let mut in_header = true;
    for text in output.lines() {
        let label = text.ends_with(">:") && !text.starts_with(char::is_whitespace);
        if in_header {
            header.push(text.to_string());
            in_header = !label;
            continue;
        }
        if label || text.trim().is_empty() {
            continue;
        }
        lines.push(Line {
            address: address_of(text),
            text: text.to_string(),
        });
    }
    (header, lines)
}

fn address_of(text: &str) -> Option<u32> {
    let (address, rest) = text.trim_start().split_once(':')?;
    rest.starts_with('\t').then_some(())?;
    if address.is_empty() || !address.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(address, 16).ok()
}

/// A stretch the decode covers: from `start` to `stop`, or to the end of `.text`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stretch {
    pub start: u32,
    pub stop: Option<u32>,
}

/// The stretches from `first`: one from each restart point to the next, or to the next skipped
/// range. A restart inside a skipped range is dropped; the end of each skipped range is one.
pub fn stretches(
    first: u32,
    restarts: impl IntoIterator<Item = u32>,
    skipped: &[(u32, u32)],
) -> Vec<Stretch> {
    let inside = |a: u32| {
        let i = skipped.partition_point(|r| r.0 <= a);
        i > 0 && a < skipped[i - 1].1
    };
    let points: BTreeSet<u32> = restarts
        .into_iter()
        .chain(skipped.iter().map(|r| r.1))
        .chain([first])
        .filter(|&a| a >= first && !inside(a))
        .collect();
    let points: Vec<u32> = points.into_iter().collect();
    points
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            let next = points.get(i + 1).copied();
            let skip = skipped
                .get(skipped.partition_point(|r| r.0 <= start))
                .map(|r| r.0);
            let stop = match (next, skip) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
            Stretch { start, stop }
        })
        .collect()
}

/// The decode of every stretch, in order. `sweep` is the decode of all of `.text`; `decode`
/// decodes one stretch afresh; `zero` says whether the byte at an address is zero.
///
/// The sweep is a stretch's decode when it has an instruction at the stretch's start and at its
/// stop, and the byte before the stop is not zero (objdump treats zeros that run up to the stop
/// address differently). When only the stop is missed, the last instruction runs past it, and
/// only that instruction is decoded afresh, which cuts it at the stop.
pub fn splice(
    sweep: &[Line],
    stretches: &[Stretch],
    mut decode: impl FnMut(Stretch) -> io::Result<Vec<Line>>,
    zero: impl Fn(u32) -> bool,
) -> io::Result<Vec<Line>> {
    let positions: Vec<(u32, usize)> = sweep
        .iter()
        .enumerate()
        .filter_map(|(i, line)| Some((line.address?, i)))
        .collect();
    let find = |address: u32| -> Result<usize, usize> {
        positions.binary_search_by_key(&address, |p| p.0)
    };
    let mut out = Vec::new();
    for &stretch in stretches {
        let in_step = find(stretch.start).ok();
        let clean_stop = stretch.stop.is_none_or(|stop| !zero(stop.wrapping_sub(1)));
        let Some(from) = in_step.filter(|_| clean_stop) else {
            out.extend(decode(stretch)?);
            continue;
        };
        let Some(stop) = stretch.stop else {
            out.extend_from_slice(&sweep[positions[from].1..]);
            continue;
        };
        match find(stop) {
            Ok(to) => out.extend_from_slice(&sweep[positions[from].1..positions[to].1]),
            Err(to) => {
                let (last, line) = positions[to - 1];
                out.extend_from_slice(&sweep[positions[from].1..line]);
                out.extend(decode(Stretch {
                    start: last,
                    stop: Some(stop),
                })?);
            }
        }
    }
    Ok(out)
}

/// The listing: the sweep restarted at `starts`, then at the jump tables' entries and ends with
/// their bytes left out, until the tables settle. Returns the header, the lines, the tables and
/// the stretches decoded afresh.
pub fn build(
    starts: &[u32],
    image: &Image,
    mut objdump: impl FnMut(Option<Stretch>) -> io::Result<String>,
) -> io::Result<(Vec<String>, Vec<Line>, Tables, usize)> {
    let (header, sweep) = split(&objdump(None)?);
    let first = sweep
        .iter()
        .find_map(|l| l.address)
        .ok_or_else(|| io::Error::other("objdump printed no instruction"))?;
    let mut cache: HashMap<Stretch, Vec<Line>> = HashMap::new();
    let mut tables = Tables::new();
    let mut index: Vec<(u32, u32)> = Vec::new();
    // A `call` target is a function start whether or not a row holds it.
    let mut entries: Vec<u32> = starts.to_vec();
    for _ in 0..ROUNDS {
        let mut skipped = bodies::table_ranges(&tables);
        skipped.extend(index.iter().copied());
        let skipped = bodies::join(skipped);
        let restarts = entries
            .iter()
            .copied()
            .chain(tables.values().flatten().copied());
        let plan = stretches(first, restarts, &skipped);
        let lines = splice(
            &sweep,
            &plan,
            |stretch| {
                if let Some(lines) = cache.get(&stretch) {
                    return Ok(lines.clone());
                }
                let lines = split(&objdump(Some(stretch))?).1;
                cache.insert(stretch, lines.clone());
                Ok(lines)
            },
            |address| image.byte(address) == Some(0),
        )?;
        let listing = listing_of(&lines)?;
        let mut called: Vec<u32> = starts.to_vec();
        called.extend(
            lines
                .iter()
                .filter_map(|l| l.text.split_once("call").map(|(_, rest)| rest.trim()))
                .filter_map(|t| t.strip_prefix("0x"))
                .filter_map(|t| u32::from_str_radix(t, 16).ok())
                .filter(|&t| listing.contains(t)),
        );
        called.sort_unstable();
        called.dedup();
        let mut grown = tables.clone();
        let no_return = bodies::no_return(&called, &listing, image);
        bodies::merge_tables(
            &mut grown,
            bodies::jump_tables(&called, &listing, image, &no_return),
        );
        let mut more = index.clone();
        more.extend(bodies::all_index_tables(
            &called, &listing, image, &no_return,
        ));
        let more = bodies::join(more);
        if grown == tables && more == index && called == entries {
            return Ok((header, lines, tables, cache.len()));
        }
        tables = grown;
        index = more;
        entries = called;
    }
    Err(io::Error::other(format!(
        "the jump tables still grow after {ROUNDS} rounds"
    )))
}

fn listing_of(lines: &[Line]) -> io::Result<Listing> {
    let mut text = String::new();
    for line in lines.iter().filter(|l| l.address.is_some()) {
        text.push_str(&line.text);
        text.push('\n');
    }
    Listing::parse(text.as_bytes())
}

/// Runs `objdump` on `exe` for the whole of `.text` or one stretch of it, beside the exe so
/// that the header names `WoW.exe` and no path on this machine.
fn run_objdump(objdump: &Path, exe: &Path, stretch: Option<Stretch>) -> io::Result<String> {
    let mut command = Command::new(objdump);
    command
        .current_dir(exe.parent().unwrap_or(Path::new(".")))
        .args(["-d", "-M", "intel", "--no-show-raw-insn", "--section=.text"]);
    if let Some(stretch) = stretch {
        command.arg(format!("--start-address={:#x}", stretch.start));
        if let Some(stop) = stretch.stop {
            command.arg(format!("--stop-address={stop:#x}"));
        }
    }
    command.arg(exe.file_name().unwrap_or(exe.as_os_str()));
    let output = command.output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "{} failed ({}): {}",
            objdump.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout).map_err(io::Error::other)
}

/// Writes the listing of `exe` to `out` with `objdump`, restarting at `starts`; returns a
/// summary line.
pub fn write(starts: &[u32], exe: &Path, objdump: &Path, out: &Path) -> io::Result<String> {
    let bytes = fs::read(exe)?;
    let image =
        Image::new(&bytes).ok_or_else(|| io::Error::other("the exe has no section table"))?;
    let (header, lines, tables, fresh) =
        build(starts, &image, |stretch| run_objdump(objdump, exe, stretch))?;
    let mut text = String::new();
    for line in header.iter().chain(lines.iter().map(|l| &l.text)) {
        text.push_str(line);
        text.push('\n');
    }
    fs::write(out, text)?;
    Ok(format!(
        "{} lines; {} stretches decoded afresh; {} jump tables left out",
        header.len() + lines.len(),
        fresh,
        tables.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<Line> {
        split(&format!(
            "\nWoW.exe:     file format pei-i386\n\n00401000 <.text>:\n{text}"
        ))
        .1
    }

    #[test]
    fn split_keeps_the_header_and_drops_labels_and_blank_lines() {
        let (header, body) = split(
            "\nWoW.exe:     file format pei-i386\n\n\nDisassembly of section .text:\n\n\
             00401000 <.text>:\n  401000:\tpush   ebp\n\t...\n\n00401010 <.text+0x10>:\n  \
             401010:\tret\n",
        );
        assert_eq!(header.len(), 7);
        assert_eq!(header[6], "00401000 <.text>:");
        let addresses: Vec<Option<u32>> = body.iter().map(|l| l.address).collect();
        assert_eq!(addresses, [Some(0x401000), None, Some(0x401010)]);
    }

    #[test]
    fn stretches_restart_at_each_point_and_stop_at_each_skipped_range() {
        let plan = stretches(
            0x401000,
            [0x401020, 0x401034, 0x401010],
            &[(0x401030, 0x401038)],
        );
        let pairs: Vec<(u32, Option<u32>)> = plan.iter().map(|s| (s.start, s.stop)).collect();
        assert_eq!(
            pairs,
            [
                (0x401000, Some(0x401010)),
                (0x401010, Some(0x401020)),
                (0x401020, Some(0x401030)),
                (0x401038, None),
            ]
        );
    }

    /// The sweep reads the jump table at 0x401010 as code and runs on out of step, so its
    /// `add` at 0x401017 swallows the entry of the function at 0x401018. The resynchronised
    /// listing leaves the table out and starts an instruction at 0x401018.
    const SWEEP: &str = "  401000:\tmov    eax,ecx
  401002:\tjmp    DWORD PTR [eax*4+0x401010]
  401009:\tret
  40100a:\tnop
  40100b:\tnop
  40100c:\tnop
  40100d:\tnop
  40100e:\tnop
  40100f:\tnop
  401010:\tor     DWORD PTR [eax+0x0],0x40
  401017:\tadd    BYTE PTR [ebp-0x75],dl
  40101a:\tin     al,dx
  40101b:\tret
";

    fn fresh(stretch: Stretch) -> io::Result<Vec<Line>> {
        let text = match (stretch.start, stretch.stop) {
            (0x401018, None) => {
                "  401018:\tpush   ebp\n  401019:\tmov    ebp,esp\n  40101b:\tret\n"
            }
            (0x401014, Some(0x401018)) => {
                "  401014:\tint3\n  401015:\tint3\n  401016:\tint3\n  401017:\tint3\n"
            }
            (0x401017, Some(0x401018)) => "  401017:\t.byte 0x0\n",
            other => panic!("decodes {other:x?} afresh"),
        };
        Ok(lines(text))
    }

    #[test]
    fn a_start_the_sweep_steps_over_is_decoded_afresh() {
        let sweep = lines(SWEEP);
        let plan = stretches(0x401000, [0x401018, 0x401009], &[(0x401010, 0x401018)]);
        let out = splice(&sweep, &plan, fresh, |_| false).expect("splices");
        let addresses: Vec<u32> = out.iter().filter_map(|l| l.address).collect();
        assert_eq!(
            addresses,
            [
                0x401000, 0x401002, 0x401009, 0x40100a, 0x40100b, 0x40100c, 0x40100d, 0x40100e,
                0x40100f, 0x401018, 0x401019, 0x40101b
            ]
        );
    }

    #[test]
    fn a_stop_the_sweep_runs_past_is_cut_there() {
        let sweep = lines(SWEEP);
        let plan = [Stretch {
            start: 0x401000,
            stop: Some(0x401018),
        }];
        let cut = |stretch: Stretch| {
            assert_eq!((stretch.start, stretch.stop), (0x401017, Some(0x401018)));
            Ok(lines("  401017:\t.byte 0x0\n"))
        };
        let out = splice(&sweep, &plan, cut, |_| false).expect("splices");
        assert_eq!(
            out.last().map(|l| l.text.as_str()),
            Some("  401017:\t.byte 0x0")
        );
        assert_eq!(out.len(), 11);
    }

    #[test]
    fn zeros_up_to_a_stop_are_decoded_afresh() {
        let sweep = lines(SWEEP);
        let plan = [Stretch {
            start: 0x401009,
            stop: Some(0x40101a),
        }];
        let whole = |stretch: Stretch| {
            assert_eq!((stretch.start, stretch.stop), (0x401009, Some(0x40101a)));
            Ok(lines("  401009:\tret\n\t...\n"))
        };
        let out = splice(&sweep, &plan, whole, |a| a == 0x401019).expect("splices");
        assert_eq!(out.len(), 2);
    }

    /// A PE with one section at 0x401000 of `nop` bytes, but for a one-entry jump table at
    /// 0x401010 and the `int3`s after it.
    fn image_bytes() -> Vec<u8> {
        let mut bytes = vec![0u8; 0x400];
        let put = |bytes: &mut Vec<u8>, at: usize, value: u32| {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        };
        put(&mut bytes, 0x3c, 0x40);
        bytes[0x46] = 1; // one section
        bytes[0x54] = 0xe0; // the optional header's size
        put(&mut bytes, 0x40 + 52, 0x400000);
        let section = 0x40 + 24 + 0xe0;
        put(&mut bytes, section + 12, 0x1000);
        put(&mut bytes, section + 16, 0x200);
        put(&mut bytes, section + 20, 0x200);
        bytes[0x200..].fill(0x90);
        put(&mut bytes, 0x210, 0x401009);
        put(&mut bytes, 0x214, 0xcccccccc);
        bytes
    }

    /// The whole pipeline on the fixture: the first round restarts at the row 0x401018 the
    /// sweep steps over, the walk from 0x401000 finds the table, and the second round leaves the
    /// table out and restarts after it, where nothing further changes.
    #[test]
    fn the_listing_follows_the_instruction_stream() {
        let bytes = image_bytes();
        let image = Image::new(&bytes).expect("a PE");
        let objdump = |stretch: Option<Stretch>| match stretch {
            None => Ok(format!(
                "\nWoW.exe:     file format pei-i386\n\n\nDisassembly of section .text:\n\n\
                 00401000 <.text>:\n{SWEEP}"
            )),
            Some(stretch) => Ok(fresh(stretch)?.into_iter().fold(
                format!(
                    "\n{:08x} <.text+0x{:x}>:\n",
                    stretch.start,
                    stretch.start - 0x401000
                ),
                |text, line| text + &line.text + "\n",
            )),
        };
        let (header, lines, tables, fresh) =
            build(&[0x401000, 0x401018], &image, objdump).expect("builds");
        assert_eq!(header.len(), 7);
        assert_eq!(tables, Tables::from([(0x401010, vec![0x401009])]));
        assert_eq!(fresh, 3);
        let addresses: Vec<u32> = lines.iter().filter_map(|l| l.address).collect();
        assert_eq!(
            addresses,
            [
                0x401000, 0x401002, 0x401009, 0x40100a, 0x40100b, 0x40100c, 0x40100d, 0x40100e,
                0x40100f, 0x401014, 0x401015, 0x401016, 0x401017, 0x401018, 0x401019, 0x40101b
            ]
        );
    }
}

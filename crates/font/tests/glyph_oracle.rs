//! Glyph-metric oracle: a check of the FreeType boundary that is not circular.
//!
//! The emulated tests compare `compute_step`, `glyph_width_scaled` and `char_advance` bit for bit
//! with `WoW.exe`, but over synthesized per-glyph advances: FreeType, the source of the real glyph
//! step-base metric (`CharCodeDesc+0x4c`) and the raw kern, is the one stubbed boundary the emulator
//! cannot reach. This test validates the composition those functions build against the real client's
//! own `FontString:GetStringWidth` output, captured in the running client (the half the emulator
//! cannot reach): a SavedVariables file written by the addon `tools/capture/BenillaReCapture`, read
//! from the path in `BENILLA_RE_GLYPH_CAPTURE`.
//!
//! The check is structural and not circular (not libm against libm): the string-width path lays a run
//! out as a per-glyph step `char_advance(cur)` (`glyph_width_scaled` plus the integer advance) plus
//! the explicit pairwise kern from the escapement (`0x5ca2d0`, `0x5ca4b0`), accumulated across the
//! string. That layout is strictly pairwise additive: each glyph's contribution depends only on
//! `(prev, cur)`. The capture gives, per (face, size), the single-glyph widths
//! `single(c) = GetStringWidth(c)`, the kern deltas `delta(L,R) = GetStringWidth(LR) - single(L) -
//! single(R)`, and full pangram widths. The model predicts `width(S) = sum single(S_i) + sum
//! delta(S_{i-1}, S_i)` and requires it to reproduce the captured pangram width. A real layout with a
//! 3-glyph context, sub-pixel accumulation, or a non-pairwise effect would fail (a structural error
//! shows as whole-unit residuals; the non-ASCII multi-byte pangram, modelled as per-byte glyphs,
//! leaves residuals of about 67 units), so it confirms that the FreeType-boundary layout is the
//! pairwise-additive composition that `glyph_width_scaled`, `char_advance` and the escapement
//! functions build, which the synthesized-advance emulated tests assume. The captured deltas carry the
//! raw FreeType kern (positive and negative); the string-width path keeps it, whereas
//! `compute_step`'s `min(rawKern,0)` clamp belongs to the wrap and measure path (tested bit for bit
//! in the emulator, not here).
//!
//! The capture comes from a run of the real client, so the test skips without the file.

use benilla_re_difftest::fixture;
use std::collections::HashMap;

// ---- minimal Lua-table-literal parser (the SavedVariables pretty-print subset) ----------------

#[derive(Debug)]
enum Val {
    Num(f64),
    Str(String),
    Tab(Vec<(Key, Val)>),
}
#[derive(Debug, Clone, PartialEq)]
enum Key {
    Int(i64),
    Name(String),
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() {
            match self.b[self.i] {
                b' ' | b'\t' | b'\n' | b'\r' => self.i += 1,
                _ => break,
            }
        }
    }

    fn value(&mut self) -> Val {
        self.ws();
        match self.b[self.i] {
            b'{' => self.table(),
            b'"' => Val::Str(self.string()),
            _ => self.number(),
        }
    }

    fn table(&mut self) -> Val {
        debug_assert_eq!(self.b[self.i], b'{');
        self.i += 1; // '{'
        let mut fields = Vec::new();
        loop {
            self.ws();
            if self.b[self.i] == b'}' {
                self.i += 1;
                break;
            }
            // field: '[' key ']' '=' value  (the addon always emits the bracketed form)
            debug_assert_eq!(self.b[self.i], b'[');
            self.i += 1; // '['
            self.ws();
            let key = if self.b[self.i] == b'"' {
                Key::Name(self.string())
            } else {
                match self.number() {
                    Val::Num(n) => Key::Int(n as i64),
                    _ => unreachable!(),
                }
            };
            self.ws();
            debug_assert_eq!(self.b[self.i], b']');
            self.i += 1; // ']'
            self.ws();
            debug_assert_eq!(self.b[self.i], b'=');
            self.i += 1; // '='
            let v = self.value();
            fields.push((key, v));
            self.ws();
            if self.b[self.i] == b',' {
                self.i += 1;
            }
        }
        Val::Tab(fields)
    }

    fn string(&mut self) -> String {
        debug_assert_eq!(self.b[self.i], b'"');
        self.i += 1; // opening quote
        let mut s = Vec::new();
        while self.b[self.i] != b'"' {
            if self.b[self.i] == b'\\' {
                self.i += 1;
                // Lua escapes the fixture actually uses: \\ \" \n \t plus \ddd is unused here.
                match self.b[self.i] {
                    b'n' => s.push(b'\n'),
                    b't' => s.push(b'\t'),
                    b'r' => s.push(b'\r'),
                    other => s.push(other), // \\ -> \, \" -> ", etc.
                }
                self.i += 1;
            } else {
                s.push(self.b[self.i]);
                self.i += 1;
            }
        }
        self.i += 1; // closing quote
        String::from_utf8_lossy(&s).into_owned()
    }

    fn number(&mut self) -> Val {
        let start = self.i;
        while self.i < self.b.len()
            && matches!(
                self.b[self.i],
                b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'
            )
        {
            self.i += 1;
        }
        let txt = std::str::from_utf8(&self.b[start..self.i]).unwrap();
        Val::Num(txt.parse().unwrap_or(f64::NAN))
    }
}

impl Val {
    fn get<'a>(&'a self, name: &str) -> Option<&'a Val> {
        match self {
            Val::Tab(fs) => fs
                .iter()
                .find(|(k, _)| matches!(k, Key::Name(n) if n == name))
                .map(|(_, v)| v),
            _ => None,
        }
    }
    fn geti(&self, idx: i64) -> Option<&Val> {
        match self {
            Val::Tab(fs) => fs
                .iter()
                .find(|(k, _)| matches!(k, Key::Int(n) if *n == idx))
                .map(|(_, v)| v),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            Val::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self {
            Val::Num(n) => Some(*n),
            Val::Str(s) => s.parse().ok(),
            _ => None,
        }
    }
    /// The `(Key, Val)` fields, for iterating a table's numeric/named children.
    fn fields(&self) -> &[(Key, Val)] {
        match self {
            Val::Tab(fs) => fs,
            _ => &[],
        }
    }
}

#[test]
fn glyph_metric_oracle() {
    let Some(bytes) = fixture("BENILLA_RE_GLYPH_CAPTURE") else {
        return;
    };
    // Skip the assignment ahead of the table, to the first `{`.
    let start = bytes
        .iter()
        .position(|&c| c == b'{')
        .expect("fixture has a table");
    let db = Parser {
        b: &bytes,
        i: start,
    }
    .value();

    // probe index (1-based) -> codepoint, for mapping glyphs[face][size][flags][i] -> cp.
    let probes = db.get("probes").expect("DB.probes");
    let mut idx_cp: HashMap<i64, i64> = HashMap::new();
    for (k, v) in probes.fields() {
        if let (Key::Int(i), Some(cp)) = (k, v.get("cp").and_then(Val::as_f64)) {
            idx_cp.insert(*i, cp as i64);
        }
    }

    let glyphs = db.get("glyphs").expect("DB.glyphs");
    let kerning = db.get("kerning").expect("DB.kerning");
    let pangrams = db.get("pangrams").expect("DB.pangrams");

    let mut cases = 0usize;
    let mut max_resid = 0.0f64;
    let mut max_delta = f64::NEG_INFINITY; // kern range, reported (raw FreeType kern is two-sided)
    let mut min_delta = f64::INFINITY;
    let mut kern_pairs = 0usize;

    // Drive every (face, size) that has all three: singles, the kern matrix, and pangrams. The addon
    // captures the full kern matrix at only 2 sizes/face (cap_spread), so this naturally selects them.
    for (face_key, kern_faces) in kerning.fields() {
        let Key::Name(face) = face_key else { continue };
        for (size_key, kern_sz) in kern_faces.fields() {
            let Key::Int(size) = size_key else { continue };

            // singles for (face, size, "normal"): index i -> cp -> width
            let Some(singles_tab) = glyphs
                .get(face)
                .and_then(|f| f.geti(*size))
                .and_then(|s| s.get("normal"))
            else {
                continue;
            };
            let mut single: HashMap<i64, f64> = HashMap::new();
            for (k, v) in singles_tab.fields() {
                if let (Key::Int(i), Some(w)) = (k, v.as_f64()) {
                    if let Some(&cp) = idx_cp.get(i) {
                        single.insert(cp, w);
                    }
                }
            }

            // kern deltas: (l,r) -> d  (pairs absent from the nonzero list are additive: delta 0)
            let mut kern: HashMap<(i64, i64), f64> = HashMap::new();
            if let Some(deltas) = kern_sz.get("deltas") {
                for (_, e) in deltas.fields() {
                    if let (Some(l), Some(r), Some(d)) = (
                        e.get("l").and_then(Val::as_f64),
                        e.get("r").and_then(Val::as_f64),
                        e.get("d").and_then(Val::as_f64),
                    ) {
                        kern.insert((l as i64, r as i64), d);
                        max_delta = max_delta.max(d);
                        min_delta = min_delta.min(d);
                        kern_pairs += 1;
                    }
                }
            }

            let Some(pans) = pangrams.get(face).and_then(|f| f.geti(*size)) else {
                continue;
            };
            for (_, pan) in pans.fields() {
                let (Some(text), Some(want)) = (
                    pan.get("text").and_then(Val::as_str),
                    pan.get("w").and_then(Val::as_f64),
                ) else {
                    continue;
                };
                // ASCII-only pangrams: each byte is exactly one glyph (byte == codepoint), so the
                // captured single/kern (keyed by ASCII code) compose it. The mixed-locale pangram #5
                // has multi-byte UTF-8 glyphs whose layout is NOT a sum of per-byte singles — skip it
                // (a byte-decomposition would be a different, wrong measurement, not a real failure).
                let chars: Vec<i64> = text.bytes().map(i64::from).collect();
                if !chars
                    .iter()
                    .all(|&c| (0x20..0x7f).contains(&c) && single.contains_key(&c))
                {
                    continue;
                }
                // Accumulate left-to-right as the binary lays out the run: each glyph contributes its
                // own step `single(cur) + delta(prev,cur)` (= compute_step's per-glyph escapement, by
                // the delta definition), summed across the string.
                let mut acc = 0.0f64;
                let mut prev: Option<i64> = None;
                for &c in &chars {
                    let d = match prev {
                        Some(p) => *kern.get(&(p, c)).unwrap_or(&0.0),
                        None => 0.0, // first glyph: no preceding kern
                    };
                    acc += single[&c] + d;
                    prev = Some(c);
                }
                let resid = (acc - want).abs();
                max_resid = max_resid.max(resid);
                assert!(
                    resid < 1e-3,
                    "glyph oracle: {face} @{size} \"{text}\": model {acc} vs real GetStringWidth \
                     {want} (residual {resid:.6e}) — the pairwise-additive layout the synthesized \
                     diffs assume does not reproduce real FreeType composition",
                );
                cases += 1;
            }
        }
    }

    assert!(cases > 0, "glyph oracle ran 0 cases — fixture present but no (face,size) had all of singles+kerning+pangrams");
    eprintln!(
        "glyph_metric_oracle: {cases} real-FreeType pangram widths reproduced by the pairwise-additive \
         glyph_width/char_advance/escapement composition (max residual {max_resid:.3e} local units, \
         < 1e-3 — float accumulation-order noise, 4+ orders below a structural error); {kern_pairs} \
         real kern pairs span [{min_delta:.3}, {max_delta:.3}] (two-sided raw FreeType kern)"
    );
}

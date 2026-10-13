//! Lua's `math.random` against draws the real client made. `math.randomseed(N)` sets the CRT
//! state to `N`; each draw takes [`rand_step`]'s `r`. With no arguments the result is
//! `(r % 32767) · (1/32767)`, the reciprocal stored at `0x822d38` (a multiply, which can differ
//! from a divide in the last bit); `math.random(1, 1e6)` is `1 + floor(unit·1e6)`.
//!
//! The draws come from an addon run in the client, as a SavedVariables file with a
//! `["random"]` table of seeds and their draws, named by `BENILLA_RE_MATH_RANDOM_CAPTURE`. The test
//! skips without it.

#![allow(clippy::float_cmp)]

use benilla_re_difftest::fixture;
use benilla_re_random::msvc_rand::rand_step;

// A parser for the subset of Lua table literals SavedVariables files are written in.

enum Val {
    Num(f64),
    Str(String),
    Tab(Vec<(Key, Val)>),
}
#[derive(PartialEq)]
enum Key {
    Int(i64),
    Name(String),
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
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
        self.i += 1; // '{'
        let mut fields = Vec::new();
        loop {
            self.ws();
            if self.b[self.i] == b'}' {
                self.i += 1;
                break;
            }
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
            self.i += 1; // ']'
            self.ws();
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
        self.i += 1; // opening quote
        let mut s = Vec::new();
        while self.b[self.i] != b'"' {
            if self.b[self.i] == b'\\' {
                self.i += 1;
                match self.b[self.i] {
                    b'n' => s.push(b'\n'),
                    b't' => s.push(b'\t'),
                    b'r' => s.push(b'\r'),
                    other => s.push(other),
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
        Val::Num(
            std::str::from_utf8(&self.b[start..self.i])
                .unwrap()
                .parse()
                .unwrap_or(f64::NAN),
        )
    }
}

impl Val {
    fn get(&self, name: &str) -> Option<&Val> {
        match self {
            Val::Tab(fs) => fs
                .iter()
                .find(|(k, _)| matches!(k, Key::Name(n) if n == name))
                .map(|(_, v)| v),
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
    fn as_str(&self) -> Option<&str> {
        match self {
            Val::Str(s) => Some(s),
            _ => None,
        }
    }
    fn fields(&self) -> &[(Key, Val)] {
        match self {
            Val::Tab(fs) => fs,
            _ => &[],
        }
    }
}

/// One `math.random()` with no arguments, advancing the state: `fild` of the `idiv 0x7fff`
/// remainder, `fmul` by `1/32767` (`0x822d38`).
fn next_unit(state: &mut u32) -> f64 {
    const RECIP_32767: f64 = 1.0 / 32767.0;
    let (s, r) = rand_step(*state);
    *state = s;
    f64::from(r % 32767) * RECIP_32767
}

#[test]
fn math_random_oracle() {
    let Some(bytes) = fixture("BENILLA_RE_MATH_RANDOM_CAPTURE") else {
        return;
    };
    // Only the `["random"]` table is parsed.
    let needle = b"[\"random\"]";
    let pos = bytes
        .windows(needle.len())
        .position(|w| w == needle)
        .expect("fixture has DB.random");
    let brace = bytes[pos..].iter().position(|&c| c == b'{').unwrap() + pos;
    let random = Parser {
        b: &bytes,
        i: brace,
    }
    .value();

    let mut seeds = 0usize;
    let mut draws = 0usize;
    for (_, rec) in random.fields() {
        let Some(seed) = rec.get("seed").and_then(Val::as_f64) else {
            continue;
        };
        // math.randomseed(N) calls srand((int)N).
        let seed_state = seed as i64 as u32;

        // With no arguments: the f64 by its bits.
        if let Some(units) = rec.get("unit") {
            let mut state = seed_state;
            for (_, v) in units.fields() {
                let want = v.as_str().and_then(|s| s.parse::<f64>().ok()).unwrap();
                let got = next_unit(&mut state);
                assert_eq!(
                    got.to_bits(),
                    want.to_bits(),
                    "math.random() unit mismatch seed {seed}: ours {got:.17} vs captured {want:.17}"
                );
                draws += 1;
            }
        }
        // math.random(1, 1e6), from the seed again.
        if let Some(mns) = rec.get("mn") {
            let mut state = seed_state;
            for (_, v) in mns.fields() {
                let want = v.as_f64().unwrap() as i64;
                let unit = next_unit(&mut state);
                let got = 1 + (unit * 1_000_000.0).floor() as i64;
                assert_eq!(
                    got, want,
                    "math.random(1,1e6) mismatch seed {seed}: ours {got} vs captured {want}"
                );
            }
        }
        seeds += 1;
    }
    assert!(
        seeds > 0,
        "math_random_oracle ran 0 seeds — fixture present but DB.random empty"
    );
    eprintln!(
        "math_random_oracle: {seeds} seeds × {} draws match the client's math.random",
        draws / seeds.max(1)
    );
}

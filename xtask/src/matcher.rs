//! `cargo xtask match <exe> <out>`: finds the function rows of another build by their signatures
//! (`signature.rs`) and writes the symbol table rebased onto it.
//!
//! - `<out>/match.tsv`: `address`, `name`, `found`, `via`: the row's address in 5875, its name,
//!   its address in the build (`missing` when the pattern occurs nowhere, `ambiguous` when
//!   several places remain, `conflict`), and what found it: `pattern`, `call` or `order`.
//! - `<out>/conflicts.tsv`: `address`, `name`, `reason` for each row in conflict.
//! - `<out>/symbols/`: the symbol tables with each found function row at its new address and the
//!   rows not found left out. Structs and fields carry no address and are copied. Globals have
//!   no signature, so that table is empty. `cargo xtask export <dir> --symbols <out>/symbols`
//!   writes the export scripts' inputs from it.
//!
//! A row is found in three steps, each building on the last:
//!
//! 1. Pattern: its pattern occurs once and the row is `unique` in 5875.
//! 2. Calls: a found function whose whole `body` matches where it was found has, at the offsets
//!    its 5875 body has them, a `call` or `jmp` whose 5875 target is row X; X is at the address
//!    that instruction goes to, when X's pattern matches there and has `MIN_FIXED` fixed bytes.
//!    Repeated until nothing new is found.
//! 3. Order: the found rows whose addresses rise with 5875's bound the rows between them. A row
//!    with `MIN_FIXED` fixed bytes and exactly one address its pattern matches between its
//!    bounds, outside the 5875 extent of every found row, is found there; so are several rows
//!    between two bounds that have exactly one way to take rising addresses. Rows in a gap with
//!    equal patterns are left out. This alternates with step 2 until nothing new is found.
//!
//! Nothing is guessed. A row still with several candidates is `ambiguous`. A conflict is
//! reported and its rows are left out, with whatever was found from them: two rows claiming one
//! address; one row claimed at two addresses; a claimed address its pattern refutes, or that
//! differs from where its pattern found it; two rows in the same gap taking one address or
//! falling out of order. Steps 2 and 3 take a found row's body to be the build's at the same
//! offsets, so a changed body can send a call elsewhere; the body check of step 2, the callee's
//! pattern and the conflicts are the guard. `call` and `order` are weaker than `pattern`.
//!
//! The build is any PE; its executable sections are searched, and nothing else of it is read.

use crate::export::Table;
use crate::signature::{self, Pe, Scanner, Signature};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Found {
    At(u32),
    Missing,
    Ambiguous,
    /// Two rows claim one address, or a claimed address contradicts the row's pattern or its
    /// other claims. Reported, not resolved.
    Conflict,
}

/// What found a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    /// Its pattern occurs once.
    Pattern,
    /// A found function calls or jumps to it, and its pattern matches there.
    Call,
    /// It is the only candidate between the addresses of its found neighbours.
    Order,
}

/// How far the propagation goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Unique patterns only.
    Pattern,
    /// And the targets of the calls of found functions, to a fixed point.
    Calls,
    /// And the order of the neighbours, alternating with the calls to a fixed point.
    Order,
}

/// How many rows each outcome took.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub rows: usize,
    pub found: usize,
    pub missing: usize,
    pub ambiguous: usize,
    pub conflict: usize,
    /// Rows whose signature is unique in 5875.
    pub unique_rows: usize,
    /// Of those, the rows found.
    pub unique_found: usize,
    pub by_pattern: usize,
    pub by_call: usize,
    pub by_order: usize,
}

impl Counts {
    pub fn summary(&self) -> String {
        format!(
            "{} rows: {} found ({} by pattern, {} by call, {} by order), {} missing, {} \
             ambiguous, {} in conflict; {} of {} rows with a unique signature found",
            self.rows,
            self.found,
            self.by_pattern,
            self.by_call,
            self.by_order,
            self.missing,
            self.ambiguous,
            self.conflict,
            self.unique_found,
            self.unique_rows
        )
    }
}

/// The result of a match, a row at a time in the order of the signatures.
#[derive(Debug)]
pub struct Outcome {
    pub found: Vec<Found>,
    pub via: Vec<Option<Via>>,
    /// Why each conflicting row is one, by row.
    pub conflicts: BTreeMap<usize, String>,
}

/// The most addresses a pattern may match for the order step to weigh them.
const CANDIDATES: usize = 1 << 20;

/// A build and the signatures to find in it.
pub struct Matcher<'a, 'b> {
    pe: &'b Pe<'a>,
    scanner: Scanner<'a>,
    signatures: &'b [Signature],
    row_of: HashMap<u32, usize>,
    unique: Vec<Found>,
    hits: HashMap<&'b signature::Pattern, Option<Vec<u32>>>,
}

type Claims = BTreeMap<usize, String>;
/// Unfound rows in a gap, each with the addresses its pattern matches there.
type Gap = Vec<(usize, Vec<u32>)>;
/// The files a match writes by path, and its counts after each stage.
pub type Built = (BTreeMap<String, String>, Vec<(Stage, Counts)>);

impl<'a, 'b> Matcher<'a, 'b> {
    pub fn new(pe: &'b Pe<'a>, signatures: &'b [Signature]) -> Matcher<'a, 'b> {
        let scanner = Scanner::new(pe);
        let mut cache: HashMap<&signature::Pattern, Found> = HashMap::new();
        let unique = signatures
            .iter()
            .map(|s| {
                let found = *cache.entry(&s.pattern).or_insert_with(|| {
                    match scanner.find(&s.pattern, 2)[..] {
                        [] => Found::Missing,
                        [at] => Found::At(at),
                        _ => Found::Ambiguous,
                    }
                });
                // A pattern that is not unique in 5875 is no evidence by itself, however
                // few places it matches in the build.
                match (s.unique, found) {
                    (false, Found::At(_)) => Found::Ambiguous,
                    _ => found,
                }
            })
            .collect();
        Matcher {
            pe,
            scanner,
            signatures,
            row_of: signatures
                .iter()
                .enumerate()
                .map(|(i, s)| (s.address, i))
                .collect(),
            unique,
            hits: HashMap::new(),
        }
    }

    /// Every row's outcome up to `stage`.
    pub fn locate(&mut self, stage: Stage) -> Outcome {
        let mut poisoned = self.shared_addresses();
        loop {
            match self.run(stage, &poisoned) {
                Ok(outcome) => return outcome,
                Err(more) => {
                    assert!(more.keys().all(|r| !poisoned.contains_key(r)));
                    poisoned.extend(more);
                }
            }
        }
    }

    /// Rows whose unique patterns land on one address.
    fn shared_addresses(&self) -> Claims {
        let mut at: HashMap<u32, Vec<usize>> = HashMap::new();
        for (i, f) in self.unique.iter().enumerate() {
            if let Found::At(a) = f {
                at.entry(*a).or_default().push(i);
            }
        }
        let mut out = Claims::new();
        for (a, rows) in at {
            if rows.len() > 1 {
                for &r in &rows {
                    out.insert(
                        r,
                        format!(
                            "its pattern and {} others match only at {a:#08x}",
                            rows.len() - 1
                        ),
                    );
                }
            }
        }
        out
    }

    /// Where the pattern of a row matches, `None` when too many places to weigh.
    fn candidates(&mut self, row: usize) -> Option<&Vec<u32>> {
        let pattern = &self.signatures[row].pattern;
        let scanner = &self.scanner;
        self.hits
            .entry(pattern)
            .or_insert_with(|| {
                let all = scanner.find(pattern, CANDIDATES);
                (all.len() < CANDIDATES).then_some(all)
            })
            .as_ref()
    }

    /// Whether the body of `row` through its last call site matches at `address`: the call
    /// offsets of a row are read only where its bytes are the ones they were taken from.
    fn body_matches(&self, row: usize, address: u32) -> bool {
        let sig = &self.signatures[row];
        sig.body.is_empty() || self.scanner.matches_at(&sig.body, address)
    }

    /// Whether the pattern of `row` has enough fixed bytes to place it by a call or by order.
    fn firm(&self, row: usize) -> bool {
        signature::fixed(&self.signatures[row].pattern) >= signature::MIN_FIXED
    }

    /// Where the call at `site` of the function at `at` goes in the build, if its bytes there
    /// are a rel32 call, jump or conditional jump.
    fn target(&self, at: u32, site: u32) -> Option<u32> {
        let start = at.checked_add(site)?;
        let code = self.pe.read(start, 6).or_else(|| self.pe.read(start, 5))?;
        let r = signature::rel32_at(code)?;
        let rel = i32::from_le_bytes(code[r..r + 4].try_into().ok()?);
        Some(start.wrapping_add(r as u32 + 4).wrapping_add(rel as u32))
    }

    /// One pass of the propagation from the unique patterns, leaving `poisoned` rows out. `Err`
    /// holds the rows that are found to be in conflict, which the caller leaves out and runs
    /// again.
    fn run(&mut self, stage: Stage, poisoned: &Claims) -> Result<Outcome, Claims> {
        let n = self.signatures.len();
        let mut at: Vec<Option<u32>> = vec![None; n];
        let mut via: Vec<Option<Via>> = vec![None; n];
        let mut taken: HashMap<u32, usize> = HashMap::new();
        for (i, f) in self.unique.iter().enumerate() {
            if let (Found::At(a), false) = (f, poisoned.contains_key(&i)) {
                at[i] = Some(*a);
                via[i] = Some(Via::Pattern);
                taken.insert(*a, i);
            }
        }
        if stage >= Stage::Calls {
            loop {
                let mut changed = self.calls(&mut at, &mut via, &mut taken, poisoned)?;
                if stage >= Stage::Order {
                    changed |= self.order(&mut at, &mut via, &mut taken, poisoned)?;
                }
                if !changed {
                    break;
                }
            }
        }
        let mut found = Vec::with_capacity(n);
        for (i, a) in at.iter().enumerate() {
            found.push(if poisoned.contains_key(&i) {
                Found::Conflict
            } else if let Some(a) = *a {
                Found::At(a)
            } else {
                match self.unique[i] {
                    Found::At(_) | Found::Conflict => Found::Ambiguous,
                    other => other,
                }
            });
        }
        Ok(Outcome {
            found,
            via,
            conflicts: poisoned.clone(),
        })
    }

    /// The call targets of the found rows, to a fixed point; whether any row was added.
    fn calls(
        &mut self,
        at: &mut [Option<u32>],
        via: &mut [Option<Via>],
        taken: &mut HashMap<u32, usize>,
        poisoned: &Claims,
    ) -> Result<bool, Claims> {
        let mut any = false;
        loop {
            // Each row's claimed addresses, with the row that makes the claim.
            let mut claims: BTreeMap<usize, BTreeMap<u32, usize>> = BTreeMap::new();
            for (caller, a) in at.iter().enumerate() {
                let Some(a) = *a else { continue };
                if self.signatures[caller].calls.is_empty() || !self.body_matches(caller, a) {
                    continue;
                }
                for &(site, old) in &self.signatures[caller].calls {
                    let Some(&x) = self.row_of.get(&old) else {
                        continue;
                    };
                    if poisoned.contains_key(&x) || !self.firm(x) {
                        continue;
                    }
                    if let Some(t) = self.target(a, site) {
                        claims.entry(x).or_default().entry(t).or_insert(caller);
                    }
                }
            }
            let mut bad = Claims::new();
            let mut by_address: HashMap<u32, Vec<usize>> = HashMap::new();
            for (&x, targets) in &claims {
                let wrong = if targets.len() > 1 {
                    Some("the found functions that call it send it to different addresses")
                } else {
                    let (&t, _) = targets.iter().next().expect("one target");
                    if at[x].is_some_and(|a| a != t) {
                        Some("a found function calls it at an address other than where it is found")
                    } else if at[x].is_none()
                        && !self.scanner.matches_at(&self.signatures[x].pattern, t)
                    {
                        Some("a found function calls it at an address its pattern does not match")
                    } else {
                        by_address.entry(t).or_default().push(x);
                        None
                    }
                };
                if let Some(w) = wrong {
                    bad.insert(x, w.to_string());
                }
            }
            for (t, rows) in by_address {
                let holder = taken.get(&t).copied();
                let all: Vec<usize> = rows
                    .iter()
                    .copied()
                    .chain(holder.filter(|h| !rows.contains(h)))
                    .collect();
                if all.len() > 1 {
                    for &r in &all {
                        bad.entry(r).or_insert_with(|| {
                            format!("{t:#08x} is claimed for {} rows", all.len())
                        });
                    }
                }
            }
            if !bad.is_empty() {
                return Err(bad);
            }
            let mut added = false;
            for (x, targets) in claims {
                let (&t, _) = targets.iter().next().expect("one target");
                if at[x].is_none() {
                    at[x] = Some(t);
                    via[x] = Some(Via::Call);
                    taken.insert(t, x);
                    added = true;
                }
            }
            if !added {
                return Ok(any);
            }
            any = true;
        }
    }

    /// The found rows whose addresses rise with 5875's, as bounds; a row off that run moved,
    /// and bounds nothing. Each gap between two bounds, with the unfound rows in it, in order,
    /// each with the addresses its pattern matches there.
    fn gaps(
        &mut self,
        at: &[Option<u32>],
        taken: &HashMap<u32, usize>,
        poisoned: &Claims,
    ) -> Vec<(bool, Gap)> {
        let found: Vec<(usize, u32)> = at
            .iter()
            .enumerate()
            .filter_map(|(i, a)| a.map(|a| (i, a)))
            .collect();
        let anchors = rising(&found);
        // The extent each found row holds, by its address.
        let extents: BTreeMap<u32, u32> = found
            .iter()
            .map(|&(i, a)| (a, a.saturating_add(self.signatures[i].size)))
            .collect();
        let inside = |c: u32| {
            extents
                .range(..c)
                .next_back()
                .is_some_and(|(_, &end)| c < end)
        };
        let mut out = Vec::new();
        for gap in 1..anchors.len() {
            let (low, high) = (anchors[gap - 1], anchors[gap]);
            let mut rows: Gap = Vec::new();
            let mut open = true;
            for (row, placed) in at.iter().enumerate().take(high.0).skip(low.0 + 1) {
                if placed.is_some()
                    || poisoned.contains_key(&row)
                    || self.unique[row] == Found::Missing
                    || !self.firm(row)
                {
                    continue;
                }
                match self.candidates(row) {
                    Some(all) => rows.push((
                        row,
                        all[all.partition_point(|&c| c <= low.1)..]
                            .iter()
                            .copied()
                            .take_while(|&c| c < high.1)
                            .filter(|&c| !taken.contains_key(&c) && !inside(c))
                            .collect(),
                    )),
                    None => open = false,
                }
            }
            // Rows with one pattern cannot be told apart by their bytes, so order alone does
            // not place them.
            let patterns = rows
                .iter()
                .map(|(row, _)| &self.signatures[*row].pattern)
                .fold(HashMap::<&signature::Pattern, usize>::new(), |mut m, p| {
                    *m.entry(p).or_default() += 1;
                    m
                });
            rows.retain(|(row, _)| patterns[&self.signatures[*row].pattern] == 1);
            if !rows.is_empty() {
                out.push((open, rows));
            }
        }
        out
    }

    /// Rows with one address to be at between their found neighbours, then rows that have one
    /// way to take rising addresses; whether any row was added, or the rows in conflict.
    ///
    /// A row with exactly one address between its neighbours is found there. Two such rows
    /// between the same neighbours that take one address or fall out of order are in conflict.
    /// After that, the unfound rows between two neighbours each take one of the addresses their
    /// patterns match there, rising; when exactly one way exists they are found. Two or more
    /// ways, or none, leave them ambiguous.
    fn order(
        &mut self,
        at: &mut [Option<u32>],
        via: &mut [Option<Via>],
        taken: &mut HashMap<u32, usize>,
        poisoned: &Claims,
    ) -> Result<bool, Claims> {
        let mut added = false;
        let mut bad = Claims::new();
        let mut singles: Vec<(usize, u32)> = Vec::new();
        for (_, rows) in self.gaps(at, taken, poisoned) {
            let mut previous: Option<(usize, u32)> = None;
            for (row, list) in &rows {
                if let [c] = list[..] {
                    if let Some((p, _)) = previous.filter(|&(_, pc)| pc >= c) {
                        let why = "it and a row before it, alone in the same gap, take one \
                                   address or fall out of order";
                        bad.insert(*row, why.to_string());
                        bad.insert(p, why.to_string());
                    }
                    previous = Some((*row, c));
                    singles.push((*row, c));
                }
            }
        }
        if !bad.is_empty() {
            return Err(bad);
        }
        for (row, c) in singles {
            at[row] = Some(c);
            via[row] = Some(Via::Order);
            taken.insert(c, row);
            added = true;
        }
        for (open, mut rows) in self.gaps(at, taken, poisoned) {
            if !open {
                continue;
            }
            rows.retain(|(_, list)| !list.is_empty());
            if let Some(chosen) = only_rising(&rows) {
                for ((row, _), c) in rows.iter().zip(chosen) {
                    at[*row] = Some(c);
                    via[*row] = Some(Via::Order);
                    taken.insert(c, *row);
                    added = true;
                }
            }
        }
        Ok(added)
    }
}

/// The one way to pick an address from each list so that the picks rise, if there is exactly one.
fn only_rising(rows: &[(usize, Vec<u32>)]) -> Option<Vec<u32>> {
    // ways[i][j]: the ways, at most 2, for rows 0..=i to rise ending with row i at its j-th.
    let mut ways: Vec<Vec<u8>> = Vec::with_capacity(rows.len());
    for (i, (_, list)) in rows.iter().enumerate() {
        let mut now = Vec::with_capacity(list.len());
        for &c in list {
            now.push(if i == 0 {
                1
            } else {
                let before = &rows[i - 1].1;
                let k = before.partition_point(|&b| b < c);
                ways[i - 1][..k].iter().fold(0u8, |a, &w| (a + w).min(2))
            });
        }
        ways.push(now);
    }
    let last = ways.last()?;
    if last.iter().fold(0u8, |a, &w| (a + w).min(2)) != 1 {
        return None;
    }
    let mut j = last.iter().position(|&w| w == 1)?;
    let mut chosen = vec![rows[rows.len() - 1].1[j]];
    for i in (0..rows.len() - 1).rev() {
        let c = chosen[chosen.len() - 1];
        let k = rows[i].1.partition_point(|&b| b < c);
        j = ways[i][..k].iter().position(|&w| w >= 1)?;
        chosen.push(rows[i].1[j]);
    }
    chosen.reverse();
    Some(chosen)
}

/// The longest run of `found` (row, address) whose addresses rise, as a subsequence.
fn rising(found: &[(usize, u32)]) -> Vec<(usize, u32)> {
    // tails[k]: the index in `found` of the smallest address ending a run of k + 1.
    let mut tails: Vec<usize> = Vec::new();
    let mut previous: Vec<Option<usize>> = vec![None; found.len()];
    for (i, &(_, a)) in found.iter().enumerate() {
        let k = tails.partition_point(|&t| found[t].1 < a);
        previous[i] = k.checked_sub(1).map(|k| tails[k]);
        if k == tails.len() {
            tails.push(i);
        } else {
            tails[k] = i;
        }
    }
    let mut out = Vec::new();
    let mut at = tails.last().copied();
    while let Some(i) = at {
        out.push(found[i]);
        at = previous[i];
    }
    out.reverse();
    out
}

pub fn count(signatures: &[Signature], outcome: &Outcome) -> Counts {
    let mut c = Counts {
        rows: signatures.len(),
        ..Counts::default()
    };
    for (i, s) in signatures.iter().enumerate() {
        match outcome.found[i] {
            Found::At(_) => {
                c.found += 1;
                match outcome.via[i] {
                    Some(Via::Pattern) => c.by_pattern += 1,
                    Some(Via::Call) => c.by_call += 1,
                    Some(Via::Order) => c.by_order += 1,
                    None => {}
                }
            }
            Found::Missing => c.missing += 1,
            Found::Ambiguous => c.ambiguous += 1,
            Found::Conflict => c.conflict += 1,
        }
        if s.unique {
            c.unique_rows += 1;
            if matches!(outcome.found[i], Found::At(_)) {
                c.unique_found += 1;
            }
        }
    }
    c
}

/// What the match writes, by path under the output directory.
pub fn build(
    pe: &Pe,
    signatures: &[Signature],
    symbols: &[(String, String)],
) -> Result<Built, String> {
    let mut matcher = Matcher::new(pe, signatures);
    let mut stages = Vec::new();
    let mut last = None;
    for stage in [Stage::Pattern, Stage::Calls, Stage::Order] {
        let outcome = matcher.locate(stage);
        stages.push((stage, count(signatures, &outcome)));
        last = Some(outcome);
    }
    let outcome = last.expect("three stages");
    let by_address: HashMap<u32, usize> = signatures
        .iter()
        .enumerate()
        .map(|(i, s)| (s.address, i))
        .collect();
    let mut files = BTreeMap::new();
    let mut report = String::from("address\tname\tfound\tvia\n");
    let mut conflicts = String::from("address\tname\treason\n");
    for (name, text) in symbols {
        let table = Table::parse(name, text)?;
        let mut out = table.columns.join("\t");
        out.push('\n');
        if name == "functions" {
            let a = table.column("address")?;
            let n = table.column("name")?;
            let mut moved: Vec<(u32, String)> = Vec::new();
            for row in &table.rows {
                let old = u32::from_str_radix(row[a].trim_start_matches("0x"), 16)
                    .map_err(|_| format!("functions: bad address {}", row[a]))?;
                let Some(&i) = by_address.get(&old) else {
                    continue;
                };
                let f = outcome.found[i];
                let shown = match f {
                    Found::At(at) => format!("{at:#08x}"),
                    Found::Missing => "missing".to_string(),
                    Found::Ambiguous => "ambiguous".to_string(),
                    Found::Conflict => "conflict".to_string(),
                };
                let how = match outcome.via[i].filter(|_| matches!(f, Found::At(_))) {
                    Some(Via::Pattern) => "pattern",
                    Some(Via::Call) => "call",
                    Some(Via::Order) => "order",
                    None => "",
                };
                let _ = writeln!(report, "{}\t{}\t{shown}\t{how}", row[a], row[n]);
                if let Some(why) = outcome.conflicts.get(&i) {
                    let _ = writeln!(conflicts, "{}\t{}\t{why}", row[a], row[n]);
                }
                if let Found::At(at) = f {
                    let mut row = row.clone();
                    row[a] = format!("{at:#08x}");
                    moved.push((at, row.join("\t")));
                }
            }
            moved.sort();
            for (_, line) in &moved {
                out.push_str(line);
                out.push('\n');
            }
        } else if name != "globals" {
            out = text.clone();
        }
        files.insert(format!("symbols/{name}.tsv"), out);
    }
    files.insert("match.tsv".to_string(), report);
    files.insert("conflicts.tsv".to_string(), conflicts);
    Ok((files, stages))
}

/// Reads `tables/signatures.tsv` and the symbol tables of `root`, matches them against the exe at
/// `exe`, writes under `out`, and prints the counts.
pub fn run(root: &Path, exe: &Path, out: &Path) -> std::io::Result<bool> {
    let io = std::io::Error::other;
    let bytes = fs::read(exe)?;
    let pe = Pe::parse(&bytes).map_err(|e| io(format!("{}: {e}", exe.display())))?;
    let signatures =
        signature::parse(&fs::read_to_string(root.join("tables/signatures.tsv"))?).map_err(io)?;
    let mut symbols = Vec::new();
    for name in crate::export::SYMBOLS {
        symbols.push((
            name.to_string(),
            fs::read_to_string(root.join("symbols").join(format!("{name}.tsv")))?,
        ));
    }
    let (files, stages) = build(&pe, &signatures, &symbols).map_err(io)?;
    for (path, text) in &files {
        let target = out.join(path);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(target, text)?;
    }
    for (stage, counts) in &stages {
        println!("{stage:?}: {}", counts.summary());
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bodies::Listing;
    use crate::exe;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask sits in the repo")
            .to_path_buf()
    }

    /// The reader's exe, or `None` to skip; with `BENILLA_RE_REQUIRE_EXE` set, a missing one
    /// panics.
    fn reference() -> Option<Vec<u8>> {
        let path = exe::locate(&root());
        match exe::identify(&path) {
            Ok(exe::Identity::Reference) => Some(std::fs::read(&path).expect("the exe reads")),
            _ => {
                assert!(
                    std::env::var_os(exe::REQUIRE_VARIABLE).is_none(),
                    "{} is not WoW.exe 1.12.1.5875",
                    path.display()
                );
                println!("skipped: no WoW.exe 1.12.1.5875");
                None
            }
        }
    }

    fn committed() -> Vec<Signature> {
        signature::parse(&std::fs::read_to_string(root().join("tables/signatures.tsv")).unwrap())
            .expect("the table parses")
    }

    fn symbols() -> Vec<(String, String)> {
        crate::export::SYMBOLS
            .iter()
            .map(|n| {
                (
                    n.to_string(),
                    std::fs::read_to_string(root().join(format!("symbols/{n}.tsv"))).unwrap(),
                )
            })
            .collect()
    }

    /// The file offset of a `.text` address: the section's raw data starts at its address.
    fn at(address: u32) -> usize {
        (address - 0x400000) as usize
    }

    #[test]
    fn every_unique_signature_finds_its_own_row_in_5875() {
        let Some(bytes) = reference() else { return };
        let pe = Pe::parse(&bytes).unwrap();
        let signatures = committed();
        let mut matcher = Matcher::new(&pe, &signatures);
        let found = matcher.locate(Stage::Pattern);
        let counts = count(&signatures, &found);
        assert_eq!(counts.unique_found, counts.unique_rows);
        assert!(counts.unique_rows > 10_000);
        assert_eq!(counts.missing, 0);
        assert_eq!(counts.conflict, 0);
        for (s, f) in signatures.iter().zip(&found.found) {
            if s.unique {
                assert_eq!(*f, Found::At(s.address), "{:#08x}", s.address);
            } else {
                assert_eq!(*f, Found::Ambiguous, "{:#08x}", s.address);
            }
        }
    }

    /// A tiny build: 0x400 bytes of `int3` at 0x1000 holding the bodies put in it.
    struct Image(Vec<u8>);

    impl Image {
        fn new() -> Image {
            Image(vec![0xcc; 0x400])
        }

        fn put(&mut self, address: u32, bytes: &[u8]) {
            let at = (address - 0x1000) as usize;
            self.0[at..at + bytes.len()].copy_from_slice(bytes);
        }

        /// `push ebp; mov ebp, esp; call target; ret; tag` at `address`; the call is at offset 3.
        fn caller(&mut self, address: u32, target: u32, tag: u8) {
            let rel = target.wrapping_sub(address + 3 + 5);
            let mut body = vec![0x55, 0x8b, 0xec, 0xe8];
            body.extend_from_slice(&rel.to_le_bytes());
            body.extend_from_slice(&[0xc3, tag]);
            self.put(address, &body);
        }

        fn pe(&self) -> Pe<'_> {
            Pe {
                base: 0x1000,
                image_size: 0x400,
                sections: vec![crate::signature::Section {
                    va: 0x1000,
                    flags: 0x2000_0000,
                    data: &self.0,
                }],
            }
        }
    }

    const TWIN: [u8; 6] = [0x55, 0x8b, 0xec, 0x33, 0xc0, 0xc3];

    fn sig(address: u32, unique: bool, pattern: &str, calls: &[(u32, u32)]) -> Signature {
        Signature {
            address,
            unique,
            pattern: signature::read_pattern(pattern).unwrap(),
            size: pattern.split(' ').count() as u32,
            body: Vec::new(),
            calls: calls.to_vec(),
        }
    }

    /// Rows in 5875 order: a caller of B, B and C (same bytes), and a unique row after them.
    fn rows() -> Vec<Signature> {
        vec![
            sig(
                0x401000,
                true,
                "55 8b ec e8 ?? ?? ?? ?? c3 00",
                &[(3, 0x402000)],
            ),
            sig(0x402000, false, "55 8b ec 33 c0 c3", &[]),
            sig(0x402010, false, "55 8b ec 33 c0 c3", &[]),
            sig(0x403000, true, "55 8b ec 83 ec 10 c3", &[]),
        ]
    }

    fn image() -> Image {
        let mut image = Image::new();
        image.caller(0x1000, 0x1040, 0);
        image.put(0x1040, &TWIN);
        image.put(0x1060, &TWIN);
        image.put(0x1080, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x10, 0xc3]);
        image
    }

    #[test]
    fn a_call_target_is_found_where_its_pattern_matches() {
        let image = image();
        let pe = image.pe();
        let signatures = rows();
        let mut matcher = Matcher::new(&pe, &signatures);
        let by_pattern = matcher.locate(Stage::Pattern);
        assert_eq!(
            by_pattern.found,
            [
                Found::At(0x1000),
                Found::Ambiguous,
                Found::Ambiguous,
                Found::At(0x1080)
            ]
        );
        let by_call = matcher.locate(Stage::Calls);
        assert_eq!(by_call.found[1], Found::At(0x1040));
        assert_eq!(by_call.via[1], Some(Via::Call));
        // Nothing calls C, and calls alone do not place it.
        assert_eq!(by_call.found[2], Found::Ambiguous);
    }

    #[test]
    fn a_row_with_one_address_between_its_neighbours_is_found_there() {
        let image = image();
        let pe = image.pe();
        let signatures = rows();
        let by_order = Matcher::new(&pe, &signatures).locate(Stage::Order);
        assert_eq!(by_order.found[2], Found::At(0x1060));
        assert_eq!(by_order.via[2], Some(Via::Order));
        assert!(by_order.conflicts.is_empty());
        // Without the call step, B and C are two rows with one pattern in one gap: their bytes
        // do not tell them apart, so order alone places neither.
        let mut only = signatures.clone();
        only[0].calls.clear();
        let found = Matcher::new(&pe, &only).locate(Stage::Order).found;
        assert_eq!(found[1], Found::Ambiguous);
        assert_eq!(found[2], Found::Ambiguous);
    }

    #[test]
    fn order_refuses_a_row_with_two_addresses_between_its_neighbours() {
        let mut image = image();
        // A copy of the twin bodies' bytes between B and the next row, and no caller of B.
        image.put(0x1070, &TWIN);
        let pe = image.pe();
        let mut signatures = rows();
        signatures.remove(1);
        signatures[0].calls.clear();
        // C alone: 0x1040, 0x1060 and 0x1070 all match, the neighbours are 0x1000 and 0x1080.
        let found = Matcher::new(&pe, &signatures).locate(Stage::Order).found;
        assert_eq!(
            found,
            [Found::At(0x1000), Found::Ambiguous, Found::At(0x1080)]
        );
    }

    #[test]
    fn order_refuses_neighbours_that_are_out_of_order() {
        // The unique rows swap places in the build: no pair bounds a gap.
        let mut image = Image::new();
        image.put(0x1000, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x10, 0xc3]);
        image.put(0x1040, &TWIN);
        image.put(0x1060, &TWIN);
        image.caller(0x1080, 0x1040, 0);
        let pe = image.pe();
        let signatures = rows();
        let found = Matcher::new(&pe, &signatures).locate(Stage::Order).found;
        assert_eq!(found[0], Found::At(0x1080));
        assert_eq!(found[3], Found::At(0x1000));
        // Only one of the two is a bound; B comes from the call, never from a wrong window.
        assert_eq!(found[1], Found::At(0x1040));
        assert_ne!(found[2], Found::At(0x1040));
    }

    #[test]
    fn a_call_to_where_the_pattern_does_not_match_is_a_conflict() {
        let mut image = image();
        // The call goes to the unique row's body, which B's pattern does not match.
        image.caller(0x1000, 0x1080, 0);
        let pe = image.pe();
        let signatures = rows();
        let mut matcher = Matcher::new(&pe, &signatures);
        let found = matcher.locate(Stage::Order);
        assert_eq!(found.found[1], Found::Conflict);
        assert!(found.conflicts[&1].contains("pattern does not match"));
        // C is not placed from B's claim; the order step may still place it: it has the one
        // address left between the neighbours that are found.
        assert_eq!(found.found[0], Found::At(0x1000));
        assert_eq!(found.found[3], Found::At(0x1080));
    }

    #[test]
    fn two_rows_claiming_one_address_are_both_in_conflict() {
        let mut image = Image::new();
        image.caller(0x1000, 0x1040, 0);
        image.caller(0x1020, 0x1040, 1);
        image.put(0x1040, &TWIN);
        image.put(0x1060, &TWIN);
        let pe = image.pe();
        let signatures = vec![
            sig(
                0x401000,
                true,
                "55 8b ec e8 ?? ?? ?? ?? c3 00",
                &[(3, 0x402000)],
            ),
            sig(
                0x401020,
                true,
                "55 8b ec e8 ?? ?? ?? ?? c3 01",
                &[(3, 0x402010)],
            ),
            sig(0x402000, false, "55 8b ec 33 c0 c3", &[]),
            sig(0x402010, false, "55 8b ec 33 c0 c3", &[]),
        ];
        let found = Matcher::new(&pe, &signatures).locate(Stage::Calls);
        assert_eq!(found.found[2], Found::Conflict);
        assert_eq!(found.found[3], Found::Conflict);
        assert_eq!(found.conflicts.len(), 2);
    }

    #[test]
    fn callers_that_send_a_row_to_two_addresses_put_it_in_conflict() {
        let mut image = Image::new();
        image.caller(0x1000, 0x1040, 0);
        image.caller(0x1020, 0x1060, 1);
        image.put(0x1040, &TWIN);
        image.put(0x1060, &TWIN);
        let pe2 = image.pe();
        let signatures = vec![
            sig(
                0x401000,
                true,
                "55 8b ec e8 ?? ?? ?? ?? c3 00",
                &[(3, 0x402000)],
            ),
            sig(
                0x401020,
                true,
                "55 8b ec e8 ?? ?? ?? ?? c3 01",
                &[(3, 0x402000)],
            ),
            sig(0x402000, false, "55 8b ec 33 c0 c3", &[]),
        ];
        let found = Matcher::new(&pe2, &signatures).locate(Stage::Calls);
        assert_eq!(found.found[2], Found::Conflict);
        assert!(found.conflicts[&2].contains("different addresses"));
        // The callers themselves stay found.
        assert_eq!(found.found[0], Found::At(0x1000));
        assert_eq!(found.found[1], Found::At(0x1020));
    }

    #[test]
    fn the_longest_rising_run_leaves_out_a_moved_row() {
        let rows: Vec<(usize, u32)> = vec![(0, 10), (1, 50), (2, 20), (3, 30), (4, 5), (5, 40)];
        let run: Vec<usize> = rising(&rows).iter().map(|r| r.0).collect();
        assert_eq!(run, [0, 2, 3, 5]);
        assert!(rising(&[]).is_empty());
    }

    #[test]
    fn rising_picks_are_found_only_when_there_is_one_way() {
        let rows = |a: &[&[u32]]| -> Vec<(usize, Vec<u32>)> {
            a.iter().enumerate().map(|(i, l)| (i, l.to_vec())).collect()
        };
        assert_eq!(only_rising(&rows(&[&[5, 9], &[7, 9]])), None);
        assert_eq!(only_rising(&rows(&[&[5, 9], &[9, 12]])), None);
        assert_eq!(only_rising(&rows(&[&[5, 9], &[5, 9]])), Some(vec![5, 9]));
        assert_eq!(only_rising(&rows(&[&[9, 5], &[5, 9]])), None);
        assert_eq!(only_rising(&rows(&[&[4], &[6]])), Some(vec![4, 6]));
        assert_eq!(only_rising(&rows(&[&[6], &[4]])), None);
    }

    #[test]
    fn propagation_on_5875_finds_rows_at_their_own_addresses_only() {
        let Some(bytes) = reference() else { return };
        let pe = Pe::parse(&bytes).unwrap();
        let signatures = committed();
        let mut matcher = Matcher::new(&pe, &signatures);
        let mut last = 0;
        for stage in [Stage::Pattern, Stage::Calls, Stage::Order] {
            let outcome = matcher.locate(stage);
            let counts = count(&signatures, &outcome);
            println!("{stage:?}: {}", counts.summary());
            for (s, f) in signatures.iter().zip(&outcome.found) {
                if let Found::At(a) = f {
                    assert_eq!(*a, s.address, "{stage:?}: {:#08x}", s.address);
                }
            }
            assert_eq!(counts.conflict, 0, "{:?}", outcome.conflicts);
            assert!(counts.found >= last);
            last = counts.found;
        }
    }

    #[test]
    fn a_patched_swapped_and_copied_build_shows_the_expected_counts() {
        let Some(mut bytes) = reference() else { return };
        let signatures = committed();
        let sizes: HashMap<u32, u32> = {
            let functions = symbols();
            let t = Table::parse("functions", &functions[0].1).unwrap();
            t.rows
                .iter()
                .map(|r| {
                    (
                        u32::from_str_radix(&r[0][2..], 16).unwrap(),
                        r[1].parse().unwrap(),
                    )
                })
                .collect()
        };
        let before = {
            let pe = Pe::parse(&bytes).unwrap();
            assert_eq!(pe.sections[0].va, 0x401000);
            count(
                &signatures,
                &Matcher::new(&pe, &signatures).locate(Stage::Pattern),
            )
        };
        // Long unique rows that no other row's patch touches.
        let long: Vec<&Signature> = signatures
            .iter()
            .filter(|s| s.unique && s.pattern.len() >= 24 && sizes[&s.address] >= 48)
            .collect();
        let pick = |n: usize| long[n * long.len() / 7];
        let (patched, swapped_a, swapped_b, copied, overwritten) =
            (pick(1), pick(2), pick(3), pick(4), pick(5));
        // Swapped bodies must have the same length.
        let twin = long
            .iter()
            .find(|s| {
                s.address != swapped_a.address && sizes[&s.address] == sizes[&swapped_a.address]
            })
            .copied()
            .unwrap_or(swapped_b);
        assert_eq!(sizes[&twin.address], sizes[&swapped_a.address]);

        // Patched: a fixed byte of the pattern changes, so the row is not found.
        let fixed = patched.pattern.iter().position(Option::is_some).unwrap() + 3;
        let p = at(patched.address) + fixed;
        bytes[p] ^= 0xff;
        // Swapped: two bodies exchange places, so each row is found at the other's address.
        let size = sizes[&swapped_a.address] as usize;
        let (x, y) = (at(swapped_a.address), at(twin.address));
        for i in 0..size {
            bytes.swap(x + i, y + i);
        }
        // Copied: one row's pattern bytes overwrite the start of another row's body, so the first
        // occurs twice and the second is not found.
        let source: Vec<u8> = bytes[at(copied.address)..][..copied.pattern.len()].to_vec();
        let d = at(overwritten.address);
        bytes[d..d + source.len()].copy_from_slice(&source);

        let pe = Pe::parse(&bytes).unwrap();
        let found = Matcher::new(&pe, &signatures).locate(Stage::Pattern).found;
        let after = count(
            &signatures,
            &Matcher::new(&pe, &signatures).locate(Stage::Pattern),
        );
        let result = |s: &Signature| found[signatures.iter().position(|t| t == s).unwrap()];
        // Propagation puts a row at its own address, or in conflict, except the swapped pair,
        // whose patterns find each other's bodies.
        let propagated = Matcher::new(&pe, &signatures).locate(Stage::Order);
        for (s, f) in signatures.iter().zip(&propagated.found) {
            if let Found::At(a) = f {
                let swapped = s.address == swapped_a.address || s.address == twin.address;
                assert!(*a == s.address || swapped, "{:#08x} at {a:#08x}", s.address);
            }
        }
        assert!(!propagated.conflicts.is_empty());
        assert_eq!(result(patched), Found::Missing);
        assert_eq!(result(swapped_a), Found::At(twin.address));
        assert_eq!(result(twin), Found::At(swapped_a.address));
        assert_eq!(result(copied), Found::Ambiguous);
        assert_eq!(result(overwritten), Found::Missing);
        assert_eq!(after.missing, before.missing + 2);
        assert_eq!(after.ambiguous, before.ambiguous + 1);
        assert_eq!(after.found, before.found - 3);
        assert_eq!(after.rows, before.rows);

        // The rebased tables hold the swapped rows at their new addresses and no missing row.
        let (files, stages) = build(&pe, &signatures, &symbols()).unwrap();
        for (stage, counts) in &stages {
            println!("{stage:?}: {}", counts.summary());
        }
        let moved = Table::parse("functions", &files["symbols/functions.tsv"]).unwrap();
        let rows: HashMap<&str, &Vec<String>> =
            moved.rows.iter().map(|r| (r[0].as_str(), r)).collect();
        // The twin's row sits where the swapped row was. The swapped row is in conflict: a
        // caller of it sends it to an address its pattern refutes.
        assert!(rows.contains_key(format!("{:#08x}", swapped_a.address).as_str()));
        assert_eq!(moved.rows.len(), stages[2].1.found);
        let report = &files["match.tsv"];
        let line = |s: &Signature| {
            let start = format!("{:#08x}\t", s.address);
            report
                .lines()
                .find(|l| l.starts_with(&start))
                .unwrap()
                .split('\t')
                .nth(2)
                .unwrap()
                .to_string()
        };
        assert_eq!(line(patched), "missing");
        // The copy's pattern matches twice; only one of the two lies between its neighbours.
        assert_eq!(line(copied), format!("{:#08x}", copied.address));
        assert_eq!(line(swapped_a), "conflict");
        assert_eq!(line(twin), format!("{:#08x}", swapped_a.address));
    }

    #[test]
    fn the_rebased_tables_of_5875_import_with_the_export_scripts() {
        let Some(bytes) = reference() else { return };
        let pe = Pe::parse(&bytes).unwrap();
        let (files, stages) = build(&pe, &committed(), &symbols()).unwrap();
        let counts = &stages[0].1;
        assert_eq!(counts.unique_found, counts.unique_rows);
        let dir = std::env::temp_dir().join(format!("xtask-match-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (path, text) in &files {
            let target = dir.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, text).unwrap();
        }
        let sources = crate::export::read(&root(), Some(&dir.join("symbols"))).unwrap();
        let built = crate::export::build(&sources).unwrap();
        assert!(!built.files.contains_key("bundle/signature"));
        // Nothing that holds 5875's addresses or its exe identity goes out as the build's.
        assert!(built.files.keys().all(|p| !p.starts_with("bundle/tables/")));
        let bundle: serde_json::Value =
            serde_json::from_str(&built.files["bundle/benilla-re.json"]).unwrap();
        assert_eq!(bundle["exe"]["rebased"], true);
        assert!(bundle["exe"]["sha256"].is_null());
        assert!(bundle["tables"]
            .as_object()
            .unwrap()
            .keys()
            .all(|k| k.starts_with("symbols/")));
        assert!(!built.files["benilla-re.h"].contains("SHA-256"));
        assert!(built.files["benilla-re.h"].contains("rebased"));
        assert!(built.files["benilla-re.h"].contains("struct"));
        let out = dir.join("export");
        for (path, text) in &built.files {
            let target = out.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, text).unwrap();
        }
        if std::process::Command::new("python3")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
        {
            let run = std::process::Command::new("python3")
                .arg("-I")
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/export/scripts_test.py"))
                .arg(&out)
                .output()
                .unwrap();
            let said = String::from_utf8_lossy(&run.stdout);
            assert!(
                run.status.success() && said.contains("IDA ok") && said.contains("Binja ok"),
                "{said}{}",
                String::from_utf8_lossy(&run.stderr)
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_same_rows_write_the_same_signatures() {
        let Some(bytes) = reference() else { return };
        let listing_path = root().join("dumps/disasm.txt");
        if !listing_path.exists() {
            assert!(
                std::env::var_os(exe::REQUIRE_VARIABLE).is_none(),
                "no dumps/disasm.txt"
            );
            println!("skipped: no dumps/disasm.txt");
            return;
        }
        let listing = Listing::read(&listing_path).unwrap();
        let pe = Pe::parse(&bytes).unwrap();
        let committed = committed();
        // A spread of rows, twice: the output is the same and is the committed one.
        let rows: Vec<(u32, u32)> = {
            let t = Table::parse("functions", &symbols()[0].1).unwrap();
            t.rows
                .iter()
                .step_by(23)
                .map(|r| {
                    (
                        u32::from_str_radix(&r[0][2..], 16).unwrap(),
                        r[1].parse().unwrap(),
                    )
                })
                .collect()
        };
        let a = signature::generate(&pe, &listing, &rows).unwrap();
        let b = signature::generate(&pe, &listing, &rows).unwrap();
        assert_eq!(signature::table(&a), signature::table(&b));
        let sampled: std::collections::HashSet<u32> = rows.iter().map(|r| r.0).collect();
        for s in &a {
            let mut c = committed
                .iter()
                .find(|c| c.address == s.address)
                .unwrap()
                .clone();
            // The committed calls name every row; these name the sampled ones.
            let all = c.calls.len();
            c.calls.retain(|(_, t)| sampled.contains(t));
            if c.calls.len() != all {
                // The body runs through the last call site, which the sample moves.
                c.body = s.body.clone();
            }
            assert_eq!(s, &c, "{:#08x}", s.address);
        }
    }

    #[test]
    fn a_pattern_not_unique_in_5875_is_never_found_by_pattern() {
        let mut image = Image::new();
        image.put(0x1040, &TWIN);
        let pe = image.pe();
        let signatures = vec![sig(0x402000, false, "55 8b ec 33 c0 c3", &[])];
        let found = Matcher::new(&pe, &signatures).locate(Stage::Order);
        assert_eq!(found.found, [Found::Ambiguous]);
    }

    #[test]
    fn a_caller_whose_body_differs_does_not_place_its_callee() {
        let mut signatures = rows();
        // The pattern is the prefix; the body runs through the call and the byte after it.
        signatures[0].pattern = signature::read_pattern("55 8b ec e8").unwrap();
        signatures[0].body = signature::read_pattern("55 8b ec e8 ?? ?? ?? ?? c3 00").unwrap();
        let image = image();
        let found = Matcher::new(&image.pe(), &signatures).locate(Stage::Calls);
        assert_eq!(found.via[1], Some(Via::Call));
        // The same caller with another byte after the call: its call offsets are not read.
        let mut other = Image::new();
        other.caller(0x1000, 0x1040, 1);
        other.put(0x1040, &TWIN);
        other.put(0x1060, &TWIN);
        other.put(0x1080, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x10, 0xc3]);
        let found = Matcher::new(&other.pe(), &signatures).locate(Stage::Calls);
        assert_eq!(found.found[0], Found::At(0x1000));
        assert_eq!(found.found[1], Found::Ambiguous);
    }

    #[test]
    fn a_callee_with_few_fixed_bytes_is_not_placed_by_a_call_or_by_order() {
        let mut signatures = rows();
        signatures[1].pattern = signature::read_pattern("33 c0 c3").unwrap();
        signatures[2].pattern = signature::read_pattern("33 c0 c3").unwrap();
        signatures[3].pattern = signature::read_pattern("83 ec 10 c3").unwrap();
        let image = image();
        let found = Matcher::new(&image.pe(), &signatures).locate(Stage::Order);
        assert_eq!(found.found[1], Found::Ambiguous);
        assert_eq!(found.found[2], Found::Ambiguous);
    }

    #[test]
    fn a_candidate_inside_a_found_row_is_dropped() {
        let build = |size_of_first: u32| {
            let mut image = image();
            image.put(0x1020, &TWIN);
            // One row between the neighbours, with the body of the twins in three places.
            let mut signatures = rows();
            signatures.remove(2);
            signatures[0].calls.clear();
            signatures[0].size = size_of_first;
            image.put(0x1060, &[0xcc; 6]);
            let pe = image.pe();
            Matcher::new(&pe, &signatures).locate(Stage::Order).found[1]
        };
        assert_eq!(build(0x10), Found::Ambiguous);
        assert_eq!(build(0x30), Found::At(0x1040));
    }

    /// How a match of a synthetic build comes out against what the build holds.
    #[derive(Debug, Default)]
    struct Score {
        right: usize,
        wrong: Vec<String>,
        conflict: usize,
        ambiguous: usize,
        missing: usize,
    }

    /// Makes the synthetic builds `names` (`src/builds.py`), matches each and scores it. `None`
    /// to skip.
    fn score_builds(names: &[&str]) -> Option<Vec<(String, Score)>> {
        let bytes = reference()?;
        let listing = root().join("dumps/disasm.txt");
        if !listing.exists() || !python_runs() {
            assert!(
                std::env::var_os(exe::REQUIRE_VARIABLE).is_none(),
                "needs dumps/disasm.txt and python3"
            );
            println!("skipped: no dumps/disasm.txt or python3");
            return None;
        }
        let dir = std::env::temp_dir().join(format!("xtask-builds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let run = std::process::Command::new("python3")
            .arg("-I")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/builds.py"))
            .arg(root())
            .arg(&dir)
            .args(names)
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let signatures = committed();
        let _ = bytes;
        let mut out = Vec::new();
        for name in names {
            let image = std::fs::read(dir.join(format!("{name}.exe"))).unwrap();
            let pe = Pe::parse(&image).unwrap();
            let truth: HashMap<u32, Option<u32>> =
                std::fs::read_to_string(dir.join(format!("{name}.truth")))
                    .unwrap()
                    .lines()
                    .map(|l| {
                        let (a, t) = l.split_once('\t').unwrap();
                        (
                            u32::from_str_radix(a, 16).unwrap(),
                            u32::from_str_radix(t, 16).ok(),
                        )
                    })
                    .collect();
            assert_eq!(truth.len(), signatures.len());
            let outcome = Matcher::new(&pe, &signatures).locate(Stage::Order);
            let mut score = Score::default();
            for (i, s) in signatures.iter().enumerate() {
                match outcome.found[i] {
                    Found::At(a) if truth[&s.address] == Some(a) => score.right += 1,
                    Found::At(a) => score.wrong.push(format!(
                        "{name}: {:#08x} at {a:#08x}, should be {:?} ({:?})",
                        s.address, truth[&s.address], outcome.via[i]
                    )),
                    Found::Conflict => score.conflict += 1,
                    Found::Ambiguous => score.ambiguous += 1,
                    Found::Missing => score.missing += 1,
                }
            }
            out.push((name.to_string(), score));
        }
        let _ = std::fs::remove_dir_all(&dir);
        Some(out)
    }

    fn python_runs() -> bool {
        std::process::Command::new("python3")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn assert_no_wrong_placement(scores: &[(String, Score)]) {
        for (name, s) in scores {
            println!(
                "{name}: right {}, wrong {}, conflict {}, ambiguous {}, missing {}",
                s.right,
                s.wrong.len(),
                s.conflict,
                s.ambiguous,
                s.missing
            );
            assert!(
                s.wrong.is_empty(),
                "{:#?}",
                &s.wrong[..s.wrong.len().min(20)]
            );
            assert!(s.right > 10_000, "{name}: only {} found", s.right);
        }
    }

    /// The builds that once put names at wrong addresses: bodies reordered, calls sent to another
    /// thunk, functions gone.
    #[test]
    fn a_changed_build_gets_no_wrong_placement() {
        let Some(scores) = score_builds(&["reorder", "thunkswap", "removed"]) else {
            return;
        };
        assert_no_wrong_placement(&scores);
    }

    /// All nine synthetic builds.
    #[test]
    #[ignore = "several minutes; run with --ignored"]
    fn every_synthetic_build_gets_no_wrong_placement() {
        let all = [
            "base",
            "shift",
            "padding",
            "reorder",
            "duplicate",
            "midcall",
            "thunkswap",
            "removed",
            "calleeprologue",
        ];
        let Some(scores) = score_builds(&all) else {
            return;
        };
        assert_no_wrong_placement(&scores);
    }
}

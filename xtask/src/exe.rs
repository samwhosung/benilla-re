//! The exe half of the check: identify the reader's `WoW.exe`; check every function row's body
//! against the disassembly; regenerate the tables; run the workspace's tests with skipping
//! forbidden, and require every test the table and the docs name to have passed.

use crate::bodies::{self, Image, Listing};
use crate::check::{Checked, Problem};
use crate::tables;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

/// The build every finding is read from, as `tools/exe.env` gives it to every tool.
const IDENTITY: &str = include_str!("../../tools/exe.env");

fn identity(key: &str) -> &'static str {
    IDENTITY
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("tools/exe.env sets {key}"))
}

/// The size of WoW.exe 1.12.1.5875 in bytes.
pub fn size() -> u64 {
    identity("EXE_SIZE").parse().expect("EXE_SIZE is a decimal")
}

/// The SHA-256 of WoW.exe 1.12.1.5875, in lowercase hex.
pub fn sha256() -> &'static str {
    identity("EXE_SHA256")
}

/// Names the exe in place of `WoW/WoW.exe`.
pub const EXE_VARIABLE: &str = "BENILLA_RE_WOW_EXE";
/// Set (to `1`), a test that needs the exe and cannot find it panics instead of skipping.
pub const REQUIRE_VARIABLE: &str = "BENILLA_RE_REQUIRE_EXE";

/// Where the exe is: `$BENILLA_RE_WOW_EXE` when set, else `WoW/WoW.exe` in the repo.
pub fn locate(root: &Path) -> PathBuf {
    match env::var_os(EXE_VARIABLE) {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => root.join("WoW").join("WoW.exe"),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Identity {
    Missing,
    /// Another build: its size and SHA-256.
    Other(u64, String),
    Reference,
}

pub fn identify(path: &Path) -> io::Result<Identity> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Identity::Missing),
        Err(error) => return Err(error),
    };
    let mut hasher = Sha256::new();
    let size = io::copy(&mut file, &mut hasher)?;
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(verdict(size, digest))
}

fn verdict(size: u64, sha256: String) -> Identity {
    if size == self::size() && sha256 == self::sha256() {
        Identity::Reference
    } else {
        Identity::Other(size, sha256)
    }
}

pub struct Outcome {
    pub problems: Vec<Problem>,
    pub summary: String,
}

fn problem(place: impl Into<String>, message: impl Into<String>) -> Problem {
    Problem {
        place: place.into(),
        message: message.into(),
    }
}

/// Identifies the exe, then checks the bodies, the tables and the tests against it.
pub fn run(root: &Path, checked: &Checked) -> io::Result<Outcome> {
    let exe = locate(root);
    let shown = exe.strip_prefix(root).unwrap_or(&exe).display().to_string();
    let refusal = match identify(&exe)? {
        Identity::Reference => None,
        Identity::Missing => Some(format!(
            "no exe here; copy WoW.exe from your 1.12.1.5875 client to WoW/WoW.exe, or set \
             {EXE_VARIABLE} to its path"
        )),
        Identity::Other(size, sha256) => Some(format!(
            "refused: {size} bytes, SHA-256 {sha256}; the findings are read from 1.12.1.5875, \
             {} bytes, SHA-256 {}",
            self::size(),
            self::sha256()
        )),
    };
    if let Some(message) = refusal {
        return Ok(Outcome {
            problems: vec![problem(shown, message)],
            summary: "WoW.exe refused, nothing run against it".to_string(),
        });
    }
    let exe = exe.canonicalize()?;
    let root = root.canonicalize()?;
    let mut problems = Vec::new();
    let mut summary = vec!["WoW.exe is 1.12.1.5875".to_string()];

    // The disassembly the findings cite, written when missing. It restarts at every row, so a
    // row it has no instruction at was added after it was written, and it is written again.
    let listing_path = root.join("dumps").join("disasm.txt");
    let rows = functions(checked);
    let mut written = false;
    if !listing_path.exists() {
        problems.extend(write_listing(&root, &exe)?);
        written = true;
    }
    if listing_path.exists() {
        let mut listing = Listing::read(&listing_path)?;
        if !written && rows.iter().any(|row| !listing.contains(row.address)) {
            problems.extend(write_listing(&root, &exe)?);
            listing = Listing::read(&listing_path)?;
            summary.push("dumps/disasm.txt rewritten for rows newer than it".to_string());
        }
        let bytes = fs::read(&exe)?;
        let image = Image::new(&bytes)
            .ok_or_else(|| io::Error::other("WoW.exe has no readable section table"))?;
        let found = bodies::check(&rows, &listing, &image);
        summary.push(format!(
            "{} bodies against {} instructions",
            checked.functions.len(),
            listing.len()
        ));
        problems.extend(found);
    }

    if listing_path.exists() {
        let listing = Listing::read(&listing_path)?;
        let text = signature_table(checked, &listing, &exe)?;
        if fs::read_to_string(root.join("tables").join("signatures.tsv"))
            .ok()
            .as_deref()
            != Some(text.as_str())
        {
            problems.push(problem(
                "tables/signatures.tsv:1",
                "differs from what `cargo xtask signatures` writes from the exe",
            ));
        }
        summary.push(format!("{} signatures", text.lines().count() - 1));
    }

    let (found, regenerated) = tables::regenerate(&root, &exe)?;
    summary.push(format!("{regenerated} tables regenerated"));
    problems.extend(found);

    let (found, tests) = run_tests(&root, &exe, checked)?;
    summary.push(tests);
    problems.extend(found);
    Ok(Outcome {
        problems,
        summary: summary.join("; "),
    })
}

/// The function rows, for the body check and the listing.
pub fn functions(checked: &Checked) -> Vec<bodies::Function> {
    checked
        .functions
        .iter()
        .filter_map(|row| {
            Some(bodies::Function {
                place: row.place(),
                address: u32::from_str_radix(row.get("address").strip_prefix("0x")?, 16).ok()?,
                size: row.get("size").parse().ok()?,
            })
        })
        .collect()
}

/// The text of `tables/signatures.tsv` for the function rows, from the exe and the listing.
pub fn signature_table(checked: &Checked, listing: &Listing, exe: &Path) -> io::Result<String> {
    let bytes = fs::read(exe)?;
    let pe = crate::signature::Pe::parse(&bytes).map_err(io::Error::other)?;
    let rows: Vec<(u32, u32)> = functions(checked)
        .iter()
        .map(|f| (f.address, f.size))
        .collect();
    let signatures = crate::signature::generate(&pe, listing, &rows).map_err(io::Error::other)?;
    Ok(crate::signature::table(&signatures))
}

/// Writes `dumps/disasm.txt` with `tools/disasm.sh`; a problem if it fails.
fn write_listing(root: &Path, exe: &Path) -> io::Result<Option<Problem>> {
    let status = Command::new("bash")
        .arg("tools/disasm.sh")
        .current_dir(root)
        .env(EXE_VARIABLE, exe)
        .status()?;
    Ok((!status.success()).then(|| problem("tools/disasm.sh", format!("failed ({status})"))))
}

/// The cargo profile the tests run under: release optimization with overflow checks and debug
/// assertions (`[profile.exetest]` in `Cargo.toml`).
const TEST_PROFILE: &str = "exetest";

/// Runs `cargo test --workspace --profile exetest` with skipping forbidden, and checks every named
/// test passed.
fn run_tests(root: &Path, exe: &Path, checked: &Checked) -> io::Result<(Vec<Problem>, String)> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let (reader, writer) = io::pipe()?;
    let mut command = Command::new(cargo);
    command
        .current_dir(root)
        .args([
            "test",
            "--workspace",
            "--profile",
            TEST_PROFILE,
            "--no-fail-fast",
            "--message-format=json-render-diagnostics",
        ])
        .env(REQUIRE_VARIABLE, "1")
        .env(EXE_VARIABLE, exe)
        .stdout(writer.try_clone()?)
        .stderr(writer);
    let mut child = command.spawn()?;
    // The command holds the pipe's write ends; dropping it lets the read see the end.
    drop(command);
    let mut run = TestRun::new(root);
    let mut lines = BufReader::new(reader);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        if lines.read_until(b'\n', &mut buffer)? == 0 {
            break;
        }
        let line = String::from_utf8_lossy(&buffer);
        let line = line.trim_end_matches(['\n', '\r']);
        if !run.line(line) {
            println!("{line}");
        }
    }
    let status = child.wait()?;

    let mut problems = Vec::new();
    if !status.success() {
        problems.push(problem(
            format!("cargo test --workspace --profile {TEST_PROFILE}"),
            format!("failed ({status})"),
        ));
    }
    let named = checked.named_tests();
    let mut passed = 0;
    for (place, test) in &named {
        match run.verdict(test) {
            Verdict::Passed => passed += 1,
            failure => problems.push(problem(
                place.clone(),
                format!("test {test} did not pass: {}", failure.describe()),
            )),
        }
    }
    let (ok, failed, ignored) = run.totals();
    Ok((
        problems,
        format!(
            "{ok} tests passed, {failed} failed, {ignored} ignored; {passed} of {} named tests \
             passed",
            named.len()
        ),
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ran {
    Ok,
    Failed,
    Ignored,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Passed,
    Failed,
    Ignored,
    NotRun,
}

impl Verdict {
    fn describe(&self) -> &'static str {
        match self {
            Verdict::Passed => "it passed",
            Verdict::Failed => "it failed",
            Verdict::Ignored => "it was ignored",
            Verdict::NotRun => "no test of that name ran in that crate's tests/",
        }
    }
}

static TEST_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^test (.+?)(?: - should panic)? \.\.\. (ok|FAILED|ignored)(?:,.*)?$")
        .expect("a valid pattern")
});
static ESCAPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("a valid pattern"));

/// What one `cargo test` run printed: which test binary belongs to which crate, and each result.
pub struct TestRun {
    root: PathBuf,
    /// A test binary's file name, to the crate whose `tests/` it was built from.
    binaries: HashMap<String, Option<String>>,
    /// The crate of the binary running now: `None` before the first, `Some(None)` for a binary
    /// outside any crate's `tests/`.
    current: Option<Option<String>>,
    results: Vec<(Option<String>, String, Ran)>,
}

impl TestRun {
    pub fn new(root: &Path) -> TestRun {
        TestRun {
            root: root.to_path_buf(),
            binaries: HashMap::new(),
            current: None,
            results: Vec::new(),
        }
    }

    /// Reads one line of the merged output; true when it is cargo's JSON, which is not shown.
    pub fn line(&mut self, line: &str) -> bool {
        if line.starts_with('{') {
            if let Ok(message) = serde_json::from_str::<serde_json::Value>(line) {
                if message.get("reason").is_some() {
                    self.artifact(&message);
                    return true;
                }
            }
        }
        let plain = ESCAPE.replace_all(line, "");
        let plain = plain.trim();
        if let Some(rest) = plain.strip_prefix("Running ") {
            let binary = rest
                .rsplit_once('(')
                .map_or(rest, |(_, path)| path.trim_end_matches(')'))
                .trim_matches('`');
            let name = file_name(binary);
            self.current = Some(self.binaries.get(&name).cloned().flatten());
        } else if plain.starts_with("Doc-tests ") {
            self.current = Some(None);
        } else if let Some(found) = TEST_LINE.captures(plain) {
            let outcome = match &found[2] {
                "ok" => Ran::Ok,
                "FAILED" => Ran::Failed,
                _ => Ran::Ignored,
            };
            let krate = self.current.clone().flatten();
            self.results.push((krate, found[1].to_string(), outcome));
        }
        false
    }

    fn artifact(&mut self, message: &serde_json::Value) {
        if message["reason"] != "compiler-artifact" {
            return;
        }
        let (Some(executable), Some(source)) = (
            message["executable"].as_str(),
            message["target"]["src_path"].as_str(),
        ) else {
            return;
        };
        let krate = self.crate_of(Path::new(source));
        self.binaries.insert(file_name(executable), krate);
    }

    /// The crate a test source belongs to: `<crate>` for a file under `crates/<crate>/tests/`.
    fn crate_of(&self, source: &Path) -> Option<String> {
        let source = source
            .canonicalize()
            .unwrap_or_else(|_| source.to_path_buf());
        let relative = source.strip_prefix(&self.root).ok()?;
        let parts: Vec<_> = relative.iter().filter_map(|c| c.to_str()).collect();
        match parts.as_slice() {
            ["crates", krate, "tests", _, ..] => Some(krate.to_string()),
            _ => None,
        }
    }

    /// Whether `crate::test_fn` passed: it ran in one of the crate's own test binaries, passed
    /// every time it ran there, and was never ignored.
    pub fn verdict(&self, test: &str) -> Verdict {
        let Some((krate, name)) = test.split_once("::") else {
            return Verdict::NotRun;
        };
        let outcomes: Vec<Ran> = self
            .results
            .iter()
            .filter(|(k, n, _)| k.as_deref() == Some(krate) && n == name)
            .map(|r| r.2)
            .collect();
        if outcomes.is_empty() {
            Verdict::NotRun
        } else if outcomes.contains(&Ran::Failed) {
            Verdict::Failed
        } else if outcomes.contains(&Ran::Ignored) {
            Verdict::Ignored
        } else {
            Verdict::Passed
        }
    }

    /// Passed, failed and ignored tests across the whole run.
    pub fn totals(&self) -> (usize, usize, usize) {
        let count = |o: Ran| self.results.iter().filter(|r| r.2 == o).count();
        (count(Ran::Ok), count(Ran::Failed), count(Ran::Ignored))
    }
}

fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether a missing exe must fail a test rather than skip it: the contract every test
    /// that needs the exe follows.
    fn required() -> bool {
        env::var(REQUIRE_VARIABLE).is_ok_and(|value| !value.is_empty() && value != "0")
    }

    /// The contract in use: skips without the exe, panics without it when the exe is required,
    /// and refuses any other build.
    #[test]
    fn identifies_the_readers_exe() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the repo");
        let exe = locate(root);
        match identify(&exe).expect("the exe reads") {
            Identity::Reference => {}
            Identity::Missing if !required() => {
                eprintln!("skipped: no exe at {}", exe.display());
            }
            Identity::Missing => panic!(
                "{REQUIRE_VARIABLE} is set and there is no exe at {}",
                exe.display()
            ),
            Identity::Other(size, sha256) => {
                panic!(
                    "{} is another build: {size} bytes, SHA-256 {sha256}",
                    exe.display()
                )
            }
        }
    }

    #[test]
    fn the_identity_comes_from_the_one_file_every_tool_reads() {
        assert!(size() > 0);
        assert_eq!(sha256().len(), 64);
        assert!(sha256()
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    }

    #[test]
    fn only_the_reference_build_is_accepted() {
        assert_eq!(verdict(size(), sha256().to_string()), Identity::Reference);
        let other = "0".repeat(64);
        assert_eq!(
            verdict(size(), other.clone()),
            Identity::Other(size(), other.clone())
        );
        assert_eq!(
            verdict(size() + 1, sha256().to_string()),
            Identity::Other(size() + 1, sha256().to_string())
        );
    }

    #[test]
    fn identify_hashes_the_file_and_names_what_it_found() {
        let dir = env::temp_dir().join(format!("xtask-identify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temporary directory");
        let path = dir.join("WoW.exe");
        assert_eq!(identify(&path).expect("reads"), Identity::Missing);
        std::fs::write(&path, b"abc").expect("writes");
        let found = identify(&path).expect("reads");
        std::fs::remove_dir_all(&dir).expect("cleans up");
        assert_eq!(
            found,
            Identity::Other(
                3,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_string()
            )
        );
    }

    /// A transcript of `cargo test --message-format=json-render-diagnostics` with stdout and
    /// stderr merged, as `run` reads it.
    const TRANSCRIPT: &str = r#"{"reason":"compiler-artifact","target":{"kind":["test"],"name":"seed","src_path":"/repo/crates/random/tests/seed.rs"},"executable":"/repo/target/debug/deps/seed-1111"}
{"reason":"compiler-artifact","target":{"kind":["test"],"name":"view","src_path":"/repo/crates/camera/tests/view.rs"},"executable":"/repo/target/debug/deps/view-2222"}
{"reason":"compiler-artifact","target":{"kind":["lib"],"name":"random","src_path":"/repo/crates/random/src/lib.rs"},"executable":null}
{"reason":"compiler-artifact","target":{"kind":["lib"],"name":"random","src_path":"/repo/crates/random/src/lib.rs"},"executable":"/repo/target/debug/deps/random-3333"}
{"reason":"build-finished","success":true}
     Running unittests src/lib.rs (target/debug/deps/random-3333)

running 1 test
test seeds_alone ... ok

     Running tests/seed.rs (target/debug/deps/seed-1111)

running 4 tests
test srand_seed_state_init ... ok
test seed_ignored ... ignored, needs a capture
test seed_fails ... FAILED
test seed_panics - should panic ... ok
[1m[92m     Running[0m tests/view.rs (target/debug/deps/view-2222)
test camera_view_lookat ... ok
test seeds_alone ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
   Doc-tests random
test crates/random/src/lib.rs - seed (line 3) ... ok
"#;

    #[test]
    fn a_named_test_passes_only_in_its_own_crates_tests() {
        let mut run = TestRun::new(Path::new("/repo"));
        let mut shown = 0;
        for line in TRANSCRIPT.replace("\\x1b", "\x1b").lines() {
            if !run.line(line) {
                shown += 1;
            }
        }
        assert_eq!(shown, 18, "every line but cargo's JSON is shown");
        assert_eq!(
            run.verdict("random::srand_seed_state_init"),
            Verdict::Passed
        );
        assert_eq!(run.verdict("random::seed_panics"), Verdict::Passed);
        assert_eq!(run.verdict("random::seed_ignored"), Verdict::Ignored);
        assert_eq!(run.verdict("random::seed_fails"), Verdict::Failed);
        assert_eq!(run.verdict("camera::camera_view_lookat"), Verdict::Passed);
        assert_eq!(run.verdict("camera::seeds_alone"), Verdict::Passed);
        // A unit test under src/ is no test under tests/, and a crate's name scopes its tests.
        assert_eq!(run.verdict("random::seeds_alone"), Verdict::NotRun);
        assert_eq!(run.verdict("random::camera_view_lookat"), Verdict::NotRun);
        assert_eq!(run.verdict("random::absent"), Verdict::NotRun);
        assert_eq!(run.totals(), (6, 1, 1));
    }
}

//! Helper shared by the storm tests: the install's `Data` folder as a fixture.

use std::path::PathBuf;

/// The install's `Data` folder: `BENILLA_RE_WOW_DATA`, else `WoW/Data` in the repo. The archives are
/// a fixture, so a test without them prints one skip line and returns, even under
/// `BENILLA_RE_REQUIRE_EXE`.
pub fn data_fixture() -> Option<PathBuf> {
    let dir = match std::env::var_os(benilla_re_difftest::DATA_VARIABLE) {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => benilla_re_difftest::repo_root().join("WoW").join("Data"),
    };
    if dir.is_dir() {
        return Some(dir);
    }
    let thread = std::thread::current();
    let test = thread.name().unwrap_or("test");
    eprintln!("skip {test}: no install data at {}", dir.display());
    None
}

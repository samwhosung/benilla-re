//! Helper shared by the dbc tests: the install's `Data` folder as a fixture.

use std::path::PathBuf;

/// The install's `Data` folder (`benilla_re_difftest::data_dir`): a test without it skips with one
/// line, and fails under `BENILLA_RE_REQUIRE_DATA`.
pub fn data_fixture() -> Option<PathBuf> {
    benilla_re_difftest::data_dir()
}

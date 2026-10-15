**Claim:** what the client does, or what the change fixes, in a sentence or two.

**Evidence:** the addresses that decide it, with the instructions where they decide a point.

**Test:** the test, and that it bites (a wrong version failed it, and where).

**Checks:** which of `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo xtask check` and `cargo xtask site <dir>` passed; the last line of `cargo xtask check --exe`, with its skipped count, or what could not run.

**Second reading wanted:** yes or no. If yes, the claim spreads (docs/CONTRIBUTING.md, Adding a finding).

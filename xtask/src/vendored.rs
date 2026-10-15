//! `cargo xtask vendored <checkout>`: whether the copies of these tables a consumer keeps are the
//! tables at the commit it pins. The consumer's `reference/PIN` names a commit of this repo, and
//! each file under its `reference/` named like a table must equal `git show <commit>:tables/<name>`
//! byte for byte. A report; with `--strict` it fails when any file differs or is missing, and when
//! no file matched a table.
//!
//! The pin is a full 40-digit commit hash; a branch or tag name moves and is refused. A file is
//! `same` when its bytes equal the pinned table's, `differs` when they do not, and `missing` when
//! the pinned commit has no such table. A checkout with no `reference/PIN` has nothing to
//! compare: the command says so and succeeds.

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Same,
    Differs,
    Missing,
}

impl Status {
    fn word(&self) -> &'static str {
        match self {
            Status::Same => "same",
            Status::Differs => "differs",
            Status::Missing => "missing",
        }
    }
}

/// What the comparison found.
#[derive(Debug, PartialEq, Eq)]
pub enum Report {
    /// The checkout has no `reference/PIN`.
    NoPin,
    Pinned {
        pin: String,
        files: Vec<(String, Status)>,
        /// Commits on `origin/main` (else `main`) that the pin lacks, when this repo has a `main`.
        behind: Option<usize>,
        /// Tables whose bytes differ between the pin and that branch.
        changed: Option<usize>,
    },
}

fn git(repo: &Path, args: &[&str]) -> io::Result<Option<Vec<u8>>> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|e| io::Error::other(format!("git: {e}")))?;
    Ok(out.status.success().then_some(out.stdout))
}

/// Whether `pin` is a full commit hash: 40 hexadecimal digits.
fn is_full_hash(pin: &str) -> bool {
    pin.len() == 40 && pin.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The first line of `reference/PIN` that is not blank or a comment.
fn pin_of(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
}

/// Compares the consumer at `consumer` with the tables of the repo at `repo`.
pub fn compare(repo: &Path, consumer: &Path) -> io::Result<Report> {
    let reference = consumer.join("reference");
    let Some(pin) = fs::read_to_string(reference.join("PIN"))
        .ok()
        .and_then(|t| pin_of(&t))
    else {
        return Ok(Report::NoPin);
    };
    if !is_full_hash(&pin) {
        return Err(io::Error::other(format!(
            "{}: the pin {pin} is not a 40-digit commit hash",
            reference.join("PIN").display()
        )));
    }
    let commit = match git(
        repo,
        &["rev-parse", "--verify", &format!("{pin}^{{commit}}")],
    )? {
        Some(out) => String::from_utf8_lossy(&out).trim().to_string(),
        None => {
            return Err(io::Error::other(format!(
                "{}: the pin {pin} is no commit of this repo",
                reference.join("PIN").display()
            )))
        }
    };
    let mut names: Vec<String> = fs::read_dir(&reference)?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.ends_with(".tsv"))
        .collect();
    names.sort();
    let mut files = Vec::new();
    for name in names {
        let table = git(repo, &["show", &format!("{commit}:tables/{name}")])?;
        let status = match table {
            None => Status::Missing,
            Some(bytes) if bytes == fs::read(reference.join(&name))? => Status::Same,
            Some(_) => Status::Differs,
        };
        files.push((name, status));
    }
    let main = ["origin/main", "main"]
        .into_iter()
        .find(|r| git(repo, &["rev-parse", "--verify", "--quiet", r]).is_ok_and(|o| o.is_some()));
    let count = |args: &[&str]| -> io::Result<Option<usize>> {
        Ok(git(repo, args)?.and_then(|o| String::from_utf8_lossy(&o).trim().parse().ok()))
    };
    let behind = match main {
        Some(m) => count(&["rev-list", "--count", &format!("{commit}..{m}")])?,
        None => None,
    };
    let changed = match main {
        Some(m) => git(
            repo,
            &["diff", "--name-only", &commit, m, "--", "tables/*.tsv"],
        )?
        .map(|o| String::from_utf8_lossy(&o).lines().count()),
        None => None,
    };
    Ok(Report::Pinned {
        pin: commit,
        files,
        behind,
        changed,
    })
}

/// Prints the report; true when the run passes: always for a report, and with `strict` only when
/// at least one file was compared and every file is the pinned table.
pub fn run(repo: &Path, consumer: &Path, strict: bool) -> io::Result<bool> {
    let report = compare(repo, consumer)?;
    match &report {
        Report::NoPin => println!(
            "{}: no reference/PIN; it names no benilla-re commit yet, so there is nothing to compare",
            consumer.display()
        ),
        Report::Pinned {
            pin,
            files,
            behind,
            changed,
        } => {
            for (name, status) in files {
                println!("{:<8} reference/{name}", status.word());
            }
            let count = |s: Status| files.iter().filter(|(_, x)| *x == s).count();
            println!(
                "pinned at {pin}: {} same, {} differs, {} missing",
                count(Status::Same),
                count(Status::Differs),
                count(Status::Missing)
            );
            match (behind, changed) {
                (Some(0), _) => println!("the pin is at main"),
                (Some(n), Some(t)) => println!(
                    "the pin is {n} commits behind main; {t} tables differ between the pin and main"
                ),
                _ => println!("this repo has no main to compare the pin with"),
            }
        }
    }
    Ok(passes(&report, strict))
}

/// Whether the run passes.
pub fn passes(report: &Report, strict: bool) -> bool {
    match report {
        Report::NoPin => true,
        Report::Pinned { files, .. } => {
            !strict
                || (!files.is_empty() && files.iter().all(|(_, status)| *status == Status::Same))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?}");
    }

    #[test]
    fn each_copy_is_same_differs_or_missing_and_a_missing_pin_is_not_a_failure() {
        let base = std::env::temp_dir().join(format!("vendored-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let repo = base.join("re");
        let consumer = base.join("consumer");
        fs::create_dir_all(repo.join("tables")).expect("a repo");
        fs::create_dir_all(consumer.join("reference")).expect("a consumer");
        run_git(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join("tables/a.tsv"), "x\n1\n").expect("a table");
        fs::write(repo.join("tables/b.tsv"), "x\n2\n").expect("a table");
        run_git(&repo, &["add", "tables"]);
        run_git(&repo, &["commit", "-q", "-m", "first"]);
        let head = git(&repo, &["rev-parse", "HEAD"])
            .expect("git runs")
            .expect("a commit");
        let pin = String::from_utf8(head).expect("text").trim().to_string();

        // No PIN yet.
        let none = compare(&repo, &consumer).expect("a report");
        assert_eq!(none, Report::NoPin);
        assert!(passes(&none, true));

        let reference = consumer.join("reference");
        fs::write(reference.join("PIN"), format!("# the pin\n{pin}\n")).expect("a pin");
        fs::write(reference.join("a.tsv"), "x\n1\n").expect("a copy");
        fs::write(reference.join("b.tsv"), "x\n3\n").expect("a copy");
        fs::write(reference.join("c.tsv"), "x\n").expect("a copy");
        fs::write(reference.join("notes.md"), "not a table").expect("a file");

        // Main moves on: a table changes.
        fs::write(repo.join("tables/a.tsv"), "x\n9\n").expect("a table");
        run_git(&repo, &["commit", "-q", "-am", "second"]);

        let Report::Pinned {
            files,
            behind,
            changed,
            ..
        } = compare(&repo, &consumer).expect("a report")
        else {
            panic!("a pinned report");
        };
        assert_eq!(
            files,
            vec![
                ("a.tsv".to_string(), Status::Same),
                ("b.tsv".to_string(), Status::Differs),
                ("c.tsv".to_string(), Status::Missing),
            ]
        );
        assert_eq!((behind, changed), (Some(1), Some(1)));
        let report = compare(&repo, &consumer).expect("a report");
        assert!(passes(&report, false));
        assert!(!passes(&report, true));

        // A branch name moves; only a full hash pins.
        fs::write(reference.join("PIN"), "main\n").expect("a pin");
        assert!(compare(&repo, &consumer).is_err());
        fs::write(reference.join("PIN"), format!("{}\n", &pin[..12])).expect("a pin");
        assert!(compare(&repo, &consumer).is_err());
        fs::write(reference.join("PIN"), "not-a-commit\n").expect("a pin");
        assert!(compare(&repo, &consumer).is_err());
        fs::remove_dir_all(&base).expect("cleaned up");
    }

    fn pinned_files(report: Report) -> Vec<(String, Status)> {
        match report {
            Report::Pinned { files, .. } => files,
            Report::NoPin => panic!("a pinned report"),
        }
    }

    #[test]
    fn line_endings_and_the_last_newline_are_bytes_and_strict_wants_every_file_compared() {
        let base = std::env::temp_dir().join(format!("vendored-bytes-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let repo = base.join("re");
        let consumer = base.join("consumer");
        fs::create_dir_all(repo.join("tables")).expect("a repo");
        fs::create_dir_all(consumer.join("reference")).expect("a consumer");
        run_git(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join("tables/a.tsv"), "x\n1\n").expect("a table");
        fs::write(repo.join("tables/b.tsv"), "x\n2\n").expect("a table");
        run_git(&repo, &["add", "tables"]);
        run_git(&repo, &["commit", "-q", "-m", "first"]);
        let head = git(&repo, &["rev-parse", "HEAD"])
            .expect("git runs")
            .expect("a commit");
        let pin = String::from_utf8(head).expect("text").trim().to_string();
        let reference = consumer.join("reference");
        fs::write(reference.join("PIN"), format!("{pin}\n")).expect("a pin");

        // CRLF and a missing final newline each differ.
        fs::write(reference.join("a.tsv"), "x\r\n1\r\n").expect("a copy");
        fs::write(reference.join("b.tsv"), "x\n2").expect("a copy");
        let report = compare(&repo, &consumer).expect("a report");
        assert_eq!(
            pinned_files(compare(&repo, &consumer).expect("a report")),
            vec![
                ("a.tsv".to_string(), Status::Differs),
                ("b.tsv".to_string(), Status::Differs),
            ]
        );
        assert!(!passes(&report, true));

        // Only a table the pin lacks: nothing was compared, so strict fails.
        fs::remove_file(reference.join("a.tsv")).expect("removed");
        fs::remove_file(reference.join("b.tsv")).expect("removed");
        fs::write(reference.join("1.12-a.tsv"), "# header\nx\n1\n").expect("a copy");
        let report = compare(&repo, &consumer).expect("a report");
        assert_eq!(
            pinned_files(compare(&repo, &consumer).expect("a report")),
            vec![("1.12-a.tsv".to_string(), Status::Missing)]
        );
        assert!(passes(&report, false));
        assert!(!passes(&report, true));

        // No file at all: strict fails too; exact copies pass.
        fs::remove_file(reference.join("1.12-a.tsv")).expect("removed");
        let report = compare(&repo, &consumer).expect("a report");
        assert!(passes(&report, false));
        assert!(!passes(&report, true));
        fs::write(reference.join("a.tsv"), "x\n1\n").expect("a copy");
        assert!(passes(&compare(&repo, &consumer).expect("a report"), true));
        fs::remove_dir_all(&base).expect("cleaned up");
    }
}

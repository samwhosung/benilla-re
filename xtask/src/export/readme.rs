//! The export's `README.md`: `tools/export/README.md` with the build, the commit and the links
//! filled in. A function of the template and the origin, so the same tree writes the same bytes.

use super::header::Origin;

/// The template with `{{BUILD}}`, `{{SITE}}` and `{{REPO}}` replaced.
pub fn fill(template: &str, origin: &Origin) -> String {
    let build = if origin.rebased {
        format!(
            "These files describe WoW.exe 1.12.1.5875 with its function addresses rebased onto \
             another build by byte patterns (`cargo xtask match`): a match of bytes and position, \
             not a proof that a function is the same, and a global has no address here. The export has no \
             `bundle/tables/` and no `signature.tsv`, and the docs describe 5875.\n\n\
             Commit: `{}`",
            origin.commit
        )
    } else {
        format!(
            "These files fit WoW.exe 1.12.1.5875:\n\n\
             | | |\n|---|---|\n\
             | size | {} bytes |\n\
             | SHA-256 | `{}` |\n\
             | commit | `{}` |\n\n\
             `shasum -a 256 WoW.exe` on macOS and Linux, `certutil -hashfile WoW.exe SHA256` on \
             Windows.",
            thousands(origin.exe_size),
            origin.exe_sha256,
            origin.commit
        )
    };
    template
        .replace("{{BUILD}}", &build)
        .replace("{{SITE}}", super::SITE)
        .replace("{{REPO}}", super::REPO)
}

fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(rebased: bool) -> Origin {
        Origin {
            exe_size: 4775986,
            exe_sha256: "ab".repeat(32),
            commit: "0123abc".to_string(),
            rebased,
        }
    }

    #[test]
    fn the_build_and_the_links_are_filled_in() {
        let text = fill("{{BUILD}}\n{{SITE}}?0x5e6853 {{REPO}}", &origin(false));
        assert!(text.contains("4,775,986 bytes"));
        assert!(text.contains(&"ab".repeat(32)));
        assert!(text.contains("`0123abc`"));
        assert!(text.contains("https://samwhosung.github.io/benilla-re/?0x5e6853"));
        assert!(text.contains("https://github.com/samwhosung/benilla-re"));
        assert!(!text.contains("{{"));
    }

    #[test]
    fn a_rebased_export_says_so_and_names_no_exe() {
        let text = fill("{{BUILD}}", &origin(true));
        assert!(text.contains("rebased"));
        assert!(!text.contains("SHA-256"));
    }
}

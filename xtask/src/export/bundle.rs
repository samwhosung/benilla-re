//! `benilla-re.json`: every table of `symbols/` and `tables/` in one file, with a schema version,
//! the identity of the exe the rows describe and the commit they come from.
//!
//! ```text
//! { "schema": 1,
//!   "exe": { "name": "WoW.exe", "version": "1.12.1.5875", "size": N, "sha256": "..." },
//!   "commit": "...",
//!   "tables": { "symbols/functions": { "columns": [...], "rows": [[...], ...] }, ... } }
//! ```
//!
//! A bundle made from a rebased symbols folder (`cargo xtask match`) has `"size": null,
//! "sha256": null, "rebased": true` and no `tables/` entries, which hold 5875's addresses.
//!
//! A table is its file without the `.tsv`, in a map keyed by its path. Every cell is a string, as
//! in the file, so a table prints back to the file's bytes.

use super::header::Origin;
use super::table::Table;
use std::fmt::Write;

/// The version of the layout above.
pub const SCHEMA: u32 = 1;

fn string(s: &str) -> String {
    serde_json::to_string(s).expect("a string is JSON")
}

/// The bundle, one row to a line.
pub fn json(origin: &Origin, tables: &[(String, Table)]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{{\n\"schema\": {SCHEMA},");
    if origin.rebased {
        out.push_str(
            "\"exe\": {\"name\": \"WoW.exe\", \"version\": \"1.12.1.5875\", \"size\": null, \"sha256\": null, \"rebased\": true},\n",
        );
    } else {
        let _ = writeln!(
            out,
            "\"exe\": {{\"name\": \"WoW.exe\", \"version\": \"1.12.1.5875\", \"size\": {}, \"sha256\": {}}},",
            origin.exe_size,
            string(&origin.exe_sha256)
        );
    }
    let _ = writeln!(out, "\"commit\": {},", string(&origin.commit));
    out.push_str("\"tables\": {\n");
    for (i, (path, table)) in tables.iter().enumerate() {
        let _ = writeln!(out, "{}: {{", string(path));
        let columns: Vec<String> = table.columns.iter().map(|c| string(c)).collect();
        let _ = writeln!(out, "\"columns\": [{}],", columns.join(", "));
        out.push_str("\"rows\": [\n");
        for (j, row) in table.rows.iter().enumerate() {
            let cells: Vec<String> = row.iter().map(|c| string(c)).collect();
            let comma = if j + 1 < table.rows.len() { "," } else { "" };
            let _ = writeln!(out, "[{}]{comma}", cells.join(","));
        }
        let comma = if i + 1 < tables.len() { "," } else { "" };
        let _ = writeln!(out, "]\n}}{comma}");
    }
    out.push_str("}\n}\n");
    out
}

/// The files a bundle holds, each as its path and text.
#[cfg(test)]
pub fn files_of(json: &str) -> Result<Vec<(String, String)>, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let tables = value["tables"].as_object().ok_or("no tables")?;
    let mut files = Vec::new();
    for (path, table) in tables {
        let columns: Vec<String> = table["columns"]
            .as_array()
            .ok_or("no columns")?
            .iter()
            .map(|c| {
                c.as_str()
                    .map(str::to_string)
                    .ok_or("a column is no string")
            })
            .collect::<Result<_, _>>()?;
        let mut rows = Vec::new();
        for row in table["rows"].as_array().ok_or("no rows")? {
            rows.push(
                row.as_array()
                    .ok_or("a row is no array")?
                    .iter()
                    .map(|c| c.as_str().map(str::to_string).ok_or("a cell is no string"))
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        files.push((format!("{path}.tsv"), Table { columns, rows }.text()));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn origin() -> Origin {
        Origin {
            exe_size: 4775986,
            exe_sha256: "ab".repeat(32),
            commit: "0123abc".to_string(),
            rebased: false,
        }
    }

    #[test]
    fn a_bundle_gives_back_the_rows_it_was_made_from() {
        let a = Table::parse("a.tsv", "x\ty\n1\t\"quoted\" \\ back\n\t\u{e9}\n").unwrap();
        let b = Table::parse("b.tsv", "only\n").unwrap();
        let tables = vec![
            ("symbols/a".to_string(), a.clone()),
            ("tables/b".to_string(), b.clone()),
        ];
        let text = json(&origin(), &tables);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["schema"], 1);
        assert_eq!(value["exe"]["size"], 4775986);
        assert_eq!(value["exe"]["sha256"], "ab".repeat(32));
        assert_eq!(value["commit"], "0123abc");
        assert_eq!(
            files_of(&text).unwrap(),
            [
                ("symbols/a.tsv".to_string(), a.text()),
                ("tables/b.tsv".to_string(), b.text())
            ]
        );
    }

    #[test]
    fn the_same_tables_print_the_same_bytes() {
        let a = Table::parse("a.tsv", "x\n1\n2\n").unwrap();
        let tables = vec![("symbols/a".to_string(), a)];
        assert_eq!(json(&origin(), &tables), json(&origin(), &tables));
    }
}

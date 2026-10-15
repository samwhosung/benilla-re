//! `x64dbg/WoW.exe.dd32`: the labels and comments of the symbol table as an x64dbg database.
//!
//! x64dbg reads a plain JSON object whose `labels` and `comments` arrays hold one object per
//! entry: `module` (the module's name), `address` (a hex string, the offset from the module's
//! base), `manual` (true, so that x64dbg keeps the entry) and `text`. The file is the debugger's
//! `db/` entry for the exe; entries for another module are ignored.

use super::table::Table;
use std::fmt::Write;

/// The module the entries belong to.
const MODULE: &str = "wow.exe";
/// The exe's image base: an address in the tables less this is the offset x64dbg stores.
const IMAGE_BASE: u32 = 0x400000;
/// The longest label and comment x64dbg stores, less one for its terminator and one to spare.
const LABEL: usize = 254;
const COMMENT: usize = 510;

fn string(s: &str) -> String {
    serde_json::to_string(s).expect("a string is JSON")
}

/// At most `limit` bytes, cut at a character.
fn clip(text: &str, limit: usize) -> &str {
    let mut end = limit.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn entry(out: &mut String, last: bool, rva: u32, text: &str) {
    let _ = writeln!(
        out,
        "  {{\"module\": {}, \"address\": \"{rva:#X}\", \"manual\": true, \"text\": {}}}{}",
        string(MODULE),
        string(text),
        if last { "" } else { "," }
    );
}

/// One row: its address, name, and the cells a comment is made of.
struct Item {
    rva: u32,
    name: String,
    comment: String,
}

fn items(table: &Table, meaning: &str) -> Result<Vec<Item>, String> {
    let (address, name, meaning, confidence, proof, finding) = (
        table.column("address")?,
        table.column("name")?,
        table.column(meaning)?,
        table.column("confidence")?,
        table.column("proof")?,
        table.column("finding")?,
    );
    let mut out = Vec::new();
    for row in &table.rows {
        let address = u32::from_str_radix(row[address].trim_start_matches("0x"), 16)
            .map_err(|_| format!("{} is no address", row[address]))?;
        let rva = address
            .checked_sub(IMAGE_BASE)
            .ok_or_else(|| format!("{address:#x} is below the image base"))?;
        let mut comment = row[meaning].clone();
        let mut how = format!("{}, proof {}", row[confidence], row[proof]);
        if !row[finding].is_empty() {
            how = format!("{how}, {}", row[finding]);
        }
        let _ = write!(comment, " ({how})");
        out.push(Item {
            rva,
            name: row[name].clone(),
            comment,
        });
    }
    Ok(out)
}

/// The database of the function and global tables.
pub fn database(functions: &Table, globals: &Table) -> Result<String, String> {
    let mut all = items(functions, "meaning")?;
    all.extend(items(globals, "meaning")?);
    all.sort_by_key(|item| item.rva);
    let labels: Vec<&Item> = all.iter().filter(|i| !i.name.is_empty()).collect();
    let mut out = String::from("{\n\"labels\": [\n");
    for (i, item) in labels.iter().enumerate() {
        entry(
            &mut out,
            i + 1 == labels.len(),
            item.rva,
            clip(&item.name, LABEL),
        );
    }
    out.push_str("],\n\"comments\": [\n");
    for (i, item) in all.iter().enumerate() {
        entry(
            &mut out,
            i + 1 == all.len(),
            item.rva,
            clip(&item.comment, COMMENT),
        );
    }
    out.push_str("]\n}\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_comments_are_rvas_in_the_module() {
        let functions = Table::parse(
            "f.tsv",
            "address\tname\tmeaning\tconfidence\tproof\tfinding\n\
             0x401000\tEntry\tStarts it\tverified\tone\tcore#entry\n\
             0x401010\t\tRuns code\tinferred\tnone\t\n",
        )
        .unwrap();
        let globals = Table::parse(
            "g.tsv",
            "address\tname\tmeaning\tconfidence\tproof\tfinding\n\
             0x81d7c0\tCCamera::vftable\tHolds the vtable\tverified\tone\tcamera#x\n",
        )
        .unwrap();
        let text = database(&functions, &globals).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let labels = value["labels"].as_array().unwrap();
        let comments = value["comments"].as_array().unwrap();
        assert_eq!(labels.len(), 2, "an unnamed row has no label");
        assert_eq!(comments.len(), 3);
        assert_eq!(
            labels[0],
            serde_json::json!({"module": "wow.exe", "address": "0x1000", "manual": true, "text": "Entry"})
        );
        assert_eq!(labels[1]["address"], "0x41D7C0");
        assert_eq!(
            comments[0]["text"],
            "Starts it (verified, proof one, core#entry)"
        );
        assert_eq!(comments[1]["text"], "Runs code (inferred, proof none)");
    }

    #[test]
    fn a_long_text_is_cut_at_a_character() {
        assert_eq!(clip("abcdef", 3), "abc");
        assert_eq!(clip("ab\u{e9}", 3), "ab");
        assert_eq!(clip("ab", 9), "ab");
    }
}

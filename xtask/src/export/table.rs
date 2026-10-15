//! A tab-separated table of the repo, read whole: its header and its rows of cells.

/// One table: the header's column names and the rows' cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    /// Parses a file: one header line, then one line per row with as many cells as columns. The
    /// text ends in a newline and holds no carriage return.
    pub fn parse(file: &str, text: &str) -> Result<Table, String> {
        if text.contains('\r') {
            return Err(format!("{file}: has a carriage return"));
        }
        let body = text
            .strip_suffix('\n')
            .ok_or_else(|| format!("{file}: does not end in a newline"))?;
        let mut lines = body.split('\n');
        let header = lines.next().unwrap_or_default();
        let columns: Vec<String> = header.split('\t').map(str::to_string).collect();
        let mut rows = Vec::new();
        for (index, line) in lines.enumerate() {
            let cells: Vec<String> = line.split('\t').map(str::to_string).collect();
            if cells.len() != columns.len() {
                return Err(format!(
                    "{file}:{}: {} cells, the header has {}",
                    index + 2,
                    cells.len(),
                    columns.len()
                ));
            }
            rows.push(cells);
        }
        Ok(Table { columns, rows })
    }

    /// The file's text again, byte for byte.
    #[cfg(test)]
    pub fn text(&self) -> String {
        let mut text = self.columns.join("\t");
        text.push('\n');
        for row in &self.rows {
            text.push_str(&row.join("\t"));
            text.push('\n');
        }
        text
    }

    /// The index of a column.
    pub fn column(&self, name: &str) -> Result<usize, String> {
        self.columns
            .iter()
            .position(|c| c == name)
            .ok_or_else(|| format!("no column {name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_prints_the_text_it_was_read_from() {
        let text = "a\tb\tc\n1\t\t3\n\t\t\nx y\tz\t\n";
        let table = Table::parse("t.tsv", text).unwrap();
        assert_eq!(table.columns, ["a", "b", "c"]);
        assert_eq!(table.rows.len(), 3);
        assert_eq!(table.text(), text);
    }

    #[test]
    fn a_ragged_row_is_refused_with_its_line() {
        let error = Table::parse("t.tsv", "a\tb\n1\n").unwrap_err();
        assert_eq!(error, "t.tsv:2: 1 cells, the header has 2");
        assert!(Table::parse("t.tsv", "a\tb\n1\t2").is_err());
    }
}

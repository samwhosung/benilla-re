//! Which row holds an address, and how much of a set of rows is named, verified, at proof two
//! and tested.

/// The extents of the rows, sorted by start: `(start, end, index into the item list)`.
pub struct Spans {
    spans: Vec<(u32, u32, usize)>,
}

impl Spans {
    /// `rows` are `(start, size)` in item order. A row of unknown size holds its own address.
    pub fn new(rows: impl Iterator<Item = (u32, u32)>) -> Spans {
        let mut spans: Vec<(u32, u32, usize)> = rows
            .enumerate()
            .map(|(k, (start, size))| (start, start.saturating_add(size.max(1)), k))
            .collect();
        spans.sort();
        Spans { spans }
    }

    /// The row with `start <= address < start + size`: the nearest start at or below the
    /// address, else an earlier row that reaches it.
    pub fn holding(&self, address: u32) -> Option<usize> {
        let upto = self
            .spans
            .partition_point(|&(start, _, _)| start <= address);
        self.spans[..upto]
            .iter()
            .rev()
            .take(8)
            .find(|&&(_, end, _)| address < end)
            .map(|&(_, _, k)| k)
    }
}

/// Counts over a set of rows.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Coverage {
    pub rows: usize,
    pub named: usize,
    pub verified: usize,
    pub proof_two: usize,
    pub tested: usize,
}

impl Coverage {
    pub fn add(&mut self, named: bool, confidence: &str, proof: &str, tested: bool) {
        self.rows += 1;
        self.named += usize::from(named);
        self.verified += usize::from(confidence == "verified");
        self.proof_two += usize::from(confidence == "verified" && proof == "two");
        self.tested += usize::from(tested);
    }

    pub fn merge(&mut self, other: &Coverage) {
        self.rows += other.rows;
        self.named += other.named;
        self.verified += other.verified;
        self.proof_two += other.proof_two;
        self.tested += other.tested;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans() -> Spans {
        // a function 0x5e6850..0x5e6880, a global 0x800000..0x800010, a global of unknown size
        Spans::new([(0x5e6850, 0x30), (0x800000, 0x10), (0x810000, 0)].into_iter())
    }

    #[test]
    fn an_exact_address_finds_its_row() {
        assert_eq!(spans().holding(0x5e6850), Some(0));
        assert_eq!(spans().holding(0x800000), Some(1));
        assert_eq!(spans().holding(0x810000), Some(2));
    }

    #[test]
    fn an_instruction_inside_an_extent_finds_the_row() {
        assert_eq!(spans().holding(0x5e687f), Some(0));
        assert_eq!(spans().holding(0x80000f), Some(1));
    }

    #[test]
    fn an_address_past_the_end_or_before_all_rows_finds_none() {
        assert_eq!(spans().holding(0x5e6880), None);
        assert_eq!(spans().holding(0x810001), None);
        assert_eq!(spans().holding(0x400000), None);
    }

    #[test]
    fn a_row_inside_another_wins_and_the_outer_row_holds_the_rest() {
        let s = Spans::new([(0x1000, 0x100), (0x1010, 0x10)].into_iter());
        assert_eq!(s.holding(0x1014), Some(1));
        assert_eq!(s.holding(0x1020), Some(0));
    }

    #[test]
    fn coverage_counts_named_verified_proof_two_and_tested() {
        let mut c = Coverage::default();
        c.add(true, "verified", "two", true);
        c.add(true, "verified", "one", false);
        c.add(false, "inferred", "none", false);
        c.add(true, "inferred", "none", false);
        assert_eq!(
            c,
            Coverage {
                rows: 4,
                named: 3,
                verified: 2,
                proof_two: 1,
                tested: 1
            }
        );
        let mut total = Coverage::default();
        total.merge(&c);
        total.merge(&c);
        assert_eq!(total.rows, 8);
        assert_eq!(total.proof_two, 2);
    }
}

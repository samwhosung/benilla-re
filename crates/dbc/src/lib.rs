//! The WDBC client database container (`DBFilesClient\*.dbc`, the stores listed in
//! `tables/dbc_stores.tsv`).
//!
//! A dependency-free reader for the fixed-layout `WDBC` table the 1.12.1 client loads: the 20-byte
//! header, the fixed-width records, the string block and the localized-string column selection. The
//! per-table row schemas are the consumers' (`tables/dbc_schemas.tsv` gives each table's field count,
//! record size and string columns). The container facts are read from the exe: the start-up routine
//! `0x402350` parses `Startup_Strings` inline, checking the magic `0x43424457` at `0x4025bc` and striding
//! its 44-byte, 11-field records with `imul ...,0x2c` at `0x402679`. The reader also enforces the size
//! invariant `20 + records * record_size + string_block == file length`.

#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::fmt;

/// `'WDBC'` little-endian (`0x43424457`) — the magic the client checks at `0x4025bc`.
const MAGIC: u32 = 0x4342_4457;
/// The fixed WDBC header size: 5 × `u32`.
const HEADER: usize = 20;

/// The 1.12.1 locale columns of a localized-string field: eight string-block-offset columns, one per
/// locale in this order, followed by one flags column. The runtime locale index at `0xc0e080`
/// selects which offset column to read, `record + base_field*4 + locale*4`; `0x4027c0` picks one
/// with `mov eax,[eax+edx*4+8]`.
///
/// The names are the client's own, the eight pointers of the table at `0x8558a4`; the matching index
/// is stored to `0xc0e080` at `0x402562`. Slot 7, `0x8558e4`, reads `xxYY`; the string `esMX` occurs
/// nowhere in the image.
pub const LOCALES: [&str; 8] = [
    "enUS", "koKR", "frFR", "deDE", "zhCN", "zhTW", "esES", "xxYY",
];

/// Why a byte slice is not a valid WDBC table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbcError {
    /// Fewer than the 20-byte header.
    TooSmall(usize),
    /// First dword is not `'WDBC'`.
    BadMagic(u32),
    /// `20 + record_count*record_size + string_block_size` ≠ the file length — not a well-formed
    /// WDBC container (the strongest structural check; holds for every real table).
    SizeMismatch { expected: usize, actual: usize },
}

impl fmt::Display for DbcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbcError::TooSmall(n) => write!(f, "WDBC: {n} bytes, smaller than the 20-byte header"),
            DbcError::BadMagic(m) => write!(f, "WDBC: bad magic {m:#010x} (expected 'WDBC')"),
            DbcError::SizeMismatch { expected, actual } => {
                write!(
                    f,
                    "WDBC: size mismatch — header implies {expected} bytes, file is {actual}"
                )
            }
        }
    }
}

impl std::error::Error for DbcError {}

/// A parsed WDBC table: owns its bytes; exposes the records and the string block. Schema-agnostic —
/// the caller reads columns by index (`field_u32`/`field_i32`/`field_f32`/`string_field`) per the
/// table's own schema (the consumer's).
pub struct Dbc {
    bytes: Vec<u8>,
    record_count: u32,
    field_count: u32,
    record_size: u32,
    string_block_size: u32,
}

impl Dbc {
    /// Parse a `DBFilesClient\*.dbc` blob (for example the bytes `benilla_re_storm::Vfs::read` returns). Validates the
    /// magic and the `header + records + string-block` size invariant.
    pub fn parse(bytes: Vec<u8>) -> Result<Dbc, DbcError> {
        if bytes.len() < HEADER {
            return Err(DbcError::TooSmall(bytes.len()));
        }
        let rd = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        let magic = rd(0);
        if magic != MAGIC {
            return Err(DbcError::BadMagic(magic));
        }
        let record_count = rd(4);
        let field_count = rd(8);
        let record_size = rd(12);
        let string_block_size = rd(16);
        let expected =
            HEADER + (record_count as usize) * (record_size as usize) + string_block_size as usize;
        if expected != bytes.len() {
            return Err(DbcError::SizeMismatch {
                expected,
                actual: bytes.len(),
            });
        }
        Ok(Dbc {
            bytes,
            record_count,
            field_count,
            record_size,
            string_block_size,
        })
    }

    pub fn record_count(&self) -> u32 {
        self.record_count
    }
    pub fn field_count(&self) -> u32 {
        self.field_count
    }
    pub fn record_size(&self) -> u32 {
        self.record_size
    }
    pub fn string_block_size(&self) -> u32 {
        self.string_block_size
    }

    fn string_block_start(&self) -> usize {
        HEADER + (self.record_count as usize) * (self.record_size as usize)
    }

    /// The raw bytes of record `rec` (`record_size` wide). Panics if `rec >= record_count`.
    pub fn record(&self, rec: u32) -> &[u8] {
        assert!(
            rec < self.record_count,
            "record {rec} out of {} ",
            self.record_count
        );
        let start = HEADER + (rec as usize) * (self.record_size as usize);
        &self.bytes[start..start + self.record_size as usize]
    }

    /// Column `field` of record `rec` as a little-endian `u32`. Panics if either index is OOB.
    pub fn field_u32(&self, rec: u32, field: u32) -> u32 {
        assert!(
            field < self.field_count,
            "field {field} out of {}",
            self.field_count
        );
        let r = self.record(rec);
        let o = (field as usize) * 4;
        u32::from_le_bytes(r[o..o + 4].try_into().unwrap())
    }

    /// Column `field` as `i32` / `f32` (the same 4 bytes, reinterpreted per the table's schema).
    pub fn field_i32(&self, rec: u32, field: u32) -> i32 {
        self.field_u32(rec, field) as i32
    }
    pub fn field_f32(&self, rec: u32, field: u32) -> f32 {
        f32::from_bits(self.field_u32(rec, field))
    }

    /// The string at byte `offset` into the string block (NUL-terminated), as raw bytes. An offset
    /// past the block, or `0` (the conventional empty slot), yields an empty slice.
    pub fn string_bytes(&self, offset: u32) -> &[u8] {
        let off = offset as usize;
        if off >= self.string_block_size as usize {
            return &[];
        }
        let base = self.string_block_start() + off;
        let block_end = self.string_block_start() + self.string_block_size as usize;
        let mut end = base;
        while end < block_end && self.bytes[end] != 0 {
            end += 1;
        }
        &self.bytes[base..end]
    }

    /// The string at `offset`, decoded as UTF-8 (lossy — DBC strings are locale-encoded; enUS is
    /// ASCII, other locales may be Windows-1252/etc., which the consumer re-decodes if needed).
    pub fn string(&self, offset: u32) -> Cow<'_, str> {
        String::from_utf8_lossy(self.string_bytes(offset))
    }

    /// Column `field` of record `rec` is a `u32` string-block offset → its string.
    pub fn string_field(&self, rec: u32, field: u32) -> Cow<'_, str> {
        self.string(self.field_u32(rec, field))
    }

    /// A localized-string field: `base_field` is the first of the 8 per-locale offset columns;
    /// `locale` (0..8, see [`LOCALES`]) picks the column. Mirrors the client's
    /// `record + base_field*4 + locale*4` selection (`0x4027c0`).
    pub fn loc_string(&self, rec: u32, base_field: u32, locale: usize) -> Cow<'_, str> {
        self.string_field(rec, base_field + locale as u32)
    }
}

//! `tdb-edit` — a staged edit session over a TDB snapshot.
//!
//! Phase 1 scope: stage fixed-bit field edits, then apply them to the original bytes and
//! re-serialize through `tdb-core` (which recomputes all CRCs). Edits are addressed by **raw**
//! (on-disk) table/field ids. Metadata-driven validation (ranges, `update` flag, enums) and the
//! entity/structural operations from `docs/EDIT-MODEL.md` are later increments.

use std::fmt;
use tdb_core::TdbFile;

/// A single staged field edit (raw on-disk ids).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldEdit {
    pub table: String,
    pub record: u32,
    pub field: String,
    pub value: u64,
}

#[derive(Debug)]
pub enum EditError {
    Parse(String),
    UnknownTable(String),
    UnknownField { table: String, field: String },
    OutOfRange { table: String, record: u32 },
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::Parse(e) => write!(f, "parse error: {e}"),
            EditError::UnknownTable(t) => write!(f, "unknown table {t:?}"),
            EditError::UnknownField { table, field } => {
                write!(f, "unknown field {field:?} in table {table:?}")
            }
            EditError::OutOfRange { table, record } => {
                write!(f, "record {record} out of range in table {table:?}")
            }
        }
    }
}
impl std::error::Error for EditError {}

/// A set of staged edits to apply to one database snapshot.
#[derive(Default, Debug)]
pub struct EditSession {
    edits: Vec<FieldEdit>,
}

impl EditSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    pub fn len(&self) -> usize {
        self.edits.len()
    }

    pub fn edits(&self) -> &[FieldEdit] {
        &self.edits
    }

    /// Stage a fixed-bit field edit (raw ids).
    pub fn set_field(&mut self, table: &str, record: u32, field: &str, value: u64) -> &mut Self {
        self.edits.push(FieldEdit {
            table: table.to_string(),
            record,
            field: field.to_string(),
            value,
        });
        self
    }

    /// Apply all staged edits to `original` and return new bytes with all CRCs recomputed.
    /// With no edits this returns a byte-identical copy (the round-trip invariant).
    pub fn apply(&self, original: &[u8]) -> Result<Vec<u8>, EditError> {
        let db = TdbFile::parse(original).map_err(|e| EditError::Parse(e.to_string()))?;
        let mut buf = original.to_vec();
        for ed in &self.edits {
            let table = db
                .table(&ed.table)
                .ok_or_else(|| EditError::UnknownTable(ed.table.clone()))?;
            let field = table
                .field(&ed.field)
                .ok_or_else(|| EditError::UnknownField {
                    table: ed.table.clone(),
                    field: ed.field.clone(),
                })?;
            if !db.set_u64(&mut buf, table, ed.record, field, ed.value) {
                return Err(EditError::OutOfRange {
                    table: ed.table.clone(),
                    record: ed.record,
                });
            }
        }
        db.recompute_crcs(&mut buf);
        Ok(buf)
    }
}

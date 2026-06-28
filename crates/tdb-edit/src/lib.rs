//! `tdb-edit` — a staged edit session over a TDB snapshot.
//!
//! Phase 1 scope: stage fixed-bit field edits, then apply them to the original bytes and
//! re-serialize through `tdb-core` (which recomputes all CRCs). Edits are addressed by **raw**
//! (on-disk) table/field ids. Metadata-driven validation (ranges, `update` flag, enums) and the
//! entity/structural operations from `docs/EDIT-MODEL.md` are later increments.

pub mod txn;

use std::fmt;
use tdb_core::TdbFile;
use tdb_defs::{FieldMetaIndex, ValidationError};

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
    UnknownField {
        table: String,
        field: String,
    },
    OutOfRange {
        table: String,
        record: u32,
    },
    /// No metadata for this `(table, field)` — a validated edit cannot be staged.
    NoMeta {
        table: String,
        field: String,
    },
    /// The value failed metadata validation at entry (range / not-updatable).
    Invalid(ValidationError),
    /// The entity model could not be assembled from the working bytes.
    Entity(String),
    /// No roster slot matches the requested `(team, player)` pair.
    NoSuchSlot { team_id: u64, player_id: u64 },
    /// An operation argument was outside its legal set.
    InvalidValue { what: &'static str, value: i64 },
    /// No player with that `wBIz` id exists.
    NoSuchPlayer(u64),
    /// A record-allocation primitive failed.
    Alloc(String),
    /// A repairing transaction's post-condition audit failed; the operation was rolled back.
    AuditFailed {
        predicate: &'static str,
        violations: usize,
    },
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
            EditError::NoMeta { table, field } => {
                write!(f, "no metadata for {table:?}.{field:?}")
            }
            EditError::Invalid(e) => write!(f, "{e}"),
            EditError::Entity(e) => write!(f, "entity model error: {e}"),
            EditError::NoSuchSlot { team_id, player_id } => {
                write!(f, "no roster slot for player {player_id} on team {team_id}")
            }
            EditError::InvalidValue { what, value } => {
                write!(f, "invalid {what}: {value}")
            }
            EditError::NoSuchPlayer(id) => write!(f, "no player with wBIz {id}"),
            EditError::Alloc(e) => write!(f, "record allocation failed: {e}"),
            EditError::AuditFailed {
                predicate,
                violations,
            } => write!(
                f,
                "operation rolled back: {predicate} audit failed with {violations} violation(s)"
            ),
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

    /// Stage a **validated** edit from a logical value (the EDIT-MODEL edit-entry guardrail).
    ///
    /// Looks up the field's metadata, validates `value` against it at entry (legal range, and the
    /// `update` flag), encodes it to raw bits (two's-complement / sentinel aware), and stages it.
    /// On rejection the edit is **not** staged and the reason is returned.
    pub fn set_field_checked(
        &mut self,
        meta: &FieldMetaIndex,
        table: &str,
        record: u32,
        field: &str,
        value: i64,
    ) -> Result<(), EditError> {
        let fm = meta.field(table, field).ok_or_else(|| EditError::NoMeta {
            table: table.to_string(),
            field: field.to_string(),
        })?;
        fm.validate(value).map_err(EditError::Invalid)?;
        self.set_field(table, record, field, fm.encode(value));
        Ok(())
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

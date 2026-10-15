//! Delta referential gate — which dangling references an operation *introduced*.
//!
//! LEAGUE-R2: the pre/post comparison is a multiset of violation identities, never a count. A
//! violation's identity is `(row key, value)` where the row key is the source table's `key` field
//! values from the metadata (keys are never editable through the scalar seam and structural
//! operations mint fresh ids, so an untouched violation keeps its identity when a row-removal
//! shifts or swaps record indices). Tables whose metadata marks no key field fall back to the
//! record index. Any identity whose post-count exceeds its pre-count is a new violation:
//! substituting one invalid value for another on the same row, or moving a violation to another
//! row while repairing the old one, is refused even when the total stays equal or drops.

use std::collections::HashMap;

use tdb_core::{FieldDescriptor, Table, TdbFile};
use tdb_defs::FieldMetaIndex;
use tdb_domain_validation::audit_edge;
use tdb_relations::{RelBinding, RelationRegistry};

/// One dangling reference present after an operation that was not present before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDanglingReference {
    pub relationship: String,
    pub source_table: String,
    pub source_field: String,
    pub target_table: String,
    /// Record index in the post-operation database.
    pub record: u32,
    pub value: u64,
    /// The row identity used for the comparison (key-field values, or `[record]` without keys).
    pub row_key: Vec<u64>,
}

/// Identity of a dangling reference: `(row key, value)`.
type Identity = (Vec<u64>, u64);
/// Dangling rows of one edge in record order: `(record, value, row key)`.
type DanglingRow = (u32, u64, Vec<u64>);

fn key_fields<'a>(table: &'a Table, meta: &FieldMetaIndex) -> Vec<&'a FieldDescriptor> {
    table
        .fields
        .iter()
        .filter(|f| {
            meta.field(&table.id.raw(), &f.id.raw())
                .map(|m| m.key)
                .unwrap_or(false)
        })
        .collect()
}

fn identities(
    db: &TdbFile,
    b: &RelBinding,
    meta: &FieldMetaIndex,
) -> Option<(HashMap<Identity, usize>, Vec<DanglingRow>)> {
    let audit = audit_edge(db, b)?;
    let table = db.table_canonical(&b.source_table)?;
    let keys = key_fields(table, meta);
    let mut counts: HashMap<Identity, usize> = HashMap::new();
    let mut rows = Vec::with_capacity(audit.dangling.len());
    for (record, value) in audit.dangling {
        let row_key: Vec<u64> = if keys.is_empty() {
            vec![u64::from(record)]
        } else {
            keys.iter()
                .map(|f| db.read_u64(table, record, f).unwrap_or(u64::MAX))
                .collect()
        };
        *counts.entry((row_key.clone(), value)).or_insert(0) += 1;
        rows.push((record, value, row_key));
    }
    Some((counts, rows))
}

/// Every dangling reference in `after` that `before` did not already carry, over all confirmed
/// relationship bindings present in both databases. Edges whose tables or fields are absent are
/// skipped (the title has no such rule). Pre-existing violations that are unchanged — including
/// ones whose record index moved — are not reported.
pub fn new_dangling_references(
    before: &TdbFile,
    after: &TdbFile,
    relations: &RelationRegistry,
    meta: &FieldMetaIndex,
) -> Vec<NewDanglingReference> {
    let mut out = Vec::new();
    for rel in relations.relationships() {
        for b in &rel.bindings {
            let (Some((pre, _)), Some((mut post_counts, post_rows))) =
                (identities(before, b, meta), identities(after, b, meta))
            else {
                continue;
            };
            // Consume the tolerated budget per identity; whatever remains is new.
            for (id, n) in &pre {
                if let Some(c) = post_counts.get_mut(id) {
                    *c = c.saturating_sub(*n);
                }
            }
            for (record, value, row_key) in post_rows {
                let Some(c) = post_counts.get_mut(&(row_key.clone(), value)) else {
                    continue;
                };
                if *c == 0 {
                    continue;
                }
                *c -= 1;
                out.push(NewDanglingReference {
                    relationship: rel.name.clone(),
                    source_table: b.source_table.clone(),
                    source_field: b.source_field.clone(),
                    target_table: b.target_table.clone(),
                    record,
                    value,
                    row_key,
                });
            }
        }
    }
    out
}

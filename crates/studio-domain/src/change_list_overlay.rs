//! Save Editor over NHL 07 / NHL 08 roster *change lists*.
//!
//! These roster saves do not hold the roster. They hold insert / delete / update companion tables
//! of the game database's `rosters` save-group tables ([`crate::roster_difflist`]); the game applies
//! them to its own database at load. The Save Editor opens such a save as one composite database:
//! the game's bundled database with the save's changes applied. Only the parent tables the save
//! can record changes to are editable; every other table is frozen (read-only), and so is any field
//! of an update-only parent that its update companion does not carry.
//!
//! Saving turns the composite back into a change list. Only rows whose key changed since the file
//! was read are regenerated; every other companion row is kept as the file holds it. A changed key
//! is recorded the way NHL 07 records it natively — a delete + insert pair where the parent has
//! both companions (game-written roster rows), else an update row (game-written skater ratings) —
//! and its companion tables are written key-sorted with `max == current` and allocation type 6
//! (empty: 2), the serialization the game itself produces. The result is re-projected and must
//! reproduce the composite's rows field for field.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use tdb_core::{Endian, Table, TdbFile};
use tdb_defs::FieldMetaIndex;

use crate::roster_difflist::{effective_roster, recognize, Mapping, Operation};
use crate::{DomainError, Result};

const FILE_HEADER: usize = 0x18;

fn invalid(msg: impl Into<String>) -> DomainError {
    DomainError::Validation(msg.into())
}

fn change_list_error(e: String) -> DomainError {
    invalid(format!(
        "Studio could not read this roster save's recorded changes against the game's database: {e}"
    ))
}

/// The composite's change-list state.
pub struct ChangeListOverlay {
    /// The game's complete database the change list applies to.
    base: Arc<Vec<u8>>,
    /// The change list the file holds now (re-based after every save).
    inner: Vec<u8>,
    /// Raw ids of the parent tables the change list can record (editable).
    parents: BTreeSet<String>,
    /// Raw ids of read-only tables, plus `table.field` entries for unrecordable fields.
    frozen: BTreeSet<String>,
    /// The last change list [`ChangeListOverlay::strip`] produced and the composite it came from,
    /// so re-reading that file reproduces the session's exact working bytes.
    packaged: Mutex<Option<(Vec<u8>, Vec<u8>)>>,
}

impl std::fmt::Debug for ChangeListOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeListOverlay")
            .field("parents", &self.parents)
            .field("frozen", &self.frozen.len())
            .finish()
    }
}

/// `true` when `inner` is a roster change list against `base`.
pub fn is_change_list(base: &[u8], inner: &[u8], meta: &FieldMetaIndex) -> bool {
    recognize(base, inner, meta).is_ok()
}

fn put_u16(b: &mut [u8], o: usize, v: u16, e: Endian) {
    let w = match e {
        Endian::Big => v.to_be_bytes(),
        Endian::Little => v.to_le_bytes(),
    };
    b[o..o + 2].copy_from_slice(&w);
}

fn put_u32(b: &mut [u8], o: usize, v: u32, e: Endian) {
    let w = match e {
        Endian::Big => v.to_be_bytes(),
        Endian::Little => v.to_le_bytes(),
    };
    b[o..o + 4].copy_from_slice(&w);
}

fn get_u32(b: &[u8], o: usize, e: Endian) -> u32 {
    let w: [u8; 4] = b[o..o + 4].try_into().unwrap();
    match e {
        Endian::Big => u32::from_be_bytes(w),
        Endian::Little => u32::from_le_bytes(w),
    }
}

/// Replace table `id`'s records with exactly `rows`, allocating `max(rows, min_max)` slots
/// (zeroed slack after the rows). Later tables move; `db_size` and every CRC are recomputed.
/// `alloc` overwrites the table's allocation-type word when given.
fn set_rows(
    bytes: &[u8],
    id: &str,
    rows: &[Vec<u8>],
    min_max: usize,
    alloc: Option<u32>,
) -> Result<Vec<u8>> {
    let db = TdbFile::parse(bytes)?;
    let t = db
        .table(id)
        .ok_or_else(|| invalid(format!("table {id} is missing")))?;
    let reclen = t.record_len_bytes as usize;
    let new_max = rows.len().max(min_max);
    if new_max > u16::MAX as usize {
        return Err(invalid(format!("table {id} would exceed its record limit")));
    }
    let start = t.record_block;
    let old_end = start + t.max_records as usize * reclen;
    if old_end > bytes.len() - 4 || rows.iter().any(|r| r.len() != reclen) {
        return Err(invalid(format!("table {id} has an inconsistent record layout")));
    }
    let mut out = Vec::with_capacity(bytes.len() + new_max * reclen);
    out.extend_from_slice(&bytes[..start]);
    for r in rows {
        out.extend_from_slice(r);
    }
    out.resize(start + new_max * reclen, 0);
    out.extend_from_slice(&bytes[old_end..]);
    let delta = (new_max * reclen) as i64 - (old_end - start) as i64;
    let e = db.endian;
    let dir_end = FILE_HEADER + 8 * db.tables.len();
    for i in 0..db.tables.len() {
        let entry = FILE_HEADER + i * 8;
        let off = get_u32(&out, entry + 4, e) as i64;
        if dir_end as i64 + off > t.block_offset as i64 {
            put_u32(&mut out, entry + 4, (off + delta) as u32, e);
        }
    }
    put_u16(&mut out, t.block_offset + 0x14, new_max as u16, e);
    put_u16(&mut out, t.block_offset + 0x16, rows.len() as u16, e);
    if let Some(a) = alloc {
        put_u32(&mut out, t.block_offset + 4, a, e);
    }
    put_u32(&mut out, 0x08, (db.db_size as i64 + delta) as u32, e);
    let view = out.clone();
    TdbFile::parse(&view)?.recompute_crcs(&mut out);
    Ok(out)
}

fn record<'a>(bytes: &'a [u8], t: &Table, i: usize) -> &'a [u8] {
    let start = t.record_block + i * t.record_len_bytes as usize;
    &bytes[start..start + t.record_len_bytes as usize]
}

fn read_field(row: &[u8], t: &Table, id: &str, e: Endian) -> Option<u64> {
    let f = t.field(id)?;
    (f.bit_width <= 64).then(|| tdb_core::read_bits(row, f.bit_offset, f.bit_width, e))
}

/// Copy every field of `to` (a companion layout) from `row`, laid out as `from` (the parent).
fn project_row(row: &[u8], from: &Table, to: &Table, e: Endian) -> Result<Vec<u8>> {
    let mut out = vec![0u8; to.record_len_bytes as usize];
    for f in &to.fields {
        let src = from
            .field(&f.id.raw())
            .ok_or_else(|| invalid(format!("field {} is missing from its parent", f.id.raw())))?;
        let mut offset = 0;
        while offset < f.bit_width {
            let width = (f.bit_width - offset).min(32);
            let bits = tdb_core::read_bits(row, src.bit_offset + offset, width, e);
            tdb_core::write_bits(&mut out, f.bit_offset + offset, width, bits, e);
            offset += width;
        }
    }
    Ok(out)
}

/// Field ids whose values differ between two rows of `t` (strings compared as raw bits).
fn differing_fields(a: &[u8], b: &[u8], t: &Table, e: Endian) -> Vec<String> {
    t.fields
        .iter()
        .filter(|f| {
            let mut offset = 0;
            while offset < f.bit_width {
                let width = (f.bit_width - offset).min(64);
                if tdb_core::read_bits(a, f.bit_offset + offset, width, e)
                    != tdb_core::read_bits(b, f.bit_offset + offset, width, e)
                {
                    return true;
                }
                offset += width;
            }
            false
        })
        .map(|f| f.id.raw())
        .collect()
}

/// Rows of `parent` keyed by `key`; a repeated key is an error.
fn keyed_rows(
    bytes: &[u8],
    db: &TdbFile<'_>,
    parent: &str,
    key: &str,
) -> Result<BTreeMap<u64, Vec<u8>>> {
    let t = db
        .table(parent)
        .ok_or_else(|| invalid(format!("table {parent} is missing")))?;
    let mut rows = BTreeMap::new();
    for i in 0..t.current_records as usize {
        let r = record(bytes, t, i);
        let k = read_field(r, t, key, db.endian)
            .ok_or_else(|| invalid(format!("{parent} has no usable key field")))?;
        if rows.insert(k, r.to_vec()).is_some() {
            return Err(invalid(format!(
                "two rows of {parent} share the key {k}; the save can record only one"
            )));
        }
    }
    Ok(rows)
}

/// The parent table each mapping belongs to, with its companions by operation.
fn companions(schema: &[Mapping]) -> BTreeMap<&str, BTreeMap<Operation, &Mapping>> {
    let mut out: BTreeMap<&str, BTreeMap<Operation, &Mapping>> = BTreeMap::new();
    for m in schema {
        out.entry(m.parent_table.as_str())
            .or_default()
            .insert(m.operation, m);
    }
    out
}

/// Open `inner` (a roster change list) over `base`: the composite database and its overlay.
pub fn compose(
    inner: &[u8],
    base: Arc<Vec<u8>>,
    meta: &FieldMetaIndex,
) -> Result<(Vec<u8>, ChangeListOverlay)> {
    let schema = recognize(&base, inner, meta).map_err(change_list_error)?.tables;
    let composite = build_composite(&base, inner, meta, &schema)?;
    let b = TdbFile::parse(&base)?;
    let by_parent = companions(&schema);
    let parents: BTreeSet<String> = by_parent.keys().map(|p| p.to_string()).collect();
    let mut frozen: BTreeSet<String> = b
        .tables
        .iter()
        .map(|t| t.id.raw())
        .filter(|id| !parents.contains(id))
        .collect();
    // A parent with no delete + insert pair records changes to existing rows only through its
    // update companion: fields that companion does not carry cannot be saved.
    for (parent, ops) in &by_parent {
        if ops.contains_key(&Operation::Insert) && ops.contains_key(&Operation::Delete) {
            continue;
        }
        let recordable: BTreeSet<String> = match ops.get(&Operation::Update) {
            Some(m) => TdbFile::parse(inner)?
                .table(&m.save_table)
                .map(|t| t.fields.iter().map(|f| f.id.raw()).collect())
                .unwrap_or_default(),
            None => BTreeSet::new(),
        };
        if let Some(t) = b.table(parent) {
            for f in &t.fields {
                if !recordable.contains(&f.id.raw()) {
                    frozen.insert(format!("{parent}.{}", f.id.raw()));
                }
            }
        }
    }
    Ok((
        composite,
        ChangeListOverlay {
            base,
            inner: inner.to_vec(),
            parents,
            frozen,
            packaged: Mutex::new(None),
        },
    ))
}

/// The game database with the change list applied. A parent keeps its rows in the database's
/// order (a replaced row stays in place, a deleted one is dropped); rows the save inserts follow,
/// in key order.
fn build_composite(
    base: &[u8],
    inner: &[u8],
    meta: &FieldMetaIndex,
    schema: &[Mapping],
) -> Result<Vec<u8>> {
    let effective = effective_roster(base, inner, meta).map_err(change_list_error)?;
    let b = TdbFile::parse(base)?;
    let mut out = base.to_vec();
    for (parent, ops) in companions(schema) {
        let key = &ops.values().next().unwrap().key;
        let t = b.table(parent).unwrap();
        let rows = &effective[parent];
        let mut ordered = Vec::with_capacity(rows.len());
        let mut placed = BTreeSet::new();
        for i in 0..t.current_records as usize {
            let k = read_field(record(base, t, i), t, key, b.endian)
                .ok_or_else(|| invalid(format!("{parent} has no usable key field")))?;
            if let Some(r) = rows.get(&k) {
                ordered.push(r.clone());
                placed.insert(k);
            }
        }
        ordered.extend(
            rows.iter()
                .filter(|(k, _)| !placed.contains(k))
                .map(|(_, r)| r.clone()),
        );
        let unchanged = ordered.len() == t.current_records as usize
            && ordered
                .iter()
                .enumerate()
                .all(|(i, r)| r.as_slice() == record(base, t, i));
        if !unchanged {
            out = set_rows(&out, parent, &ordered, t.max_records as usize, None)?;
        }
    }
    Ok(out)
}

impl ChangeListOverlay {
    /// Raw ids of read-only tables, plus `table.field` entries for fields the save cannot record.
    pub fn frozen(&self) -> &BTreeSet<String> {
        &self.frozen
    }

    /// How many of the game's tables the save can record changes to.
    pub fn editable_tables(&self) -> usize {
        self.parents.len()
    }

    /// The change list for `composite`: the file's own rows for every key that has not changed
    /// since it was read, regenerated rows for every key that has. Pure (records nothing).
    pub fn change_list_for(&self, composite: &[u8], meta: &FieldMetaIndex) -> Result<Vec<u8>> {
        let base = &self.base[..];
        let schema = recognize(base, &self.inner, meta).map_err(change_list_error)?.tables;
        let opened = effective_roster(base, &self.inner, meta).map_err(change_list_error)?;
        let (b, c, s) = (
            TdbFile::parse(base)?,
            TdbFile::parse(composite)?,
            TdbFile::parse(&self.inner)?,
        );
        let e = s.endian;
        let by_parent = companions(&schema);
        let name = |id: &str| meta.table_name(id).unwrap_or(id).to_string();

        // Every companion's rows as the file holds them.
        let mut lists: BTreeMap<String, Vec<Vec<u8>>> = BTreeMap::new();
        for t in &s.tables {
            let rows = (0..t.current_records as usize)
                .map(|i| record(&self.inner, t, i).to_vec())
                .collect();
            lists.insert(t.id.raw(), rows);
        }
        let mut touched = BTreeSet::new();
        let mut current_all = BTreeMap::new();
        for (parent, ops) in &by_parent {
            let key = ops.values().next().unwrap().key.clone();
            let pt = b.table(parent).unwrap();
            let current = keyed_rows(composite, &c, parent, &key)?;
            let before = &opened[*parent];
            let base_rows = keyed_rows(base, &b, parent, &key)?;
            let keys: BTreeSet<u64> = current.keys().chain(before.keys()).copied().collect();
            for k in keys {
                let same = match (before.get(&k), current.get(&k)) {
                    (Some(x), Some(y)) => differing_fields(x, y, pt, e).is_empty(),
                    (None, None) => true,
                    _ => false,
                };
                if same {
                    continue;
                }
                // Drop whatever the file recorded for this key, then record it afresh.
                for m in ops.values() {
                    let ct = s.table(&m.save_table).unwrap();
                    let list = lists.get_mut(&m.save_table).unwrap();
                    list.retain(|r| read_field(r, ct, &key, e) != Some(k));
                    touched.insert(m.save_table.clone());
                }
                let room = |lists: &BTreeMap<String, Vec<Vec<u8>>>, op: Operation| {
                    ops.get(&op)
                        .filter(|m| lists[&m.save_table].len() < m.capacity as usize)
                        .copied()
                };
                let full = |op: Operation| {
                    ops.get(&op).map(|m| {
                        invalid(format!(
                            "This roster save has room for {} recorded changes to {}, and this change needs more. Undo some changes to {} to save.",
                            m.capacity,
                            name(parent),
                            name(parent)
                        ))
                    })
                };
                let push = |lists: &mut BTreeMap<String, Vec<Vec<u8>>>, m: &Mapping, row: &[u8]| -> Result<()> {
                    let ct = s.table(&m.save_table).unwrap();
                    let projected = project_row(row, pt, ct, e)?;
                    lists.get_mut(&m.save_table).unwrap().push(projected);
                    Ok(())
                };
                match (base_rows.get(&k), current.get(&k)) {
                    (None, None) => {}
                    (Some(old), None) => match room(&lists, Operation::Delete) {
                        Some(d) => push(&mut lists, d, old)?,
                        None => {
                            return Err(full(Operation::Delete).unwrap_or_else(|| {
                                invalid(format!(
                                    "This roster save cannot record removing a row from {}. Undo that change to save.",
                                    name(parent)
                                ))
                            }))
                        }
                    },
                    (None, Some(new)) => match room(&lists, Operation::Insert) {
                        Some(i) => push(&mut lists, i, new)?,
                        None => {
                            return Err(full(Operation::Insert).unwrap_or_else(|| {
                                invalid(format!(
                                    "This roster save cannot record adding a row to {}. Undo that change to save.",
                                    name(parent)
                                ))
                            }))
                        }
                    },
                    (Some(old), Some(new)) => {
                        let diff = differing_fields(old, new, pt, e);
                        if diff.is_empty() {
                            continue; // back to the game's own value: nothing to record
                        }
                        if let (Some(d), Some(i)) =
                            (room(&lists, Operation::Delete), room(&lists, Operation::Insert))
                        {
                            push(&mut lists, d, old)?;
                            push(&mut lists, i, new)?;
                            continue;
                        }
                        let update = ops.get(&Operation::Update).copied();
                        let carried: BTreeSet<String> = update
                            .and_then(|m| s.table(&m.save_table))
                            .map(|t| t.fields.iter().map(|f| f.id.raw()).collect())
                            .unwrap_or_default();
                        if let Some(f) = diff.iter().find(|f| !carried.contains(*f)) {
                            let field = meta.field(parent, f).map_or(f.as_str(), |m| m.name.as_str());
                            return Err(full(Operation::Insert).unwrap_or_else(|| {
                                invalid(format!(
                                    "This roster save cannot record a change to {field} in {}; the game records changes to only some of that table's fields. Undo that change to save.",
                                    name(parent)
                                ))
                            }));
                        }
                        match room(&lists, Operation::Update) {
                            Some(u) => push(&mut lists, u, new)?,
                            None => return Err(full(Operation::Update).unwrap()),
                        }
                    }
                }
            }
            current_all.insert(parent.to_string(), current);
        }
        if touched.is_empty() {
            return Ok(self.inner.clone());
        }

        // Companions the game writes are key-sorted, `max == current`, allocation 6 (empty: 2).
        let mut out = self.inner.clone();
        for id in &touched {
            let m = schema.iter().find(|m| &m.save_table == id).unwrap();
            let ct = s.table(id).unwrap();
            let mut rows = lists.remove(id).unwrap();
            rows.sort_by_key(|r| read_field(r, ct, &m.key, e));
            let alloc = if rows.is_empty() { 2 } else { 6 };
            out = set_rows(&out, id, &rows, 0, Some(alloc))?;
        }

        // The game's reading of the new change list must be exactly the composite's rows.
        let check = effective_roster(base, &out, meta).map_err(|e| {
            invalid(format!("Studio built a roster change list the game's rules reject ({e}); nothing was saved"))
        })?;
        for (parent, current) in &current_all {
            let pt = b.table(parent).unwrap();
            let read = &check[parent];
            let agrees = read.len() == current.len()
                && current.iter().all(|(k, row)| {
                    read.get(k)
                        .is_some_and(|r| differing_fields(r, row, pt, e).is_empty())
                });
            if !agrees {
                return Err(invalid(format!(
                    "Studio could not express the changes to {} as a roster change list; nothing was saved",
                    name(parent)
                )));
            }
        }
        Ok(out)
    }

    /// [`ChangeListOverlay::change_list_for`], remembering the result so the saved file re-reads
    /// as this exact composite.
    pub fn strip(&self, composite: &[u8], meta: &FieldMetaIndex) -> Result<Vec<u8>> {
        let out = self.change_list_for(composite, meta)?;
        *self.packaged.lock().unwrap() = Some((out.clone(), composite.to_vec()));
        Ok(out)
    }

    /// The composite for a change list read from the file: the session's own composite when the
    /// file holds what [`ChangeListOverlay::strip`] last produced, else a fresh composition.
    pub fn recompose(&self, inner: &[u8], meta: &FieldMetaIndex) -> Result<Vec<u8>> {
        if let Some((packed, composite)) = &*self.packaged.lock().unwrap() {
            if packed == inner {
                return Ok(composite.clone());
            }
        }
        let schema = recognize(&self.base, inner, meta).map_err(change_list_error)?.tables;
        let parents: BTreeSet<String> = schema.iter().map(|m| m.parent_table.clone()).collect();
        if parents != self.parents {
            return Err(invalid(
                "the file's tables changed shape since it was opened; reopen it",
            ));
        }
        build_composite(&self.base, inner, meta, &schema)
    }

    /// The file now holds `inner`: later saves record changes against it.
    pub fn rebase(&mut self, inner: Vec<u8>) {
        self.inner = inner;
    }
}

/// Put a change list back into its save container. NHL 07 / NHL 08 roster containers are a
/// fixed allocation: the change list is followed by zeros up to the original file length, and
/// only the container's first checksum covers it.
pub fn pack_container(original: &[u8], inner: &[u8]) -> Result<Vec<u8>> {
    use tdb_savedata::{ContainerVersion, SaveContainer};
    let parsed = SaveContainer::parse(original)?;
    if parsed.header.version != ContainerVersion::RawV3 {
        return Err(invalid(
            "Studio writes roster change lists only in the NHL 07 / NHL 08 save container",
        ));
    }
    let hlen = parsed.header.version.header_len();
    let old = parsed.extract_db()?;
    if original[hlen + old.len()..].iter().any(|b| *b != 0) {
        return Err(invalid(
            "This roster save carries data after its change list that Studio does not understand, so it cannot be rewritten.",
        ));
    }
    if hlen + inner.len() > original.len() {
        return Err(invalid(
            "These changes do not fit in the roster save's fixed space. Undo some changes to save.",
        ));
    }
    let mut out = original[..hlen].to_vec();
    out.extend_from_slice(inner);
    out.resize(original.len(), 0);
    tdb_savedata::checksum::recompute_a_only(&mut out);
    Ok(out)
}

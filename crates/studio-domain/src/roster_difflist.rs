//! Metadata-declared roster change lists and a narrowly scoped experimental NHL07 writer.
//! Neither snapshot mirror admission nor production roster-save editing is enabled here.
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use tdb_core::{Table, TdbFile};
use tdb_defs::FieldMetaIndex;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Insert,
    Delete,
    Update,
}

#[derive(Debug, Serialize)]
pub struct Mapping {
    pub save_table: String,
    pub parent_table: String,
    pub parent_name: String,
    pub operation: Operation,
    pub key: String,
    pub capacity: u16,
    pub records: u16,
}

#[derive(Debug, Serialize)]
pub struct Schema {
    pub tables: Vec<Mapping>,
}

fn validate(db: &TdbFile<'_>) -> Result<(), String> {
    if !db.size_ok() || !db.header_crc_ok() {
        return Err("invalid database size or header checksum".into());
    }
    let mut layout: Vec<_> = db.tables.iter().collect();
    layout.sort_by_key(|t| t.block_offset);
    let mut end = 24 + 8 * db.tables.len();
    for (i, table) in layout.iter().enumerate() {
        let next = layout
            .get(i + 1)
            .map_or(db.db_size as usize - 4, |t| t.block_offset);
        let records_end = table.record_block as u64
            + u64::from(table.record_len_bytes) * u64::from(table.max_records);
        if table.block_offset < end || records_end > next as u64 {
            return Err("overlapping table layout".into());
        }
        end = records_end as usize;
    }
    let mut ids = BTreeSet::new();
    for table in &db.tables {
        if !ids.insert(table.id.raw()) || table.current_records > table.max_records {
            return Err("duplicate table or invalid record count".into());
        }
        let end = (table.record_block as u64)
            + u64::from(table.record_len_bytes) * u64::from(table.max_records);
        if end > db.db_size as u64 {
            return Err(format!("{}: records exceed database", table.id.raw()));
        }
        let mut fields = BTreeSet::new();
        for field in &table.fields {
            if !fields.insert(field.id.raw())
                || u64::from(field.bit_offset) + u64::from(field.bit_width)
                    > u64::from(table.record_len_bytes) * 8
            {
                return Err(format!(
                    "{}: duplicate or out-of-bounds field",
                    table.id.raw()
                ));
            }
        }
    }
    Ok(())
}

/// Every companion must have an explicit XML declaration and matching descriptor shape.
/// There is deliberately no fallback to adjacency, row position, or another title's IDs.
pub fn recognize(
    base_bytes: &[u8],
    save_bytes: &[u8],
    meta: &FieldMetaIndex,
) -> Result<Schema, String> {
    let base = TdbFile::parse(base_bytes).map_err(|e| e.to_string())?;
    let save = TdbFile::parse(save_bytes).map_err(|e| e.to_string())?;
    validate(&base)?;
    validate(&save)?;
    if base.endian != save.endian {
        return Err("base and change-list byte orders differ".into());
    }
    let mut declared = BTreeMap::new();
    for (parent_id, spec) in meta
        .save_tables()
        .filter(|(_, s)| s.groups.iter().any(|g| g == "rosters"))
    {
        for (id, operation, limit) in [
            (&spec.insert_id, Operation::Insert, spec.max_insert),
            (&spec.delete_id, Operation::Delete, spec.max_delete),
            (&spec.update_id, Operation::Update, spec.max_update),
        ] {
            let Some(id) = id else {
                continue;
            };
            if declared
                .insert(id.as_str(), (parent_id, spec, operation, limit))
                .is_some()
            {
                return Err(format!("ambiguous declaration for {id}"));
            }
        }
    }
    if declared.is_empty() || declared.len() != save.tables.len() {
        return Err("roster companion table set differs from metadata".into());
    }
    let mut mappings = Vec::new();
    for table in &save.tables {
        let id = table.id.raw();
        let (parent_id, spec, operation, limit) = declared
            .get(id.as_str())
            .ok_or_else(|| format!("undeclared roster table {id}"))?;
        let parent = base
            .table(parent_id)
            .ok_or_else(|| format!("missing parent {parent_id}"))?;
        if spec.keys.len() != 1 {
            return Err(format!("{parent_id}: requires one declared key"));
        }
        let key = &spec.keys[0];
        let expected: BTreeSet<String> = match operation {
            Operation::Insert => parent.fields.iter().map(|f| f.id.raw()).collect(),
            Operation::Delete => [key.clone()].into_iter().collect(),
            Operation::Update => spec
                .update_fields
                .iter()
                .cloned()
                .chain([key.clone()])
                .collect(),
        };
        if expected != table.fields.iter().map(|f| f.id.raw()).collect() {
            return Err(format!(
                "{id}: fields do not match declared {operation:?} schema"
            ));
        }
        for field in &table.fields {
            let original = parent
                .field(&field.id.raw())
                .ok_or("missing parent field")?;
            if field.kind != original.kind || field.bit_width != original.bit_width {
                return Err(format!("{id}: field type/width differs from parent"));
            }
        }
        let capacity = match limit {
            Some(-1) => parent.max_records,
            Some(n) => u16::try_from(*n).map_err(|_| format!("{id}: invalid capacity"))?,
            None => return Err(format!("{id}: missing capacity")),
        };
        if table.current_records > capacity || table.max_records > capacity {
            return Err(format!("{id}: exceeds declared capacity {capacity}"));
        }
        mappings.push(Mapping {
            save_table: id,
            parent_table: (*parent_id).into(),
            parent_name: meta.table_name(parent_id).unwrap_or(parent_id).into(),
            operation: *operation,
            key: key.clone(),
            capacity,
            records: table.current_records,
        });
    }
    Ok(Schema { tables: mappings })
}

/// Diagnostic projection only. Rows retain the parent database's binary field layout.
/// Populated-save semantics still require validation against controlled in-game saves.
pub fn effective_roster(
    base_bytes: &[u8],
    save_bytes: &[u8],
    meta: &FieldMetaIndex,
) -> Result<BTreeMap<String, BTreeMap<u64, Vec<u8>>>, String> {
    let schema = recognize(base_bytes, save_bytes, meta)?;
    let base = TdbFile::parse(base_bytes).map_err(|e| e.to_string())?;
    let save = TdbFile::parse(save_bytes).map_err(|e| e.to_string())?;
    let mut result = BTreeMap::new();
    for mapping in &schema.tables {
        if result.contains_key(&mapping.parent_table) {
            continue;
        }
        let table = base.table(&mapping.parent_table).unwrap();
        let key = table.field(&mapping.key).ok_or("missing parent key")?;
        if key.kind != 3 || key.bit_width == 0 || key.bit_width > 64 {
            return Err("unsupported key encoding".into());
        }
        let mut rows = BTreeMap::new();
        for i in 0..table.current_records {
            let value = base
                .read_u64(table, i.into(), key)
                .ok_or("invalid parent key")?;
            if rows
                .insert(value, row(base_bytes, table, i).to_vec())
                .is_some()
            {
                return Err(format!(
                    "{}: duplicate parent key {value}",
                    mapping.parent_table
                ));
            }
        }
        result.insert(mapping.parent_table.clone(), rows);
    }
    // NHL07 game-written rosters replace existing rows with a delete/insert pair.
    // Directory order puts insert first; logical application must delete first.
    let mut touched = BTreeSet::new();
    let mut ordered: Vec<_> = schema.tables.iter().collect();
    ordered.sort_by_key(|m| match m.operation {
        Operation::Delete => 0,
        Operation::Insert => 1,
        Operation::Update => 2,
    });
    for mapping in ordered {
        let table = save.table(&mapping.save_table).unwrap();
        let parent = base.table(&mapping.parent_table).unwrap();
        let rows = result.get_mut(&mapping.parent_table).unwrap();
        let key = table.field(&mapping.key).unwrap();
        for i in 0..table.current_records {
            let value = save
                .read_u64(table, i.into(), key)
                .ok_or("invalid change-list key")?;
            let operation = match mapping.operation {
                Operation::Delete => 0,
                Operation::Insert => 1,
                Operation::Update => 2,
            };
            if !touched.insert((mapping.parent_table.clone(), value, operation)) {
                return Err("duplicate operation for the same parent key".into());
            }
            if operation == 2
                && (touched.contains(&(mapping.parent_table.clone(), value, 0))
                    || touched.contains(&(mapping.parent_table.clone(), value, 1)))
            {
                return Err(
                    "update combined with insert/delete requires runtime validation".into(),
                );
            }
            if mapping.operation == Operation::Delete {
                rows.remove(&value)
                    .ok_or("delete references missing parent key")?;
                continue;
            }
            let mut output = if mapping.operation == Operation::Insert {
                if rows.contains_key(&value) {
                    return Err("insert duplicates an existing parent key".into());
                }
                vec![0; parent.record_len_bytes as usize]
            } else {
                rows.get(&value)
                    .ok_or("update references missing parent key")?
                    .clone()
            };
            let source = row(save_bytes, table, i);
            for field in &table.fields {
                let dest = parent.field(&field.id.raw()).unwrap();
                // Copy raw bits in bounded chunks, including inline strings, without reinterpretation.
                let mut offset = 0;
                while offset < field.bit_width {
                    let width = (field.bit_width - offset).min(64);
                    let bits =
                        tdb_core::read_bits(source, field.bit_offset + offset, width, save.endian);
                    tdb_core::write_bits(
                        &mut output,
                        dest.bit_offset + offset,
                        width,
                        bits,
                        base.endian,
                    );
                    offset += width;
                }
            }
            rows.insert(value, output);
        }
    }
    Ok(result)
}

fn row<'a>(bytes: &'a [u8], table: &Table, index: u16) -> &'a [u8] {
    let start = table.record_block + usize::from(index) * table.record_len_bytes as usize;
    &bytes[start..start + table.record_len_bytes as usize]
}

/// Experimental NHL07 jersey writer for native qualification, not Studio save admission.
/// Preserves all existing replacements and edits one roster row by playerindex.
/// Only the observed roster delete/insert layout is accepted; no other edit type is enabled.
pub fn experimental_nhl07_jersey(
    base: &[u8],
    container: &[u8],
    meta: &FieldMetaIndex,
    player: u64,
    number: u64,
) -> Result<Vec<u8>, String> {
    if !(1..=99).contains(&number) {
        return Err("jersey number must be between 1 and 99".into());
    }
    write_nhl07_scalar(
        base,
        container,
        meta,
        player,
        number,
        "ulGe",
        "tRVs",
        &[("AtUj", Operation::Delete), ("ibGM", Operation::Insert)],
    )
}

/// Controlled native probe for stock skater speed. Value is the raw six-bit rating,
/// not a guessed conversion to the game's displayed scale. Update-table acceptance
/// still requires native verification; this does not enable the production editor.
pub fn experimental_nhl07_speed_raw(
    base: &[u8],
    container: &[u8],
    meta: &FieldMetaIndex,
    game_id: u64,
    value: u64,
) -> Result<Vec<u8>, String> {
    if value > 63 {
        return Err("raw speed must fit six bits (0–63)".into());
    }
    write_nhl07_scalar(
        base,
        container,
        meta,
        game_id,
        value,
        "yvSd",
        "YWPq",
        &[("RosT", Operation::Update)],
    )
}

fn write_nhl07_scalar(
    base: &[u8],
    container: &[u8],
    meta: &FieldMetaIndex,
    player: u64,
    number: u64,
    parent_id: &str,
    field_id: &str,
    operations: &[(&str, Operation)],
) -> Result<Vec<u8>, String> {
    use tdb_savedata::{ContainerVersion, SaveContainer, SaveType};
    let parsed = SaveContainer::parse(container).map_err(|e| e.to_string())?;
    if parsed.header.version != ContainerVersion::RawV3
        || parsed.header.save_type != SaveType::Roster
        || !parsed.checksums_valid()
    {
        return Err("requires a valid NHL07 raw roster container".into());
    }
    let inner = parsed.extract_db().map_err(|e| e.to_string())?;
    let schema = recognize(base, &inner, meta)?;
    let original = TdbFile::parse(base).map_err(|e| e.to_string())?;
    if schema.tables.len() != 31
        || original
            .table("cPbu")
            .is_none_or(|t| t.record_len_bytes != 508)
        || container.len() != 1_598_888
    {
        return Err("not the qualified NHL07 roster layout".into());
    }
    if container[28 + inner.len()..].iter().any(|b| *b != 0) {
        return Err("nonzero save trailer cannot be consumed as allocation".into());
    }
    let db = TdbFile::parse(&inner).map_err(|e| e.to_string())?;
    let mut checked = inner.clone();
    db.recompute_crcs(&mut checked);
    if checked != inner {
        return Err("inner database checksum mismatch".into());
    }
    for t in &db.tables {
        let id = t.id.raw();
        if t.current_records != t.max_records
            || t.data_alloc_type != if t.current_records == 0 { 2 } else { 6 }
            || (t.current_records > 0 && id != "ibGM" && id != "AtUj" && id != "RosT")
        {
            return Err(
                "populated save layout is outside the qualified roster replacement case".into(),
            );
        }
    }
    let effective = effective_roster(base, &inner, meta)?;
    let parent = original
        .table(parent_id)
        .ok_or("missing NHL07 parent table")?;
    let current = effective
        .get(parent_id)
        .and_then(|rows| rows.get(&player))
        .ok_or("player key does not exist in the parent table")?;
    let jersey = parent.field(field_id).ok_or("missing target field")?;
    if tdb_core::read_bits(
        current,
        jersey.bit_offset,
        jersey.bit_width,
        original.endian,
    ) == number
    {
        return Ok(container.to_vec());
    }
    let mut desired = current.clone();
    tdb_core::write_bits(
        &mut desired,
        jersey.bit_offset,
        jersey.bit_width,
        number,
        original.endian,
    );
    let mut output = inner.clone();
    for &(id, operation) in operations {
        let mapping = schema
            .tables
            .iter()
            .find(|m| m.save_table == id && m.operation == operation)
            .ok_or("missing roster companion")?;
        let view = TdbFile::parse(&output).map_err(|e| e.to_string())?;
        let table = view.table(id).unwrap();
        let key = table.field(&mapping.key).unwrap();
        let index = (0..table.current_records)
            .find(|i| view.read_u64(table, (*i).into(), key) == Some(player));
        let index = if let Some(index) = index {
            index as u32
        } else {
            if table.current_records >= mapping.capacity {
                return Err("roster change-list capacity reached".into());
            }
            let (grown, index) = view.append_record(id).map_err(|e| e.to_string())?;
            output = grown;
            index
        };
        let snapshot = output.clone();
        let view = TdbFile::parse(&snapshot).map_err(|e| e.to_string())?;
        let table = view.table(id).unwrap();
        let start = table.record_block + index as usize * table.record_len_bytes as usize;
        for f in &table.fields {
            let source = parent.field(&f.id.raw()).ok_or("missing parent field")?;
            for offset in (0..f.bit_width).step_by(32) {
                let width = (f.bit_width - offset).min(32);
                let bits = tdb_core::read_bits(
                    &desired,
                    source.bit_offset + offset,
                    width,
                    original.endian,
                );
                tdb_core::write_bits(
                    &mut output[start..start + table.record_len_bytes as usize],
                    f.bit_offset + offset,
                    width,
                    bits,
                    view.endian,
                );
            }
        }
        // Game-written populated companions switch allocation type from 2 to 6.
        output[table.block_offset + 4..table.block_offset + 8].copy_from_slice(&6u32.to_be_bytes());
        // Observed game output is key-sorted; maintain it when appending a replacement.
        let key = table.field(&mapping.key).unwrap();
        let mut order: Vec<u32> = (0..table.current_records as u32).collect();
        order.sort_by_key(|i| {
            let start = table.record_block + *i as usize * table.record_len_bytes as usize;
            tdb_core::read_bits(
                &output[start..start + table.record_len_bytes as usize],
                key.bit_offset,
                key.bit_width,
                view.endian,
            )
        });
        view.permute_records(&mut output, id, &order)
            .map_err(|e| e.to_string())?;
        view.recompute_crcs(&mut output);
    }
    let projected = effective_roster(base, &output, meta)?;
    let mut expected = effective;
    expected.get_mut(parent_id).unwrap().insert(player, desired);
    if projected != expected {
        return Err("write changed more than the requested field".into());
    }
    if 28 + output.len() > container.len() {
        return Err("save allocation exhausted".into());
    }
    // The zero tail is reserved capacity, not a trailer to append after the grown TDB.
    let mut packed = container[..28].to_vec();
    packed.extend_from_slice(&output);
    packed.resize(container.len(), 0);
    tdb_savedata::checksum::recompute_a_only(&mut packed);
    Ok(packed)
}

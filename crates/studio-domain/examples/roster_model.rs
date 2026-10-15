//! Read-only NHL07 difflist schema and effective-roster diagnostic.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: roster_model <game.db> <roster save> <game-meta.xml>".into());
    }
    let base = std::fs::read(&args[1])?;
    let container = std::fs::read(&args[2])?;
    let parsed = tdb_savedata::SaveContainer::parse(&container)?;
    if !parsed.checksums_valid() {
        return Err("save container checksum mismatch".into());
    }
    let save = parsed.extract_db()?;
    let meta = tdb_defs::FieldMetaIndex::from_xml(&std::fs::read_to_string(&args[3])?)?;
    let schema = studio_domain::roster_difflist::recognize(&base, &save, &meta)?;
    let effective = studio_domain::roster_difflist::effective_roster(&base, &save, &meta);
    let db = tdb_core::TdbFile::parse(&save)?;
    let populated: Vec<_> = schema.tables.iter().filter(|m| m.records > 0).map(|m| {
        let table = db.table(&m.save_table).unwrap();
        let rows: Vec<_> = (0..table.current_records).map(|i| {
            table.fields.iter().map(|f| {
                let id = f.id.raw();
                let name = meta.field(&m.parent_table, &id).map(|f| f.name.clone()).unwrap_or(id);
                let value = if f.kind == 3 { serde_json::json!(db.read_u64(table, i.into(), f)) }
                    else { serde_json::json!(db.read_inline_str(table, i.into(), f).map(|b| String::from_utf8_lossy(b).into_owned())) };
                (name, value)
            }).collect::<std::collections::BTreeMap<_,_>>()
        }).collect();
        serde_json::json!({"table": m.save_table, "parent": m.parent_name, "max_records": table.max_records, "alloc_type": table.data_alloc_type, "rows": rows})
    }).collect();
    let original = tdb_core::TdbFile::parse(&base)?;
    let mut differences = Vec::new();
    if let Ok(effective) = &effective {
        for (id, rows) in effective {
            let table = original.table(id).unwrap();
            let mapping = schema
                .tables
                .iter()
                .find(|m| &m.parent_table == id)
                .unwrap();
            let key = table.field(&mapping.key).unwrap();
            for i in 0..table.current_records {
                let value = original.read_u64(table, i.into(), key).unwrap();
                let Some(row) = rows.get(&value) else {
                    continue;
                };
                let mut changes = std::collections::BTreeMap::new();
                for f in table.fields.iter().filter(|f| f.kind == 3) {
                    let before = original.read_u64(table, i.into(), f).unwrap();
                    let after =
                        tdb_core::read_bits(row, f.bit_offset, f.bit_width, original.endian);
                    if before != after {
                        let name = meta
                            .field(id, &f.id.raw())
                            .map(|f| f.name.clone())
                            .unwrap_or(f.id.raw());
                        changes.insert(name, serde_json::json!({"before": before,"after": after}));
                    }
                }
                if !changes.is_empty() {
                    differences.push(serde_json::json!({"parent": mapping.parent_name,"key": value,"changes": changes}));
                }
            }
        }
    }
    // Include player identities referenced by roster replacements for readable QA reports.
    let bio = original.table("cPbu").unwrap();
    let slots = original.table("caBZ").unwrap();
    let slot_key = slots.field("qEfv").unwrap();
    let bio_key = bio.field("zIBw").unwrap();
    let player_id = slots
        .fields
        .iter()
        .find(|f| {
            meta.field("caBZ", &f.id.raw())
                .is_some_and(|m| m.name == "playerid")
        })
        .unwrap();
    let mut identities = Vec::new();
    for d in &differences {
        if d["parent"] != "exhibitionrostertable" {
            continue;
        }
        let key = d["key"].as_u64().unwrap();
        let Some(slot) = (0..slots.current_records)
            .find(|i| original.read_u64(slots, (*i).into(), slot_key) == Some(key))
        else {
            continue;
        };
        let player = original.read_u64(slots, slot.into(), player_id).unwrap();
        let Some(index) = (0..bio.current_records)
            .find(|i| original.read_u64(bio, (*i).into(), bio_key) == Some(player))
        else {
            continue;
        };
        let names: std::collections::BTreeMap<_, _> = bio
            .fields
            .iter()
            .filter(|f| {
                meta.field("cPbu", &f.id.raw())
                    .is_some_and(|m| m.name == "firstname" || m.name == "lastname")
            })
            .map(|f| {
                (
                    meta.field("cPbu", &f.id.raw()).unwrap().name.clone(),
                    original
                        .read_inline_str(bio, index.into(), f)
                        .map(|b| String::from_utf8_lossy(b).into_owned()),
                )
            })
            .collect();
        identities.push(serde_json::json!({"playerindex": key,"game_id":player,"name":names}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "save_model": "difflist", "write_enabled": false,
            "mapping": schema, "populated": populated, "inner_db_bytes": save.len(), "differences": differences, "players": identities,
            "projection_error": effective.as_ref().err(),
            "effective_record_counts": effective.as_ref().ok().map(|e| e.iter().map(|(k, v)| (k, v.len())).collect::<std::collections::BTreeMap<_,_>>())
        }))?
    );
    Ok(())
}

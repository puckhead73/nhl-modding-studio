//! NHL07 arena controls: label existing assignments without inventing asset identities.
use std::collections::BTreeMap;
use tdb_core::{Table, TdbFile};
use tdb_defs::FieldMetaIndex;
use crate::view::{descriptor_by_name, read_str_by_name, FieldView};

pub fn is_assignment(name: &str) -> bool {
    matches!(name, "icelogo" | "doorlogo" | "jumbotron" | "goalhorn" | "periodhorn" |
        "arenaaudioid" | "cityaudioid" | "chantaudioid" | "puck" | "roof" | "walls" |
        "railings" | "step" | "boardtrim" | "boardad" | "redline" | "seatlow" |
        "seatmedium" | "seathigh" | "icead1" | "icead2" | "icead3")
}

pub fn decorate(db: &TdbFile, meta: &FieldMetaIndex, source: &Table, fields: &mut [FieldView]) {
    if meta.table_name(&source.id.raw()) != Some("stockarenatable") { return; }
    for field in fields {
        if field.name == "arenatype" {
            field.enum_options = vec![(0, "North American ice".into()), (1, "International ice".into())];
        } else if matches!(field.name.as_str(), "boardadshown" | "icelogoshown") {
            field.enum_options = vec![(0, "Hidden".into()), (1, "Shown".into())];
        } else if is_assignment(&field.name) {
            let Some(descriptor) = descriptor_by_name(meta, source, &field.name) else { continue; };
            let mut choices: BTreeMap<i64, Vec<String>> = BTreeMap::new();
            for row in 0..source.current_records as u32 {
                let Some(value) = db.read_u64(source, row, descriptor) else { continue; };
                let name = read_str_by_name(db, meta, source, row, "arenaname");
                if !name.trim().is_empty() { choices.entry(value as i64).or_default().push(name); }
            }
            field.enum_options = choices.into_iter().map(|(value, names)| {
                let extra = if names.len() > 1 { format!(" (+{} venues)", names.len()-1) } else { String::new() };
                (value, format!("{}{} — option {value}", names[0], extra))
            }).collect();
            field.vocabulary_note = Some("Existing assignment used by the named venue; asset appearance and playback require in-game verification.".into());
        } else if matches!(field.name.as_str(), "icequality" | "crowdtype" | "crease") {
            field.editable = false;
            field.vocabulary_note = Some("This setting's NHL 07 behavior has not been verified.".into());
        }
        field.enum_label = field.enum_options.iter().find(|(v,_)| Some(*v)==field.decoded).map(|(_,label)|label.clone());
        if let Some(label) = &field.enum_label { field.display = label.clone(); }
    }
}

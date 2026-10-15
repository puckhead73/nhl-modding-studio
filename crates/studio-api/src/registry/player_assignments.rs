//! NHL07 assignments use combinations observed in the project's original database.
//! Later-title brand enums and generic-head sentinels are deliberately not borrowed.
use super::Registry;
use crate::{convert, dto::OpOutcomeDto, error::ApiError};
use serde::Serialize;
use std::collections::BTreeMap;
use studio_domain::{FieldValue, RosterModel};

#[derive(Serialize)]
pub struct AssignmentOption { pub key: String, pub label: String, pub users: usize }
#[derive(Serialize)]
pub struct PlayerAssignment {
    pub key: String, pub label: String, pub section: String, pub current: String,
    pub options: Vec<AssignmentOption>, pub availability: String,
}

// A selection copies these fields together; the same groups drive selective reverts.
pub(super) fn assignment_specs(goalie: bool) -> Vec<(&'static str, &'static str, &'static str, Vec<&'static str>)> {
    let mut out = vec![
        ("stick", "Stick", "Equipment", vec!["wXFx"]),
        ("skates", "Skates", "Equipment", vec!["Qurh"]),
        ("tape", "Stick tape", "Equipment", vec!["loJT"]),
    ];
    if goalie { out.extend([
        ("mask", "Mask", "Equipment", vec!["NaVF", "kGII"]),
        ("pads", "Pads", "Equipment", vec!["OGav", "gGPi"]),
        ("blocker", "Blocker", "Equipment", vec!["Kmxz", "JGoJ"]),
        ("trapper", "Trapper", "Equipment", vec!["WKkb", "hDYT"]),
    ]); } else { out.extend([
        ("helmet", "Helmet", "Equipment", vec!["NaVF", "IqZs"]),
        ("gloves", "Gloves", "Equipment", vec!["WKkb"]),
        ("visor", "Visor", "Equipment", vec!["zSZR"]),
        ("visor_color", "Visor color", "Equipment", vec!["OGav"]),
        ("pants", "Pants", "Equipment", vec!["sQPG"]),
        ("neck", "Neck protection", "Equipment", vec!["gfrc"]),
        ("shoulders", "Shoulder pads", "Equipment", vec!["PPXl"]),
        ("jersey_tuck", "Jersey tuck", "Equipment", vec!["Ypqj"]),
        ("sock_tuck", "Sock tuck", "Equipment", vec!["LWaV"]),
    ]); }
    out.extend([
        ("head", "Head and complexion", "Appearance", vec!["pPaD", "KTDQ", "CBfK", "Zief"]),
        ("skin", "Skin tone", "Appearance", vec!["KTDQ"]),
        ("left_eye", "Left eye color", "Appearance", vec!["Zief"]),
        ("right_eye", "Right eye color", "Appearance", vec!["CBfK"]),
        ("portrait", "Portrait", "Portrait and commentary", vec!["lOnr"]),
        ("audio", "Commentary name", "Portrait and commentary", vec!["oazN", "KqtL"]),
    ]);
    out
}
pub(super) fn assignment_group(field: &str) -> Option<(&'static str, &'static str)> {
    assignment_specs(false).into_iter().chain(assignment_specs(true))
        .find(|(_, _, _, fields)| fields.contains(&field)).map(|(_, label, section, _)| (label, section))
}
pub(super) fn field_label(field: &str, goalie: bool) -> Option<&'static str> {
    match field {
        "pPaD" => Some("Head"), "KTDQ" => Some("Skin tone"), "CBfK" => Some("Right eye color"), "Zief" => Some("Left eye color"),
        "KqtL" => Some("Default commentary name"), "kGII" => Some("Custom mask"), "JGoJ" => Some("Custom blocker"),
        "hDYT" => Some("Custom trapper"), "gGPi" => Some("Custom pads"), "IqZs" => Some("Helmet variation"),
        _ => assignment_specs(goalie).into_iter().find(|s| s.3.contains(&field)).map(|s| s.1),
    }
}
fn fail(text: impl ToString) -> ApiError { ApiError::Workspace(text.to_string()) }
fn key(values: &[i64]) -> String { values.iter().map(ToString::to_string).collect::<Vec<_>>().join(":") }

fn asset_status(root: Option<&str>, assignment: &str, value: i64) -> &'static str {
    let relative = match assignment {
        "head" => format!("rendering/player/player_{value}/head_{value}.rx2"),
        "portrait" => format!("fe/ion/artassets/playerheads/p{value}.big"),
        _ => return "Not verified in game source",
    };
    let Some(root) = root.map(std::path::Path::new).filter(|p| p.is_dir()) else { return "Not verified in game source"; };
    if root.join(&relative).is_file() { return "Available in game source"; }
    // Only a receipt proving this member was written justifies a Missing warning.
    if let Ok(files) = std::fs::read_dir(root) {
        for file in files.flatten().filter(|f| f.file_name().to_string_lossy().ends_with(".nhlms-extracted.json")) {
            let Some(receipt) = std::fs::read(file.path()).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()) else { continue; };
            if receipt["members"].as_array().is_some_and(|members| members.iter().any(|m| m["name"].as_str() == Some(&relative) && m["outcome"] == "written")) {
                return "Missing from extracted game source";
            }
        }
    }
    "Not verified in game source"
}

impl Registry {
    fn assignment_source(&self, id: u32) -> Result<(Vec<u8>, Option<String>), ApiError> {
        let projects = self.projects.lock().unwrap();
        let binding = projects.bindings.get(&id).ok_or_else(|| fail("Open a Studio project first."))?;
        let m = &binding.store.manifest;
        if m.game_id != "nhl07" || m.platform != "xbox360" && m.platform != "xbox" {
            return Err(fail("These assignments are qualified for NHL 07 Xbox 360 only."));
        }
        let source = m.source.components.iter().find(|c| c.role == "main_db").ok_or_else(|| fail("The original database is missing."))?;
        Ok((std::fs::read(binding.store.path(&source.baseline)).map_err(fail)?, m.destinations.assets_dir.clone().or(m.destinations.install_dir.clone())))
    }

    pub fn player_assignments(&self, id: u32, player_id: u64) -> Result<Vec<PlayerAssignment>, ApiError> {
        let (baseline, root) = self.assignment_source(id)?;
        self.with_workspace(id, |ws| {
            let session = ws.primary();
            let current = session.player_view(player_id)?.ok_or_else(|| fail("Player not found."))?;
            let db = tdb_core::TdbFile::parse(&baseline).map_err(fail)?;
            let model = RosterModel::build(&db).map_err(fail)?;
            let table = db.table_canonical("ubPc").ok_or_else(|| fail("Player database is missing."))?;
            // Read original names directly: no rating/formula calculation for this catalogue.
            let name_field = |name| table.fields.iter().find(|f| session.project().meta().field(&table.id.raw(), &f.id.raw()).is_some_and(|m| m.name == name));
            let first = name_field("firstname"); let last = name_field("lastname");
            let player_name = |record| {
                let read = |f: Option<&tdb_core::FieldDescriptor>| f.and_then(|f| db.read_inline_str(table, record, f)).map(|s| String::from_utf8_lossy(s).into_owned()).unwrap_or_default();
                format!("{} {}", read(first), read(last))
            };
            let goalie = current.kind == "goalie";
            let mut result = Vec::new();
            for (name, label, section, fields) in assignment_specs(goalie) {
                let values = |record| -> Result<Vec<i64>, ApiError> {
                    fields.iter().map(|id| {
                        let f = table.fields.iter().find(|f| f.id.canonical(db.endian) == *id).ok_or_else(|| fail("Assignment field missing."))?;
                        let raw = db.read_u64(table, record, f).ok_or_else(|| fail("Assignment value missing."))?;
                        Ok(session.project().meta().field(&table.id.raw(), &f.id.raw()).map(|m| m.decode_raw(raw)).unwrap_or(raw as i64))
                    }).collect()
                };
                let current_values: Vec<_> = fields.iter().map(|id| current.all_fields.iter().find(|f| f.studio_field == *id).and_then(|f| f.decoded).ok_or_else(|| fail("Current assignment missing."))).collect::<Result<_, _>>()?;
                let mut choices: BTreeMap<String, (Vec<String>, usize)> = BTreeMap::new();
                for p in &model.players {
                    if section == "Equipment" && (p.goalie_ai.is_some()) != goalie { continue; }
                    let item = choices.entry(key(&values(p.bio_record)?)).or_default();
                    item.1 += 1;
                    if item.0.len() < 3 {
                        item.0.push(player_name(p.bio_record));
                    }
                }
                let mut options: Vec<_> = choices.into_iter().map(|(key, (names, users))| AssignmentOption { label: format!("{} (option {})", names.join(", "), key), key, users }).collect();
                options.sort_by(|a,b| a.label.cmp(&b.label));
                let selected = key(&current_values);
                if !options.iter().any(|o| o.key == selected) { options.insert(0, AssignmentOption { key: selected.clone(), label: format!("Current custom assignment ({selected})"), users: 0 }); }
                result.push(PlayerAssignment { key: name.into(), label: label.into(), section: section.into(), current: selected, options,
                    availability: asset_status(root.as_deref(), name, current_values[0]).into() });
            }
            Ok(result)
        })
    }

    pub fn set_player_assignment(&self, id: u32, player_id: u64, assignment: &str, choice: &str) -> Result<OpOutcomeDto, ApiError> {
        // Validate against original combinations, never accept arbitrary client-supplied cells.
        let catalogue = self.player_assignments(id, player_id)?;
        let selected = catalogue.iter().find(|a| a.key == assignment).ok_or_else(|| fail("This assignment does not apply to this player."))?;
        if !selected.options.iter().any(|o| o.key == choice && o.users > 0) { return Err(fail("Choose an existing NHL 07 assignment.")); }
        let values = choice.split(':').map(|v| v.parse::<i64>().map_err(fail)).collect::<Result<Vec<_>,_>>()?;
        let (_, root) = self.assignment_source(id)?;
        if values.first().is_some_and(|value| asset_status(root.as_deref(), assignment, *value).starts_with("Missing")) {
            return Err(fail("This asset is missing from the extracted source. Restore or relink the game source before selecting it."));
        }
        self.with_workspace(id, |ws| {
            let player = ws.primary().player_view(player_id)?.ok_or_else(|| fail("Player not found."))?;
            let (_, _, _, fields) = assignment_specs(player.kind == "goalie").into_iter().find(|s| s.0 == assignment).ok_or_else(|| fail("Assignment is not applicable."))?;
            if fields.len() != values.len() { return Err(fail("Incomplete assignment.")); }
            let cells = fields.into_iter().zip(values).map(|(f,v)| ("ubPc".into(), player.bio_record, f.into(), FieldValue::Int(v))).collect::<Vec<_>>();
            Ok(convert::outcome_dto(ws.set_package_fields(&cells)?))
        })
    }
}




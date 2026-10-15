//! Selective, undoable restoration from the project's immutable creation baseline.
use super::Registry;
use crate::{convert, dto::OpOutcomeDto, error::ApiError};
use studio_domain::{FieldRaw, FieldValue};
use studio_domain::RosterModel;

impl Registry {
    pub fn project_revert_change(&self, id: u32, change_id: &str) -> Result<OpOutcomeDto, ApiError> {
        if !self.project_overview(id)?.changes.iter().any(|c| c.id == change_id) {
            return Err(ApiError::Workspace("This edit is no longer present. Refresh Project Overview.".into()));
        }
        let baseline = {
            let projects = self.projects.lock().unwrap();
            let binding = projects.bindings.get(&id).ok_or_else(|| ApiError::Workspace("Open a project first.".into()))?;
            let component = binding.store.manifest.source.components.iter().find(|c| c.role == "main_db")
                .ok_or_else(|| ApiError::Workspace("This project has no original database.".into()))?;
            std::fs::read(binding.store.path(&component.baseline)).map_err(|e| ApiError::Workspace(e.to_string()))?
        };
        self.with_workspace(id, |ws| {
            if change_id == "organization" { return Ok(convert::outcome_dto(ws.restore_competition(&baseline)?)); }
            if let Some(table) = change_id.strip_prefix("schedule:") {
                let original = ws.primary().schedule_model_from_bytes(&baseline, table)?;
                let current = ws.primary().schedule_model(table)?;
                if original.slots.fingerprint != current.slots.fingerprint {
                    return Err(ApiError::Workspace("Restore the league membership changes before restoring this schedule.".into()));
                }
                return Ok(convert::outcome_dto(ws.replace_schedules(&[(table.into(), original.rows)])?));
            }
            let session = ws.primary();
            let project = session.project();
            let diff = studio_domain::pending::pending_changes(&baseline, session.bytes(), project.meta(), project.enums(), None, usize::MAX);
            if let Some(error) = diff.error { return Err(ApiError::Workspace(error)); }
            let old_db = tdb_core::TdbFile::parse(&baseline).map_err(|e| ApiError::Workspace(e.to_string()))?;
            let old = RosterModel::build(&old_db).map_err(|e| ApiError::Workspace(e.to_string()))?;
            let now = session.model()?;
            let target = diff.cells.iter().find(|c| format!("{}:{}:{}", c.studio_table, c.record, c.studio_field) == change_id);
            let fa_player = change_id.strip_prefix("free-agent:").and_then(|s| s.parse::<u64>().ok());
            if target.is_none() && fa_player.is_none() {
                return Err(ApiError::Workspace("This file or record change must currently be reverted in its editor.".into()));
            }
            // A roster move/release/sign includes the player's biography, occupied slots and
            // explicit free-agent membership. Restoring just one flag would corrupt that link.
            let roster_player = fa_player.or_else(|| target.and_then(|c| {
                if c.studio_table == "ubPc" && c.studio_field == "dXSB" {
                    old.players.iter().find(|p| p.bio_record == c.record).map(|p| p.player_id)
                } else if c.studio_table == "eGlu" && matches!(c.studio_field.as_str(), "XSWT" | "dXSB") {
                    old.slots.iter().chain(now.slots.iter()).find(|s| s.record == c.record && s.player_id.is_some()).and_then(|s| s.player_id)
                } else { None }
            }));
            let mut roster_players: std::collections::HashSet<u64> = roster_player.into_iter().collect();
            // Trades and subsequent moves can connect several occupied slots. Restore that
            // connected roster change as one operation, never overwrite a different player.
            let roster_rows = loop {
                let rows: std::collections::HashSet<u32> = old.slots.iter().chain(now.slots.iter())
                    .filter(|s| s.player_id.is_some_and(|p| roster_players.contains(&p))).map(|s| s.record).collect();
                let count = roster_players.len();
                roster_players.extend(old.slots.iter().chain(now.slots.iter()).filter(|s| rows.contains(&s.record)).filter_map(|s| s.player_id));
                if roster_players.len() == count { break rows; }
            };
            let team = target.filter(|c| c.studio_table == "eGlu")
                .and_then(|c| now.slots.iter().find(|s| s.record == c.record)).map(|s| s.team_id);
            let line_fields = team.and_then(|t| session.lines_view(t).ok().flatten()).map(|v| v.slot_fields.into_iter().map(|f| f.studio_field).collect::<Vec<_>>()).unwrap_or_default();
            let jersey_player = target.and_then(|c| {
                if c.studio_field != "sVRt" { return None; }
                if c.studio_table == "ubPc" { old.players.iter().find(|p| p.bio_record == c.record).map(|p| p.player_id) }
                else if c.studio_table == "eGlu" { now.slots.iter().find(|s| s.record == c.record).and_then(|s| s.player_id) } else { None }
            });
            let primary_team = jersey_player.and_then(|p| session.player_view(p).ok().flatten()).and_then(|p| p.header.primary_team_id);
            let jersey_team = team.or(primary_team);
            let mut cells = Vec::new();
            for c in &diff.cells {
                let mut selected = target.is_some_and(|t| c.studio_table == t.studio_table && c.record == t.record && c.studio_field == t.studio_field);
                for player in &roster_players {
                    selected |= c.studio_table == "eGlu" && roster_rows.contains(&c.record);
                    selected |= c.studio_table == "ubPc" && matches!(c.studio_field.as_str(), "dXSB" | "sVRt") && old.players.iter().any(|p| p.player_id == *player && p.bio_record == c.record);
                }
                if let Some(t) = target.filter(|t| t.studio_table == "eGlu") {
                    // A line position and captaincy have multiple participating roster rows.
                    if t.studio_field == "mCcl" || line_fields.contains(&t.studio_field) {
                        selected |= c.studio_table == "eGlu" && c.studio_field == t.studio_field && now.slots.iter().any(|s| s.record == c.record && Some(s.team_id) == team);
                    }
                    if t.studio_field == "hSZj" {
                        selected |= c.studio_table == "eGlu" && c.record == t.record && (line_fields.contains(&c.studio_field) || c.studio_field == "mCcl");
                    }
                }
                if let Some(player) = jersey_player {
                    selected |= c.studio_field == "sVRt" && ((c.studio_table == "ubPc" && jersey_team == primary_team && old.players.iter().any(|p| p.player_id == player && p.bio_record == c.record))
                        || (c.studio_table == "eGlu" && now.slots.iter().any(|s| s.record == c.record && s.player_id == Some(player) && Some(s.team_id) == jersey_team)));
                }
                if project.game_id() == Some("nhl07") {
                    if let Some(t) = target.filter(|t| t.studio_table == "ubPc") {
                        let goalie = now.players.iter().find(|p| p.bio_record == t.record).is_some_and(|p| p.goalie_ai.is_some());
                        // Head complexion, paired audio and custom gear flags restore together.
                        if let Some(spec) = super::player_assignments::assignment_specs(goalie).iter().find(|s| s.3.contains(&t.studio_field.as_str())) {
                            selected |= c.studio_table == "ubPc" && c.record == t.record && spec.3.contains(&c.studio_field.as_str());
                        }
                    }
                }
                if selected && c.studio_table != "VoEQ" {
                    let value = match &c.before.raw { FieldRaw::Text(v) => FieldValue::Text(v.clone()), FieldRaw::Int(_) => FieldValue::Int(c.before.decoded.ok_or_else(|| ApiError::Workspace("The original value cannot be decoded.".into()))?) };
                    cells.push((c.studio_table.clone(), c.record, c.studio_field.clone(), value));
                }
            }
            // Unlike ordinary scalar admission, line placement also has positional rules.
            // Check the proposed roster before committing any baseline cells.
            let affected_teams: std::collections::HashSet<_> = cells.iter().filter(|c| c.0 == "eGlu")
                .filter_map(|c| now.slots.iter().find(|s| s.record == c.1).map(|s| s.team_id)).collect();
            for team in affected_teams {
                let Some(before) = session.lines_view(team)? else { continue };
                let mut after = before.clone();
                for (table, record, field, value) in &cells {
                    if table != "eGlu" { continue; }
                    let FieldValue::Int(value) = value else { continue };
                    let Some(entry) = after.entries.iter_mut().find(|e| e.slot_record == *record) else { continue };
                    if field == "hSZj" { entry.dressed = Some(*value != 0); }
                    else if field == "mCcl" { entry.captaincy = Some(*value as u64); }
                    else if after.slot_fields.iter().any(|f| f.studio_field == *field) {
                        entry.slots.retain(|f| f != field);
                        if *value != 0 { entry.slots.push(field.clone()); }
                    }
                }
                for field in &after.slot_fields {
                    let count = after.entries.iter().filter(|e| e.slots.contains(&field.studio_field)).count();
                    let prior_count = before.entries.iter().filter(|e| e.slots.contains(&field.studio_field)).count();
                    if count > 1 && count > prior_count {
                        return Err(ApiError::Workspace("A restored line position is now occupied. Revert that line assignment first.".into()));
                    }
                }
                if roster_players.is_empty() {
                    for entry in &after.entries {
                        let prior = before.entries.iter().find(|e| e.slot_record == entry.slot_record).unwrap();
                        if entry.dressed == Some(false) && (entry.slots.iter().any(|s| !prior.slots.contains(s)) || entry.captaincy.unwrap_or(0) > prior.captaincy.unwrap_or(0)) {
                            return Err(ApiError::Workspace("Dress this player before restoring their line assignment or captaincy.".into()));
                        }
                    }
                }
            }
            let pool = if !roster_players.is_empty() {
                match (studio_domain::explicit_free_agents(&baseline).map_err(|e| ApiError::Workspace(e.to_string()))?, studio_domain::explicit_free_agents(session.bytes()).map_err(|e| ApiError::Workspace(e.to_string()))?) {
                    (Some(before), Some(mut after)) => {
                        after.retain(|p| !roster_players.contains(p));
                        after.extend(before.iter().copied().filter(|p| roster_players.contains(p)));
                        Some(after)
                    }, _ => None,
                }
            } else { None };
            Ok(convert::outcome_dto(ws.set_project_revert(&cells, pool.as_deref())?))
        })
    }
}

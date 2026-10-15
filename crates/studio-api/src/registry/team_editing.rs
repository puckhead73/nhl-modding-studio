use super::Registry;
use crate::{convert, dto::OpOutcomeDto, error::ApiError};
use std::collections::{BTreeMap, HashSet};

impl Registry {
    /// A line swap, best-lines assignment or captain replacement is one undoable edit.
    pub fn set_team_roster(&self, id: u32, team: u64, writes: &[(u32, String, i64)]) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            let mut view = ws.display().lines_view(team)?.ok_or_else(|| ApiError::Workspace("Choose an existing team.".into()))?;
            if writes.is_empty() { return Err(ApiError::Workspace("No roster changes were selected.".into())); }
            let mut rows: BTreeMap<u32, Vec<(String, i64)>> = BTreeMap::new();
            let mut seen = HashSet::new();
            let mut touched = HashSet::new();
            for (record, field, value) in writes {
                if !seen.insert((*record, field.clone())) { return Err(ApiError::Workspace("The same roster field was supplied twice.".into())); }
                let entry = view.entries.iter_mut().find(|e| e.slot_record == *record && e.player_id.is_some())
                    .ok_or_else(|| ApiError::Workspace("The selected player is not on this team's roster.".into()))?;
                if field == "mCcl" {
                    if !(0..=2).contains(value) || (*value != 0 && entry.dressed == Some(false)) {
                        return Err(ApiError::Workspace("Choose a dressed player for captain or alternate.".into()));
                    }
                    entry.captaincy = Some(*value as u64);
                } else {
                    let slot = view.slot_fields.iter().find(|s| s.studio_field == *field)
                        .ok_or_else(|| ApiError::Workspace("Only line assignments and captaincy can be changed here.".into()))?;
                    if !matches!(value, 0 | 1) { return Err(ApiError::Workspace("Line assignments must be on or off.".into())); }
                    if *value == 1 {
                        if entry.dressed == Some(false) { return Err(ApiError::Workspace("Dress the player before assigning a line.".into())); }
                        if (slot.name == "g1" || slot.name == "g2") != (entry.position_label == "G") {
                            return Err(ApiError::Workspace("Goalies can only fill goalie slots; skaters can only fill skater slots.".into()));
                        }
                        if !entry.slots.contains(field) { entry.slots.push(field.clone()); }
                    } else { entry.slots.retain(|s| s != field); }
                    touched.insert(field.clone());
                }
                rows.entry(*record).or_default().push((field.clone(), *value));
            }
            for field in touched {
                if view.entries.iter().filter(|e| e.slots.contains(&field)).count() > 1 {
                    return Err(ApiError::Workspace("A line slot cannot contain two players. Replace or swap its current player.".into()));
                }
            }
            if writes.iter().any(|(_, f, _)| f == "mCcl") && view.entries.iter().filter(|e| e.captaincy == Some(1)).count() > 1 {
                return Err(ApiError::Workspace("A team can have only one captain.".into()));
            }
            Ok(convert::outcome_dto(ws.set_cells_int_rows("eGlu", &rows.into_iter().collect::<Vec<_>>())?))
        })
    }
}

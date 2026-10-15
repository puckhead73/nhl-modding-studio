//! Project-wide review: immutable source baseline versus current working state, never the
//! session's last-save baseline. Saving therefore cannot erase the overview.
use super::Registry;
use crate::{
    convert,
    dto::{ProjectDetailsDto, ProjectOverviewChangeDto, ProjectOverviewDto},
    error::ApiError,
};
use studio_domain::project_store::{ProjectDetails, ProjectStore};

fn details(store: &ProjectStore) -> ProjectDetailsDto {
    let d = &store.manifest.details;
    ProjectDetailsDto {
        name: if d.name.is_empty() {
            store
                .dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into()
        } else {
            d.name.clone()
        },
        version: if d.version.is_empty() {
            "1.0.0".into()
        } else {
            d.version.clone()
        },
        author: d.author.clone(),
        description: d.description.clone(),
    }
}

impl Registry {
    pub fn project_set_details(
        &self,
        id: u32,
        d: ProjectDetailsDto,
    ) -> Result<ProjectDetailsDto, ApiError> {
        if d.name.trim().is_empty() || d.version.trim().is_empty() {
            return Err(ApiError::Workspace("Name and version are required.".into()));
        }
        if d.name.len() > 120
            || d.version.len() > 40
            || d.author.len() > 200
            || d.description.len() > 4000
        {
            return Err(ApiError::Workspace("Project details are too long.".into()));
        }
        let mut projects = self.projects.lock().unwrap();
        let b = projects
            .bindings
            .get_mut(&id)
            .ok_or_else(|| ApiError::Workspace("Open a Studio project first.".into()))?;
        let prior = b.store.manifest.details.clone();
        b.store.manifest.details = ProjectDetails {
            name: d.name.trim().into(),
            version: d.version.trim().into(),
            author: d.author.trim().into(),
            description: d.description,
        };
        if let Err(e) = b.store.save_manifest() {
            b.store.manifest.details = prior;
            return Err(e.into());
        }
        Ok(details(&b.store))
    }

    pub fn project_overview(&self, id: u32) -> Result<ProjectOverviewDto, ApiError> {
        // Release the project lock before acquiring the workspace lock (same order as save).
        let (mut out, baseline, dictionary_baseline) = {
            let projects = self.projects.lock().unwrap();
            let b = projects
                .bindings
                .get(&id)
                .ok_or_else(|| ApiError::Workspace("Open a Studio project first.".into()))?;
            let s = &b.store;
            let mut out = ProjectOverviewDto {
                details: details(s),
                dirty: false,
                changes: vec![],
                notices: vec![],
            };
            for f in &s.manifest.staged_files {
                let path = s.working_dir().join("overlay").join(&f.relative);
                let bytes = std::fs::read(&path).map_err(|e| {
                    ApiError::Workspace(format!("Cannot review {}: {e}", f.relative))
                })?;
                let hash = studio_domain::baselines::sha256_hex(&bytes);
                if f.prior.as_ref().is_some_and(|p| p.sha256 == hash) {
                    continue;
                }
                out.changes.push(ProjectOverviewChangeDto {
                    id: format!("asset:{}", f.relative),
                    groups: vec!["Shared assets".into()],
                    label: f.relative.clone(),
                    before: if f.prior.is_some() {
                        "Original asset"
                    } else {
                        "Not present"
                    }
                    .into(),
                    after: "Project asset".into(),
                    route: Some("/app/assets/files".into()),
                });
            }
            // Count changed archive files once, not every historical member replacement.
            for archive in &s.manifest.materialized_archives {
                let current = studio_domain::tdb_txn::FileIdentity::of_file(&s.working_dir().join(&archive.relative))
                    .map_err(|e| ApiError::Workspace(format!("Cannot review {}: {e}", archive.relative)))?;
                if current.sha256 == archive.prior.sha256 { continue; }
                out.changes.push(ProjectOverviewChangeDto {
                    id: format!("archive:{}", archive.relative), groups: vec!["Shared assets".into(), "Archives".into()],
                    label: archive.relative.clone(), before: "Original archive".into(), after: "Modified archive".into(),
                    route: Some("/app/assets/files".into()),
                });
            }
            if !s.manifest.member_edits.is_empty() {
                out.notices.push("Archive summaries show changed files. Individual member comparisons and asset ownership will be added with their editors.".into());
            }
            let read = |role: &str| -> Result<Option<Vec<u8>>, ApiError> {
                s.manifest
                    .source
                    .components
                    .iter()
                    .find(|c| c.role == role)
                    .map(|c| {
                        std::fs::read(s.path(&c.baseline)).map_err(|e| {
                            ApiError::Workspace(format!("Cannot read the original {role}: {e}"))
                        })
                    })
                    .transpose()
            };
            (out, read("main_db")?, read("dictionary")?)
        };
        if !self.has_session(id) {
            return Ok(out);
        }
        self.with_workspace(id, |ws| {
            out.dirty = ws.is_dirty();
            let session = ws.primary();
            if let Some(base) = baseline {
                let project = session.project();
                let original_db = tdb_core::TdbFile::parse(&base).map_err(|e| ApiError::Workspace(e.to_string()))?;
                let leagues = session.league_catalog().ok();
                let diff = studio_domain::pending::pending_changes(&base, session.bytes(), project.meta(), project.enums(), leagues.as_ref(), usize::MAX);
                let review = convert::pending_file_dto("primary", "Database", "", &diff);
                if let Some(error) = review.error { return Err(ApiError::Workspace(error)); }
                let players = session.player_summaries()?;
                let teams = session.team_summaries()?;
                let model = session.model()?;
                let db = tdb_core::TdbFile::parse(session.bytes()).map_err(|e| ApiError::Workspace(e.to_string()))?;
                let old_pool = studio_domain::explicit_free_agents(&base).map_err(|e| ApiError::Workspace(e.to_string()))?;
                let new_pool = studio_domain::explicit_free_agents(session.bytes()).map_err(|e| ApiError::Workspace(e.to_string()))?;
                if let (Some(before), Some(after)) = (&old_pool, &new_pool) {
                    for p in &players {
                        let was = before.contains(&p.player_id);
                        let now = after.contains(&p.player_id);
                        if was == now { continue; }
                        out.changes.push(ProjectOverviewChangeDto {
                            id: format!("free-agent:{}", p.player_id),
                            groups: vec!["Free agency".into(), format!("{} {}", p.first_name, p.last_name)],
                            label: "Roster status".into(),
                            before: if was { "Free agent" } else { "Signed" }.into(),
                            after: if now { "Free agent" } else { "Signed" }.into(),
                            route: Some(format!("/app/database/players/{}?tab=player-data", p.player_id)),
                        });
                    }
                }
                let mut schedule_tables = std::collections::HashSet::new();
                let organization = project.game_id() == Some("nhl07") && review.cells.iter().any(|c| c.studio_table == "HqzU" && matches!(c.studio_field.as_str(), "xMjj" | "rasz" | "xgeX"));
                if organization {
                    out.changes.push(ProjectOverviewChangeDto { id:"organization".into(), groups:vec!["Leagues".into()], label:"League organization and schedules".into(), before:review.cells.iter().filter(|c|c.studio_table=="HqzU" && matches!(c.studio_field.as_str(),"xMjj"|"rasz"|"xgeX")).map(|c|format!("{} · {}: {}",c.entity,c.field_name,c.before)).collect::<Vec<_>>().join("; "), after:review.cells.iter().filter(|c|c.studio_table=="HqzU" && matches!(c.studio_field.as_str(),"xMjj"|"rasz"|"xgeX")).map(|c|format!("{} · {}: {}",c.entity,c.field_name,c.after)).collect::<Vec<_>>().join("; "), route:Some("/app/database/leagues".into()) });
                }
                if project.game_id() == Some("nhl07") {
                    for table in session.schedule_tables()? {
                        let Ok(current) = session.schedule_model(&table.studio_table) else { continue };
                        if !studio_domain::schedule_edit::supports_resize(&current) { continue; }
                        schedule_tables.insert(table.studio_table.clone());
                        let original = session.schedule_model_from_bytes(&base, &table.studio_table)?;
                        if original.rows == current.rows { continue; }
                        if organization {if let Some(change)=out.changes.iter_mut().find(|c|c.id=="organization"){change.after.push_str(&format!("; {}: {} → {} games",table.name,original.rows.len(),current.rows.len()));}continue;}
                        out.changes.push(ProjectOverviewChangeDto {
                            id: format!("schedule:{}", table.studio_table),
                            groups: vec![table.league_label.clone().unwrap_or_else(|| "League".into()), "Schedules".into()],
                            label: match table.name.as_str(){"nhlschedulelongleap"=>"NHL 82-game template (leap season)","nhlschedulelongnoleap"=>"NHL 82-game template", "nhlschedulemedium"=>"NHL 58-game template","nhlschedulesmall"=>"NHL 29-game template","intldelschedule"=>"DEL season","intlelitserienschedule"=>"Elitserien season","intlsmliigaschedule"=>"SM-liiga season", _=>&table.name}.into(),
                            before: format!("{} games", original.rows.len()),
                            after: format!("{} games — modified schedule", current.rows.len()),
                            route: table.league_code.map(|code| format!("/app/database/leagues/{code}")),
                        });
                    }
                }
                for c in review.cells {
                    if schedule_tables.contains(&c.studio_table) { continue; }
                    if organization && c.studio_table == "HqzU" && matches!(c.studio_field.as_str(), "xMjj" | "rasz" | "xgeX") { continue; }
                    if c.studio_table == "VoEQ" { continue; }
                    let player_id = match c.studio_table.as_str() {
                        "ubPc" => model.players.iter().find(|p| p.bio_record == c.record).map(|p| p.player_id),
                        "dSvy" => model.players.iter().find(|p| p.skater_ai == Some(c.record)).map(|p| p.player_id),
                        "mHuy" => model.players.iter().find(|p| p.goalie_ai == Some(c.record)).map(|p| p.player_id),
                        "xmja" => model.players.iter().find(|p| p.skater_equip == Some(c.record)).map(|p| p.player_id),
                        "fMVl" => model.players.iter().find(|p| p.goalie_equip == Some(c.record)).map(|p| p.player_id),
                        "eGlu" => model.slots.iter().find(|s| s.record == c.record).and_then(|s| s.player_id),
                        _ => None,
                    };
                    let player = player_id.and_then(|id| players.iter().find(|p| p.player_id == id));
                    let league = leagues.as_ref().and_then(|catalog| if c.studio_table == "fsLK" { catalog.leagues.iter().find(|l| l.record == Some(c.record)) } else { None });
                    let team = player.and_then(|p| p.team_id).and_then(|id| teams.iter().find(|t| t.team_id == id))
                        .or_else(|| if c.studio_table == "kOtt" { teams.iter().find(|t| t.record == c.record) } else { None })
                        .or_else(|| {
                            if c.studio_table != "EeYo" && !(c.studio_table == "StEO" && db.table_canonical("cubz").is_some()) { return None; }
                            let table = db.table_canonical(&c.studio_table)?;
                            let index = table.fields.iter().find(|f| f.id.canonical(db.endian) == "vfEq")?;
                            let id = db.read_u64(table, c.record, index)?;
                            teams.iter().find(|t| t.team_id == id)
                        })
                        .or_else(|| {
                            // NHL07 keeps team labels and league membership behind its thin
                            // exhibition-team index. Resolve the join rather than matching names.
                            if !matches!(c.studio_table.as_str(), "HqzU" | "fLEY") { return None; }
                            let source = db.table_canonical(&c.studio_table)?;
                            let key = source.fields.iter().find(|f| project.meta().field(&source.id.raw(), &f.id.raw()).is_some_and(|m| m.name == "index"))?;
                            let key = db.read_u64(source, c.record, key)?;
                            let index = db.table_canonical("kOtt")?;
                            teams.iter().find(|t| {
                                let read = |field| index.fields.iter().find(|f| f.id.canonical(db.endian) == field).and_then(|f| db.read_u64(index, t.record, f));
                                read("mCXg") == Some(key) && read("RREB").unwrap_or(0) == if c.studio_table == "fLEY" { 1 } else { 0 }
                            })
                        });
                    let mut groups = if let Some(t) = team { vec![t.league_label.clone().unwrap_or_else(|| "Other leagues".into()), t.name.clone()] }
                        else if player.is_some() { vec!["Free agents / unassigned".into()] }
                        else { vec!["Other database changes".into()] };
                    let route;
                    if let Some(p) = player {
                        let assignment = (project.game_id() == Some("nhl07") && c.studio_table == "ubPc").then(|| super::player_assignments::assignment_group(&c.studio_field)).flatten();
                        groups.extend(["Players".into(), format!("{} {}", p.first_name, p.last_name), assignment.map(|a| a.1).unwrap_or_else(|| match c.studio_table.as_str() { "eGlu" => "Roster", "dSvy" | "mHuy" => "Attributes", "xmja" | "fMVl" => "Equipment", _ => "Player data" }).into()]);
                        let tab = if assignment.is_some() { "equipment" } else { match c.studio_table.as_str() { "dSvy" | "mHuy" => "attributes", "xmja" | "fMVl" => "equipment", _ => "player-data" } };
                        route = Some(format!("/app/database/players/{}?tab={tab}", p.player_id));
                    } else if let Some(t) = team {
                        groups.push("Team changes".into());
                        route = Some(format!("/app/database/teams/{}", t.team_id));
                    } else if let Some(l) = league {
                        groups = vec![l.label.clone().unwrap_or_else(|| format!("League {}", l.code)), "League changes".into()];
                        route = Some(format!("/app/database/leagues/{}", l.code));
                    } else {
                        groups.extend([c.table_name.clone(), c.entity.clone()]);
                        route = None;
                    }
                    let (groups, route) = if c.studio_table == "cubz" {
                        // Arena metadata can be shared by several teams. Keep one shared
                        // entry and route to the shared arena editor.
                        let stock = db.table_canonical("cubz");
                        let key = stock.and_then(|t| t.fields.iter().find(|f| f.id.canonical(db.endian) == "vfEq").and_then(|f| db.read_u64(t, c.record, f)));
                        let selector = db.table_canonical("StEO");
                        let owner = selector.and_then(|t| {
                            let f = |id| t.fields.iter().find(|f| f.id.canonical(db.endian) == id);
                            let (arena, kind, index) = (f("buDa")?, f("RREB")?, f("vfEq")?);
                            (0..t.current_records as u32).find_map(|r| {
                                if db.read_u64(t, r, arena) == key && db.read_u64(t, r, kind) == Some(0) { db.read_u64(t, r, index) } else { None }
                            })
                        });
                        (vec!["Shared changes".into(), "Arenas".into(), owner.and_then(|id| session.arena_view(id).ok().flatten()).and_then(|a| a.all_fields.into_iter().find(|f| f.name == "arenaname")).map(|f| f.display).unwrap_or_else(||c.entity.clone())], owner.map(|id| format!("/app/database/arenas/{id}")))
                    } else { (groups, route) };
                    let team_value = |raw: &str, display: String, pool: &Option<Vec<u64>>| {
                        if c.studio_field == "dXSB" && c.studio_table == "ubPc" && player_id.is_some_and(|pid| pool.as_ref().is_some_and(|p| p.contains(&pid))) { return "Free agent".into(); }
                        if c.studio_field == "dXSB" || c.field_name == "team" || c.field_name == "teamid" {
                            raw.parse::<u64>().ok().and_then(|id| teams.iter().find(|t| t.team_id == id)).map(|t| t.name.clone()).unwrap_or(display)
                        } else { display }
                    };
                    let label = if project.game_id() == Some("nhl07") && c.studio_table == "ubPc" {
                        super::player_assignments::field_label(&c.studio_field, player.is_some_and(|p| p.is_goalie)).unwrap_or(&c.field_name).to_string()
                    } else { c.field_name.clone() };
                    let assignment_value = |raw: &str, fallback: String| {
                        if project.game_id() != Some("nhl07") || c.studio_table != "ubPc" || super::player_assignments::assignment_group(&c.studio_field).is_none() { return fallback; }
                        let Some(value) = raw.parse::<u64>().ok() else { return fallback; };
                        let donor = original_db.table_canonical("ubPc").and_then(|t| {
                            let field = t.fields.iter().find(|f| f.id.canonical(original_db.endian) == c.studio_field)?;
                            model.players.iter().filter(|p| player.is_none_or(|owner| owner.is_goalie == p.goalie_ai.is_some()))
                                .find(|p| original_db.read_u64(t, p.bio_record, field) == Some(value))
                                .and_then(|p| players.iter().find(|s| s.player_id == p.player_id))
                        });
                        donor.map(|p| format!("Option {raw} — {} {}", p.first_name, p.last_name)).unwrap_or_else(|| format!("Custom option {raw}"))
                    };
                    let before = assignment_value(&c.before_raw, team_value(&c.before_raw, c.before, &old_pool));
                    let after = assignment_value(&c.after_raw, team_value(&c.after_raw, c.after, &new_pool));
                    out.changes.push(ProjectOverviewChangeDto { id: format!("{}:{}:{}", c.studio_table, c.record, c.studio_field), groups, label, before, after, route });
                }
                for t in review.tables {
                    if schedule_tables.contains(&t.studio_table) { continue; }
                    if t.studio_table == "VoEQ" { continue; }
                    out.changes.push(ProjectOverviewChangeDto { id: format!("table:{}", t.studio_table), groups: vec!["Other database changes".into(), t.table_name], label: "Records".into(), before: t.before_records.map(|n| n.to_string()).unwrap_or_else(|| "Not present".into()), after: t.note, route: None });
                }
                if review.integrity.other_bytes > 0 { out.notices.push(format!("Some database changes could not be identified ({} bytes). They are not included in the edit summary below.", review.integrity.other_bytes)); }
            }
            if let Some(base) = dictionary_baseline {
                let current = ws.dictionary_edit().map(|e| e.output.as_slice()).or_else(|| ws.dictionary().map(|d| d.bytes.as_slice()));
                if let Some(current) = current {
                    let a = studio_domain::tdb_loc::LocDictionary::parse(&base).map_err(|e| ApiError::Workspace(e.to_string()))?;
                    let b = studio_domain::tdb_loc::LocDictionary::parse(current).map_err(|e| ApiError::Workspace(e.to_string()))?;
                    for i in 0..a.rows.len().max(b.rows.len()) {
                        let before = a.rows.get(i); let after = b.rows.get(i);
                        if before.map(|r| (&r.key, &r.text)) == after.map(|r| (&r.key, &r.text)) { continue; }
                        out.changes.push(ProjectOverviewChangeDto { id: format!("loc:{i}"), groups: vec!["Shared changes".into(), "Localization".into()], label: after.or(before).and_then(|r| r.key.clone()).unwrap_or_else(|| format!("Text {}", i + 1)), before: before.and_then(|r| r.text.clone()).unwrap_or_else(|| "Not present".into()), after: after.and_then(|r| r.text.clone()).unwrap_or_else(|| "Removed".into()), route: None });
                    }
                } else { out.notices.push("Localization could not be reviewed because its file is not loaded.".into()); }
            }
            if !ws.asset_member_edits().is_empty() || !ws.settings_csv_edits().is_empty() {
                out.notices.push("There are unsaved asset or settings edits. Detailed summaries for these workflows are still pending.".into());
            }
            Ok(out)
        })
    }
}



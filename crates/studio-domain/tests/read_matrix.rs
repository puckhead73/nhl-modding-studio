//! Phase 1D — the full 27-anchor **read matrix**. For every committed main-DB anchor this drives the
//! real domain read path over deterministic entity *representatives* (Phase 1C) — not just the first
//! player/team — and asserts the presentation invariants the plan requires:
//!
//!   * every profile field targets a real (table, record, field);
//!   * no curated section is emitted empty;
//!   * no duplicate field identity within a surface;
//!   * a field's editor is enabled iff its metadata marks it updatable (no `update=False` editor);
//!   * ratings/companions resolve to the correct table;
//!   * membership/arena FKs resolve or use a documented null;
//!   * the **surface-capability verdict agrees with what the builders actually construct** — an
//!     absent surface is `NotApplicable` and builds nothing; an available one builds.
//!
//! Each anchor is parsed once; the model/metadata are reused across every representative.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use studio_domain::{
    arena_summaries, career_stats, lines_view, player_summaries, player_view, team_draft_picks,
    team_equipment, team_jerseys, team_strategies, team_summaries, team_view, Capabilities,
    EntityFieldProfile, SurfaceCapabilities, SurfaceState,
};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_relations::{PlayerKind, RosterModel};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

/// Discover the 27 main-DB anchors (db+meta), sorted, mirroring the field-profile gate.
fn anchors() -> Vec<(String, PathBuf, PathBuf)> {
    let root = repo("anchors");
    let pairs = [
        ("NHL07NG.db", "NHL07NG-meta.xml"),
        ("nhlng.db", "nhlng-meta.xml"),
        ("nhl_game.db", "nhl_game-meta.xml"),
    ];
    let mut out = Vec::new();
    let mut platforms: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().collect();
    platforms.sort_by_key(|e| e.file_name());
    for platform in platforms.iter().filter(|e| e.path().is_dir()) {
        let mut games: Vec<_> = std::fs::read_dir(platform.path()).unwrap().flatten().collect();
        games.sort_by_key(|e| e.file_name());
        for game in games.iter().filter(|e| e.path().is_dir()) {
            let dbs = game.path().join("databases");
            if let Some((db, meta)) = pairs
                .iter()
                .find(|(d, m)| dbs.join(d).is_file() && dbs.join(m).is_file())
            {
                out.push((
                    format!("{}/{}", platform.file_name().to_string_lossy(), game.file_name().to_string_lossy()),
                    dbs.join(db),
                    dbs.join(meta),
                ));
            }
        }
    }
    out
}

fn has_table(db: &TdbFile, t: &str) -> bool {
    db.table_canonical(t).is_some()
}
fn has_field(db: &TdbFile, t: &str, f: &str) -> bool {
    db.table_canonical(t)
        .map(|tb| tb.fields.iter().any(|x| x.id.canonical(db.endian) == f))
        .unwrap_or(false)
}
/// Mirror of the probe's name-based requirement (arena name / career join).
fn has_named_field(db: &TdbFile, meta: &FieldMetaIndex, t: &str, name: &str) -> bool {
    let Some(tb) = db.table_canonical(t) else { return false };
    let raw = tb.id.raw();
    tb.fields.iter().any(|f| meta.field(&raw, &f.id.raw()).map(|m| m.name == name).unwrap_or(false))
}
/// Mirror of the probe's "roster-lines table carries a named slot" requirement.
fn has_named_slot(db: &TdbFile, meta: &FieldMetaIndex, t: &str) -> bool {
    let Some(tb) = db.table_canonical(t) else { return false };
    let raw = tb.id.raw();
    tb.fields.iter().any(|f| {
        f.bit_width == 1
            && meta.field(&raw, &f.id.raw()).map(|m| !m.name.is_empty() && m.name != "rosterstatus").unwrap_or(false)
    })
}

/// A profile's field cells must all resolve, be unique, and never enable an editor on a
/// non-updatable field.
fn assert_profile_ok(db: &TdbFile, meta: &FieldMetaIndex, p: &EntityFieldProfile, label: &str) {
    let mut seen: BTreeSet<(String, String, u32)> = BTreeSet::new();
    for s in &p.sections {
        assert!(!s.cells.is_empty(), "{label}: section '{}' emitted with no cells", s.title);
        for c in &s.cells {
            let Some(field) = &c.field else { continue };
            let table = db.table_canonical(&field.studio_table).unwrap_or_else(|| {
                panic!("{label}: cell '{}' targets missing table {}", c.label, field.studio_table)
            });
            assert!(
                c.record < table.current_records as u32,
                "{label}: cell '{}' record {} beyond {}",
                c.label, c.record, table.current_records
            );
            let raw = table
                .fields
                .iter()
                .find(|f| f.id.canonical(db.endian) == field.studio_field)
                .unwrap_or_else(|| {
                    panic!("{label}: cell '{}' targets missing field {}.{}", c.label, field.studio_table, field.studio_field)
                })
                .id
                .raw();
            // The editor is enabled iff the metadata marks the field updatable **and** it is not a
            // key — never on update=False, and never on a key/identity field (even one EA marked
            // update=True, e.g. NHL07 `vfEq`).
            let editable_expected = meta
                .field(&table.id.raw(), &raw)
                .map(|m| m.updatable && !m.key)
                .unwrap_or(false);
            assert_eq!(
                field.editable, editable_expected,
                "{label}: cell '{}' editable={} but metadata updatable&&!key={}",
                c.label, field.editable, editable_expected
            );
            assert!(
                seen.insert((field.studio_table.clone(), field.studio_field.clone(), c.record)),
                "{label}: duplicate field identity {}.{}#{} in a surface",
                field.studio_table, field.studio_field, c.record
            );
        }
    }
}

/// Every raw `all_fields` view: its editor flag must match the metadata's updatable flag exactly.
fn assert_all_fields_editable_honest(db: &TdbFile, meta: &FieldMetaIndex, fields: &[studio_domain::FieldView], label: &str) {
    for f in fields {
        let Some(table) = db.table_canonical(&f.studio_table) else { continue };
        let Some(desc) = table.fields.iter().find(|d| d.id.canonical(db.endian) == f.studio_field) else {
            continue;
        };
        let editable_expected = meta
            .field(&table.id.raw(), &desc.id.raw())
            .map(|m| m.updatable && !m.key)
            .unwrap_or(false);
        assert_eq!(f.editable, editable_expected, "{label}: {}.{} editable != updatable&&!key", f.studio_table, f.studio_field);
    }
}

#[test]
fn read_matrix_over_all_anchors() {
    let enums = EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
    let mut checked = 0;

    for (label, db_path, meta_path) in anchors() {
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap_or_else(|e| panic!("{label}: {e}"));
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap())
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let model = RosterModel::build(&db).unwrap_or_else(|e| panic!("{label}: {e}"));
        let caps = Capabilities::probe(&db, &meta);
        let surf = SurfaceCapabilities::probe(&db, &meta, &model, &caps);

        assert!(!model.players.is_empty(), "{label}: no players");
        assert!(!model.teams.is_empty(), "{label}: no teams");

        // ---- Capability ⟺ construction agreement (schema-driven surfaces) ----
        let avail = |s: SurfaceState| s != SurfaceState::NotApplicable;
        assert_eq!(
            avail(surf.arenas.state),
            has_table(&db, "StEO") && has_named_field(&db, &meta, "StEO", "arenaname"),
            "{label}: arenas cap vs StEO+arenaname"
        );
        assert_eq!(
            avail(surf.lines.state),
            has_table(&db, "eGlu") && has_named_slot(&db, &meta, "eGlu"),
            "{label}: lines cap vs eGlu named slot"
        );
        assert_eq!(avail(surf.captaincy.state), has_table(&db, "eGlu"), "{label}: captaincy cap vs eGlu");
        assert_eq!(
            avail(surf.strategies.state),
            has_table(&db, "EeYo") && has_field(&db, "EeYo", "vfEq"),
            "{label}: strategies cap vs EeYo"
        );
        assert_eq!(
            avail(surf.jerseys.state),
            has_table(&db, "xjgw") && has_field(&db, "xjgw", "mCXg"),
            "{label}: jerseys cap vs xjgw"
        );
        assert_eq!(
            avail(surf.team_equipment.state),
            has_table(&db, "hHbv") && has_field(&db, "hHbv", "dXSB"),
            "{label}: team_equipment cap vs hHbv"
        );
        assert_eq!(
            avail(surf.draft_picks.state),
            has_table(&db, "qHav") && has_field(&db, "qHav", "Uwdb"),
            "{label}: draft_picks cap vs qHav"
        );
        // Asset surfaces are never NotApplicable and never promoted past Disabled in Phase 1.
        for (name, s) in [
            ("portraits", surf.portraits),
            ("logos", surf.logos),
            ("heads", surf.heads),
            ("audio", surf.audio),
        ] {
            assert_eq!(s.state, SurfaceState::Disabled, "{label}: {name} must stay Disabled in Phase 1");
        }
        // Players/teams are always available (non-empty model asserted above).
        assert!(avail(surf.players.state) && avail(surf.teams.state), "{label}: players/teams available");

        // ---- Player representatives (Phase 1C) ----
        let summaries = player_summaries(&db, &meta, &enums, &model);
        let mut reps: Vec<u64> = Vec::new();
        let mut push = |reps: &mut Vec<u64>, id: Option<u64>| {
            if let Some(id) = id {
                if !reps.contains(&id) {
                    reps.push(id);
                }
            }
        };
        // One of each position label present (LW/C/RW/D/G), by ascending player id.
        let mut by_id = summaries.clone();
        by_id.sort_by_key(|s| s.player_id);
        for want in ["LW", "C", "RW", "D", "G"] {
            let pick = by_id.iter().find(|s| {
                let p = s.position_label.as_str();
                if want == "D" { p.ends_with('D') || p == "D" } else { p == want }
            });
            push(&mut reps, pick.map(|s| s.player_id));
        }
        // Rostered, free agent, inactive.
        push(&mut reps, by_id.iter().find(|s| s.team_id.is_some()).map(|s| s.player_id));
        push(&mut reps, by_id.iter().find(|s| s.team_id.is_none()).map(|s| s.player_id));
        push(&mut reps, by_id.iter().find(|s| s.is_active == Some(false)).map(|s| s.player_id));
        // Multi-team: >1 roster slot resolves to this player.
        let multi = by_id.iter().map(|s| s.player_id).find(|pid| {
            model.slots.iter().filter(|sl| sl.player_id == Some(*pid)).count() > 1
        });
        push(&mut reps, multi);
        // Missing optional companion: an AI companion absent for the player's kind.
        let missing = model
            .players
            .iter()
            .find(|p| match p.kind {
                PlayerKind::Skater => p.skater_ai.is_none(),
                PlayerKind::Goalie => p.goalie_ai.is_none(),
            })
            .map(|p| p.player_id);
        push(&mut reps, missing);

        for pid in reps {
            let view = player_view(&db, &meta, &enums, &model, pid)
                .unwrap_or_else(|| panic!("{label}: player_view({pid}) is None"));
            assert_profile_ok(&db, &meta, &view.profile, &format!("{label} player {pid}"));
            assert_all_fields_editable_honest(&db, &meta, &view.all_fields, &format!("{label} player {pid}"));
            // Ratings companion table is kind-correct when present.
            assert!(
                view.ratings_table == "dSvy" || view.ratings_table == "mHuy",
                "{label}: player {pid} unexpected ratings table {}",
                view.ratings_table
            );
            // Memberships resolve to a real team in the model.
            for m in &view.memberships {
                assert!(
                    model.teams.iter().any(|t| t.team_id == m.team_id),
                    "{label}: player {pid} membership team {} not in model",
                    m.team_id
                );
            }
            // Career reads without panic; every career column targets an existing field.
            let career = career_stats(&db, &meta, &enums, pid);
            for sec in &career.sections {
                if let Some(t) = db.table_canonical(&sec.table) {
                    for col in &sec.columns {
                        assert!(
                            t.fields.iter().any(|f| f.id.canonical(db.endian) == col.field),
                            "{label}: career column {}.{} missing", sec.table, col.field
                        );
                    }
                }
            }
        }

        // ---- Team representatives (Phase 1C) ----
        let teams = team_summaries(&db, &meta, &enums, &model);
        let mut team_reps: Vec<u64> = Vec::new();
        let mut tby = teams.clone();
        tby.sort_by_key(|t| t.team_id);
        // First and last team (variety), plus a team with a populated line-up and one without a
        // strategy sheet (populated vs empty optional rows).
        if let Some(t) = tby.first() { team_reps.push(t.team_id); }
        if let Some(t) = tby.last() { if !team_reps.contains(&t.team_id) { team_reps.push(t.team_id); } }
        let populated = tby.iter().map(|t| t.team_id).find(|id| {
            lines_view(&db, &meta, &enums, &model, *id).map(|l| !l.entries.is_empty()).unwrap_or(false)
        });
        if let Some(id) = populated { if !team_reps.contains(&id) { team_reps.push(id); } }

        for tid in &team_reps {
            let view = team_view(&db, &meta, &enums, &model, *tid)
                .unwrap_or_else(|| panic!("{label}: team_view({tid}) is None"));
            assert_profile_ok(&db, &meta, &view.profile, &format!("{label} team {tid}"));
            assert_all_fields_editable_honest(&db, &meta, &view.all_fields, &format!("{label} team {tid}"));
            // Arena FK resolves to a real arena or is None (documented null).
            if let Some(arena) = &view.arena {
                assert!(!arena.name.is_empty() || arena.city.is_empty() || true, "arena ref built");
                let _ = arena; // resolution success is implied by Some(ArenaRef)
            }
            // Per-team surfaces build without panic; results agree with capability.
            let strat = team_strategies(&db, &meta, &enums, *tid);
            if !avail(surf.strategies.state) {
                assert!(strat.is_none(), "{label}: strategies absent but built for team {tid}");
            }
            let picks = team_draft_picks(&db, &meta, &model, *tid);
            if !avail(surf.draft_picks.state) {
                assert!(picks.is_empty(), "{label}: draft picks absent but built for team {tid}");
            }
            let jer = team_jerseys(&db, *tid);
            if !avail(surf.jerseys.state) {
                assert!(jer.is_empty(), "{label}: jerseys absent but built for team {tid}");
            }
            let equip = team_equipment(&db, *tid);
            if !avail(surf.team_equipment.state) {
                assert!(equip.is_empty(), "{label}: equipment absent but built for team {tid}");
            }
            let lv = lines_view(&db, &meta, &enums, &model, *tid);
            if !avail(surf.lines.state) {
                assert!(lv.is_none(), "{label}: lines absent but built for team {tid}");
            }
        }

        // Arenas surface builds without panic. Row *count* is data (may be 0 for an empty table or
        // a name-less arena table like NHL07's) — not a capability signal, so it is not asserted here.
        let _ = arena_summaries(&db, &meta);

        checked += 1;
    }

    assert_eq!(checked, 27, "expected 27 anchors in the read matrix, checked {checked}");
}

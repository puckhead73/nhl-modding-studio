//! Phase 2.5 (22 Sep 2026) — the arena **venue-parameter inventory** through the registry seam on
//! the committed anchors, across the era signatures research 2026-09-21 pm handoff R51 measured:
//! the gen6 thin table (NHL 07), the NHL 08 rink-dressing set, the gen7 core, and the PS4 colour
//! zones (NHL 17 / 21).
//!
//! What this asserts is what Studio *computes from the database* — which fields the title carries,
//! how many are constant across all its arena rows, which render as colours — cross-checked
//! against the handoff's per-anchor figures. A class is a naming candidate, so the test checks
//! that the classes are **applied and surfaced**, never that they mean anything at runtime.

use std::path::{Path, PathBuf};

use studio_api::Registry;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn open(reg: &Registry, platform: &str, game: &str, db: &str) -> u32 {
    let dir = match platform {
        "xbox" => "xbox360",
        "ps3" => "ps3",
        _ => "ps4",
    };
    reg.open_workspace_from_paths(
        Some(game),
        studio_api::parse_platform(platform).unwrap(),
        &s(&repo(&format!("anchors/{dir}/{game}/databases/{db}.db"))),
        None,
        &s(&repo(&format!("anchors/{dir}/{game}/databases/{db}-meta.xml"))),
        &s(&repo("defs")),
    )
    .unwrap()
    .project_id
}

/// The first arena id the title carries, so the test does not hard-code a row that an era lacks.
fn first_arena(reg: &Registry, id: u32) -> u64 {
    let arenas = reg.list_arenas(id).expect("arena list");
    arenas.first().map(|a| a.arena_id).expect("at least one arena")
}

/// Across four era signatures: the inventory lists the title's own fields, every field carries a
/// class and a gloss, the unclassified ones are refused with a reason, constants are measured over
/// the file's own arena rows, and the notes never promise a runtime effect.
#[test]
fn the_venue_inventory_covers_each_era_signature_and_refuses_what_it_cannot_explain() {
    let reg = Registry::default();
    for (platform, game, db, min_fields, expect_colours) in [
        ("ps3", "nhl07", "NHL07NG", 2usize, false),
        ("ps3", "nhl08", "nhlng", 20, false),
        ("xbox", "nhllegacy", "nhlng", 12, false),
        ("ps4", "nhl21", "nhl_game", 50, true),
    ] {
        let id = open(&reg, platform, game, db);
        let arena_id = first_arena(&reg, id);
        let v = reg
            .arena_venue_view(id, arena_id, &repo("defs"), None)
            .unwrap_or_else(|e| panic!("{game}: {e}"))
            .unwrap_or_else(|| panic!("{game}: arena {arena_id} has no venue view"));

        assert!(v.rows > 0, "{game}: the inventory names the arena-row population");
        assert!(v.fields.len() >= min_fields, "{game}: {} fields, expected at least {min_fields}", v.fields.len());
        // Every field is classified and glossed; the counts add up to the field list.
        assert!(v.fields.iter().all(|f| !f.class.is_empty() && !f.class_note.is_empty()), "{game}");
        assert_eq!(v.class_counts.iter().map(|(_, n)| *n).sum::<usize>(), v.fields.len(), "{game}: class counts cover every field");
        // Nothing is offered for editing without a reason to believe it means something.
        for f in &v.fields {
            if f.class == "unknown" {
                assert!(!f.editable && f.refusal.as_deref().is_some_and(|r| r.contains("not classified")), "{game}: {} is unclassified but offered", f.name);
            }
            if !f.editable {
                assert!(f.refusal.is_some(), "{game}: {} is read-only without a reason", f.name);
            }
            if let Some(c) = f.constant {
                assert_eq!(f.distinct, 1, "{game}: {} is constant but has {} distinct values", f.name, f.distinct);
                assert_eq!(f.value, c, "{game}: {} constant value disagrees with this row", f.name);
            } else {
                assert!(f.distinct > 1, "{game}: {} is not constant but has one value", f.name);
            }
        }
        // The disclaimer is always present and always says the same thing.
        assert!(v.notes.iter().any(|n| n.contains("not a traced runtime effect")), "{game}: {:?}", v.notes);

        let colours = v.fields.iter().filter(|f| f.colour.is_some()).count();
        if expect_colours {
            // R51 §2: the colour zones are non-zero only on the few custom arenas, so a given row
            // may legitimately carry none — but the title must *have* colour-capable fields, and
            // some arena in the file must carry one.
            // Walk the title's real arena ids (they are not 0..n), since R51 §2 measures the
            // zones as non-zero on only ~4 % of rows — the created / custom arenas.
            let ids: Vec<u64> = reg.list_arenas(id).unwrap().iter().map(|a| a.arena_id).collect();
            let with_colour = ids
                .iter()
                .filter(|a| {
                    reg.arena_venue_view(id, **a, &repo("defs"), None)
                        .ok()
                        .flatten()
                        .is_some_and(|w| w.fields.iter().any(|f| f.colour.is_some()))
                })
                .count();
            assert!(with_colour > 0, "{game}: none of the {} arena rows carries a colour zone", ids.len());
            eprintln!("{game}: {with_colour} of {} arena rows carry at least one colour zone", ids.len());
            assert!(v.fields.iter().any(|f| f.name.contains("zone")), "{game}: the PS4 zone fields are listed");
        } else {
            assert_eq!(colours, 0, "{game}: a pre-PS4 title has no colour zones");
        }

        // A colour is rendered as `#RRGGBB`, never as a bare number.
        for f in v.fields.iter().filter(|f| f.colour.is_some()) {
            let c = f.colour.as_deref().unwrap();
            assert!(c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit()), "{game}: {} colour {c}", f.name);
        }
        eprintln!(
            "{game}: {} fields over {} arena rows; {} constant, {} unclassified, {} colours on this row; classes {:?}",
            v.fields.len(),
            v.rows,
            v.fields.iter().filter(|f| f.constant.is_some()).count(),
            v.fields.iter().filter(|f| f.class == "unknown").count(),
            colours,
            v.class_counts
        );
    }
}

/// The identity fields the arena header already edits are listed but never offered twice, so one
/// value never has two editors.
#[test]
fn identity_fields_defer_to_the_identity_card() {
    let reg = Registry::default();
    let id = open(&reg, "xbox", "nhllegacy", "nhlng");
    let arena_id = first_arena(&reg, id);
    let v = reg.arena_venue_view(id, arena_id, &repo("defs"), None).unwrap().unwrap();
    let index = v.fields.iter().find(|f| f.name == "index").expect("the index field is listed");
    assert!(!index.editable);
    assert!(index.refusal.as_deref().is_some_and(|r| r.contains("identity card")), "{:?}", index.refusal);
}

/// Phase 2.7 (PS4) — the goal-horn **catalogue join** on the real extracted trees: research
/// 2026-09-21 pm handoff R50 §2 measured 30 of 30 NHL-club arenas naming their own club under the
/// "value − 1" rule on every title from NHL 17 to 21. This reproduces that through the arena venue
/// card, and checks that the two shipped gaps (the missing Rangers row, the 690–693 sentinels) are
/// named refusals rather than wrong rows.
#[test]
#[ignore = "real extracted PS4 trees under H:/Emulators/ShadPS4/Games (read only): select with --ignored"]
fn the_ps4_goalhorn_catalogue_names_the_club_on_the_real_trees() {
    let reg = Registry::default();
    for (game, cusa) in [("nhl21", "CUSA18182"), ("nhl17", "CUSA04326")] {
        let root = PathBuf::from(format!(r"H:\Emulators\ShadPS4\Games\{cusa}\databigs\Extracted"));
        assert!(root.is_dir(), "{}: missing (fails, never skips)", root.display());
        let id = open(&reg, "ps4", game, "nhl_game");
        let ids: Vec<u64> = reg.list_arenas(id).unwrap().iter().map(|a| a.arena_id).collect();

        let (mut team_rows, mut generic_rows, mut sentinels, mut rangers, mut unresolved) = (0, 0, 0, 0, 0);
        for a in &ids {
            let Some(v) = reg.arena_venue_view(id, *a, &repo("defs"), Some(&root)).unwrap() else { continue };
            let Some(h) = v.fields.iter().find(|f| f.name == "goalhorn") else { continue };
            match (&h.resolved, &h.resolved_note) {
                (Some(_), Some(note)) if note.contains("team") => team_rows += 1,
                (Some(_), _) => generic_rows += 1,
                (None, Some(note)) if note.contains("sentinel") => sentinels += 1,
                (None, Some(note)) if note.contains("New York Rangers") => rangers += 1,
                _ => unresolved += 1,
            }
        }
        eprintln!("{game}: {} arenas — {team_rows} club horns, {generic_rows} generic, {sentinels} sentinels, {rangers} Rangers gap, {unresolved} unresolved", ids.len());
        // R50 §2: every one of the 30 clubs whose row exists resolves to its own club row.
        assert!(team_rows >= 30, "{game}: only {team_rows} arenas resolved a premade club horn (research measured 30 / 30)");
        assert!(rangers >= 1, "{game}: the shipped missing-Rangers gap is not reported as such");
        assert_eq!(unresolved, 0, "{game}: {unresolved} arenas resolved to neither a row nor a named reason");
    }
}

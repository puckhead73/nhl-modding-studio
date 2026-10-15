//! League API surface (O2 / L1.1-C): read, valid edit, oversize/multibyte rejection, key refusal,
//! undo/redo, and target changes through the exact Registry seam the app calls.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::FieldValue;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn s(p: PathBuf) -> String {
    p.to_str().unwrap().to_string()
}

fn open(reg: &Registry, game: &str, platform: &str, db: &str, meta: &str) -> u32 {
    reg.open_workspace_from_paths(
        Some(game),
        parse_platform(platform).unwrap(),
        &s(repo(db)),
        None,
        &s(repo(meta)),
        &s(repo("defs")),
    )
    .unwrap()
    .project_id
}

#[test]
fn nhl21_league_read_edit_undo_redo() {
    let reg = Registry::default();
    let id = open(
        &reg,
        "nhl21",
        "ps4",
        "anchors/ps4/nhl21/databases/nhl_game.db",
        "anchors/ps4/nhl21/databases/nhl_game-meta.xml",
    );
    let catalog = reg.list_leagues(id).unwrap();
    assert_eq!(catalog.source, "stockleagues");
    assert!(catalog.name_edit_blocker.is_none());
    assert_eq!(
        catalog
            .leagues
            .iter()
            .filter(|l| l.record.is_some())
            .count(),
        31
    );
    let alls = catalog.leagues.iter().find(|l| l.code == 8).unwrap();
    assert_eq!(alls.name_key.as_deref(), Some("Allsvenskan"));
    assert_eq!(alls.display, "Allsvenskan");
    assert_eq!(alls.confidence.as_deref(), Some("native"));

    let view = reg.league_view(id, 8).unwrap().unwrap();
    assert_eq!(view.row_table.as_deref(), Some("fsLK"));
    assert_eq!(view.name_max_bytes, Some(25));
    assert_eq!(view.members.len(), 14);
    assert!(view.members.iter().all(|t| t.league == Some(8)));
    assert!(view.members.iter().any(|t| t.abbrev == "AIK"));
    let name = view.fields.iter().find(|f| f.name == "name").unwrap();
    assert!(name.editable && name.is_string);
    let key = view.fields.iter().find(|f| f.name == "leagueid").unwrap();
    assert!(!key.editable, "the primary key is never editable");
    assert!(reg.league_view(id, 99).unwrap().is_none());

    // Key refusal.
    let err = match reg.set_field(id, "fsLK", 8, "aQrQ", FieldValue::Int(9)) {
        Err(e) => e,
        Ok(_) => panic!("leagueid is protected"),
    };
    assert!(err.to_string().contains("not updatable"), "{err}");
    // Oversize (26 ASCII bytes) and multibyte-overflow (13 two-byte chars = 26 bytes) rejection —
    // the limit is bytes, not characters.
    for text in ["X".repeat(26), "é".repeat(13)] {
        let err = match reg.set_field(id, "fsLK", 8, "vIys", FieldValue::Text(text)) {
            Err(e) => e,
            Ok(_) => panic!("over the 25-byte width"),
        };
        assert!(
            err.to_string()
                .contains("the text is 26 bytes but this field holds at most 25 bytes"),
            "{err}"
        );
    }
    // 12 two-byte chars = 24 bytes fits.
    reg.set_field(id, "fsLK", 8, "vIys", FieldValue::Text("é".repeat(12)))
        .unwrap();
    assert!(reg.undo(id).unwrap());

    // Valid re-point to another existing key; the route/identity (code 8) is unchanged.
    reg.set_field(
        id,
        "fsLK",
        8,
        "vIys",
        FieldValue::Text("TXT_ROW_LEAGUE".into()),
    )
    .unwrap();
    assert!(reg.summary(id).unwrap().dirty);
    let view = reg.league_view(id, 8).unwrap().unwrap();
    assert_eq!(view.summary.name_key.as_deref(), Some("TXT_ROW_LEAGUE"));
    assert_eq!(view.summary.code, 8);
    assert_eq!(view.members.len(), 14, "membership untouched");
    // Team surfaces share the resolver.
    let teams = reg.list_teams(id).unwrap();
    let aik = teams.iter().find(|t| t.abbrev == "AIK").unwrap();
    assert_eq!(aik.league_label.as_deref(), Some("TXT_ROW_LEAGUE"));
    let tv = reg.team_view(id, aik.team_id).unwrap().unwrap();
    assert_eq!(tv.league_label.as_deref(), Some("TXT_ROW_LEAGUE"));

    assert!(reg.undo(id).unwrap());
    assert_eq!(
        reg.league_view(id, 8)
            .unwrap()
            .unwrap()
            .summary
            .name_key
            .as_deref(),
        Some("Allsvenskan")
    );
    assert!(!reg.summary(id).unwrap().dirty);
    assert!(reg.redo(id).unwrap());
    assert_eq!(
        reg.league_view(id, 8)
            .unwrap()
            .unwrap()
            .summary
            .name_key
            .as_deref(),
        Some("TXT_ROW_LEAGUE")
    );
}

#[test]
fn gen7_leagues_are_derived_and_name_edit_is_blocked_with_a_reason() {
    let reg = Registry::default();
    let id = open(
        &reg,
        "nhllegacy",
        "ps3",
        "anchors/ps3/nhllegacy/databases/nhlng.db",
        "anchors/ps3/nhllegacy/databases/nhlng-meta.xml",
    );
    let catalog = reg.list_leagues(id).unwrap();
    assert_eq!(catalog.source, "derived-codes");
    assert_eq!(catalog.vocabulary.as_deref(), Some("gen7-nhllegacy"));
    assert!(catalog.name_edit_blocker.is_some());
    let norway = catalog.leagues.iter().find(|l| l.code == 7).unwrap();
    assert_eq!(norway.display, "GET-ligaen (Norway)");
    assert_eq!(norway.member_teams, 2);
    let view = reg.league_view(id, 7).unwrap().unwrap();
    assert!(view.fields.is_empty() && view.row_table.is_none());
    assert!(view.name_edit_blocker.is_some());
    assert_eq!(view.members.len(), 2);

    // A second target in the same registry resolves independently (no stale labels).
    let id2 = open(
        &reg,
        "nhl11",
        "ps3",
        "anchors/ps3/nhl11/databases/nhlng.db",
        "anchors/ps3/nhl11/databases/nhlng-meta.xml",
    );
    let c2 = reg.list_leagues(id2).unwrap();
    assert_eq!(c2.vocabulary.as_deref(), Some("gen7-nhl11"));
    assert_eq!(
        c2.leagues.iter().find(|l| l.code == 8).unwrap().display,
        "OHL"
    );
    assert_eq!(
        reg.list_leagues(id)
            .unwrap()
            .leagues
            .iter()
            .find(|l| l.code == 8)
            .unwrap()
            .display,
        "National Teams"
    );
}

/// O6: the target's own dictionary resolves PS4 name keys to display text; keys absent from it
/// stay keys; a dictionary attached to one project never leaks into another.
/// REG-1 (2026-09-18): the `anchors/ps4/nhl20` database and dictionary used here are the NHL 19
/// base members under an NHL 20 path; the pair is self-consistent (same capture), which is all
/// this case needs. The real NHL 20 files are covered by `tests/league_private_sources.rs`.
#[test]
fn attached_dictionary_resolves_ps4_labels_without_leaking() {
    let reg = Registry::default();
    let id20 = open(
        &reg,
        "nhl20",
        "ps4",
        "anchors/ps4/nhl20/databases/nhl_game.db",
        "anchors/ps4/nhl20/databases/nhl_game-meta.xml",
    );
    let id21 = open(
        &reg,
        "nhl21",
        "ps4",
        "anchors/ps4/nhl21/databases/nhl_game.db",
        "anchors/ps4/nhl21/databases/nhl_game-meta.xml",
    );
    let before = reg.list_leagues(id20).unwrap();
    assert!(before.dictionary.is_none());
    assert!(before.leagues.iter().all(|l| l.label.is_none()));

    let d = reg
        .attach_dictionary(id20, &s(repo("anchors/ps4/nhl20/databases/eng_us.db")))
        .unwrap();
    assert_eq!(d.locale, "eng_us");
    assert_eq!(d.rows, 37_734);
    let c = reg.list_leagues(id20).unwrap();
    assert_eq!(c.dictionary.as_ref().unwrap().rows, 37_734);
    let alls = c.leagues.iter().find(|l| l.code == 8).unwrap();
    assert_eq!(alls.name_key.as_deref(), Some("Allsvenskan"));
    assert_eq!(alls.label.as_deref(), Some("HockeyAllsvenskan"));
    assert_eq!(alls.display, "HockeyAllsvenskan");
    let nhl = c.leagues.iter().find(|l| l.code == 0).unwrap();
    assert_eq!(nhl.label.as_deref(), Some("NHL\u{ae}"));
    // `HUT` is not an exact row, but the game looks keys up by hash (case-folded): the row keyed
    // `hut` carries the same hash and supplies the text (LOC-W1 / P4).
    let hut = c
        .leagues
        .iter()
        .find(|l| l.name_key.as_deref() == Some("HUT"))
        .unwrap();
    assert_eq!(hut.label.as_deref(), Some("Hockey Ultimate Team"));
    assert!(hut.evidence.contains("by hash"), "{}", hut.evidence);
    // A key with no row at all (exact or by hash) stays a key.
    let mascot = c
        .leagues
        .iter()
        .find(|l| l.name_key.as_deref() == Some("TXT_LEAGUE_MASCOT"))
        .unwrap();
    assert!(mascot.label.is_none(), "TXT_LEAGUE_MASCOT is not in this dictionary: shown as a key");
    assert_eq!(mascot.display, "TXT_LEAGUE_MASCOT");
    assert!(mascot.evidence.contains("not in this target"));
    // Team surfaces share the resolved text.
    let teams = reg.list_teams(id20).unwrap();
    assert_eq!(
        teams
            .iter()
            .find(|t| t.abbrev == "AIK")
            .unwrap()
            .league_label
            .as_deref(),
        Some("HockeyAllsvenskan")
    );
    // A re-pointed key resolves through the same dictionary.
    reg.set_field(
        id20,
        "fsLK",
        8,
        "vIys",
        FieldValue::Text("TXT_ROW_LEAGUE".into()),
    )
    .unwrap();
    assert_eq!(
        reg.league_view(id20, 8)
            .unwrap()
            .unwrap()
            .summary
            .label
            .as_deref(),
        Some("Rest Of World")
    );
    // No leakage into the other project.
    let c21 = reg.list_leagues(id21).unwrap();
    assert!(c21.dictionary.is_none());
    assert!(c21.leagues.iter().all(|l| l.label.is_none()));
    // A file that is not a dictionary is refused, leaving the previous attachment in place.
    assert!(reg
        .attach_dictionary(id20, &s(repo("anchors/ps4/nhl20/databases/nhl_game.db")))
        .is_err());
    assert!(reg.list_leagues(id20).unwrap().dictionary.is_some());
}

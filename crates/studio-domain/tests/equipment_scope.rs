//! Title-scoped equipment vocabularies (coordinator decision, 25 Sep 2026; review item 8e).
//!
//! The stick / glove / helmet / skate / visor / tape / tuck catalogues were established on NHL
//! Legacy. They must label only a project opened as NHL Legacy; on every other title the stored id
//! is a number with a note saying no catalogue exists for the title, an unknown id round-trips
//! through edits, save and reopen untouched, and a label from Legacy is never borrowed. Run on all
//! 27 committed anchors through `Project` + `EditSession` on in-memory copies.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_domain::{EditSession, FieldValue, FieldView, Platform, Project, ProjectRole};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn anchors() -> Vec<(String, String, PathBuf, PathBuf)> {
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
        let mut games: Vec<_> = std::fs::read_dir(platform.path())
            .unwrap()
            .flatten()
            .collect();
        games.sort_by_key(|e| e.file_name());
        for game in games.iter().filter(|e| e.path().is_dir()) {
            let dbs = game.path().join("databases");
            if let Some((db, meta)) = pairs
                .iter()
                .find(|(d, m)| dbs.join(d).is_file() && dbs.join(m).is_file())
            {
                out.push((
                    platform.file_name().to_string_lossy().into_owned(),
                    game.file_name().to_string_lossy().into_owned(),
                    dbs.join(db),
                    dbs.join(meta),
                ));
            }
        }
    }
    assert_eq!(
        out.len(),
        27,
        "27 anchor directories (ps4/nhl20 is an NHL 19 base copy)"
    );
    out
}

fn platform_of(dir: &str) -> Platform {
    match dir {
        "ps3" => Platform::Ps3,
        "ps4" => Platform::Ps4,
        "xbox360" => Platform::Xbox,
        other => panic!("{other}"),
    }
}

fn open(platform: &str, db: &Path, meta: &Path, game: Option<&str>) -> EditSession {
    let mut sess = EditSession::begin(
        Project::load(
            db,
            platform_of(platform),
            ProjectRole::Nhlng,
            meta,
            repo("defs"),
        )
        .unwrap(),
    );
    sess.set_game_id(game.map(str::to_string));
    sess
}

/// The equipment columns the Legacy catalogues bind (metadata name → studio field id).
const CATALOGUED: &[(&str, &str)] = &[
    ("stick", "wXFx"),
    ("gloves", "NmUR"),
    ("helmetbrand", "YUZp"),
    ("skatebrand", "AOOe"),
    ("visortype", "cnFB"),
    ("visorcolor", "DBYQ"),
    ("sticktapecolor", "NISO"),
];

fn equip<'a>(fields: &'a [FieldView], name: &str) -> Option<&'a FieldView> {
    fields.iter().find(|f| f.name == name)
}

#[test]
fn equipment_labels_appear_only_on_nhl_legacy_and_unknown_ids_round_trip_everywhere_else() {
    let mut table: BTreeMap<String, String> = BTreeMap::new();
    for (platform, game, db_path, meta_path) in anchors() {
        let label = format!("{platform}/{game}");
        let mut sess = open(&platform, &db_path, &meta_path, Some(&game));
        let subject = sess
            .player_summaries()
            .unwrap()
            .into_iter()
            .find(|p| !p.is_goalie && p.team_id.is_some())
            .unwrap();
        let v = sess.player_view(subject.player_id).unwrap().unwrap();
        let Some(rec) = v.equipment_record else {
            assert_eq!(
                game, "nhl07",
                "{label}: only NHL 07 lacks the equipment companion"
            );
            table.insert(label, "n/a — no equipment companion on this title".into());
            continue;
        };
        let is_legacy = game == "nhllegacy";
        // NHL07 stores gear in biography fields, without the later companion's
        // glove/helmet/curve schema or its Legacy vocabulary bindings.
        if v.equipment_table == "ubPc" {
            assert_eq!(game, "nhl07", "{label}: early biography gear");
            let stick = equip(&v.equipment, "stick").expect("early stick field");
            assert!(stick.enum_options.is_empty(), "{label}: no borrowed labels");
            assert_eq!(stick.enum_label, None);
            let top = stick.range_high.unwrap();
            sess.set_field(v.equipment_table, rec, &stick.studio_field, FieldValue::Int(top)).unwrap();
            let saved = sess.save().unwrap();
            let db = TdbFile::parse(&saved).unwrap();
            let t = db.table_canonical("ubPc").unwrap();
            let f = t.fields.iter().find(|f| f.id.canonical(db.endian) == stick.studio_field).unwrap();
            assert_eq!(db.read_u64(t, rec, f), Some(top as u64));
            assert!(sess.undo());
            table.insert(label, format!("biography gear; raw stick id {top} round-trips"));
            continue;
        }
        let mut present = Vec::new();
        for (name, id) in CATALOGUED {
            let Some(f) = equip(&v.equipment, name) else {
                continue;
            };
            assert_eq!(f.studio_field, *id, "{label}: {name} id");
            present.push(*name);
            let note = f
                .vocabulary_note
                .clone()
                .unwrap_or_else(|| panic!("{label}: {name} carries a vocabulary note"));
            if is_legacy {
                assert!(
                    !f.enum_options.is_empty(),
                    "{label}: {name} labelled on Legacy"
                );
                assert!(
                    note.starts_with("labels established for NHL Legacy"),
                    "{label}: {name}: {note}"
                );
            } else {
                assert!(
                    f.enum_options.is_empty(),
                    "{label}: {name} borrowed Legacy labels: {:?}",
                    f.enum_options.first()
                );
                assert_eq!(f.enum_label, None, "{label}: {name}");
                assert_eq!(
                    f.display,
                    f.decoded.unwrap().to_string(),
                    "{label}: {name} shows the stored number"
                );
                assert!(
                    note.contains("no catalogue for") && note.contains("NHL Legacy"),
                    "{label}: {name}: {note}"
                );
                assert!(
                    !note.contains("wXFx") && !note.contains("xmja"),
                    "{label}: raw ids stay out: {note}"
                );
            }
        }
        assert!(
            present.contains(&"stick")
                && present.contains(&"gloves")
                && present.contains(&"helmetbrand"),
            "{label}: {present:?}"
        );

        // An id outside every catalogue (the field's range top) round-trips through an unrelated
        // edit, save and reopen, on every title including Legacy — a label never gates a write.
        let stick = equip(&v.equipment, "stick").unwrap();
        let top = stick.range_high.unwrap();
        let in_catalogue = stick.enum_options.iter().any(|(r, _)| *r == top);
        sess.set_field(v.equipment_table, rec, "wXFx", FieldValue::Int(top))
            .unwrap_or_else(|e| panic!("{label}: unknown stick id {top}: {e}"));
        let curve = equip(&v.equipment, "stickcurve").unwrap();
        let other_curve = if curve.decoded == Some(0) { 1 } else { 0 };
        sess.set_field(
            v.equipment_table,
            rec,
            &curve.studio_field,
            FieldValue::Int(other_curve),
        )
        .unwrap();
        let after = sess.player_view(subject.player_id).unwrap().unwrap();
        let s1 = equip(&after.equipment, "stick").unwrap();
        assert_eq!(
            s1.decoded,
            Some(top),
            "{label}: unknown id survives the unrelated edit"
        );
        if !in_catalogue {
            assert_eq!(
                s1.enum_label, None,
                "{label}: an uncatalogued id has no label"
            );
        }
        let saved = sess.save().unwrap();
        let db = TdbFile::parse(&saved).unwrap();
        let t = db.table_canonical(v.equipment_table).unwrap();
        let f = t
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "wXFx")
            .unwrap();
        assert_eq!(
            db.read_u64(t, rec, f),
            Some(top as u64),
            "{label}: saved bytes"
        );
        assert!(sess.undo() && sess.undo());

        // With no title declared, Legacy shows no labels either (nothing is borrowed on a guess).
        if is_legacy {
            let anon = open(&platform, &db_path, &meta_path, None);
            let av = anon.player_view(subject.player_id).unwrap().unwrap();
            let s = equip(&av.equipment, "stick").unwrap();
            assert!(s.enum_options.is_empty(), "{label}: no title → no labels");
            assert!(s
                .vocabulary_note
                .as_deref()
                .unwrap()
                .contains("unknown title"));
        }
        table.insert(
            label,
            format!(
                "{} — {}; stick id {top} round-trips",
                if is_legacy {
                    "labelled (NHL Legacy catalogues)"
                } else {
                    "numbers + note"
                },
                present.join(", ")
            ),
        );
    }
    eprintln!("\n| anchor | equipment vocabulary |");
    eprintln!("|---|---|");
    for (l, r) in &table {
        eprintln!("| {l} | {r} |");
    }
    assert_eq!(table.len(), 27);
}

/// The scope is on the definitions, not in code: every equipment vocabulary bound to the skater
/// equipment table carries `[scope] games = ["nhllegacy"]`, while nationality and player-style vocabularies have their own title scopes.
#[test]
fn every_skater_equipment_vocabulary_is_scoped_to_nhl_legacy() {
    let reg = tdb_enums::EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
    let mut scoped = Vec::new();
    let mut unscoped_equipment = Vec::new();
    for d in reg.defs() {
        let on_equipment = d
            .bindings
            .iter()
            .any(|b| b.table_raw_id == "xmja" || b.table_raw_id == "fMVl");
        match (&d.scope, on_equipment) {
            (Some(s), true) => {
                assert_eq!(s.games, vec!["nhllegacy".to_string()], "{}", d.name);
                assert!(
                    s.platforms.is_empty(),
                    "{}: platform split only where evidence requires it",
                    d.name
                );
                scoped.push(d.name.clone());
            }
            (None, true) => unscoped_equipment.push(d.name.clone()),
            (Some(_), false) => assert!(matches!(d.name.as_str(), "Nationality" | "Player Style"), "{}: unexpected scoped vocabulary", d.name),
            (None, false) => {}
        }
    }
    scoped.sort();
    assert_eq!(
        scoped,
        [
            "Glove Model",
            "Helmet Model",
            "Jersey Tuck",
            "Pant Length",
            "Skate Model",
            "Sock Tuck",
            "Stick Model",
            "Stick Tape Color",
            "Visor Style",
            "Visor Tint",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
    );
    assert!(
        unscoped_equipment.is_empty(),
        "unscoped equipment vocabularies: {unscoped_equipment:?}"
    );
    assert_eq!(reg.len(), 34); // Includes the four additional title-scoped nationality catalogues.
}

//! Misc 5.4 (22 Sep 2026) — the PS4 song catalogue listing through the registry seam: the real
//! NHL 17 / 18 / 19 / 21 installs reproduce research R54 §1 (row counts, the NHL 19 repeated
//! headers, composers), NHL 15 is "no catalogue", and a non-PS4 project says why. Read-only.

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
    reg.open_workspace_from_paths(Some(game), studio_api::parse_platform(platform).unwrap(), &s(&repo(&format!("anchors/{dir}/{game}/databases/{db}.db"))), None, &s(&repo(&format!("anchors/{dir}/{game}/databases/{db}-meta.xml"))), &s(&repo("defs")))
        .unwrap()
        .project_id
}

#[test]
fn a_gen7_project_and_a_ps4_root_without_the_file_say_why_nothing_is_listed() {
    let reg = Registry::default();
    let id = open(&reg, "xbox", "nhllegacy", "nhlng");
    let l = reg.song_catalogue(id, "X:/nowhere", "", 50).unwrap();
    assert!(!l.available && l.reason.as_deref().unwrap().contains("PS4 NHL 17–21"));
    let id = open(&reg, "ps4", "nhl21", "nhl_game");
    let tmp = std::env::temp_dir().join(format!("nhlms-songsapi-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("nhl/data/kettle/rendering")).unwrap();
    let l = reg.song_catalogue(id, &s(&tmp), "", 50).unwrap();
    assert!(!l.available && l.reason.as_deref().unwrap().contains("songs.csv"));
    let id = open(&reg, "ps4", "nhl15", "nhl_game");
    let l = reg.song_catalogue(id, &s(&tmp), "", 50).unwrap();
    assert!(!l.available && l.reason.as_deref().unwrap().contains("NHL 15 / 16"));
    let _ = std::fs::remove_dir_all(&tmp);
}

/// A disposable PS4 install root with a two-row song catalogue, a genre file and a goal-horn
/// catalogue in the measured directory; removes itself.
struct Ps4Root(PathBuf);
impl Ps4Root {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-songloc-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("nhl/data/kettle/rendering")).unwrap();
        let csv = d.join("nhl/data/common/gamemodes/creation_zone_csv");
        std::fs::create_dir_all(&csv).unwrap();
        std::fs::write(
            csv.join("songs.csv"),
            "index,identifier,artid,audioid,composer,genre,isgoalsong,ispowerplaysong,iswinsong,name\n\
             30,AlienDance,53,195704,Bob Bradley,11,1,0,1,$TXT_SONG_ALIEN_DANCE\n\
             31,NotShipped,54,195705,Nobody,99,1,0,1,$TXT_SONG_NOT_A_SHIPPED_SONG\n",
        )
        .unwrap();
        std::fs::write(csv.join("songgenres.csv"), "genre,name\n11,$TXT_ROCK\n99,$TXT_NOT_A_GENRE\n").unwrap();
        // Every premade club value 1–31 names the Ducks row here, so whichever horn the anchor's
        // first arena carries resolves to a team row; 31 / 41 are the generic horn.
        let mut horns = String::from("index,identifier,artid,audioid,name,ispremade,ishornelement,isflare,tone\n");
        for a in 0..31 {
            horns.push_str(&format!("{a},AnaheimDucks,{a},{a},$NHLTeamName_ANA,1,0,0,-1\n"));
        }
        horns.push_str("40,GenericHorn1,31,41,$TXT_HORN_ONE,0,0,0,-1\n");
        horns.push_str("41,GenericHorn2,31,31,$TXT_HORN_ONE,0,0,0,-1\n");
        std::fs::write(csv.join("goalhorns.csv"), horns).unwrap();
        Ps4Root(d)
    }
}
impl Drop for Ps4Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Coordinator review of receipts 10–17, item 2 (22 Sep 2026): song and horn keys resolve through
/// **each open project's own attached dictionary** and nothing else. Two PS4 projects side by side:
/// a dictionary attached to one never labels the other; a re-attach (the reload path) with a
/// different file changes what resolves and the reply's stamp; detach returns the raw keys with the
/// explicit unresolved state; closing one project leaves the other's resolution intact. Every reply
/// carries the dictionary it resolved through, so a late reply for another attachment is
/// recognisable by its stamp. Committed anchors read only; the install root is disposable.
#[test]
fn song_and_horn_keys_resolve_through_each_projects_own_dictionary_across_attach_detach_reload_and_switch() {
    let reg = Registry::default();
    let root = Ps4Root::new("seam");
    let install = s(&root.0);
    let defs = repo("defs");
    let a = open(&reg, "ps4", "nhl17", "nhl_game");
    let b = open(&reg, "ps4", "nhl18", "nhl_game");
    let dict17 = s(&repo("anchors/ps4/nhl17/databases/eng_us.db"));
    let dict18 = s(&repo("anchors/ps4/nhl18/databases/eng_us.db"));
    let dict15 = s(&repo("anchors/ps4/nhl15/databases/eng_us.db"));
    let songs = |id: u32| reg.song_catalogue(id, &install, "", 50).unwrap();
    let row = |l: &studio_api::dto::SongCatalogueDto, ident: &str| l.rows.iter().find(|r| r.identifier == ident).cloned().unwrap();
    let horn = |id: u32| {
        let arena = reg.list_arenas(id).unwrap().first().unwrap().arena_id;
        let v = reg.arena_venue_view(id, arena, &defs, Some(&root.0)).unwrap().unwrap();
        v.fields.into_iter().find(|f| f.name == "goalhorn").expect("PS4 arenas carry goalhorn")
    };

    // 1. Nothing attached: raw keys, explicit state, no stamp — on both projects.
    for id in [a, b] {
        let l = songs(id);
        assert!(l.available, "{:?}", l.reason);
        let r = row(&l, "AlienDance");
        assert_eq!((r.title.as_deref(), r.title_lookup.as_str(), r.name.as_str()), (None, "no-dictionary", "$TXT_SONG_ALIEN_DANCE"));
        assert_eq!((r.genre_label.as_deref(), r.genre_lookup.as_deref()), (None, Some("no-dictionary")));
        assert!(l.dictionary.is_none());
        assert!(l.notes.iter().any(|n| n.contains("no dictionary is attached") && n.contains("never borrowed")), "{:?}", l.notes);
        let g = l.genre_counts.iter().find(|g| g.genre == 11).unwrap();
        assert_eq!((g.key.as_str(), g.label.as_deref(), g.lookup.as_str()), ("$TXT_ROCK", None, "no-dictionary"));
        let h = horn(id);
        assert_eq!(h.resolved_key_lookup.as_deref(), Some("no-dictionary"));
        assert!(h.resolved.as_deref().unwrap().starts_with('$'), "{:?}", h.resolved);
    }

    // 2. Attach NHL 17's own dictionary to A: A resolves, B does not.
    let d = reg.attach_dictionary(a, &dict17).unwrap();
    let la = songs(a);
    let r = row(&la, "AlienDance");
    assert_eq!((r.title.as_deref(), r.title_lookup.as_str()), (Some("Alien Dance"), "exact"));
    assert_eq!((r.genre_label.as_deref(), r.genre_lookup.as_deref()), (Some("Rock"), Some("exact")));
    let r = row(&la, "NotShipped");
    assert_eq!((r.title.as_deref(), r.title_lookup.as_str(), r.name.as_str()), (None, "absent", "$TXT_SONG_NOT_A_SHIPPED_SONG"));
    assert_eq!(r.genre_lookup.as_deref(), Some("absent"));
    let stamp_a17 = la.dictionary.clone().expect("stamped with the dictionary it resolved through");
    assert_eq!((stamp_a17.locale.as_str(), stamp_a17.sha256.as_str(), stamp_a17.rows), ("eng_us", d.sha256.as_str(), d.rows));
    assert!(la.notes.iter().any(|n| n.contains("1 resolved, 1 not in the dictionary")), "{:?}", la.notes);
    let g = la.genre_counts.iter().find(|g| g.genre == 99).unwrap();
    assert_eq!((g.label.as_deref(), g.lookup.as_str()), (None, "absent"));
    // The query matches the resolved text too.
    assert_eq!(reg.song_catalogue(a, &install, "alien dance", 50).unwrap().matching, 1);
    let h = horn(a);
    assert_eq!(h.resolved_key_lookup.as_deref(), Some("exact"));
    assert!(h.resolved.as_deref().unwrap().starts_with("Anaheim Ducks") || h.resolved.as_deref().unwrap().starts_with("$TXT_HORN_ONE"), "{:?}", h.resolved);
    assert!(h.resolved_key.as_deref().is_some_and(|k| k.starts_with('$')));
    let lb = songs(b);
    assert_eq!(row(&lb, "AlienDance").title_lookup, "no-dictionary", "B never borrows A's dictionary");
    assert!(lb.dictionary.is_none());
    assert_eq!(horn(b).resolved_key_lookup.as_deref(), Some("no-dictionary"));

    // 3. B attaches its own (NHL 18): its own stamp, its own resolution.
    let d18 = reg.attach_dictionary(b, &dict18).unwrap();
    let lb = songs(b);
    assert_eq!(row(&lb, "AlienDance").title.as_deref(), Some("Alien Dance"));
    assert_eq!(lb.dictionary.as_ref().unwrap().sha256, d18.sha256);
    assert_ne!(lb.dictionary.as_ref().unwrap().sha256, stamp_a17.sha256, "each project is stamped with its own attachment");

    // 4. Re-attach A with another file (the reload path): what resolves follows the new bytes, and
    //    the stamp changes — a reply carrying the old stamp is recognisably stale.
    let d15 = reg.attach_dictionary(a, &dict15).unwrap();
    let la = songs(a);
    assert_eq!(row(&la, "AlienDance").title_lookup, "absent", "NHL 15's dictionary carries no song keys");
    assert_eq!(la.dictionary.as_ref().unwrap().sha256, d15.sha256);
    assert_ne!(la.dictionary.as_ref().unwrap().sha256, stamp_a17.sha256);
    let h = horn(a);
    assert!(matches!(h.resolved_key_lookup.as_deref(), Some("exact") | Some("absent")), "{:?}", h.resolved_key_lookup);

    // 5. Detach A: raw keys again, no stamp; B untouched.
    assert!(reg.detach_dictionary(a).unwrap());
    let la = songs(a);
    assert_eq!(row(&la, "AlienDance").title_lookup, "no-dictionary");
    assert!(la.dictionary.is_none());
    assert_eq!(horn(a).resolved_key_lookup.as_deref(), Some("no-dictionary"));
    assert_eq!(row(&songs(b), "AlienDance").title.as_deref(), Some("Alien Dance"));

    // 6. Project switch: closing A leaves B resolving through its own attachment.
    reg.close_project(a);
    let lb = songs(b);
    assert_eq!(row(&lb, "AlienDance").title.as_deref(), Some("Alien Dance"));
    assert_eq!(lb.dictionary.as_ref().unwrap().sha256, d18.sha256);
}

#[test]
#[ignore = "real extracted PS4 trees under H:/Emulators/ShadPS4/Games (read only): select with --ignored"]
fn the_real_catalogues_reproduce_the_research() {
    let reg = Registry::default();
    // R54 §1: rows / composers; NHL 19 repeats its header.
    for (game, cusa, rows, composers, repeats) in [("nhl21", "CUSA18182", 422usize, 135usize, 0usize), ("nhl19", "CUSA11126", 637, 133, 1), ("nhl18", "CUSA07569", 245, 129, 0), ("nhl17", "CUSA04326", 191, 97, 0)] {
        let root = PathBuf::from(format!(r"H:\Emulators\ShadPS4\Games\{cusa}\databigs\Extracted"));
        assert!(root.is_dir(), "{}: missing (fails, never skips)", root.display());
        let id = open(&reg, "ps4", game, "nhl_game");
        let l = reg.song_catalogue(id, &s(&root), "", 5000).unwrap();
        eprintln!("{game}: available {} rows {} composers {} genres {:?} notes {}", l.available, l.rows_total, l.composers, l.genre_counts.iter().map(|g| format!("{}:{}:{}", g.genre, g.key, g.rows)).collect::<Vec<_>>(), l.notes.len());
        assert!(l.available, "{game}: {:?}", l.reason);
        assert_eq!(l.rows_total, rows, "{game}: rows (R54 §1)");
        // Studio counts distinct composers case-insensitively after trimming; R54's count differs by
        // one on NHL 21 (a case / spacing variant of one name) — within one is the research figure.
        assert!(l.composers.abs_diff(composers) <= 1, "{game}: {} distinct composers vs R54's {composers}", l.composers);
        assert_eq!(l.rows.len(), rows);
        assert!(l.genre_counts.iter().all(|g| g.key.starts_with("$TXT_")), "{game}: genres named from songgenres.csv");
        if repeats > 0 {
            assert!(l.notes.iter().any(|n| n.contains("repeats its header")), "{game}: {:?}", l.notes);
        }
        // Every row: a `$TXT_SONG_` (or `$None`) key, and the eligibility flags R54 recorded (all
        // rows goal / PP / win eligible on every title; intro never set).
        assert!(l.rows.iter().all(|r| r.name.starts_with("$TXT_SONG_") || r.name == "$None"), "{game}");
        assert!(l.rows.iter().all(|r| r.is_goal && r.is_win), "{game}: goal / win eligibility");
        assert!(l.rows.iter().all(|r| r.is_intro != Some(true)), "{game}: isintrosong is 0 everywhere (R54)");
        assert!(l.notes.iter().any(|n| n.contains("not the team's song")), "{game}");
        // A composer filter narrows and never invents.
        let f = reg.song_catalogue(id, &s(&root), "bradley", 5000).unwrap();
        assert!(f.matching > 0 && f.matching < rows && f.rows.iter().all(|r| r.composer.to_ascii_lowercase().contains("bradley")), "{game}");
        reg.close_project(id);
    }
    // NHL 15: no catalogue, by contract — even with the real root attached.
    let root = PathBuf::from(r"H:\Emulators\ShadPS4\Games\CUSA00561\databigs\Extracted");
    assert!(root.is_dir(), "{}: missing (fails, never skips)", root.display());
    let id = open(&reg, "ps4", "nhl15", "nhl_game");
    let l = reg.song_catalogue(id, &s(&root), "", 50).unwrap();
    assert!(!l.available && l.reason.as_deref().unwrap().contains("NHL 15 / 16"));
}

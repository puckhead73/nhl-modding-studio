//! Players 3.3 (25 Sep 2026, package P3.3-A) — **where player text lives**, per anchor.
//!
//! The roadmap's 3.3 asks Studio to distinguish direct biography strings from localized text and
//! from audio-name references, and never to assume every player name lives in a locale dictionary.
//! Measured here on all 27 anchors: the bio table's string columns (first / last name, nickname
//! where the title has one, birth city, and the NHL 08–10 `statename`) — how many are populated,
//! how many carry a `$`-prefixed dictionary key (none, everywhere), how many are non-ASCII UTF-8;
//! and the audio-name reference columns the title carries (`audioid` on every title, NHL 07's
//! `defaultaudioid`, NHL 21's `surnameid`).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

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
    assert_eq!(out.len(), 27);
    out
}

#[derive(Default)]
struct Col {
    width: u32,
    filled: usize,
    keys: usize,
    non_ascii: usize,
    not_utf8: usize,
    key_samples: Vec<String>,
}

#[test]
fn player_text_is_inline_biography_never_a_dictionary_key_and_the_audio_reference_columns_are_named(
) {
    eprintln!("\n| anchor | string column: width B / filled / `$`-keys / non-ASCII / not-UTF-8 | audio-name reference columns |");
    eprintln!("|---|---|---|");
    for (platform, game, db_path, meta_path) in anchors() {
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let bio = db.table_canonical("ubPc").unwrap();
        let mut cols: BTreeMap<String, Col> = BTreeMap::new();
        let mut audio: Vec<String> = Vec::new();
        for f in &bio.fields {
            let Some(m) = meta.field(&bio.id.raw(), &f.id.raw()) else {
                continue;
            };
            let is_string = matches!(m.kind, tdb_defs::FieldType::String);
            // The birth city is addressed by its studio id (`MFzJ`); its metadata name is generic.
            let is_birth_city = f.id.canonical(db.endian) == "MFzJ";
            if is_string
                && (is_birth_city
                    || matches!(
                        m.name.as_str(),
                        "firstname" | "lastname" | "nickname" | "statename"
                    ))
            {
                let label = if is_birth_city {
                    "birthcity".to_string()
                } else {
                    m.name.clone()
                };
                let c = cols.entry(label).or_default();
                c.width = f.bit_width / 8;
                for rec in 0..bio.current_records as u32 {
                    let Some(raw) = db.read_inline_str(bio, rec, f) else {
                        continue;
                    };
                    if raw.is_empty() {
                        continue;
                    }
                    c.filled += 1;
                    if raw[0] == b'$' {
                        c.keys += 1;
                        if c.key_samples.len() < 4 {
                            c.key_samples
                                .push(String::from_utf8_lossy(raw).into_owned());
                        }
                    }
                    if !raw.is_ascii() {
                        c.non_ascii += 1;
                        if std::str::from_utf8(raw).is_err() {
                            c.not_utf8 += 1;
                        }
                    }
                }
            }
            if m.name.contains("audio") || m.name == "surnameid" {
                audio.push(format!("{} ({} bits)", m.name, f.bit_width));
            }
        }
        let summary: Vec<String> = cols
            .iter()
            .map(|(n, c)| {
                format!(
                    "{n} {} / {} / {} / {} / {}{}",
                    c.width,
                    c.filled,
                    c.keys,
                    c.non_ascii,
                    c.not_utf8,
                    if c.key_samples.is_empty() {
                        String::new()
                    } else {
                        format!(" (keys e.g. {:?})", c.key_samples)
                    }
                )
            })
            .collect();
        eprintln!(
            "| {platform}/{game} | {} | {} |",
            summary.join("; "),
            audio.join(", ")
        );
        // Direct biography strings: populated names, never a dictionary key, always UTF-8.
        for (n, c) in &cols {
            if n != "birthcity" {
                assert_eq!(c.keys, 0, "{platform}/{game}: {n} carries `$` keys");
            } else {
                // Measured: eight PS4 NHL 17–21 rows store the birth city as the token `$BURNABY`.
                let expected = if platform == "ps4"
                    && matches!(
                        game.as_str(),
                        "nhl17" | "nhl18" | "nhl19" | "nhl20" | "nhl21"
                    ) {
                    8
                } else {
                    0
                };
                assert_eq!(c.keys, expected, "{platform}/{game}: birth-city `$` tokens");
            }
            assert_eq!(
                c.not_utf8, 0,
                "{platform}/{game}: {n} carries non-UTF-8 bytes"
            );
        }
        assert!(
            cols["firstname"].filled > 1000 && cols["lastname"].filled > 1000,
            "{platform}/{game}: names populated"
        );
        // The audio-name reference is a separate numeric column on every title.
        assert!(
            audio.iter().any(|a| a.starts_with("audioid")),
            "{platform}/{game}: audioid present"
        );
        if game == "nhl21" {
            assert!(
                audio.iter().any(|a| a.starts_with("surnameid")),
                "{platform}/{game}: NHL 21 surname id"
            );
        }
        if game == "nhl07" {
            assert!(
                audio.iter().any(|a| a.starts_with("defaultaudioid")),
                "{platform}/{game}: NHL 07 default audio id"
            );
        }
        if platform == "ps4" {
            assert!(
                cols.contains_key("nickname"),
                "{platform}/{game}: PS4 nickname column"
            );
        }
    }
}

/// The `$BURNABY` birth-city token on PS4 NHL 17–21: which players carry it, and whether the
/// title's own dictionary resolves the key.
#[test]
fn the_burnaby_birth_city_token_and_its_dictionary_lookup() {
    use studio_domain::leagues::AttachedDictionary;
    use studio_domain::text_keys::resolve_key_text;
    for game in ["nhl17", "nhl18", "nhl19", "nhl20", "nhl21"] {
        let dir = repo(&format!("anchors/ps4/{game}/databases"));
        let bytes = std::fs::read(dir.join("nhl_game.db")).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let bio = db.table_canonical("ubPc").unwrap();
        let by_id = |id: &str| {
            bio.fields
                .iter()
                .find(|f| f.id.canonical(db.endian) == id)
                .unwrap()
        };
        let (first, last, city) = (by_id("HdeP"), by_id("QbMR"), by_id("MFzJ"));
        let mut names = Vec::new();
        for rec in 0..bio.current_records as u32 {
            let c = db
                .read_inline_str(bio, rec, city)
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            if c.starts_with('$') {
                let f = db
                    .read_inline_str(bio, rec, first)
                    .map(|b| String::from_utf8_lossy(b).into_owned())
                    .unwrap_or_default();
                let l = db
                    .read_inline_str(bio, rec, last)
                    .map(|b| String::from_utf8_lossy(b).into_owned())
                    .unwrap_or_default();
                names.push(format!("{f} {l} ({c})"));
            }
        }
        let dict_path = dir.join("eng_us.db");
        let lookup = if dict_path.is_file() {
            let dict = AttachedDictionary::load(&dict_path.to_string_lossy()).unwrap();
            let k = resolve_key_text(Some(&dict), "$BURNABY");
            format!(
                "{} → {:?} ({})",
                k.lookup.as_str(),
                k.text,
                k.row_key.clone().unwrap_or_default()
            )
        } else {
            "no dictionary anchor".to_string()
        };
        eprintln!(
            "ps4/{game}: {} rows — {}; dictionary: {lookup}",
            names.len(),
            names.join(", ")
        );
        assert_eq!(names.len(), 8, "ps4/{game}");
    }
}

//! LOC-W2 — in-place free-text re-encode on the committed dictionaries: a shorter text fits and
//! round-trips byte-exactly in length, the rest of the dictionary is untouched, refusals name
//! unsupported characters (NHL 10's upper-case tree) and over-length texts, and every anchor's
//! tree lacks `~` (research P1.3-X: the only printable ASCII absent from every tree; `` ` `` and
//! `|` are present on a few titles).

use std::path::{Path, PathBuf};

use tdb_loc::{apply_retext, plan_retext, tree_alphabet, verify, LocDictionary};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

#[test]
fn legacy_norway_becomes_nordic_in_place() {
    let bytes = std::fs::read(repo("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db")).unwrap();
    let before = LocDictionary::parse(&bytes).unwrap();
    assert_eq!(before.text("TXT_LEAGUE_NORWAY"), Some("Norway"));
    // Same-length text with tree symbols only.
    let plan = plan_retext(&bytes, "TXT_LEAGUE_NORWAY", "Nordic").unwrap();
    assert!(plan.new_extent_bytes <= plan.old_extent_bytes, "{plan:?}");
    assert!(plan.sharing_text.is_empty());
    let out = apply_retext(&bytes, &plan).unwrap();
    assert_eq!(out.bytes.len(), bytes.len());
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.text("TXT_LEAGUE_NORWAY"), Some("Nordic"));
    assert_eq!(after.text("TXT_ROW_LEAGUE"), Some("Rest Of World"));
    assert_eq!(after.len(), before.len());
    // Every other row identical.
    for (a, b) in before.rows.iter().zip(after.rows.iter()) {
        if a.key.as_deref() != Some("TXT_LEAGUE_NORWAY") {
            assert_eq!(a.text, b.text);
        }
    }
    assert!(verify(&out.bytes).unwrap().iter().all(|c| c.matches()));
    // Only the string's own bytes and CRC words differ.
    let region_bytes: usize = out
        .changed_ranges
        .iter()
        .filter(|r| r.start >= plan.pool_start + plan.text_pointer as usize && r.end <= plan.pool_start + plan.text_pointer as usize + plan.old_extent_bytes)
        .map(|r| r.end - r.start)
        .sum();
    assert!(region_bytes > 0 && region_bytes <= plan.old_extent_bytes);
    assert!(!out.crc_scopes_changed.is_empty());
    // A longer text that does not fit is refused with the numbers.
    let err = plan_retext(&bytes, "TXT_LEAGUE_NORWAY", "Kingdom of Norway Hockey League").unwrap_err().to_string();
    assert!(err.contains("growth path"), "{err}");
    // An unencodable character is named.
    let err = plan_retext(&bytes, "TXT_LEAGUE_NORWAY", "N~rway").unwrap_err().to_string();
    assert!(err.contains("no symbol for '~'"), "{err}");
    // Same text: nothing to change.
    assert!(plan_retext(&bytes, "TXT_LEAGUE_NORWAY", "Norway").is_err());
    // A stale plan is refused after the bytes changed.
    assert!(apply_retext(&out.bytes, &plan).is_err());
}

#[test]
fn nhl10_upper_case_tree_refuses_lowercase_and_accepts_caps() {
    let bytes = std::fs::read(repo("anchors/xbox360/nhl10/databases/nhl_eng_us.db")).unwrap();
    let alpha = tree_alphabet(&bytes).unwrap();
    assert!(alpha.contains(&b'D') && alpha.contains(&b's') && !alpha.contains(&b'a'), "{:?}", String::from_utf8_lossy(&alpha));
    let d = LocDictionary::parse(&bytes).unwrap();
    let old = d.text("TXT_LEAGUE_SWISS_LEAGUE").unwrap().to_string();
    assert_eq!(old, "NATIONAL LEAGUE");
    let err = plan_retext(&bytes, "TXT_LEAGUE_SWISS_LEAGUE", "Swiss League").unwrap_err().to_string();
    assert!(err.contains("no symbol for"), "{err}");
    let plan = plan_retext(&bytes, "TXT_LEAGUE_SWISS_LEAGUE", "SWISS LEAGUE").unwrap();
    let out = apply_retext(&bytes, &plan).unwrap();
    assert_eq!(LocDictionary::parse(&out.bytes).unwrap().text("TXT_LEAGUE_SWISS_LEAGUE"), Some("SWISS LEAGUE"));
}

#[test]
fn ps4_little_endian_dictionary_re_encodes_too() {
    // The anchor committed under nhl20 is the NHL 19 base dictionary (REG-1); little-endian PS4.
    let bytes = std::fs::read(repo("anchors/ps4/nhl20/databases/eng_us.db")).unwrap();
    let d = LocDictionary::parse(&bytes).unwrap();
    assert_eq!(d.text("TXT_ROW_LEAGUE"), Some("Rest Of World"));
    let plan = plan_retext(&bytes, "TXT_ROW_LEAGUE", "Rest Of Earth").unwrap();
    assert!(!plan.endian_big);
    let out = apply_retext(&bytes, &plan).unwrap();
    assert_eq!(out.bytes.len(), bytes.len());
    assert_eq!(LocDictionary::parse(&out.bytes).unwrap().text("TXT_ROW_LEAGUE"), Some("Rest Of Earth"));
    assert!(verify(&out.bytes).unwrap().iter().all(|c| c.matches()));
}

#[test]
fn every_committed_tree_lacks_tilde() {
    for platform in ["ps3", "xbox360", "ps4"] {
        let root = repo("anchors").join(platform);
        for game in std::fs::read_dir(&root).unwrap().flatten() {
            for name in ["nhl_eng_us.db", "eng_us.db"] {
                let p = game.path().join("databases").join(name);
                if !p.is_file() {
                    continue;
                }
                let bytes = std::fs::read(&p).unwrap();
                let alpha = tree_alphabet(&bytes).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                assert!(!alpha.contains(&b'~'), "{}: tree encodes '~'", p.display());
                assert!(alpha.contains(&b' ') && alpha.contains(&b'E'), "{}", p.display());
            }
        }
    }
}

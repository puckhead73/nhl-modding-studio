//! P1-BS-1 — the declaration-scoped offset-binary sites of the NHL 15 PS4 settings catalogue
//! (`stocksettings.min` / `.id`, `stocksettingcontext.subgroupparent` / `.subgrouporder`).
//!
//! Evidence (research handoff 22 rev. 3 §1.2 / §1.9): the two `stocksettings` sites are witnessed
//! by the executable's compiled twin of the table (222/223 `id`, 222/223 `min`) and by the table's
//! own size arithmetic (213/223 rows `size == max - min + 1`); `subgroupparent` is inferred by
//! lineage from the NHL 16 group parents; `subgrouporder` is **structural only** and is carried
//! with that mark, not as an externally confirmed reading. Nothing here establishes a rule for any
//! other negative-`rangelow` field.
//!
//! Sources: the committed NHL 15 PS4 anchor (`ae9d1d56…`) and, `--ignored`, the private patch 01.06
//! database (`abe36f58…`) under `NHL_STUDIO_FIXTURES` (`databases/nhl15-ps4-cusa00561/patch_inxp00.big/
//! nhl_game.db`, extracted from `patch_inxp00.big` `629091b6…` entry 0; the patch archive carries no
//! `nhl_game-meta.xml` member, so the anchor metadata `d9cf1be4…` is used for both copies).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tdb_core::TdbFile;
use tdb_defs::{FieldMetaIndex, OffsetEvidence, SCOPED_OFFSET_BINARY_SITES};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

const NHL15_DB_SHA: &str = "ae9d1d56bf896b83b3376cc179e5bf3a6986cb594880f43de8a81e5b7e5078d8";
const NHL15_META_SHA: &str = "d9cf1be40ffc8180c173e460bf3a120e6b719b0fcd70a939b8c232d07ac085c3";
const NHL15_PATCH_DB_SHA: &str = "abe36f58fd0a65ab074aff13f83f2e671b707afc539ee57c01df128200c320ff";

fn nhl15_meta() -> FieldMetaIndex {
    let xml = std::fs::read(repo("anchors/ps4/nhl15/databases/nhl_game-meta.xml")).unwrap();
    assert_eq!(sha256(&xml), NHL15_META_SHA, "NHL 15 PS4 metadata is not the recorded anchor");
    FieldMetaIndex::from_xml(std::str::from_utf8(&xml).unwrap()).unwrap()
}

fn nhl15_anchor() -> Vec<u8> {
    let bytes = std::fs::read(repo("anchors/ps4/nhl15/databases/nhl_game.db")).unwrap();
    assert_eq!(sha256(&bytes), NHL15_DB_SHA, "NHL 15 PS4 database is not the recorded anchor");
    bytes
}

/// One decoded `stocksettings` row (logical values through `FieldMeta`).
#[derive(Debug, Clone)]
struct Setting {
    legacyid: i64,
    stringid: String,
    id: i64,
    min: i64,
    max: i64,
    size: i64,
    default: i64,
}

fn read_settings(db: &TdbFile, meta: &FieldMetaIndex) -> Vec<Setting> {
    let t = db.table("fRvu").expect("stocksettings");
    let f = |raw: &str| t.field(raw).unwrap().clone();
    let (jjhq, sahv, ckjp, sbeu, kodr, qrwm, nphf) = (f("jjHq"), f("sAhV"), f("CkjP"), f("SbEu"), f("KODr"), f("qrwM"), f("npHF"));
    let m = |raw: &str| meta.field("fRvu", raw).unwrap();
    (0..t.current_records as u32)
        .map(|r| Setting {
            legacyid: m("jjHq").decode_raw(db.read_u64(t, r, &jjhq).unwrap()),
            stringid: String::from_utf8_lossy(db.read_inline_str(t, r, &sahv).unwrap()).into_owned(),
            id: m("CkjP").decode_raw(db.read_u64(t, r, &ckjp).unwrap()),
            min: m("SbEu").decode_raw(db.read_u64(t, r, &sbeu).unwrap()),
            max: m("KODr").decode_raw(db.read_u64(t, r, &kodr).unwrap()),
            size: m("qrwM").decode_raw(db.read_u64(t, r, &qrwm).unwrap()),
            default: m("npHF").decode_raw(db.read_u64(t, r, &nphf).unwrap()),
        })
        .collect()
}

/// One decoded `stocksettingcontext` row.
#[derive(Debug, Clone)]
struct ContextRow {
    context: i64,
    group: i64,
    subgroupparent: i64,
    subgrouporder: i64,
}

fn read_contexts(db: &TdbFile, meta: &FieldMetaIndex) -> Vec<ContextRow> {
    let t = db.table("HwRX").expect("stocksettingcontext");
    let f = |raw: &str| t.field(raw).unwrap().clone();
    let (mmia, oaja, qhrr, nvbd) = (f("mmiA"), f("OAja"), f("QHRR"), f("NVBd"));
    let m = |raw: &str| meta.field("HwRX", raw).unwrap();
    (0..t.current_records as u32)
        .map(|r| ContextRow {
            context: m("mmiA").decode_raw(db.read_u64(t, r, &mmia).unwrap()),
            group: m("OAja").decode_raw(db.read_u64(t, r, &oaja).unwrap()),
            subgroupparent: m("QHRR").decode_raw(db.read_u64(t, r, &qhrr).unwrap()),
            subgrouporder: m("NVBd").decode_raw(db.read_u64(t, r, &nvbd).unwrap()),
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// 1. Site registration: table + field + declaration shape, with the evidence level preserved.
// ---------------------------------------------------------------------------------------------

#[test]
fn the_four_sites_are_offset_binary_on_the_nhl15_ps4_declaration_with_their_evidence_levels() {
    let meta = nhl15_meta();
    let expect = [
        ("fRvu", "SbEu", OffsetEvidence::ExecutableWitnessed),
        ("fRvu", "CkjP", OffsetEvidence::ExecutableWitnessed),
        ("HwRX", "QHRR", OffsetEvidence::LineageInferred),
        ("HwRX", "NVBd", OffsetEvidence::StructuralOnly),
    ];
    for (t, f, ev) in expect {
        let m = meta.field(t, f).unwrap();
        assert!(m.offset_binary, "{t}.{f} must take the offset reading");
        assert_eq!(m.offset_evidence, Some(ev), "{t}.{f} evidence level");
        assert!(!m.signed() && !m.neg1_sentinel(), "{t}.{f} is neither two's complement nor a sentinel");
    }
    // The structural-only site is exactly one and it is `subgrouporder`: the distinction is kept.
    let structural: Vec<_> = SCOPED_OFFSET_BINARY_SITES.iter().filter(|s| s.evidence == OffsetEvidence::StructuralOnly).collect();
    assert_eq!(structural.len(), 1);
    assert_eq!((structural[0].table, structural[0].field), ("HwRX", "NVBd"));
    assert!(OffsetEvidence::StructuralOnly.label().contains("unverified"));
}

#[test]
fn the_unrelated_ckjp_field_of_leaguesimpopups_and_the_other_catalogue_fields_are_untouched() {
    let meta = nhl15_meta();
    // `qbPU.CkjP` = leaguesimpopups.id, 14-bit 0..16383: the raw field id collides, the table does
    // not — plain unsigned, raw == value.
    let m = meta.field("qbPU", "CkjP").unwrap();
    assert!(!m.offset_binary && m.offset_evidence.is_none() && !m.signed() && !m.neg1_sentinel());
    assert_eq!((m.bit_depth, m.range_low, m.range_high), (14, Some(0), Some(16383)));
    for raw in [0u64, 1, 16382, 16383] {
        assert_eq!(m.decode_raw(raw), raw as i64);
        assert_eq!(m.encode(raw as i64), raw);
    }
    // The non-negative fields of the same two tables stay plain unsigned.
    for (t, f) in [("fRvu", "jjHq"), ("fRvu", "npHF"), ("fRvu", "qrwM"), ("fRvu", "KODr"), ("fRvu", "cVLR"), ("HwRX", "mmiA"), ("HwRX", "OAja"), ("HwRX", "ORkb")] {
        let m = meta.field(t, f).unwrap();
        assert!(!m.offset_binary && m.offset_evidence.is_none(), "{t}.{f}");
    }
    // The three other catalogue tables have no negative range at all and no site.
    for t in ["gZtX", "Oreg", "FzWx"] {
        for f in ["jjHq", "mmiA", "npHF", "OAja", "ORkb", "ylMD"] {
            if let Some(m) = meta.field(t, f) {
                assert!(!m.offset_binary && m.range_low == Some(0), "{t}.{f}");
            }
        }
    }
}

#[test]
fn the_anchored_plus_minus_sites_and_a_twos_complement_slider_keep_their_readings() {
    // The career plus/minus tables are gen7 (NHL 15 PS4 has none): check them on the PS3 Legacy
    // anchor, where both sites exist, then sweep NHL 15 PS4's negative fields.
    let legacy = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo("anchors/ps3/nhllegacy/databases/nhlng-meta.xml")).unwrap()).unwrap();
    for (t, f, lo, hi) in [("ySst", "tmoU", -64, 127), ("OqAO", "tmoU", -32, 31)] {
        let m = legacy.field(t, f).unwrap_or_else(|| panic!("{t}.{f} missing on PS3 Legacy"));
        assert!(m.offset_binary, "{t}.{f}");
        assert_eq!(m.offset_evidence, Some(OffsetEvidence::Anchored), "{t}.{f}");
        assert_eq!((m.range_low, m.range_high), (Some(lo), Some(hi)));
        // raw = value - range_low, unchanged from before this package (Stastny +5 = raw 69).
        assert_eq!(m.decode_raw(m.encode(5)), 5);
        assert_eq!(m.encode(0), (-lo) as u64);
    }
    assert_eq!(legacy.field("ySst", "tmoU").unwrap().decode_raw(69), 5);
    assert!(legacy.field("fRvu", "SbEu").is_none(), "Legacy declares no stocksettings table");
    let meta = nhl15_meta();
    // Every other negative-rangelow integer field of this title stays two's complement or a
    // sentinel, exactly as before: the scoped list adds nothing outside its four sites.
    let xml = std::fs::read_to_string(repo("anchors/ps4/nhl15/databases/nhl_game-meta.xml")).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let mut negative = 0;
    let mut twos = 0;
    let mut sentinels = 0;
    let mut plain = 0;
    for table in doc.descendants().filter(|n| n.has_tag_name("table")) {
        let tsn = table.attribute("shortname").unwrap();
        for field in table.descendants().filter(|n| n.has_tag_name("field")) {
            let fsn = field.attribute("shortname").unwrap();
            let m = meta.field(tsn, fsn).unwrap();
            if m.range_low.map_or(true, |lo| lo >= 0) {
                continue;
            }
            negative += 1;
            if m.offset_binary {
                assert!(
                    [("ySst", "tmoU"), ("OqAO", "tmoU"), ("fRvu", "SbEu"), ("fRvu", "CkjP"), ("HwRX", "QHRR"), ("HwRX", "NVBd")].contains(&(tsn, fsn)),
                    "{tsn}.{fsn} took the offset reading without being a listed site"
                );
                continue;
            }
            if m.signed() {
                twos += 1;
                // A signed slider round-trips its whole range through two's complement.
                for v in [m.range_low.unwrap(), -1, 0, m.range_high.unwrap()] {
                    assert_eq!(m.decode_raw(m.encode(v)), v, "{tsn}.{fsn} {v}");
                }
            } else if m.neg1_sentinel() {
                sentinels += 1;
                assert_eq!(m.decode_raw((1u64 << m.bit_depth) - 1), -1, "{tsn}.{fsn}");
            } else {
                // Neither reading applies (e.g. `jrow.zwIO` 11-bit -512..1535): raw is returned as
                // is, exactly as before this package — not promoted to offset-binary here.
                plain += 1;
                assert_eq!(m.decode_raw(5), 5, "{tsn}.{fsn}");
            }
        }
    }
    assert!(negative > 4 && twos > 0 && sentinels > 0, "negative {negative}, two's {twos}, sentinels {sentinels}, plain {plain}");
    eprintln!("NHL 15 PS4: {negative} negative-rangelow fields — 4 offset-binary (scoped), {twos} two's complement, {sentinels} -1 sentinels, {plain} plain");
}

/// The scoped sites are bound to the NHL 15 PS4 declaration: no other committed anchor's metadata
/// takes them (the tables do not exist there), and the anchored plus/minus sites are the only
/// offset-binary fields elsewhere.
#[test]
fn no_other_committed_anchor_metadata_takes_a_scoped_site() {
    let mut seen = 0;
    for entry in walk_metas(&repo("anchors")) {
        let xml = std::fs::read_to_string(&entry).unwrap();
        let meta = FieldMetaIndex::from_xml(&xml).unwrap();
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let is_nhl15_ps4 = entry.to_string_lossy().replace('\\', "/").contains("/ps4/nhl15/");
        for table in doc.descendants().filter(|n| n.has_tag_name("table")) {
            let tsn = table.attribute("shortname").unwrap();
            for field in table.descendants().filter(|n| n.has_tag_name("field")) {
                let fsn = field.attribute("shortname").unwrap();
                let m = meta.field(tsn, fsn).unwrap();
                match m.offset_evidence {
                    None => assert!(!m.offset_binary),
                    Some(OffsetEvidence::Anchored) => assert!(matches!((tsn, fsn), ("ySst", "tmoU") | ("OqAO", "tmoU")), "{}: {tsn}.{fsn}", entry.display()),
                    Some(_) => assert!(is_nhl15_ps4, "{}: {tsn}.{fsn} took a scoped site outside NHL 15 PS4", entry.display()),
                }
            }
        }
        seen += 1;
    }
    assert!(seen >= 27, "only {seen} metadata files scanned");
}

fn walk_metas(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(walk_metas(&p));
        } else if p.file_name().map_or(false, |n| n.to_string_lossy().ends_with("-meta.xml")) {
            out.push(p);
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// 2. Declaration mismatches: the same raw ids under a different shape are NOT offset-binary.
// ---------------------------------------------------------------------------------------------

fn synthetic_meta(tables: &[(&str, &[(&str, u32, i64, i64)])]) -> FieldMetaIndex {
    let mut xml = String::from("<database>");
    for (tsn, fields) in tables {
        xml.push_str(&format!("<table name=\"t_{tsn}\" shortname=\"{tsn}\"><fields>"));
        for (fsn, depth, lo, hi) in *fields {
            xml.push_str(&format!(
                "<field name=\"f_{fsn}\" shortname=\"{fsn}\" type=\"FIELDTYPE_INTEGER\" depth=\"{depth}\" rangelow=\"{lo}\" rangehigh=\"{hi}\" />"
            ));
        }
        xml.push_str("</fields></table>");
    }
    xml.push_str("</database>");
    FieldMetaIndex::from_xml(&xml).unwrap()
}

const STOCKSETTINGS_SHAPE: &[(&str, u32, i64, i64)] = &[
    ("jjHq", 8, 0, 255),
    ("cVLR", 2, 0, 3),
    ("npHF", 3, 0, 7),
    ("sAhV", 400, 0, 0),
    ("CkjP", 10, -1, 511),
    ("SbEu", 4, -8, 7),
    ("qrwM", 5, 0, 31),
    ("KODr", 5, 0, 31),
];

const CONTEXT_SHAPE: &[(&str, u32, i64, i64)] = &[
    ("mmiA", 6, 0, 63),
    ("OAja", 7, 0, 127),
    ("QHRR", 7, -1, 126),
    ("ORkb", 3, 0, 7),
    ("NVBd", 5, -1, 15),
];

#[test]
fn an_exact_synthetic_declaration_matches_and_field_order_does_not_matter() {
    let meta = synthetic_meta(&[("fRvu", STOCKSETTINGS_SHAPE), ("HwRX", CONTEXT_SHAPE)]);
    assert!(meta.field("fRvu", "SbEu").unwrap().offset_binary);
    assert!(meta.field("fRvu", "CkjP").unwrap().offset_binary);
    assert!(meta.field("HwRX", "QHRR").unwrap().offset_binary);
    assert!(meta.field("HwRX", "NVBd").unwrap().offset_binary);
    let mut reversed: Vec<_> = STOCKSETTINGS_SHAPE.to_vec();
    reversed.reverse();
    let meta = synthetic_meta(&[("fRvu", &reversed)]);
    assert!(meta.field("fRvu", "SbEu").unwrap().offset_binary && meta.field("fRvu", "CkjP").unwrap().offset_binary);
}

#[test]
fn a_different_declaration_shape_refuses_the_site() {
    // (a) the site's own depth differs (min declared 5 bits): two's complement as before.
    let mut shape: Vec<_> = STOCKSETTINGS_SHAPE.to_vec();
    shape[5] = ("SbEu", 5, -8, 7);
    let meta = synthetic_meta(&[("fRvu", &shape)]);
    let m = meta.field("fRvu", "SbEu").unwrap();
    assert!(!m.offset_binary && m.offset_evidence.is_none() && m.signed());
    assert_eq!(m.decode_raw(0b11000), -8);
    // (b) the site's own range differs (id declared -1..1023): not the NHL 15 declaration.
    let mut shape: Vec<_> = STOCKSETTINGS_SHAPE.to_vec();
    shape[4] = ("CkjP", 10, -1, 1023);
    let meta = synthetic_meta(&[("fRvu", &shape)]);
    let m = meta.field("fRvu", "CkjP").unwrap();
    assert!(!m.offset_binary && m.neg1_sentinel());
    assert_eq!(m.decode_raw(1023), -1);
    // (c) a sibling field of the table differs (default widened to 4 bits): the whole table's
    //     shape is the key, so neither site takes the reading.
    let mut shape: Vec<_> = STOCKSETTINGS_SHAPE.to_vec();
    shape[2] = ("npHF", 4, 0, 10);
    let meta = synthetic_meta(&[("fRvu", &shape)]);
    assert!(!meta.field("fRvu", "SbEu").unwrap().offset_binary);
    assert!(!meta.field("fRvu", "CkjP").unwrap().offset_binary);
    // (d) an extra field: not the declaration either.
    let mut shape: Vec<_> = STOCKSETTINGS_SHAPE.to_vec();
    shape.push(("zzZz", 1, 0, 1));
    let meta = synthetic_meta(&[("fRvu", &shape)]);
    assert!(!meta.field("fRvu", "SbEu").unwrap().offset_binary);
    // (e) a missing field.
    let shape: Vec<_> = STOCKSETTINGS_SHAPE[..7].to_vec();
    let meta = synthetic_meta(&[("fRvu", &shape)]);
    assert!(!meta.field("fRvu", "SbEu").unwrap().offset_binary);
    // (f) the exact stocksettings shape under another table id: the table is part of the key.
    let meta = synthetic_meta(&[("xxXx", STOCKSETTINGS_SHAPE)]);
    let m = meta.field("xxXx", "SbEu").unwrap();
    assert!(!m.offset_binary && m.signed());
    let m = meta.field("xxXx", "CkjP").unwrap();
    assert!(!m.offset_binary && m.signed(), "-1..511 fits 10 signed bits: two's complement, as before");
    // (g) the context table with `subgrouporder` re-ranged: both context sites refuse.
    let mut shape: Vec<_> = CONTEXT_SHAPE.to_vec();
    shape[4] = ("NVBd", 5, 0, 31);
    let meta = synthetic_meta(&[("HwRX", &shape)]);
    assert!(!meta.field("HwRX", "QHRR").unwrap().offset_binary);
    assert!(!meta.field("HwRX", "NVBd").unwrap().offset_binary);
}

// ---------------------------------------------------------------------------------------------
// 3. Decode / encode inverses and bounds on the four sites.
// ---------------------------------------------------------------------------------------------

#[test]
fn raw_to_logical_to_raw_is_the_identity_on_every_raw_value_of_each_site() {
    let meta = nhl15_meta();
    for (t, f, lo, hi) in [("fRvu", "SbEu", -8i64, 7i64), ("fRvu", "CkjP", -1, 511), ("HwRX", "QHRR", -1, 126), ("HwRX", "NVBd", -1, 15)] {
        let m = meta.field(t, f).unwrap();
        let w = m.bit_depth;
        for raw in 0..(1u64 << w) {
            let v = m.decode_raw(raw);
            assert_eq!(v, raw as i64 + lo, "{t}.{f} raw {raw}");
            assert_eq!(m.encode(v), raw, "{t}.{f} value {v}");
        }
        // Logical bounds: the legal range maps exactly onto the raw span; the encoder masks to
        // the width, so validation (not encoding) is what keeps a value inside it.
        assert_eq!(m.encode(lo), 0);
        assert!(m.validate(lo).is_ok() && m.validate(hi).is_ok());
        assert!(m.validate(lo - 1).is_err() && m.validate(hi + 1).is_err());
        // No all-ones sentinel: the top raw value is `raw + lo`, which is `hi` where the legal
        // range fills the width (`min`, `subgroupparent`) and beyond it otherwise (`id` 1023 → 1022,
        // `subgrouporder` 31 → 30) — such a raw value is out of range, never "-1".
        assert_eq!(m.decode_raw((1u64 << w) - 1), (1i64 << w) - 1 + lo);
        assert!(m.decode_raw((1u64 << w) - 1) >= hi);
    }
    // The handoff's worked examples.
    let min = meta.field("fRvu", "SbEu").unwrap();
    assert_eq!((min.decode_raw(8), min.decode_raw(11), min.decode_raw(1), min.decode_raw(9)), (0, 3, -7, 1));
    let id = meta.field("fRvu", "CkjP").unwrap();
    assert_eq!(id.decode_raw(153), 152);
    let parent = meta.field("HwRX", "QHRR").unwrap();
    assert_eq!((parent.decode_raw(37), parent.decode_raw(0), parent.decode_raw(75), parent.decode_raw(76)), (36, -1, 74, 75));
    let order = meta.field("HwRX", "NVBd").unwrap();
    assert_eq!((order.decode_raw(0), order.decode_raw(2), order.decode_raw(7)), (-1, 1, 6));
}

// ---------------------------------------------------------------------------------------------
// 4. The NHL 15 PS4 anchor through the real parser (handoff 22 §7 T1 / T2 / T3).
// ---------------------------------------------------------------------------------------------

fn assert_catalogue_shape(db: &TdbFile, rows: [u16; 5]) {
    for (raw, n) in ["fRvu", "gZtX", "HwRX", "Oreg", "FzWx"].iter().zip(rows) {
        let t = db.table(raw).unwrap();
        assert_eq!((t.current_records, t.max_records), (n, n), "{raw} packed row count");
    }
}

fn assert_settings_decode(settings: &[Setting]) {
    assert_eq!(settings.len(), 223);
    // legacyid 0..222 contiguous, key-sorted.
    assert!(settings.iter().enumerate().all(|(i, s)| s.legacyid == i as i64));
    let consistent = settings.iter().filter(|s| s.size == s.max - s.min + 1).count();
    assert_eq!(consistent, 213, "size == max - min + 1 under the offset reading");
    assert_eq!(settings.iter().filter(|s| s.min < 0).count(), 2, "exactly the two advantage sliders are negative");
    let by_name = |n: &str| settings.iter().filter(|s| s.stringid == n).collect::<Vec<_>>();
    let adv = by_name("PlayerAdvantage");
    assert_eq!((adv[0].min, adv[0].max, adv[0].size), (-7, 7, 15));
    let gadv = by_name("GoalieAdvantage");
    assert_eq!((gadv[0].min, gadv[0].max, gadv[0].size), (-7, 7, 15));
    let pl = by_name("PeriodLength");
    assert_eq!((pl[0].min, pl[0].max, pl[0].size, pl[0].default), (3, 20, 18, 5));
    let mud = by_name("TXT_MUD_MAX_USERS");
    assert_eq!((mud[0].min, mud[0].max, mud[0].size), (1, 24, 25));
    for lid in [72, 86, 116] {
        let s = &settings[lid];
        assert_eq!((s.stringid.as_str(), s.id, s.min, s.max, s.size), ("PlayoffSeriesLength", 152, 0, 4, 5), "legacyid {lid}");
    }
    assert_eq!(settings[68].id, 18, "SeasonTieBreak shares id 18");
    assert_eq!(settings[71].id, 155);
    assert_eq!(settings[73].id, 156);
    assert_eq!(settings[70].id, 162);
    // SHOOTING_CONTROL_TOGGLE has id -1: raw 0 under the offset reading.
    let toggle = by_name("SHOOTING_CONTROL_TOGGLE");
    assert_eq!(toggle.len(), 1);
    assert_eq!(toggle[0].id, -1);
    // Every Phase 1 eligible row has min 0 (code = value = index).
    for lid in [67, 68, 69, 70, 71, 72, 73, 77, 83, 85, 86, 87, 88, 89, 116, 117, 118, 119, 120, 123, 212] {
        assert_eq!(settings[lid].min, 0, "legacyid {lid} min");
        assert_eq!(settings[lid].size, settings[lid].max + 1, "legacyid {lid} size");
    }
}

fn assert_contexts_decode(rows: &[ContextRow], expected_rows: usize) {
    assert_eq!(rows.len(), expected_rows);
    let mut parents: BTreeMap<i64, usize> = BTreeMap::new();
    let mut orders: BTreeMap<i64, usize> = BTreeMap::new();
    for r in rows {
        *parents.entry(r.subgroupparent).or_default() += 1;
        *orders.entry(r.subgrouporder).or_default() += 1;
    }
    assert_eq!(parents.keys().copied().collect::<Vec<_>>(), [-1, 36, 74, 75], "subgroupparent value set");
    assert!(orders.keys().all(|&o| o == -1 || (1..=6).contains(&o)), "subgrouporder ∈ {{-1, 1..6}}: {orders:?}");
    // Only the sub-group rows under container 36 carry an order (1..6); rows without a parent and
    // the rows under the two main-screen containers 74/75 read -1 (structural correspondence
    // between the two inferred sites; the executable says nothing about either).
    for r in rows {
        assert_eq!(r.subgroupparent == 36, r.subgrouporder != -1, "{r:?}");
    }
    // Group 36's children are the six sub-groups 50..58 (handoff 22 §1.4, NHL 16 lineage).
    let under_36: Vec<i64> = rows.iter().filter(|r| r.subgroupparent == 36).map(|r| r.group).collect();
    assert!(under_36.iter().all(|g| (50..=58).contains(g)), "{under_36:?}");
}

#[test]
fn nhl15_ps4_anchor_decodes_the_catalogue_with_the_offset_reading() {
    let bytes = nhl15_anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = nhl15_meta();
    assert_catalogue_shape(&db, [223, 373, 197, 702, 25]);
    let settings = read_settings(&db, &meta);
    assert_settings_decode(&settings);
    let ctx = read_contexts(&db, &meta);
    assert_contexts_decode(&ctx, 197);
    assert_eq!(ctx.iter().filter(|r| r.subgroupparent == 36).count(), 36);
    assert_eq!(ctx.iter().filter(|r| r.subgroupparent == 74).count(), 4);
    assert_eq!(ctx.iter().filter(|r| r.subgroupparent == 75).count(), 2);
    // Base-copy witnesses that the patch changes (T3): legacyid 82 is still id 158, 0..1, size 2.
    assert_eq!((settings[82].stringid.as_str(), settings[82].id, settings[82].max, settings[82].size), ("NumberOfTeams", 158, 1, 2));
    // Context 9 has no defaults rows in the base copy.
    let defaults = db.table("gZtX").unwrap();
    let (fc, fl) = (defaults.field("mmiA").unwrap().clone(), defaults.field("jjHq").unwrap().clone());
    let ctx9 = (0..defaults.current_records as u32).filter(|&r| db.read_u64(defaults, r, &fc).unwrap() == 9).count();
    assert_eq!(ctx9, 0);
    // T2 joins (as decoded through the real parser): the dangling legacyid 236 ×4 in contexts 49–52.
    let dangling: Vec<(u64, u64)> = (0..defaults.current_records as u32)
        .map(|r| (db.read_u64(defaults, r, &fc).unwrap(), db.read_u64(defaults, r, &fl).unwrap()))
        .filter(|(_, l)| *l as usize >= settings.len())
        .collect();
    assert_eq!(dangling, [(49, 236), (50, 236), (51, 236), (52, 236)]);
    // NHL 15 PS4 declares no career plus/minus tables; the anchored sites are checked on the PS3
    // Legacy anchor below (`anchored_plus_minus_sites_still_decode_in_range_on_legacy`).
    assert!(db.table("ySst").is_none() && db.table("OqAO").is_none());
    // The colliding `leaguesimpopups.id` decodes raw == value on every row.
    let pop = db.table("qbPU").unwrap();
    let fid = pop.field("CkjP").unwrap().clone();
    let m = meta.field("qbPU", "CkjP").unwrap();
    for r in 0..pop.current_records as u32 {
        let raw = db.read_u64(pop, r, &fid).unwrap();
        assert_eq!(m.decode_raw(raw), raw as i64);
    }
}

/// The private NHL 15 PS4 patch 01.06 database: the same declaration (anchor metadata), the
/// patched row counts and the re-pointed `NumberOfTeams` row (handoff 22 §1.6 / §7 T3).
#[test]
#[ignore = "private fixture databases/nhl15-ps4-cusa00561/patch_inxp00.big/nhl_game.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl15_ps4_patch_01_06_database_decodes_with_the_same_scoped_sites() {
    let root = std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private NHL 15 PS4 patch fixture cannot be resolved");
    let path = PathBuf::from(root).join("databases/nhl15-ps4-cusa00561/patch_inxp00.big/nhl_game.db");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    assert!(!bytes.is_empty(), "{} is empty", path.display());
    assert_eq!(sha256(&bytes), NHL15_PATCH_DB_SHA, "{} is not the recorded patch 01.06 database", path.display());
    assert_eq!(bytes.len(), 16_981_204);
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = nhl15_meta();
    assert_catalogue_shape(&db, [223, 379, 201, 704, 25]);
    let settings = read_settings(&db, &meta);
    assert_settings_decode(&settings);
    // legacyid 82 NumberOfTeams re-pointed to id 186 with 0..3, size 4.
    assert_eq!((settings[82].stringid.as_str(), settings[82].id, settings[82].min, settings[82].max, settings[82].size), ("NumberOfTeams", 186, 0, 3, 4));
    let ctx = read_contexts(&db, &meta);
    assert_contexts_decode(&ctx, 201);
    // Six context-9 defaults at records 193..=198 (key order), values as recorded.
    let defaults = db.table("gZtX").unwrap();
    let (fc, fl, fd) = (defaults.field("mmiA").unwrap().clone(), defaults.field("jjHq").unwrap().clone(), defaults.field("npHF").unwrap().clone());
    let ctx9: Vec<(u32, u64, u64)> = (0..defaults.current_records as u32)
        .filter(|&r| db.read_u64(defaults, r, &fc).unwrap() == 9)
        .map(|r| (r, db.read_u64(defaults, r, &fl).unwrap(), db.read_u64(defaults, r, &fd).unwrap()))
        .collect();
    assert_eq!(ctx9.iter().map(|c| c.0).collect::<Vec<_>>(), [193, 194, 195, 196, 197, 198]);
    let by_id: BTreeMap<u64, u64> = ctx9.iter().map(|c| (c.1, c.2)).collect();
    assert_eq!(by_id.get(&73), Some(&0), "PlayoffTieBreak");
    assert_eq!(by_id.get(&82), Some(&3), "NumberOfTeams");
    assert_eq!(by_id.get(&86), Some(&3), "PlayoffSeriesLength (tournament)");
    assert_eq!(by_id.get(&3), Some(&5), "PeriodLength");
    // Key order is preserved throughout the grown table.
    let keys: Vec<(u64, u64)> = (0..defaults.current_records as u32).map(|r| (db.read_u64(defaults, r, &fc).unwrap(), db.read_u64(defaults, r, &fl).unwrap())).collect();
    assert!(keys.windows(2).all(|w| w[0] <= w[1]), "stocksettingdefaults is key-sorted");
}

/// The pre-existing anchored sites keep decoding real gen7 data inside their legal range.
#[test]
fn anchored_plus_minus_sites_still_decode_in_range_on_legacy() {
    let bytes = std::fs::read(repo("anchors/ps3/nhllegacy/databases/nhlng.db")).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo("anchors/ps3/nhllegacy/databases/nhlng-meta.xml")).unwrap()).unwrap();
    let mut checked = 0;
    for (t, f) in [("ySst", "tmoU"), ("OqAO", "tmoU")] {
        let table = db.table(t).unwrap();
        let fd = table.field(f).unwrap().clone();
        let m = meta.field(t, f).unwrap();
        assert_eq!(m.offset_evidence, Some(OffsetEvidence::Anchored));
        for r in 0..table.current_records as u32 {
            let v = m.decode_raw(db.read_u64(table, r, &fd).unwrap());
            assert!(v >= m.range_low.unwrap() && v <= m.range_high.unwrap(), "{t}.{f} rec {r}: {v}");
            checked += 1;
        }
    }
    assert!(checked > 100, "{checked}");
}

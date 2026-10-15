//! P1-D — per-title / platform / locale coverage of the dictionary operations, each file with its
//! own tree and provenance: LOC-W1 re-point, LOC-W2 in-place re-text, LOC-W3 no-op identity and
//! growth (text growth everywhere; insertion on class-A files, refused on class B). The committed
//! anchors (25 `eng_us`) run by default; the 100 loose locale files of the Xbox 360 installs and the
//! PS3 disc trees (`<root>/loc/<platform>-<title>/*.db`, identities in `loc/manifest.json`) and the
//! six hash-bound install dictionaries run `--ignored` under `NHL_STUDIO_FIXTURES`. A PS3 file is
//! its own dictionary here, never the Xbox file standing in for it.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tdb_loc::{apply_growth, apply_repoint, apply_retext, plan_growth, plan_repoint, plan_retext, verify, GrowthOp, LocDictionary, OrderClass};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

/// Every operation on one dictionary file, each verified independently. Returns the order class.
fn exercise(label: &str, bytes: &[u8]) -> OrderClass {
    for c in verify(bytes).unwrap_or_else(|e| panic!("{label}: {e}")) {
        assert!(c.matches(), "{label}: CRC scope {} stored {:08x} computed {:08x}", c.name, c.stored, c.computed);
    }
    assert_eq!(tdb_core::TdbFile::parse(bytes).unwrap().serialize(), bytes, "{label}: round-trip invariant");
    let d = LocDictionary::parse(bytes).unwrap_or_else(|e| panic!("{label}: {e}"));
    let class = OrderClass::of(&d.rows);

    // LOC-W1: re-point the first text row at the first row with a different text.
    let a = d.rows.iter().position(|r| r.key.is_some() && r.text.is_some()).unwrap();
    let b = d.rows.iter().enumerate().position(|(i, r)| i != a && r.key.is_some() && r.text.is_some() && r.text != d.rows[a].text).unwrap();
    let (ka, kb) = (d.rows[a].key.clone().unwrap(), d.rows[b].key.clone().unwrap());
    let plan = plan_repoint(bytes, &ka, &kb).unwrap_or_else(|e| panic!("{label}: repoint {e}"));
    let out = apply_repoint(bytes, &plan).unwrap_or_else(|e| panic!("{label}: repoint apply {e}"));
    assert_eq!(out.bytes.len(), bytes.len(), "{label}");
    assert_eq!(LocDictionary::parse(&out.bytes).unwrap().text(&ka), d.text(&kb), "{label}: repoint");

    // LOC-W2: re-encode a text in place with a strict prefix of itself (always fits, always in the
    // tree). Pick a row whose text has ≥ 3 characters.
    let (kr, tr) = d.rows.iter().find_map(|r| match (&r.key, &r.text) {
        (Some(k), Some(t)) if t.chars().count() >= 3 => Some((k.clone(), t.clone())),
        _ => None,
    }).unwrap_or_else(|| panic!("{label}: no re-textable row"));
    let shorter: String = tr.chars().take(tr.chars().count() - 1).collect();
    let plan = plan_retext(bytes, &kr, &shorter).unwrap_or_else(|e| panic!("{label}: retext {e}"));
    assert!(plan.new_extent_bytes <= plan.old_extent_bytes, "{label}");
    let out = apply_retext(bytes, &plan).unwrap_or_else(|e| panic!("{label}: retext apply {e}"));
    assert_eq!(out.bytes.len(), bytes.len(), "{label}: in place");
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.text(&kr), Some(shorter.as_str()), "{label}: retext");
    assert_eq!(after.len(), d.len());
    assert!(verify(&out.bytes).unwrap().iter().all(|c| c.matches()));
    // Every other row identical (the sharers of the string, if any, were reported by the plan).
    for (x, y) in d.rows.iter().zip(&after.rows) {
        if x.key.as_deref() != Some(&kr) && !plan.sharing_text.iter().any(|s| Some(&s.key) == x.key.as_ref()) {
            assert_eq!(x, y, "{label}: row changed");
        }
    }

    // LOC-W3 P: no-op identity, then text growth (the same text doubled: only tree symbols), then
    // on class A an insertion at its sorted position.
    let noop = plan_growth(bytes, &[]).unwrap_or_else(|e| panic!("{label}: growth noop {e}"));
    assert_eq!(apply_growth(bytes, &noop).unwrap().bytes, bytes, "{label}: P no-op identity");
    let doubled = format!("{tr}{tr}");
    let mut ops = vec![GrowthOp::TextGrow { key: kr.clone(), new_text: doubled.clone(), all_sharers: false }];
    let new_key = (0..10_000u32).map(|i| format!("STUDIO_COVERAGE_{i}")).find(|k| !d.rows.iter().any(|r| r.hash == tdb_loc::loc_hash(k))).unwrap();
    // The new key's characters must be in the tree: use only symbols the file already encodes
    // (uppercase letters, digits and `_` exist on every measured tree — NHL 10's upper-case tree
    // included); if not, skip the insertion for this file and say so.
    let alphabet = tdb_loc::tree_alphabet(bytes).unwrap();
    let key_ok = new_key.bytes().all(|b| alphabet.contains(&b));
    if matches!(class, OrderClass::A) && key_ok {
        ops.push(GrowthOp::InsertKey { key: new_key.clone(), text: Some(shorter.clone()) });
    }
    let plan = plan_growth(bytes, &ops).unwrap_or_else(|e| panic!("{label}: growth {e}"));
    let out = apply_growth(bytes, &plan).unwrap_or_else(|e| panic!("{label}: growth apply {e}"));
    let after = LocDictionary::parse(&out.bytes).unwrap();
    assert_eq!(after.text(&kr), Some(doubled.as_str()), "{label}: growth text");
    assert_eq!(OrderClass::of(&after.rows), class, "{label}: order class preserved");
    if matches!(class, OrderClass::A) && key_ok {
        assert_eq!(after.len(), d.len() + 1);
        assert_eq!(after.text(&new_key), Some(shorter.as_str()), "{label}: inserted key");
    } else {
        assert_eq!(after.len(), d.len());
        if matches!(class, OrderClass::B { .. }) {
            let r = plan_growth(bytes, &[GrowthOp::InsertKey { key: new_key.clone(), text: None }]).unwrap_err();
            assert_eq!(r.kind, if key_ok { "baseline-order-anomalous" } else { "needs-rebuild" }, "{label}");
        }
    }
    class
}

#[test]
fn every_committed_dictionary_takes_repoint_retext_and_growth() {
    let mut n = 0;
    for platform in ["ps3", "xbox360"] {
        for game in ["nhl07", "nhl08", "nhl09", "nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15", "nhllegacy"] {
            let rel = format!("anchors/{platform}/{game}/databases/nhl_eng_us.db");
            exercise(&rel, &std::fs::read(repo(&rel)).unwrap());
            n += 1;
        }
    }
    for game in ["nhl15", "nhl16", "nhl17", "nhl18", "nhl20"] {
        let rel = format!("anchors/ps4/{game}/databases/eng_us.db");
        exercise(&rel, &std::fs::read(repo(&rel)).unwrap());
        n += 1;
    }
    assert_eq!(n, 25);
}

fn fixture_root() -> PathBuf {
    PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private locale fixtures cannot be resolved"))
}

/// The 100 loose locale dictionaries: 53 Xbox 360 (07–Legacy incl. `lng_lg`) and 47 PS3 (08–Legacy),
/// each with its own tree; identities recorded in the manifest at extraction.
#[test]
#[ignore = "private fixtures loc/<platform>-<title>/*.db + loc/manifest.json: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_loose_locale_dictionary_takes_repoint_retext_and_growth() {
    let root = fixture_root().join("loc");
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap_or_else(|e| panic!("cannot read {}: {e}", root.join("manifest.json").display()))).unwrap();
    let mut n = 0;
    let mut classes = std::collections::BTreeMap::new();
    for (dir, files) in manifest.as_object().unwrap() {
        for (name, meta) in files.as_object().unwrap() {
            let path = root.join(dir).join(name);
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            assert!(!bytes.is_empty(), "{}", path.display());
            assert_eq!(format!("{:x}", Sha256::digest(&bytes)), meta["sha256"].as_str().unwrap(), "{dir}/{name}: fixture identity");
            let class = exercise(&format!("{dir}/{name}"), &bytes);
            *classes.entry(format!("{dir} {}", class.as_str())).or_insert(0usize) += 1;
            n += 1;
        }
    }
    assert_eq!(n, 100, "the locale census: 53 Xbox 360 + 47 PS3 loose files");
    // Research 20 B2: NHL 08/09/10 (every locale) and the `lng_lg` template are class B, the rest A.
    for (k, v) in &classes {
        let b_expected = k.contains("nhl08") || k.contains("nhl09") || k.contains("nhl10");
        if k.ends_with(" B") {
            assert!(b_expected || *v <= 1, "{k}: {v} class-B files (only NHL 08/09/10 and lng_lg are class B)");
        }
    }
    eprintln!("locale sweep: {classes:?}");
}

/// The six hash-bound install dictionaries (NHL 19 base/patch, 20, 21 PS4; NHL 08/09 PS3 disc).
#[test]
#[ignore = "private fixtures databases/*/…/eng_us.db and nhl_eng_us.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_private_install_dictionary_takes_repoint_retext_and_growth() {
    let root = fixture_root();
    let sources = [
        "databases/nhl20-ps4-cusa15439/inxp_00.big/eng_us.db",
        "databases/nhl19-ps4-cusa11126/inxp_00.big/eng_us.db",
        "databases/nhl19-ps4-cusa11126/patch_inxp_56.big/eng_us.db",
        "databases/nhl21-ps4-cusa18182/inxp_00.big/eng_us.db",
        "databases/nhl08-ps3-blus30062/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db",
        "databases/nhl09-ps3-blus30168/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db",
    ];
    for rel in sources {
        let path = root.join(rel);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(!bytes.is_empty(), "{rel}");
        let class = exercise(rel, &bytes);
        // Research 20 census: the PS3 NHL 08 disc dictionary has 0 descents (class A — insertion
        // offered) although the "ps3 anchor" (an Xbox file) has 2; PS3 NHL 09 has 2 (class B).
        // The PS4 install files are class A.
        let expect_b = rel.contains("nhl09");
        assert_eq!(matches!(class, OrderClass::B { .. }), expect_b, "{rel}: class {}", class.as_str());
    }
}

/// C7 (overnight 2026-09-20): the 99 PS4 locale dictionaries (seven locales per title — `cze_cz`,
/// `eng_us`, `fin_fi`, `fre_fr`, `ger_de`, `rus_ru`, `swe_se`; NHL 21 adds `lng_lg` — of the seven
/// installs' `inxp_00.big`, plus the copies in NHL 15's `patch_inxp00.big` and NHL 19's six
/// `patch_inxp_5x.big`), extracted by the developer generator: each takes re-point, re-text and
/// growth with its own tree, and the canonical rebuild's no-op is the input minus its slack while a
/// new-glyph rebuild decodes. Every file is class A (research 20 B2 for PS4).
#[test]
#[ignore = "private fixtures loc-ps4/<title>/<archive>/*.db + loc-ps4/manifest.json: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_ps4_locale_dictionary_takes_every_operation_including_the_rebuild() {
    let root = fixture_root().join("loc-ps4");
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap_or_else(|e| panic!("cannot read {}: {e}", root.join("manifest.json").display()))).unwrap();
    let mut n = 0;
    let mut locales = std::collections::BTreeSet::new();
    for (dir, files) in manifest.as_object().unwrap() {
        for (name, meta) in files.as_object().unwrap() {
            let path = root.join(dir).join(name);
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            assert!(!bytes.is_empty(), "{}", path.display());
            assert_eq!(format!("{:x}", Sha256::digest(&bytes)), meta["sha256"].as_str().unwrap(), "{dir}/{name}: fixture identity");
            let label = format!("{dir}/{name}");
            let class = exercise(&label, &bytes);
            assert!(matches!(class, OrderClass::A), "{label}: PS4 dictionaries are class A");
            // Canonical rebuild: no-op identity minus slack; new glyph decodes.
            let plan = tdb_loc::plan_rebuild(&bytes, &[], false).unwrap_or_else(|e| panic!("{label}: {e:?}"));
            let out = tdb_loc::apply_rebuild(&bytes, &plan).unwrap_or_else(|e| panic!("{label}: {e:?}"));
            assert!(out.accounting.pool_prefix_identical, "{label}: repacked pool == input minus slack");
            assert!(out.accounting.discarded.iter().all(|d| d.kind == "trailing-slack"), "{label}: {:?}", out.accounting.discarded);
            assert_eq!(LocDictionary::parse(&bytes).unwrap().rows, LocDictionary::parse(&out.bytes).unwrap().rows, "{label}");
            let d = LocDictionary::parse(&bytes).unwrap();
            let first = d.rows.iter().find(|r| r.key.is_some() && r.text.is_some()).unwrap().key.clone().unwrap();
            let plan = tdb_loc::plan_rebuild(&bytes, &[GrowthOp::TextGrow { key: first.clone(), new_text: "Studio ~ probe".into(), all_sharers: false }], false).unwrap_or_else(|e| panic!("{label}: {e:?}"));
            let out = tdb_loc::apply_rebuild(&bytes, &plan).unwrap_or_else(|e| panic!("{label}: {e:?}"));
            assert_eq!(LocDictionary::parse(&out.bytes).unwrap().text(&first), Some("Studio ~ probe"), "{label}");
            locales.insert(name.clone());
            n += 1;
        }
    }
    assert_eq!(n, 99, "the developer extraction: 99 PS4 locale members in 14 archives");
    assert!(locales.contains("fre_fr.db") && locales.contains("rus_ru.db") && locales.contains("lng_lg.db"));
    eprintln!("ps4 locale sweep: {n} files, locales {locales:?}");
}

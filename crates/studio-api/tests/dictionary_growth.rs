//! LOC-W3 through the [`Registry`] seam: a growth (a longer text + a new key) held as the pending
//! dictionary change, listed in the pending review, written by the copied-target save into (a) a
//! raw archive member (Xbox 360 `cacheboot.big` layout — the member grows, so the stager appends
//! and retargets), (b) a `chunkzip` member re-encoded with the native encoder (the PS4
//! `inxp_00.big` shape), and (c) the loose file by in-place save; restore returns the original
//! bytes. Synthetic archives in the stager's qualified layout; nothing here is evidence a game
//! accepts the result.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::tdb_txn::FileIdentity;
use tdb_archive_big::stage::{synth::build_archive, BoundSource, Placement};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn sha(path: &Path) -> String {
    FileIdentity::of_file(path).unwrap().sha256
}

const DB: &str = "anchors/ps3/nhllegacy/databases/nhlng.db";
const META: &str = "anchors/ps3/nhllegacy/databases/nhlng-meta.xml";
const DICT_X360: &str = "anchors/xbox360/nhllegacy/databases/nhl_eng_us.db";
const DB_PS4: &str = "anchors/ps4/nhl18/databases/nhl_game.db";
const META_PS4: &str = "anchors/ps4/nhl18/databases/nhl_game-meta.xml";
const DICT_PS4: &str = "anchors/ps4/nhl18/databases/eng_us.db";

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-locw3-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn ops(key: &str, text: &str, new_key: &str) -> Vec<studio_api::dto::DictionaryGrowthOpDto> {
    vec![
        studio_api::dto::DictionaryGrowthOpDto { kind: "text".into(), key: key.into(), text: Some(text.into()), all_sharers: false },
        studio_api::dto::DictionaryGrowthOpDto { kind: "insert".into(), key: new_key.into(), text: Some("Studio League".into()), all_sharers: false },
    ]
}

#[test]
fn growth_flows_through_pending_review_a_raw_archive_member_and_restore() {
    let tmp = Tmp::new("x360");
    let root = tmp.0.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    std::fs::create_dir_all(root.join("db")).unwrap();
    std::fs::create_dir_all(root.join("fe/loc")).unwrap();
    let db = root.join("db/nhlng.db");
    let dict = root.join("fe/loc/nhl_eng_us.db");
    std::fs::copy(repo(DB), &db).unwrap();
    std::fs::copy(repo(DICT_X360), &dict).unwrap();
    let dict_bytes = std::fs::read(&dict).unwrap();
    // The dictionary member is followed immediately by another member: a grown member cannot fit.
    let archive_bytes = build_archive(
        &["db", "fe/loc", "rendering"],
        &[
            (0, "nhlng.db", std::fs::read(&db).unwrap()),
            (1, "nhl_eng_us.db", dict_bytes.clone()),
            (2, "blocker.rpsgl", vec![7u8; 300]),
        ],
    );
    let archive = root.join("cacheboot.big");
    std::fs::write(&archive, &archive_bytes).unwrap();
    let archive_sha = sha(&archive);

    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhllegacy"), parse_platform("xbox360").unwrap(), &s(&db), None, &s(&repo(META)), &s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id, &s(&dict)).unwrap();
    // Preview: the text grows (append), the key is inserted at its sorted position.
    let p = reg.dictionary_growth_preview(id, &ops("TXT_LEAGUE_NORWAY", "Kingdom of Norway (Studio)", "TXT_LEAGUE_STUDIO_NEW")).unwrap();
    assert_eq!((p.order_class.as_str(), p.rows_after, p.inserts.len(), p.edits.len()), ("A", p.rows_before + 1, 1, 1));
    assert!(p.warnings.iter().any(|w| w.contains("runtime gate R-A")), "{:?}", p.warnings);
    // C5 glyph gate, static half (research 2026-09-21 handoff 09): plain Latin text is inside the
    // shipped fonts of xbox360-nhllegacy; a C1 control is refused on input.
    assert!(p.warnings.iter().any(|w| w.contains("glyph gate (static): every character") && w.contains("xbox360-nhllegacy")), "{:?}", p.warnings);
    let e = match reg.dictionary_growth_preview(id, &ops("TXT_LEAGUE_NORWAY", "Norway\u{81}", "TXT_LEAGUE_STUDIO_NEW")) {
        Ok(_) => panic!("a C1 control must be refused on input"),
        Err(e) => e.to_string(),
    };
    assert!(e.contains("C1 control") && e.contains("U+0081"), "{e}");
    assert!(!reg.summary(id).unwrap().dirty, "a preview holds nothing");
    // Hold.
    let held = reg.dictionary_growth_set(id, &ops("TXT_LEAGUE_NORWAY", "Kingdom of Norway (Studio)", "TXT_LEAGUE_STUDIO_NEW")).unwrap();
    assert_eq!(held.appended_bytes, p.appended_bytes);
    assert!(reg.summary(id).unwrap().dirty);
    let pending = reg.pending_changes(id, 50).unwrap();
    let d = pending.dictionary.as_ref().expect("pending dictionary growth");
    assert_eq!(d.kind, "growth");
    assert_eq!((d.key.as_str(), d.before.as_str(), d.after.as_str()), ("TXT_LEAGUE_NORWAY", "Norway", "Kingdom of Norway (Studio)"));
    assert!(d.changed_bytes > 16, "{}", d.changed_bytes);
    // A second growth replaces the pending one (one pending dictionary operation at a time).
    let held2 = reg.dictionary_growth_set(id, &ops("TXT_LEAGUE_NORWAY", "Norway (Studio)", "TXT_LEAGUE_STUDIO_NEW")).unwrap();
    assert_ne!(held2.appended_bytes, held.appended_bytes);
    assert_eq!(reg.pending_changes(id, 50).unwrap().dictionary.unwrap().after, "Norway (Studio)");
    // The labels resolve through the pending working dictionary.
    let cat = reg.list_leagues(id).unwrap();
    assert!(cat.leagues.iter().any(|l| l.display.contains("Norway (Studio)")), "{:?}", cat.leagues.iter().map(|l| l.display.clone()).collect::<Vec<_>>());

    // Copied-target save with both members named: the grown dictionary member is appended and
    // its row retargeted; the database member (unchanged bytes) is a no-op.
    let review = reg
        .copied_target_review_archive_members(
            id,
            &s(&root),
            vec![
                studio_api::dto::ArchiveMemberSpecDto { archive_relative: "cacheboot.big".into(), member_path: "db/nhlng.db".into(), artifact_key: "primary".into() },
                studio_api::dto::ArchiveMemberSpecDto { archive_relative: "cacheboot.big".into(), member_path: "fe/loc/nhl_eng_us.db".into(), artifact_key: "dictionary".into() },
            ],
        )
        .unwrap();
    let arch = review.artifacts.iter().find(|a| a.key == "archive").unwrap();
    assert!(arch.output.len > archive_bytes.len() as u64, "append-and-retarget grows the archive");
    let dict_art = review.artifacts.iter().find(|a| a.key == "dictionary").unwrap();
    let dict_out = (dict_art.output.len, dict_art.output.sha256.clone());
    assert!(dict_out.0 > dict_bytes.len() as u64);
    let applied = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let out = std::fs::read(&archive).unwrap();
    let bound = BoundSource::bind(&mut std::io::Cursor::new(out.clone())).unwrap();
    let m = bound.entries.iter().find(|e| e.path == "fe/loc/nhl_eng_us.db").unwrap();
    let member = &out[m.offset as usize..m.offset as usize + m.size as usize];
    assert_eq!(FileIdentity::of_bytes(member).sha256, dict_out.1);
    let grown = tdb_loc::LocDictionary::parse(member).unwrap();
    assert_eq!(grown.text("TXT_LEAGUE_NORWAY"), Some("Norway (Studio)"));
    assert_eq!(grown.text("TXT_LEAGUE_STUDIO_NEW"), Some("Studio League"));
    assert_eq!(tdb_loc::OrderClass::of(&grown.rows), tdb_loc::OrderClass::A);
    // The other members are byte-identical; the old dictionary bytes are still in place (dead).
    let blk = bound.entries.iter().find(|e| e.path == "rendering/blocker.rpsgl").unwrap();
    assert_eq!(&out[blk.offset as usize..blk.offset as usize + 300], &[7u8; 300][..]);
    let dbm = bound.entries.iter().find(|e| e.path == "db/nhlng.db").unwrap();
    assert_eq!(&out[dbm.offset as usize..dbm.offset as usize + dbm.size as usize], &std::fs::read(&db).unwrap()[..]);
    let old = BoundSource::bind(&mut std::io::Cursor::new(archive_bytes.clone())).unwrap();
    let om = old.entries.iter().find(|e| e.path == "fe/loc/nhl_eng_us.db").unwrap();
    assert_eq!(&out[om.offset as usize..om.offset as usize + om.size as usize], &dict_bytes[..], "old bytes left in place");
    assert_eq!(sha(&dict), dict_out.1, "the loose copy was replaced too");
    // The session adopted the grown dictionary as its baseline: clean, and a further edit plans on it.
    assert!(!reg.summary(id).unwrap().dirty);
    let p2 = reg.dictionary_growth_preview(id, &vec![studio_api::dto::DictionaryGrowthOpDto { kind: "text".into(), key: "TXT_LEAGUE_STUDIO_NEW".into(), text: Some("Studio League, renamed".into()), all_sharers: false }]).unwrap();
    assert!(p2.studio_touched || p2.rows_before == grown.len());
    // Restore.
    let restored = reg.copied_target_restore(&s(&root), &applied.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(sha(&archive), archive_sha);
    assert_eq!(FileIdentity::of_bytes(&dict_bytes).sha256, sha(&dict));
}

#[test]
fn growth_replaces_a_chunkzip_dictionary_member_with_the_native_encoder() {
    let tmp = Tmp::new("ps4");
    let root = tmp.0.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let db_dir = root.join("nhl/data/common/gamemodes/database/t3db");
    let loc_dir = root.join("nhl/data/kettle/loc");
    std::fs::create_dir_all(&db_dir).unwrap();
    std::fs::create_dir_all(&loc_dir).unwrap();
    let db = db_dir.join("nhl_game.db");
    let dict = loc_dir.join("eng_us.db");
    std::fs::copy(repo(DB_PS4), &db).unwrap();
    std::fs::copy(repo(DICT_PS4), &dict).unwrap();
    let dict_bytes = std::fs::read(&dict).unwrap();
    let stored = tdb_archive_codec::compress_chunkzip_native(&dict_bytes);
    let archive_bytes = build_archive(
        &["nhl/data/common/gamemodes/database/t3db", "nhl/data/kettle/loc", "nhl/data/other"],
        &[(0, "nhl_game.db", std::fs::read(&db).unwrap()), (1, "eng_us.db", stored.clone()), (2, "tail.bin", vec![9u8; 64])],
    );
    std::fs::create_dir_all(root.join("databigs")).unwrap();
    let archive = root.join("databigs/inxp_00.big");
    std::fs::write(&archive, &archive_bytes).unwrap();
    let archive_sha = sha(&archive);

    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhl18"), parse_platform("ps4").unwrap(), &s(&db), None, &s(&repo(META_PS4)), &s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id, &s(&dict)).unwrap();
    let d = tdb_loc::LocDictionary::parse(&dict_bytes).unwrap();
    let key = d.rows.iter().find(|r| r.text.as_deref().is_some_and(|t| t.len() > 4 && t.is_ascii())).unwrap().key.clone().unwrap();
    let longer = format!("{} (Studio)", d.text(&key).unwrap());
    reg.dictionary_growth_set(id, &ops(&key, &longer, "TXT_LEAGUE_STUDIO_NEW")).unwrap();
    let review = reg
        .copied_target_review_archive_members(
            id,
            &s(&root),
            vec![studio_api::dto::ArchiveMemberSpecDto { archive_relative: "databigs/inxp_00.big".into(), member_path: "nhl/data/kettle/loc/eng_us.db".into(), artifact_key: "dictionary".into() }],
        )
        .unwrap();
    let dict_out = review.artifacts.iter().find(|a| a.key == "dictionary").unwrap().output.sha256.clone();
    let applied = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let out = std::fs::read(&archive).unwrap();
    let bound = BoundSource::bind(&mut std::io::Cursor::new(out.clone())).unwrap();
    let m = bound.entries.iter().find(|e| e.path == "nhl/data/kettle/loc/eng_us.db").unwrap();
    assert_eq!(m.stored_form, tdb_archive_big::stage::StoredForm::ChunkZip, "the member stays chunkzip");
    let member = &out[m.offset as usize..m.offset as usize + m.size as usize];
    tdb_archive_codec::is_native_chunkzip_framing(member).unwrap();
    let decoded = tdb_archive_codec::decompress_chunkzip(member).unwrap();
    assert_eq!(FileIdentity::of_bytes(&decoded).sha256, dict_out, "decodes to the staged grown dictionary");
    let grown = tdb_loc::LocDictionary::parse(&decoded).unwrap();
    assert_eq!(grown.text(&key), Some(longer.as_str()));
    assert_eq!(grown.text("TXT_LEAGUE_STUDIO_NEW"), Some("Studio League"));
    let t = bound.entries.iter().find(|e| e.path == "nhl/data/other/tail.bin").unwrap();
    assert_eq!(&out[t.offset as usize..t.offset as usize + 64], &[9u8; 64][..]);
    // The placement is whatever the encoded length required; both are valid, the stager decided.
    let restored = reg.copied_target_restore(&s(&root), &applied.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(sha(&archive), archive_sha);
}

#[test]
fn growth_writes_the_loose_dictionary_in_place_and_the_disc_image_route_stays_same_length() {
    let tmp = Tmp::new("inplace");
    let root = tmp.0.join("copy");
    std::fs::create_dir_all(root.join("USRDIR/db")).unwrap();
    std::fs::create_dir_all(root.join("USRDIR/fe/loc")).unwrap();
    let db = root.join("USRDIR/db/nhlng.db");
    let dict = root.join("USRDIR/fe/loc/nhl_eng_us.db");
    std::fs::copy(repo(DB), &db).unwrap();
    std::fs::copy(repo("anchors/ps3/nhllegacy/databases/nhl_eng_us.db"), &dict).unwrap();
    let before = std::fs::read(&dict).unwrap();
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhllegacy"), parse_platform("ps3").unwrap(), &s(&db), None, &s(&repo(META)), &s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id, &s(&dict)).unwrap();
    reg.dictionary_growth_set(id, &ops("TXT_LEAGUE_NORWAY", "Norway, grown in place", "TXT_LEAGUE_STUDIO_NEW")).unwrap();
    reg.save_workspace(id).unwrap();
    let after = std::fs::read(&dict).unwrap();
    assert!(after.len() > before.len());
    let grown = tdb_loc::LocDictionary::parse(&after).unwrap();
    assert_eq!(grown.text("TXT_LEAGUE_NORWAY"), Some("Norway, grown in place"));
    assert_eq!(grown.text("TXT_LEAGUE_STUDIO_NEW"), Some("Studio League"));
    assert!(!reg.summary(id).unwrap().dirty);
    // A refusal on a class-B title is stated as such through the route.
    let tmp2 = Tmp::new("classb");
    let db2 = tmp2.0.join("nhlng.db");
    let dict2 = tmp2.0.join("nhl_eng_us.db");
    std::fs::copy(repo("anchors/xbox360/nhl10/databases/nhlng.db"), &db2).unwrap();
    std::fs::copy(repo("anchors/xbox360/nhl10/databases/nhl_eng_us.db"), &dict2).unwrap();
    let id2 = reg.open_workspace_from_paths(Some("nhl10"), parse_platform("xbox360").unwrap(), &s(&db2), None, &s(&repo("anchors/xbox360/nhl10/databases/nhlng-meta.xml")), &s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id2, &s(&dict2)).unwrap();
    let e = match reg.dictionary_growth_preview(id2, &vec![studio_api::dto::DictionaryGrowthOpDto { kind: "insert".into(), key: "STUDIO_NEW".into(), text: Some("X".into()), all_sharers: false }]) {
        Ok(_) => panic!("class B must refuse insertion"),
        Err(e) => e.to_string(),
    };
    assert!(e.contains("baseline-order-anomalous"), "{e}");
}

/// C5 (overnight 2026-09-20): the **canonical rebuild** through the route — the preserving path
/// refuses a text needing `~` (`needs-rebuild`), the rebuild is chosen explicitly (`serializer:
/// rebuild`), previewed with its own facts, held as the pending dictionary change (listed like any
/// growth), written by the in-place save, reopened with the new symbol, and a further preserving
/// growth on the rebuilt file works; the raw-archive-member placement takes the rebuilt bytes as
/// well (append-and-retarget) with restore. A forced tree rebuild (`rebuild-tree`) is accepted too.
#[test]
fn a_canonical_rebuild_is_explicit_previewed_held_saved_and_reopened() {
    let tmp = Tmp::new("rebuild");
    let root = tmp.0.join("copy");
    std::fs::create_dir_all(root.join("USRDIR/db")).unwrap();
    std::fs::create_dir_all(root.join("USRDIR/fe/loc")).unwrap();
    let db = root.join("USRDIR/db/nhlng.db");
    let dict = root.join("USRDIR/fe/loc/nhl_eng_us.db");
    std::fs::copy(repo(DB), &db).unwrap();
    std::fs::copy(repo("anchors/ps3/nhllegacy/databases/nhl_eng_us.db"), &dict).unwrap();
    let before = std::fs::read(&dict).unwrap();
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhllegacy"), parse_platform("ps3").unwrap(), &s(&db), None, &s(&repo(META)), &s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id, &s(&dict)).unwrap();
    let tilde_ops = ops("TXT_LEAGUE_NORWAY", "Norway ~ Nordics", "TXT_LEAGUE_STUDIO_TILDE");
    // Preserving: refused with the symbol named.
    let e = match reg.dictionary_growth_preview(id, &tilde_ops) {
        Ok(_) => panic!("the preserving path must refuse a new symbol"),
        Err(e) => e.to_string(),
    };
    assert!(e.contains("needs-rebuild") && e.contains('~'), "{e}");
    assert!(!reg.summary(id).unwrap().dirty);
    // Rebuild: explicit.
    let opt = studio_api::dto::DictionaryGrowthOptionsDto { serializer: Some("rebuild".into()) };
    let p = reg.dictionary_growth_preview_with(id, &tilde_ops, Some(&opt)).unwrap();
    assert_eq!(p.serializer, "canonical-rebuild");
    assert_eq!(p.new_symbols, vec!["~".to_string()]);
    assert!(p.warnings.iter().any(|w| w.contains("REBUILDS")));
    assert!(p.warnings.iter().any(|w| w.contains("glyph")));
    // The static gate on ps3-nhllegacy: `~` is in the shipped fonts; a Polish capital is not
    // (the measured union of research 2026-09-21 handoff 09), while `ó` is.
    assert!(p.warnings.iter().any(|w| w.contains("glyph gate (static): every character") && w.contains("ps3-nhllegacy")), "{:?}", p.warnings);
    let l_ops = ops("TXT_LEAGUE_NORWAY", "Norway \u{141}\u{f3}d", "TXT_LEAGUE_STUDIO_TILDE");
    let pl = reg.dictionary_growth_preview_with(id, &l_ops, Some(&opt)).unwrap();
    let g = pl.warnings.iter().find(|w| w.contains("glyph gate (static)")).unwrap_or_else(|| panic!("{:?}", pl.warnings));
    assert!(g.contains("'\u{141}' U+0141") && g.contains("not in any shipped font of ps3-nhllegacy") && !g.contains("U+00F3"), "{g}");
    assert!(p.rebuild_accounting.is_empty(), "accounting only after the hold");
    assert!(!reg.summary(id).unwrap().dirty);
    let held = reg.dictionary_growth_set_with(id, &tilde_ops, Some(&opt)).unwrap();
    assert_eq!(held.serializer, "canonical-rebuild");
    assert!(held.rebuild_accounting.iter().any(|l| l.contains("compression tree rebuilt")), "{:?}", held.rebuild_accounting);
    assert!(reg.summary(id).unwrap().dirty);
    let pending = reg.pending_changes(id, 100).unwrap();
    let pd = pending.dictionary.expect("pending dictionary change listed");
    assert_eq!(pd.kind, "growth");
    // An unknown serializer name is refused before anything is planned.
    let bad = studio_api::dto::DictionaryGrowthOptionsDto { serializer: Some("magic".into()) };
    assert!(reg.dictionary_growth_preview_with(id, &tilde_ops, Some(&bad)).is_err());
    // Save in place; reopen.
    reg.save_workspace(id).unwrap();
    let after = std::fs::read(&dict).unwrap();
    assert_ne!(after, before);
    let d = tdb_loc::LocDictionary::parse(&after).unwrap();
    assert_eq!(d.text("TXT_LEAGUE_NORWAY"), Some("Norway ~ Nordics"));
    assert_eq!(d.text("TXT_LEAGUE_STUDIO_TILDE"), Some("Studio League"));
    assert_eq!(d.rows.len(), tdb_loc::LocDictionary::parse(&before).unwrap().rows.len() + 1);
    assert!(!reg.summary(id).unwrap().dirty);
    for c in tdb_loc::verify(&after).unwrap() {
        assert!(c.matches());
    }
    // A preserving growth on the rebuilt file (now the baseline) works.
    reg.dictionary_growth_set(id, &vec![studio_api::dto::DictionaryGrowthOpDto { kind: "text".into(), key: "TXT_LEAGUE_STUDIO_TILDE".into(), text: Some("Studio ~ League".into()), all_sharers: false }]).unwrap();
    reg.save_workspace(id).unwrap();
    assert_eq!(tdb_loc::LocDictionary::parse(&std::fs::read(&dict).unwrap()).unwrap().text("TXT_LEAGUE_STUDIO_TILDE"), Some("Studio ~ League"));
    // Forced tree rebuild on an unchanged alphabet is accepted and decodes.
    let force = studio_api::dto::DictionaryGrowthOptionsDto { serializer: Some("rebuild-tree".into()) };
    let p = reg.dictionary_growth_preview_with(id, &vec![studio_api::dto::DictionaryGrowthOpDto { kind: "text".into(), key: "TXT_LEAGUE_NORWAY".into(), text: Some("Norway again".into()), all_sharers: false }], Some(&force)).unwrap();
    assert_eq!(p.serializer, "canonical-rebuild");
    assert!(p.new_symbols.is_empty());

    // Archive placement of a rebuilt dictionary: the raw member grows/changes → append-and-retarget.
    let tmp2 = Tmp::new("rebuild-arc");
    let root2 = tmp2.0.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root2).unwrap();
    std::fs::create_dir_all(root2.join("db")).unwrap();
    let db2 = root2.join("db/nhlng.db");
    std::fs::copy(repo(DB), &db2).unwrap();
    let dict_bytes = std::fs::read(repo(DICT_X360)).unwrap();
    let files: Vec<(u16, &str, Vec<u8>)> = vec![(0, "first.bin", b"FIRST".to_vec()), (1, "nhl_eng_us.db", dict_bytes.clone()), (0, "last.bin", vec![0xAB; 64])];
    let bytes = build_archive(&["fe/data", "fe/loc"], &files);
    let archive = root2.join("cacheboot.big");
    std::fs::write(&archive, &bytes).unwrap();
    std::fs::create_dir_all(root2.join("fe/loc")).unwrap();
    let loose = root2.join("fe/loc/nhl_eng_us.db");
    std::fs::write(&loose, &dict_bytes).unwrap();
    let id2 = reg.open_workspace_from_paths(Some("nhllegacy"), parse_platform("xbox360").unwrap(), &s(&db2), None, &s(&repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml")), &s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id2, &s(&loose)).unwrap();
    reg.dictionary_growth_set_with(id2, &ops("TXT_LEAGUE_NORWAY", "Norway ~ Nordics", "TXT_LEAGUE_STUDIO_TILDE"), Some(&opt)).unwrap();
    let review = reg
        .copied_target_review_archive_members(id2, &s(&root2), vec![studio_api::dto::ArchiveMemberSpecDto { archive_relative: "cacheboot.big".into(), member_path: "fe/loc/nhl_eng_us.db".into(), artifact_key: "dictionary".into() }])
        .unwrap();
    let apply = reg.copied_target_apply(id2, review.handle).unwrap();
    assert_eq!(apply.state, "committed", "{}", apply.state);
    let all = std::fs::read(&archive).unwrap();
    let mut cur = std::io::Cursor::new(&all);
    let bound = BoundSource::bind(&mut cur).unwrap();
    let e = bound.entries.iter().find(|e| e.path == "fe/loc/nhl_eng_us.db").unwrap();
    let member = &all[e.offset as usize..e.offset as usize + e.size as usize];
    let d2 = tdb_loc::LocDictionary::parse(member).unwrap();
    assert_eq!(d2.text("TXT_LEAGUE_NORWAY"), Some("Norway ~ Nordics"));
    let restored = reg.copied_target_restore(&s(&root2), &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(sha(&archive), sha_bytes(&bytes));
}

fn sha_bytes(b: &[u8]) -> String {
    FileIdentity::of_bytes(b).sha256
}

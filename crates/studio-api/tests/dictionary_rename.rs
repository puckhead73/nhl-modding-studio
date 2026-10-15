//! LOC-W1: the dictionary rename (an existing key re-pointed at another existing key's text)
//! through the real Registry seam on copied gen7 fixtures — preview → set → pending review →
//! copied-target review → apply → reopen → restore — as a loose file (the PS3 layout) and as a
//! raw archive member (the Xbox 360 / Recompiled `cacheboot.big` layout, synthetic archive).
//! Anchors are read only; everything lands in a temp copied root.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn s(p: &Path) -> String {
    p.to_str().unwrap().to_string()
}

fn sha(path: &Path) -> String {
    studio_domain::tdb_txn::FileIdentity::of_file(path)
        .unwrap()
        .sha256
}

const DB: &str = "anchors/ps3/nhllegacy/databases/nhlng.db";
const META: &str = "anchors/ps3/nhllegacy/databases/nhlng-meta.xml";
const DICT: &str = "anchors/ps3/nhllegacy/databases/nhl_eng_us.db";
const DB_SHA: &str = "105d2186da3d1f0f3d3ea8a350ead42ac9d5d802ca9fd37652c39bb4e1f1c230";
const DICT_SHA: &str = "708145517ee4f58c603b8f381e180e7249daf2331ebca0d0a58fc851699f8581";
const DICT_X360: &str = "anchors/xbox360/nhllegacy/databases/nhl_eng_us.db";

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-locw1-{tag}-{}", std::process::id()));
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

/// A marked copied root in the PS3 layout: `USRDIR/db/nhlng.db` + `USRDIR/fe/loc/nhl_eng_us.db`.
fn ps3_tree(base: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let root = base.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let db_dir = root.join("USRDIR/db");
    let loc_dir = root.join("USRDIR/fe/loc");
    std::fs::create_dir_all(&db_dir).unwrap();
    std::fs::create_dir_all(&loc_dir).unwrap();
    let db = db_dir.join("nhlng.db");
    let dict = loc_dir.join("nhl_eng_us.db");
    std::fs::copy(repo(DB), &db).unwrap();
    std::fs::copy(repo(DICT), &dict).unwrap();
    (root, db, dict)
}

fn open(reg: &Registry, db: &Path) -> u32 {
    reg.open_workspace_from_paths(
        Some("nhllegacy"),
        parse_platform("ps3").unwrap(),
        &s(db),
        None,
        &s(&repo(META)),
        &s(&repo("defs")),
    )
    .unwrap()
    .project_id
}

#[test]
fn rename_preview_set_pending_review_apply_reopen_restore_on_the_ps3_loose_layout() {
    let tmp = Tmp::new("ps3");
    let (root, db, dict) = ps3_tree(&tmp.0);
    let reg = Registry::default();
    let id = open(&reg, &db);

    // No dictionary yet: the rename is refused with a plain reason.
    let err = match reg.dictionary_rename_preview(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE") {
        Ok(_) => panic!("preview without a dictionary must be refused"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("no dictionary is attached"), "{err}");

    let d = reg.attach_dictionary(id, &s(&dict)).unwrap();
    assert_eq!(d.sha256, DICT_SHA);

    // Preview: read-only, nothing pending.
    let p = reg
        .dictionary_rename_preview(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE")
        .unwrap();
    assert_eq!((p.text_before.as_str(), p.text_after.as_str()), ("Norway", "Rest Of World"));
    assert_eq!(p.source_lookup, "exact");
    assert!(p.endian_big);
    assert!(p.warnings.iter().any(|w| w.contains("Xbox 360")));
    assert!(!reg.summary(id).unwrap().dirty);
    assert!(reg.pending_changes(id, 10).unwrap().dictionary.is_none());

    // Refusals: unknown key, ambiguous nothing, same row, null-safe.
    assert!(reg.dictionary_rename_preview(id, "NO_SUCH_KEY", "TXT_ROW_LEAGUE").is_err());
    assert!(reg.dictionary_rename_preview(id, "TXT_ROW_LEAGUE", "TXT_ROW_LEAGUE").is_err());

    // Set: held as the pending dictionary change; the review shows it as one entry.
    let set = reg
        .dictionary_rename_set(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE")
        .unwrap();
    assert_eq!(set.text_after, "Rest Of World");
    assert!(reg.summary(id).unwrap().dirty);
    let pending = reg.pending_changes(id, 10).unwrap();
    assert!(pending.dirty);
    assert_eq!(pending.primary.total_cells, 0, "the database is untouched");
    let pd = pending.dictionary.as_ref().expect("dictionary section");
    assert_eq!((pd.key.as_str(), pd.before.as_str(), pd.after.as_str()), ("TXT_LEAGUE_NORWAY", "Norway", "Rest Of World"));
    assert_eq!(pd.baseline_sha256, DICT_SHA);
    assert_eq!(pd.crc_scopes_changed, vec!["body"]);
    assert!(pd.changed_bytes <= 8 && pd.changed_bytes >= 5, "{}", pd.changed_bytes);
    assert_eq!(sha(&dict), DICT_SHA, "nothing written yet");

    // Clear = undo; set again.
    assert!(reg.dictionary_rename_clear(id).unwrap());
    assert!(!reg.summary(id).unwrap().dirty);
    assert!(!reg.dictionary_rename_clear(id).unwrap());
    reg.dictionary_rename_set(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();

    // Copied-target review: the primary (unchanged) and the dictionary artifact.
    let review = reg.copied_target_review(id, &s(&root)).unwrap();
    assert!(!review.live_install_allowed);
    let dict_art = review
        .artifacts
        .iter()
        .find(|a| a.key == "dictionary")
        .expect("dictionary artifact");
    assert_eq!(dict_art.kind, "dictionary");
    assert_eq!(dict_art.prior.as_ref().unwrap().sha256, DICT_SHA);
    assert_eq!(dict_art.output.sha256, pd.working_sha256);
    assert_eq!(dict_art.producer, "tdb-loc::repoint");
    assert!(dict_art
        .qualification_items
        .iter()
        .any(|q| q.subject.contains("consumers") && q.status == "unresolved"));
    // A change after review invalidates the handle (dictionary edits move the revision too).
    let stale = review.handle;
    reg.dictionary_rename_clear(id).unwrap();
    assert!(reg.copied_target_apply(id, stale).is_err());
    reg.dictionary_rename_set(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
    let review = reg.copied_target_review(id, &s(&root)).unwrap();
    let out_sha = review
        .artifacts
        .iter()
        .find(|a| a.key == "dictionary")
        .unwrap()
        .output
        .sha256
        .clone();

    // Apply: the dictionary is installed, the session adopts it, the rename is no longer pending.
    let applied = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert!(!applied.project.dirty);
    assert_eq!(sha(&dict), out_sha);
    assert_eq!(sha(&db), DB_SHA, "the database member is byte-identical");
    assert!(reg.pending_changes(id, 10).unwrap().dictionary.is_none());
    assert!(!reg.summary(id).unwrap().dirty);

    // Allowed-difference audit on the installed dictionary: exactly the slot and the body CRC.
    let before = std::fs::read(repo(DICT)).unwrap();
    let after = std::fs::read(&dict).unwrap();
    assert_eq!(before.len(), after.len());
    let diff: Vec<usize> = (0..before.len()).filter(|&i| before[i] != after[i]).collect();
    assert!(diff.len() <= 8, "{diff:?}");
    let scopes = tdb_loc::verify(&after).unwrap();
    assert!(scopes.iter().all(|c| c.matches()));
    let d2 = tdb_loc::LocDictionary::parse(&after).unwrap();
    assert_eq!(d2.text("TXT_LEAGUE_NORWAY"), Some("Rest Of World"));
    assert_eq!(d2.text("TXT_ROW_LEAGUE"), Some("Rest Of World"));
    assert_eq!(d2.len(), 25_783);

    // Reopen in a fresh registry: attaching the installed dictionary shows the new text; a second
    // rename of the same key at the same donor is now a no-op refusal.
    let reg2 = Registry::default();
    let id2 = open(&reg2, &db);
    let d2ref = reg2.attach_dictionary(id2, &s(&dict)).unwrap();
    assert_eq!(d2ref.sha256, out_sha);
    assert!(reg2
        .dictionary_rename_preview(id2, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE")
        .is_err());
    drop(reg2);

    // Restore: byte-exact, and the open session reloads the baseline dictionary.
    let restored = reg.copied_target_restore(&s(&root), &applied.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(sha(&dict), DICT_SHA);
    assert_eq!(sha(&db), DB_SHA);
    let p3 = reg
        .dictionary_rename_preview(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE")
        .unwrap();
    assert_eq!(p3.text_before, "Norway");
}

#[test]
fn rename_replaces_the_dictionary_member_of_a_16_byte_row_archive() {
    let tmp = Tmp::new("x360");
    let root = tmp.0.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    // Xbox 360 layout: loose `db/nhlng.db`, the dictionary packed raw in `cacheboot.big`
    // (member `fe/loc/nhl_eng_us.db`) and also present loose (the extracted tree the app opens).
    std::fs::create_dir_all(root.join("db")).unwrap();
    std::fs::create_dir_all(root.join("fe/loc")).unwrap();
    let db = root.join("db/nhlng.db");
    let dict = root.join("fe/loc/nhl_eng_us.db");
    std::fs::copy(repo(DB), &db).unwrap();
    std::fs::copy(repo(DICT_X360), &dict).unwrap();
    let dict_bytes = std::fs::read(&dict).unwrap();
    let archive_bytes = tdb_archive_big::stage::synth::build_archive(
        &["db", "fe/loc", "rendering"],
        &[
            (2, "blocker.rpsgl", vec![7u8; 300]),
            (0, "nhlng.db", std::fs::read(&db).unwrap()),
            (1, "nhl_eng_us.db", dict_bytes.clone()),
        ],
    );
    let archive = root.join("cacheboot.big");
    std::fs::write(&archive, &archive_bytes).unwrap();
    let archive_sha = sha(&archive);

    let reg = Registry::default();
    let id = reg
        .open_workspace_from_paths(
            Some("nhllegacy"),
            parse_platform("xbox360").unwrap(),
            &s(&db),
            None,
            &s(&repo(META)),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    reg.attach_dictionary(id, &s(&dict)).unwrap();

    // Without a pending rename the dictionary member cannot be named.
    let err = match reg.copied_target_review_archive_members(
        id,
        &s(&root),
        vec![studio_api::dto::ArchiveMemberSpecDto {
            archive_relative: "cacheboot.big".into(),
            member_path: "fe/loc/nhl_eng_us.db".into(),
            artifact_key: "dictionary".into(),
        }],
    ) {
        Ok(_) => panic!("must be refused"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("no dictionary rename is pending"), "{err}");

    reg.dictionary_rename_set(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
    // Both members of the one archive in one staged archive: the database (unchanged bytes) and
    // the dictionary (rename output).
    let review = reg
        .copied_target_review_archive_members(
            id,
            &s(&root),
            vec![
                studio_api::dto::ArchiveMemberSpecDto {
                    archive_relative: "cacheboot.big".into(),
                    member_path: "db/nhlng.db".into(),
                    artifact_key: "primary".into(),
                },
                studio_api::dto::ArchiveMemberSpecDto {
                    archive_relative: "cacheboot.big".into(),
                    member_path: "fe/loc/nhl_eng_us.db".into(),
                    artifact_key: "dictionary".into(),
                },
            ],
        )
        .unwrap();
    let keys: Vec<&str> = review.artifacts.iter().map(|a| a.key.as_str()).collect();
    assert!(keys.contains(&"primary") && keys.contains(&"dictionary") && keys.contains(&"archive"), "{keys:?}");
    let arch = review.artifacts.iter().find(|a| a.key == "archive").unwrap();
    assert_eq!(arch.prior.as_ref().unwrap().sha256, archive_sha);
    assert_eq!(arch.output.len, archive_bytes.len() as u64, "same-length member replacement");
    let dict_out = review
        .artifacts
        .iter()
        .find(|a| a.key == "dictionary")
        .unwrap()
        .output
        .sha256
        .clone();

    let applied = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(applied.state, "committed");
    // The archive member now holds the rename output; every other member is unchanged.
    let out = std::fs::read(&archive).unwrap();
    let bound = tdb_archive_big::stage::BoundSource::bind(&mut std::io::Cursor::new(out.clone())).unwrap();
    let m = bound.entries.iter().find(|e| e.path == "fe/loc/nhl_eng_us.db").unwrap();
    let member = &out[m.offset as usize..m.offset as usize + m.size as usize];
    assert_eq!(studio_domain::tdb_txn::FileIdentity::of_bytes(member).sha256, dict_out);
    assert_eq!(tdb_loc::LocDictionary::parse(member).unwrap().text("TXT_LEAGUE_NORWAY"), Some("Rest Of World"));
    let blk = bound.entries.iter().find(|e| e.path == "rendering/blocker.rpsgl").unwrap();
    assert_eq!(&out[blk.offset as usize..blk.offset as usize + 300], &[7u8; 300][..]);
    assert_eq!(sha(&dict), dict_out, "the loose copy was replaced too");

    let restored = reg.copied_target_restore(&s(&root), &applied.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(sha(&archive), archive_sha);
    assert_eq!(studio_domain::tdb_txn::FileIdentity::of_bytes(&dict_bytes).sha256, sha(&dict));
}

#[test]
fn save_in_place_writes_the_rename_and_refuses_a_changed_dictionary() {
    let tmp = Tmp::new("inplace");
    let (_root, db, dict) = ps3_tree(&tmp.0);
    let reg = Registry::default();
    let id = open(&reg, &db);
    reg.attach_dictionary(id, &s(&dict)).unwrap();
    reg.dictionary_rename_set(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
    // The file changed underneath: refused, nothing written.
    let original = std::fs::read(&dict).unwrap();
    let mut tampered = original.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    std::fs::write(&dict, &tampered).unwrap();
    let err = match reg.save_workspace(id) {
        Ok(_) => panic!("must be refused"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("changed since it was attached"), "{err}");
    std::fs::write(&dict, &original).unwrap();
    let files = reg.save_workspace(id).unwrap();
    assert_eq!(files, 2, "database + dictionary");
    assert_ne!(sha(&dict), DICT_SHA);
    assert!(!reg.summary(id).unwrap().dirty);
    assert_eq!(
        tdb_loc::LocDictionary::parse(&std::fs::read(&dict).unwrap()).unwrap().text("TXT_LEAGUE_NORWAY"),
        Some("Rest Of World")
    );
}

/// gen7 catalogs resolve their labels through the attached dictionary via the vocabulary's
/// `dictionary_key`, the pending rename shows up in the catalog, and the PS4 hash fallback
/// resolves `AHL` through the row keyed `ahl`. The code -> key evidence is resolved **per
/// target** (R1 F5): the Legacy vocabulary's `traced` class is witnessed on NHL Legacy Xbox 360's
/// executable only, so the PS3 copy of the same mapping is `by-reference`.
#[test]
fn gen7_catalog_labels_resolve_through_the_dictionary_keys_and_reflect_the_pending_rename() {
    let tmp = Tmp::new("catalog");
    let (_root, db, dict) = ps3_tree(&tmp.0);
    let reg = Registry::default();
    let id = open(&reg, &db);
    let before = reg.list_leagues(id).unwrap();
    let norway = before.leagues.iter().find(|l| l.code == 7).unwrap();
    assert_eq!(norway.label.as_deref(), Some("GET-ligaen (Norway)"));
    assert_eq!(norway.dictionary_key.as_deref(), Some("TXT_LEAGUE_NORWAY"));
    assert_eq!(
        norway.dictionary_key_evidence.as_deref(),
        Some("by-reference"),
        "PS3 inherits the Xbox 360 trace by reference"
    );
    assert!(norway.evidence.contains("traced on nhllegacy xbox"), "{}", norway.evidence);
    assert!(norway.evidence.contains("own executable was not inspected"), "{}", norway.evidence);
    assert!(before.dictionary.is_none());

    reg.attach_dictionary(id, &s(&dict)).unwrap();
    let cat = reg.list_leagues(id).unwrap();
    assert!(cat.dictionary.is_some());
    let norway = cat.leagues.iter().find(|l| l.code == 7).unwrap();
    assert_eq!(norway.label.as_deref(), Some("Norway"));
    assert!(norway.evidence.contains("binding: by-reference"), "{}", norway.evidence);
    let wc = cat.leagues.iter().find(|l| l.code == 14).unwrap();
    assert_eq!(wc.label.as_deref(), Some("NHL\u{ae}"), "Winter Classic rows bind to the NHL key");
    let ahl = cat.leagues.iter().find(|l| l.code == 1).unwrap();
    assert_eq!(ahl.label.as_deref(), Some("AHL"));
    assert!(ahl.evidence.contains("by hash"), "{}", ahl.evidence);

    reg.dictionary_rename_set(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
    let cat = reg.list_leagues(id).unwrap();
    assert_eq!(cat.leagues.iter().find(|l| l.code == 7).unwrap().label.as_deref(), Some("Rest Of World"));
    reg.dictionary_rename_clear(id).unwrap();
    assert_eq!(reg.list_leagues(id).unwrap().leagues.iter().find(|l| l.code == 7).unwrap().label.as_deref(), Some("Norway"));

    // NHL Legacy Xbox 360: the witnessed target — the same key is `traced` here.
    let regx = Registry::default();
    let idx = regx
        .open_workspace_from_paths(
            Some("nhllegacy"),
            parse_platform("xbox360").unwrap(),
            &s(&repo("anchors/xbox360/nhllegacy/databases/nhlng.db")),
            None,
            &s(&repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml")),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    regx.attach_dictionary(idx, &s(&repo(DICT_X360))).unwrap();
    let cx = regx.list_leagues(idx).unwrap();
    let nx = cx.leagues.iter().find(|l| l.code == 7).unwrap();
    assert_eq!(nx.label.as_deref(), Some("Norway"));
    assert_eq!(nx.dictionary_key_evidence.as_deref(), Some("traced"));
    assert!(nx.evidence.contains("binding: traced"), "{}", nx.evidence);
    assert!(nx.evidence.contains("default.xex"), "{}", nx.evidence);

    // Legacy Recompiled: same database bytes as Xbox 360, but not the witnessed executable.
    let regr = Registry::default();
    let idr = regr
        .open_workspace_from_paths(
            Some("legacyrecomp"),
            parse_platform("recomp").unwrap(),
            &s(&repo("anchors/xbox360/nhllegacy/databases/nhlng.db")),
            None,
            &s(&repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml")),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    let cr = regr.list_leagues(idr).unwrap();
    let nr = cr.leagues.iter().find(|l| l.code == 7).unwrap();
    assert_eq!(nr.dictionary_key.as_deref(), Some("TXT_LEAGUE_NORWAY"));
    assert_eq!(nr.dictionary_key_evidence.as_deref(), Some("by-reference"));

    // NHL 12 PS3: the key table is traced on NHL 12's own Xbox 360 executable (P1.1-X), so the
    // PS3 copy of the vocabulary shows it by reference; uppercase texts on that title's own dictionary.
    let reg12 = Registry::default();
    let id12 = reg12
        .open_workspace_from_paths(
            Some("nhl12"),
            parse_platform("ps3").unwrap(),
            &s(&repo("anchors/ps3/nhl12/databases/nhlng.db")),
            None,
            &s(&repo("anchors/ps3/nhl12/databases/nhlng-meta.xml")),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    reg12.attach_dictionary(id12, &s(&repo("anchors/ps3/nhl12/databases/nhl_eng_us.db"))).unwrap();
    let c12 = reg12.list_leagues(id12).unwrap();
    let legends = c12.leagues.iter().find(|l| l.code == 12).unwrap();
    assert_eq!(legends.label.as_deref(), Some("LEGEND TEAM"));
    assert_eq!(legends.dictionary_key_evidence.as_deref(), Some("by-reference"));
    assert!(legends.evidence.contains("binding: by-reference"), "{}", legends.evidence);
    assert!(legends.evidence.contains("traced on nhl12 xbox"), "{}", legends.evidence);
}

/// LOC-W2: an in-place free-text re-encode through the Registry — preview, hold, pending review
/// (`kind = retext`), copied-target apply (producer `tdb-loc::retext`), reopen, restore, and the
/// same hold through the in-place save. The text must fit the old string; unsupported characters
/// and longer texts are refused with the exact reason.
#[test]
fn free_text_retext_flows_through_pending_review_copied_target_and_in_place_save() {
    let tmp = Tmp::new("retext");
    let (root, db, dict) = ps3_tree(&tmp.0);
    let reg = Registry::default();
    let id = open(&reg, &db);
    reg.attach_dictionary(id, &s(&dict)).unwrap();

    let p = reg.dictionary_retext_preview(id, "TXT_LEAGUE_NORWAY", "Nordic").unwrap();
    assert_eq!((p.text_before.as_str(), p.text_after.as_str()), ("Norway", "Nordic"));
    assert!(p.new_extent_bytes <= p.old_extent_bytes);
    assert!(p.warnings.iter().any(|w| w.contains("bits available in place")), "{:?}", p.warnings);
    assert!(p.warnings.iter().any(|w| w.contains("league Norway of Norway")), "consumer disclosure: {:?}", p.warnings);
    let err = match reg.dictionary_retext_preview(id, "TXT_LEAGUE_NORWAY", "Norway Elite Hockey League") {
        Ok(_) => panic!("must not fit"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("growth path"), "{err}");
    let err = match reg.dictionary_retext_preview(id, "TXT_LEAGUE_NORWAY", "N~") {
        Ok(_) => panic!("tilde is not in the tree"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("no symbol for '~'"), "{err}");
    assert!(!reg.summary(id).unwrap().dirty, "previews change nothing");

    reg.dictionary_retext_set(id, "TXT_LEAGUE_NORWAY", "Nordic").unwrap();
    assert!(reg.summary(id).unwrap().dirty);
    let pending = reg.pending_changes(id, 10).unwrap();
    let pd = pending.dictionary.as_ref().expect("dictionary section");
    assert_eq!((pd.kind.as_str(), pd.key.as_str(), pd.before.as_str(), pd.after.as_str()), ("retext", "TXT_LEAGUE_NORWAY", "Norway", "Nordic"));
    assert!(pd.donor_key.is_empty());
    assert_eq!(reg.list_leagues(id).unwrap().leagues.iter().find(|l| l.code == 7).unwrap().label.as_deref(), Some("Nordic"));

    // Copied-target apply carries the retext producer and installs it.
    let review = reg.copied_target_review(id, &s(&root)).unwrap();
    let art = review.artifacts.iter().find(|a| a.key == "dictionary").unwrap();
    assert_eq!(art.producer, "tdb-loc::retext");
    let applied = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert!(!reg.summary(id).unwrap().dirty);
    let after = std::fs::read(&dict).unwrap();
    assert_eq!(after.len(), std::fs::read(repo(DICT)).unwrap().len());
    assert_eq!(tdb_loc::LocDictionary::parse(&after).unwrap().text("TXT_LEAGUE_NORWAY"), Some("Nordic"));
    assert!(tdb_loc::verify(&after).unwrap().iter().all(|c| c.matches()));
    let restored = reg.copied_target_restore(&s(&root), &applied.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(sha(&dict), DICT_SHA);

    // In-place save of a second retext (the workspace refreshed to the restored baseline).
    reg.dictionary_retext_set(id, "TXT_LEAGUE_NORWAY", "Norge").unwrap();
    assert_eq!(reg.save_workspace(id).unwrap(), 2);
    let after = std::fs::read(&dict).unwrap();
    assert_eq!(tdb_loc::LocDictionary::parse(&after).unwrap().text("TXT_LEAGUE_NORWAY"), Some("Norge"));
    assert!(!reg.summary(id).unwrap().dirty);
}

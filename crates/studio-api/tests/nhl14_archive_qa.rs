//! Real NHL14 cache-container deployment and restoration on a fresh owned copy.
use std::{path::{Path, PathBuf}, io::Cursor};
use studio_api::Registry;
use studio_domain::{FieldValue, Platform};

#[test]
#[ignore = "copies the real NHL14 cache archive into a fresh QA folder"]
fn database_archive_apply_reopen_restore() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ps3 = std::env::var("NHL14_QA_PLATFORM").as_deref() == Ok("ps3");
    let platform = if ps3 { Platform::Ps3 } else { Platform::Xbox };
    let source = PathBuf::from(if ps3 {
        "S:/TEMP/nhl-studio-overnight-20261003/nhl14/ps3-existing-routes-1/relocated PS3 disc/PS3_GAME/USRDIR"
    } else { "H:/TEMP/nhl-studio-overnight-20261003/nhl14/source-routes-1/relocated disc dump" });
    let root = PathBuf::from(if ps3 {
        "E:/Repositories/nhl-modding-studio/target/overnight-20261003/nhl14/ps3-archive-roundtrip-1"
    } else { "H:/TEMP/nhl-studio-overnight-20261003/nhl14/archive-roundtrip-2" });
    assert!(!root.exists());
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    std::fs::create_dir(root.join("db")).unwrap();
    let db = root.join("db/nhlng.db");
    let archive = root.join("cacheboot.big");
    let before_db = std::fs::read(source.join("db/nhlng.db")).unwrap();
    let before_archive = std::fs::read(source.join("cacheboot.big")).unwrap();
    std::fs::write(&db,&before_db).unwrap();
    std::fs::write(&archive,&before_archive).unwrap();
    let reg = Registry::default();
    let metadata = if ps3 { "anchors/ps3/nhl14/databases/nhlng-meta.xml" } else { "anchors/xbox360/nhl14/databases/nhlng-meta.xml" };
    let id = reg.open_workspace_from_paths(Some("nhl14"),platform,&db.to_string_lossy(),None,
        &repo.join(metadata).to_string_lossy(),&repo.join("defs").to_string_lossy()).unwrap().project_id;
    let team = reg.team_view(id,0).unwrap().unwrap();
    let field = team.all_fields.iter().find(|f| f.name == "fullname").unwrap();
    reg.set_field(id,&field.studio_table,team.record,&field.studio_field,FieldValue::Text("QA Ducks".into())).unwrap();
    let review = reg.copied_target_review_archive_member(id,&root.to_string_lossy(),"cacheboot.big","db/nhlng.db").unwrap();
    let applied = reg.copied_target_apply(id,review.handle).unwrap();
    assert_eq!(applied.state,"committed");
    let after = std::fs::read(&archive).unwrap();
    let bound = tdb_archive_big::stage::BoundSource::bind(&mut Cursor::new(&after)).unwrap();
    let entry = bound.entries.iter().find(|e| e.path == "db/nhlng.db").unwrap();
    let raw = after[entry.offset as usize..entry.offset as usize+entry.size as usize].to_vec();
    match tdb_archive_big::eb::decode_payload(raw).unwrap() {
        tdb_archive_big::eb::Payload::Data(bytes) => {
            assert_eq!(bytes,std::fs::read(&db).unwrap());
            assert_ne!(bytes,before_db);
            assert!(tdb_core::TdbFile::parse(&bytes).unwrap().header_crc_ok());
        }
        other => panic!("database member not decoded: {other:?}"),
    }
    let restored = reg.copied_target_restore(&root.to_string_lossy(),&applied.txn_id).unwrap();
    assert_eq!(restored.state,"restored");
    assert_eq!(std::fs::read(&archive).unwrap(),before_archive);
    assert_eq!(std::fs::read(&db).unwrap(),before_db);
    assert_eq!(std::fs::read(source.join("cacheboot.big")).unwrap(),before_archive);
    std::fs::write(root.join("qa-receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "transaction":applied.txn_id,"archive_decoded_matches_edited_db":true,"crc_ok":true,
        "archive_and_db_restored":true,"source_preserved":true,"game_verified":false
    })).unwrap()).unwrap();
}




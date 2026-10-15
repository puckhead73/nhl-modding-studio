//! Real NHL21 cache-container deployment and restoration on a fresh owned copy.
use std::{path::{Path, PathBuf}, io::Cursor};
use studio_api::Registry;
use studio_domain::{FieldValue, Platform};

#[test]
#[ignore = "copies the real NHL21 cache archive into a fresh QA folder"]
fn database_archive_apply_reopen_restore() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let platform = Platform::Ps4;
    let source = PathBuf::from("H:/Emulators/ShadPS4/Games/CUSA18182/databigs");
    let root = repo.join("target/overnight-20261003/nhl21/ps4-archive-roundtrip-1");
    assert!(!root.exists());
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    std::fs::create_dir_all(root.join("nhl/data/common/gamemodes/database/t3db")).unwrap();
    let db = root.join("nhl/data/common/gamemodes/database/t3db/nhl_game.db");
    let archive = root.join("inxp_00.big");
    let before_db = std::fs::read(PathBuf::from("H:/Emulators/ShadPS4/Games/CUSA18182/databigs/Extracted/nhl/data/common/gamemodes/database/t3db/nhl_game.db")).unwrap();
    let before_archive = std::fs::read(source.join("inxp_00.big")).unwrap();
    std::fs::write(&db,&before_db).unwrap();
    std::fs::write(&archive,&before_archive).unwrap();
    let reg = Registry::default();
    let metadata = "H:/Emulators/ShadPS4/Games/CUSA18182/databigs/Extracted/nhl/data/common/gamemodes/database/t3db/nhl_game-meta.xml";
    let id = reg.open_workspace_from_paths(Some("nhl21"),platform,&db.to_string_lossy(),None,
        &repo.join(metadata).to_string_lossy(),&repo.join("defs").to_string_lossy()).unwrap().project_id;
    let team = reg.team_view(id,0).unwrap().unwrap();
    let field = team.all_fields.iter().find(|f| f.name == "nickname").unwrap();
    reg.set_field(id,&field.studio_table,team.record,&field.studio_field,FieldValue::Text("QA Ducks".into())).unwrap();
    let review = reg.copied_target_review_archive_member(id,&root.to_string_lossy(),"inxp_00.big","nhl/data/common/gamemodes/database/t3db/nhl_game.db").unwrap();
    let applied = reg.copied_target_apply(id,review.handle).unwrap();
    assert_eq!(applied.state,"committed");
    let after = std::fs::read(&archive).unwrap();
    let bound = tdb_archive_big::stage::BoundSource::bind(&mut Cursor::new(&after)).unwrap();
    let entry = bound.entries.iter().find(|e| e.path == "nhl/data/common/gamemodes/database/t3db/nhl_game.db").unwrap();
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
    assert_eq!(std::fs::read(source.join("inxp_00.big")).unwrap(),before_archive);
    std::fs::write(root.join("qa-receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "transaction":applied.txn_id,"archive_decoded_matches_edited_db":true,"crc_ok":true,
        "archive_and_db_restored":true,"source_preserved":true,"game_verified":false
    })).unwrap()).unwrap();
}

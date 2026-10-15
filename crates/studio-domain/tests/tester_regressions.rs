use std::path::{Path, PathBuf};
use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn session(game: &str, platform: Platform, file: &str) -> EditSession {
    let folder = if platform == Platform::Xbox { "xbox360" } else { "ps3" };
    let mut p = Project::load(repo(&format!("anchors/{folder}/{game}/databases/{file}.db")), platform,
        ProjectRole::Nhlng, repo(&format!("anchors/{folder}/{game}/databases/{file}-meta.xml")), repo("defs")).unwrap();
    p.set_game_id(Some(game.into()));
    EditSession::begin(p)
}

#[test]
fn early_uniforms_follow_stock_art_instead_of_exhibition_index() {
    let s = session("nhl07", Platform::Xbox, "NHL07NG");
    let map = s.jersey_art_ids().unwrap();
    let mut checked = 0;
    for team in s.team_summaries().unwrap() {
        if ["Austria", "Canada", "Germany"].iter().any(|n| team.name.contains(n)) {
            let art = s.team_view(team.team_id).unwrap().unwrap().logo_artid.unwrap();
            assert_eq!(map[&team.team_id], vec![art]);
            assert_ne!(art, team.team_id, "regression requires a nonidentity mapping");
            checked += 1;
        }
    }
    assert_eq!(checked, 3);
}

#[test]
fn roster_key_replay_allows_permutations_only() {
    let mut s = session("nhl08", Platform::Ps3, "nhlng");
    let bytes = s.bytes().to_vec();
    let db = tdb_core::TdbFile::parse(&bytes).unwrap();
    let t = db.table_canonical("eGlu").unwrap();
    let key = t.fields.iter().find(|f| f.id.canonical(db.endian) == "toWX").unwrap();
    let a = db.read_u64(t, 0, key).unwrap() as i64;
    let b = db.read_u64(t, 1, key).unwrap() as i64;
    let cell = |row, value| ("eGlu".into(), row, "toWX".into(), FieldValue::Int(value));
    assert!(s.set_field("eGlu", 0, "toWX", FieldValue::Int(b)).is_err());
    assert!(s.set_package_fields(&[cell(0, b)]).is_err());
    assert_eq!(s.bytes(), bytes);
    s.set_package_fields(&[cell(0, b), cell(1, a)]).unwrap();
    let result = tdb_core::TdbFile::parse(s.bytes()).unwrap();
    assert_eq!(result.read_u64(result.table_canonical("eGlu").unwrap(), 0, key), Some(b as u64));
}

#[test]
#[ignore = "requires NHLMS_NHL07_RENDERING pointing at extracted game assets"]
fn native_early_goalie_has_two_decodable_pads_and_requested_mask() {
    use studio_domain::kit_preview::*;
    let root = PathBuf::from(std::env::var("NHLMS_NHL07_RENDERING").unwrap());
    let defaults = goalie_kit_mesh_defaults(&root);
    let mut pieces = vec![KitPieceInput { kind:"pads".into(), mesh_path:defaults.pad.unwrap(),
        texture_paths:vec![], prebuilt_textures:vec![], mirror_x:false, raise_frac:0.0 }];
    pair_early_equipment(&mut pieces);
    let paint = goalie_mask_paint(&root, Some("nhl07"), 0, 7);
    assert_eq!(paint.len(), 1);
    pieces.push(KitPieceInput { kind:"mask".into(), mesh_path:defaults.mask.unwrap(),
        texture_paths:paint, prebuilt_textures:vec![], mirror_x:false, raise_frac:0.0 });
    let result = decode_kit_preview(&pieces).unwrap();
    assert_eq!(result.pieces.iter().filter(|p|p.kind == "pads").count(), 2);
    assert!(!result.textures.is_empty());
    let xs:Vec<_> = result.pieces.iter().filter(|p|p.kind == "pads")
        .flat_map(|p|p.submeshes.iter().flat_map(|s|s.positions.iter().map(|v|v[0]))).collect();
    assert!(xs.iter().any(|x| *x < -0.02) && xs.iter().any(|x| *x > 0.02));
}

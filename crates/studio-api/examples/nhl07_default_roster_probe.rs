//! NHL07 native default-database experiment, kept separate from the unqualified transfer UI.
//! Usage: nhl07_default_roster_probe SOURCE_PROJECT OUTPUT_DIR [NEW_PROJECT INSTALL_ROOT]
//! Two arguments only generate and verify an artifact; four additionally import/deploy it.
use std::path::PathBuf;
use studio_api::{Registry, dto::ImportSourcesDto};
use studio_domain::{Platform, tdb_txn::FileIdentity};
use tdb_core::TdbFile;

fn canonical(raw: &str) -> String { raw.chars().rev().collect() }
fn field<'a>(db: &TdbFile<'a>, table: &tdb_core::Table, raw: &str) -> tdb_core::FieldDescriptor {
    table.fields.iter().find(|f| f.id.canonical(db.endian) == canonical(raw)).unwrap().clone()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 && args.len() != 5 { return Err("usage: SOURCE_PROJECT OUTPUT_DIR [NEW_PROJECT INSTALL_ROOT]".into()); }
    let source = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(source.join("nhl-studio-project.json"))?)?;
    assert_eq!(manifest["game_id"], "nhl07");
    assert_eq!(manifest["platform"], "xbox");
    let input = source.join(manifest["working"]["main_db"].as_str().ok_or("no main database")?);
    let meta = source.join(manifest["working"]["meta"].as_str().ok_or("no metadata")?);
    let bytes = std::fs::read(&input)?;
    let db = TdbFile::parse(&bytes)?;
    let defs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../defs");
    let reg = Registry::default();
    let preview = reg.load_project(&input.to_string_lossy(), Platform::Xbox, studio_domain::ProjectRole::Nhlng, &meta.to_string_lossy(), &defs.to_string_lossy())?;
    let players = reg.list_players(preview.project_id)?;
    let demitra = players.iter().find(|p| p.player_id == 613 && p.first_name == "Pavol" && p.last_name == "Demitra").ok_or("Demitra identity mismatch")?;
    assert_eq!(demitra.team_id, Some(14));
    let teams = reg.list_teams(preview.project_id)?;
    assert!(teams.iter().any(|t| t.team_id == 28 && t.name.contains("Vancouver")));
    reg.close_project(preview.project_id);
    // Resolve the domestic slot by the actual schema keys, never by record position.
    let slots = db.table_canonical(&canonical("caBZ")).unwrap();
    let pfield = field(&db, slots, "qFky");
    let index = field(&db, slots, "qEfv");
    let candidate_slots: Vec<u64> = (0..slots.current_records as u32).filter(|&r| db.read_u64(slots,r,&pfield)==Some(613)).map(|r| db.read_u64(slots,r,&index).unwrap()).collect();
    let mut replacement = bytes.clone();
    let mut changes = Vec::new();
    // Both the default/stock and exhibition copies are changed identically for this first
    // native baseline-loading test. This does not prove which copy Restore Defaults reads.
    for (raw_table, key_name, bio) in [("cPbu","zIBw",true),("aUhv","zIBw",true),("ulGe","TWSX",false),("Smgy","TWSX",false)] {
        let table = db.table_canonical(&canonical(raw_table)).unwrap();
        let key = field(&db,table,key_name);
        let team = field(&db,table,"BSXd");
        let rows: Vec<_> = (0..table.current_records as u32).filter(|&r| {
            let id = db.read_u64(table,r,&key).unwrap();
            (if bio { id==613 } else { candidate_slots.contains(&id) }) && db.read_u64(table,r,&team)==Some(14)
        }).collect();
        assert_eq!(rows.len(),1,"ambiguous domestic record in {raw_table}");
        let row = rows[0];
        for (raw_field,value) in [("BSXd",28),("tRVs",90)] {
            let f = field(&db,table,raw_field);
            let before = db.read_u64(table,row,&f).ok_or("unreadable field")?;
            assert!(db.set_u64(&mut replacement,table,row,&f,value));
            changes.push(serde_json::json!({"table":raw_table,"row":row,"field":raw_field,"before":before,"after":value}));
        }
    }
    db.recompute_crcs(&mut replacement);
    let checked = TdbFile::parse(&replacement)?;
    assert!(checked.header_crc_ok());
    assert_eq!(checked.serialize(),replacement,"CRC/serialization mismatch");
    // Verify the complete schema and all cell values: exactly the eight requested edits.
    let mut changed_cells = 0;
    for t in &db.tables {
        let after = checked.table_canonical(&t.id.canonical(db.endian)).unwrap();
        assert_eq!(t.current_records,after.current_records);
        for r in 0..t.current_records as u32 {
            for f in &t.fields {
                if f.bit_width <= 64 && db.read_u64(t,r,f) != checked.read_u64(after,r,f) { changed_cells += 1; }
            }
        }
    }
    assert_eq!(changed_cells,8);
    std::fs::create_dir_all(&out)?;
    let artifact = out.join("NHL07NG.db");
    if artifact.exists() { assert_eq!(std::fs::read(&artifact)?,replacement,"existing artifact differs"); }
    else { use std::io::Write; let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&artifact)?; f.write_all(&replacement)?; f.sync_all()?; }
    let output_meta = out.join("NHL07NG-meta.xml");
    if !output_meta.exists() { std::fs::copy(&meta,&output_meta)?; }
    let receipt=serde_json::json!({"source":input,"source_identity":FileIdentity::of_bytes(&bytes),"artifact":artifact,"output_identity":FileIdentity::of_bytes(&replacement),"changes":changes,"native_verified":false,"limitations":"Controlled imported database; Studio transfer UI still assumes fixed roster blocks. Both stock/default and exhibition copies changed; active runtime copy not yet isolated."});
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&receipt)?)?;
    println!("{}",serde_json::to_string_pretty(&receipt)?);
    if args.len()==5 {
        let sources=ImportSourcesDto { main_db:Some(artifact.to_string_lossy().into_owned()),meta:Some(output_meta.to_string_lossy().into_owned()),dictionary:Some(source.join(manifest["working"]["dictionary"].as_str().ok_or("no dictionary")?).to_string_lossy().into_owned()),..Default::default() };
        let opened=if PathBuf::from(&args[3]).join("nhl-studio-project.json").exists() { reg.project_open(&args[3],&defs.to_string_lossy())? } else { reg.project_create_import(&args[3],"nhl07",Platform::Xbox,&sources,&args[4],None,None,None,&defs.to_string_lossy())? };
        let id=opened.summary.project_id;
        let demitra=reg.player_view(id,613)?.ok_or("missing imported player")?;
        assert!(demitra.memberships.iter().any(|m|m.team_id==28 && m.jersey==Some(90)));
        assert!(!demitra.memberships.iter().any(|m|m.team_id==14));
        assert_eq!(std::fs::read(opened.project.working_main_db.as_ref().unwrap())?,replacement);
        // Imported edited inputs differ from the installed vanilla file. Permit replacement
        // only after independently matching that destination to the untouched source bytes.
        let installed=PathBuf::from(opened.project.destinations.main_db_path.as_ref().ok_or("missing destination")?);
        assert_eq!(std::fs::read(&installed)?,bytes,"installation changed since the source was read");
        let plan=reg.project_deploy_plan(id,true)?;
        println!("DEPLOY_PLAN {}",serde_json::to_string(&plan)?);
        if !plan.deployable { return Err("deployment blocked, imported project retained".into()); }
        println!("DEPLOY {}",serde_json::to_string(&reg.project_deploy_apply(id,plan.handle)?)?);
        reg.close_project(id);
        let opened=reg.project_open(&args[3],&defs.to_string_lossy())?;
        println!("REOPEN {}",serde_json::to_string(&reg.player_view(opened.summary.project_id,613)?.unwrap().memberships)?);
        reg.close_project(opened.summary.project_id);
    }
    Ok(())
}



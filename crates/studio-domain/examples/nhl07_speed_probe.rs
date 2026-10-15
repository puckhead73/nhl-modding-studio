//! Controlled, separate-file NHL07 attribute probe. Inputs are never modified.
use std::io::Write;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 {
        return Err("usage: nhl07_speed_probe <game.db> <save> <meta.xml> <game_id> <raw-speed> <new-output-file>".into());
    }
    let base = std::fs::read(&args[1])?;
    let source = std::fs::read(&args[2])?;
    let meta = tdb_defs::FieldMetaIndex::from_xml(&std::fs::read_to_string(&args[3])?)?;
    let player: u64 = args[4].parse()?;
    let value: u64 = args[5].parse()?;
    let inner = tdb_savedata::SaveContainer::parse(&source)?.extract_db()?;
    let effective = studio_domain::roster_difflist::effective_roster(&base, &inner, &meta)?;
    let db = tdb_core::TdbFile::parse(&base)?;
    let field = db
        .table("yvSd")
        .ok_or("missing AI table")?
        .field("YWPq")
        .ok_or("missing speed field")?;
    let row = effective
        .get("yvSd")
        .and_then(|t| t.get(&player))
        .ok_or("unknown stock skater")?;
    let before = tdb_core::read_bits(row, field.bit_offset, field.bit_width, db.endian);
    let output = studio_domain::roster_difflist::experimental_nhl07_speed_raw(
        &base, &source, &meta, player, value,
    )?;
    if !tdb_savedata::SaveContainer::parse(&output)?.checksums_valid() {
        return Err("generated checksum failed".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[6])?;
    file.write_all(&output)?;
    file.sync_all()?;
    println!("Created {}: game_id {player}, raw speed {before} -> {value}, {} bytes. Native attribute acceptance remains unverified.", args[6], output.len());
    Ok(())
}

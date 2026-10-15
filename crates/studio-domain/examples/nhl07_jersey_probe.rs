//! Produce a separate experimental save for native verification; never overwrite an input.
use std::io::Write;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 {
        return Err("usage: nhl07_jersey_probe <game.db> <save> <meta.xml> <playerindex> <jersey> <new-output-file>".into());
    }
    let base = std::fs::read(&args[1])?;
    let source = std::fs::read(&args[2])?;
    let meta = tdb_defs::FieldMetaIndex::from_xml(&std::fs::read_to_string(&args[3])?)?;
    let output = studio_domain::roster_difflist::experimental_nhl07_jersey(
        &base,
        &source,
        &meta,
        args[4].parse()?,
        args[5].parse()?,
    )?;
    let parsed = tdb_savedata::SaveContainer::parse(&output)?;
    if !parsed.checksums_valid() {
        return Err("generated save checksum failed".into());
    }
    // create_new refuses an existing destination, including the source itself.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[6])?;
    file.write_all(&output)?;
    file.sync_all()?;
    println!(
        "Created {} ({} bytes). Native loading is not yet verified.",
        args[6],
        output.len()
    );
    Ok(())
}

//! Read-only source decode for format qualification; never overwrites output.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{io::Write, path::Path};
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {return Err("usage: native_asset_decode source new-output".into());}
    let mut bytes=std::fs::read(Path::new(&args[1]))?;
    bytes=tdb_archive_codec::decompress_if_wrapped(&bytes)?;
    if tdb_archive_codec::is_refpack(&bytes) {
        bytes=tdb_archive_codec::refpack_decompress(&bytes)?;
    }
    let mut out=std::fs::OpenOptions::new().write(true).create_new(true).open(&args[2])?;
    out.write_all(&bytes)?;
    Ok(())
}

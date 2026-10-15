//! Validate a proposed append-and-retarget through the production EB parser.
use std::{
    error::Error,
    io::{Read, Seek, SeekFrom},
};
use tdb_archive_big::eb::{decode_payload, EbArchive, EbHeader, Payload};
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 6 {
        return Err("usage: eb_verify_append archive index new_offset new_archive_size stored_file decoded_file".into());
    }
    let index: usize = a[1].parse()?;
    let offset: u64 = a[2].parse()?;
    let size: u64 = a[3].parse()?;
    let mut f = std::fs::File::open(&a[0])?;
    let mut h = [0u8; 32];
    f.read_exact(&mut h)?;
    let header = EbHeader::parse(&h)?;
    let mut prefix = vec![0; header.table_end()];
    f.seek(SeekFrom::Start(0))?;
    f.read_exact(&mut prefix)?;
    let before = EbArchive::parse(&prefix)?;
    let payload = std::fs::read(&a[4])?;
    assert!(
        offset >= f.metadata()?.len()
            && offset.is_multiple_of(16)
            && offset + payload.len() as u64 <= size
    );
    prefix[24..32].copy_from_slice(&size.to_be_bytes());
    let row = 48 + index * 20;
    prefix[row..row + 4].copy_from_slice(&u32::try_from(offset / 16)?.to_be_bytes());
    prefix[row + 8..row + 12].copy_from_slice(&u32::try_from(payload.len())?.to_be_bytes());
    let after = EbArchive::parse(&prefix)?;
    for (i, (a, b)) in before.entries.iter().zip(&after.entries).enumerate() {
        if i != index {
            assert_eq!(a, b);
        } else {
            assert_eq!(a.path, b.path);
            assert_eq!(b.offset, offset);
            assert_eq!(b.size as usize, payload.len());
        }
    }
    match decode_payload(payload)? {
        Payload::Data(d) => assert_eq!(d, std::fs::read(&a[5])?),
        _ => return Err("unsupported codec".into()),
    }
    println!(
        "PASS index {index}, append {offset}, all {} unrelated entries unchanged",
        before.entries.len() - 1
    );
    Ok(())
}

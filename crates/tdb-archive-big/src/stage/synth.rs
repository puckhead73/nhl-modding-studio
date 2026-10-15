//! Synthetic archives in the qualified 16-byte-row layout, for tests and
//! copied-target demonstrations in other crates. Not a production writer.

use super::{path_hash, QUALIFIED_VERSION};
use crate::eb::{entry_size_for, FILE_TABLE_OFFSET, MAGIC_EB};

/// Build an `EB\0\3` archive with version `0x0000_0400`, 16-byte rows
/// (`word_4 = 0`, `+12 = path_hash(path)`), payloads in table order each
/// zero-padded to the next 16-byte line, one zero line between the file
/// table and the name table, and `archive_size` = padded end = length.
/// Header words `0x10` and `0x20..0x30` are zero.
///
/// `files` are `(dir_index, basename, stored bytes)`.
pub fn build_archive(dirs: &[&str], files: &[(u16, &str, Vec<u8>)]) -> Vec<u8> {
    let row_len = entry_size_for(QUALIFIED_VERSION);
    let file_count = files.len();
    let name_slot = files.iter().map(|(_, n, _)| n.len() + 3).max().unwrap_or(3);
    let dir_slot = dirs.iter().map(|d| d.len() + 1).max().unwrap_or(1);
    let name_table_offset = FILE_TABLE_OFFSET + file_count * row_len + 16;
    let name_end = name_table_offset + file_count * name_slot;
    let dir_start = (name_end + 15) & !15;
    let table_end = dir_start + dirs.len() * dir_slot;
    let data_start = (table_end + 15) & !15;

    let mut offsets = Vec::with_capacity(file_count);
    let mut cursor = data_start as u64;
    for (_, _, body) in files {
        offsets.push(cursor);
        cursor = (cursor + body.len() as u64 + 15) & !15;
    }
    let archive_size = cursor;

    let mut out = vec![0u8; archive_size as usize];
    out[0..4].copy_from_slice(MAGIC_EB);
    out[0x04..0x08].copy_from_slice(&(file_count as u32).to_be_bytes());
    out[0x08..0x0C].copy_from_slice(&QUALIFIED_VERSION.to_be_bytes());
    out[0x0C..0x10].copy_from_slice(&(name_table_offset as u32).to_be_bytes());
    out[0x14] = name_slot as u8;
    out[0x15] = dir_slot as u8;
    out[0x16..0x18].copy_from_slice(&(dirs.len() as u16).to_be_bytes());
    out[0x18..0x20].copy_from_slice(&archive_size.to_be_bytes());
    for (i, (dir_index, name, body)) in files.iter().enumerate() {
        let e = FILE_TABLE_OFFSET + i * row_len;
        out[e..e + 4].copy_from_slice(&((offsets[i] >> 4) as u32).to_be_bytes());
        out[e + 8..e + 12].copy_from_slice(&(body.len() as u32).to_be_bytes());
        let path = format!("{}/{}", dirs[*dir_index as usize], name);
        out[e + 12..e + 16].copy_from_slice(&path_hash(&path).to_be_bytes());
        let n = name_table_offset + i * name_slot;
        out[n..n + 2].copy_from_slice(&dir_index.to_be_bytes());
        out[n + 2..n + 2 + name.len()].copy_from_slice(name.as_bytes());
        let o = offsets[i] as usize;
        out[o..o + body.len()].copy_from_slice(body);
    }
    for (i, dir) in dirs.iter().enumerate() {
        let d = dir_start + i * dir_slot;
        out[d..d + dir.len()].copy_from_slice(dir.as_bytes());
    }
    out
}

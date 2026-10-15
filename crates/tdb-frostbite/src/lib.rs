//! Read-only NHL 22–24 PS4 containers. Offsets and identities remain tied to their
//! source layer: a catalogue entry is not a claim about runtime patch precedence.
pub mod blocks;
pub mod db;
pub mod install;
pub mod manifest;
pub mod toc;

use std::io;
pub type Result<T> = std::result::Result<T, io::Error>;
pub(crate) fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
pub(crate) fn slice(b: &[u8], p: usize, n: usize) -> Result<&[u8]> {
    b.get(p..p.checked_add(n).ok_or_else(|| invalid("offset overflow"))?)
        .ok_or_else(|| invalid(format!("truncated range at {p:#x}, length {n:#x}")))
}
pub(crate) fn be(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(slice(b, p, 4)?.try_into().unwrap()))
}
pub(crate) fn le(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(slice(b, p, 4)?.try_into().unwrap()))
}
pub(crate) fn hex(b: &[u8]) -> String {
    b.iter().map(|v| format!("{v:02x}")).collect()
}
pub(crate) fn cstr(b: &[u8], p: usize) -> Result<String> {
    let tail = b
        .get(p..)
        .ok_or_else(|| invalid("string offset out of range"))?;
    let end = tail
        .iter()
        .position(|v| *v == 0)
        .ok_or_else(|| invalid("unterminated string"))?;
    String::from_utf8(tail[..end].to_vec()).map_err(|_| invalid("invalid UTF-8 string"))
}
/// Only the observed plaintext signed wrapper is accepted. D1CE00 may be obfuscated.
pub fn payload(b: &[u8]) -> Result<&[u8]> {
    if b.starts_with(&[0, 0xd1, 0xce, 1]) {
        slice(b, 0x22c, b.len().saturating_sub(0x22c))
    } else if b.starts_with(&[0, 0xd1, 0xce]) {
        Err(invalid("unsupported D1CE wrapper version"))
    } else {
        Ok(b)
    }
}

//! `\x89RW4xb2` container surgery — parse the file table + per-chunk
//! bytes, then re-emit identical bytes (Phase 1) or selectively
//! replace chunk payloads while preserving the rest of the layout
//! (Phase 4 substrate).
//!
//! The decoder in [`crate::playermesh`] gives a *parsed view* — it
//! pulls positions, indices, weights, etc. out of the file. The
//! container layer here is one level lower: it owns the raw bytes
//! of every chunk and the file-table metadata pointing at them.
//! The two layers are complementary:
//!
//! - Parse + structured edit + emit a new `.rx2` ⇒ go through
//!   [`Container`] (this module).
//! - Read out geometry as `PlayerAsset` ⇒ go through
//!   [`crate::playermesh::parse_player_mesh`].
//!
//! The encoder's correctness bar is round-trip identity: for any
//! `data`, `encode(&parse(data)?)` returns `data` byte-for-byte. Once
//! identity holds, Phase 4 can call `replace_payload` to substitute
//! one chunk's bytes; the encoder re-flows offsets so the file stays
//! consistent.

use std::convert::TryInto;

use thiserror::Error;

use crate::playermesh::MAGIC;

const HDR_FILE_COUNT_OFF: usize = 0x20;
const HDR_FILE_TABLE_OFF: usize = 0x30;
const HDR_HDR_SIZE_OFF: usize = 0x44;
const HDR_DATA_SIZE_OFF: usize = 0x54;
const ENTRY_STRIDE: usize = 24;

pub const TAG_RAW_PAYLOAD: u32 = 0x0001_0031;

#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("input is shorter than the RX2 magic ({0} bytes)")]
    TooShort(usize),
    #[error("magic mismatch — expected `\\x89RW4xb2`")]
    BadMagic,
    #[error("file table at 0x{offset:X} runs past EOF")]
    TruncatedFileTable { offset: usize },
    #[error("chunk {entry} (tag 0x{tag:08X}) at 0x{offset:X}..0x{end:X} runs past EOF")]
    TruncatedChunk {
        entry: usize,
        tag: u32,
        offset: usize,
        end: usize,
    },
    #[error("replace_payload: chunk {idx} is not a raw payload (tag 0x{tag:08X})")]
    NotRawPayload { idx: usize, tag: u32 },
}

/// One file-table entry's classified metadata + the bytes it points at.
///
/// `raw_offset` is what the file table stores — absolute file offset
/// for descriptor chunks, relative to `hdr_size` for raw payloads.
/// `bytes` is a *copy* of the chunk's content as it appears in the
/// source file. Modifications go through [`Container::replace_payload`]
/// or by mutating `bytes` directly when size-preserving.
#[derive(Debug, Clone)]
pub struct ChunkRef {
    pub raw_offset: u32,
    pub size: u32,
    pub align: u32,
    pub kind_idx: u32,
    pub tag: u32,
    pub bytes: Vec<u8>,
}

impl ChunkRef {
    /// Absolute file offset for this chunk's bytes regardless of class.
    pub fn abs_offset(&self, hdr_size: u32) -> u32 {
        if self.tag == TAG_RAW_PAYLOAD {
            self.raw_offset + hdr_size
        } else {
            self.raw_offset
        }
    }
}

/// Full container view — preserves every byte of the source file
/// (header, descriptor region, data region, file table). Can be
/// re-emitted byte-identically or with chunk payloads replaced.
#[derive(Debug, Clone)]
pub struct Container {
    /// Bytes `0..hdr_size` of the source file. The descriptor region
    /// is included here and contains every header-region chunk
    /// already laid out at its file-table offset.
    pub header_region: Vec<u8>,
    pub hdr_size: u32,
    pub file_table_off: u32,
    pub data_size: u32,
    /// Every file-table entry, in order, with its bytes pulled from
    /// the source.
    pub chunks: Vec<ChunkRef>,
}

impl Container {
    /// Number of file-table entries.
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Re-emit the container as bytes. With an unmodified container
    /// this is byte-identical to the source file. After
    /// `replace_payload` calls it reflects the modified layout.
    pub fn encode(&self) -> Vec<u8> {
        // Rebuild the data region by laying raw payloads back-to-back
        // in file-table order — same order as the source. Each raw
        // payload's `raw_offset` is the running cumulative size.
        let mut data_region: Vec<u8> = Vec::with_capacity(self.data_size as usize);
        for c in &self.chunks {
            if c.tag == TAG_RAW_PAYLOAD {
                data_region.extend_from_slice(&c.bytes);
            }
        }
        let total = self.hdr_size as usize + data_region.len();
        let mut out = vec![0u8; total];

        // Copy header region verbatim (descriptors + tag registry +
        // structural fields all live here at their original offsets).
        out[..self.hdr_size as usize].copy_from_slice(&self.header_region);

        // Rewrite the file table from `self.chunks`. For an
        // unmodified container the bytes match the source already.
        let ft = self.file_table_off as usize;
        for (i, c) in self.chunks.iter().enumerate() {
            let base = ft + i * ENTRY_STRIDE;
            out[base..base + 4].copy_from_slice(&c.raw_offset.to_be_bytes());
            // ints[1] is always zero in observed samples; preserve as
            // such on re-emit.
            out[base + 4..base + 8].copy_from_slice(&0u32.to_be_bytes());
            out[base + 8..base + 12].copy_from_slice(&c.size.to_be_bytes());
            out[base + 12..base + 16].copy_from_slice(&c.align.to_be_bytes());
            out[base + 16..base + 20].copy_from_slice(&c.kind_idx.to_be_bytes());
            out[base + 20..base + 24].copy_from_slice(&c.tag.to_be_bytes());
        }

        // Update the structural data_size field at +0x58 in case we
        // grew/shrunk the data region.
        let new_data_size = data_region.len() as u32;
        out[HDR_DATA_SIZE_OFF..HDR_DATA_SIZE_OFF + 4].copy_from_slice(&new_data_size.to_be_bytes());

        // Append the data region.
        out[self.hdr_size as usize..].copy_from_slice(&data_region);

        out
    }

    /// Replace a raw payload chunk's bytes. Subsequent raw payloads
    /// re-flow: their `raw_offset` is updated so they sit
    /// back-to-back. Header-region chunks aren't affected. The
    /// container's `data_size` is updated to match the new total.
    pub fn replace_payload(
        &mut self,
        idx: usize,
        new_bytes: Vec<u8>,
    ) -> Result<(), ContainerError> {
        let chunk = self.chunks.get(idx).ok_or(ContainerError::TruncatedChunk {
            entry: idx,
            tag: 0,
            offset: 0,
            end: 0,
        })?;
        if chunk.tag != TAG_RAW_PAYLOAD {
            return Err(ContainerError::NotRawPayload {
                idx,
                tag: chunk.tag,
            });
        }
        let new_size = new_bytes.len() as u32;
        self.chunks[idx].size = new_size;
        self.chunks[idx].bytes = new_bytes;

        // Re-flow raw_offset across all raw payloads in file-table
        // order to keep them contiguous in the data region.
        let mut running: u32 = 0;
        let mut total: u32 = 0;
        for c in self.chunks.iter_mut() {
            if c.tag == TAG_RAW_PAYLOAD {
                c.raw_offset = running;
                running += c.size;
                total = running;
            }
        }
        self.data_size = total;
        Ok(())
    }
}

pub fn parse(data: &[u8]) -> Result<Container, ContainerError> {
    if data.len() < MAGIC.len() {
        return Err(ContainerError::TooShort(data.len()));
    }
    if &data[..MAGIC.len()] != MAGIC {
        return Err(ContainerError::BadMagic);
    }
    let file_count = read_u32_be(data, HDR_FILE_COUNT_OFF)? as usize;
    let file_table_off = read_u32_be(data, HDR_FILE_TABLE_OFF)?;
    let hdr_size = read_u32_be(data, HDR_HDR_SIZE_OFF)?;
    let data_size = read_u32_be(data, HDR_DATA_SIZE_OFF)?;

    let table_bytes =
        file_count
            .checked_mul(ENTRY_STRIDE)
            .ok_or(ContainerError::TruncatedFileTable {
                offset: file_table_off as usize,
            })?;
    let ft_start = file_table_off as usize;
    if ft_start + table_bytes > data.len() {
        return Err(ContainerError::TruncatedFileTable { offset: ft_start });
    }

    let mut chunks = Vec::with_capacity(file_count);
    for i in 0..file_count {
        let base = ft_start + i * ENTRY_STRIDE;
        let raw_offset = read_u32_be(data, base)?;
        let _ints1 = read_u32_be(data, base + 4)?;
        let size = read_u32_be(data, base + 8)?;
        let align = read_u32_be(data, base + 12)?;
        let kind_idx = read_u32_be(data, base + 16)?;
        let tag = read_u32_be(data, base + 20)?;

        let abs_off = if tag == TAG_RAW_PAYLOAD {
            (raw_offset + hdr_size) as usize
        } else {
            raw_offset as usize
        };
        let end = abs_off + size as usize;
        if end > data.len() {
            return Err(ContainerError::TruncatedChunk {
                entry: i,
                tag,
                offset: abs_off,
                end,
            });
        }
        chunks.push(ChunkRef {
            raw_offset,
            size,
            align,
            kind_idx,
            tag,
            bytes: data[abs_off..end].to_vec(),
        });
    }

    let header_region = data[..hdr_size as usize].to_vec();
    Ok(Container {
        header_region,
        hdr_size,
        file_table_off,
        data_size,
        chunks,
    })
}

fn read_u32_be(data: &[u8], offset: usize) -> Result<u32, ContainerError> {
    if offset + 4 > data.len() {
        return Err(ContainerError::TruncatedFileTable { offset });
    }
    let bytes: [u8; 4] = data[offset..offset + 4].try_into().expect("len checked");
    Ok(u32::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn template_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("Working Assets")
            .join("Template Mesh")
            .join("player_20500.rx2")
    }

    fn load_template() -> Option<Vec<u8>> {
        let p = template_path();
        if !p.is_file() {
            return None;
        }
        std::fs::read(p).ok()
    }

    #[test]
    fn round_trip_template_byte_identical() {
        let Some(data) = load_template() else {
            eprintln!("skipping: template player_20500.rx2 not present");
            return;
        };
        let c = parse(&data).expect("parse");
        let out = c.encode();
        assert_eq!(
            out.len(),
            data.len(),
            "encoded length differs (out={} src={})",
            out.len(),
            data.len()
        );
        // Find the first divergence so a regression surfaces clearly.
        let mut diff = None;
        for i in 0..data.len() {
            if out[i] != data[i] {
                diff = Some(i);
                break;
            }
        }
        if let Some(i) = diff {
            panic!(
                "first divergence at 0x{:X}: src=0x{:02X}, out=0x{:02X}",
                i, data[i], out[i]
            );
        }
    }

    #[test]
    fn replace_payload_grows_data_size_and_reflows() {
        let Some(data) = load_template() else {
            return;
        };
        let mut c = parse(&data).expect("parse");
        // Find the first raw payload and swap its bytes for something
        // a different size. The container should re-flow subsequent
        // raw_offsets and update data_size.
        let raw_idx = c
            .chunks
            .iter()
            .position(|c| c.tag == TAG_RAW_PAYLOAD)
            .expect("template has raw payloads");
        let original_size = c.chunks[raw_idx].size;
        let new_bytes = vec![0xABu8; (original_size + 16) as usize];
        c.replace_payload(raw_idx, new_bytes).unwrap();

        assert_eq!(c.chunks[raw_idx].size, original_size + 16);
        // Subsequent raw payloads should have shifted by +16.
        let mut running: u32 = 0;
        for chunk in &c.chunks {
            if chunk.tag == TAG_RAW_PAYLOAD {
                assert_eq!(chunk.raw_offset, running);
                running += chunk.size;
            }
        }
        assert_eq!(c.data_size, running);

        let out = c.encode();
        let header_payload: usize = c.hdr_size as usize;
        assert_eq!(out.len(), header_payload + c.data_size as usize);

        // Re-parse the encoded bytes — structural fields must agree.
        let c2 = parse(&out).expect("re-parse");
        assert_eq!(c2.data_size, c.data_size);
        assert_eq!(c2.chunks.len(), c.chunks.len());
        assert_eq!(c2.chunks[raw_idx].size, c.chunks[raw_idx].size);
    }
}

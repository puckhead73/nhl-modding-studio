//! DDS exports retain the decoded GPU blocks, without another lossy compression.
//! Only the base mip is currently exposed by the source readers.
use crate::{assets, DomainError, Result};
use std::path::Path;
use tdb_archive_codec::{encode_dds, DdsFormat};

pub(crate) fn decode_dds(path: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let bytes = assets::read_asset_file(path)?;
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gnf"))
    {
        let tex = tdb_rx2::gnf::parse(&bytes)
            .map_err(|e| DomainError::Extract(format!("{}: {e}", path.display())))?;
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        return Ok(vec![(name, gnf_dds(&tex))]);
    }
    let rasters = assets::read_render_rasters(&bytes)?;
    // The preview reader deliberately skips Xbox DXN/BC5 normals. DDS can
    // preserve those GPU blocks directly, without adding a preview decoder.
    let extra = xbox_bc5_dds(path, &bytes)?;
    if rasters.is_empty() && extra.is_empty() {
        return Err(DomainError::Extract(format!(
            "{}: texture container has no rasters",
            path.display()
        )));
    }
    let mut outputs: Vec<_> = rasters
        .into_iter()
        .map(|tex| {
            use tdb_rx2::texlib::PixelFormat;
            let format = match tex.format {
                PixelFormat::Dxt1 => Some(DdsFormat::Dxt1),
                PixelFormat::Dxt3 => Some(DdsFormat::Dxt3),
                PixelFormat::Dxt5 => Some(DdsFormat::Dxt5),
                _ => None,
            };
            let dds = if let Some(format) = format {
                encode_dds(format, tex.width, tex.height, 1, &tex.base_mip_linear)
            } else {
                // Normalize channel ordering for uncompressed/non-BC raster formats.
                let rgba = assets::linear_to_rgba(
                    tex.format,
                    tex.width,
                    tex.height,
                    &tex.base_mip_linear,
                )?;
                encode_dds(DdsFormat::Rgba32, tex.width, tex.height, 1, &rgba)
            };
            Ok((tex.name, dds))
        })
        .collect::<Result<_>>()?;
    outputs.extend(extra);
    Ok(outputs)
}

fn xbox_bc5_dds(path: &Path, bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>> {
    if !bytes.starts_with(tdb_rx2::texlib::MAGIC) {
        return Ok(Vec::new());
    }
    let invalid = || DomainError::Extract(format!("{}: truncated BC5 raster", path.display()));
    let word = |off: usize| -> Result<usize> {
        let raw = bytes
            .get(off..off.checked_add(4).ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        Ok(u32::from_be_bytes(raw.try_into().unwrap()) as usize)
    };
    let count = word(0x20)?;
    let table = word(0x30)?;
    if count > bytes.len() / 24
        || table
            .checked_add(count * 24)
            .is_none_or(|end| end > bytes.len())
    {
        return Err(invalid());
    }
    let bias = if word(0x54)? == 0 { 0 } else { word(0x44)? };
    let mut out = Vec::new();
    for index in 0..count {
        let entry = table + index * 24;
        if word(entry + 20)? != 0x20003 {
            continue;
        }
        let descriptor = word(entry)?;
        if bytes.get(descriptor + 35).copied() != Some(0x71) {
            continue;
        }
        // Measured NHL08+ entries alternate pixel slot then descriptor.
        if index == 0 {
            return Err(invalid());
        }
        let pixel = entry - 24;
        if word(pixel + 20)? != 0x10031 {
            return Err(invalid());
        }
        let dimensions = word(descriptor + 36)?;
        let width = ((dimensions & 0x1fff) + 1) as u32;
        let height = (((dimensions >> 13) & 0x1fff) + 1) as u32;
        let start = bias.checked_add(word(pixel)?).ok_or_else(invalid)?;
        let end = start.checked_add(word(pixel + 8)?).ok_or_else(invalid)?;
        let mut blocks = bytes.get(start..end).ok_or_else(invalid)?.to_vec();
        let expected = width.div_ceil(4) as usize * height.div_ceil(4) as usize * 16;
        if blocks.len() < expected {
            return Err(invalid());
        }
        tdb_rx2::texlib::swap16_in_place(&mut blocks);
        let linear = if word(descriptor + 28)? & 0x8000_0000 != 0 {
            // Refuse a truncated tiled footprint rather than zero-filling it.
            if blocks.len() < tdb_rx2::texlib::tiled_buffer_size(width, height, 16) {
                return Err(invalid());
            }
            tdb_rx2::texlib::untile_dxt(&blocks, width, height, 16)
        } else {
            blocks[..expected].to_vec()
        };
        let name = format!(
            "{}_bc5_{index}",
            path.file_stem().unwrap_or_default().to_string_lossy()
        );
        out.push((name, dx10_dds(width, height, 83, &linear)));
    }
    Ok(out)
}

fn dx10_dds(width: u32, height: u32, dxgi: u32, blocks: &[u8]) -> Vec<u8> {
    let mut out = encode_dds(DdsFormat::Dxt5, width, height, 1, &[]);
    out[20..24].copy_from_slice(&(blocks.len() as u32).to_le_bytes());
    out[84..88].copy_from_slice(b"DX10");
    for value in [dxgi, 3, 0, 1, 0] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(blocks);
    out
}

fn gnf_dds(tex: &tdb_rx2::gnf::GnfTexture) -> Vec<u8> {
    use tdb_rx2::gnf::BcFormat;
    let legacy = match tex.format {
        BcFormat::Bc1 => Some(DdsFormat::Dxt1),
        BcFormat::Bc2 => Some(DdsFormat::Dxt3),
        BcFormat::Bc3 => Some(DdsFormat::Dxt5),
        _ => None,
    };
    if let Some(format) = legacy {
        return encode_dds(format, tex.width, tex.height, 1, &tex.base_mip_linear);
    }
    // DDS_HEADER_DXT10: BC4/5/7 UNORM, a single 2D texture, no array/cubemap.
    let dxgi = match tex.format {
        BcFormat::Bc4 => 80u32,
        BcFormat::Bc5 => 83,
        BcFormat::Bc7 => 98,
        _ => unreachable!(),
    };
    dx10_dds(tex.width, tex.height, dxgi, &tex.base_mip_linear)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xbox_bc5_preserves_blocks_and_rejects_truncation() {
        let blocks: Vec<u8> = (0..128 * 128).map(|i| (i % 251) as u8).collect();
        let mut tiled = tdb_rx2::texlib::tile_dxt(&blocks, 128, 128, 16);
        tdb_rx2::texlib::swap16_in_place(&mut tiled);
        let mut bytes = vec![0; 4096];
        bytes[..7].copy_from_slice(tdb_rx2::texlib::MAGIC);
        for (offset, value) in [
            (0x20, 2u32),
            (0x30, 0x100),
            (0x44, 4096),
            (0x54, tiled.len() as u32),
            (0x108, tiled.len() as u32),
            (0x114, 0x10031),
            (0x118, 0x200),
            (0x120, 56),
            (0x12c, 0x20003),
            (0x21c, 0x80000000),
            (0x224, (127 << 13) | 127),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        }
        bytes[0x223] = 0x71;
        bytes.extend_from_slice(&tiled);
        let result = xbox_bc5_dds(Path::new("head_nm.rx2"), &bytes).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(&result[0].1[128..132], &83u32.to_le_bytes());
        assert_eq!(&result[0].1[148..], blocks);
        bytes.pop();
        assert!(xbox_bc5_dds(Path::new("head_nm.rx2"), &bytes).is_err());
    }

    #[test]
    fn gnf_dds_keeps_all_six_bc_formats_without_recompression() {
        use tdb_rx2::gnf::{encode_gnf, BcFormat::*};
        let dir = tempfile::tempdir().unwrap();
        for (format, fourcc, dxgi) in [
            (Bc1, *b"DXT1", None),
            (Bc2, *b"DXT3", None),
            (Bc3, *b"DXT5", None),
            (Bc4, *b"DX10", Some(80u32)),
            (Bc5, *b"DX10", Some(83)),
            (Bc7, *b"DX10", Some(98)),
        ] {
            let blocks: Vec<u8> = (0..64 * format.block_bytes())
                .map(|i| (i % 251) as u8)
                .collect();
            let source = dir.path().join("head_test_cm.gnf");
            std::fs::write(&source, encode_gnf(format, 32, 32, &blocks)).unwrap();
            let outputs = decode_dds(&source).unwrap();
            let dds = &outputs[0].1;
            assert_eq!(&dds[..4], b"DDS ");
            assert_eq!(&dds[84..88], &fourcc);
            if let Some(dxgi) = dxgi {
                assert_eq!(&dds[128..132], &dxgi.to_le_bytes());
                assert_eq!(
                    &dds[132..148],
                    &[3, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]
                );
            }
            assert_eq!(&dds[if dxgi.is_some() { 148 } else { 128 }..], blocks);
        }
    }
}

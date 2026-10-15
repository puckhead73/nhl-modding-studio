//! Export original GNF files, lossless full-mip DDS, and viewable PNG base mips.
//! Usage: export_gnf_maps input-directory output-directory
use std::{error::Error, fs, path::Path};
use tdb_archive_codec::dds::{decode_dds_to_rgba, encode_dds, DdsFormat};
use tdb_rx2::gnf::{self, BcFormat};

// BC5_SNORM endpoints are signed, unlike the existing UNORM-only preview helper.
// Microsoft block-compression specification: two independently encoded BC4 channels.
fn signed_block(block: &[u8]) -> [f32; 16] {
    let a = (block[0] as i8).max(-127) as f32 / 127.;
    let b = (block[1] as i8).max(-127) as f32 / 127.;
    let mut palette = [a, b, 0., 0., 0., 0., -1., 1.];
    let end = if a > b { 8 } else { 6 };
    for (i, value) in palette.iter_mut().enumerate().take(end).skip(2) {
        *value = ((end - i) as f32 * a + (i - 1) as f32 * b) / (end - 1) as f32;
    }
    let bits = block[2..8]
        .iter()
        .enumerate()
        .fold(0u64, |n, (i, v)| n | ((*v as u64) << (8 * i)));
    std::array::from_fn(|i| palette[((bits >> (i * 3)) & 7) as usize])
}

fn signed_normal(data: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut rgba = vec![0; (w * h * 4) as usize];
    for (i, block) in data.chunks_exact(16).enumerate() {
        let r = signed_block(&block[..8]);
        let g = signed_block(&block[8..]);
        for p in 0..16 {
            let x = (i as u32 % w.div_ceil(4)) * 4 + p as u32 % 4;
            let y = (i as u32 / w.div_ceil(4)) * 4 + p as u32 / 4;
            if x >= w || y >= h {
                continue;
            }
            let z = (1. - r[p] * r[p] - g[p] * g[p]).max(0.).sqrt();
            let off = ((y * w + x) * 4) as usize;
            for (j, v) in [r[p], g[p], z].iter().enumerate() {
                rgba[off + j] = ((v * 0.5 + 0.5) * 255.).round() as u8;
            }
            rgba[off + 3] = 255;
        }
    }
    rgba
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: export_gnf_maps input-directory output-directory".into());
    }
    let out = Path::new(&args[2]);
    fs::create_dir_all(out)?;
    let mut report = Vec::new();
    for entry in fs::read_dir(&args[1])? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("gnf") {
            continue;
        }
        let bytes = fs::read(&path)?;
        let tex = gnf::parse(&bytes)?;
        let read = |off| u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
        let levels = ((read(0x1c) >> 16) & 15) + 1;
        let numeric_format = (read(0x14) >> 26) & 15;
        let block = match tex.format {
            BcFormat::Bc1 | BcFormat::Bc4 => 8,
            _ => 16,
        };
        let mut payload = Vec::new();
        let mut offset = 256;
        for level in 0..levels {
            let w = (tex.width >> level).max(1);
            let h = (tex.height >> level).max(1);
            let size = (w.div_ceil(32) * h.div_ceil(32) * 64 * block) as usize;
            let mut view = bytes[..256].to_vec();
            view[0x18..0x1c].copy_from_slice(&((w - 1) | ((h - 1) << 14)).to_le_bytes());
            view.extend_from_slice(
                bytes
                    .get(offset..offset + size)
                    .ok_or("truncated mip chain")?,
            );
            payload.extend(gnf::parse(&view)?.base_mip_linear);
            offset += size;
        }
        assert_eq!(offset, bytes.len());
        let fmt = match tex.format {
            BcFormat::Bc1 => DdsFormat::Dxt1,
            BcFormat::Bc2 => DdsFormat::Dxt3,
            _ => DdsFormat::Dxt5,
        };
        let mut dds = encode_dds(fmt, tex.width, tex.height, levels, &payload);
        let rgba = match tex.format {
            BcFormat::Bc1 | BcFormat::Bc2 | BcFormat::Bc3 => decode_dds_to_rgba(&dds)?.rgba,
            BcFormat::Bc5 => {
                // Preserve signedness and both original compressed channels exactly.
                assert!(numeric_format <= 1);
                dds[84..88].copy_from_slice(b"DX10");
                let dx10: Vec<u8> = [83u32 + numeric_format, 3, 0, 1, 0]
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect();
                dds.splice(128..128, dx10);
                if numeric_format == 1 {
                    signed_normal(&tex.base_mip_linear, tex.width, tex.height)
                } else {
                    gnf::decode_bc5(&tex.base_mip_linear, tex.width, tex.height)
                }
            }
            _ => return Err("unsupported export format".into()),
        };
        let stem = path.file_stem().unwrap().to_str().unwrap();
        fs::write(out.join(path.file_name().unwrap()), &bytes)?;
        fs::write(out.join(format!("{stem}.dds")), &dds)?;
        image::save_buffer(
            out.join(format!("{stem}.png")),
            &rgba,
            tex.width,
            tex.height,
            image::ColorType::Rgba8,
        )?;
        for (channel, index) in [("R", 0), ("G", 1), ("B", 2), ("A", 3)] {
            let plane: Vec<u8> = rgba.chunks_exact(4).map(|p| p[index]).collect();
            image::save_buffer(
                out.join(format!("{stem}_{channel}.png")),
                &plane,
                tex.width,
                tex.height,
                image::ColorType::L8,
            )?;
        }
        report.push(serde_json::json!({"name":stem,"width":tex.width,"height":tex.height,"format":format!("{:?}",tex.format),"numeric_format":numeric_format,"mips":levels,"sha256":tdb_rx2::mcd_write::fingerprint(&bytes),"header_words":(0..12).map(|i|format!("{:08x}",read(i*4))).collect::<Vec<_>>(),"normal_png_blue_reconstructed":tex.format==BcFormat::Bc5}));
        println!(
            "EXPORTED {stem} {}x{} {:?}, {levels} mips",
            tex.width, tex.height, tex.format
        );
    }
    fs::write(out.join("maps.json"), serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[test]
fn signed_endpoints_and_extremes() {
    assert!((signed_block(&[127, 129, 2, 0, 0, 0, 0, 0])[0] - 5.0 / 7.0).abs() < 1e-6);
    assert!((signed_block(&[129, 127, 2, 0, 0, 0, 0, 0])[0] + 3.0 / 5.0).abs() < 1e-6);
    assert_eq!(signed_block(&[128, 127, 0, 0, 0, 0, 0, 0]), [-1.; 16]);
    assert_eq!(
        signed_block(&[0, 0, 255, 255, 255, 255, 255, 255]),
        [1.; 16]
    );
    let decoded = signed_normal(&[0; 16], 4, 4);
    assert!(decoded.chunks_exact(4).all(|p| p == [128, 128, 255, 255]));
}

//! Measured 132-byte NHL22–24 PS4 texture resources, 8×8 Morton microtiles.
//! Exports the full-resolution base mip as DDS without recompressing GPU blocks.
use crate::{hex, invalid, le, slice, Result};

pub fn chunk_guid(resource: &[u8]) -> Result<String> {
    if resource.len() != 132 {
        return Err(invalid("unsupported texture resource size"));
    }
    Ok(hex(slice(resource, 32, 16)?))
}

pub fn dds(resource: &[u8], pixels: &[u8]) -> Result<Vec<u8>> {
    chunk_guid(resource)?;
    let width = u16::from_le_bytes(resource[22..24].try_into().unwrap()) as usize;
    let height = u16::from_le_bytes(resource[24..26].try_into().unwrap()) as usize;
    let depth = u16::from_le_bytes(resource[26..28].try_into().unwrap());
    if width == 0
        || height == 0
        || width > 16384
        || height > 16384
        || depth != 1
        || le(resource, 8)? != 0
    {
        return Err(invalid("unsupported texture dimensions/type"));
    }
    // Values measured from the shipped head families. Refuse other formats.
    let (dxgi, element_bytes, block) = match le(resource, 12)? {
        0x12 => (28u32, 4usize, 1usize), // RGBA8
        0x36 => (71, 8, 4),              // BC1
        0x37 => (72, 8, 4),              // BC1 sRGB
        0x3c => (77, 16, 4),             // BC3
        0x3d => (78, 16, 4),             // BC3 sRGB
        0x3e => (80, 8, 4),              // BC4
        0x3f => (83, 16, 4),             // BC5
        0x42 => (98, 16, 4),             // BC7
        0x43 => (99, 16, 4),             // BC7 sRGB
        format => {
            return Err(invalid(format!(
                "unsupported Frostbite texture format {format:#x}"
            )))
        }
    };
    if resource[20..22] != [1, 8] {
        return Err(invalid("unqualified texture layout flags"));
    }
    let grid_w = width.div_ceil(block);
    let grid_h = height.div_ceil(block);
    let tiled_bytes = grid_w.div_ceil(8) * grid_h.div_ceil(8) * 64 * element_bytes;
    let declared = le(resource, 48)? as usize;
    if declared < tiled_bytes || pixels.len() < declared {
        return Err(invalid("missing full-resolution texture mip"));
    }
    let size = grid_w * grid_h * element_bytes;
    if size > 256 * 1024 * 1024 {
        return Err(invalid("texture exceeds output limit"));
    }
    let mut linear = vec![0; size];
    for y in 0..grid_h {
        for x in 0..grid_w {
            let mut morton = 0;
            for bit in 0..3 {
                morton |= ((x >> bit) & 1) << (bit * 2);
                morton |= ((y >> bit) & 1) << (bit * 2 + 1);
            }
            let source = (((y / 8) * grid_w.div_ceil(8) + x / 8) * 64 + morton) * element_bytes;
            let destination = (y * grid_w + x) * element_bytes;
            linear[destination..destination + element_bytes]
                .copy_from_slice(&pixels[source..source + element_bytes]);
        }
    }
    let mut out = vec![0; 148];
    out[..4].copy_from_slice(b"DDS ");
    for (offset, value) in [
        (4, 124u32),
        (8, if block == 1 { 0x100f } else { 0x81007 }),
        (12, height as u32),
        (16, width as u32),
        (
            20,
            if block == 1 {
                (width * 4) as u32
            } else {
                size as u32
            },
        ),
        (28, 1),
        (76, 32),
        (80, 4),
        (108, 0x1000),
        (128, dxgi),
        (132, 3),
        (140, 1),
    ] {
        out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    out[84..88].copy_from_slice(b"DX10");
    out.extend(linear);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn base_mip_morton_order_and_missing_streamed_mip() {
        let mut resource = vec![0; 132];
        resource[12..16].copy_from_slice(&0x37u32.to_le_bytes());
        resource[20..22].copy_from_slice(&[1, 8]);
        resource[22..24].copy_from_slice(&32u16.to_le_bytes());
        resource[24..26].copy_from_slice(&32u16.to_le_bytes());
        resource[26..28].copy_from_slice(&1u16.to_le_bytes());
        resource[48..52].copy_from_slice(&512u32.to_le_bytes());
        let pixels: Vec<_> = (0..64u8).flat_map(|i| [i; 8]).collect();
        let result = dds(&resource, &pixels).unwrap();
        assert_eq!(
            &result[148..180],
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5,
                5, 5, 5, 5
            ]
        );
        assert!(dds(&resource, &pixels[..128]).is_err());
    }
}

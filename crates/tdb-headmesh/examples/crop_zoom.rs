//! Crop a region of a texlib raster and magnify it nearest-neighbour, so per-texel
//! structure (compression banding, bake streaks, resample ringing) is visible by eye.

fn main() {
    let mut a = std::env::args().skip(1);
    let path = a.next().expect("texlib");
    let out = a.next().expect("out.png");
    let x0: u32 = a.next().unwrap().parse().unwrap();
    let y0: u32 = a.next().unwrap().parse().unwrap();
    let cw: u32 = a.next().unwrap().parse().unwrap();
    let chh: u32 = a.next().unwrap().parse().unwrap();
    let zoom: u32 = a.next().unwrap_or_else(|| "4".into()).parse().unwrap();

    let bytes = std::fs::read(&path).unwrap();
    let lib = tdb_rx2::texlib::parse(&bytes).unwrap();
    let idx = lib
        .textures
        .iter()
        .position(|t| t.name.to_ascii_lowercase().contains("head"))
        .expect("head slot");
    let t = &lib.textures[idx];
    let mut tiled = lib.base_mip_tiled(idx).unwrap().to_vec();
    tdb_rx2::texlib::swap16_in_place(&mut tiled);
    let linear = tdb_rx2::texlib::untile_dxt(&tiled, t.width, t.height, t.format.bytes_per_block());
    let fourcc = t.format.fourcc().unwrap();
    let mut dds = Vec::new();
    dds.extend_from_slice(b"DDS ");
    let mut hdr = [0u8; 124];
    hdr[0..4].copy_from_slice(&124u32.to_le_bytes());
    hdr[4..8].copy_from_slice(&0x0008_1007u32.to_le_bytes());
    hdr[8..12].copy_from_slice(&t.height.to_le_bytes());
    hdr[12..16].copy_from_slice(&t.width.to_le_bytes());
    hdr[16..20].copy_from_slice(&(linear.len() as u32).to_le_bytes());
    hdr[72..76].copy_from_slice(&32u32.to_le_bytes());
    hdr[76..80].copy_from_slice(&0x4u32.to_le_bytes());
    hdr[80..84].copy_from_slice(fourcc);
    hdr[108..112].copy_from_slice(&0x1000u32.to_le_bytes());
    dds.extend_from_slice(&hdr);
    dds.extend_from_slice(&linear);
    let (rgba, w, _h) = tdb_headmesh::texture::decode_source_dds(&dds).unwrap();

    let mut buf = vec![0u8; (cw * zoom * chh * zoom * 4) as usize];
    for y in 0..chh * zoom {
        for x in 0..cw * zoom {
            let sx = x0 + x / zoom;
            let sy = y0 + y / zoom;
            let s = ((sy * w + sx) * 4) as usize;
            let d = ((y * cw * zoom + x) * 4) as usize;
            buf[d..d + 4].copy_from_slice(&rgba[s..s + 4]);
        }
    }
    image::save_buffer(&out, &buf, cw * zoom, chh * zoom, image::ColorType::Rgba8).unwrap();
    println!("wrote {out}");
}

//! Decode a texlib's head raster to PNG so a built atlas can be checked by eye.
//!
//! The stored order is tiled → `swap16`, so decoding reverses it (`swap16` → untile)
//! before handing the DXT payload to the shared DDS decoder. Skipping the swap yields a
//! recognisable but neon-miscoloured image — a useful reminder that "the layout looks
//! right" is not the same as "the raster is right".

fn main() {
    let mut a = std::env::args().skip(1);
    let path = a
        .next()
        .expect("usage: dump_texlib_head <texlib.rx2> <out.png>");
    let out = a
        .next()
        .expect("usage: dump_texlib_head <texlib.rx2> <out.png>");
    let bytes = std::fs::read(&path).unwrap();
    let lib = tdb_rx2::texlib::parse(&bytes).unwrap();
    for (i, t) in lib.textures.iter().enumerate() {
        println!("{i}: {} {}x{} {:?}", t.name, t.width, t.height, t.format);
    }
    let idx = lib
        .textures
        .iter()
        .position(|t| t.name.to_ascii_lowercase().contains("head"))
        .expect("head slot");
    let t = &lib.textures[idx];
    // Stored order is tiled → swap16, so decoding is swap16 → untile (see tdb_rx2::xpr).
    let mut tiled = lib.base_mip_tiled(idx).expect("base mip").to_vec();
    tdb_rx2::texlib::swap16_in_place(&mut tiled);
    let linear = tdb_rx2::texlib::untile_dxt(&tiled, t.width, t.height, t.format.bytes_per_block());
    // Wrap the raw DXT in a DDS so the shared decoder can read it.
    let fourcc = t.format.fourcc().expect("dxt slot");
    let mut dds = Vec::new();
    dds.extend_from_slice(b"DDS ");
    let mut hdr = [0u8; 124];
    hdr[0..4].copy_from_slice(&124u32.to_le_bytes());
    hdr[4..8].copy_from_slice(&0x0008_1007u32.to_le_bytes()); // caps|height|width|pixelformat|linearsize
    hdr[8..12].copy_from_slice(&t.height.to_le_bytes());
    hdr[12..16].copy_from_slice(&t.width.to_le_bytes());
    hdr[16..20].copy_from_slice(&(linear.len() as u32).to_le_bytes());
    // ddspf begins at header offset 72 (after dwReserved1[11]).
    hdr[72..76].copy_from_slice(&32u32.to_le_bytes()); // ddspf.dwSize
    hdr[76..80].copy_from_slice(&0x4u32.to_le_bytes()); // DDPF_FOURCC
    hdr[80..84].copy_from_slice(fourcc);
    hdr[108..112].copy_from_slice(&0x1000u32.to_le_bytes()); // DDSCAPS_TEXTURE
    dds.extend_from_slice(&hdr);
    dds.extend_from_slice(&linear);

    let (rgba, w, h) = tdb_headmesh::texture::decode_source_dds(&dds).unwrap();
    image::save_buffer(&out, &rgba, w, h, image::ColorType::Rgba8).unwrap();
    println!("wrote {out} ({w}x{h}) from slot {idx} ({})", t.name);
}

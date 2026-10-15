use tdb_frostbite::{blocks, db, install, manifest, toc};
fn put(b: &mut [u8], p: usize, v: u32) {
    b[p..p + 4].copy_from_slice(&v.to_be_bytes());
}
fn stored(data: &[u8]) -> Vec<u8> {
    let n = data.len() as u32;
    let mut b = n.to_be_bytes().to_vec();
    b.extend([0, 0x70 | ((n >> 16) as u8)]);
    b.extend((n as u16).to_be_bytes());
    b.extend(data);
    b
}
fn empty_manifest() -> Vec<u8> {
    let mut b = 32u32.to_be_bytes().to_vec();
    for v in [0x9d798ed6u32, 0, 0, 0, 0, u32::MAX, 0, 0] {
        b.extend(v.to_le_bytes());
    }
    b
}
fn plain_toc() -> Vec<u8> {
    // One inline bundle, one manifest span, one TOC-level chunk.
    let mut b = vec![0; 160];
    for (i, v) in [60, 64, 1, 80, 84, 1, 104, 104, 157, 104, 3, 0, 0, 0, 0]
        .into_iter()
        .enumerate()
    {
        put(&mut b, i * 4, v);
    }
    put(&mut b, 64, 0);
    put(&mut b, 68, 0x40000029);
    b[72..80].copy_from_slice(&116u64.to_be_bytes());
    put(&mut b, 100, 0x01000000);
    put(&mut b, 104, 1);
    put(&mut b, 108, 36);
    put(&mut b, 112, 11);
    put(&mut b, 128, 1);
    put(&mut b, 148, 1);
    put(&mut b, 152, 0); // Adjust span follows 32-byte record header.
    b.resize(171, 0);
    put(&mut b, 68, 0x4000002d);
    put(&mut b, 32, 161);
    put(&mut b, 148, 1);
    put(&mut b, 152, 0);
    put(&mut b, 156, 36);
    b[160] = 1;
    b[161..165].copy_from_slice(b"abc\0");
    b.truncate(165);
    b
}
#[test]
fn dbobject_exact_lengths_types_and_wrapper() {
    let b = [0x82, 8, 8, b'h', 0, 1, 0, 0, 0, 0];
    assert_eq!(db::parse(&b).unwrap()["h"], 1);
    for end in 0..b.len() {
        assert!(db::parse(&b[..end]).is_err());
    }
    let mut signed = vec![0; 0x22c];
    signed[..4].copy_from_slice(&[0, 0xd1, 0xce, 1]);
    signed.extend(b);
    assert_eq!(db::parse(&signed).unwrap()["h"], 1);
    signed[3] = 0;
    assert!(db::parse(&signed).is_err());
    assert!(db::parse(&[0x82, 255, 255, 255, 255, 255]).is_err());
    assert!(db::parse(&[0x82, 1, 0, 9]).is_err());
}
#[test]
fn manifest_empty_is_valid_and_all_truncations_fail() {
    let b = empty_manifest();
    assert!(manifest::parse(&b).unwrap().is_empty());
    for end in 0..b.len() {
        assert!(manifest::parse(&b[..end]).is_err());
    }
    let mut wrong = b;
    wrong[4] = 0;
    assert!(manifest::parse(&wrong).is_err());
}
#[test]
fn manifest_retains_names_hashes_and_resource_metadata() {
    let mut body = Vec::new();
    for v in [0x9d798ed6u32, 2, 1, 1, 0, 116, 0, 0] {
        body.extend(v.to_le_bytes());
    }
    body.extend([1; 20]);
    body.extend([2; 20]);
    for v in [0u32, 20, 4, 30, 0x6bde20ba] {
        body.extend(v.to_le_bytes());
    }
    body.extend([3; 16]);
    body.extend(0x1234u64.to_le_bytes());
    body.extend(b"one\0two\0");
    let mut b = (body.len() as u32).to_be_bytes().to_vec();
    b.extend(body);
    let a = manifest::parse(&b).unwrap();
    assert_eq!(a[0].name, "one");
    assert_eq!(a[1].name, "two");
    assert_eq!(a[1].resource_id.as_deref(), Some("0000000000001234"));
    assert_eq!(a[1].resource_type, Some(0x6bde20ba));
}
#[test]
fn blocks_extended_sizes_mixed_codecs_and_limits() {
    let data = vec![42; 0x10001];
    let b = stored(&data);
    assert_eq!(blocks::decode_stored(&b, data.len()).unwrap(), data);
    assert!(blocks::decode_stored(&b, 100).is_err());
    assert!(blocks::inspect(&b[..b.len() - 1]).is_err());
    let mut mixed = stored(b"abc");
    let mut compressed = stored(b"xyz");
    compressed[4] = 0x11;
    mixed.extend(compressed);
    assert!(blocks::decode_stored(&mixed, 100)
        .unwrap_err()
        .to_string()
        .contains("0x11"));
    assert_eq!(
        blocks::decode_with(&mixed, 100, |_, _, _| Ok(b"def".to_vec())).unwrap(),
        b"abcdef"
    );
    assert!(blocks::decode_with(&mixed, 100, |_, _, _| Ok(vec![0])).is_err());
}
#[test]
fn toc_spans_names_and_invalid_indices() {
    let b = plain_toc();
    let t = toc::parse(&b).unwrap();
    assert_eq!(t.bundles[0].name, "abc");
    assert_eq!(t.bundles[0].spans[0].size, 36);
    assert_eq!(t.chunks[0].span.offset, 36);
    for end in 0..b.len() {
        assert!(toc::parse(&b[..end]).is_err(), "accepted truncation {end}");
    }
    let mut wrong = b.clone();
    put(&mut wrong, 100, 0x01000001);
    assert!(toc::parse(&wrong).is_err());
    let mut wrong = b;
    wrong[160] = 0;
    assert!(toc::parse(&wrong).is_err());
}
#[test]
fn huffman_names_are_bit_offsets_and_reject_cycles() {
    let mut b = plain_toc();
    b.resize(188, 0);
    put(&mut b, 44, 4);
    put(&mut b, 32, 168);
    put(&mut b, 48, 1);
    put(&mut b, 52, 4);
    put(&mut b, 56, 172);
    // pair 0 = a/b; root pair 1 = NUL/pair0. LSB bits: 1,0 (a), 0 (NUL).
    put(&mut b, 168, 1);
    put(&mut b, 172, !(b'a' as u32));
    put(&mut b, 176, !(b'b' as u32));
    put(&mut b, 180, u32::MAX);
    put(&mut b, 184, 0);
    assert_eq!(toc::parse(&b).unwrap().bundles[0].name, "a");
    put(&mut b, 184, 1);
    assert!(toc::parse(&b).is_err());
}
#[test]
fn malformed_inputs_do_not_panic() {
    let mut seed = 12345678u32;
    for len in 0..512 {
        let mut b = vec![0; len];
        for v in &mut b {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *v = (seed >> 24) as u8;
        }
        let _ = db::parse(&b);
        let _ = toc::parse(&b);
        let _ = manifest::parse(&b);
        let _ = blocks::inspect(&b);
    }
}
#[test]
fn install_paths_and_span_bounds() {
    let dir = tempfile::tempdir().unwrap();
    assert!(install::inspect(dir.path()).is_err());
    assert!(install::cas_path(dir.path(), 0xff000001).is_err());
    assert!(install::read_toc(dir.path(), "../Data", "Gen4b/ui").is_err());
    assert!(install::read_toc(dir.path(), "Data", "../secret").is_err());
    let p = dir
        .path()
        .join("Data/Gen4b/superbundlelayout/nhl_installpackage_00");
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join("cas_01.cas"), [0; 8]).unwrap();
    assert!(install::read_span(
        dir.path(),
        &toc::Span {
            cas: 1,
            offset: 7,
            size: 2
        }
    )
    .is_err());
    assert_eq!(
        install::read_span(
            dir.path(),
            &toc::Span {
                cas: 1,
                offset: 7,
                size: 1
            }
        )
        .unwrap(),
        [0]
    );
}

#[test]
fn install_onboarding_and_chunk_export_are_read_only() {
    // DbObject root -> superBundles list -> anonymous object -> name string.
    fn container(tag: u8, name: &str, mut entries: Vec<u8>) -> Vec<u8> {
        entries.push(0);
        assert!(entries.len() < 128);
        let mut b = vec![tag];
        if tag & 128 == 0 {
            b.extend(name.as_bytes());
            b.push(0);
        }
        b.push(entries.len() as u8);
        b.extend(entries);
        b
    }
    let mut field = vec![7];
    field.extend(b"name\0");
    field.push(9);
    field.extend(b"Gen4b/ui\0");
    let layout = container(
        0x82,
        "",
        container(1, "superBundles", container(0x82, "", field)),
    );
    let dir = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let base = dir.path().join("Data/Gen4b");
    std::fs::create_dir_all(base.join("superbundlelayout/nhl_installpackage_00")).unwrap();
    std::fs::write(dir.path().join("Data/layout.toc"), layout).unwrap();
    std::fs::write(base.join("ui.toc"), plain_toc()).unwrap();
    let mut cas = empty_manifest();
    cas.extend(stored(b"abc"));
    let cas_path = base.join("superbundlelayout/nhl_installpackage_00/cas_01.cas");
    std::fs::write(&cas_path, &cas).unwrap();
    let report = install::inspect(dir.path()).unwrap();
    assert_eq!(report.layers[0].superbundles, ["Gen4b/ui"]);
    assert_eq!(
        install::bundles(dir.path(), "Data", "Gen4b/ui", "abc", 0, 100)
            .unwrap()
            .matched,
        1
    );
    assert!(install::assets(dir.path(), "Data", "Gen4b/ui", 0)
        .unwrap()
        .assets
        .is_empty());
    let dest = out.path().join("chunk.bin");
    assert_eq!(
        install::export_chunk(dir.path(), "Data", "Gen4b/ui", 0, true, &dest).unwrap(),
        3
    );
    assert_eq!(std::fs::read(&dest).unwrap(), b"abc");
    assert!(install::export_chunk(dir.path(), "Data", "Gen4b/ui", 0, true, &dest).is_err());
    assert!(install::export_chunk(
        dir.path(),
        "Data",
        "Gen4b/ui",
        0,
        false,
        &dir.path().join("overwrite.bin")
    )
    .is_err());
    assert_eq!(std::fs::read(cas_path).unwrap(), cas);
}

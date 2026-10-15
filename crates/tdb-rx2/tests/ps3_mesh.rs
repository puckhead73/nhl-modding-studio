use std::path::PathBuf;
use tdb_rx2::ps3_mesh;
#[test]
#[ignore = "requires NHLMS_PS3_MESH_FIXTURES pointing at Legacy rendering; reads original files only"]
fn legacy_edge_read_write_and_rejection() {
    let root = PathBuf::from(
        std::env::var_os("NHLMS_PS3_MESH_FIXTURES").expect("Legacy rendering fixture root"),
    );
    let cases = [
        ("player/player_66.rpsgl", 3643, 6298),
        ("player/player_67.rpsgl", 4200, 7535),
        ("player/player_71.rpsgl", 4751, 7772),
        ("player/player_76.rpsgl", 4239, 7356),
        ("player/player_80.rpsgl", 4234, 7356),
        ("player/player_88.rpsgl", 4239, 7356),
        ("player/player_89.rpsgl", 4059, 7268),
        ("skate/playerskate_0.rpsgl", 528, 652),
        ("helmet/helmet_110_0.rpsgl", 1692, 2178),
        ("glove/glove_0.rpsgl", 780, 1000),
        ("playerstick/playerstick_0_2.rpsgl", 229, 374),
        ("trapper/trapper_0_0.rpsgl", 749, 1048),
        ("goaliehelmet/cage_0_0_0.rpsgl", 606, 824),
    ];
    for (file, nv, nt) in cases {
        let original = std::fs::read(root.join(file)).unwrap();
        let d = ps3_mesh::parse(&original).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(
            d.asset
                .submeshes
                .iter()
                .map(|s| s.vertices.len())
                .sum::<usize>(),
            nv,
            "{file}"
        );
        if nt > 0 {
            assert_eq!(
                d.asset
                    .submeshes
                    .iter()
                    .map(|s| s.triangles.len())
                    .sum::<usize>(),
                nt,
                "{file}"
            );
        }
        assert_eq!(
            ps3_mesh::patch_positions(&original, &d.asset).unwrap(),
            original
        );
        assert_eq!(
            ps3_mesh::import_obj(&original, &ps3_mesh::export_obj(&d.asset)).unwrap(),
            original
        );
        let mut uv_edit=d.asset.clone();uv_edit.submeshes[0].vertices[0].uv[0]+=0.01;
        let uv_bytes=ps3_mesh::patch_mesh(&original,&uv_edit).unwrap();
        let uv_read=ps3_mesh::parse(&uv_bytes).unwrap();
        assert!((uv_read.asset.submeshes[0].vertices[0].uv[0]-uv_edit.submeshes[0].vertices[0].uv[0]).abs()<0.002);
        let mut normal_edit=d.asset.clone();normal_edit.submeshes[0].vertices[0].normal=[0.0,1.0,0.0];
        let normal_bytes=ps3_mesh::patch_mesh(&original,&normal_edit).unwrap();
        assert!(ps3_mesh::parse(&normal_bytes).unwrap().asset.submeshes[0].vertices[0].normal[1]>0.99);
        let mut edit = d.asset.clone();
        edit.submeshes[0].vertices[0].position[0] += 1.0 / 16384.0;
        let changed = ps3_mesh::patch_positions(&original, &edit).unwrap();
        assert_eq!(changed.len(), original.len());
        assert_ne!(changed, original);
        assert_eq!(
            ps3_mesh::parse(&changed).unwrap().asset.submeshes[0].vertices[0].position,
            edit.submeshes[0].vertices[0].position
        );
        edit.submeshes[0].triangles.reverse();
        assert!(ps3_mesh::patch_positions(&original, &edit).is_err());
        let mut bad = d.asset.clone();
        bad.submeshes[0].vertices[0].position[0] = f32::NAN;
        assert!(ps3_mesh::patch_positions(&original, &bad).is_err());
        bad.submeshes[0].vertices[0].position[0] = 1000.0;
        assert!(ps3_mesh::patch_positions(&original, &bad).is_err());
        for length in [8, 64, 192, original.len() / 2, original.len() - 1] {
            assert!(
                ps3_mesh::parse(&original[..length]).is_err(),
                "truncated {file}"
            );
        }
        assert_eq!(std::fs::read(root.join(file)).unwrap(), original);
        println!("{file}: decode, no-op, edit, invalid topology/range, truncation passed");
    }
}

#[test]
#[ignore = "requires NHLMS_PS3_MESH_FIXTURES"]
fn stadium_geometry_export_is_read_only() {
 let root=PathBuf::from(std::env::var_os("NHLMS_PS3_MESH_FIXTURES").expect("fixture root"));
 let bytes=std::fs::read(root.join("stadium/stadium_0_container_0.rpsgl")).unwrap();let d=ps3_mesh::parse(&bytes).unwrap();
 assert_eq!(d.asset.submeshes.iter().map(|s|s.vertices.len()).sum::<usize>(),85467);
 assert_eq!(d.asset.submeshes.iter().map(|s|s.triangles.len()).sum::<usize>(),58087);
 assert!(ps3_mesh::patch_mesh(&bytes,&d.asset).is_err());
 assert!(tdb_rx2::playermesh::write_glb_geometry_only(&d.asset).starts_with(b"glTF"));
}

#[test]
#[ignore = "requires NHLMS_PS3_MESH_FIXTURES and NHLMS_PS3_REFERENCE_JSON from independent RPSGL2 reader"]
// RPSGL2 omits the EDGE normal range scale; its normals are not a valid oracle.
// Normal decoding has an independent Xbox witness and encode/decode regression test.
fn agrees_with_reference_reader() {
 let root=PathBuf::from(std::env::var_os("NHLMS_PS3_MESH_FIXTURES").expect("fixture root"));
 let expected:serde_json::Value=serde_json::from_slice(&std::fs::read(std::env::var_os("NHLMS_PS3_REFERENCE_JSON").expect("reference JSON")).unwrap()).unwrap();
 for row in expected.as_array().unwrap() {
 let file=row["file"].as_str().unwrap();let d=ps3_mesh::parse(&std::fs::read(root.join(file)).unwrap()).unwrap();
 let vertices:Vec<_>=d.asset.submeshes.iter().flat_map(|s|s.vertices.iter()).collect();
 assert_eq!(vertices.len(),row["positions"].as_array().unwrap().len());
 for (i,v) in vertices.iter().enumerate(){for (key,actual) in [("positions",v.position.as_slice()),("uv",v.uv.as_slice())] {for (k,x) in actual.iter().enumerate(){let want=row[key][i][k].as_f64().unwrap();let tolerance=if key=="positions" {1e-4}else{2e-5};assert!((*x as f64-want).abs()<=tolerance,"{file} {key} vertex {i}/{k}: {x} vs {want}");}}}
 let mut offset=0u32;let mut triangles=Vec::new();for m in &d.asset.submeshes {triangles.extend(m.triangles.iter().map(|t|t.map(|i|i+offset)));offset+=m.vertices.len() as u32;}
 assert_eq!(serde_json::to_value(triangles).unwrap(),row["triangles"],"{file}");
 }
}


#[test]
#[ignore = "requires NHL09_PS3_SAMPLES and NHL09_XBOX_RENDERING"]
fn nhl09_early_edge_matches_xbox_geometry_and_uvs() {
    let ps3=PathBuf::from(std::env::var("NHL09_PS3_SAMPLES").unwrap());
    let xbox=PathBuf::from(std::env::var("NHL09_XBOX_RENDERING").unwrap());
    for name in ["glove/glove_0_0_0_0_0", "player/hair_10000_0_0_0"] {
        let bytes=std::fs::read(ps3.join(name).with_extension("rpsgl")).unwrap();
        let doc=ps3_mesh::parse(&bytes).unwrap();
        let raw=std::fs::read(xbox.join(name).with_extension("rx2")).unwrap();
        let raw=if tdb_archive_codec::is_refpack(&raw){tdb_archive_codec::refpack_decompress(&raw).unwrap()}else{raw};
        let reference=tdb_rx2::playermesh::parse_equipment_mesh(&raw).unwrap();
        let vertices:Vec<_>=reference.submeshes.iter().flat_map(|s|&s.vertices).collect();
        assert_eq!(doc.asset.submeshes.iter().map(|s|s.vertices.len()).sum::<usize>(),vertices.len());
        for v in doc.asset.submeshes.iter().flat_map(|s|&s.vertices) {
            assert!(vertices.iter().any(|r|
                v.position.iter().zip(r.position).all(|(a,b)|(a-b).abs()<0.001)
                && v.uv.iter().zip(r.uv).all(|(a,b)|(a-b).abs()<0.002)
                && v.normal.iter().zip(r.normal).all(|(a,b)|(a-b).abs()<0.03)), "{name}: unmatched vertex {v:?}");
        }
        assert!(ps3_mesh::patch_mesh(&bytes,&doc.asset).is_err(), "unqualified early mesh writes must remain unavailable");
        assert!(ps3_mesh::parse(&bytes[..bytes.len()/2]).is_err());
        println!("{name}: every position, normal and UV matches Xbox within codec precision");
    }
}

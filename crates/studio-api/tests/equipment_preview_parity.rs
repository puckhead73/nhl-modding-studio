//! Real game fixtures, read-only sources; extracted samples remain under target/.
use std::{
    fs,
    io::{BufRead, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
use studio_api::{dto::AssetRootsArgsDto, Registry};
use studio_domain::Platform;
fn repo(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)
}
fn directory<R: Read + Seek>(f: &mut R) -> Vec<(String, u64, u64)> {
    let mut h = [0u8; 48];
    f.read_exact(&mut h).unwrap();
    if h.starts_with(b"EB") {
        let header = tdb_archive_big::eb::EbHeader::parse(&h).unwrap();
        let mut b = vec![0; header.table_end()];
        f.seek(SeekFrom::Start(0)).unwrap();
        f.read_exact(&mut b).unwrap();
        return tdb_archive_big::eb::EbArchive::parse(&b)
            .unwrap()
            .entries
            .into_iter()
            .map(|e| (e.path, e.offset, e.size as u64))
            .collect();
    }
    if !h.starts_with(b"BIG") {
        return vec![];
    }
    f.seek(SeekFrom::Start(16)).unwrap();
    let mut r = std::io::BufReader::new(f);
    let mut b = h[..16].to_vec();
    let width = if matches!(&h[..4], b"BIG5" | b"BIG6" | b"BIG7" | b"BIG8" | b"BIG9") {
        9
    } else {
        8
    };
    for _ in 0..u32::from_be_bytes(h[8..12].try_into().unwrap()) {
        let mut n = vec![0; width];
        r.read_exact(&mut n).unwrap();
        b.extend(n);
        r.read_until(0, &mut b).unwrap();
    }
    tdb_archive_big::Archive::parse(b)
        .unwrap()
        .entries
        .into_iter()
        .map(|e| (e.name, e.offset as u64, e.size as u64))
        .collect()
}
fn sample<R: Read + Seek>(f: &mut R, out: &Path, head: u64) {
    for (name, offset, size) in directory(f) {
        let name = name.replace('\\', "/");
        let parts: Vec<_> = name.split('/').collect();
        if parts.len() < 3
            || parts[0] != "rendering"
            || !(name.ends_with(".rx2") || name.ends_with(".rpsgl"))
        {
            continue;
        }
        if ![
            "jersey",
            "pant",
            "sock",
            "glove",
            "helmet",
            "helmet_common",
            "skate",
            "player",
            "player_common",
        ]
        .contains(&parts[1])
        {
            continue;
        }
        let stem = Path::new(&name).file_stem().unwrap().to_string_lossy();
        if parts[1] == "player"
            && !stem.split('_').any(|n| n == head.to_string())
            && !stem.starts_with("eyes_")
            && !stem.starts_with("texlib_999")
        {
            continue;
        }
        if ["jersey", "pant"].contains(&parts[1])
            && stem.starts_with("texlib_")
            && stem.split('_').nth(2) != Some("0")
        {
            continue;
        }
        if parts[1] == "jersey" && stem.starts_with("name_") {
            continue;
        }
        if parts[1] == "sock"
            && stem.starts_with("sock_")
            && stem.ends_with("_cm")
            && stem.split('_').nth(2) != Some("0")
        {
            continue;
        }
        // Archive names are untrusted, even for read-only QA fixtures.
        assert!(Path::new(&name)
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_))));
        let target = out.join(name);
        if target.is_file() && target.metadata().unwrap().len() > 0 {
            continue;
        }
        f.seek(SeekFrom::Start(offset)).unwrap();
        let mut raw = vec![0u8; size as usize];
        f.read_exact(&mut raw).unwrap();
        let raw = match tdb_archive_big::eb::decode_payload(raw).unwrap() {
            tdb_archive_big::eb::Payload::Data(d) => d,
            _ => panic!("unsupported compression"),
        };
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, raw).unwrap();
    }
}
#[test]
#[ignore = "requires the local PS3 game sources and Xbox ISOs; samples only into target"]
fn every_title_has_paired_textured_skater_equipment() {
    let mut failures = vec![];
    for game in [
        "nhl10",
        "nhl11",
        "nhl12",
        "nhl13",
        "nhl14",
        "nhl15",
        "nhllegacy",
    ] {
        if std::env::var("KIT_GAME").is_ok_and(|g| g != game) {
            continue;
        }
        for (platform, label) in [(Platform::Ps3, "ps3"), (Platform::Xbox, "xbox360")] {
            if std::env::var("KIT_PLATFORM").is_ok_and(|g| g != label) {
                continue;
            }
            let reg = Registry::default();
            let d = repo(&format!("anchors/{label}/{game}/databases"));
            let id = reg
                .open_workspace_from_paths(
                    Some(game),
                    platform,
                    d.join("nhlng.db").to_str().unwrap(),
                    None,
                    d.join("nhlng-meta.xml").to_str().unwrap(),
                    repo("defs").to_str().unwrap(),
                )
                .unwrap()
                .project_id;
            let players = reg.list_players(id).unwrap();
            let player = players
                .iter()
                .find(|p| p.team_id == Some(0) && !p.is_goalie)
                .unwrap();
            let view = reg.player_view(id, player.player_id).unwrap().unwrap();
            let head = view
                .all_fields
                .iter()
                .find(|f| f.studio_field == "pPaD")
                .unwrap()
                .raw_int
                .unwrap();
            let out = repo(&format!("target/equipment-parity/{label}/{game}"));
            fs::create_dir_all(&out).unwrap();
            if platform == Platform::Ps3 {
                let folder = match game {
                    "nhl12" => "NHL 12/[BLUS 30791] NHL® 12".to_string(),
                    "nhllegacy" => "NHL Legacy - Vanilla".to_string(),
                    _ => format!("NHL {}", &game[3..]),
                };
                let root = Path::new(r"X:\Games\Emulation\Roms\Sony Playstation 3")
                    .join(folder)
                    .join("PS3_GAME/USRDIR");
                for e in fs::read_dir(root).unwrap().flatten().filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|s| s.eq_ignore_ascii_case("big"))
                }) {
                    sample(&mut fs::File::open(e.path()).unwrap(), &out, head);
                }
            } else {
                let root = Path::new(r"X:\Games\Emulation\Roms\XBOX360\ISOs");
                let prefix = if game == "nhllegacy" {
                    "NHL Legacy".to_string()
                } else {
                    format!("NHL {} ", &game[3..])
                };
                let path = fs::read_dir(root)
                    .unwrap()
                    .flatten()
                    .find(|e| {
                        e.file_name().to_string_lossy().starts_with(&prefix)
                            && e.path().extension().is_some_and(|s| s == "iso")
                    })
                    .unwrap()
                    .path();
                let iso = tdb_xiso::Xiso::open(path).unwrap();
                for member in iso
                    .members()
                    .iter()
                    .filter(|m| !m.is_dir() && !m.path.contains('/') && m.path.ends_with(".big"))
                {
                    sample(&mut iso.member_reader(member).unwrap(), &out, head);
                }
            }
            let args = AssetRootsArgsDto {
                install_dir: out.to_string_lossy().into_owned(),
                platform: Some(
                    if platform == Platform::Ps3 {
                        "ps3"
                    } else {
                        "xbox"
                    }
                    .into(),
                ),
                rendering_override: None,
                fe_override: None,
            };
            let jerseys = reg.list_team_jerseys(id, 0).unwrap();
            let style = jerseys.first().map(|j| j.style_raw).unwrap_or(0);
            let variant = jerseys.first().map(|j| j.variant_raw).unwrap_or(0);
            for gear in [false, true] {
                let kit = match reg.team_kit_preview(
                    id,
                    &args,
                    0,
                    style,
                    variant,
                    Some(player.player_id),
                    gear,
                ) {
                    Ok(k) => k,
                    Err(e) => {
                        failures.push(format!("{label}/{game} gear={gear}: {e}"));
                        continue;
                    }
                };
                for kind in ["gloves", "socks", "skates", "helmet"] {
                    let Some(p) = kit.pieces.iter().find(|p| p.kind == kind) else {
                        failures.push(format!("{label}/{game} gear={gear}: missing {kind}"));
                        continue;
                    };
                    let min = p
                        .submeshes
                        .iter()
                        .flat_map(|s| s.positions.iter().map(|v| v[0]))
                        .fold(f32::INFINITY, f32::min);
                    let max = p
                        .submeshes
                        .iter()
                        .flat_map(|s| s.positions.iter().map(|v| v[0]))
                        .fold(f32::NEG_INFINITY, f32::max);
                    let textured = p.submeshes.iter().all(|s| {
                        s.material_index
                            .is_some_and(|i| !kit.textures[i as usize].rgba_base64.is_empty())
                    });
                    println!("{label}/{game} gear={gear} {kind}: x={min:.3}..{max:.3} textured={textured} source={}",p.source_path.replace('\n'," + "));
                    if kind != "helmet" && !(min < -0.02 && max > 0.02) {
                        failures.push(format!("{label}/{game} gear={gear}: {kind} missing a side"));
                    }
                    if kind == "gloves"
                        && p.submeshes.iter().any(|s| {
                            s.material_index
                                .is_some_and(|i| !kit.textures[i as usize].name.contains("_cm"))
                        })
                    {
                        failures.push(format!("{label}/{game} gear={gear}: glove uses a raw mask instead of a painted diffuse"));
                    }
                    if !textured {
                        failures.push(format!(
                            "{label}/{game} gear={gear}: {kind} missing textures"
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

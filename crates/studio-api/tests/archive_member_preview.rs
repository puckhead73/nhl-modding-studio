//! R4 (23 Sep 2026), dispatch item 4 — the **archive-member texture preview** through the registry
//! seam: a texture inside an `EB\0\3` render archive (PS3 NHL 12 / 13 kit members in
//! `cacherender.big` / `nocacherender.big`) previews like a loose file, addressed by the archive's
//! install-relative path and the member's exact path. The kit card's archived components carry
//! both fields, so the card decodes them without parsing a display string. Read-only: no write
//! route exists for an archive member here. The default case is synthetic (a v`0x00000400` archive
//! with a raw RW4ps3 member); the real PS3 NHL 13 archive runs `--ignored` and decodes a
//! `chunklzx` member.

use std::path::{Path, PathBuf};

use studio_api::dto::AssetRootsArgsDto;
use studio_api::Registry;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn open(reg: &Registry, platform: &str, game: &str, db: &str) -> u32 {
    let dir = match platform {
        "xbox" => "xbox360",
        "ps3" => "ps3",
        _ => "ps4",
    };
    reg.open_workspace_from_paths(
        Some(game),
        studio_api::parse_platform(platform).unwrap(),
        &s(&repo(&format!("anchors/{dir}/{game}/databases/{db}.db"))),
        None,
        &s(&repo(&format!("anchors/{dir}/{game}/databases/{db}-meta.xml"))),
        &s(&repo("defs")),
    )
    .unwrap()
    .project_id
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-ebprev-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("PS3_GAME/USRDIR/rendering/boot")).unwrap();
        Tmp(d)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A one-slot RW4ps3 texlib (64 × 32 ARGB8), as `loose_texture_route.rs` builds it.
fn rw4ps3_64x32() -> Vec<u8> {
    let (w, h) = (64u32, 32u32);
    let hdr = 0x200usize;
    let slot: Vec<u8> = (0..w * h).flat_map(|i| [255u8, (i % 64) as u8 * 4, (i / 64) as u8 * 8, 0x40]).collect();
    let mut t = vec![0u8; hdr + slot.len()];
    t[..7].copy_from_slice(b"\x89RW4ps3");
    t[0x20..0x24].copy_from_slice(&2u32.to_be_bytes());
    t[0x30..0x34].copy_from_slice(&0x1B4u32.to_be_bytes());
    t[0x44..0x48].copy_from_slice(&(hdr as u32).to_be_bytes());
    let d = 0x17C;
    for (k, sg) in [0x6007u32, 0x6007, 0x8035].iter().enumerate() {
        t[d + 32 + k * 4..d + 36 + k * 4].copy_from_slice(&sg.to_be_bytes());
    }
    t[d + 48..d + 50].copy_from_slice(&(w as u16).to_be_bytes());
    t[d + 50..d + 52].copy_from_slice(&(h as u16).to_be_bytes());
    t[d + 52] = 1;
    let tb = 0x1B4;
    for (k, v) in [0u32, 0, slot.len() as u32, 0x80, 1].iter().enumerate() {
        t[tb + k * 4..tb + k * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    t[tb + 20..tb + 24].copy_from_slice(&[0, 1, 0, 0x30]);
    for (k, v) in [d as u32, 0, 0x38, 4, 0xA].iter().enumerate() {
        t[tb + 24 + k * 4..tb + 28 + k * 4].copy_from_slice(&v.to_be_bytes());
    }
    t[tb + 44..tb + 48].copy_from_slice(&[0, 2, 0, 3]);
    t[hdr..].copy_from_slice(&slot);
    t
}

/// A synthetic `EB\0\3` v`0x00000400` archive (16-byte entries) with raw payloads.
fn write_eb_v400(path: &Path, dirs: &[&str], members: &[(u16, &str, &[u8])]) {
    const HDR: usize = 0x30;
    let align16 = |n: usize| (n + 15) & !15;
    let name_slot = members.iter().map(|(_, n, _)| n.len() + 3).max().unwrap_or(3);
    let dir_slot = dirs.iter().map(|d| d.len() + 1).max().unwrap_or(1);
    let name_off = HDR + members.len() * 16;
    let dir_start = align16(name_off + members.len() * name_slot);
    let data_start = align16(dir_start + dirs.len() * dir_slot);
    let mut offsets = Vec::new();
    let mut cur = data_start;
    for (_, _, body) in members {
        offsets.push(cur);
        cur = align16(cur + body.len());
    }
    let mut out = vec![0u8; cur];
    out[0..4].copy_from_slice(b"EB\x00\x03");
    out[0x04..0x08].copy_from_slice(&(members.len() as u32).to_be_bytes());
    out[0x08..0x0C].copy_from_slice(&0x0000_0400u32.to_be_bytes());
    out[0x0C..0x10].copy_from_slice(&(name_off as u32).to_be_bytes());
    out[0x14] = name_slot as u8;
    out[0x15] = dir_slot as u8;
    out[0x16..0x18].copy_from_slice(&(dirs.len() as u16).to_be_bytes());
    out[0x18..0x20].copy_from_slice(&(cur as u64).to_be_bytes());
    for (i, (di, name, body)) in members.iter().enumerate() {
        let e = HDR + i * 16;
        out[e..e + 4].copy_from_slice(&((offsets[i] >> 4) as u32).to_be_bytes());
        out[e + 8..e + 12].copy_from_slice(&(body.len() as u32).to_be_bytes());
        let n = name_off + i * name_slot;
        out[n..n + 2].copy_from_slice(&di.to_be_bytes());
        out[n + 2..n + 2 + name.len()].copy_from_slice(name.as_bytes());
        out[offsets[i]..offsets[i] + body.len()].copy_from_slice(body);
    }
    for (i, d) in dirs.iter().enumerate() {
        let o = dir_start + i * dir_slot;
        out[o..o + d.len()].copy_from_slice(d.as_bytes());
    }
    std::fs::write(path, out).unwrap();
}

fn decode_data_url(url: &str) -> studio_domain::DecodedImage {
    use base64::Engine;
    let b64 = url.split_once(',').unwrap().1;
    let png = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
    studio_domain::decode_source_image_bytes(&png).unwrap()
}

/// The first team of the anchor that registers at least one kit row.
fn team_with_rows(reg: &Registry, id: u32) -> u64 {
    reg.list_teams(id).unwrap().into_iter().map(|t| t.team_id).find(|&t| !reg.list_team_jerseys(id, t).unwrap().is_empty()).expect("a team with kit rows")
}

/// A synthetic PS3 NHL 12 install: the kit textures of one real stock row inside
/// `cacherender.big` (jersey, pants, sock file 0) and `nocacherender.big` (sock file 1). The kit
/// card's components carry the archive path and member path; the preview route decodes the member
/// to the raster it holds; a member the table lacks and an archive path outside the root refuse.
#[test]
fn an_archived_kit_member_previews_through_the_archive_route_and_the_card_fields_address_it() {
    let tmp = Tmp::new("synthetic");
    let reg = Registry::default();
    let id = open(&reg, "ps3", "nhl12", "nhlng");
    let team = team_with_rows(&reg, id);
    let usrdir = tmp.0.join("PS3_GAME/USRDIR");
    let args = AssetRootsArgsDto {
        install_dir: s(&tmp.0),
        platform: Some("ps3".into()),
        rendering_override: Some(s(&usrdir.join("rendering"))),
        fe_override: None,
    };
    // The row's (style, art, variant) from the untouched contract listing.
    let first = reg.team_kits(id, &args, team).unwrap();
    let k = first.kits.iter().find(|k| k.refusal.is_none()).expect("a stock row");
    let (style, art, variant) = (k.style, k.team_art_id, k.variant);
    let jersey = format!("texlib_{style}_{art}_{variant}.rpsgl");
    let sock0 = format!("sock_{style}_0_{art}_{variant}_cm.rpsgl");
    let sock1 = format!("sock_{style}_1_{art}_{variant}_cm.rpsgl");
    let tex = rw4ps3_64x32();
    write_eb_v400(
        &usrdir.join("cacherender.big"),
        &["rendering/jersey", "rendering/pant", "rendering/sock"],
        &[(0, &jersey, &tex), (1, &jersey, &tex), (2, &sock0, &tex), (0, "texlib_9_9_9.rpsgl", b"not a texture")],
    );
    write_eb_v400(&usrdir.join("nocacherender.big"), &["rendering/sock"], &[(0, &sock1, &tex)]);

    let v = reg.team_kits(id, &args, team).unwrap();
    let k = v.kits.iter().find(|k| k.style == style && k.variant == variant).unwrap();
    assert_eq!(k.completeness, "complete", "{:?}", k.components.iter().map(|c| (&c.label, &c.status, &c.reason)).collect::<Vec<_>>());
    assert_eq!(k.components.len(), 4, "NHL 12: both sock files");
    for c in &k.components {
        assert_eq!(c.status, "present");
        assert!(c.rel_path.is_none() && c.path.is_none());
        let expected_archive = if c.label == "socks (file 1)" { "PS3_GAME/USRDIR/nocacherender.big" } else { "PS3_GAME/USRDIR/cacherender.big" };
        assert_eq!(c.archive_rel_path.as_deref(), Some(expected_archive), "{}", c.label);
        assert!(c.member_path.as_deref().is_some_and(|m| m.starts_with("rendering/") && m.ends_with(".rpsgl")), "{:?}", c.member_path);
        assert_eq!(c.archive_member.as_deref(), Some(format!("{}:{}", Path::new(expected_archive).file_name().unwrap().to_string_lossy(), c.member_path.as_deref().unwrap()).as_str()));
        // The card's fields address the member; the route decodes it.
        let p = reg.archive_member_texture_preview(&s(&tmp.0), c.archive_rel_path.as_deref().unwrap(), c.member_path.as_deref().unwrap(), 0, None).unwrap_or_else(|e| panic!("{}: {e}", c.label));
        assert_eq!((p.width, p.height, p.container.as_deref()), (64, 32, Some("texlib")));
        let img = decode_data_url(&p.data_url);
        assert_eq!((img.width, img.height), (64, 32));
        // A thumbnail scales but keeps the reported full size.
        let t = reg.archive_member_texture_preview(&s(&tmp.0), c.archive_rel_path.as_deref().unwrap(), c.member_path.as_deref().unwrap(), 16, None).unwrap();
        assert_eq!((t.width, t.height), (64, 32));
        assert!(decode_data_url(&t.data_url).width <= 16);
    }
    assert!(v.directories.iter().all(|d| d.status == "archived" && d.archive.as_deref().is_some_and(|a| a.contains("cacherender.big"))), "{:?}", v.directories);
    assert!(v.notes.iter().any(|n| n.contains("listed from the render archive")), "{:?}", v.notes);

    // Refusals: a member the file table lacks; a member that is not a texture; an archive outside
    // the install root; a missing archive.
    let err = reg.archive_member_texture_preview(&s(&tmp.0), "PS3_GAME/USRDIR/cacherender.big", "rendering/jersey/nope.rpsgl", 0, None).unwrap_err().to_string();
    assert!(err.contains("no member `rendering/jersey/nope.rpsgl`"), "{err}");
    let err = reg.archive_member_texture_preview(&s(&tmp.0), "PS3_GAME/USRDIR/cacherender.big", "rendering/jersey/texlib_9_9_9.rpsgl", 0, None).unwrap_err().to_string();
    assert!(err.contains("unrecognized texture container"), "{err}");
    let err = reg.archive_member_texture_preview(&s(&tmp.0), "../outside.big", "x", 0, None).unwrap_err().to_string();
    assert!(err.contains("escapes the install root"), "{err}");
    assert!(reg.archive_member_texture_preview(&s(&tmp.0), "PS3_GAME/USRDIR/missing.big", "x", 0, None).is_err());
}

/// The real PS3 NHL 13 render archives: the first `rendering/jersey/texlib_*.rpsgl` member
/// (NHL 13 keeps jerseys in `nocacherender.big` and pants / socks in `cacherender.big`; stored
/// `chunklzx`) decodes to a raster through the route. Read-only.
#[test]
#[ignore = "real PS3 NHL 13 install under X:/Games/Emulation/Roms/Sony Playstation 3 (read only): select with --ignored"]
fn a_real_ps3_nhl13_jersey_member_decodes_from_the_render_archives() {
    let root = PathBuf::from(r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 13");
    let usrdir = root.join("PS3_GAME/USRDIR");
    let mut found: Option<(&str, String)> = None;
    for a in studio_domain::kit_files::RENDER_ARCHIVES {
        let archive = usrdir.join(a);
        assert!(archive.is_file(), "{} missing (fails, never skips)", archive.display());
        let members = studio_domain::kit_files::list_eb_members(&archive).unwrap();
        if let Some(m) = members.iter().find(|m| m.starts_with("rendering/jersey/texlib_") && m.ends_with(".rpsgl")) {
            found = Some((a, m.clone()));
            break;
        }
    }
    let (archive_name, jersey) = found.expect("a jersey member in one of the render archives");
    let reg = Registry::default();
    let p = reg.archive_member_texture_preview(&s(&root), &format!("PS3_GAME/USRDIR/{archive_name}"), &jersey, 0, None).unwrap_or_else(|e| panic!("{archive_name}:{jersey}: {e}"));
    eprintln!("{archive_name}:{jersey}");
    eprintln!("{jersey}: {}x{} {:?}", p.width, p.height, p.container);
    assert_eq!(p.container.as_deref(), Some("texlib"));
    assert!(p.width >= 256 && p.height >= 256, "a jersey colour map is at least 256²");
    let img = decode_data_url(&p.data_url);
    assert_eq!((img.width, img.height), (p.width, p.height));
    assert!(img.rgba.chunks_exact(4).any(|px| px[0] != px[1] || px[1] != px[2]), "the decoded jersey is not a flat grey");
}

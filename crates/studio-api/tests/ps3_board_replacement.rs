//! Receipt 67 — PS3 board-ad raster replacement (C37; research 2026-09-28 handoff 10 §5 / §8) through the public Registry and the
//! copied-target transaction, on owned copied-target roots built from the genuine PS3 NHL Legacy (pair) and NHL 13 (archive-only)
//! installs. The installs are only read; every write lands in an owned root under the temp directory (never a user install).
//!
//! Per topology, with one DXT1 and one DXT3 raster (the research samples: `stadium_321_container_0` descriptor 124, DXT1 256 × 512;
//! `stadium_13_container_0` descriptor 30, DXT3 512 × 128):
//! - review → pending → copied-target review lists the archive (and on the pair both loose packages) in one change set;
//! - faults: a failure after the first output persisted and after the second both roll back to the original hashes; a failure
//!   during the rollback itself is never reported as success, and recovery then restores both originals;
//! - stale sources: a metadata-preserving change to the loose file (pair) or to the member's stored bytes, and a loose file
//!   appearing (archive-only), refuse the apply with nothing written; a diverged pair and a loose file refuse the preview;
//! - the successful apply: every declared level of both rasters is exactly the encoder's output for that level at its offset, and
//!   decodes through this file's own DXT decoder (independent of the encoder library) to the level image within BC tolerance;
//!   bytes outside the rasters are unchanged; loose == unwrapped member; every other archive member's stored bytes unchanged;
//! - reopen in a fresh registry: the new bytes are what the next plan reads; restore returns both files to their original hashes.
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use studio_api::{parse_platform, Registry};
use studio_domain::tdb_txn::{self, Event, Hooks};

const LEGACY: &str = r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL Legacy - Vanilla\PS3_GAME\USRDIR";
const NHL13: &str = r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 13\PS3_GAME\USRDIR";
const DXT1_PKG: (&str, u32) = ("stadium_321_container_0.rpsgl", 124);
const DXT3_PKG: (&str, u32) = ("stadium_13_container_0.rpsgl", 30);

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn sha_file(p: &Path) -> String {
    let mut f = std::fs::File::open(p).unwrap();
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 22];
    loop {
        let n = f.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    format!("{:x}", h.finalize())
}

/// An exclusive per-run scratch folder, removed on drop.
struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("nhlms-r67-board-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir(&d).expect("an exclusive scratch folder");
        Scratch(d)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Fails the operation at the first matching event (a simulated I/O failure there).
struct FailAt(Vec<String>);
impl Hooks for FailAt {
    fn event(&mut self, e: &Event) -> std::io::Result<()> {
        let name = format!("{e:?}");
        if let Some(i) = self.0.iter().position(|x| *x == name) {
            self.0.remove(i);
            return Err(std::io::Error::other(format!("injected failure at {name}")));
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ an independent DXT1 / DXT3 decoder (spec, not texpresso)

fn rgb565(c: u16) -> [i32; 3] {
    let (r, g, b) = ((c >> 11) & 31, (c >> 5) & 63, c & 31);
    [((r << 3) | (r >> 2)) as i32, ((g << 2) | (g >> 4)) as i32, ((b << 3) | (b >> 2)) as i32]
}
fn decode_dxt(data: &[u8], w: u32, h: u32, dxt3: bool) -> Vec<u8> {
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    let bpb = if dxt3 { 16 } else { 8 };
    let mut out = vec![0u8; (w * h * 4) as usize];
    for by in 0..bh {
        for bx in 0..bw {
            let blk = &data[((by * bw + bx) as usize) * bpb..][..bpb];
            let (alpha, col) = if dxt3 { (Some(&blk[..8]), &blk[8..16]) } else { (None, &blk[..8]) };
            let (c0, c1) = (u16::from_le_bytes([col[0], col[1]]), u16::from_le_bytes([col[2], col[3]]));
            let (p0, p1) = (rgb565(c0), rgb565(c1));
            let four = dxt3 || c0 > c1;
            let pal: [[i32; 4]; 4] = [
                [p0[0], p0[1], p0[2], 255],
                [p1[0], p1[1], p1[2], 255],
                if four { [(2 * p0[0] + p1[0]) / 3, (2 * p0[1] + p1[1]) / 3, (2 * p0[2] + p1[2]) / 3, 255] } else { [(p0[0] + p1[0]) / 2, (p0[1] + p1[1]) / 2, (p0[2] + p1[2]) / 2, 255] },
                if four { [(p0[0] + 2 * p1[0]) / 3, (p0[1] + 2 * p1[1]) / 3, (p0[2] + 2 * p1[2]) / 3, 255] } else { [0, 0, 0, 0] },
            ];
            let idx = u32::from_le_bytes([col[4], col[5], col[6], col[7]]);
            let a64 = alpha.map(|a| u64::from_le_bytes(a.try_into().unwrap()));
            for t in 0..16u32 {
                let (x, y) = (bx * 4 + t % 4, by * 4 + t / 4);
                if x >= w || y >= h {
                    continue;
                }
                let mut px = pal[((idx >> (2 * t)) & 3) as usize];
                if let Some(a) = a64 {
                    px[3] = (((a >> (4 * t)) & 15) * 17) as i32;
                }
                let o = ((y * w + x) * 4) as usize;
                for c in 0..4 {
                    out[o + c] = px[c] as u8;
                }
            }
        }
    }
    out
}

/// A BC-friendly image: horizontal red ramp, vertical green ramp, constant blue, alpha ramp along the diagonal (DXT3's alpha).
fn gradient(w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            v.extend_from_slice(&[(x * 255 / (w - 1).max(1)) as u8, (y * 255 / (h - 1).max(1)) as u8, 96, (((x + y) * 255) / (w + h - 2).max(1)) as u8]);
        }
    }
    v
}
fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) {
    std::fs::write(path, studio_domain::DecodedImage { width: w, height: h, rgba: rgba.to_vec() }.to_png().unwrap()).unwrap();
}

/// The raster `descriptor` of a package: its admitted edge.
fn edge_of(pkg: &[u8], descriptor: u32) -> tdb_rx2::venue_board::BoardEdge {
    tdb_rx2::venue_board::admit(pkg).unwrap().0.into_iter().find(|e| e.descriptor == descriptor as usize).expect("the admitted research sample")
}

/// Every declared level of `edge` in `pkg`: exactly the encoder's output of that level's image at its offset, and the independent
/// decode within BC tolerance of the level image. Returns the per-level mean absolute error.
fn check_levels(pkg: &[u8], edge: &tdb_rx2::venue_board::BoardEdge, base: &[u8]) -> Vec<f64> {
    let dxt3 = edge.format == tdb_rx2::texlib::PixelFormat::Dxt3;
    let dds = if dxt3 { tdb_archive_codec::DdsFormat::Dxt3 } else { tdb_archive_codec::DdsFormat::Dxt1 };
    // The writer encodes DXT1 opaque (no punch-through): the expected levels come from the same opaque image.
    let base: Vec<u8> = if dxt3 { base.to_vec() } else { base.chunks(4).flat_map(|p| [p[0], p[1], p[2], 255]).collect() };
    let levels = tdb_rx2::venue_board::levels_from_rgba(edge.width, edge.height, edge.levels, &base);
    let mut errs = Vec::new();
    for (l, (lw, lh, px)) in edge.chain.iter().zip(&levels) {
        let at = &pkg[edge.start + l.offset..edge.start + l.offset + l.len];
        assert_eq!(at, &tdb_archive_codec::compress_bc(dds, *lw, *lh, px)[..], "level {} is the encoder's output at its offset", l.level);
        let dec = decode_dxt(at, *lw, *lh, dxt3);
        let (mut sum, mut max) = (0u64, 0u8);
        for (i, (a, b)) in dec.iter().zip(px).enumerate() {
            if !dxt3 && i % 4 == 3 {
                continue; // DXT1 carries no alpha here
            }
            let d = a.abs_diff(*b);
            sum += d as u64;
            max = max.max(d);
        }
        let n = if dxt3 { px.len() } else { px.len() / 4 * 3 };
        let mean = sum as f64 / n as f64;
        // Large levels hold smooth ramps well inside each 4 × 4 block; at 8 texels or fewer on an axis a block spans a large part
        // of the whole ramp, and a 4-colour-per-block format is coarser by construction — the bound follows the block coverage.
        let (bm, bx) = if (*lw).min(*lh) >= 16 { (10.0, 64) } else { (24.0, 112) };
        assert!(mean <= bm && max <= bx, "level {} ({lw}×{lh}) decodes too far from its image: mean {mean:.2}, max {max}", l.level);
        errs.push(mean);
    }
    errs
}

/// Stored bytes of every member of an EB archive, by path (lower case).
fn stored_members(archive: &Path) -> Vec<(String, String)> {
    let entries = studio_domain::ps4_vault::eb_index(archive).unwrap();
    let mut f = std::fs::File::open(archive).unwrap();
    let mut out = Vec::new();
    for e in entries {
        let mut raw = vec![0u8; e.size as usize];
        f.seek(SeekFrom::Start(e.offset)).unwrap();
        f.read_exact(&mut raw).unwrap();
        out.push((e.path.to_ascii_lowercase(), sha(&raw)));
    }
    out.sort();
    out
}

struct Root {
    _scratch: Scratch,
    root: PathBuf,
    usrdir: PathBuf,
    archive: PathBuf,
}

/// An owned copied-target root: the title's archive, (pair) the two loose packages, and the title's anchor database.
fn build_root(tag: &str, src: &str, archive: &str, anchor: &str, pair: bool) -> Root {
    let scratch = Scratch::new(tag);
    // The copied-target root is the install folder itself (`USRDIR`): the archive and package paths are relative to it.
    let root = scratch.0.join("USRDIR");
    tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let usrdir = root.clone();
    std::fs::create_dir_all(usrdir.join("rendering").join("stadium")).unwrap();
    std::fs::copy(Path::new(src).join(archive), usrdir.join(archive)).expect("the title's archive (fails, never skips)");
    if pair {
        for (p, _) in [DXT1_PKG, DXT3_PKG] {
            std::fs::copy(Path::new(src).join("rendering").join("stadium").join(p), usrdir.join("rendering").join("stadium").join(p)).unwrap();
        }
    }
    let a = repo(anchor);
    std::fs::copy(a.join("nhlng.db"), usrdir.join("nhlng.db")).unwrap();
    Root { root, archive: usrdir.join(archive), usrdir, _scratch: scratch }
}

fn open(reg: &Registry, game: &str, r: &Root, anchor: &str) -> u32 {
    reg.open_workspace_from_paths(Some(game), parse_platform("ps3").unwrap(), &s(&r.usrdir.join("nhlng.db")), None, &s(&repo(anchor).join("nhlng-meta.xml")), &s(&repo("defs"))).unwrap().project_id
}

/// Preview + join both rasters; returns the image paths and the plan tickets.
fn stage_both(reg: &Registry, id: u32, r: &Root, images: &[(PathBuf, u32, u32, Vec<u8>)]) -> Vec<studio_api::dto::BoardReplacementPreviewDto> {
    let mut out = Vec::new();
    for ((pkg, desc), (img, ..)) in [DXT1_PKG, DXT3_PKG].into_iter().zip(images) {
        let p = reg.ps3_board_replacement_preview(id, &s(&r.usrdir), pkg, desc, &s(img)).unwrap();
        assert!(p.plan.is_some(), "{pkg}: {:?} {:?}", p.refusal_kind, p.refusal);
        reg.asset_member_apply_review(id, &p.plan.as_ref().unwrap().ticket).unwrap();
        out.push(p);
    }
    out
}

fn run_topology(game: &str, src: &str, archive: &str, anchor: &str, pair: bool) {
    let r = build_root(game, src, archive, anchor, pair);
    let (loose321, loose13) = (r.usrdir.join("rendering/stadium").join(DXT1_PKG.0), r.usrdir.join("rendering/stadium").join(DXT3_PKG.0));
    let member = |p: &str| format!("rendering/stadium/{p}");
    let orig_archive = sha_file(&r.archive);
    let orig_members = stored_members(&r.archive);
    let plain = |p: &str| studio_domain::read_eb_member(&r.archive, &member(p)).unwrap();
    let (pkg1, pkg3) = (plain(DXT1_PKG.0), plain(DXT3_PKG.0));
    let (e1, e3) = (edge_of(&pkg1, DXT1_PKG.1), edge_of(&pkg3, DXT3_PKG.1));
    assert_eq!((e1.format, e1.width, e1.height, e1.levels), (tdb_rx2::texlib::PixelFormat::Dxt1, 256, 512, 10));
    assert_eq!((e3.format, e3.width, e3.height, e3.levels), (tdb_rx2::texlib::PixelFormat::Dxt3, 512, 128, 10));
    let originals = |what: &str| {
        assert_eq!(sha_file(&r.archive), orig_archive, "{what}: the archive is its original");
        if pair {
            assert_eq!(std::fs::read(&loose321).unwrap(), pkg1, "{what}: loose 321 is its original");
            assert_eq!(std::fs::read(&loose13).unwrap(), pkg3, "{what}: loose 13 is its original");
        } else {
            assert!(!loose321.exists() && !loose13.exists(), "{what}: no loose file was invented");
        }
    };
    let images: Vec<(PathBuf, u32, u32, Vec<u8>)> = [(&e1, "dxt1"), (&e3, "dxt3")]
        .into_iter()
        .map(|(e, t)| {
            let px = gradient(e.width, e.height);
            let p = r.root.parent().unwrap().join(format!("{t}.png"));
            write_png(&p, e.width, e.height, &px);
            (p, e.width, e.height, px)
        })
        .collect();
    let reg = Registry::default();
    let id = open(&reg, game, &r, anchor);

    // Refusals at preview: a wrong size, DXT5-free titles only, a diverged pair or an invented loose file.
    let small = r.root.parent().unwrap().join("small.png");
    write_png(&small, 64, 64, &gradient(64, 64));
    let p = reg.ps3_board_replacement_preview(id, &s(&r.usrdir), DXT1_PKG.0, DXT1_PKG.1, &s(&small)).unwrap();
    assert_eq!(p.refusal_kind.as_deref(), Some("size"), "{:?}", p.refusal);
    let p = reg.ps3_board_replacement_preview(id, &s(&r.usrdir), DXT1_PKG.0, 1, &s(&images[0].0)).unwrap();
    assert_eq!(p.refusal_kind.as_deref(), Some("not-admitted"), "{:?}", p.refusal);
    if pair {
        let mut diverged = pkg1.clone();
        diverged[0x40] ^= 1;
        std::fs::write(&loose321, &diverged).unwrap();
        let p = reg.ps3_board_replacement_preview(id, &s(&r.usrdir), DXT1_PKG.0, DXT1_PKG.1, &s(&images[0].0)).unwrap();
        assert_eq!(p.refusal_kind.as_deref(), Some("diverged"), "{:?}", p.refusal);
        std::fs::write(&loose321, &pkg1).unwrap();
    } else {
        std::fs::write(&loose321, &pkg1).unwrap();
        let p = reg.ps3_board_replacement_preview(id, &s(&r.usrdir), DXT1_PKG.0, DXT1_PKG.1, &s(&images[0].0)).unwrap();
        assert_eq!(p.refusal_kind.as_deref(), Some("topology"), "{:?}", p.refusal);
        std::fs::remove_file(&loose321).unwrap();
    }
    originals("preview refusals");

    // The review: disclosures and one change set.
    let previews = stage_both(&reg, id, &r, &images);
    let p1 = &previews[0];
    assert_eq!(p1.topology.as_deref(), Some(if pair { "pair" } else { "archive-only" }));
    assert_eq!((p1.format.as_deref(), p1.levels), (Some("DXT1"), Some(10)));
    let mut mats = p1.materials.clone();
    mats.sort();
    assert!(["adbd_placeholder", "adbd_placeholder1_Clone_393", "adbd_sign_mult_Mat1_Clone_178"].iter().all(|m| mats.iter().any(|x| x == m)), "{mats:?}");
    assert_eq!(previews[1].materials, vec!["adbd_staples".to_string()]);
    let plan1 = p1.plan.as_ref().unwrap();
    assert_eq!(plan1.paired_loose.is_some(), pair);
    assert_eq!(plan1.loose_must_be_absent.is_some(), !pair);
    assert!(plan1.notes.iter().any(|n| n.contains("adbd_placeholder")) && plan1.notes.iter().any(|n| n.contains("not observed")), "{:?}", plan1.notes);
    assert!(plan1.notes.iter().any(|n| n.contains("encoded opaque")), "DXT1 alpha disclosure: {:?}", plan1.notes);
    assert!(!previews[1].plan.as_ref().unwrap().notes.iter().any(|n| n.contains("encoded opaque")), "DXT3 keeps its alpha");
    let pending = reg.pending_changes(id, 10).unwrap();
    assert_eq!(pending.asset_members.as_ref().map(|v| v.len()), Some(2));
    let root = s(&r.root);
    let keys = |rv: &studio_api::dto::CopiedTargetReviewDto| rv.artifacts.iter().map(|a| a.key.clone()).collect::<Vec<_>>();
    let rv = reg.copied_target_review(id, &root).unwrap();
    let expect_keys: Vec<String> = if pair { vec!["primary", "archive-asset-0", "loose-asset-0-0", "loose-asset-0-1"] } else { vec!["primary", "archive-asset-0"] }.into_iter().map(String::from).collect();
    assert_eq!(keys(&rv), expect_keys);

    // Faults while the outputs persist: after the first, and after the second (pair), both roll back to the originals.
    let firsts: Vec<&str> = if pair { vec!["archive-asset-0", "loose-asset-0-0"] } else { vec!["archive-asset-0"] };
    let mut rv = rv;
    for at in firsts {
        let e = reg.copied_target_apply_with_hooks(id, rv.handle, Some(&mut FailAt(vec![format!("AfterReplaced(\"{at}\")")]))).err().expect("refused").to_string();
        assert!(e.contains("RolledBack") || e.contains("rolled back"), "{at}: {e}");
        originals(&format!("failure after {at} persisted"));
        rv = reg.copied_target_review(id, &root).unwrap();
    }
    // A failure during the rollback itself: never success; recovery then restores the originals.
    let last = if pair { "loose-asset-0-1" } else { "archive-asset-0" };
    let e = reg
        .copied_target_apply_with_hooks(id, rv.handle, Some(&mut FailAt(vec![format!("AfterReplaced(\"{last}\")"), "BeforeRevert(\"archive-asset-0\")".into()])))
        .err()
        .expect("refused")
        .to_string();
    assert!(!e.contains("committed"), "{e}");
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.transactions.iter().all(|t| t.state != "committed"), "{:?}", st.transactions.iter().map(|t| &t.state).collect::<Vec<_>>());
    let rec = reg.copied_target_recover(&root).unwrap();
    let states: Vec<&str> = rec.status.transactions.iter().map(|t| t.state.as_str()).collect();
    assert!(states.last() == Some(&"rolled-back") && !states.contains(&"committed"), "recovery rolls it back: {states:?}");
    originals("rollback failure, then recovery");

    // Stale sources: a metadata-preserving change after review refuses with nothing written.
    let keep_mtime = |p: &Path, f: &dyn Fn(&mut Vec<u8>)| {
        let m = std::fs::metadata(p).unwrap().modified().unwrap();
        let mut b = std::fs::read(p).unwrap();
        f(&mut b);
        std::fs::write(p, &b).unwrap();
        std::fs::OpenOptions::new().write(true).open(p).unwrap().set_modified(m).unwrap();
    };
    let rv = reg.copied_target_review(id, &root).unwrap();
    if pair {
        keep_mtime(&loose13, &|b| b[0x80] ^= 1);
        let e = reg.copied_target_apply(id, rv.handle).err().expect("refused").to_string();
        assert!(e.contains("changed") || e.contains("is "), "{e}");
        keep_mtime(&loose13, &|b| b[0x80] ^= 1);
    } else {
        std::fs::write(&loose321, b"appeared").unwrap();
        let e = reg.copied_target_apply(id, rv.handle).err().expect("refused").to_string();
        assert!(e.contains("appeared after the plan was made"), "{e}");
        std::fs::remove_file(&loose321).unwrap();
    }
    originals("stale loose / appearing loose");
    // The member's stored bytes changed under the same length and time: the archive's identity no longer matches.
    let rv = reg.copied_target_review(id, &root).unwrap();
    let entry = studio_domain::ps4_vault::eb_index(&r.archive).unwrap().into_iter().find(|e| e.path.eq_ignore_ascii_case(&member(DXT3_PKG.0))).unwrap();
    let flip = |p: &Path| {
        let m = std::fs::metadata(p).unwrap().modified().unwrap();
        let mut f = std::fs::OpenOptions::new().read(true).write(true).open(p).unwrap();
        let at = entry.offset + entry.size as u64 - 1;
        let mut b = [0u8; 1];
        f.seek(SeekFrom::Start(at)).unwrap();
        f.read_exact(&mut b).unwrap();
        b[0] ^= 1;
        f.seek(SeekFrom::Start(at)).unwrap();
        std::io::Write::write_all(&mut f, &b).unwrap();
        drop(f);
        std::fs::OpenOptions::new().write(true).open(p).unwrap().set_modified(m).unwrap();
    };
    flip(&r.archive);
    let e = reg.copied_target_apply(id, rv.handle).err().expect("refused").to_string();
    assert!(!e.is_empty());
    flip(&r.archive);
    originals("stale member");

    // The successful apply.
    let rv = reg.copied_target_review(id, &root).unwrap();
    let applied = reg.copied_target_apply(id, rv.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let (new1, new3) = (plain(DXT1_PKG.0), plain(DXT3_PKG.0));
    let err1 = check_levels(&new1, &e1, &images[0].3);
    let err3 = check_levels(&new3, &e3, &images[1].3);
    for (old, new, e) in [(&pkg1, &new1, &e1), (&pkg3, &new3, &e3)] {
        assert_eq!(new.len(), old.len());
        assert!(new[..e.start] == old[..e.start] && new[e.start + e.len..] == old[e.start + e.len..], "only the raster changed");
        assert_eq!(tdb_rx2::venue_board::admit(new).unwrap().0.len(), tdb_rx2::venue_board::admit(old).unwrap().0.len(), "every edge resolves as before");
    }
    if pair {
        assert_eq!(std::fs::read(&loose321).unwrap(), new1, "loose 321 == the unwrapped member");
        assert_eq!(std::fs::read(&loose13).unwrap(), new3, "loose 13 == the unwrapped member");
    } else {
        assert!(!loose321.exists() && !loose13.exists());
    }
    let changed = [member(DXT1_PKG.0), member(DXT3_PKG.0)];
    let after = stored_members(&r.archive);
    let others_before: Vec<_> = orig_members.iter().filter(|(p, _)| !changed.contains(p)).collect();
    let others_after: Vec<_> = after.iter().filter(|(p, _)| !changed.contains(p)).collect();
    assert_eq!(others_before, others_after, "no other member's stored bytes changed");
    let new_archive = sha_file(&r.archive);

    // Reopen in a fresh registry: the next plan reads the new bytes.
    let reg2 = Registry::default();
    let id2 = open(&reg2, game, &r, anchor);
    let again = reg2.ps3_board_replacement_preview(id2, &s(&r.usrdir), DXT1_PKG.0, DXT1_PKG.1, &s(&images[0].0)).unwrap();
    assert!(again.plan.unwrap().notes.iter().any(|n| n.contains(&format!("unwrapped sha256 {}", sha(&new1)))));
    drop(reg2);

    // Restore: both files back to their original hashes.
    let restored = reg.copied_target_restore(&root, &applied.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    originals("restore");
    eprintln!(
        "{game} {}: archive {orig_archive} → {new_archive} → restored; {} other members unchanged; DXT1 level errors {:?}; DXT3 {:?}",
        if pair { "pair" } else { "archive-only" },
        others_after.len(),
        err1.iter().map(|e| format!("{e:.1}")).collect::<Vec<_>>(),
        err3.iter().map(|e| format!("{e:.1}")).collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "copies PS3 NHL Legacy's cacherender.big (1.4 GB) from X:/Games/Emulation/Roms (read only) into an owned root: select with --ignored"]
fn ps3_legacy_board_pair_replacement_through_the_copied_target_transaction() {
    run_topology("nhllegacy", LEGACY, "cacherender.big", "anchors/ps3/nhllegacy/databases", true);
}

#[test]
#[ignore = "copies PS3 NHL 13's cacherender.big (1.2 GB) from X:/Games/Emulation/Roms (read only) into an owned root: select with --ignored"]
fn ps3_nhl13_archive_only_board_replacement_through_the_copied_target_transaction() {
    run_topology("nhl13", NHL13, "cacherender.big", "anchors/ps3/nhl13/databases", false);
}

#[test]
fn other_titles_and_platforms_are_refused_by_name() {
    use studio_domain::board_replace::ps3_board_topology;
    use studio_domain::Platform;
    assert_eq!(ps3_board_topology(Some("nhllegacy"), Platform::Ps3).unwrap(), ("cacherender.big", true));
    assert_eq!(ps3_board_topology(Some("nhl11"), Platform::Ps3).unwrap(), ("stadium.big", false));
    assert_eq!(ps3_board_topology(Some("nhllegacy"), Platform::Xbox).unwrap_err().kind, "withheld");
    assert_eq!(ps3_board_topology(Some("legacyrecomp"), Platform::Recomp).unwrap_err().kind, "withheld");
    assert_eq!(ps3_board_topology(Some("nhl10"), Platform::Ps3).unwrap_err().kind, "unmeasured");
    assert_eq!(ps3_board_topology(Some("nhl21"), Platform::Ps4).unwrap_err().kind, "unsupported");
}

//! P0 of the overnight 2026-09-24 campaign — F1 of the 24 Sep coordinator review: the loose-file
//! texture write is bound to the plan the operator saw. A plan is a server-held review of the
//! target's bytes, the raster, the source's identity and dimensions and the displayed policy; the
//! write names it and refuses — before any backup or mutation — when any of those drifted.
//!
//! RED evidence (`p0-f1-red.log`): with the previous route the 512-square single-level target
//! planned "allowed, no notes", was replaced at the same path by a 10-level raster of the same
//! base size, and the same-size write **succeeded** as an undisclosed partial replacement
//! (`SHOWN []; APPLIED ["Partial replacement …"]; BYTES_CHANGED true`). Every case below is the
//! green side of that contract; nothing here is game evidence.
use std::path::{Path, PathBuf};

use base64::Engine as _;
use studio_api::Registry;
use tdb_rx2::texlib::{base_mip_byte_size, parse, LinearMip, PixelFormat};

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-review-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn flat_rgba(w: u32, h: u32, rgb: [u8; 3]) -> studio_domain::DecodedImage {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    studio_domain::DecodedImage { width: w, height: h, rgba }
}
fn flat_png(dir: &Path, name: &str, w: u32, h: u32, rgb: [u8; 3]) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, flat_rgba(w, h, rgb).to_png().unwrap()).unwrap();
    p
}
fn b64_png(w: u32, h: u32, rgb: [u8; 3]) -> String {
    base64::engine::general_purpose::STANDARD.encode(flat_rgba(w, h, rgb).to_png().unwrap())
}

/// `flat`: a 512² single-level DXT1 raster (+ a 128² second raster); `stored`: the same file with
/// raster 0 grown to a 10-level chain through the codec-level resize (the only way to a mipped
/// synthetic raster — outside the Replace flow).
fn fixtures() -> (Vec<u8>, Vec<u8>) {
    let a = vec![0u8; base_mip_byte_size(PixelFormat::Dxt1, 512, 512)];
    let b: Vec<u8> = (0..base_mip_byte_size(PixelFormat::Dxt1, 128, 128)).map(|i| 0x30 | (i as u8 & 0xF)).collect();
    let flat = tdb_rx2::texlib::encode_synthetic_texlib(&[(512, 512, PixelFormat::Dxt1, &a), (128, 128, PixelFormat::Dxt1, &b)]);
    let lib = parse(&flat).unwrap();
    let chain: Vec<(u32, u32, Vec<u8>)> = (0..10u32)
        .map(|l| {
            let (w, h) = ((512u32 >> l).max(1), (512u32 >> l).max(1));
            (w, h, vec![0x10 + l as u8; base_mip_byte_size(PixelFormat::Dxt1, w, h)])
        })
        .collect();
    let mips: Vec<LinearMip<'_>> = chain.iter().map(|(w, h, d)| LinearMip { width: *w, height: *h, linear_dxt: d }).collect();
    let stored = lib.replace_with_resize(0, PixelFormat::Dxt1, &mips).unwrap();
    assert_eq!(parse(&stored).unwrap().declared_mip_count(0), 10);
    assert_eq!(parse(&flat).unwrap().declared_mip_count(0), 1);
    (flat, stored)
}

struct Case {
    _tmp: Tmp,
    root: PathBuf,
    reg: Registry,
    flat: Vec<u8>,
    stored: Vec<u8>,
}
const REL: &str = "rendering/jersey/texlib_review.rx2";
impl Case {
    fn new(tag: &str, initial: Initial) -> Self {
        let tmp = Tmp::new(tag);
        let root = tmp.0.clone();
        let (flat, stored) = fixtures();
        let target = root.join(REL);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, match initial { Initial::Flat => &flat, Initial::Stored => &stored }).unwrap();
        Case { _tmp: tmp, root, reg: Registry::default(), flat, stored }
    }
    fn target(&self) -> PathBuf {
        self.root.join(REL)
    }
    fn bak(&self) -> PathBuf {
        PathBuf::from(format!("{}.bak", s(&self.target())))
    }
    fn untouched(&self, expected: &[u8], why: &str) {
        assert_eq!(std::fs::read(self.target()).unwrap(), expected, "{why}: the target is untouched");
        assert!(!self.bak().exists(), "{why}: no backup is written for a refused write");
    }
}
enum Initial {
    Flat,
    Stored,
}

fn refusal<T>(r: Result<T, studio_api::ApiError>, what: &str) -> String {
    match r {
        Ok(_) => panic!("{what}: must refuse, but the write succeeded"),
        Err(e) => e.to_string(),
    }
}

// ---------------------------------------------------------------------------- the reviewed defect

#[test]
fn f1_an_allowed_to_allowed_policy_change_refuses_and_leaves_target_and_backup_alone() {
    let c = Case::new("f1", Initial::Flat);
    let src = flat_png(&c.root, "same-512.png", 512, 512, [10, 200, 10]);
    let plan = c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src), Some(0)).unwrap();
    assert!(plan.checked && plan.allowed && plan.notes.is_empty(), "the single-level raster plans a full replacement: {plan:?}");
    assert_eq!((plan.source_width, plan.source_height), (512, 512));
    // The file at the same path is now the mipped raster: a same-size write would be partial.
    std::fs::write(c.target(), &c.stored).unwrap();
    let err = refusal(c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), plan.review_id), "F1");
    assert!(err.contains("changed since the plan was shown") && err.contains("Nothing was written"), "{err}");
    c.untouched(&c.stored, "F1");
    // A fresh plan shows the partial-replacement disclosure; only that plan may write.
    let fresh = c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src), Some(0)).unwrap();
    assert!(fresh.allowed && fresh.notes.len() == 1 && fresh.notes[0].contains("Partial replacement"), "{fresh:?}");
    assert_ne!(fresh.review_id, plan.review_id);
    let r = c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), fresh.review_id).expect("the freshly reviewed partial write");
    assert_eq!(r.notes, fresh.notes, "the result repeats the reviewed disclosure word for word");
    assert_eq!(std::fs::read(c.bak()).unwrap(), c.stored, "the backup is the file as reviewed");
}

#[test]
fn an_allowed_to_refused_change_refuses_from_the_review_not_only_from_the_writer() {
    // Plan a changed-size write on the single-level raster (allowed, its own note); the file
    // becomes the mipped raster (a changed size is refused there). The review's byte binding
    // refuses first, with the drift message; the earlier route relied on the writer's refusal.
    let c = Case::new("a2r", Initial::Flat);
    let src = flat_png(&c.root, "half-256.png", 256, 256, [200, 30, 30]);
    let plan = c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src), Some(0)).unwrap();
    assert!(plan.allowed && plan.notes.iter().any(|n| n.contains("declares a single level")), "{plan:?}");
    std::fs::write(c.target(), &c.stored).unwrap();
    let err = refusal(c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), plan.review_id), "allowed→refused");
    assert!(err.contains("changed since the plan was shown"), "{err}");
    c.untouched(&c.stored, "allowed→refused");
    let fresh = c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src), Some(0)).unwrap();
    assert!(!fresh.allowed && fresh.refusal.as_deref().unwrap().contains("declares 10 mip levels"), "{fresh:?}");
    // Writing with a review that is itself a refusal refuses with that reason.
    let err = refusal(c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), fresh.review_id), "reviewed refusal");
    assert!(err.contains("declares 10 mip levels"), "{err}");
    c.untouched(&c.stored, "reviewed refusal");
}

#[test]
fn a_source_replaced_at_the_same_path_with_unchanged_dimensions_refuses() {
    let c = Case::new("srcpath", Initial::Stored);
    let src = flat_png(&c.root, "edit.png", 512, 512, [10, 200, 10]);
    let plan = c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src), Some(0)).unwrap();
    assert!(plan.allowed);
    // Same path, same 512 × 512, different pixels.
    flat_png(&c.root, "edit.png", 512, 512, [200, 10, 10]);
    let err = refusal(c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), plan.review_id), "source replaced");
    assert!(err.contains("source image") && err.contains("changed since it was reviewed"), "{err}");
    c.untouched(&c.stored, "source replaced");
    // A source removed since the review refuses too (it is not re-planned behind the review).
    let plan = c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src), Some(0)).unwrap();
    std::fs::remove_file(&src).unwrap();
    let err = refusal(c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), plan.review_id), "source removed");
    assert!(err.contains("could not be read"), "{err}");
    c.untouched(&c.stored, "source removed");
}

#[test]
fn an_unchanged_plan_writes_once_and_the_consumed_review_cannot_write_again() {
    let c = Case::new("ok", Initial::Stored);
    let src = flat_png(&c.root, "edit.png", 512, 512, [10, 200, 10]);
    let plan = c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src), Some(0)).unwrap();
    assert!(plan.allowed && plan.notes.len() == 1, "{plan:?}");
    let r = c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), plan.review_id).expect("unchanged plan");
    assert_eq!(r.notes, plan.notes);
    assert_eq!((r.width, r.height), (512, 512));
    let written = std::fs::read(c.target()).unwrap();
    assert_ne!(written, c.stored, "the base changed");
    assert_eq!(written.len(), c.stored.len(), "a partial replacement never changes the length");
    assert_eq!(std::fs::read(c.bak()).unwrap(), c.stored, "the .bak is the original");
    // Repeated dispatch with the same review: consumed, refuses, nothing moves.
    let err = refusal(c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), plan.review_id), "repeat");
    assert!(err.contains("is not current"), "{err}");
    assert_eq!(std::fs::read(c.target()).unwrap(), written, "the second dispatch wrote nothing");
    assert_eq!(std::fs::read(c.bak()).unwrap(), c.stored, "the backup is kept");
    // A second, freshly reviewed write succeeds and keeps the pristine backup.
    let src2 = flat_png(&c.root, "edit2.png", 512, 512, [20, 20, 220]);
    let plan2 = c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src2), Some(0)).unwrap();
    let r2 = c.reg.import_texture(&s(&c.root), REL, &s(&src2), true, Some(0), plan2.review_id).expect("second reviewed write");
    assert_eq!(r2.backup_path, r.backup_path);
    assert_eq!(std::fs::read(c.bak()).unwrap(), c.stored, "the first .bak is still the original");
    assert_ne!(std::fs::read(c.target()).unwrap(), written);
}

#[test]
fn a_review_is_bound_to_its_target_raster_install_root_and_source() {
    let c = Case::new("sel", Initial::Stored);
    let other_rel = "rendering/jersey/texlib_other.rx2";
    std::fs::write(c.root.join(other_rel), &c.flat).unwrap();
    let other_root = Tmp::new("sel-other-root");
    std::fs::create_dir_all(other_root.0.join(REL).parent().unwrap()).unwrap();
    std::fs::write(other_root.0.join(REL), &c.stored).unwrap();
    let src = flat_png(&c.root, "edit.png", 512, 512, [10, 200, 10]);
    let other_src = flat_png(&c.root, "edit-other.png", 512, 512, [10, 200, 10]);

    let plan = || c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src), Some(0)).unwrap().review_id;
    let cases: Vec<(&str, Result<_, _>)> = vec![
        ("another target path", c.reg.import_texture(&s(&c.root), other_rel, &s(&src), true, Some(0), plan())),
        ("another raster", c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(1), plan())),
        ("the primary raster instead of raster 0", c.reg.import_texture(&s(&c.root), REL, &s(&src), true, None, plan())),
        ("another install root", c.reg.import_texture(&s(&other_root.0), REL, &s(&src), true, Some(0), plan())),
        ("another source path", c.reg.import_texture(&s(&c.root), REL, &s(&other_src), true, Some(0), plan())),
    ];
    for (what, r) in cases {
        let err = refusal(r, what);
        assert!(err.contains("Nothing was written"), "{what}: {err}");
        c.untouched(&c.stored, what);
        assert_eq!(std::fs::read(c.root.join(other_rel)).unwrap(), c.flat, "{what}: the other target is untouched");
        assert_eq!(std::fs::read(other_root.0.join(REL)).unwrap(), c.stored, "{what}: the other root is untouched");
    }
    // An unknown review, and a review evicted by the held-review bound, both refuse.
    let err = refusal(c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), 999_999), "unknown review");
    assert!(err.contains("is not current"), "{err}");
    let first = plan();
    for _ in 0..8 {
        plan();
    }
    let err = refusal(c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), first), "evicted review");
    assert!(err.contains("is not current"), "{err}");
    c.untouched(&c.stored, "unknown / evicted review");
}

#[test]
fn the_bytes_route_reviews_the_baked_image_and_refuses_drift_and_cross_route_use() {
    let c = Case::new("bytes", Initial::Stored);
    let png = b64_png(512, 512, [10, 200, 10]);
    let plan = c.reg.texture_replace_plan_bytes(&s(&c.root), REL, &png, Some(0)).unwrap();
    assert!(plan.allowed && plan.notes.len() == 1 && plan.notes[0].contains("Partial replacement"), "{plan:?}");
    assert_eq!((plan.source_width, plan.source_height), (512, 512));
    // A bytes review cannot be spent through the file route, nor a file review through the bytes route.
    let src = flat_png(&c.root, "edit.png", 512, 512, [10, 200, 10]);
    let err = refusal(c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), plan.review_id), "bytes review via file route");
    assert!(err.contains("bytes route"), "{err}");
    c.untouched(&c.stored, "cross-route");
    let file_plan = c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src), Some(0)).unwrap();
    let err = refusal(c.reg.import_texture_bytes(&s(&c.root), REL, true, Some(0), file_plan.review_id), "file review via bytes route");
    assert!(err.contains("file route"), "{err}");
    c.untouched(&c.stored, "cross-route");
    // Target drift refuses the bytes route too.
    let plan = c.reg.texture_replace_plan_bytes(&s(&c.root), REL, &png, Some(0)).unwrap();
    std::fs::write(c.target(), &c.flat).unwrap();
    let err = refusal(c.reg.import_texture_bytes(&s(&c.root), REL, true, Some(0), plan.review_id), "bytes target drift");
    assert!(err.contains("changed since the plan was shown"), "{err}");
    c.untouched(&c.flat, "bytes target drift");
    // The unchanged case writes the reviewed image (no second transfer), repeats the disclosure,
    // and the review is consumed.
    std::fs::write(c.target(), &c.stored).unwrap();
    let plan = c.reg.texture_replace_plan_bytes(&s(&c.root), REL, &png, Some(0)).unwrap();
    let r = c.reg.import_texture_bytes(&s(&c.root), REL, true, Some(0), plan.review_id).expect("reviewed bytes write");
    assert_eq!(r.notes, plan.notes);
    assert_eq!(std::fs::read(c.bak()).unwrap(), c.stored);
    let after = c.reg.texture_preview(&s(&c.root), REL, 0, Some(0)).unwrap();
    assert_eq!((after.width, after.height), (512, 512));
    let err = refusal(c.reg.import_texture_bytes(&s(&c.root), REL, true, Some(0), plan.review_id), "bytes repeat");
    assert!(err.contains("is not current"), "{err}");
    // A refused review (changed size on the mipped raster) never writes through the bytes route.
    let small = b64_png(256, 256, [200, 30, 30]);
    let plan = c.reg.texture_replace_plan_bytes(&s(&c.root), REL, &small, Some(0)).unwrap();
    assert!(!plan.allowed, "{plan:?}");
    let before = std::fs::read(c.target()).unwrap();
    let err = refusal(c.reg.import_texture_bytes(&s(&c.root), REL, true, Some(0), plan.review_id), "bytes refusal");
    assert!(err.contains("declares 10 mip levels"), "{err}");
    assert_eq!(std::fs::read(c.target()).unwrap(), before);
}

#[test]
fn a_container_without_a_pre_check_still_writes_under_its_review_and_still_refuses_target_drift() {
    // PNG (PS4 NHL 15 members) has no pre-check; GNF has carried the writer's own policy since
    // P0-1 (13 Oct 2026). Both write under their review and both refuse a target that drifted.
    let tmp = Tmp::new("unchecked");
    let root = &tmp.0;
    let reg = Registry::default();
    let gnf = tdb_rx2::gnf::encode_gnf(tdb_rx2::gnf::BcFormat::Bc1, 32, 32, &(0..32u32 * 32 / 16 * 8).map(|i| (i * 7) as u8).collect::<Vec<u8>>());
    let png = flat_rgba(32, 32, [200, 10, 10]).to_png().unwrap();
    let src = flat_png(root, "edit-64.png", 64, 64, [10, 200, 10]);
    for (rel, original, container, checked) in [("data/textures/plan.png", &png, "png", false), ("data/textures/plan.gnf", &gnf, "gnf", true)] {
        std::fs::create_dir_all(root.join(rel).parent().unwrap()).unwrap();
        std::fs::write(root.join(rel), original).unwrap();
        let plan = reg.texture_replace_plan(&s(root), rel, &s(&src), None).unwrap();
        assert_eq!(plan.container.as_deref(), Some(container));
        assert!(plan.checked == checked && plan.allowed && plan.notes.is_empty() && plan.refusal.is_none(), "{container}: a pre-check is stated, never claimed: {plan:?}");
        // The supported route is preserved: the reviewed plan writes.
        let r = reg.import_texture(&s(root), rel, &s(&src), true, None, plan.review_id).unwrap_or_else(|e| panic!("{container} write under its review: {e}"));
        assert_eq!((r.width, r.height), (64, 64));
        assert_eq!(&std::fs::read(format!("{}.bak", s(&root.join(rel)))).unwrap(), original);
        // …but it is no bypass: a target that changed since the plan refuses like any other.
        let plan = reg.texture_replace_plan(&s(root), rel, &s(&src), None).unwrap();
        std::fs::write(root.join(rel), original).unwrap();
        let err = refusal(reg.import_texture(&s(root), rel, &s(&src), true, None, plan.review_id), container);
        assert!(err.contains("changed since the plan was shown"), "{container}: {err}");
        assert_eq!(&std::fs::read(root.join(rel)).unwrap(), original);
    }
}

#[test]
fn a_plan_that_cannot_read_its_inputs_holds_no_review() {
    let tmp = Tmp::new("inputs");
    let root = &tmp.0;
    let reg = Registry::default();
    let (flat, _) = fixtures();
    std::fs::create_dir_all(root.join(REL).parent().unwrap()).unwrap();
    std::fs::write(root.join(REL), &flat).unwrap();
    let src = flat_png(root, "edit.png", 512, 512, [10, 200, 10]);
    assert!(reg.texture_replace_plan(&s(root), "../escape.rx2", &s(&src), None).is_err(), "containment");
    assert!(reg.texture_replace_plan(&s(root), "rendering/jersey/missing.rx2", &s(&src), None).is_err(), "missing target");
    assert!(reg.texture_replace_plan(&s(root), REL, &s(&root.join("missing.png")), None).is_err(), "missing source");
    assert!(reg.texture_replace_plan_bytes(&s(root), REL, "not base64!", None).is_err(), "bad base64");
    // The first review id is 1: none of the failures above allocated one.
    let plan = reg.texture_replace_plan(&s(root), REL, &s(&src), Some(0)).unwrap();
    assert_eq!(plan.review_id, 1);
}

#[test]
fn refpack_wrapped_texture_reviews_the_inner_mip_policy() {
    let c = Case::new("wrapped-mips", Initial::Stored);
    let wrapped = tdb_archive_codec::refpack_compress(&c.stored);
    std::fs::write(c.target(), &wrapped).unwrap();
    let src = flat_png(&c.root, "wrapped.png", 512, 512, [240, 10, 180]);
    let plan = c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src), Some(0)).unwrap();
    assert!(plan.checked && plan.allowed && !plan.notes.is_empty(), "wrapped mip policy must be reviewed: {plan:?}");
    let written = c.reg.import_texture(&s(&c.root), REL, &s(&src), true, Some(0), plan.review_id).unwrap();
    assert_eq!(written.notes, plan.notes);
    assert_eq!(std::fs::read(c.bak()).unwrap(), wrapped);
    let changed = std::fs::read(c.target()).unwrap();
    assert!(tdb_archive_codec::is_refpack(&changed));
    let inner = tdb_archive_codec::refpack_decompress(&changed).unwrap();
    assert_eq!(parse(&inner).unwrap().declared_mip_count(0), 10);
    assert_ne!(inner, c.stored);

    let small = flat_png(&c.root, "wrapped-small.png", 256, 256, [20, 30, 40]);
    let refused = c.reg.texture_replace_plan(&s(&c.root), REL, &s(&small), Some(0)).unwrap();
    assert!(refused.checked && !refused.allowed && refused.refusal.is_some());
    // Match the writer's single-wrapper limit instead of recursively inflating arbitrary layers.
    std::fs::write(c.target(), tdb_archive_codec::refpack_compress(&wrapped)).unwrap();
    assert!(c.reg.texture_replace_plan(&s(&c.root), REL, &s(&src), Some(0)).unwrap_err().to_string().contains("another RefPack"));
}

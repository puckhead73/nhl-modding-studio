//! P1-A (overnight 2026-09-22) — **real** PS4 `GNF` changed-dimension round trips.
//!
//! Receipt 11's GNF resize claim and receipt 16's correction were both proved on *synthetic*
//! containers; the coordinator's R1 review recorded "no real GNF resize evidence" as an explicit
//! gap. This closes it on hash-identified shipped files from two titles (NHL 21 CUSA18182 and
//! NHL 16 CUSA01909), through the codec **and** through `Registry::import_texture` — the same
//! route the Replace dialog dispatches.
//!
//! Every case works on a **disposable copy** in a temp root; the fixture bytes are read-only
//! inputs and are asserted unchanged at the end of each case. Each fixture is pinned by SHA-256,
//! so a substituted or re-extracted file fails loudly instead of silently changing what the
//! evidence covers.
//!
//! What a case asserts, per research 2026-09-21 pm handoff 03 (per-title census) and handoff 15
//! (the size rule):
//!
//! * the shipped file passes `check_resize_rule` (the "descriptor reproducible" gate) at its
//!   shipped dimensions, format and level count;
//! * after a changed-dimension write: the header re-reads at the **requested** size and level
//!   count, `pitch` / `d7` / `streamSize` follow the rule, and the file length is exactly
//!   `0x100 + chain`;
//! * the preserved fields are byte-identical — every header byte outside the five the rule
//!   rewrites, including the 16-byte tail (`USER` or zeros) and `d5` / `d6`;
//! * the pixels decode: the base mip of the rewritten file is the image that was written
//!   (bounded by the block codec's own error), and it is *not* the original image;
//! * the written file re-opens through `texture_preview` at the new size, and the `.bak` restores
//!   the original bytes exactly.
//!
//! **This is codec + route evidence only.** Whether a PS4 build loads a resized texture is card
//! R31-GNF-1 and is not claimed here. Packaging (the registry's size directory) is a separate
//! gate and is likewise not claimed.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use studio_api::Registry;
use tdb_rx2::gnf::{self, BcFormat};

/// Environment variable naming the private-fixture root (the `tdb-rx2` C0-FX1 contract).
const FIXTURE_ENV: &str = "NHL_STUDIO_FIXTURES";

/// One shipped GNF this suite resizes: fixture-relative path, its SHA-256, the title it came from
/// and the shape the census recorded for it.
struct Source {
    rel: &'static str,
    sha256: &'static str,
    title: &'static str,
    origin: &'static str,
    w: u32,
    h: u32,
    format: BcFormat,
    levels: u32,
}

/// The six pinned sources: BC1 / BC3 / BC5, single-level and mipped, on two titles. BC2, BC4, BC7
/// and BC6H are **not** covered by a real file here (BC7 is exercised synthetically in
/// `registry.rs`; BC6H has no encoder and stays refused) — recorded as a gap, not implied.
const SOURCES: &[Source] = &[
    Source { rel: "ps4-gnf/nhl21-alphaheat.gnf", sha256: "28937d7d4e018541a53c8f9842180abda101cf9d974c90a016ca13dcea1797fa", title: "NHL 21", origin: "CUSA18182 nhl/data/kettle/rendering/textures/alphaheat.gnf", w: 256, h: 256, format: BcFormat::Bc1, levels: 9 },
    Source { rel: "ps4-gnf/nhl21-customlogo_46_diff.gnf", sha256: "786bd86f9499304428e7d05cad95f31ad93757f7128d192ff76750ee0574cad2", title: "NHL 21", origin: "CUSA18182 …/rendering/customlogos/customlogo_46_diff.gnf", w: 1024, h: 1024, format: BcFormat::Bc3, levels: 11 },
    Source { rel: "ps4-gnf/nhl21-customlogo_adirondack_thunder_normal.gnf", sha256: "3795c66f4624e2510ff4b80015248120b866a58a80e8e7c27308c58e4a4663b0", title: "NHL 21", origin: "CUSA18182 …/customlogos/customlogo_adirondack_thunder_normal.gnf", w: 1024, h: 1024, format: BcFormat::Bc5, levels: 11 },
    Source { rel: "ps4-gnf/nhl21-customlogo_adirondack_thunder_recolor2.gnf", sha256: "6023b048e6c249ff85418d70fe45b451935c2527e558a83954b98eda0a7d00c3", title: "NHL 21", origin: "CUSA18182 …/customlogos/customlogo_adirondack_thunder_recolor2.gnf", w: 1024, h: 1024, format: BcFormat::Bc1, levels: 1 },
    Source { rel: "ps4-gnf/nhl16-hat_0_0_nm.gnf", sha256: "12616901eab208c2c6683a1a7fedfef423c32af89b52446c3e68e05d2ac5ecec", title: "NHL 16", origin: "CUSA01909 nhl/data/kettle/rendering/textures/hat_0_0_nm.gnf", w: 512, h: 512, format: BcFormat::Bc5, levels: 10 },
    Source { rel: "ps4-gnf/nhl16-trophy_0_0_coeff.gnf", sha256: "d5319215aff64059f0ab5dbd1b5961e155d3dec5e738d766bac67dc120257845", title: "NHL 16", origin: "CUSA01909 …/rendering/textures/trophy_0_0_coeff.gnf", w: 1024, h: 1024, format: BcFormat::Bc3, levels: 11 },
];

fn fixture_root() -> PathBuf {
    PathBuf::from(std::env::var_os(FIXTURE_ENV).unwrap_or_else(|| {
        panic!("{FIXTURE_ENV} is not set; the private PS4 GNF fixtures cannot be resolved (docs/validation/c0-fx1-explicit-fixture-tests.md)")
    }))
}

/// Read a pinned source and fail loudly on a missing file or a hash mismatch — a substituted
/// fixture must never be silently accepted as the evidence this suite claims.
fn load(src: &Source) -> Vec<u8> {
    let path = fixture_root().join(src.rel);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!("fixture `{}` ({}) is missing: {e}\n  expected at: {}\n  original: {}", src.rel, src.title, path.display(), src.origin)
    });
    let got = hex(&bytes);
    assert_eq!(got, src.sha256, "fixture `{}` is not the pinned source (expected {}, found {got}); original: {}", src.rel, src.sha256, src.origin);
    bytes
}

fn hex(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    h.finalize().iter().map(|x| format!("{x:02x}")).collect()
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Plan, then write under that review — the dialog's two-step dispatch (24 Sep 2026 review F1).
fn reviewed_import(reg: &Registry, install_dir: &str, rel: &str, src: &str, backup: bool, idx: Option<u32>) -> Result<studio_api::ImportTextureSummaryDto, studio_api::ApiError> {
    let plan = reg.texture_replace_plan(install_dir, rel, src, idx)?;
    reg.import_texture(install_dir, rel, src, backup, idx, plan.review_id)
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-gnf-real-{tag}-{}", std::process::id()));
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

/// The five header words the resize rule rewrites: `streamSize` (0x0C), the dimension word (0x18),
/// `last_level` inside 0x1C, `pitch` inside 0x20, `d7` (0x2C). Every other byte of the 256-byte
/// header must survive a resize untouched.
fn preserved_header_bytes(hdr: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(0x100);
    for (i, b) in hdr[..0x100].iter().enumerate() {
        let rewritten = matches!(i, 0x0C..=0x0F | 0x18..=0x1B | 0x1C..=0x1F | 0x20..=0x23 | 0x2C..=0x2F);
        if !rewritten {
            out.push(*b);
        }
    }
    out
}

/// A flat RGBA image of `w × h`.
fn flat(w: u32, h: u32, rgb: [u8; 3]) -> studio_domain::DecodedImage {
    let rgba = (0..(w as usize * h as usize)).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect();
    studio_domain::DecodedImage { width: w, height: h, rgba }
}

/// Mean absolute per-channel difference of two equally sized RGBA buffers.
fn mad(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(x, y)| (*x as i32 - *y as i32).abs() as f64).sum::<f64>() / a.len().max(1) as f64
}

/// The shipped headers pass the rule at the shape the census recorded, and the rule's reading of
/// dimensions / format / level count matches that census — the premise every resize below rests on.
#[test]
#[ignore = "private fixtures ps4-gnf/*.gnf (real NHL 16 / 21 members): set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_pinned_source_passes_the_rule_at_its_shipped_shape() {
    for src in SOURCES {
        let bytes = load(src);
        let got = gnf::check_resize_rule(&bytes).unwrap_or_else(|e| panic!("{}: shipped header refused: {e}", src.rel));
        assert_eq!(got, (src.w, src.h, src.format, src.levels), "{}: the rule reads a different shape than the census recorded", src.rel);
        let parsed = gnf::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", src.rel));
        assert_eq!((parsed.width, parsed.height, parsed.format), (src.w, src.h, src.format), "{}", src.rel);
        assert_eq!(parsed.tiling_index, 13, "{}", src.rel);
    }
}

/// Halve each source through the **codec** on disposable bytes: the rewritten header reads back at
/// the requested shape, every preserved byte is identical, the payload relationships hold, and the
/// file re-parses with the pixels that were written. The fixture bytes are untouched.
#[test]
#[ignore = "private fixtures ps4-gnf/*.gnf (real NHL 16 / 21 members): set NHL_STUDIO_FIXTURES and select with --ignored"]
fn halving_a_real_gnf_through_the_codec_rewrites_only_the_rule_fields() {
    for src in SOURCES {
        let original = load(src);
        let (w, h) = (src.w / 2, src.h / 2);
        let elem = src.format.block_bytes();
        // One flat level per halving the new size affords, capped by the file's own level count —
        // the same cap `replace_gnf_texture` applies.
        let levels_n = src.levels.min(32 - w.max(h).leading_zeros()).max(1);
        let data: Vec<(u32, u32, Vec<u8>)> = (0..levels_n)
            .map(|l| {
                let (mw, mh) = ((w >> l).max(1), (h >> l).max(1));
                (mw, mh, vec![0xA5u8; (mw.div_ceil(4) * mh.div_ceil(4)) as usize * elem])
            })
            .collect();
        let levels: Vec<gnf::LinearLevel<'_>> = data.iter().map(|(mw, mh, d)| gnf::LinearLevel { width: *mw, height: *mh, linear: d }).collect();

        let out = gnf::replace_resized(&original, &levels).unwrap_or_else(|e| panic!("{}: halving refused: {e}", src.rel));

        assert_eq!(gnf::check_resize_rule(&out).unwrap(), (w, h, src.format, levels_n), "{}: the written header does not read back as requested", src.rel);
        assert_eq!(out.len(), 0x100 + gnf::chain_tiled_bytes(w, h, elem, levels_n), "{}: file length is not header + chain", src.rel);
        assert_eq!(preserved_header_bytes(&out), preserved_header_bytes(&original), "{}: a header byte outside the rule's five fields changed", src.rel);
        let re = gnf::parse(&out).unwrap_or_else(|e| panic!("{}: rewritten file does not parse: {e}", src.rel));
        assert_eq!((re.width, re.height, re.format), (w, h, src.format), "{}", src.rel);
        assert_eq!(re.base_mip_linear, data[0].2, "{}: the base mip is not the level that was written", src.rel);
        // Growing back to the shipped size is equally representable.
        let back: Vec<(u32, u32, Vec<u8>)> = (0..src.levels)
            .map(|l| {
                let (mw, mh) = ((src.w >> l).max(1), (src.h >> l).max(1));
                (mw, mh, vec![0x5Au8; (mw.div_ceil(4) * mh.div_ceil(4)) as usize * elem])
            })
            .collect();
        let lv: Vec<gnf::LinearLevel<'_>> = back.iter().map(|(mw, mh, d)| gnf::LinearLevel { width: *mw, height: *mh, linear: d }).collect();
        let grown = gnf::replace_resized(&out, &lv).unwrap_or_else(|e| panic!("{}: regrow refused: {e}", src.rel));
        assert_eq!(gnf::check_resize_rule(&grown).unwrap(), (src.w, src.h, src.format, src.levels), "{}", src.rel);
        assert_eq!(grown.len(), original.len(), "{}: back at the shipped shape the file length matches the shipped file", src.rel);
        assert_eq!(preserved_header_bytes(&grown), preserved_header_bytes(&original), "{}", src.rel);
        // The rule fields, too: a round trip to the shipped shape reproduces the shipped header.
        assert_eq!(&grown[..0x100], &original[..0x100], "{}: the shipped header is reproduced exactly at the shipped shape", src.rel);

        assert_eq!(hex(&load(src)), src.sha256, "{}: the fixture was modified", src.rel);
    }
}

/// The same change of dimensions through `Registry::import_texture` — the loose-file route the
/// Replace dialog dispatches — on a disposable copy: the written file re-opens at the new size
/// through `texture_preview`, the decoded pixels are the image that was imported and not the
/// original, and the one-time `.bak` restores the shipped bytes exactly.
#[test]
#[ignore = "private fixtures ps4-gnf/*.gnf (real NHL 16 / 21 members): set NHL_STUDIO_FIXTURES and select with --ignored"]
fn a_real_gnf_changes_dimensions_through_the_public_route_and_restores_from_its_backup() {
    let tmp = Tmp::new("route");
    let reg = Registry::default();
    let root = &tmp.0;
    for (n, src) in SOURCES.iter().enumerate() {
        let original = load(src);
        let rel = format!("rendering/textures/case{n}.gnf");
        let target = root.join(&rel);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, &original).unwrap();
        let bak = PathBuf::from(format!("{}.bak", s(&target)));
        let _ = std::fs::remove_file(&bak);

        let before = reg.texture_preview(&s(root), &rel, 0, None).unwrap_or_else(|e| panic!("{}: preview: {e}", src.rel));
        assert_eq!((before.width, before.height), (src.w, src.h), "{}", src.rel);
        assert_eq!(before.container.as_deref(), Some("gnf"), "{}", src.rel);
        let original_pixels = decode_data_url(&before.data_url);

        // A visibly different image at half the shipped size.
        let (w, h) = (src.w / 2, src.h / 2);
        let edited = flat(w, h, [20, 200, 40]);
        let png = root.join(format!("edit{n}.png"));
        std::fs::write(&png, edited.to_png().unwrap()).unwrap();

        let summary = reviewed_import(&reg, &s(root), &rel, &s(&png), true, None).unwrap_or_else(|e| panic!("{}: import: {e}", src.rel));
        assert_eq!((summary.width, summary.height), (w, h), "{}", src.rel);
        assert_eq!(summary.container, "gnf", "{}", src.rel);
        assert_eq!(std::fs::read(&bak).unwrap(), original, "{}: the .bak is the shipped file", src.rel);

        let written = std::fs::read(&target).unwrap();
        assert_eq!(written.len() as u64, summary.bytes_written, "{}", src.rel);
        let (gw, gh, gf, _) = gnf::check_resize_rule(&written).unwrap_or_else(|e| panic!("{}: written file fails the rule: {e}", src.rel));
        assert_eq!((gw, gh, gf), (w, h, src.format), "{}", src.rel);
        assert_eq!(preserved_header_bytes(&written), preserved_header_bytes(&original), "{}: preserved header bytes", src.rel);

        // Re-open through the same read path the dialog uses.
        let after = reg.texture_preview(&s(root), &rel, 0, None).unwrap_or_else(|e| panic!("{}: re-preview: {e}", src.rel));
        assert_eq!((after.width, after.height), (w, h), "{}: the written file re-opens at the new size", src.rel);
        let got = decode_data_url(&after.data_url);
        // BC5 stores two channels; the decoder renders what the format can carry, so the colour
        // check is per-format: a two-channel normal map cannot reproduce blue.
        let bound = match src.format {
            BcFormat::Bc5 | BcFormat::Bc4 => 140.0,
            _ => 24.0,
        };
        let err = mad(&got.rgba, &edited.rgba);
        assert!(err <= bound, "{}: the written pixels are not the imported image (MAD {err} > {bound})", src.rel);
        // And they are not the shipped image any more (compare at a common size).
        let shrunk = image::imageops::resize(
            &image::RgbaImage::from_raw(original_pixels.width, original_pixels.height, original_pixels.rgba.clone()).unwrap(),
            w,
            h,
            image::imageops::FilterType::Triangle,
        );
        assert!(mad(&got.rgba, shrunk.as_raw()) > 10.0, "{}: the written file still looks like the shipped texture", src.rel);

        // Restore from the backup: byte-exact, and it re-opens at the shipped size.
        std::fs::copy(&bak, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), original, "{}: restore is byte-exact", src.rel);
        let restored = reg.texture_preview(&s(root), &rel, 0, None).unwrap();
        assert_eq!((restored.width, restored.height), (src.w, src.h), "{}", src.rel);

        assert_eq!(hex(&load(src)), src.sha256, "{}: the fixture was modified", src.rel);
    }
}

fn decode_data_url(url: &str) -> studio_domain::DecodedImage {
    use base64::Engine;
    let b64 = url.split_once(',').unwrap().1;
    let png = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
    studio_domain::decode_source_image_bytes(&png).unwrap()
}

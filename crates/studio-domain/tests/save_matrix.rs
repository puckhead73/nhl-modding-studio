//! Phase 4 — the **save-container & onboarding matrix**. For every committed save fixture under
//! `anchors/*/*/savefiles/`, on in-memory copies (the anchor files are never written), this proves the
//! automated half of `docs/plans/full-game-qa-editability.md` § Phase 4:
//!
//!   * **detect the container type by content, not folder name** — every classification is derived
//!     from the bytes (`SaveContainer::parse` / `is_ps4_save`), never the directory token;
//!   * **extract and validate the inner DB** where one is present;
//!   * **repack → reopen → verify**, checking the two container checksums, the preserved opaque
//!     trailer, and **deterministic** no-op behaviour (a re-pack of the unmodified inner DB round-trips
//!     the inner bytes exactly, and repacking twice is byte-identical);
//!   * **a representative edit survives the container** — a modified inner payload repacks and
//!     re-extracts byte-for-byte, and ROSTER saves additionally take a real scalar edit through the
//!     `EditSession` (the app write seam) and survive save → reopen with valid checksums;
//!   * **malformed / truncated input is refused** cleanly (an `Err`, never a panic), leaving the
//!     original bytes intact.
//!
//! Coverage spans every container variant the codec handles — gen6 `RawV3`, `RawV1` (NHL 10/11,
//! **experimental**: stale `tag_1c`), `RawV2` (NHL 12/13), `ZlibV4` (NHL 14/15/Legacy), and the PS4
//! LZ4 `DATA` framings — plus the honestly-unsupported ones (settings `v5`, multi-part dynasty/season,
//! saves with no inner TDB), which are classified and recorded read-only/blocked rather than edited.
//!
//! The per-fixture outcome is emitted deterministically to `docs/audits/data/save-matrix.csv` and
//! guarded against drift — regenerate an intended change with:
//!
//! ```text
//! QA_WRITE=1 cargo test -p studio-domain --test save_matrix
//! ```

use std::path::{Path, PathBuf};

use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole};
use tdb_core::TdbFile;
use tdb_defs::{FieldMetaIndex, FieldType};
use tdb_qa::catalog::CATALOG;
use tdb_qa::evidence::emit::{file_digest, Emitter};
use tdb_qa::evidence::model::Outcome;
use tdb_qa::evidence::CoverageKey;
use tdb_savedata::{
    is_ps4_save, ps4_extract_roster, ps4_repack_roster, ContainerVersion, SaveContainer, SaveType,
};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const CASE: &str = "studio_domain::save_matrix::save_matrix_over_all_fixtures";

/// The `save-inner-container` operations this emitter covers — contract §4.1.
///
/// * `container-detect` — the classification is derived from the bytes alone
///   (`SaveContainer::parse` / `is_ps4_save`), never the directory name.
/// * `container-extract` — `extract_db` produced a payload that parses as a coherent inner TDB.
/// * `container-repack` — identity repack preserves the inner bytes with valid checksums and is
///   deterministic, a modified payload is carried faithfully, truncated input is refused cleanly,
///   and on a ROSTER save a real `EditSession` scalar edit survives save → reopen.
///
/// **Not claimed.** `save-raw-db` — the `Codec::RawDb` branch classifies a loose database and saves
/// nothing. `save-outer-package` — no console packaging is exercised anywhere here.
/// `db-operations/scalar-edit` — the semantic roster edit runs on the *inner subset* of a save and
/// is recorded as part of what `container-repack` proves, not as main-database coverage.
///
/// **Variants are structural.** Every fixture observation carries the container codec in the key's
/// `variant` component (`v1`..`v4`, `ps4-lz4`, `raw-db`, `unsupported`) and repeats codec,
/// container version, save mode and slot as observation **facets**, so format identity is
/// machine-readable rather than prose. `catalog_universe()` enumerates variant-less cells only and
/// no accepted document declares which variants a target must cover, so a variant cell resolves to
/// `applicability = unknown` and the variant-less parent is **never** satisfied by variant
/// evidence. Declaring a required-variant catalog is an architect decision, not this emitter's.
const OPERATIONS: &[&str] = &["container-detect", "container-extract", "container-repack"];

// ----------------------------------------------------------------------------- fixture discovery

/// One save fixture: platform + game + slot dir name + the container file inside it.
struct Save {
    platform_dir: String,
    game: String,
    slot: String,
    platform: Platform,
    container: PathBuf,
}

fn platform_of(dir: &str) -> Option<Platform> {
    match dir {
        "ps3" => Some(Platform::Ps3),
        "xbox360" => Some(Platform::Xbox),
        "ps4" => Some(Platform::Ps4),
        _ => None,
    }
}

/// The payload file inside a save slot dir: `SYS-DATA` (PS3), `DATA` (PS4), else the file that shares
/// the slot dir's name (Xbox360).
fn container_file(slot_dir: &Path, platform: Platform) -> Option<PathBuf> {
    let named = |n: &str| {
        let p = slot_dir.join(n);
        p.is_file().then_some(p)
    };
    match platform {
        Platform::Ps3 => named("SYS-DATA"),
        Platform::Ps4 => named("DATA"),
        Platform::Xbox => {
            let base = slot_dir.file_name()?.to_string_lossy().into_owned();
            named(&base)
        }
        Platform::Recomp => None,
    }
}

/// Every save fixture across all anchors, stably ordered (platform → game → slot).
fn saves() -> Vec<Save> {
    let root = repo("anchors");
    let mut out = Vec::new();
    let mut platforms: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().collect();
    platforms.sort_by_key(|e| e.file_name());
    for platform in platforms.iter().filter(|e| e.path().is_dir()) {
        let pdir = platform.file_name().to_string_lossy().into_owned();
        let Some(plat) = platform_of(&pdir) else {
            continue;
        };
        let mut games: Vec<_> = std::fs::read_dir(platform.path())
            .unwrap()
            .flatten()
            .collect();
        games.sort_by_key(|e| e.file_name());
        for game in games.iter().filter(|e| e.path().is_dir()) {
            let gname = game.file_name().to_string_lossy().into_owned();
            let sf = game.path().join("savefiles");
            if !sf.is_dir() {
                continue;
            }
            let mut slots: Vec<_> = std::fs::read_dir(&sf).unwrap().flatten().collect();
            slots.sort_by_key(|e| e.file_name());
            for slot in slots.iter().filter(|e| e.path().is_dir()) {
                let sname = slot.file_name().to_string_lossy().into_owned();
                if let Some(container) = container_file(&slot.path(), plat) {
                    out.push(Save {
                        platform_dir: pdir.clone(),
                        game: gname.clone(),
                        slot: sname,
                        platform: plat,
                        container,
                    });
                }
            }
        }
    }
    let only = std::env::var("NHLMS_ONLY").ok();
    if let Some(only) = only {
        out.retain(|s| format!("{}/{}/{}", s.platform_dir, s.game, s.slot).contains(&only));
    }
    out
}

/// The metadata XML matching a title's main DB (roster inner schemas mirror it): the first that exists
/// in the game's `databases/`, with the byte-identical xbox360 sibling as a fallback for PS3 titles
/// that ship none.
fn meta_for(platform_dir: &str, game: &str) -> Option<PathBuf> {
    let names = ["nhlng-meta.xml", "nhl_game-meta.xml", "NHL07NG-meta.xml"];
    let dbdir = repo(&format!("anchors/{platform_dir}/{game}/databases"));
    for n in names {
        let p = dbdir.join(n);
        if p.is_file() {
            return Some(p);
        }
    }
    if platform_dir == "ps3" {
        let sib = repo(&format!("anchors/xbox360/{game}/databases"));
        for n in names {
            let p = sib.join(n);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

// ----------------------------------------------------------------------------- classification

#[derive(Clone, Copy, PartialEq)]
enum Codec {
    /// A PS3/Xbox360 `SaveContainer` of the given version.
    Container(ContainerVersion),
    /// A PS4 `DATA` roster save, tagged with its observed compression.
    Ps4Roster(&'static str),
    /// A raw TDB file (no container wrapper) — not expected among saves, handled for completeness.
    RawDb,
    /// Bytes that classify to no supported save shape (e.g. settings `v5`, non-roster PS4 `DATA`).
    Unsupported,
}

fn version_str(v: ContainerVersion) -> &'static str {
    match v {
        ContainerVersion::RawV3 => "v3",
        ContainerVersion::RawV1 => "v1",
        ContainerVersion::RawV2 => "v2",
        ContainerVersion::ZlibV4 => "v4",
    }
}

/// The codec name recorded in an observation's detail — the container variant is carried here
/// rather than in the coverage key (see [`OPERATIONS`]).
fn codec_label(c: Codec) -> &'static str {
    match c {
        Codec::Container(v) => version_str(v),
        Codec::Ps4Roster(label) => label,
        Codec::RawDb => "raw-db",
        Codec::Unsupported => "unsupported",
    }
}

fn save_type_str(t: SaveType) -> &'static str {
    match t {
        SaveType::Settings => "settings",
        SaveType::BeAGm => "beagm",
        SaveType::Roster => "roster",
        SaveType::BeAPro => "beapro",
        SaveType::HockeyCard => "hockeycard",
        SaveType::Season => "season",
        SaveType::Unknown => "unknown",
    }
}

/// Classify **from bytes only** — the plan's "detect type by content, not folder name".
fn classify(bytes: &[u8]) -> (Codec, Option<SaveType>) {
    if TdbFile::parse(bytes).is_ok() {
        return (Codec::RawDb, None);
    }
    if let Ok(c) = SaveContainer::parse(bytes) {
        return (Codec::Container(c.header.version), Some(c.header.save_type));
    }
    if is_ps4_save(bytes) {
        return (Codec::Ps4Roster(tdb_savedata::ps4::compression_label(bytes).unwrap()), Some(SaveType::Roster));
    }
    (Codec::Unsupported, None)
}

// ----------------------------------------------------------------------------- inner-DB edit pick

/// The first editable, bounded, non-key integer field present in `db` (studio ids), for the
/// representative scalar edit on a roster save. `None` if the schema exposes none.
fn pick_editable_int(db: &TdbFile, meta: &FieldMetaIndex) -> Option<(String, String, i64, i64)> {
    let mut tables: Vec<&tdb_core::Table> =
        db.tables.iter().filter(|t| t.current_records > 0).collect();
    tables.sort_by_key(|t| t.id.canonical(db.endian));
    for t in tables {
        let traw = t.id.raw();
        let mut fields: Vec<&tdb_core::FieldDescriptor> = t.fields.iter().collect();
        fields.sort_by_key(|f| f.id.canonical(db.endian));
        for f in fields {
            let Some(fm) = meta.field(&traw, &f.id.raw()) else {
                continue;
            };
            if fm.key || !fm.updatable || fm.kind != FieldType::Integer {
                continue;
            }
            match (fm.range_low, fm.range_high) {
                (Some(lo), Some(hi)) if hi > lo && lo >= 0 => {
                    return Some((t.id.canonical(db.endian), f.id.canonical(db.endian), lo, hi));
                }
                _ => {}
            }
        }
    }
    None
}

// ----------------------------------------------------------------------------- proofs

/// **Extraction only.** A payload comes out of the container and parses as a coherent inner TDB,
/// or the codec declines to produce one.
///
/// Split out of the old combined round-trip so that a failure lands on the operation whose
/// assertions failed: everything here is `container-extract`, everything in
/// [`container_repack_proof`] is `container-repack`. `None` is a clean decline — a multi-part
/// layout or a non-DB payload — and it is **not** evidence that extraction or repacking works.
fn container_extract_proof(bytes: &[u8], tag: &str) -> Option<Vec<u8>> {
    let c = SaveContainer::parse(bytes).unwrap_or_else(|e| panic!("{tag}: re-parse: {e}"));
    let inner = c.extract_db().ok()?; // e.g. multi-part layout whose first segment isn't a whole body
    if TdbFile::parse(&inner).is_err() {
        return None; // extracted, but not a coherent inner TDB (multi-part / non-DB payload)
    }
    Some(inner)
}

/// **Repack only**, over an inner payload [`container_extract_proof`] already produced. Assertions
/// are unchanged from the combined version this replaces.
fn container_repack_proof(bytes: &[u8], inner: &[u8], tag: &str) {
    let c = SaveContainer::parse(bytes).unwrap_or_else(|e| panic!("{tag}: re-parse: {e}"));
    let inner = inner.to_vec();

    // Identity repack: inner survives, checksums verify, deterministic, opaque trailer preserved.
    let repacked = c
        .repack(&inner)
        .unwrap_or_else(|e| panic!("{tag}: identity repack: {e}"));
    let rc =
        SaveContainer::parse(&repacked).unwrap_or_else(|e| panic!("{tag}: repacked re-parse: {e}"));
    assert!(
        rc.checksums_valid(),
        "{tag}: identity repack checksums invalid"
    );
    assert_eq!(
        rc.extract_db().unwrap(),
        inner,
        "{tag}: inner DB not preserved by identity repack"
    );
    assert_eq!(
        c.repack(&inner).unwrap(),
        repacked,
        "{tag}: repack is not deterministic"
    );

    // Modified payload: flip one interior byte of the inner DB → repack → re-extract must reproduce it
    // exactly, with valid checksums. Proves the codec faithfully carries an edited body of every type.
    let mut edited = inner.clone();
    let mid = edited.len() / 2;
    edited[mid] ^= 0xFF;
    let rp = c
        .repack(&edited)
        .unwrap_or_else(|e| panic!("{tag}: modified repack: {e}"));
    let rpc = SaveContainer::parse(&rp).unwrap();
    assert!(
        rpc.checksums_valid(),
        "{tag}: modified repack checksums invalid"
    );
    assert_eq!(
        rpc.extract_db().unwrap(),
        edited,
        "{tag}: modified inner DB not carried faithfully"
    );
}

/// **PS4 extraction only.** `None` when the `DATA` record is not a roster TDB — a BeAPro, Profile
/// or BeAGM payload that `is_ps4_save` matches but that carries no database.
fn ps4_extract_proof(bytes: &[u8]) -> Option<Vec<u8>> {
    let inner = ps4_extract_roster(bytes).ok()?;
    TdbFile::parse(&inner).ok()?;
    Some(inner)
}

/// **PS4 repack only** (inner-DB identity + modified fidelity + deterministic).
fn ps4_repack_proof(bytes: &[u8], inner: &[u8], tag: &str) {
    let inner = inner.to_vec();

    let repacked =
        ps4_repack_roster(bytes, &inner).unwrap_or_else(|e| panic!("{tag}: repack: {e}"));
    assert_eq!(
        ps4_extract_roster(&repacked).unwrap(),
        inner,
        "{tag}: inner not preserved by repack"
    );
    assert_eq!(
        ps4_repack_roster(bytes, &inner).unwrap(),
        repacked,
        "{tag}: PS4 repack not deterministic"
    );

    let mut edited = inner.clone();
    let mid = edited.len() / 2;
    edited[mid] ^= 0xFF;
    let rp =
        ps4_repack_roster(bytes, &edited).unwrap_or_else(|e| panic!("{tag}: modified repack: {e}"));
    assert_eq!(
        ps4_extract_roster(&rp).unwrap(),
        edited,
        "{tag}: modified inner not carried faithfully"
    );
}

/// A real scalar edit through the app write seam on a ROSTER save: open via `Project::load`, stage a
/// bounded field change with `EditSession::set_field`, save (repack), reopen the container, and confirm
/// the edit survives in the inner DB with valid checksums. Returns `true` if an edit was exercised.
fn semantic_roster_edit(save: &Save, tag: &str) -> bool {
    let Some(meta_path) = meta_for(&save.platform_dir, &save.game) else {
        return false;
    };
    let project = match Project::load(
        &save.container,
        save.platform,
        ProjectRole::Roster,
        &meta_path,
        repo("defs"),
    ) {
        Ok(p) => p,
        Err(_) => return false, // schema/meta mismatch on this subset — codec round-trip still proven
    };
    let mut sess = EditSession::begin(project);

    // No-op save first: the inner DB must be byte-identical to the freshly-extracted baseline.
    let baseline_inner = sess.bytes().to_vec();
    let saved0 = sess
        .save()
        .unwrap_or_else(|e| panic!("{tag}: no-op save: {e}"));
    if save.platform == Platform::Ps4 {
        assert_eq!(ps4_extract_roster(&saved0).unwrap(), ps4_extract_roster(&std::fs::read(&save.container).unwrap()).unwrap(), "{tag}: no-op payload/trailer changed");
    }
    assert_eq!(
        reopen_inner(save.platform, &saved0),
        baseline_inner,
        "{tag}: no-op save changed the inner DB"
    );

    // Pick an editable field on the inner roster subset.
    let db = TdbFile::parse(&baseline_inner).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
    let Some((table, field, lo, hi)) = pick_editable_int(&db, &meta) else {
        return false;
    };
    let base_raw = {
        let t = db.table_canonical(&table).unwrap();
        let f = t
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == field)
            .unwrap();
        db.read_u64(t, 0, f).unwrap() as i64
    };
    let target = if base_raw == hi { lo } else { hi };
    drop(db);

    if sess
        .set_field(&table, 0, &field, FieldValue::Int(target))
        .is_err()
    {
        return false; // field not writable on this subset — codec round-trip already proven
    }

    // Save → reopen the container → the edit is present in the inner DB, checksums verify.
    let saved = sess
        .save()
        .unwrap_or_else(|e| panic!("{tag}: edited save: {e}"));
    if save.platform == Platform::Ps4 {
        let before = ps4_extract_roster(&std::fs::read(&save.container).unwrap()).unwrap();
        let after = ps4_extract_roster(&saved).unwrap();
        let declared = TdbFile::parse(&before).unwrap().db_size as usize;
        assert_eq!(&before[declared..], &after[declared..], "{tag}: edit changed opaque trailer");
        let db = TdbFile::parse(&after[..declared]).unwrap();
        assert_eq!(db.serialize(), &after[..declared], "{tag}: edited TDB CRCs do not verify");
    }
    if save.platform != Platform::Ps4 {
        assert!(
            SaveContainer::parse(&saved).unwrap().checksums_valid(),
            "{tag}: edited container checksums invalid"
        );
    }
    let inner = reopen_inner(save.platform, &saved);
    let db2 =
        TdbFile::parse(&inner).unwrap_or_else(|e| panic!("{tag}: edited inner re-parse: {e}"));
    assert!(db2.header_crc_ok(), "{tag}: edited inner DB failed CRC");
    let t = db2.table_canonical(&table).unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db2.endian) == field)
        .unwrap();
    assert_eq!(
        db2.read_u64(t, 0, f).unwrap() as i64,
        target,
        "{tag}: edit did not survive the container"
    );
    true
}

fn reopen_inner(platform: Platform, saved: &[u8]) -> Vec<u8> {
    match platform {
        Platform::Ps4 => {
            let mut inner = ps4_extract_roster(saved).unwrap();
            let declared = TdbFile::parse(&inner).unwrap().db_size as usize;
            inner.truncate(declared);
            inner
        },
        _ => SaveContainer::parse(saved).unwrap().extract_db().unwrap(),
    }
}

/// Truncated / malformed input is refused cleanly (no panic), never silently yielding a usable inner
/// DB, and the original bytes are untouched (we only ever read copies). A "usable" inner DB is one that
/// parses *and* passes its size/CRC self-check — a truncated body must fail that.
///
/// The cut points are **header-relative**, not fractions of the file: a `ZlibV4` save zero-pads the
/// compressed body up to the original allocation, so lopping off the file's back half removes only
/// padding — a genuine truncation must instead remove the header or bite into the (small) body.
fn assert_malformed_rejected(bytes: &[u8], codec: Codec, platform: Platform, tag: &str) {
    let before = bytes.to_vec();
    let complete_tdb = |inner: &[u8]| {
        TdbFile::parse(inner)
            .map(|db| db.size_ok())
            .unwrap_or(false)
    };

    let mut cuts = vec![8usize]; // below any container header → TooShort
    match codec {
        Codec::Container(v) => {
            let h = v.header_len();
            cuts.push(h); // header only, empty body
            cuts.push(h + 16); // a sliver of body — an incomplete TDB / truncated zlib stream
        }
        Codec::Ps4Roster(_) => {
            cuts.push(64);
            cuts.push(256);
        }
        _ => {}
    }

    for cut in cuts {
        if cut >= bytes.len() {
            continue;
        }
        let trunc = &bytes[..cut];
        let container_tdb = SaveContainer::parse(trunc)
            .ok()
            .and_then(|c| c.extract_db().ok())
            .map(|inner| complete_tdb(&inner))
            .unwrap_or(false);
        let ps4_tdb = matches!(platform, Platform::Ps4)
            && ps4_extract_roster(trunc)
                .ok()
                .map(|i| complete_tdb(&i))
                .unwrap_or(false);
        assert!(
            !container_tdb && !ps4_tdb,
            "{tag}: a truncated container ({cut} bytes) still yielded a complete inner DB"
        );
    }
    assert_eq!(
        bytes,
        &before[..],
        "{tag}: malformed probing mutated the source bytes"
    );
}

// ----------------------------------------------------------------------------- CSV

fn csv_cell(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// ----------------------------------------------------------------------------- the test

#[test]
fn save_matrix_over_all_fixtures() {
    let mut csv =
        String::from("platform,game,slot,detected_type,codec,inner_tdb,semantic_edit,status\n");
    let mut n_roundtrip = 0;
    let mut n_semantic = 0;
    let mut n_readonly = 0;
    let mut n_checked = 0;

    let mut ev = Emitter::from_env("save_matrix", "save-containers", &repo_root());
    // Declared before anything runs, over the whole catalog × the three covered operations.
    let declared: Vec<(String, Vec<(&'static str, CoverageKey)>)> = CATALOG
        .iter()
        .map(|unit| {
            let target = format!("{}-{}", unit.game, unit.platform);
            let keys = OPERATIONS
                .iter()
                .map(|op| {
                    (
                        *op,
                        ev.declare(&format!("{target}/save-inner-container/{op}")),
                    )
                })
                .collect();
            (target, keys)
        })
        .collect();
    let mut covered: Vec<String> = Vec::new();

    for save in saves() {
        let tag = format!("{}/{}/{}", save.platform_dir, save.game, save.slot);
        let target = format!("{}-{}", save.game, save.platform_dir);
        let bytes = std::fs::read(&save.container).unwrap_or_else(|e| panic!("{tag}: read: {e}"));
        let fixture = file_digest(&save.container).unwrap_or_default();
        let in_catalog = declared.iter().any(|(t, _)| *t == target);
        if in_catalog {
            // Attribute the save fixture on the run (C0-EMIT1-R2): repository-relative path plus
            // the captured content hash, so a repacked or replaced fixture stales this run.
            ev.input_digest(
                &save
                    .container
                    .strip_prefix(repo(""))
                    .unwrap_or(&save.container)
                    .to_string_lossy()
                    .replace('\\', "/"),
                (!fixture.is_empty()).then(|| fixture.clone()),
            );
        }
        if in_catalog && !covered.contains(&target) {
            covered.push(target.clone());
        }

        // (1) Detection is content-derived, and a truncated container of a recognised codec is
        //     refused cleanly. Both are asserted before anything is recorded.
        //
        // Classification has to run before the key is known, because the key *carries* the codec it
        // finds. A panic in here therefore has no operation-specific key to attach to and is caught
        // by the emitter's drop guard instead; everything after this point is attributed to the
        // exact operation whose assertions ran.
        let (codec, save_type) = {
            let (codec, save_type) = classify(&bytes);
            if !matches!(codec, Codec::Unsupported) {
                assert_malformed_rejected(&bytes, codec, save.platform, &tag);
            }
            (codec, save_type)
        };

        // Format and mode identity, recorded **structurally** rather than only in prose: the codec
        // becomes the key's `variant` component, and codec, container version, save mode and slot
        // are attached to every observation as facets.
        let variant = codec_label(codec);
        let mode = save_type.map(save_type_str).unwrap_or("unknown");
        let container_version = match codec {
            Codec::Container(v) => version_str(v),
            _ => "",
        };
        let facets: Vec<(&str, &str)> = vec![
            ("codec", variant),
            ("container-version", container_version),
            ("save-mode", mode),
            ("slot", save.slot.as_str()),
        ];
        let variant_key = |ev: &mut Emitter, op: &str| -> Option<CoverageKey> {
            in_catalog.then(|| ev.declare(&format!("{target}/save-inner-container/{op}/{variant}")))
        };

        let detect_case = format!("{CASE}#{}-detect", save.slot);
        if let Some(key) = variant_key(&mut ev, "container-detect") {
            let (outcome, detail) = match codec {
                Codec::Unsupported => (
                    Outcome::RefusedClean,
                    "bytes classify to no supported save shape; refused, not guessed".to_string(),
                ),
                Codec::RawDb => (
                    Outcome::RefusedClean,
                    "classified as a loose database, not a save container".to_string(),
                ),
                c => (
                    Outcome::Pass,
                    format!(
                        "detected {} from bytes alone; truncated input refused cleanly",
                        codec_label(c)
                    ),
                ),
            };
            ev.observe_faceted(
                &key,
                &detect_case,
                outcome,
                &fixture,
                "",
                &format!("{tag}: {detail}"),
                &facets,
            );
        }

        // (2) Extraction, on its own case and its own key. A failure here attaches to
        //     `container-extract`; a clean decline is a refusal, and is **not** evidence that
        //     extraction — let alone repacking — works.
        let extract_case = format!("{CASE}#{}-extract", save.slot);
        let inner: Option<Vec<u8>> = match codec {
            Codec::Container(_) | Codec::Ps4Roster(_) => {
                let extract = |bytes: &[u8]| match codec {
                    Codec::Ps4Roster(_) => ps4_extract_proof(bytes),
                    _ => container_extract_proof(bytes, &tag),
                };
                match variant_key(&mut ev, "container-extract") {
                    Some(key) => {
                        let out = ev.case(&key, &extract_case, || extract(&bytes));
                        let (outcome, detail) = match &out {
                            Some(_) => (
                                Outcome::Pass,
                                "extracted a payload that parses as a coherent inner TDB",
                            ),
                            None => (
                                Outcome::RefusedClean,
                                "no coherent inner TDB (multi-part or non-DB payload); declined \
                                 without a panic and without a partial result",
                            ),
                        };
                        ev.observe_faceted(
                            &key,
                            &extract_case,
                            outcome,
                            &fixture,
                            "",
                            &format!("{tag}: {detail}"),
                            &facets,
                        );
                        out
                    }
                    None => extract(&bytes),
                }
            }
            Codec::RawDb | Codec::Unsupported => {
                if let Some(key) = variant_key(&mut ev, "container-extract") {
                    ev.observe_faceted(
                        &key,
                        &extract_case,
                        Outcome::NotAttempted,
                        &fixture,
                        "",
                        &format!(
                            "{tag}: not a container payload ({variant}); no extraction was \
                             attempted, and refusing the input proves nothing about extraction"
                        ),
                        &facets,
                    );
                }
                None
            }
        };

        // (3) Repack, on its own case and its own key, over the payload extraction produced.
        let repack_case = format!("{CASE}#{}-repack", save.slot);
        let is_roster = matches!(codec, Codec::Ps4Roster(_)) || save_type == Some(SaveType::Roster);
        let mut edited = false;
        match &inner {
            Some(inner_bytes) => {
                let repack = |bytes: &[u8]| {
                    match codec {
                        Codec::Ps4Roster(_) => ps4_repack_proof(bytes, inner_bytes, &tag),
                        _ => container_repack_proof(bytes, inner_bytes, &tag),
                    }
                    if is_roster {
                        semantic_roster_edit(&save, &tag)
                    } else {
                        false
                    }
                };
                edited = match variant_key(&mut ev, "container-repack") {
                    Some(key) => {
                        let e = ev.case(&key, &repack_case, || repack(&bytes));
                        ev.observe_faceted(
                            &key,
                            &repack_case,
                            Outcome::Pass,
                            &fixture,
                            "",
                            &format!(
                                "{tag}: identity repack preserves the inner DB with valid \
                                 checksums and is deterministic; a modified payload is carried \
                                 faithfully{}",
                                if e {
                                    "; an EditSession scalar edit survives save -> reopen"
                                } else {
                                    ""
                                }
                            ),
                            &facets,
                        );
                        e
                    }
                    None => repack(&bytes),
                };
            }
            None => {
                if let Some(key) = variant_key(&mut ev, "container-repack") {
                    ev.observe_faceted(
                        &key,
                        &repack_case,
                        Outcome::NotAttempted,
                        &fixture,
                        "",
                        &format!(
                            "{tag}: extraction produced no inner payload ({variant}), so no repack \
                             was attempted on this fixture"
                        ),
                        &facets,
                    );
                }
            }
        }

        let has_inner = inner.is_some();
        let (type_str, codec_str, inner_tdb, semantic, status): (&str, &str, &str, &str, &str) =
            match codec {
                Codec::Container(v) => {
                    if !has_inner {
                        n_readonly += 1;
                        (mode, version_str(v), "no", "no", "read-only")
                    } else {
                        n_roundtrip += 1;
                        if save_type == Some(SaveType::Roster) {
                            if edited {
                                n_semantic += 1;
                            }
                            let status = if v == ContainerVersion::RawV1 {
                                "experimental-roundtrip"
                            } else {
                                "edit-roundtrip"
                            };
                            (
                                mode,
                                version_str(v),
                                "yes",
                                if edited { "yes" } else { "no" },
                                status,
                            )
                        } else {
                            // Non-roster inner TDB: codec fidelity proven; not a roster-editor target.
                            (mode, version_str(v), "yes", "no", "roundtrip")
                        }
                    }
                }
                Codec::Ps4Roster(_) => {
                    // `is_ps4_save` matches a PS4 DATA with a supported compressed record, including
                    // BeAPro / Profile / BeAGM saves whose payload is not a roster TDB. Only when
                    // the extracted payload actually parses as a TDB is it a roster.
                    if has_inner {
                        if edited {
                            n_semantic += 1;
                        }
                        n_roundtrip += 1;
                        (
                            "roster",
                            codec_label(codec),
                            "yes",
                            if edited { "yes" } else { "no" },
                            "edit-roundtrip",
                        )
                    } else {
                        n_readonly += 1;
                        ("non-roster", codec_label(codec), "no", "no", "read-only")
                    }
                }
                Codec::RawDb => {
                    n_readonly += 1;
                    ("raw", "raw-db", "yes", "no", "read-only")
                }
                Codec::Unsupported => {
                    n_readonly += 1;
                    (mode, "unsupported", "no", "no", "read-only")
                }
            };

        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            csv_cell(&save.platform_dir),
            csv_cell(&save.game),
            csv_cell(&save.slot),
            csv_cell(type_str),
            csv_cell(codec_str),
            csv_cell(inner_tdb),
            csv_cell(semantic),
            csv_cell(status),
        ));
        n_checked += 1;
    }

    // The variant-less parent cells stay conservative on purpose. Every fixture's evidence is
    // recorded against a variant-bearing key, and no accepted document declares which variants a
    // target must cover — so a parent cell is never satisfied by variant evidence, and this emitter
    // says exactly that instead of leaving the reader to infer it from an absence.
    for (target, keys) in &declared {
        let (outcome, detail) = if covered.contains(target) {
            (
                Outcome::NotAttempted,
                "fixtures exist and were exercised, but every observation is recorded against a                  variant-bearing key; no required-variant catalog exists, so this variant-less                  cell is deliberately left unsatisfied",
            )
        } else {
            (
                Outcome::SkippedMissingFixture,
                "no save fixture for this unit; nothing was executed",
            )
        };
        for (_, key) in keys {
            ev.observe(key, &format!("{CASE}#{target}"), outcome, detail);
        }
    }

    assert!(
        n_checked > 100,
        "expected the full save-fixture set, checked {n_checked}"
    );
    assert!(n_roundtrip > 0, "no container round-trip was proven");
    assert!(
        n_semantic > 0,
        "no ROSTER save took a real edit through the app seam"
    );
    let _ = n_readonly;

    let path = repo("docs/audits/data/save-matrix.csv");
    if std::env::var("QA_WRITE").is_ok() {
        std::fs::write(&path, &csv).unwrap();
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    // Record-level, not byte-level: the generator writes LF and a Windows checkout materialises
    // the committed file CRLF, so byte equality failed from a clean checkout while passing in a
    // working tree (C0-EVID-PROBE1 §4.2). `compare_records` treats the two legal record
    // terminators as equivalent and nothing else — a CR inside a quoted field stays payload, and
    // every value, field, row order and row count is still compared verbatim.
    if let Err(drift) = tdb_qa::evidence::csv::compare_records(&csv, &committed) {
        // Deliberately NOT `ev.finish()`: the drop guard must seal this run `failed`, exactly as
        // the previous `assert_eq!` left it. A drift failure never yields a selectable run.
        panic!(
            "save-matrix.csv drift — regenerate with `QA_WRITE=1 cargo test -p studio-domain --test save_matrix`\n{drift}"
        );
    }
    ev.finish();
}

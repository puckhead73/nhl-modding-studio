//! Receipt 66 (review F2) — a PS4 reassignment is checked at Apply against the attached source **as it is now**, never a
//! cached catalogue's old evidence. Runs through the public Registry on an owned scratch copy of NHL 21's `boot.big` alone
//! (not the install) and a copied identity-definitions root; the genuine install and the repository `defs` are only read.
//!
//! Covered: a warm cache and review; a vault member changed with the archive's length and modification time preserved
//! (Apply refused, player bytes and pending state unchanged — heads and hair); an unchanged source reusing the cache (same
//! fingerprint, exact member hashes in the plan); a timestamp-changing replacement with the same bytes (accepted: identity is
//! content, not metadata) and with another title's bytes (refused); a changed identity-definition file (refused, including an
//! edit to unrelated text: the granularity is the whole file) and a changed defs root with identical content (accepted).
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use studio_api::dto::{AssetRootsArgsDto, PlayerReassignPlanDto};
use studio_api::{parse_platform, Registry};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// An exclusive per-run scratch directory: created new (never reused, never pre-existing), removed on drop only after its
/// resolved path is checked to still be the one created, directly under the temp root and not a reparse point.
struct Scratch {
    dir: PathBuf,
    root: PathBuf,
}
impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::fs::canonicalize(std::env::temp_dir()).unwrap();
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = root.join(format!("nhlms-r66-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir(&dir).expect("an exclusive scratch directory (create_dir refuses an existing one)");
        Scratch { dir, root }
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let Ok(meta) = std::fs::symlink_metadata(&self.dir) else { return };
        let resolved = std::fs::canonicalize(&self.dir).ok();
        if meta.file_type().is_symlink() || resolved.as_deref().and_then(Path::parent) != Some(self.root.as_path()) {
            eprintln!("scratch {} not removed: containment check failed", self.dir.display());
            return;
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                eprintln!("scratch {} not removed: reparse point", self.dir.display());
                return;
            }
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

const NHL21: &str = r"H:\Emulators\ShadPS4\Games\CUSA18182";
const NHL20: &str = r"H:\Emulators\ShadPS4\Games\CUSA15439";

fn sha(p: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(p).unwrap()))
}

/// Overwrite `boot` with `bytes`, keeping its modification time (the review F2 collision: same length, same time).
fn write_keep_mtime(boot: &Path, bytes: &[u8]) {
    let before = std::fs::metadata(boot).unwrap();
    std::fs::write(boot, bytes).unwrap();
    let f = std::fs::OpenOptions::new().write(true).open(boot).unwrap();
    f.set_modified(before.modified().unwrap()).unwrap();
    drop(f);
    let after = std::fs::metadata(boot).unwrap();
    assert_eq!((after.len(), after.modified().unwrap()), (before.len(), before.modified().unwrap()), "length and time preserved");
}

/// Check (`apply` None) or apply the reviewed fingerprint.
fn call(reg: &Registry, id: u32, args: &AssetRootsArgsDto, defs: &Path, kind: &str, rec: u32, target: i64, apply: Option<&str>) -> Result<PlayerReassignPlanDto, String> {
    reg.player_reassign(id, args, rec, kind, target, apply.is_some(), apply.filter(|f| !f.is_empty()), defs).map_err(|e| e.to_string())
}

#[test]
#[ignore = "genuine NHL 21 / NHL 20 PS4 installs under H:/Emulators/ShadPS4/Games (read only; one boot.big copied to owned scratch): select with --ignored"]
fn apply_revalidates_the_attached_vault_and_identity_definitions() {
    let sc = Scratch::new("vault");
    let install = sc.dir.join("install");
    std::fs::create_dir_all(install.join("databigs")).unwrap();
    let boot = install.join("databigs").join("boot.big");
    let genuine = std::fs::read(Path::new(NHL21).join("databigs").join("boot.big")).expect("NHL 21 boot.big (fails, never skips)");
    std::fs::write(&boot, &genuine).unwrap();
    let defs = sc.dir.join("defs");
    std::fs::create_dir_all(defs.join("ps4")).unwrap();
    let ids_file = defs.join("ps4").join("vault-identities.json");
    let ids_text = std::fs::read_to_string(repo("defs/ps4/vault-identities.json")).unwrap();
    std::fs::write(&ids_file, &ids_text).unwrap();
    // A disposable copy of the NHL 21 anchor database.
    let anchor = repo("anchors/ps4/nhl21/databases");
    let (db, meta) = (sc.dir.join("nhl_game.db"), sc.dir.join("nhl_game-meta.xml"));
    std::fs::copy(anchor.join("nhl_game.db"), &db).unwrap();
    std::fs::copy(anchor.join("nhl_game-meta.xml"), &meta).unwrap();
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhl21"), parse_platform("ps4").unwrap(), &s(&db), None, &s(&meta), &s(&repo("defs"))).unwrap().project_id;
    let args = AssetRootsArgsDto { install_dir: s(&install), platform: Some("ps4".into()), rendering_override: None, fe_override: None };

    // Subject and targets, from the scratch copy's own catalogues.
    let heads = studio_domain::ps4_heads::HeadCatalogue::load(&install, &defs, "nhl21").unwrap();
    let hair = studio_domain::ps4_hair::HairCatalogue::load(&install, &defs, "nhl21").unwrap();
    let player = reg.list_players(id).unwrap().into_iter().find(|p| !p.is_goalie && p.team_id.is_some()).unwrap();
    let cells = || {
        let v = reg.player_view(id, player.player_id).unwrap().unwrap();
        let get = |n: &str| v.all_fields.iter().find(|f| f.name == n).and_then(|f| f.decoded).unwrap();
        (v.bio_record, get("headid"), get("headtexture"), get("hairid"))
    };
    let (rec, head0, tex0, hair0) = cells();
    let head_t = *heads.heads.iter().find(|(h, n)| **h >= 60_000 && **n == 1 && **h != head0).unwrap().0;
    let hair_t = *hair.styles.iter().find(|(h, n)| **n == 1 && **h != hair0).unwrap().0;
    let unchanged = |why: &str| {
        assert_eq!(cells(), (rec, head0, tex0, hair0), "{why}: the player row is unchanged");
        assert!(!reg.pending_changes(id, 10).unwrap().dirty, "{why}: nothing pending");
    };

    // 1. Warm the cache and review. The plan names the exact member hashes and the definitions hash as read.
    let t = std::time::Instant::now();
    let p = call(&reg, id, &args, &defs, "head", rec, head_t, None).unwrap();
    let cold = t.elapsed();
    assert!(p.allowed, "{:?}", p.refusals);
    let (src, _) = studio_domain::ps4_vault::read_install_vault(&install, &defs).unwrap();
    for (n, h) in &src.members {
        assert!(p.source_identity.iter().any(|l| l.ends_with(&format!("{n} sha256 {h}"))), "{n}: {:?}", p.source_identity);
    }
    assert!(p.source_identity.iter().any(|l| l.ends_with(&sha(&ids_file))), "{:?}", p.source_identity);
    let hp = call(&reg, id, &args, &defs, "hair", rec, hair_t, None).unwrap();
    assert!(hp.allowed && hp.written_fields.len() == 1, "{:?}", hp.refusals);
    // Unchanged source: the cache is reused and the review stands.
    let t = std::time::Instant::now();
    assert_eq!(call(&reg, id, &args, &defs, "head", rec, head_t, None).unwrap().fingerprint, p.fingerprint);
    let warm = t.elapsed();

    // 2. A vault member changed while the archive keeps its length and modification time — twice: (a) one stored byte
    // flipped so the member still decodes, to **different** bytes of the same length (the identity-hash path); (b) the last
    // stored byte flipped, which breaks the member's compression (the decode path). Each is followed by a restore.
    let entries = studio_domain::ps4_vault::eb_index(&boot).unwrap();
    let member = entries.iter().find(|e| e.path.eq_ignore_ascii_case("nhl/data/common/database/attrib/gccollections.bin")).unwrap();
    let (m0, m1) = (member.offset as usize, member.offset as usize + member.size as usize);
    let form = if genuine[m0..].starts_with(b"chunkzip") { "chunkzip" } else { "stored" };
    let plain = |raw: &[u8]| match tdb_archive_big::eb::decode_payload(raw.to_vec()) {
        Ok(tdb_archive_big::eb::Payload::Data(d)) => Some(d),
        _ => None,
    };
    let original = plain(&genuine[m0..m1]).unwrap();
    // The first byte, from the back, whose flip still decodes to other bytes of the same length (searched in memory).
    let decodable = (m0 + 64..m1).rev().step_by(97).take(4000).find(|&at| {
        let mut r = genuine[m0..m1].to_vec();
        r[at - m0] ^= 0x01;
        plain(&r).is_some_and(|d| d.len() == original.len() && d != original)
    });
    let mut observed = Vec::new();
    for (case, at) in [("decodes to other bytes", decodable.expect("a decodable single-byte change")), ("breaks the compression", m1 - 1)] {
        let mut changed = genuine.clone();
        changed[at] ^= 0x01;
        write_keep_mtime(&boot, &changed);
        assert_ne!(sha(&boot), format!("{:x}", Sha256::digest(&genuine)), "content changed");
        observed.push(match studio_domain::ps4_vault::read_install_vault(&install, &defs) {
            Ok((s2, _)) => {
                assert_ne!(s2.members, src.members, "the member hash moves with the bytes");
                assert_eq!(s2.title, None, "no recorded identity matches the changed member");
                format!("{case} (byte {at}): re-hashed, matches no title")
            }
            Err(e) => format!("{case} (byte {at}): {e}"),
        });
        let e = call(&reg, id, &args, &defs, "head", rec, head_t, Some(&p.fingerprint)).err().expect("refused");
        assert!(e.contains("not applied"), "{case}: {e}");
        if case.starts_with("decodes") {
            assert!(e.contains("matches no recorded PS4 title identity"), "the identity check refused it: {e}");
        }
        unchanged(&format!("head, {case}"));
        let e2 = call(&reg, id, &args, &defs, "hair", rec, hair_t, Some(&hp.fingerprint)).err().expect("refused");
        assert!(e2.contains("not applied"), "{case}: {e2}");
        unchanged(&format!("hair, {case}"));
        let now = call(&reg, id, &args, &defs, "head", rec, head_t, None).unwrap();
        assert!(!now.allowed && now.source_identity.is_empty(), "{:?}", now.refusals);
        write_keep_mtime(&boot, &genuine);
    }
    let observed = observed.join("; ");

    // 3. Timestamp-changing replacement with the original bytes: identity is content, so the old review stands again.
    std::fs::write(&boot, &genuine).unwrap();
    assert_eq!(call(&reg, id, &args, &defs, "head", rec, head_t, None).unwrap().fingerprint, p.fingerprint);
    // …and with another title's bytes: refused as a source mismatch.
    std::fs::copy(Path::new(NHL20).join("databigs").join("boot.big"), &boot).expect("NHL 20 boot.big (fails, never skips)");
    let e3 = call(&reg, id, &args, &defs, "head", rec, head_t, Some(&p.fingerprint)).err().expect("refused");
    assert!(e3.contains("source mismatch") && e3.contains("nhl20"), "{e3}");
    unchanged("another title's archive");
    std::fs::write(&boot, &genuine).unwrap();

    // 4. Identity definitions. Unrelated text in the file changes its hash: the review is stale (whole-file granularity).
    std::fs::write(&ids_file, ids_text.replacen("PS4 Attrib/VLT vault identities", "PS4 Attrib/VLT vault identities (edited)", 1)).unwrap();
    let e4 = call(&reg, id, &args, &defs, "head", rec, head_t, Some(&p.fingerprint)).err().expect("refused");
    assert!(e4.contains("differs from the one reviewed") && e4.contains("Nothing was written"), "{e4}");
    let e4h = call(&reg, id, &args, &defs, "hair", rec, hair_t, Some(&hp.fingerprint)).err().expect("refused");
    assert!(e4h.contains("differs from the one reviewed"), "{e4h}");
    unchanged("definitions edited");
    // This title's record changed: the members no longer match it, refused.
    let nhl21_vlt = &src.members.iter().find(|(n, _)| n == "gccollections.vlt").unwrap().1;
    std::fs::write(&ids_file, ids_text.replace(nhl21_vlt.as_str(), &"0".repeat(64))).unwrap();
    let e5 = call(&reg, id, &args, &defs, "head", rec, head_t, Some(&p.fingerprint)).err().expect("refused");
    assert!(e5.contains("matches no recorded PS4 title identity"), "{e5}");
    unchanged("this title's definition changed");
    // Another defs root with identical content: the same identity, so the original review stands; the apply writes.
    std::fs::write(&ids_file, &ids_text).unwrap();
    let r = call(&reg, id, &args, &repo("defs"), "head", rec, head_t, Some(&p.fingerprint)).unwrap();
    assert!(r.applied.is_some());
    assert_eq!(cells(), (rec, head_t, head_t, hair0), "headid and headtexture written; hair untouched");
    assert!(reg.undo(id).unwrap());
    unchanged("after undo");
    eprintln!(
        "nhl21 scratch boot.big: gccollections.bin {form} [{m0}..{m1}) → {observed}; head {head0}→{head_t}, hair {hair0}→{hair_t}; cold check {cold:?}, warm {warm:?}"
    );
}

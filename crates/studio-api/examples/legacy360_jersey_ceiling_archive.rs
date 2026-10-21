//! Jersey-ceiling probe, step 2: write the saved project database **into `cacheboot.big`** — the
//! archive the Xbox 360 game actually loads its database from — through the Studio's copied-target
//! archive-member route, then (with `--install`) place the rebuilt archive into the game folder
//! with a verified backup. No loose-file extraction is involved: this is the trial for modding
//! archives in place.
//!
//! Usage: `legacy360_jersey_ceiling_archive PROJECT_DIR GAME_ROOT [--install]`
//! - `PROJECT_DIR`: the Studio project built by `legacy360_jersey_ceiling_mod` (saved, clean).
//! - `GAME_ROOT`: the extracted Xbox 360 game folder holding `cacheboot.big`.
//! Without `--install` the rebuilt archive stays in the project's `working/` folder.
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use studio_api::Registry;
use studio_domain::tdb_archive_big::stage::BoundSource;
use studio_domain::tdb_txn::FileIdentity;

const ARCHIVE: &str = "cacheboot.big";
const MEMBER: &str = "db/nhlng.db";
const BACKUP_SUFFIX: &str = ".nhlms-vanilla-backup";

fn text(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// The stored bytes of `member` inside an EB archive on disk.
fn member_bytes(archive: &Path, member: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut f = std::fs::File::open(archive)?;
    let bound = BoundSource::bind(&mut f)?;
    let e = bound.entries.iter().find(|e| e.path == member).ok_or(format!("{} has no member {member}", archive.display()))?;
    let mut buf = vec![0u8; e.size as usize];
    f.seek(SeekFrom::Start(e.offset))?;
    f.read_exact(&mut buf)?;
    Ok(buf)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args.len() > 4 || (args.len() == 4 && args[3] != "--install") {
        return Err("usage: legacy360_jersey_ceiling_archive PROJECT_DIR GAME_ROOT [--install]".into());
    }
    let project_dir = PathBuf::from(&args[1]);
    let game_root = PathBuf::from(&args[2]);
    let install = args.len() == 4;
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let defs = text(&repo.join("defs"));

    let game_archive = game_root.join(ARCHIVE);
    let vanilla = FileIdentity::of_file(&game_archive)?;

    let reg = Registry::default();
    let opened = reg.project_open(&text(&project_dir), &defs)?;
    let id = opened.summary.project_id;
    let working_db = PathBuf::from(opened.project.working_main_db.clone().ok_or("project has no working database")?);
    let saved_db = FileIdentity::of_file(&working_db)?;
    let working_root = project_dir.join("working");

    // The project's working folder is a copied-target root; the game's archive is copied into it
    // and rebuilt there. The game folder itself is never written by the engine.
    let staged_archive = working_root.join(ARCHIVE);
    if staged_archive.exists() {
        let have = FileIdentity::of_file(&staged_archive)?;
        if have.sha256 != vanilla.sha256 {
            println!("NOTE working/{ARCHIVE} already differs from the game's copy (a previous rebuild); re-copying the game's archive first");
            std::fs::remove_file(&staged_archive)?;
        }
    }
    if !staged_archive.exists() {
        std::fs::copy(&game_archive, &staged_archive)?;
    }

    let review = reg.copied_target_review_archive_member(id, &text(&working_root), ARCHIVE, MEMBER)?;
    println!("REVIEW handle {} txn {} qualification {}", review.handle, review.txn_id, review.qualification_state);
    for a in &review.artifacts {
        println!("  artifact {} ({}): {} -> {} prior {:?} output {} bytes {}", a.key, a.kind, a.label, a.destination_path, a.prior.as_ref().map(|p| &p.sha256[..12]), a.output.len, &a.output.sha256[..12]);
    }
    let applied = reg.copied_target_apply(id, review.handle)?;
    println!("APPLY state {} txn {} recovery {}", applied.state, applied.txn_id, applied.recovery_dir);
    for l in &applied.log {
        println!("  log: {l}");
    }
    if applied.state != "committed" {
        return Err(format!("the archive rebuild did not commit (state {})", applied.state).into());
    }

    // Verify: the member inside the rebuilt archive is exactly the saved database.
    let stored = member_bytes(&staged_archive, MEMBER)?;
    let stored_id = FileIdentity::of_bytes(&stored);
    if stored_id.sha256 != saved_db.sha256 {
        return Err(format!("member {MEMBER} in the rebuilt archive ({} bytes, {}) is not the saved database ({} bytes, {})", stored.len(), stored_id.sha256, saved_db.len, saved_db.sha256).into());
    }
    let rebuilt = FileIdentity::of_file(&staged_archive)?;
    println!("VERIFIED {MEMBER} inside working/{ARCHIVE} == saved database ({} bytes); archive {} bytes sha256 {}", stored.len(), rebuilt.len, rebuilt.sha256);

    let mut installed = None;
    if install {
        let backup = game_root.join(format!("{ARCHIVE}{BACKUP_SUFFIX}"));
        if backup.exists() {
            let b = FileIdentity::of_file(&backup)?;
            if b.sha256 != vanilla.sha256 {
                return Err(format!("{} exists and is not the game's current archive; refusing to overwrite a backup", backup.display()).into());
            }
        } else {
            std::fs::copy(&game_archive, &backup)?;
            let b = FileIdentity::of_file(&backup)?;
            if b.sha256 != vanilla.sha256 {
                return Err("the backup copy does not match the game's archive; nothing installed".into());
            }
        }
        let tmp = game_root.join(format!("{ARCHIVE}.nhlms-install-tmp"));
        std::fs::copy(&staged_archive, &tmp)?;
        if FileIdentity::of_file(&tmp)?.sha256 != rebuilt.sha256 {
            let _ = std::fs::remove_file(&tmp);
            return Err("the copy into the game folder does not match the rebuilt archive; nothing installed".into());
        }
        std::fs::rename(&tmp, &game_archive)?;
        let now = FileIdentity::of_file(&game_archive)?;
        println!("INSTALLED {} ({} bytes sha256 {}); backup at {}", game_archive.display(), now.len, now.sha256, backup.display());
        installed = Some(serde_json::json!({ "path": text(&game_archive), "sha256": now.sha256, "backup": text(&backup), "backup_sha256": vanilla.sha256 }));
    }

    let receipt = serde_json::json!({
        "test": "NHL Legacy (Xbox 360): saved project database written into cacheboot.big (archive-member route, no loose extraction)",
        "project": text(&project_dir),
        "saved_database": saved_db,
        "game_archive_before": { "path": text(&game_archive), "sha256": vanilla.sha256, "len": vanilla.len },
        "rebuilt_archive": { "path": text(&staged_archive), "sha256": rebuilt.sha256, "len": rebuilt.len },
        "copied_target": { "txn_id": applied.txn_id, "state": applied.state, "qualification": review.qualification_state, "recovery_dir": applied.recovery_dir },
        "installed": installed,
        "uninstall": format!("copy {ARCHIVE}{BACKUP_SUFFIX} back over {ARCHIVE} in the game folder (its sha256 is recorded above)"),
        "game_verified": false,
    });
    let out = project_dir.parent().unwrap_or(&project_dir).join("archive-receipt.json");
    std::fs::write(&out, serde_json::to_vec_pretty(&receipt)?)?;
    println!("RESULT {}", serde_json::to_string(&receipt)?);
    reg.close_project(id);
    Ok(())
}

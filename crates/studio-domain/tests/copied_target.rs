//! S2-C domain slice: a real [`Workspace`] (the nhl14 PS3 `nhlng.db` + aligned ROSTER pair, on
//! temp copies) staged as one copied-target change set — primary, roster and the save-index header
//! the copied tree newly requires — through `tdb-txn`, with the session-baseline lifecycle across
//! two consecutive saves and an ordered restore.
//!
//! The copied tree's **layout** is the header-indexed content shape verified on a real NHL Legacy
//! Recomp tree (`content_header`); the files inside it are the committed PS3 nhl14 anchors. That is
//! a fixture layout for the planner and the transaction, not a claim about any game reading it.

use std::path::{Path, PathBuf};

use studio_domain::content_header::{plan_content_header, HeaderPlan};
use studio_domain::copied_target::{
    stage_workspace, verify_plan_inputs, ArtifactRole, BoundInputKind, KEY_PRIMARY, KEY_ROSTER,
};
use studio_domain::tdb_txn::{self, Expected, FileIdentity, NoHooks, State, TxnError};
use studio_domain::{
    BaselineAction, EditSession, FieldValue, Platform, Project, ProjectRole, Workspace,
};

const NHLNG: &str = "anchors/ps3/nhl14/databases/nhlng.db";
const META: &str = "anchors/ps3/nhl14/databases/nhlng-meta.xml";
const DEFS: &str = "defs";
const ROSTER: &str = "anchors/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA";
const PROFILE: &str = "B13EBABEBABEBABE";
const TITLE: &str = "454109EC";
const SAVE_NAME: &str = "ROSTER 20260723000649";

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-s2c-{tag}-{}", std::process::id()));
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

/// A copied-target root holding `USRDIR/nhlng.db` and the roster in the header-indexed content
/// layout (`Headers/00000001/` present, header absent). Returns `(root, nhlng, roster, headers)`.
fn copied_tree(base: &Path, with_headers_index: bool) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let root = base.join("copy");
    tdb_txn::engine::create_copied_target_root(&root).unwrap();
    std::fs::create_dir_all(root.join("USRDIR")).unwrap();
    let nhlng = root.join("USRDIR/nhlng.db");
    std::fs::copy(repo(NHLNG), &nhlng).unwrap();
    let save_dir = root
        .join(PROFILE)
        .join(TITLE)
        .join("00000001")
        .join(SAVE_NAME);
    std::fs::create_dir_all(&save_dir).unwrap();
    let roster = save_dir.join(SAVE_NAME);
    std::fs::copy(repo(ROSTER), &roster).unwrap();
    let headers = root.join(PROFILE).join(TITLE).join("Headers/00000001");
    if with_headers_index {
        std::fs::create_dir_all(&headers).unwrap();
    }
    (root, nhlng, roster, headers)
}

fn open_ws(nhlng: &Path, roster: &Path) -> Workspace {
    let n = EditSession::begin(
        Project::load(
            nhlng,
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    let r = EditSession::begin(
        Project::load(
            roster,
            Platform::Ps3,
            ProjectRole::Roster,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    Workspace::with_roster(
        n,
        nhlng.to_string_lossy().into_owned(),
        r,
        roster.to_string_lossy().into_owned(),
    )
    .unwrap()
}

fn edit(ws: &mut Workspace, value: i64) {
    ws.set_field("ubPc", 0, "vjla", FieldValue::Int(value))
        .unwrap();
    assert!(ws.is_dirty());
}

#[test]
fn workspace_saves_twice_through_one_journal_and_restores_in_order() {
    let tmp = Tmp::new("cycle");
    let (root, nhlng, roster, headers) = copied_tree(&tmp.0, true);
    let staging = tmp.0.join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    let orig_nhlng = FileIdentity::of_file(&nhlng).unwrap();
    let orig_roster = FileIdentity::of_file(&roster).unwrap();
    let header_path = headers.join(format!("{SAVE_NAME}.header"));

    let mut ws = open_ws(&nhlng, &roster);
    assert!(ws.roster_aligned());
    edit(&mut ws, 2);
    let rev1 = ws.revision();

    // ---- cycle 1: primary + roster + the header the copied tree newly requires.
    let plan = stage_workspace(&ws, &root, &staging.join("1"), "cycle 1").unwrap_err();
    assert!(
        plan.to_string().contains("staging"),
        "staging dir must exist: {plan}"
    );
    std::fs::create_dir_all(staging.join("1")).unwrap();
    let plan = stage_workspace(&ws, &root, &staging.join("1"), "cycle 1").unwrap();
    assert_eq!(plan.revision, rev1);
    let keys: Vec<&str> = plan
        .artifacts
        .iter()
        .map(|a| a.artifact.key.as_str())
        .collect();
    assert_eq!(keys, vec![KEY_PRIMARY, KEY_ROSTER, "roster.header"]);
    let header = plan.artifact("roster.header").unwrap();
    assert_eq!(
        header.role,
        ArtifactRole::ContentHeader {
            for_key: KEY_ROSTER.into()
        }
    );
    assert_eq!(header.artifact.expected_prior, Expected::Absent);
    assert_eq!(header.artifact.dependencies, vec![KEY_ROSTER.to_owned()]);
    assert_eq!(
        header.artifact.destination,
        format!("{PROFILE}/{TITLE}/Headers/00000001/{SAVE_NAME}.header")
    );
    assert!(plan.bound_inputs.is_empty(), "synthesized: nothing to bind");
    assert!(header.artifact.validation.qualification.items[0]
        .note
        .contains("synthesized"));
    assert_eq!(
        plan.artifact(KEY_PRIMARY).unwrap().artifact.expected_prior,
        Expected::Present(orig_nhlng.clone())
    );
    assert_eq!(
        plan.artifact(KEY_PRIMARY).unwrap().artifact.destination,
        "USRDIR/nhlng.db"
    );
    assert!(plan
        .change_set
        .artifacts
        .iter()
        .all(|a| !a.validation.qualification.is_qualified()));
    let mut txn1 = tdb_txn::prepare(&plan.root, &plan.change_set, &mut NoHooks).unwrap();
    assert!(txn1.journal.applied_unqualified);
    let order: Vec<_> = txn1
        .journal
        .artifacts
        .iter()
        .map(|a| a.key.clone())
        .collect();
    assert!(
        order.iter().position(|k| k == KEY_ROSTER)
            < order.iter().position(|k| k == "roster.header")
    );
    verify_plan_inputs(&plan).unwrap();
    let out1 = tdb_txn::apply(&mut txn1, &plan.change_set, &mut NoHooks).unwrap();
    assert_eq!(out1.state, State::Committed);
    assert_eq!(std::fs::read(&header_path).unwrap().len(), 0x148);
    assert_eq!(
        FileIdentity::of_file(&header_path).unwrap(),
        header.artifact.replacement_identity
    );
    let header_bytes_1 = std::fs::read(&header_path).unwrap();

    // ---- post-commit lifecycle: the sessions adopt the installed files.
    let installed_nhlng = FileIdentity::of_file(&nhlng).unwrap();
    let installed_roster = FileIdentity::of_file(&roster).unwrap();
    assert!(ws.is_dirty(), "not rebound yet");
    ws.rebind_after_copied_save(Some(&installed_nhlng), Some(&installed_roster), None)
        .unwrap();
    assert!(!ws.is_dirty(), "the installed file is the baseline now");
    assert_eq!(
        ws.primary().project().loaded_identity(),
        Some(&installed_nhlng)
    );
    assert_eq!(ws.undo_depth(), 1, "history kept across the save");
    let reloaded = Project::load(
        &nhlng,
        Platform::Ps3,
        ProjectRole::Nhlng,
        repo(META),
        repo(DEFS),
    )
    .unwrap();
    assert_eq!(reloaded.db_bytes(), ws.primary().bytes());

    // ---- cycle 2: the header now exists and is preserved byte-for-byte; the priors are the
    // installed identities, not the originals.
    edit(&mut ws, 3);
    assert_eq!(
        plan_content_header(&roster).unwrap(),
        HeaderPlan::Present(header_path.clone())
    );
    std::fs::create_dir_all(staging.join("2")).unwrap();
    let plan2 = stage_workspace(&ws, &root, &staging.join("2"), "cycle 2").unwrap();
    let keys2: Vec<&str> = plan2
        .artifacts
        .iter()
        .map(|a| a.artifact.key.as_str())
        .collect();
    assert_eq!(keys2, vec![KEY_PRIMARY, KEY_ROSTER]);
    assert_eq!(
        plan2.artifact(KEY_PRIMARY).unwrap().artifact.expected_prior,
        Expected::Present(installed_nhlng.clone())
    );
    let mut txn2 = tdb_txn::prepare(&plan2.root, &plan2.change_set, &mut NoHooks).unwrap();
    let out2 = tdb_txn::apply(&mut txn2, &plan2.change_set, &mut NoHooks).unwrap();
    assert_eq!(out2.state, State::Committed);
    assert_eq!(std::fs::read(&header_path).unwrap(), header_bytes_1);
    ws.rebind_after_copied_save(
        Some(&FileIdentity::of_file(&nhlng).unwrap()),
        Some(&FileIdentity::of_file(&roster).unwrap()),
        None,
    )
    .unwrap();
    assert!(!ws.is_dirty());

    // ---- ordered restore: the first transaction cannot be restored while the second owns the
    // destinations; restoring the second reloads the clean workspace to the first's bytes.
    let e = tdb_txn::restore(&root, &txn1.txn_id, &mut NoHooks).unwrap_err();
    assert!(matches!(e, TxnError::RestoreOutOfOrder { .. }), "{e}");
    tdb_txn::restore(&root, &txn2.txn_id, &mut NoHooks).unwrap();
    let r = ws.refresh_baselines().unwrap();
    assert_eq!(r.primary, BaselineAction::Reloaded);
    assert_eq!(r.roster, Some(BaselineAction::Reloaded));
    assert!(ws.stale_baseline().is_none());
    assert_eq!(FileIdentity::of_file(&nhlng).unwrap(), installed_nhlng);
    assert_eq!(
        FileIdentity::of_bytes(ws.primary().bytes()),
        FileIdentity::of_bytes(reloaded.db_bytes()),
        "the workspace now shows cycle 1's bytes"
    );
    assert_eq!(ws.undo_depth(), 0, "history discarded on reload");
    tdb_txn::restore(&root, &txn1.txn_id, &mut NoHooks).unwrap();
    let r = ws.refresh_baselines().unwrap();
    assert_eq!(r.primary, BaselineAction::Reloaded);
    assert_eq!(FileIdentity::of_file(&nhlng).unwrap(), orig_nhlng);
    assert_eq!(FileIdentity::of_file(&roster).unwrap(), orig_roster);
    assert!(
        !header_path.exists(),
        "the originally absent header is gone again"
    );
    assert_eq!(
        std::fs::read(&nhlng).unwrap(),
        std::fs::read(repo(NHLNG)).unwrap()
    );
    assert_eq!(
        std::fs::read(&roster).unwrap(),
        std::fs::read(repo(ROSTER)).unwrap()
    );
}

#[test]
fn a_dirty_workspace_keeps_its_edits_when_the_files_change_underneath_it() {
    let tmp = Tmp::new("stale");
    let (root, nhlng, roster, _) = copied_tree(&tmp.0, true);
    let staging = tmp.0.join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    let mut ws = open_ws(&nhlng, &roster);
    edit(&mut ws, 2);
    let plan = stage_workspace(&ws, &root, &staging, "save").unwrap();
    let mut txn = tdb_txn::prepare(&plan.root, &plan.change_set, &mut NoHooks).unwrap();
    tdb_txn::apply(&mut txn, &plan.change_set, &mut NoHooks).unwrap();
    ws.rebind_after_copied_save(
        Some(&FileIdentity::of_file(&nhlng).unwrap()),
        Some(&FileIdentity::of_file(&roster).unwrap()),
        None,
    )
    .unwrap();
    // A further edit, then the transaction is restored underneath the dirty workspace.
    edit(&mut ws, 3);
    let dirty_bytes = ws.primary().bytes().to_vec();
    tdb_txn::restore(&root, &txn.txn_id, &mut NoHooks).unwrap();
    let r = ws.refresh_baselines().unwrap();
    assert_eq!(r.primary, BaselineAction::StaleKeptEdits);
    assert_eq!(r.roster, Some(BaselineAction::StaleKeptEdits));
    assert_eq!(ws.primary().bytes(), dirty_bytes.as_slice(), "edits kept");
    assert!(ws.is_dirty());
    let reason = ws.stale_baseline().unwrap().to_owned();
    assert!(
        reason.contains("main database") && reason.contains("roster save"),
        "{reason}"
    );
    // Saving refuses until an explicit reload; the refusal carries the reason.
    let e = stage_workspace(&ws, &root, &staging.join("x"), "again").unwrap_err();
    assert!(e.to_string().contains("edits were kept"), "{e}");
    ws.reload_from_disk().unwrap();
    assert!(!ws.is_dirty());
    assert!(ws.stale_baseline().is_none());
    assert_eq!(
        std::fs::read(&nhlng).unwrap(),
        std::fs::read(repo(NHLNG)).unwrap()
    );
    assert_eq!(
        ws.primary().project().loaded_identity(),
        Some(&FileIdentity::of_file(&nhlng).unwrap())
    );
}

#[test]
fn template_headers_are_bound_inputs_and_existing_headers_are_never_rewritten() {
    let tmp = Tmp::new("template");
    let (root, nhlng, roster, headers) = copied_tree(&tmp.0, true);
    let staging = tmp.0.join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    // A same-type sibling sidecar with game-authored display strings and a stale name tail.
    let mut template = vec![0u8; 0x148];
    template[0x00..0x08].copy_from_slice(&[0, 0, 0, 1, 0, 0, 0, 1]);
    template[0x08..0x16].copy_from_slice(b"\0R\0O\0S\0T\0E\0R\0\x31");
    let old_name = b"ROSTER 20260702231158\x001134";
    template[0x108..0x108 + old_name.len()].copy_from_slice(old_name);
    template[0x138..0x148].copy_from_slice(&[
        0xB1, 0x3E, 0xBA, 0xBE, 0xBA, 0xBE, 0xBA, 0xBE, 0x45, 0x41, 0x09, 0xEC, 0, 0, 0, 0,
    ]);
    let template_path = headers.join("ROSTER 20260702231158.header");
    std::fs::write(&template_path, &template).unwrap();

    let mut ws = open_ws(&nhlng, &roster);
    edit(&mut ws, 2);
    let plan = stage_workspace(&ws, &root, &staging, "t").unwrap();
    let header = plan.artifact("roster.header").unwrap();
    let bytes = std::fs::read(&header.artifact.replacement).unwrap();
    assert_eq!(
        &bytes[0x08..0x16],
        b"\0R\0O\0S\0T\0E\0R\0\x31",
        "display strings cloned"
    );
    assert_eq!(&bytes[0x108..0x108 + SAVE_NAME.len()], SAVE_NAME.as_bytes());
    assert!(bytes[0x108 + SAVE_NAME.len()..0x138]
        .iter()
        .all(|&b| b == 0));
    assert_eq!(plan.bound_inputs.len(), 1);
    assert_eq!(
        std::fs::canonicalize(&plan.bound_inputs[0].path).unwrap(),
        std::fs::canonicalize(&template_path).unwrap()
    );
    assert_eq!(
        plan.bound_inputs[0].identity,
        FileIdentity::of_bytes(&template)
    );
    assert!(header.artifact.validation.detail["template"].ends_with("ROSTER 20260702231158.header"));
    verify_plan_inputs(&plan).unwrap();
    // The template changes after planning: the plan is invalid, and says which input moved.
    let mut t2 = template.clone();
    t2[0x08] ^= 1;
    std::fs::write(&template_path, &t2).unwrap();
    let e = verify_plan_inputs(&plan).unwrap_err().to_string();
    assert!(
        e.contains("template for the save index header") && e.contains("re-prepare"),
        "{e}"
    );
    std::fs::write(&template_path, &template).unwrap();
    verify_plan_inputs(&plan).unwrap();
    assert_eq!(
        std::fs::read(&template_path).unwrap(),
        template,
        "the template itself is untouched"
    );

    // A header that already exists is neither staged nor rewritten.
    let existing = headers.join(format!("{SAVE_NAME}.header"));
    std::fs::write(&existing, vec![0xEE; 0x148]).unwrap();
    let staging2 = tmp.0.join("staging2");
    std::fs::create_dir_all(&staging2).unwrap();
    let plan2 = stage_workspace(&ws, &root, &staging2, "t2").unwrap();
    assert!(plan2.artifact("roster.header").is_none());
    // R1: the existing header is the reason no header is planned — it is bound, not forgotten.
    assert_eq!(plan2.bound_inputs.len(), 1);
    assert_eq!(plan2.bound_inputs[0].kind, BoundInputKind::ContentHeader);
    assert_eq!(
        plan2.bound_inputs[0].identity,
        FileIdentity::of_bytes(&vec![0xEE; 0x148])
    );
    assert_eq!(std::fs::read(&existing).unwrap(), vec![0xEE; 0x148]);
}

#[test]
fn refusals_name_the_root_the_directory_and_the_change() {
    let tmp = Tmp::new("refuse");
    let (root, nhlng, roster, headers) = copied_tree(&tmp.0, true);
    let staging = tmp.0.join("staging");
    std::fs::create_dir_all(&staging).unwrap();

    // A workspace whose roster lives outside the root: refused, root never widened.
    let outside = tmp.0.join("elsewhere");
    std::fs::create_dir_all(&outside).unwrap();
    let outside_roster = outside.join("ROSTER");
    std::fs::copy(repo(ROSTER), &outside_roster).unwrap();
    let mut ws = open_ws(&nhlng, &outside_roster);
    edit(&mut ws, 2);
    let e = stage_workspace(&ws, &root, &staging, "x")
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("attached roster save")
            && e.contains("not inside the selected copied-target root"),
        "{e}"
    );

    // A root without the marker is not a copied target.
    let plain = tmp.0.join("plain");
    std::fs::create_dir_all(&plain).unwrap();
    let e = stage_workspace(&ws, &plain, &staging, "x")
        .unwrap_err()
        .to_string();
    assert!(e.contains("not a copied-target root"), "{e}");

    // The real pair: a missing parent directory is reported by name, before anything is prepared.
    let mut ws = open_ws(&nhlng, &roster);
    edit(&mut ws, 2);
    let staging_b = tmp.0.join("staging-b");
    std::fs::create_dir_all(&staging_b).unwrap();
    let plan = stage_workspace(&ws, &root, &staging_b, "x").unwrap();
    assert!(plan.artifact("roster.header").is_some());
    std::fs::remove_dir_all(&headers).unwrap();
    let e = verify_plan_inputs(&plan).unwrap_err().to_string();
    assert!(
        e.contains("Headers") && e.contains("does not exist in the copied target"),
        "{e}"
    );
    let e = tdb_txn::prepare(&plan.root, &plan.change_set, &mut NoHooks).unwrap_err();
    assert!(
        matches!(e, TxnError::MissingParent { .. }),
        "the engine agrees: {e}"
    );
    assert_eq!(
        std::fs::read(&nhlng).unwrap(),
        std::fs::read(repo(NHLNG)).unwrap()
    );

    // Without a Headers index the tree is not header-indexed: no header artifact, plain save.
    let tmp2 = Tmp::new("plain-tree");
    let (root2, nhlng2, roster2, _) = copied_tree(&tmp2.0, false);
    let mut ws2 = open_ws(&nhlng2, &roster2);
    edit(&mut ws2, 2);
    let staging2 = tmp2.0.join("staging");
    std::fs::create_dir_all(&staging2).unwrap();
    let plan2 = stage_workspace(&ws2, &root2, &staging2, "plain").unwrap();
    assert_eq!(plan2.artifacts.len(), 2);
    assert!(plan2.artifact("roster.header").is_none());

    // The destination changing after the session opened is a conflict, never a new baseline.
    let mut edited = std::fs::read(&nhlng2).unwrap();
    edited[100] ^= 1;
    std::fs::write(&nhlng2, &edited).unwrap();
    let e = verify_plan_inputs(&plan2).unwrap_err().to_string();
    assert!(
        e.contains("changed on disk after the session opened"),
        "{e}"
    );
    let staging3 = tmp2.0.join("staging3");
    std::fs::create_dir_all(&staging3).unwrap();
    let e = stage_workspace(&ws2, &root2, &staging3, "again")
        .unwrap_err()
        .to_string();
    assert!(e.contains("changed since"), "{e}");
    assert!(ws2.is_dirty(), "edits kept through every refusal");
}

/// R3: a refresh that cannot be completed (a corrupt primary, a missing roster) changes nothing —
/// no session is half-refreshed — reports the error, keeps the edits and marks the workspace
/// unusable for copied-target saves until a reload succeeds.
#[test]
fn a_failed_refresh_leaves_the_workspace_untouched_and_marked() {
    let tmp = Tmp::new("refresh-fail");
    let (root, nhlng, roster, _) = copied_tree(&tmp.0, true);
    let staging = tmp.0.join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    let mut ws = open_ws(&nhlng, &roster);
    let orig_nhlng = std::fs::read(&nhlng).unwrap();
    let orig_roster = std::fs::read(&roster).unwrap();

    // Clean workspace, corrupt primary on disk: the primary would reload, but the reload cannot be
    // prepared — nothing changes, the reason is recorded.
    std::fs::write(&nhlng, b"not a database at all").unwrap();
    let loaded_before = ws.primary().project().loaded_identity().cloned();
    let e = ws.refresh_baselines().unwrap_err();
    assert!(!ws.is_dirty());
    assert_eq!(
        ws.primary().project().loaded_identity().cloned(),
        loaded_before
    );
    assert_eq!(ws.primary().bytes(), &orig_nhlng[..]);
    let reason = ws.stale_baseline().unwrap().to_owned();
    assert!(reason.contains("could not be re-read"), "{reason}");
    let refused = stage_workspace(&ws, &root, &staging, "x")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("could not be re-read"), "{refused}");
    let _ = e;
    // The file comes back exactly: a refresh finds the loaded identity and clears the mark.
    std::fs::write(&nhlng, &orig_nhlng).unwrap();
    let r = ws.refresh_baselines().unwrap();
    assert_eq!(r.primary, BaselineAction::Unchanged);
    assert!(ws.stale_baseline().is_none());

    // Dirty workspace, roster missing: the edits are kept, the primary is not touched even though
    // it changed on disk (a partial refresh would have adopted it), and reload refuses until the
    // file is back.
    edit(&mut ws, 2);
    let dirty_bytes = ws.primary().bytes().to_vec();
    // A valid database with a different identity (the edit's own packaged bytes) lands on disk.
    let changed = ws.primary().save().unwrap();
    assert_ne!(changed, orig_nhlng);
    std::fs::write(&nhlng, &changed).unwrap();
    std::fs::remove_file(&roster).unwrap();
    ws.refresh_baselines().unwrap_err();
    assert!(ws.is_dirty());
    assert_eq!(ws.primary().bytes(), dirty_bytes.as_slice());
    assert_eq!(
        ws.primary().project().loaded_identity().cloned(),
        loaded_before
    );
    assert!(ws.stale_baseline().is_some());
    ws.reload_from_disk().unwrap_err();
    assert_eq!(
        ws.primary().bytes(),
        dirty_bytes.as_slice(),
        "a failed reload changes nothing"
    );
    assert!(ws
        .stale_baseline()
        .unwrap()
        .contains("could not be reloaded"));
    std::fs::write(&roster, &orig_roster).unwrap();
    ws.reload_from_disk().unwrap();
    assert!(!ws.is_dirty());
    assert!(ws.stale_baseline().is_none());
    assert_eq!(ws.primary().bytes(), &changed[..]);
    assert!(ws.roster_aligned());
}

/// R1: a directory or redirected file where the header should be is never accepted as the
/// companion that justifies planning no header.
#[test]
fn a_directory_in_the_headers_place_is_refused_not_bound() {
    let tmp = Tmp::new("header-dir");
    let (root, nhlng, roster, headers) = copied_tree(&tmp.0, true);
    let staging = tmp.0.join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::create_dir_all(headers.join(format!("{SAVE_NAME}.header"))).unwrap();
    let mut ws = open_ws(&nhlng, &roster);
    edit(&mut ws, 2);
    let e = stage_workspace(&ws, &root, &staging, "x")
        .unwrap_err()
        .to_string();
    assert!(e.contains("not a regular file"), "{e}");
    assert!(ws.is_dirty());
}

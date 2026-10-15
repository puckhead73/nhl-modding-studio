//! S2-B — the Studio side of the copied-target transaction: turning the
//! things Studio can produce into [`tdb_txn::Artifact`]s that carry their
//! own validation and (unresolved) qualification, so one change set can
//! replace an archive, a database/save and a companion file through the
//! same journal and the same recovery policy.
//!
//! Nothing here writes a live target. The artifacts are handed to
//! [`tdb_txn::prepare`] / [`tdb_txn::apply`] against a root created by
//! [`tdb_txn::engine::create_copied_target_root`]; the existing direct
//! writers (`EditSession::save_to`, `Workspace::save_in_place`) are not
//! changed by this package.
//!
//! ## Validation is re-done here, not trusted (S2-B-R1, R4)
//!
//! * An archive enters only as a [`StagedArchive`], which this module
//!   constructs by running the S2-A stager itself (`stage_archive`); its
//!   fields are private, so a caller cannot assemble one from a report and a
//!   file of its own. [`archive_artifact`] then **re-runs the bound
//!   verification** (`stage::verify` against the recorded plan, the source
//!   and the staged output as they are on disk now) before producing the
//!   artifact.
//! * A session enters with the identity of the file it was **loaded from**
//!   ([`Project::loaded_identity`]) as its expected prior; if the destination
//!   no longer matches it, that is a conflict, not a new baseline.
//! * Staging file names are derived from validated keys only
//!   ([`tdb_txn::changeset::valid_key`]) and are created with `create_new`,
//!   never over an existing file.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use tdb_archive_big::stage::{
    self, BoundSource, Plan, QualificationState, QualificationStatus, StageReport,
    VerificationReport,
};
use tdb_txn::changeset::valid_key;
use tdb_txn::{Artifact, Expected, FileIdentity, Qualification, QualificationItem, Validation};

use crate::error::DomainError;
use crate::{EditSession, Project, Result};

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> DomainError + '_ {
    move |source| DomainError::Io {
        path: path.display().to_string(),
        source,
    }
}

fn check_key(key: &str) -> Result<()> {
    if valid_key(key) {
        Ok(())
    } else {
        Err(DomainError::Validation(format!(
            "artifact key `{key}` is not a plain name (letters, digits, `_`, `-`, `.`; no separators)"
        )))
    }
}

/// Write `bytes` to a **new** file `<staging_dir>/<key><suffix>`; an existing
/// file at that name is an error, never overwritten.
fn stage_bytes(staging_dir: &Path, key: &str, suffix: &str, bytes: &[u8]) -> Result<PathBuf> {
    check_key(key)?;
    let path = staging_dir.join(format!("{key}{suffix}"));
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(io(&path))?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(io(&path))?;
    Ok(path)
}

fn identity_of(id: &stage::SourceIdentity) -> FileIdentity {
    FileIdentity {
        len: id.len,
        sha256: stage::hex(&id.sha256),
    }
}

/// Map an S2-A qualification onto the engine's opaque record. Item statuses
/// and the state are carried as the stager states them; nothing is resolved.
pub fn qualification_of(report: &StageReport) -> Qualification {
    Qualification {
        state: match report.qualification.state {
            QualificationState::Qualified => "qualified",
            QualificationState::Unqualified => "unqualified",
        }
        .into(),
        items: report
            .qualification
            .items
            .iter()
            .map(|i| QualificationItem {
                subject: i.subject.clone(),
                status: match i.status {
                    QualificationStatus::Resolved => "resolved",
                    QualificationStatus::Unresolved => "unresolved",
                    QualificationStatus::Blocked => "blocked",
                }
                .into(),
                note: i.note.clone(),
            })
            .collect(),
    }
}

/// An archive staged by this module through the S2-A stager: the bound
/// source, the plan, the paths and the reports, all private. Only
/// [`stage_archive`] makes one.
#[derive(Debug)]
pub struct StagedArchive {
    bound: BoundSource,
    plan: Plan,
    source_path: PathBuf,
    output_path: PathBuf,
    report: StageReport,
    verification: VerificationReport,
}

impl StagedArchive {
    pub fn report(&self) -> &StageReport {
        &self.report
    }
    pub fn verification(&self) -> &VerificationReport {
        &self.verification
    }
    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}

/// Run the S2-A stager for `plan` on `source_path` into the new file
/// `output_path` (refused if it exists), verifying the result.
pub fn stage_archive(
    bound: BoundSource,
    plan: Plan,
    source_path: &Path,
    output_path: &Path,
) -> Result<StagedArchive> {
    let (report, verification) = stage::stage_file(&bound, &plan, source_path, output_path)
        .map_err(|e| DomainError::Validation(format!("staging {}: {e}", source_path.display())))?;
    Ok(StagedArchive {
        bound,
        plan,
        source_path: source_path.to_path_buf(),
        output_path: output_path.to_path_buf(),
        report,
        verification,
    })
}

/// A staged archive as a transaction artifact. The bound verification is
/// **re-run now** against the source and the output as they are on disk;
/// its result, not the earlier report, decides. The destination is expected
/// to be the source's exact identity (the copied target's archive *is* the
/// source the plan was bound to).
pub fn archive_artifact(key: &str, destination: &str, staged: &StagedArchive) -> Result<Artifact> {
    check_key(key)?;
    let mut out = std::fs::File::open(&staged.output_path).map_err(io(&staged.output_path))?;
    let mut src = std::fs::File::open(&staged.source_path).map_err(io(&staged.source_path))?;
    let fresh = stage::verify(
        &staged.bound,
        &staged.plan,
        &staged.report,
        &mut out,
        &mut src,
    )
    .map_err(|e| {
        DomainError::Validation(format!(
            "archive `{key}`: re-verification of {} against its plan failed: {e}",
            staged.output_path.display()
        ))
    })?;
    if fresh.entries_reparsed != staged.verification.entries_reparsed
        || fresh.changed_entries_decoded != staged.verification.changed_entries_decoded
        || fresh.untouched_bytes_compared != staged.verification.untouched_bytes_compared
        || fresh.output != staged.verification.output
    {
        return Err(DomainError::Validation(format!(
            "archive `{key}`: re-verification disagrees with the staging-time verification"
        )));
    }
    let Some(verified) = fresh.output.as_ref() else {
        return Err(DomainError::Validation(format!(
            "archive `{key}`: verification produced no output identity"
        )));
    };
    let source = identity_of(&staged.report.source);
    let output = identity_of(verified);
    if output != identity_of(&staged.report.output) {
        return Err(DomainError::Validation(format!(
            "archive `{key}`: verified output identity differs from the stage report"
        )));
    }
    let mut detail = BTreeMap::new();
    detail.insert(
        "original_eof".into(),
        staged.report.original_eof.to_string(),
    );
    detail.insert("new_eof".into(), staged.report.new_eof.to_string());
    detail.insert(
        "changed_ranges".into(),
        format!("{:?}", staged.report.changed_ranges),
    );
    detail.insert(
        "entries_reparsed".into(),
        fresh.entries_reparsed.to_string(),
    );
    detail.insert(
        "changed_entries_decoded".into(),
        fresh.changed_entries_decoded.to_string(),
    );
    detail.insert(
        "untouched_bytes_compared".into(),
        fresh.untouched_bytes_compared.to_string(),
    );
    detail.insert(
        "edits".into(),
        staged
            .plan
            .edits
            .iter()
            .map(|e| format!("{}:{:?}", e.entry_index, e.placement))
            .collect::<Vec<_>>()
            .join(";"),
    );
    Ok(Artifact {
        key: key.into(),
        destination: destination.into(),
        expected_prior: Expected::Present(source),
        replacement: staged.output_path.clone(),
        replacement_identity: output,
        validation: Validation {
            producer: "tdb-archive-big::stage".into(),
            structurally_verified: true,
            deploy_blockers: staged.report.deploy_blockers.clone(),
            qualification: qualification_of(&staged.report),
            detail,
        },
        dependencies: vec![],
    })
}

/// C4: a BIG4 archive + `.bh` pair staged through `tdb_archive_big::big4`.
#[derive(Debug)]
pub struct StagedBig4 {
    bound: tdb_archive_big::big4::BoundBig4,
    plan: tdb_archive_big::big4::Big4Plan,
    source_path: PathBuf,
    sidecar_source_path: PathBuf,
    output_path: PathBuf,
    sidecar_output_path: PathBuf,
    report: tdb_archive_big::big4::Big4StageReport,
}

impl StagedBig4 {
    pub fn plan(&self) -> &tdb_archive_big::big4::Big4Plan {
        &self.plan
    }
    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
    pub fn sidecar_output_path(&self) -> &Path {
        &self.sidecar_output_path
    }
}

/// Run the BIG4 stager for `plan` on the bound `source_path` + `sidecar_source_path` into the new
/// files `output_path` / `sidecar_output_path` (refused if they exist), verifying the result.
pub fn stage_big4(
    bound: tdb_archive_big::big4::BoundBig4,
    plan: tdb_archive_big::big4::Big4Plan,
    source_path: &Path,
    sidecar_source_path: &Path,
    output_path: &Path,
    sidecar_output_path: &Path,
) -> Result<StagedBig4> {
    let report = tdb_archive_big::big4::stage_files(&bound, &plan, source_path, output_path, sidecar_output_path)
        .map_err(|e| DomainError::Validation(format!("staging {}: {e}", source_path.display())))?;
    Ok(StagedBig4 {
        bound,
        plan,
        source_path: source_path.to_path_buf(),
        sidecar_source_path: sidecar_source_path.to_path_buf(),
        output_path: output_path.to_path_buf(),
        sidecar_output_path: sidecar_output_path.to_path_buf(),
        report,
    })
}

/// The two artifacts of a staged BIG4 pair (`<key>` = the archive, `<key>.bh` = the sidecar).
/// The bound verification is **re-run now** against the staged files and the sources on disk;
/// its result decides. Unqualified as every archive artifact is: native loading is unverified,
/// and an appended entry is a shape no stock archive exhibits.
pub fn big4_artifacts(key: &str, destination: &str, sidecar_destination: &str, staged: &StagedBig4) -> Result<(Artifact, Artifact)> {
    check_key(key)?;
    let sidecar_out = std::fs::read(&staged.sidecar_output_path).map_err(io(&staged.sidecar_output_path))?;
    let mut out = std::fs::File::open(&staged.output_path).map_err(io(&staged.output_path))?;
    let mut src = std::fs::File::open(&staged.source_path).map_err(io(&staged.source_path))?;
    let fresh = tdb_archive_big::big4::verify(&staged.bound, &staged.plan, &mut out, &sidecar_out, &mut src)
        .map_err(|e| DomainError::Validation(format!("archive `{key}`: re-verification of {} against its plan failed: {e}", staged.output_path.display())))?;
    if fresh != staged.report {
        return Err(DomainError::Validation(format!("archive `{key}`: re-verification disagrees with the staging-time verification")));
    }
    let sidecar_src = std::fs::read(&staged.sidecar_source_path).map_err(io(&staged.sidecar_source_path))?;
    if sidecar_src.len() as u64 != staged.bound.identity.sidecar_len || { use sha2::Digest; <[u8; 32]>::from(sha2::Sha256::digest(&sidecar_src)) } != staged.bound.identity.sidecar_sha256 {
        return Err(DomainError::Validation(format!("archive `{key}`: the sidecar {} changed since the plan was bound", staged.sidecar_source_path.display())));
    }
    let mut items = vec![
        QualificationItem { subject: "native loader acceptance".into(), status: "unresolved".into(), note: "no game-verified load of a staged BIG4 + .bh pair exists for this title/platform; structural verification is not game acceptance".into() },
    ];
    for e in &staged.plan.edits {
        let how = match &e.placement {
            tdb_archive_big::big4::Big4Placement::NoOp => continue,
            tdb_archive_big::big4::Big4Placement::FitInSlot { .. } => "fit-in-slot (directory size word + .bh record patched)".to_string(),
            tdb_archive_big::big4::Big4Placement::AppendAndRetarget { .. } => "append-and-retarget (entry re-pointed past the original EOF; directory offset order broken — a shape no stock archive exhibits, card R2-BIG4-1)".to_string(),
            tdb_archive_big::big4::Big4Placement::Reflow { delta, .. } => format!("ordered reflow (slot grown by {delta} bytes, every later entry and .bh record moved up by {delta}; the stock shape, reproduced against the research placement oracle on the six stock archives; loader acceptance is card R2-BIG4-1)"),
        };
        items.push(QualificationItem { subject: format!("entry {} `{}`: replacement", e.entry_index, e.path), status: "unresolved".into(), note: format!("{how}; acceptance of replaced content in this slot by the game is not established") });
    }
    let qualification = Qualification { state: "unqualified".into(), items };
    let mut detail = BTreeMap::new();
    detail.insert("original_len".into(), staged.plan.original_len.to_string());
    detail.insert("new_len".into(), staged.plan.new_len.to_string());
    detail.insert("changed_ranges".into(), format!("{:?}", staged.plan.changed_ranges));
    detail.insert("edits".into(), staged.plan.edits.iter().map(|e| format!("{}:{:?}", e.entry_index, e.placement)).collect::<Vec<_>>().join(";"));
    detail.insert("notes".into(), staged.plan.notes.join(" | "));
    let big = Artifact {
        key: key.into(),
        destination: destination.into(),
        expected_prior: Expected::Present(FileIdentity { len: staged.bound.identity.len, sha256: stage::hex(&staged.bound.identity.sha256) }),
        replacement: staged.output_path.clone(),
        replacement_identity: FileIdentity { len: fresh.output_len, sha256: stage::hex(&fresh.output_sha256) },
        validation: Validation { producer: "tdb-archive-big::big4".into(), structurally_verified: true, deploy_blockers: vec![], qualification: qualification.clone(), detail: detail.clone() },
        dependencies: vec![],
    };
    let bh = Artifact {
        key: format!("{key}.bh"),
        destination: sidecar_destination.into(),
        expected_prior: Expected::Present(FileIdentity { len: staged.bound.identity.sidecar_len, sha256: stage::hex(&staged.bound.identity.sidecar_sha256) }),
        replacement: staged.sidecar_output_path.clone(),
        replacement_identity: FileIdentity { len: sidecar_out.len() as u64, sha256: stage::hex(&fresh.output_sidecar_sha256) },
        validation: Validation { producer: "tdb-archive-big::big4".into(), structurally_verified: true, deploy_blockers: vec![], qualification, detail: BTreeMap::from([("role".to_string(), "Viv4 sidecar index of the archive artifact".to_string())]) },
        dependencies: vec![key.into()],
    };
    Ok((big, bh))
}

/// An edit session's packaged bytes as a transaction artifact: serialized
/// through [`EditSession::save`] into `staging_dir`, re-parsed through
/// [`Project::reparse_packaged`] (the same detection and endian check a load
/// performs), and bound to the identity of the file the session was
/// **loaded from**. The destination must still be that file; a destination
/// edited since the session opened is a conflict here, never a new baseline.
pub fn session_artifact(
    key: &str,
    destination: &str,
    destination_path: &Path,
    session: &EditSession,
    staging_dir: &Path,
) -> Result<Artifact> {
    check_key(key)?;
    let Some(loaded) = session.project().loaded_identity().cloned() else {
        return Err(DomainError::Validation(format!(
            "session `{key}`: the project was not loaded from a file, so it has no baseline identity"
        )));
    };
    let current = FileIdentity::of_file(destination_path).map_err(io(destination_path))?;
    if current != loaded {
        return Err(DomainError::Validation(format!(
            "session `{key}`: destination {} is {current}, but the session was opened on {loaded}; it changed since — reopen before saving",
            destination_path.display()
        )));
    }
    let bytes = session.save()?;
    let platform = session.project().platform;
    let reparsed = Project::reparse_packaged(bytes.clone(), platform)?;
    let staged = stage_bytes(staging_dir, key, ".packaged", &bytes)?;
    let identity = FileIdentity::of_file(&staged).map_err(io(&staged))?;
    if identity != FileIdentity::of_bytes(&bytes) {
        return Err(DomainError::Validation(format!(
            "session `{key}`: staged file does not hold the packaged bytes"
        )));
    }
    let mut detail = BTreeMap::new();
    detail.insert("platform".into(), platform.as_str().to_owned());
    detail.insert("role".into(), format!("{:?}", session.project().role));
    detail.insert("inner_len".into(), reparsed.inner_len.to_string());
    detail.insert("table_count".into(), reparsed.table_count.to_string());
    detail.insert("dirty".into(), session.is_dirty().to_string());
    Ok(Artifact {
        key: key.into(),
        destination: destination.into(),
        expected_prior: Expected::Present(loaded),
        replacement: staged,
        replacement_identity: identity,
        validation: Validation {
            producer: "studio-domain::session".into(),
            structurally_verified: true,
            deploy_blockers: vec![],
            qualification: Qualification {
                state: "unqualified".into(),
                items: vec![QualificationItem {
                    subject: "native acceptance of this saved database/save".into(),
                    status: "unresolved".into(),
                    note: format!(
                        "re-parses through the loader ({} tables, {} inner bytes); no game-verified load of this file on {} is recorded",
                        reparsed.table_count,
                        reparsed.inner_len,
                        platform.as_str()
                    ),
                }],
            },
            detail,
        },
        dependencies: vec![],
    })
}

/// A new companion file (expected absent at the destination), written into
/// `staging_dir` as a new file and bound to its bytes.
pub fn companion_artifact(
    key: &str,
    destination: &str,
    bytes: &[u8],
    staging_dir: &Path,
    dependencies: Vec<String>,
) -> Result<Artifact> {
    let staged = stage_bytes(staging_dir, key, ".companion", bytes)?;
    let identity = FileIdentity::of_file(&staged).map_err(io(&staged))?;
    Ok(Artifact {
        key: key.into(),
        destination: destination.into(),
        expected_prior: Expected::Absent,
        replacement: staged,
        replacement_identity: identity,
        validation: Validation {
            producer: "studio-domain::companion".into(),
            structurally_verified: true,
            deploy_blockers: vec![],
            qualification: Qualification {
                state: "unqualified".into(),
                items: vec![QualificationItem {
                    subject: "companion file acceptance".into(),
                    status: "unresolved".into(),
                    note: "a new file beside the target; whether the game reads or tolerates this file is not established".into(),
                }],
            },
            detail: BTreeMap::new(),
        },
        dependencies,
    })
}

/// Receipt 67 (C37): an **existing** loose file replaced by `bytes` — the destination is expected to be exactly `prior` (the
/// identity the reviewed plan bound), the replacement is staged into `staging_dir` as a new file and bound to its bytes. The
/// engine backs the prior up, installs, and recovers or restores it like every other artifact of the change set.
pub fn loose_file_artifact(key: &str, destination: &str, prior: FileIdentity, bytes: &[u8], staging_dir: &Path, dependencies: Vec<String>, note: &str) -> Result<Artifact> {
    let staged = stage_bytes(staging_dir, key, ".loose", bytes)?;
    let identity = FileIdentity::of_file(&staged).map_err(io(&staged))?;
    if identity != FileIdentity::of_bytes(bytes) {
        return Err(DomainError::Validation(format!("loose `{key}`: the staged file does not hold the planned bytes")));
    }
    Ok(Artifact {
        key: key.into(),
        destination: destination.into(),
        expected_prior: Expected::Present(prior),
        replacement: staged,
        replacement_identity: identity,
        validation: Validation {
            producer: "studio-domain::board_replace".into(),
            structurally_verified: true,
            deploy_blockers: vec![],
            qualification: Qualification {
                state: "unqualified".into(),
                items: vec![QualificationItem { subject: "native acceptance of the replaced loose file".into(), status: "unresolved".into(), note: note.into() }],
            },
            detail: BTreeMap::new(),
        },
        dependencies,
    })
}

/// The attached dictionary with the pending LOC-W1 rename applied, as a transaction artifact.
/// The rename is **re-applied here** from the baseline bytes and must reproduce the held output
/// exactly (the plan is re-derived, the CRC scopes re-verified, the allowed differences re-audited
/// by `tdb-loc`); the destination is expected to be exactly the attached baseline.
pub fn dictionary_artifact(
    key: &str,
    destination: &str,
    baseline: &crate::leagues::AttachedDictionary,
    edit: &crate::dictionary_edit::DictionaryEdit,
    staging_dir: &Path,
) -> Result<Artifact> {
    check_key(key)?;
    let (fresh_bytes, changed_ranges, crc_scopes_changed) = edit
        .plan
        .reapply(&baseline.bytes)
        .map_err(|e| DomainError::Validation(format!("dictionary `{key}`: {e}")))?;
    if fresh_bytes != *edit.output {
        return Err(DomainError::Validation(format!(
            "dictionary `{key}`: re-applying the dictionary edit does not reproduce the held output"
        )));
    }
    let staged = stage_bytes(staging_dir, key, ".dictionary", &fresh_bytes)?;
    let identity = FileIdentity::of_file(&staged).map_err(io(&staged))?;
    if identity != edit.output_identity {
        return Err(DomainError::Validation(format!(
            "dictionary `{key}`: staged file does not hold the edit output"
        )));
    }
    let mut detail = BTreeMap::new();
    for (k, v) in edit.plan.detail() {
        detail.insert(k, v);
    }
    detail.insert("changed_ranges".into(), format!("{:?}", changed_ranges));
    detail.insert("crc_scopes_changed".into(), crc_scopes_changed.join(","));
    let (source_key, text_after) = (edit.plan.source_key().to_string(), edit.plan.text_after());
    Ok(Artifact {
        key: key.into(),
        destination: destination.into(),
        expected_prior: Expected::Present(FileIdentity {
            len: baseline.bytes.len() as u64,
            sha256: baseline.sha256.clone(),
        }),
        replacement: staged,
        replacement_identity: identity,
        validation: Validation {
            producer: edit.plan.producer().into(),
            structurally_verified: true,
            deploy_blockers: vec![],
            qualification: Qualification {
                state: "unqualified".into(),
                items: vec![
                    QualificationItem {
                        subject: "dictionary consumers of the key".into(),
                        status: "unresolved".into(),
                        note: format!(
                            "the text under key {:?} now reads {:?}; which game screens look this key up is not established by Studio (the league-code -> key binding is traced on the Xbox 360 executables and by reference elsewhere)",
                            source_key, text_after
                        ),
                    },
                    QualificationItem {
                        subject: "native acceptance of the rewritten dictionary".into(),
                        status: "unresolved".into(),
                        note: "every CRC scope re-verifies and every other row is byte-identical; no game-verified load of a rewritten dictionary is recorded on any title".into(),
                    },
                ],
            },
            detail,
        },
        dependencies: vec![],
    })
}

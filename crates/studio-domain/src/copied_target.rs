//! S2-C — a [`Workspace`] as one copied-target change set: the primary session, the attached
//! roster session, and every content-header sidecar those files newly require, packaged through
//! the existing serializers and the S2-B adapters into artifacts the shared `tdb-txn` engine
//! applies, recovers and restores as a unit.
//!
//! This is the staging sibling of [`Workspace::save_in_place`]. It **reads** the workspace and the
//! copied tree and produces files in a staging directory; nothing here touches a destination —
//! `tdb_txn::prepare` / `tdb_txn::apply` do, later, on the caller's decision. Every session keeps
//! its dirty edits whatever happens here.
//!
//! Rules (from the S2-C dispatch):
//!
//! * Destinations are the sessions' own on-disk paths, and every one of them must lie inside the
//!   explicitly selected, marked copied-target root. A path outside it is a refusal, never an
//!   implicit widening of the root. Each destination must still carry the identity the session was
//!   loaded from (the S2-B session adapter's rule).
//! * A content header the copied tree newly requires ([`crate::content_header::plan_content_header`])
//!   joins the same change set, expected **absent**, depending on the save it indexes. An existing
//!   header is never rewritten. When the planned bytes were cloned from a sibling template, that
//!   template is a **bound input**: its identity is recorded and re-verified before apply.
//! * A header path outside the root is a refusal. A destination whose parent directory does not
//!   exist is refused with the directory named ([`verify_plan_inputs`]); directories are never
//!   created.
//! * Everything produced here is **unqualified** and the change set is `Mode::CopiedTarget`; there
//!   is no live mode.

use std::path::{Component, Path, PathBuf};

use tdb_txn::{Artifact, ChangeSet, Expected, FileIdentity, Mode};

use crate::content_header::{plan_content_header, HeaderPlan};
use crate::error::DomainError;
use crate::txn::{companion_artifact, session_artifact};
use crate::workspace::WorkspaceRevision;
use crate::{Result, Workspace};

/// What a planned artifact is, for the consumer that labels it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactRole {
    /// The workspace's canonical/primary session (the main database, or a roster save opened
    /// alone).
    Primary,
    /// The attached roster mirror.
    Roster,
    /// A content-header sidecar newly required for the artifact `for_key`.
    ContentHeader { for_key: String },
    /// An archive staged through the S2-A stager ([`CopiedTargetPlan::add_archive`]).
    Archive,
    /// The attached localization dictionary with the pending LOC-W1 rename applied.
    Dictionary,
    /// Receipt 67 (C37): the loose half of a PS3 board pair, replaced with its archive member in one change set.
    LooseAsset,
}

/// One artifact of the plan with the context the engine does not carry.
#[derive(Debug, Clone)]
pub struct PlannedArtifact {
    pub role: ArtifactRole,
    /// The destination as an absolute path in the copied tree.
    pub destination_path: PathBuf,
    pub artifact: Artifact,
}

/// A file the plan's bytes were derived from but which is not itself replaced. Re-verified before
/// apply; a change invalidates the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundInput {
    /// `content-header` (an existing sidecar the save relies on, preserved as-is) or
    /// `header-template` (the sibling a new header's bytes were cloned from).
    pub kind: BoundInputKind,
    /// What the input was used for (human-readable).
    pub purpose: String,
    /// Canonical path (inside the plan's root).
    pub path: PathBuf,
    pub identity: FileIdentity,
}

/// Why a file is bound to a plan without being replaced by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundInputKind {
    /// The save's existing index header: its presence is what justified planning no header
    /// artifact, so it must still be exactly there (a regular file, inside the root) at apply.
    ContentHeader,
    /// A same-type sibling header whose bytes a new header was cloned from.
    HeaderTemplate,
}

impl BoundInputKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BoundInputKind::ContentHeader => "content-header",
            BoundInputKind::HeaderTemplate => "header-template",
        }
    }
}

/// A bound input's file as it is on disk right now: a plain regular file (never a symlink, junction
/// or directory), whose canonical location lies inside `root`, with its identity. Anything else is
/// a refusal naming the file and what is wrong with it.
fn regular_file_in_root(root: &Path, path: &Path, what: &str) -> Result<FileIdentity> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| {
        DomainError::Validation(format!(
            "{what} {} cannot be inspected ({e}); it was present when the plan was made",
            shown(path)
        ))
    })?;
    if meta.file_type().is_symlink() {
        return Err(DomainError::Validation(format!(
            "{what} {} is a symlink or junction, not a regular file; a redirected header is not treated as the save's companion",
            shown(path)
        )));
    }
    if !meta.is_file() {
        return Err(DomainError::Validation(format!(
            "{what} {} is not a regular file (a directory in the header's place is not a companion)",
            shown(path)
        )));
    }
    let canon = std::fs::canonicalize(path).map_err(io(path))?;
    if !canon.starts_with(root) {
        return Err(DomainError::Validation(format!(
            "{what} {} resolves to {} outside the selected copied-target root {}",
            shown(path),
            shown(&canon),
            shown(root)
        )));
    }
    FileIdentity::of_file(&canon).map_err(io(&canon))
}

/// D4: the pending text-member rewrites of one archive (the NHL 16–21 `setting_csv` stored-default
/// edits), carried on the plan so the copied-target route stages the archive with them through the
/// same stager and adapter as any other archive artifact. The bytes are the workspace's own held
/// plans (packaged under the workspace lock for this revision), never re-derived at review time.
#[derive(Debug, Clone)]
pub struct TextMemberReplacements {
    /// The archive relative to the install/copied root (`databigs/inxp_00.big`).
    pub archive_relative: String,
    /// The source archive the edits were planned on (for the review's disclosure).
    pub source_archive: PathBuf,
    pub members: Vec<crate::settings_csv_edit::CsvMemberEdit>,
    /// One line per pending edit (the review text the user saw).
    pub descriptions: Vec<String>,
    /// C4: `Some` when the archive is a BIG4 + `.bh` pair (NHL 07–10); the sidecar is staged and
    /// installed with the archive as one change set.
    pub big4: Option<Big4GroupInfo>,
    /// C7: the member paths whose replacement was chosen as **whole group** (every row of the
    /// deduplicated block follows); every other shared member takes a private copy.
    pub whole_group: Vec<String>,
    /// Receipt 67 (C37): the loose halves of PS3 board pairs in this group, written with the archive in the same change set.
    pub loose: Vec<crate::asset_member_edit::PairedLoose>,
    /// Receipt 67 (C37): root-relative loose paths that must stay absent (archive-only packages).
    pub absent_loose: Vec<String>,
}

/// The BIG4 facts of a texture-member group (see `asset_member_edit::Big4MemberInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Big4GroupInfo {
    pub sidecar_relative: String,
    /// `member_path → .bh decoded-length word` for every member of the group.
    pub decoded_len: std::collections::BTreeMap<String, u32>,
    /// The placement each member's review predicted (`fit` / `reflow` / `append`), by member path.
    pub placement: std::collections::BTreeMap<String, crate::asset_member_edit::Big4Placement>,
    /// NHL 07: the boot.big cascade bound by the members' reviews (every member of the group
    /// binds the same one; a differing binding is refused at planning).
    pub cascade: Option<crate::asset_member_edit::Big4Cascade>,
    pub growth_refusal: Option<String>,
}

/// A staged workspace, ready for `tdb_txn::prepare`.
#[derive(Debug, Clone)]
pub struct CopiedTargetPlan {
    /// The canonical copied-target root every destination lies under.
    pub root: PathBuf,
    /// The workspace content revision the packaged bytes came from.
    pub revision: WorkspaceRevision,
    pub artifacts: Vec<PlannedArtifact>,
    pub bound_inputs: Vec<BoundInput>,
    pub change_set: ChangeSet,
    /// D4: pending text-member rewrites the route must stage into the copied archive (empty when
    /// no CSV stored-default edit is pending).
    pub text_members: Option<TextMemberReplacements>,
    /// D8: pending texture-member replacements, one group per archive (`databigs/inxp_10.big`
    /// …), staged by the route under the private-copy rule for shared spans.
    pub asset_members: Vec<TextMemberReplacements>,
    /// Receipt 67 (C37): paths in the copied tree that must still be absent at apply (an archive-only package's loose name).
    pub absent_paths: Vec<PathBuf>,
}

impl CopiedTargetPlan {
    pub fn artifact(&self, key: &str) -> Option<&PlannedArtifact> {
        self.artifacts.iter().find(|a| a.artifact.key == key)
    }

    /// Add an archive staged through the controlled S2-A adapter ([`crate::txn::StagedArchive`],
    /// which only [`crate::txn::stage_archive`] can make) to this plan, so it is prepared, applied,
    /// recovered and restored by the same service as the sessions and headers — there is no
    /// separate archive installer. `destination_path` is the archive as it sits in the copied tree
    /// and must lie inside the plan's root; the S2-B adapter re-runs the bound verification and
    /// binds the destination to the archive the plan was made from. The change set (and its id) is
    /// rebuilt.
    pub fn add_archive(
        &mut self,
        key: &str,
        destination_path: &Path,
        staged: &crate::txn::StagedArchive,
    ) -> Result<()> {
        let (canon, rel) = root_relative(&self.root, destination_path, "the archive")?;
        let artifact = crate::txn::archive_artifact(key, &rel, staged)?;
        self.artifacts.push(PlannedArtifact {
            role: ArtifactRole::Archive,
            destination_path: canon,
            artifact,
        });
        self.change_set = ChangeSet::new(
            self.change_set.title.clone(),
            Mode::CopiedTarget,
            self.artifacts.iter().map(|a| a.artifact.clone()).collect(),
        );
        verify_plan_inputs(self)
    }

    /// Receipt 67 (C37): add the loose half of a PS3 board pair — an existing file in the copied tree replaced by `bytes`,
    /// expected to be exactly `prior` — as one more artifact of the same change set as its archive (`depends_on`). The change set
    /// is rebuilt and the inputs re-verified.
    #[allow(clippy::too_many_arguments)]
    pub fn add_loose(&mut self, key: &str, destination_path: &Path, prior: FileIdentity, bytes: &[u8], staging_dir: &Path, depends_on: &str) -> Result<()> {
        let (canon, rel) = root_relative(&self.root, destination_path, "the loose package")?;
        let artifact = crate::txn::loose_file_artifact(
            key,
            &rel,
            prior,
            bytes,
            staging_dir,
            vec![depends_on.to_string()],
            "the loose copy of a PS3 board pair, replaced with its archive member; which copy the console loads is not established",
        )?;
        self.artifacts.push(PlannedArtifact { role: ArtifactRole::LooseAsset, destination_path: canon, artifact });
        self.change_set = ChangeSet::new(self.change_set.title.clone(), Mode::CopiedTarget, self.artifacts.iter().map(|a| a.artifact.clone()).collect());
        verify_plan_inputs(self)
    }

    /// C4: add a staged BIG4 archive **and its `.bh` sidecar** as two artifacts (`<key>` and
    /// `<key>.bh`) of one change set — the pair is installed, recovered and restored together.
    pub fn add_big4(
        &mut self,
        key: &str,
        archive_destination: &Path,
        sidecar_destination: &Path,
        staged: &crate::txn::StagedBig4,
    ) -> Result<()> {
        let (canon_big, rel_big) = root_relative(&self.root, archive_destination, "the archive")?;
        let (canon_bh, rel_bh) = root_relative(&self.root, sidecar_destination, "the sidecar")?;
        let (a_big, a_bh) = crate::txn::big4_artifacts(key, &rel_big, &rel_bh, staged)?;
        self.artifacts.push(PlannedArtifact { role: ArtifactRole::Archive, destination_path: canon_big, artifact: a_big });
        self.artifacts.push(PlannedArtifact { role: ArtifactRole::Archive, destination_path: canon_bh, artifact: a_bh });
        self.change_set = ChangeSet::new(
            self.change_set.title.clone(),
            Mode::CopiedTarget,
            self.artifacts.iter().map(|a| a.artifact.clone()).collect(),
        );
        verify_plan_inputs(self)
    }
}

/// Artifact keys used by the plan (stable; the consumer maps them to labels).
pub const KEY_PRIMARY: &str = "primary";
pub const KEY_ROSTER: &str = "roster";
pub const KEY_DICTIONARY: &str = "dictionary";

fn header_key(for_key: &str) -> String {
    format!("{for_key}.header")
}

/// A path in a message for the user: the verbatim prefix (`\?\`) Windows' `canonicalize` adds is
/// stripped; nothing else changes.
fn shown(p: &Path) -> String {
    let s = p.display().to_string();
    match s.strip_prefix(r"\\?\UNC\") {
        Some(rest) => format!(r"\\{rest}"),
        None => s.strip_prefix(r"\\?\").map(str::to_owned).unwrap_or(s),
    }
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> DomainError + '_ {
    move |source| DomainError::Io {
        path: shown(path).to_string(),
        source,
    }
}

/// `path` relative to the canonical `root`, as the engine wants it (forward slashes, plain
/// components), or a refusal naming both when the path is not inside the root.
fn root_relative(root: &Path, path: &Path, what: &str) -> Result<(PathBuf, String)> {
    let canon = std::fs::canonicalize(path).map_err(io(path))?;
    let rel = canon.strip_prefix(root).map_err(|_| {
        DomainError::Validation(format!(
            "{what} {} is not inside the selected copied-target root {}; the root is never widened implicitly — select the root that contains it, or open the workspace from files inside the root",
            shown(path),
            shown(root)
        ))
    })?;
    let mut parts = Vec::new();
    for c in rel.components() {
        match c {
            Component::Normal(n) => parts.push(n.to_string_lossy().into_owned()),
            Component::CurDir => {}
            other => {
                return Err(DomainError::Validation(format!(
                    "{what} {} resolves through an unexpected path component {other:?}",
                    shown(path)
                )))
            }
        }
    }
    if parts.is_empty() {
        return Err(DomainError::Validation(format!(
            "{what} {} is the root itself, not a file in it",
            shown(path)
        )));
    }
    Ok((canon, parts.join("/")))
}

/// Stage `ws` for the copied-target root `root`: package every session through the S2-B session
/// adapter (serialize → re-parse → bind to the loaded identity), plan the content headers the
/// destinations newly require, and assemble one change set. Staging files are created as new files
/// in `staging_dir`, which the caller owns and empties afterwards.
pub fn stage_workspace(
    ws: &Workspace,
    root: &Path,
    staging_dir: &Path,
    title: &str,
) -> Result<CopiedTargetPlan> {
    if let Some(reason) = ws.stale_baseline() {
        return Err(DomainError::Validation(format!(
            "this workspace cannot be saved to the copied target: {reason}"
        )));
    }
    let root = tdb_txn::copied_target_root(root).map_err(|e| {
        DomainError::Validation(format!(
            "{} is not a copied-target root: {e}. Copied-target saves only go to a root created and marked for that purpose; live game trees are never written by this path",
            shown(root)
        ))
    })?;
    let mut artifacts: Vec<PlannedArtifact> = Vec::new();
    let mut bound_inputs: Vec<BoundInput> = Vec::new();

    let mut sessions = vec![(
        KEY_PRIMARY,
        ArtifactRole::Primary,
        ws.primary(),
        PathBuf::from(ws.primary_path()),
    )];
    if let (Some(session), Some(path)) = (ws.roster_session(), ws.roster_path()) {
        sessions.push((
            KEY_ROSTER,
            ArtifactRole::Roster,
            session,
            PathBuf::from(path),
        ));
    }

    for (key, role, session, path) in sessions {
        let what = match role {
            ArtifactRole::Primary => "the workspace's main file",
            ArtifactRole::Roster => "the attached roster save",
            ArtifactRole::ContentHeader { .. }
            | ArtifactRole::Archive
            | ArtifactRole::Dictionary
            | ArtifactRole::LooseAsset => unreachable!(),
        };
        let (canon, rel) = root_relative(&root, &path, what)?;
        let artifact = session_artifact(key, &rel, &canon, session, staging_dir)?;
        artifacts.push(PlannedArtifact {
            role: role.clone(),
            destination_path: canon.clone(),
            artifact,
        });

        // The header companion, planned on the destination as it sits in the copied tree.
        match plan_content_header(&canon)? {
            HeaderPlan::NotIndexed => {}
            HeaderPlan::Present(header_path) => {
                // The existing header is the reason no header artifact is planned: bind it. It
                // must be a regular file inside the root now and still be exactly this at apply;
                // it is never rewritten or regenerated.
                let what = format!(
                    "the existing save index header of {}",
                    canon.file_name().unwrap_or_default().to_string_lossy()
                );
                let identity = regular_file_in_root(&root, &header_path, &what)?;
                let canon_header = std::fs::canonicalize(&header_path).map_err(io(&header_path))?;
                bound_inputs.push(BoundInput {
                    kind: BoundInputKind::ContentHeader,
                    purpose: format!(
                        "existing save index header for {} (preserved as-is; no header is written because it is present)",
                        canon.file_name().unwrap_or_default().to_string_lossy()
                    ),
                    path: canon_header,
                    identity,
                });
            }
            HeaderPlan::Required {
                header_path,
                bytes,
                template,
            } => {
                // Containment of the header path is decided here, on the plan, before any
                // staging file for it exists: outside the root is a refusal.
                let parent = header_path
                    .parent()
                    .ok_or_else(|| DomainError::Validation("header path has no parent".into()))?;
                let parent_canon = std::fs::canonicalize(parent).map_err(io(parent))?;
                if !parent_canon.starts_with(&root) {
                    return Err(DomainError::Validation(format!(
                        "the save index header {} that {} requires would be written outside the selected copied-target root {}; the root is never widened implicitly. Select the root that contains the whole content tree",
                        shown(&header_path),
                        shown(&canon),
                        shown(&root)
                    )));
                }
                let header_canon =
                    parent_canon.join(header_path.file_name().ok_or_else(|| {
                        DomainError::Validation("header path has no name".into())
                    })?);
                let rel = header_canon
                    .strip_prefix(&root)
                    .map(|r| {
                        r.components()
                            .map(|c| c.as_os_str().to_string_lossy().into_owned())
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .expect("checked above");
                let hkey = header_key(key);
                let mut artifact =
                    companion_artifact(&hkey, &rel, &bytes, staging_dir, vec![key.to_owned()])?;
                artifact.validation.producer = "studio-domain::content_header".into();
                artifact.validation.detail.insert(
                    "indexes".into(),
                    canon
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                );
                artifact.validation.qualification.items[0].subject =
                    "save index header acceptance".into();
                artifact.validation.qualification.items[0].note = match &template {
                    Some(t) => format!(
                        "cloned from the same-type sidecar {} with the content name patched; the layout is verified against game-written headers, this specific file is not game-verified",
                        shown(&t.path)
                    ),
                    None => "synthesized from the verified 0x148-byte layout (no same-type sidecar to clone); this specific file is not game-verified".into(),
                };
                if let Some(t) = template {
                    artifact
                        .validation
                        .detail
                        .insert("template".into(), shown(&t.path).to_string());
                    artifact
                        .validation
                        .detail
                        .insert("template_identity".into(), t.identity.to_string());
                    let template_path = std::fs::canonicalize(&t.path).map_err(io(&t.path))?;
                    bound_inputs.push(BoundInput {
                        kind: BoundInputKind::HeaderTemplate,
                        purpose: format!("template for the save index header {rel}"),
                        path: template_path,
                        identity: t.identity,
                    });
                }
                artifacts.push(PlannedArtifact {
                    role: ArtifactRole::ContentHeader {
                        for_key: key.to_owned(),
                    },
                    destination_path: header_canon,
                    artifact,
                });
            }
        }
    }

    // The pending dictionary rename (LOC-W1), if any: the attached file is the destination, its
    // attached identity the expected prior, the verified output the replacement.
    if let (Some(edit), Some(base)) = (ws.dictionary_edit(), ws.dictionary()) {
        let path = PathBuf::from(&base.path);
        let (canon, rel) = root_relative(&root, &path, "the attached localization dictionary")?;
        let artifact = crate::txn::dictionary_artifact(KEY_DICTIONARY, &rel, base, edit, staging_dir)?;
        artifacts.push(PlannedArtifact {
            role: ArtifactRole::Dictionary,
            destination_path: canon,
            artifact,
        });
    }

    // D4: the pending NHL 16–21 catalogue member rewrites, if any — the archive itself is bound and
    // staged by the route (it needs the copied root's archive and the stager); the plan carries
    // exactly the bytes the held reviews produced.
    let text_members = ws.settings_csv_working_set().filter(|w| !w.edits.is_empty()).map(|w| TextMemberReplacements {
        archive_relative: "databigs/inxp_00.big".to_string(),
        source_archive: w.archive_path.clone(),
        members: w.replacements(),
        descriptions: w.edits.iter().map(|e| e.description.clone()).collect(),
        big4: None,
        whole_group: Vec::new(),
        loose: Vec::new(),
        absent_loose: Vec::new(),
    });

    // D8: the pending texture-member replacements, grouped per archive in first-seen order.
    let mut asset_members: Vec<TextMemberReplacements> = Vec::new();
    for m in ws.asset_member_edits() {
        if let Some(g) = asset_members.iter_mut().find(|g| g.archive_relative == m.archive_relative) {
            g.members.push(m.member.clone());
            g.descriptions.push(m.description.clone());
            if let (Some(gi), Some(mi)) = (g.big4.as_mut(), m.big4.as_ref()) {
                gi.decoded_len.insert(m.member_path.clone(), mi.decoded_len);
                gi.placement.insert(m.member_path.clone(), mi.placement);
                if gi.cascade.is_none() {
                    gi.cascade = mi.cascade.clone();
                }
            }
            if m.whole_group {
                g.whole_group.push(m.member_path.clone());
            }
            g.loose.extend(m.paired_loose.clone());
            g.absent_loose.extend(m.loose_must_be_absent.clone());
        } else {
            asset_members.push(TextMemberReplacements {
                archive_relative: m.archive_relative.clone(),
                source_archive: m.archive_path.clone(),
                members: vec![m.member.clone()],
                descriptions: vec![m.description.clone()],
                big4: m.big4.as_ref().map(|b| Big4GroupInfo {
                    sidecar_relative: b.sidecar_relative.clone(),
                    decoded_len: std::collections::BTreeMap::from([(m.member_path.clone(), b.decoded_len)]),
                    placement: std::collections::BTreeMap::from([(m.member_path.clone(), b.placement)]),
                    cascade: b.cascade.clone(),
                    growth_refusal: b.growth_refusal.clone(),
                }),
                whole_group: if m.whole_group { vec![m.member_path.clone()] } else { Vec::new() },
                loose: m.paired_loose.clone().into_iter().collect(),
                absent_loose: m.loose_must_be_absent.clone().into_iter().collect(),
            });
        }
    }

    let change_set = ChangeSet::new(
        title,
        Mode::CopiedTarget,
        artifacts.iter().map(|a| a.artifact.clone()).collect(),
    );
    let plan = CopiedTargetPlan {
        root,
        revision: ws.revision(),
        artifacts,
        bound_inputs,
        change_set,
        text_members,
        asset_members,
        absent_paths: Vec::new(),
    };
    verify_plan_inputs(&plan)?;
    Ok(plan)
}

/// D4: the member replacements a plan's pending text-member rewrites need on the copied root's
/// archive, as [`tdb_archive_big::stage::Replacement`]s bound to the archive's current entries.
/// Refuses — before anything is staged — a member whose stored bytes in the copied archive are not
/// the ones the edit was planned on (the copy differs from the source), a member whose new stored
/// bytes do not decode to the held plain text, and a shared-span entry.
pub fn text_member_replacements(
    bound: &tdb_archive_big::stage::BoundSource,
    text: &TextMemberReplacements,
) -> Result<Vec<tdb_archive_big::stage::Replacement>> {
    member_replacements(bound, text, false)
}

/// [`text_member_replacements`] with the shared-span rule chosen by the caller: `allow_shared`
/// lets a deduplicated member through (the stager then takes a private copy under
/// [`tdb_archive_big::stage::SharedSpanRule::PrivateCopy`]); the D4 text route keeps refusing.
pub fn member_replacements(
    bound: &tdb_archive_big::stage::BoundSource,
    text: &TextMemberReplacements,
    allow_shared: bool,
) -> Result<Vec<tdb_archive_big::stage::Replacement>> {
    use tdb_archive_big::stage::{ExpectedDecoded, Replacement, StoredForm};
    let mut out = Vec::new();
    for m in &text.members {
        let Some(entry) = bound.entries.iter().find(|e| e.path == m.member_path) else {
            return Err(DomainError::Validation(format!("the copied archive has no member {:?}", m.member_path)));
        };
        if entry.shared_span && !allow_shared {
            return Err(DomainError::Validation(format!("member {:?} shares its stored bytes with another entry; it cannot be replaced unambiguously", m.member_path)));
        }
        let stored_hex = tdb_archive_big::stage::hex(&entry.stored_sha256);
        if let Some(expected) = &m.source_stored_sha256 {
            if *expected != stored_hex {
                return Err(DomainError::Validation(format!(
                    "member {:?} in the copied archive ({stored_hex}) is not the member the edit was planned on ({expected}); the copy differs from the opened install — plan the edit on this copy",
                    m.member_path
                )));
            }
        }
        let expected_form = match m.stored_form.as_str() {
            "chunkzip" => StoredForm::ChunkZip,
            "chunklzx-store" => StoredForm::ChunkLzx,
            _ => StoredForm::Raw,
        };
        if entry.stored_form != expected_form {
            return Err(DomainError::Validation(format!("member {:?} is stored as {:?} in the copied archive, the edit was planned for {:?}", m.member_path, entry.stored_form, expected_form)));
        }
        // Independent decode of the held stored bytes must give the held plain text.
        let decoded = if m.stored_form == "chunkzip" {
            tdb_archive_codec::decompress_chunkzip(&m.new_stored).map_err(|e| DomainError::Validation(format!("member {:?}: the held re-encoded bytes do not decode: {e}", m.member_path)))?
        } else if m.stored_form == "chunklzx-store" {
            tdb_archive_codec::decompress_chunklzx(&m.new_stored).map_err(|e| DomainError::Validation(format!("member {:?}: the held store-encoded bytes do not decode: {e}", m.member_path)))?
        } else {
            (*m.new_stored).clone()
        };
        if decoded != *m.new_plain {
            return Err(DomainError::Validation(format!("member {:?}: the held re-encoded bytes decode to something other than the held text", m.member_path)));
        }
        out.push(Replacement {
            entry_index: entry.index,
            expected_path: entry.path.clone(),
            expected_stored_sha256: entry.stored_sha256,
            new_stored: (*m.new_stored).clone(),
            expected_decoded: ExpectedDecoded::of(&m.new_plain),
        });
    }
    Ok(out)
}

/// C4: the BIG4 replacements of a group against the bound archive + sidecar: every member must
/// exist with the stored identity the edit was planned on (a differing copy refuses), the held
/// stored bytes must decode to the held plain bytes (RefPack) or be them (raw), and the `.bh`
/// decoded-length word comes from the group.
pub fn big4_member_replacements(
    bound: &tdb_archive_big::big4::BoundBig4,
    group: &TextMemberReplacements,
) -> Result<Vec<tdb_archive_big::big4::Big4Replacement>> {
    use tdb_archive_big::big4::Big4Replacement;
    let info = group.big4.as_ref().ok_or_else(|| DomainError::Validation("not a BIG4 group".into()))?;
    let mut out = Vec::new();
    for m in &group.members {
        let Some(entry) = bound.entries.iter().find(|e| e.path.eq_ignore_ascii_case(&m.member_path)) else {
            return Err(DomainError::Validation(format!("the copied archive has no member {:?}", m.member_path)));
        };
        let expected_hex = m.source_stored_sha256.as_deref().ok_or_else(|| DomainError::Validation(format!("member {:?} carries no source identity", m.member_path)))?;
        let mut expected = [0u8; 32];
        for (i, b) in expected.iter_mut().enumerate() {
            *b = u8::from_str_radix(&expected_hex[i * 2..i * 2 + 2], 16).map_err(|_| DomainError::Validation("malformed stored identity".into()))?;
        }
        let decoded = if m.stored_form == "refpack" {
            tdb_archive_codec::refpack_decompress(&m.new_stored).map_err(|e| DomainError::Validation(format!("member {:?}: the held re-wrapped bytes do not decode: {e}", m.member_path)))?
        } else {
            (*m.new_stored).clone()
        };
        if decoded != *m.new_plain {
            return Err(DomainError::Validation(format!("member {:?}: the held stored bytes decode to something other than the held member", m.member_path)));
        }
        let decoded_len = *info.decoded_len.get(&m.member_path).ok_or_else(|| DomainError::Validation(format!("member {:?}: no `.bh` decoded length was planned", m.member_path)))?;
        // The apply does exactly what the review predicted: a member previewed as fitting is
        // refused if the copy no longer fits it; reflow / append as previewed.
        let growth = match info.placement.get(&m.member_path) {
            Some(crate::asset_member_edit::Big4Placement::Reflow) => tdb_archive_big::big4::Growth::Reflow,
            Some(crate::asset_member_edit::Big4Placement::Append) => tdb_archive_big::big4::Growth::Append,
            Some(crate::asset_member_edit::Big4Placement::Fit) | None => tdb_archive_big::big4::Growth::Refuse(format!("member {:?} was previewed as fitting its slot; the copied archive no longer fits it — preview again", m.member_path)),
        };
        out.push(Big4Replacement {
            entry_index: entry.index,
            expected_path: entry.path.clone(),
            expected_stored_sha256: expected,
            new_stored: (*m.new_stored).clone(),
            decoded_len,
            growth: Some(growth),
        });
    }
    Ok(out)
}

/// Re-verify everything the plan assumes about the copied tree **now**: every destination still
/// holds its expected prior (or is still absent), every destination's parent directory exists, and
/// every bound input still carries its recorded identity. Run at staging and again immediately
/// before apply; a refusal names the file and what changed, and nothing has been mutated.
pub fn verify_plan_inputs(plan: &CopiedTargetPlan) -> Result<()> {
    match verify_plan_inputs_unless(plan, &|| false)? {
        PlanVerification::Verified => Ok(()),
        PlanVerification::Cancelled => unreachable!("never cancelled"),
    }
}

/// The cheap half of [`verify_plan_inputs`]: every destination's parent directory exists (this
/// package never creates directories). No file is opened or hashed, so a caller may run it under
/// a lock it must not hold for long, ahead of the identity checks.
pub fn verify_plan_layout(plan: &CopiedTargetPlan) -> Result<()> {
    for a in &plan.artifacts {
        let dest = &a.destination_path;
        let Some(parent) = dest.parent() else {
            return Err(DomainError::Validation(format!(
                "destination {} has no parent directory",
                shown(dest)
            )));
        };
        if !parent.is_dir() {
            return Err(DomainError::Validation(format!(
                "the directory {} that must hold {} does not exist in the copied target; this package does not create directories — provision the copied tree's directory structure first",
                shown(parent),
                dest.file_name().unwrap_or_default().to_string_lossy()
            )));
        }
    }
    Ok(())
}

/// The result of [`verify_plan_inputs_unless`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanVerification {
    /// Every input was checked and holds.
    Verified,
    /// `cancelled` answered true between two files; the remaining inputs were **not** checked.
    /// Nothing was mutated. The caller must not treat the plan as verified.
    Cancelled,
}

/// [`verify_plan_inputs`] that polls `cancelled` before each file is hashed (LEAGUE-RESP-1): on a
/// real PS4 archive the destination prior and the staged output are ~777 MB each, so a
/// cancellation requested during the apply's preflight is honoured within one file's hash instead
/// of after the whole verification. A refusal is still a refusal; `Ok(Cancelled)` is neither.
pub fn verify_plan_inputs_unless(
    plan: &CopiedTargetPlan,
    cancelled: &dyn Fn() -> bool,
) -> Result<PlanVerification> {
    verify_plan_layout(plan)?;
    for a in &plan.artifacts {
        if cancelled() {
            return Ok(PlanVerification::Cancelled);
        }
        let dest = &a.destination_path;
        let actual = FileIdentity::of_path_or_absent(dest).map_err(io(dest))?;
        match (&a.artifact.expected_prior, actual) {
            (Expected::Absent, None) => {}
            (Expected::Absent, Some(x)) => {
                return Err(DomainError::Validation(format!(
                    "{} appeared on disk ({x}) after it was planned as a new file; re-prepare so the plan reflects the tree as it is",
                    shown(dest)
                )))
            }
            (Expected::Present(p), Some(x)) if *p == x => {}
            (Expected::Present(p), other) => {
                return Err(DomainError::Validation(format!(
                    "{} is {} but the session was opened on {p}; it changed on disk after the session opened — reload or re-prepare, the edits are kept",
                    shown(dest),
                    other.map(|i| i.to_string()).unwrap_or_else(|| "absent".into())
                )))
            }
        }
        if cancelled() {
            return Ok(PlanVerification::Cancelled);
        }
        let rep = FileIdentity::of_path_or_absent(&a.artifact.replacement)
            .map_err(io(&a.artifact.replacement))?;
        if rep.as_ref() != Some(&a.artifact.replacement_identity) {
            return Err(DomainError::Validation(format!(
                "the staged output for {} ({}) is {}; it must be exactly {} — re-prepare",
                a.artifact.key,
                shown(&a.artifact.replacement),
                rep.map(|i| i.to_string())
                    .unwrap_or_else(|| "missing".into()),
                a.artifact.replacement_identity
            )));
        }
    }
    for b in &plan.bound_inputs {
        if cancelled() {
            return Ok(PlanVerification::Cancelled);
        }
        let what = format!("{} ({})", b.kind.as_str(), b.purpose);
        let actual = match std::fs::symlink_metadata(&b.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(DomainError::Validation(format!(
                    "{what}: {} is gone; it was present when the plan was made and the plan relies on it — re-prepare (the file is not regenerated silently)",
                    shown(&b.path)
                )))
            }
            _ => regular_file_in_root(&plan.root, &b.path, &what)?,
        };
        if actual != b.identity {
            return Err(DomainError::Validation(format!(
                "{what}: {} is {actual} but the plan was derived from {}; it changed after planning — re-prepare",
                shown(&b.path),
                b.identity
            )));
        }
    }
    // Receipt 67 (C37): an archive-only board package must not have acquired a loose copy since it was planned.
    for p in &plan.absent_paths {
        if std::fs::symlink_metadata(p).is_ok() {
            return Err(DomainError::Validation(format!(
                "{} appeared after the plan was made; this title ships the package only inside its archive, and a loose copy is another topology — nothing was written, plan again",
                shown(p)
            )));
        }
    }
    Ok(PlanVerification::Verified)
}

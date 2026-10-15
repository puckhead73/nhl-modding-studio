//! Checked restore and recovery of a recorded installation or deployment (R71-R3).
//!
//! One record (an installation under the app's `installs/`, a deployment under the project's
//! `deployments/`) names every destination it touched with the identity it held before and the
//! identity it was given. Restoring is the same operation whatever the record's state: every
//! destination is read and classified **by its current identity** — holding the installed bytes
//! (put the prior back, or remove a created sidecar), holding the prior bytes or absent as it was
//! (already restored, left alone), missing with a verified original backup (recreate it), or
//! different present bytes (a foreign change: the whole restore refuses
//! and nothing is touched). The puts-back and removals then run as **one** coordinated step set of
//! the in-place publisher ([`ips::publish_all`]), so a removal that fails rolls the puts-back
//! back, and a split outcome is recorded per item so the next attempt resumes exactly where the
//! previous one stopped. There is no second transaction engine here: this module only plans and
//! classifies around the existing publisher.

use std::path::PathBuf;

use studio_domain::in_place_save::{self as ips, PlannedRemoval, PlannedWrite, Prior};
use studio_domain::tdb_txn::FileIdentity;

use crate::error::ApiError;

/// A destination as its record describes it.
pub(crate) struct RecordedItem {
    pub label: String,
    pub destination: PathBuf,
    /// What the destination held before (`None`: it did not exist; the operation created it).
    pub prior: Option<FileIdentity>,
    /// What the operation gave it.
    pub replacement: FileIdentity,
    /// The backup file holding the prior bytes (absolute), when `prior` is `Some`.
    pub backup: Option<PathBuf>,
}

/// What the checked restore found for one item.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ItemAction {
    /// Holds the prior bytes (or is absent as it was): nothing to do.
    AlreadyPrior,
    /// Holds the installed bytes or is missing: the verified backup goes back.
    Restore,
    /// Holds the created sidecar: it is removed.
    Remove,
}

pub(crate) struct RestorePlan {
    pub writes: Vec<PlannedWrite>,
    pub removals: Vec<PlannedRemoval>,
    /// One per recorded item, in order.
    pub actions: Vec<ItemAction>,
}

/// Record states a restore (or a recovery after an interrupted operation) accepts.
pub(crate) fn recoverable(state: &str) -> bool {
    matches!(state, "committed" | "restore-incomplete" | "split" | "publishing")
}

/// Classify every destination and plan the step set. `what` names the operation ("installed" /
/// "deployed") for messages. A foreign change anywhere refuses the whole plan.
pub(crate) fn plan_restore(items: &[RecordedItem], what: &str) -> Result<RestorePlan, ApiError> {
    let mut writes = Vec::new();
    let mut removals = Vec::new();
    let mut actions = Vec::new();
    let operation = if what == "deployed" { "deployment" } else { "installation" };
    for it in items {
        let display = it.destination.display().to_string();
        let current = FileIdentity::of_path_or_absent(&it.destination).map_err(|e| ApiError::Workspace(format!("{}: {display} cannot be read: {e}", it.label)))?;
        let action = match (&current, &it.prior) {
            (None, None) => ItemAction::AlreadyPrior,
            (Some(cur), Some(prior)) if cur == prior && *cur != it.replacement => ItemAction::AlreadyPrior,
            (Some(cur), Some(prior)) if cur == prior => ItemAction::AlreadyPrior, // prior == replacement: nothing to put back
            (Some(cur), Some(_)) if *cur == it.replacement => ItemAction::Restore,
            (Some(cur), None) if *cur == it.replacement => ItemAction::Remove,
            (Some(cur), _) => {
                return Err(ApiError::Workspace(format!(
                    "{}: {display} is {cur} now, not the {what} file ({}) and not the original{}; it changed after the {operation} and is not touched — nothing restored",
                    it.label,
                    it.replacement.sha256,
                    it.prior.as_ref().map(|p| format!(" ({})", p.sha256)).unwrap_or_default()
                )))
            }
            // A deleted loose file can be recovered from the verified original
            // backup. No foreign bytes are overwritten; rollback restores absence.
            (None, Some(_)) => ItemAction::Restore,
        };
        match action {
            ItemAction::Restore => {
                let backup = it.backup.as_ref().ok_or_else(|| ApiError::Workspace(format!("{}: the record has no backup for {display}; nothing restored", it.label)))?;
                let bytes = std::fs::read(backup).map_err(|e| ApiError::Workspace(format!("{}: the backup {} cannot be read: {e}", it.label, backup.display())))?;
                let identity = FileIdentity::of_bytes(&bytes);
                if Some(&identity) != it.prior.as_ref() {
                    return Err(ApiError::Workspace(format!("{}: the backup {} is not the recorded prior file; nothing restored", it.label, backup.display())));
                }
                let prior = if current.is_some() {
                    Prior::Bytes(std::fs::read(&it.destination).map_err(|source| ApiError::Io { path: display.clone(), source })?)
                } else { Prior::Absent };
                writes.push(PlannedWrite { role: "restored file", path: it.destination.clone(), prior, bytes, identity });
            }
            ItemAction::Remove => {
                let now = std::fs::read(&it.destination).map_err(|source| ApiError::Io { path: display.clone(), source })?;
                removals.push(PlannedRemoval { role: "created sidecar", path: it.destination.clone(), prior: now });
            }
            ItemAction::AlreadyPrior => {}
        }
        actions.push(action);
    }
    Ok(RestorePlan { writes, removals, actions })
}

/// One destination's record state from what it holds **now**: the operation's bytes (`holding`,
/// i.e. "installed" / "deployed"), its prior state ("prior": the prior bytes, or absent when the
/// operation created it), or anything else ("unknown").
pub(crate) fn state_on_disk(destination: &std::path::Path, prior: Option<&FileIdentity>, replacement: &FileIdentity, holding: &str) -> String {
    match (FileIdentity::of_path_or_absent(destination), prior) {
        (Ok(Some(cur)), _) if cur == *replacement => holding.to_string(),
        (Ok(Some(cur)), Some(p)) if cur == *p => "prior".to_string(),
        (Ok(None), None) => "prior".to_string(),
        _ => "unknown".to_string(),
    }
}

/// The record state and every item's state after the step set ran. Item states are read from the
/// destinations themselves (never inferred from the outcome's lists), so a split or interrupted
/// step set is recorded exactly as it left the disk. `holding` is the item state that means "still
/// holds the operation's bytes" ("installed" / "deployed"); `previous` is the record's state before
/// this attempt (kept when nothing changed).
pub(crate) fn classify(outcome: &ips::Outcome, items: &[RecordedItem], _plan: &RestorePlan, holding: &str, previous: &str) -> (String, Vec<String>) {
    let states = items.iter().map(|it| state_on_disk(&it.destination, it.prior.as_ref(), &it.replacement, holding)).collect();
    let record = match outcome {
        ips::Outcome::Committed => "restored".to_string(),
        ips::Outcome::RolledBack { .. } => previous.to_string(),
        ips::Outcome::Split { .. } => "restore-incomplete".to_string(),
    };
    (record, states)
}

/// `true` when every item already holds its prior state (a recovery with nothing left to do).
pub(crate) fn nothing_to_do(plan: &RestorePlan) -> bool {
    plan.writes.is_empty() && plan.removals.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_texture_restores_verified_backup_but_preserves_foreign_files() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("texlib_0_1_0.rpsgl");
        let backup = tmp.path().join("backup");
        std::fs::write(&backup, b"original texture").unwrap();
        let items = [RecordedItem { label: "jersey".into(), destination: path.clone(),
            prior: Some(FileIdentity::of_bytes(b"original texture")),
            replacement: FileIdentity::of_bytes(b"mod texture"), backup: Some(backup.clone()) }];
        let plan = plan_restore(&items, "installed").unwrap();
        assert_eq!(plan.writes.len(), 1);
        assert_eq!(plan.writes[0].prior, Prior::Absent);
        assert_eq!(plan.writes[0].bytes, b"original texture");
        std::fs::write(&backup, b"corrupt backup").unwrap();
        assert!(plan_restore(&items, "installed").is_err());
        std::fs::write(&backup, b"original texture").unwrap();
        assert_eq!(ips::publish_all(&plan.writes, &plan.removals, &mut ips::NoHooks), ips::Outcome::Committed);
        assert_eq!(std::fs::read(&path).unwrap(), b"original texture");
        std::fs::write(&path, b"another mod").unwrap();
        assert!(plan_restore(&items, "installed").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"another mod");
    }
}

//! LEAGUE-RESP-1 — lock contention around copied-target review and apply, through the
//! [`Registry`] seam (no Tauri). The desktop smoke measured Close answered ~32 s late while an
//! apply's preflight hashed a 777 MB bound input under the registry's `inner` mutex, and the
//! review held the same mutex for minutes while it staged an archive. These cases hold an apply
//! and a review at exactly those points — with a **barrier**, not a sleep — and assert that
//! close, status, cancel and edits on other projects are answered at once with the right
//! outcome, that competing edits/reviews/aliases on the held project are *refused* (never
//! waited for, never accepted), that the transaction semantics survive (cancel → aborted, files
//! untouched, edits kept; refusal → root free, project still owned), and that ownership is
//! released cleanly afterwards.
//!
//! Every held call runs on a helper thread with a deadline: a call that blocks until the barrier
//! lifts is reported as such instead of hanging the suite. "Prompt" here means the registry
//! answered within `PROMPT`; the receipt reports the observed values.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use studio_api::{parse_platform, ApplyPhase, Registry};
use studio_domain::tdb_txn;
use studio_domain::FieldValue;

const NHLNG: &str = "anchors/ps3/nhl14/databases/nhlng.db";
const META: &str = "anchors/ps3/nhl14/databases/nhlng-meta.xml";
const DEFS: &str = "defs";
const ROSTER: &str = "anchors/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA";
const PROFILE: &str = "B13EBABEBABEBABE";
const TITLE: &str = "454109EC";
const SAVE_NAME: &str = "ROSTER 20260723000649";

/// What "answered promptly" means at the registry level (the desktop target is one second
/// end-to-end; the registry's own share must be far below it).
const PROMPT: Duration = Duration::from_millis(1000);
/// A held call that has not returned by then is blocked on the barrier: the defect.
const DEADLINE: Duration = Duration::from_secs(20);

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

trait ErrText {
    fn err_text(self) -> String;
}
impl<T> ErrText for Result<T, studio_api::ApiError> {
    fn err_text(self) -> String {
        match self {
            Ok(_) => panic!("expected a refusal"),
            Err(e) => e.to_string(),
        }
    }
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-resp1-{tag}-{}", std::process::id()));
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

struct Tree {
    root: PathBuf,
    nhlng: PathBuf,
    roster: PathBuf,
}

fn copied_tree(base: &Path) -> Tree {
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
    std::fs::create_dir_all(root.join(PROFILE).join(TITLE).join("Headers/00000001")).unwrap();
    Tree {
        root,
        nhlng,
        roster,
    }
}

fn open(reg: &Registry, t: &Tree) -> u32 {
    let summary = reg
        .open_workspace_from_paths(
            Some("nhl14"),
            parse_platform("ps3").unwrap(),
            &s(&t.nhlng),
            Some(&s(&t.roster)),
            &s(&repo(META)),
            &s(&repo(DEFS)),
        )
        .unwrap();
    assert!(summary.roster_attached && summary.roster_aligned);
    summary.project_id
}

/// A second, independent database inside the same root (a different file, so a project on it
/// shares the root but none of the first project's files).
fn open_second_db(reg: &Registry, t: &Tree) -> (u32, PathBuf) {
    let db = t.root.join("USRDIR/nhlng-second.db");
    std::fs::copy(repo(NHLNG), &db).unwrap();
    let id = reg
        .load_project(
            &s(&db),
            parse_platform("ps3").unwrap(),
            studio_api::parse_role("nhlng").unwrap(),
            &s(&repo(META)),
            &s(&repo(DEFS)),
        )
        .unwrap()
        .project_id;
    (id, db)
}

fn edit(reg: &Registry, id: u32, value: i64) {
    reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(value))
        .unwrap();
}

fn try_edit(reg: &Registry, id: u32, value: i64) -> Result<(), String> {
    reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(value))
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Run `f` on a helper thread and wait for its answer; the elapsed time is measured from the
/// call. A call still outstanding after [`DEADLINE`] is the contention defect this suite exists
/// for, reported by `what`.
fn timed<T: Send + 'static>(what: &str, f: impl FnOnce() -> T + Send + 'static) -> (T, Duration) {
    let (tx, rx) = mpsc::channel();
    let start = Instant::now();
    std::thread::spawn(move || {
        let v = f();
        let _ = tx.send(v);
    });
    match rx.recv_timeout(DEADLINE) {
        Ok(v) => (v, start.elapsed()),
        Err(_) => panic!("{what}: still blocked after {DEADLINE:?} — it waits on a lock the held operation holds"),
    }
}

fn prompt(what: &str, took: Duration) {
    eprintln!("  {what}: {took:?}");
    assert!(
        took < PROMPT,
        "{what} took {took:?}; must answer within {PROMPT:?}"
    );
}

/// Sends `()` when the held operation reaches the barrier; the operation continues once the
/// test sends on `release`.
struct Barrier {
    reached_tx: mpsc::Sender<()>,
    release_rx: mpsc::Receiver<()>,
}

fn barrier() -> (Barrier, mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (reached_tx, reached_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    (
        Barrier {
            reached_tx,
            release_rx,
        },
        reached_rx,
        release_tx,
    )
}

// ------------------------------------------------------------------------------- apply preflight

/// The measured desktop freeze: Close (and status, cancel, other projects' edits) during the
/// apply's identity verification. The apply is held with its reservation taken and `inner`
/// free — exactly where the 777 MB hash runs on a real archive. Close answers `busy` naming the
/// apply; status lists the in-flight handle; cancel is acknowledged; the held project's edits
/// are refused (never waited for); an unrelated project edits and closes normally. The cancel
/// requested during the preflight takes effect: the set is aborted, nothing written, edits kept,
/// and both the initiator and its alias close cleanly afterwards.
#[test]
fn close_status_cancel_and_edits_answer_promptly_during_apply_preflight() {
    let tmp = Tmp::new("apply-preflight");
    let t = copied_tree(&tmp.0);
    let tmp_c = Tmp::new("apply-unrelated");
    let t_c = copied_tree(&tmp_c.0);
    let reg = std::sync::Arc::new(Registry::default());
    let a = open(&reg, &t);
    let b = open(&reg, &t); // alias on the same files
    let c = open(&reg, &t_c);
    let root = s(&t.root);
    let orig = std::fs::read(&t.nhlng).unwrap();
    edit(&reg, a, 2);
    let review = reg.copied_target_review(a, &root).unwrap();
    let handle = review.handle;

    let (bar, reached, release) = barrier();
    let apply = {
        let reg = reg.clone();
        std::thread::spawn(move || {
            let mut phases = Vec::new();
            let mut bar = Some(bar);
            let result = reg.copied_target_apply_observed(a, handle, None, &mut |phase| {
                phases.push(phase);
                if phase == ApplyPhase::Reserved {
                    let b = bar.take().unwrap();
                    b.reached_tx.send(()).unwrap();
                    b.release_rx.recv().unwrap();
                }
            });
            (result.map_err(|e| e.to_string()), phases)
        })
    };
    reached.recv().unwrap();
    eprintln!("held: apply of handle {handle} at its preflight (reservation held, inner free)");

    // Close: refused, promptly, naming the apply — for the initiator and for the alias.
    for id in [a, b] {
        let reg2 = reg.clone();
        let (o, took) = timed("close_project_outcome", move || {
            reg2.close_project_outcome(id)
        });
        prompt(&format!("close_project_outcome({id})"), took);
        assert_eq!(o.outcome, "busy");
        let why = o.reason.unwrap();
        assert!(
            why.contains(&format!("apply of review handle {handle}")) && why.contains("cancel it"),
            "{why}"
        );
    }
    // Status: the in-flight handle, both held projects, the reservation.
    let (st, took) = {
        let reg2 = reg.clone();
        let root2 = root.clone();
        timed("copied_target_status", move || {
            reg2.copied_target_status(&root2).unwrap()
        })
    };
    prompt("copied_target_status", took);
    assert_eq!(st.pending_handles.len(), 1);
    assert_eq!(st.pending_handles[0].handle, handle);
    assert!(st.pending_handles[0].in_progress);
    assert_eq!(st.busy_projects, vec![a, b]);
    assert_eq!(st.reservations.len(), 1);
    assert!(!st.apply_allowed);
    // Edits on the held projects: refused at once, never queued behind the apply.
    for id in [a, b] {
        let reg2 = reg.clone();
        let (r, took) = timed("set_field(held)", move || try_edit(&reg2, id, 1));
        prompt(&format!("set_field({id}) refusal"), took);
        let e = r.unwrap_err();
        assert!(e.contains("locked by a copied-target operation"), "{e}");
    }
    // An unrelated project is unaffected: it edits and closes.
    let (r, took) = {
        let reg2 = reg.clone();
        timed("set_field(unrelated)", move || try_edit(&reg2, c, 1))
    };
    prompt("set_field(unrelated)", took);
    r.unwrap();
    let (o, took) = {
        let reg2 = reg.clone();
        timed("close_project_outcome(unrelated)", move || {
            reg2.close_project_outcome(c)
        })
    };
    prompt("close_project_outcome(unrelated)", took);
    assert_eq!(o.outcome, "closed");
    // A second apply / review / recovery / restore on the held project or root: refused at once.
    let (e, took) = {
        let reg2 = reg.clone();
        timed("second apply", move || {
            reg2.copied_target_apply(a, handle).err_text()
        })
    };
    prompt("second apply refusal", took);
    assert!(e.contains("unknown, already applied"), "{e}");
    let (e, took) = {
        let reg2 = reg.clone();
        let root2 = root.clone();
        timed("review during apply", move || {
            reg2.copied_target_review(a, &root2).err_text()
        })
    };
    prompt("review-during-apply refusal", took);
    assert!(e.contains("apply in progress"), "{e}");
    // Cancel: acknowledged now; honoured by the apply once the barrier lifts.
    let (did, took) = {
        let reg2 = reg.clone();
        timed("copied_target_cancel", move || {
            reg2.copied_target_cancel(handle).unwrap()
        })
    };
    prompt("copied_target_cancel", took);
    assert!(did);

    release.send(()).unwrap();
    let (result, phases) = apply.join().unwrap();
    assert_eq!(phases, vec![ApplyPhase::Reserved, ApplyPhase::Verified]);
    let e = match result {
        Ok(_) => panic!("the cancelled apply must not commit"),
        Err(e) => e,
    };
    assert!(e.to_lowercase().contains("cancel"), "{e}");
    assert_eq!(std::fs::read(&t.nhlng).unwrap(), orig, "nothing written");
    assert!(reg.summary(a).unwrap().dirty, "edits kept");
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.reservations.is_empty() && st.busy_projects.is_empty());
    assert!(st.pending_handles.is_empty());
    assert!(st.apply_allowed, "{}", st.guidance);
    // Ownership after the refusal: both projects are still open and close cleanly.
    assert!(reg.summary(b).is_ok());
    assert_eq!(reg.close_project_outcome(b).outcome, "closed");
    assert_eq!(reg.close_project_outcome(a).outcome, "closed");
    assert_eq!(reg.close_project_outcome(a).outcome, "already_closed");
}

/// A cancel requested while the handle is merely prepared is honoured before the apply's
/// preflight hashes anything: the engine aborts the set at its first checkpoint, nothing is
/// written and the edits are kept. (Behaviourally the same as a cancel mid-preflight; this pins
/// the "skip the hashing" path so a cancelled apply never verifies 1.5 GB for nothing.)
#[test]
fn cancel_before_apply_aborts_without_verifying() {
    let tmp = Tmp::new("cancel-before");
    let t = copied_tree(&tmp.0);
    let reg = Registry::default();
    let a = open(&reg, &t);
    let root = s(&t.root);
    let orig = std::fs::read(&t.nhlng).unwrap();
    edit(&reg, a, 2);
    let review = reg.copied_target_review(a, &root).unwrap();
    assert!(reg.copied_target_cancel(review.handle).unwrap());
    let mut phases = Vec::new();
    let e = reg
        .copied_target_apply_observed(a, review.handle, None, &mut |p| phases.push(p))
        .err_text();
    assert_eq!(phases, vec![ApplyPhase::Reserved, ApplyPhase::Verified]);
    assert!(e.to_lowercase().contains("cancel"), "{e}");
    assert_eq!(std::fs::read(&t.nhlng).unwrap(), orig);
    assert!(reg.summary(a).unwrap().dirty);
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.apply_allowed && st.reservations.is_empty());
    assert_eq!(reg.close_project_outcome(a).outcome, "closed");
}

// ---------------------------------------------------------------------------------- review

/// The review-time case the dispatch asked to measure: the review is held after the sessions
/// were packaged and `inner` released, where an archive would be staged and the set prepared.
/// The project, its alias and its files are reserved for the review's duration: Close answers
/// `busy` naming the review; status shows the reservation (no handle yet — nothing is
/// prepared); edits, a competing review and a new alias on the files are refused at once; an
/// unrelated project works. When the barrier lifts the review completes on the packaged
/// revision, the reservation is gone, the handle is listed and applies; both projects close.
#[test]
fn close_status_and_edits_answer_promptly_during_review_staging() {
    let tmp = Tmp::new("review-staging");
    let t = copied_tree(&tmp.0);
    let tmp_c = Tmp::new("review-unrelated");
    let t_c = copied_tree(&tmp_c.0);
    let reg = std::sync::Arc::new(Registry::default());
    let a = open(&reg, &t);
    let b = open(&reg, &t);
    let c = open(&reg, &t_c);
    let root = s(&t.root);
    edit(&reg, a, 2);
    let revision_before = reg.summary(a).unwrap().workspace_revision.clone();

    let (bar, reached, release) = barrier();
    let review = {
        let reg = reg.clone();
        let root = root.clone();
        std::thread::spawn(move || {
            reg.copied_target_review_with(a, &root, move |_plan, staging| {
                assert!(staging.is_dir(), "the staging directory exists while held");
                bar.reached_tx.send(()).unwrap();
                bar.release_rx.recv().unwrap();
                Ok(())
            })
            .map_err(|e| e.to_string())
        })
    };
    reached.recv().unwrap();
    eprintln!("held: review of project {a} after packaging (reservation held, inner free)");

    for id in [a, b] {
        let reg2 = reg.clone();
        let (o, took) = timed("close_project_outcome", move || {
            reg2.close_project_outcome(id)
        });
        prompt(&format!("close_project_outcome({id})"), took);
        assert_eq!(o.outcome, "busy");
        let why = o.reason.unwrap();
        assert!(
            why.contains(&format!("review of project {a}")) && why.contains("wait for it"),
            "{why}"
        );
    }
    let (st, took) = {
        let reg2 = reg.clone();
        let root2 = root.clone();
        timed("copied_target_status", move || {
            reg2.copied_target_status(&root2).unwrap()
        })
    };
    prompt("copied_target_status", took);
    assert!(st.pending_handles.is_empty(), "nothing is prepared yet");
    assert_eq!(st.busy_projects, vec![a, b]);
    assert_eq!(st.reservations.len(), 1);
    assert!(st.reservations[0]
        .holder
        .contains(&format!("review of project {a}")));
    assert_eq!(st.reservations[0].projects, vec![a, b]);
    assert!(!st.reservations[0].whole_root);
    assert_eq!(
        st.reservations[0].files.len(),
        3,
        "primary, roster, new header"
    );
    for id in [a, b] {
        let reg2 = reg.clone();
        let (r, took) = timed("set_field(held)", move || try_edit(&reg2, id, 1));
        prompt(&format!("set_field({id}) refusal"), took);
        let e = r.unwrap_err();
        assert!(
            e.contains("locked by a copied-target operation")
                && e.contains(&format!("review of project {a}")),
            "{e}"
        );
    }
    // A competing review of the held project, and of its alias: refused at once.
    for id in [a, b] {
        let reg2 = reg.clone();
        let root2 = root.clone();
        let (e, took) = timed("competing review", move || {
            reg2.copied_target_review(id, &root2).err_text()
        });
        prompt(&format!("competing review({id}) refusal"), took);
        assert!(
            e.contains("locked by a copied-target operation")
                && e.contains(&format!("review of project {a}")),
            "{e}"
        );
    }
    // A new alias on the held files cannot be opened.
    let (e, took) = {
        let reg2 = reg.clone();
        let (nhlng, roster) = (s(&t.nhlng), s(&t.roster));
        timed("open alias", move || {
            reg2.open_workspace_from_paths(
                Some("nhl14"),
                parse_platform("ps3").unwrap(),
                &nhlng,
                Some(&roster),
                &s(&repo(META)),
                &s(&repo(DEFS)),
            )
            .err_text()
        })
    };
    prompt("open-alias refusal", took);
    assert!(e.contains("held by a copied-target operation"), "{e}");
    // Save-as into a held file from an unrelated project is refused; the project itself works.
    let (r, took) = {
        let reg2 = reg.clone();
        timed("set_field(unrelated)", move || try_edit(&reg2, c, 1))
    };
    prompt("set_field(unrelated)", took);
    r.unwrap();
    let e = reg.save(c, &s(&t.nhlng)).err_text();
    assert!(e.contains("held by a copied-target operation"), "{e}");

    release.send(()).unwrap();
    let review = review.join().unwrap().unwrap();
    assert_eq!(review.project_id, a);
    assert_eq!(
        review.workspace_revision, revision_before,
        "packaged revision"
    );
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.reservations.is_empty() && st.busy_projects.is_empty());
    assert_eq!(st.pending_handles.len(), 1);
    assert_eq!(st.pending_handles[0].handle, review.handle);
    assert!(!st.pending_handles[0].in_progress);
    // The project is editable again (which makes the handle stale — as designed) …
    assert_eq!(
        reg.summary(a).unwrap().workspace_revision,
        revision_before,
        "nothing changed the project during the review"
    );
    // … but first the handle applies as prepared.
    let applied = reg.copied_target_apply(a, review.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert_eq!(applied.aliases.len(), 1);
    assert_eq!(applied.aliases[0].project_id, b);
    assert!(!reg.summary(a).unwrap().dirty);
    assert_eq!(reg.close_project_outcome(b).outcome, "closed");
    assert_eq!(reg.close_project_outcome(a).outcome, "closed");
}

/// A review that fails after packaging (the staging hook refuses) releases its reservation and
/// its staging at once: the root is free, the project is still owned and editable, and the next
/// review succeeds.
#[test]
fn a_failed_review_releases_its_reservation_and_staging() {
    let tmp = Tmp::new("review-fails");
    let t = copied_tree(&tmp.0);
    let reg = Registry::default();
    let a = open(&reg, &t);
    let root = s(&t.root);
    edit(&reg, a, 2);
    let mut staging_seen = PathBuf::new();
    let e = reg
        .copied_target_review_with(a, &root, |_plan, staging| {
            staging_seen = staging.to_path_buf();
            Err(studio_api::ApiError::CopiedTarget(
                "the staging hook refused".into(),
            ))
        })
        .err_text();
    assert!(e.contains("the staging hook refused"), "{e}");
    assert!(!staging_seen.exists(), "the staging directory was removed");
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.reservations.is_empty() && st.busy_projects.is_empty());
    assert!(st.pending_handles.is_empty() && st.apply_allowed);
    assert!(st.staging_leftovers.is_empty());
    assert!(reg.summary(a).unwrap().dirty, "edits kept");
    edit(&reg, a, 1);
    let r = reg.copied_target_review(a, &root).unwrap();
    assert_eq!(
        reg.copied_target_apply(a, r.handle).unwrap().state,
        "committed"
    );
    assert_eq!(reg.close_project_outcome(a).outcome, "closed");
}

/// Two projects on **different** files in one root review concurrently — the concurrency the
/// shorter lock scope newly allows. Neither waits for the other: the second review runs to a
/// prepared handle while the first is held at its staging hook; when the first then tries to
/// prepare, the engine refuses the busy root (its `ACTIVE` marker belongs to the second) and the
/// first review fails cleanly — reservation released, staging removed, edits kept — while the
/// second's handle applies. There is no window in which both hold a prepared set.
#[test]
fn concurrent_reviews_on_one_root_contend_at_prepare_never_on_the_mutex() {
    let tmp = Tmp::new("review-concurrent");
    let t = copied_tree(&tmp.0);
    let reg = std::sync::Arc::new(Registry::default());
    let a = open(&reg, &t);
    let (d, second_db) = open_second_db(&reg, &t);
    let root = s(&t.root);
    edit(&reg, a, 2);
    edit(&reg, d, 2);
    let second_orig = std::fs::read(&second_db).unwrap();

    let (bar, reached, release) = barrier();
    let first = {
        let reg = reg.clone();
        let root = root.clone();
        std::thread::spawn(move || {
            reg.copied_target_review_with(a, &root, move |_plan, _| {
                bar.reached_tx.send(()).unwrap();
                bar.release_rx.recv().unwrap();
                Ok(())
            })
            .map_err(|e| e.to_string())
        })
    };
    reached.recv().unwrap();
    // The second project's review completes while the first is held.
    let (second, took) = {
        let reg2 = reg.clone();
        let root2 = root.clone();
        timed("second review", move || {
            reg2.copied_target_review(d, &root2)
                .map_err(|e| e.to_string())
        })
    };
    let second = second.unwrap();
    eprintln!("  second review while the first is held: {took:?}");
    assert_eq!(second.project_id, d);
    let st = reg.copied_target_status(&root).unwrap();
    assert_eq!(st.pending_handles.len(), 1);
    assert_eq!(st.reservations.len(), 1, "the first review's");
    assert_eq!(st.busy_projects, vec![a]);

    release.send(()).unwrap();
    let e = match first.join().unwrap() {
        Ok(_) => panic!("the first review must not prepare on a busy root"),
        Err(e) => e,
    };
    assert!(
        e.to_lowercase().contains("busy") || e.to_lowercase().contains("active"),
        "the first review is refused by the engine at prepare: {e}"
    );
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.reservations.is_empty() && st.busy_projects.is_empty());
    assert_eq!(st.pending_handles.len(), 1, "only the second's handle");
    assert!(st.staging_leftovers.is_empty());
    assert!(
        reg.summary(a).unwrap().dirty,
        "the first project's edits are kept"
    );
    let applied = reg.copied_target_apply(d, second.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert_ne!(std::fs::read(&second_db).unwrap(), second_orig);
    assert_eq!(reg.close_project_outcome(d).outcome, "closed");
    // And the first project can now review and apply on the freed root.
    let r = reg.copied_target_review(a, &root).unwrap();
    assert_eq!(
        reg.copied_target_apply(a, r.handle).unwrap().state,
        "committed"
    );
    assert_eq!(reg.close_project_outcome(a).outcome, "closed");
}

//! Opt-in layer-A observation for the existing QA matrix emitters —
//! [contract §3.1, §7.1](../../../../docs/plans/qa-evidence-contract.md).
//!
//! [`observe`](super::observe) is the pure run writer: it accumulates a whole run in memory and
//! seals it in one call. That is the right shape for a synthetic fixture and the wrong shape for a
//! long test binary, where a panic in case 200 would otherwise discard the 199 observations already
//! made. This module adds the lifecycle the real emitters need on top of it:
//!
//! * **Opt-in only.** Nothing is written unless [`ROOT_ENV`] names an evidence root, and that root
//!   may not be inside the repository (U3 — where layer A permanently lives is undecided, so this
//!   package puts nothing in the working tree). An emitter running without it holds a disabled sink
//!   that records nothing and touches no file.
//! * **Identity and declaration persist first.** The moment the root is accepted, an
//!   [in-progress journal](super::journal) is created outside `auto/runs/` holding the run's
//!   identity, its declared keys and — as they happen — its observations. Identity is written
//!   atomically, so a journal is either fully identified or not identified at all.
//! * **Append-only records.** Every declaration, observation, unmapped record and facet is
//!   **appended**, never rewritten. A whole-file rewrite risks every previously persisted record on
//!   every write; an append can damage only the record it is writing, and the loader drops that
//!   fragment and keeps the rest. See [`journal`](super::journal) for the recovery rules.
//! * **An executed failure survives the panic that produced it.** [`Emitter::case`] catches the
//!   unwind, records `fail`, and **immediately resumes it**: the test fails exactly as it did
//!   before, and the failure is now also evidence. The [`Drop`] guard then seals the run as
//!   `failed`, retaining every observation made before it.
//! * **Persistence failure is never silent.** A failed append is recorded and turns the emitter's
//!   workflow into a non-success one: the next observation call panics naming the retained journal,
//!   and [`Emitter::finish`] seals the run `failed` and panics. The one exception is while a panic
//!   is already unwinding — a second panic there aborts the process and would destroy the journal.
//!
//! **What is not covered.** A hard kill (`SIGKILL`, `abort`, a stack overflow, a `panic = "abort"`
//! profile) runs no destructor, so the run is never sealed. Since C0-S2-INT1-R1 that is no longer
//! invisible: the journal is loaded by the merge as **incomplete execution evidence** — cited,
//! never selected, and blocking where it persisted a failure. It is still never sealed
//! automatically, because a journal on disk does not prove the process that wrote it is dead.
//!
//! There is no power-loss durability (nothing is flushed to the device) and no concurrency control.
//!
//! Timestamps written here are recorded metadata. **R-NO-TIME** forbids any decision reading them.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::journal;
use super::key::CoverageKey;
use super::model::{Outcome, RunState, UnmappedObservation};
use super::observe::RunBuilder;
use super::scopes::{Scopes, UNKNOWN};
use super::store;

/// The **only** switch that turns observation on. Its value is the evidence root.
pub const ROOT_ENV: &str = "QA_EVIDENCE_ROOT";

/// The compile-time toolchain identities of contract §5.3.
///
/// **R-FRESH-UNKNOWN.** An identity the build could not resolve is `None`, which makes every scope
/// declaring it digest to [`UNKNOWN`] and therefore not-fresh. It is never guessed, and never
/// substituted with something adjacent — `std::env::consts::ARCH` is an architecture, not a target
/// triple, and reporting it as one would be a fabricated identity.
pub fn toolchain_identity(name: &str) -> Option<String> {
    match name {
        "rustc-version" => option_env!("QA_RUSTC_VERSION").map(str::to_string),
        "target-triple" => option_env!("QA_TARGET_TRIPLE").map(str::to_string),
        _ => None,
    }
}

/// Every scope's current `implementation_digest`, computed from the repository exactly as
/// `qa-evidence` computes it. A scope whose inputs cannot be read digests to [`UNKNOWN`].
pub fn scope_digests(repo_root: &Path) -> BTreeMap<(String, u32), String> {
    let mut out = BTreeMap::new();
    let path = repo_root.join("defs/qa/scopes.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return out;
    };
    let Ok(scopes) = Scopes::parse(&text) else {
        return out;
    };
    let Ok(contents) = store::collect_scope_contents(repo_root, &scopes) else {
        return out;
    };
    for scope in &scopes.scopes {
        out.insert(
            (scope.id.clone(), scope.version),
            scopes.digest(&scope.id, &contents, &toolchain_identity),
        );
    }
    out
}

/// The digest and version of one scope, or `(UNKNOWN, 0)` when the definition cannot be resolved.
fn scope_identity(repo_root: &Path, scope_id: &str) -> (String, u32) {
    scope_digests(repo_root)
        .into_iter()
        .find(|((id, _), _)| id == scope_id)
        .map(|((_, version), digest)| (digest, version))
        .unwrap_or_else(|| (UNKNOWN.to_string(), 0))
}

/// SHA-256 of a file's bytes, or `None` when it cannot be read. An unreadable fixture never yields
/// a digest-shaped placeholder.
pub fn file_digest(path: &Path) -> Option<String> {
    std::fs::read(path).ok().map(|b| store::sha256_hex(&b))
}

// ------------------------------------------------------------------ the sink ------

enum Sink {
    /// No `QA_EVIDENCE_ROOT`: records nothing and writes nothing, ever.
    Disabled,
    Active(Box<Active>),
}

struct Active {
    root: PathBuf,
    journal_dir: PathBuf,
    identity: journal::IdentityFile,
    builder: RunBuilder,
    declared: BTreeSet<String>,
    /// The first persistence or sealing failure. Once set, the emitter's workflow can no longer
    /// report success: the run seals `failed` and the caller is told, loudly, where the journal is.
    persist_error: Option<String>,
    sealed: bool,
}

/// An emitter's observation sink. Construct with [`Emitter::from_env`], declare every key the
/// emitter intends to cover, wrap each case in [`Emitter::case`], record its outcome, and call
/// [`Emitter::finish`] when the emitter completes.
pub struct Emitter {
    sink: Sink,
}

impl Emitter {
    /// Read [`ROOT_ENV`]. Unset → a disabled sink. Set → the run's identity and journal are created
    /// immediately, before a single case runs.
    ///
    /// # Panics
    ///
    /// When the variable is set but unusable: a root inside `repo_root` (U3), or a journal whose
    /// identity cannot be created. Observation was asked for explicitly, so failing to provide it
    /// is an error rather than a silent downgrade to writing nothing.
    pub fn from_env(emitter: &str, scope_id: &str, repo_root: &Path) -> Self {
        let Some(raw) = std::env::var_os(ROOT_ENV) else {
            return Emitter {
                sink: Sink::Disabled,
            };
        };
        let root = PathBuf::from(&raw);
        if root.as_os_str().is_empty() {
            panic!("{ROOT_ENV} is set but empty; unset it or name an evidence root");
        }
        let repo = repo_root
            .canonicalize()
            .unwrap_or_else(|_| repo_root.to_path_buf());
        std::fs::create_dir_all(&root)
            .unwrap_or_else(|e| panic!("{ROOT_ENV}={}: {e}", root.display()));
        let resolved = root
            .canonicalize()
            .unwrap_or_else(|e| panic!("{ROOT_ENV}={}: cannot resolve: {e}", root.display()));
        if resolved.starts_with(&repo) {
            panic!(
                "{ROOT_ENV}={} resolves inside the repository ({}); layer-A storage in git is \
                 undecided (U3), so the root must be outside it",
                resolved.display(),
                repo.display()
            );
        }

        let started_utc = utc_now();
        let (implementation_digest, scope_version) = scope_identity(repo_root, scope_id);
        let toolchain = toolchain_identity("rustc-version").unwrap_or_else(|| UNKNOWN.to_string());
        let builder = RunBuilder::new(
            emitter,
            &started_utc,
            scope_id,
            scope_version,
            &implementation_digest,
        )
        .toolchain(&toolchain);

        // The journal id is not the run id: the run id is content-derived and is only known once
        // every declaration and observation is in. The nonce keeps two runs of one emitter in one
        // second apart, and is never read by a decision (R-NO-TIME).
        let journal_id = format!(
            "{emitter}-{}-{:08x}",
            basic(&started_utc),
            nonce(emitter, &started_utc)
        );
        let journal_dir = journal::journals_dir(&resolved).join(&journal_id);
        std::fs::create_dir_all(&journal_dir)
            .unwrap_or_else(|e| panic!("{}: {e}", journal_dir.display()));

        let active = Active {
            root: resolved,
            journal_dir,
            identity: journal::IdentityFile {
                schema: journal::IDENTITY_SCHEMA.to_string(),
                journal_id,
                emitter: emitter.to_string(),
                started_utc,
                host_os: std::env::consts::OS.to_string(),
                toolchain,
                scope_id: scope_id.to_string(),
                scope_version,
                implementation_digest,
                sealed_run_id: None,
            },
            builder,
            declared: BTreeSet::new(),
            persist_error: None,
            sealed: false,
        };
        // Identity first, atomically: a journal is either fully identified or not identified at all.
        active
            .write_identity()
            .unwrap_or_else(|e| panic!("could not persist the run identity: {e}"));
        Emitter {
            sink: Sink::Active(Box::new(active)),
        }
    }

    /// A disabled sink, for a caller that must not observe regardless of the environment.
    pub fn disabled() -> Self {
        Emitter {
            sink: Sink::Disabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self.sink, Sink::Active(_))
    }

    /// The in-progress journal directory, when observation is on.
    pub fn journal_dir(&self) -> Option<&Path> {
        match &self.sink {
            Sink::Active(a) => Some(&a.journal_dir),
            Sink::Disabled => None,
        }
    }

    /// Record a real input digest on the manifest. A `None` digest is recorded as the literal
    /// `unknown` (R-UNKNOWN), never omitted and never invented.
    pub fn input_digest(&mut self, name: &str, digest: Option<String>) {
        if let Sink::Active(a) = &mut self.sink {
            let value = digest.unwrap_or_else(|| UNKNOWN.to_string());
            let builder =
                std::mem::replace(&mut a.builder, RunBuilder::new("", "", "", 0, UNKNOWN));
            a.builder = builder.input_digest(name, &value);
        }
    }

    /// Declare a key this emitter intends to cover, and persist the declaration immediately.
    ///
    /// # Panics
    ///
    /// On a key that does not parse — a programming error in the mapping, caught in **both** modes
    /// so a disabled run cannot hide it. Also on a persistence failure (see
    /// [`Emitter::check_persistence`]).
    pub fn declare(&mut self, key: &str) -> CoverageKey {
        let parsed = CoverageKey::parse(key)
            .unwrap_or_else(|e| panic!("emitter declared an invalid coverage key {key:?}: {e}"));
        if let Sink::Active(a) = &mut self.sink {
            a.builder.declare(parsed.clone());
            let text = parsed.to_string();
            if a.declared.insert(text.clone()) {
                a.append("keys-declared.txt", None, &format!("{text}\n"));
            }
        }
        self.check_persistence();
        parsed
    }

    /// Run one case, recording an **executed failure** if it panics and then resuming the panic
    /// unchanged.
    ///
    /// Nothing is recorded on success: the caller records the outcome afterwards, so a `pass` is
    /// only ever written once the case's assertions have actually succeeded.
    ///
    /// [`AssertUnwindSafe`] is sound here because the panic is resumed immediately: no state the
    /// closure touched is observed afterwards by this function or by its caller.
    pub fn case<T>(&mut self, key: &CoverageKey, case_id: &str, f: impl FnOnce() -> T) -> T {
        match std::panic::catch_unwind(AssertUnwindSafe(f)) {
            Ok(value) => value,
            Err(payload) => {
                // Recording must never raise a second panic while this one unwinds: a double panic
                // aborts the process and would destroy the very evidence being recorded.
                self.observe(key, case_id, Outcome::Fail, &panic_message(&*payload));
                std::panic::resume_unwind(payload);
            }
        }
    }

    /// As [`Emitter::case`], for a case whose emitter operation has no coverage key. A panic is
    /// recorded as an unmapped executed failure, then resumed unchanged.
    pub fn case_unmapped<T>(
        &mut self,
        target: &str,
        emitter_operation: &str,
        case_id: &str,
        unresolved: &str,
        f: impl FnOnce() -> T,
    ) -> T {
        match std::panic::catch_unwind(AssertUnwindSafe(f)) {
            Ok(value) => value,
            Err(payload) => {
                self.observe_unmapped(
                    target,
                    emitter_operation,
                    case_id,
                    Outcome::Fail,
                    "",
                    unresolved,
                    &panic_message(&*payload),
                );
                std::panic::resume_unwind(payload);
            }
        }
    }

    /// Record what a case did, and persist it immediately.
    pub fn observe(&mut self, key: &CoverageKey, case_id: &str, outcome: Outcome, detail: &str) {
        self.observe_faceted(key, case_id, outcome, "", "", detail, &[]);
    }

    /// As [`Emitter::observe`], carrying the real fixture and output digests the case worked over.
    pub fn observe_hashed(
        &mut self,
        key: &CoverageKey,
        case_id: &str,
        outcome: Outcome,
        fixture_sha256: &str,
        output_sha256: &str,
        detail: &str,
    ) {
        self.observe_faceted(
            key,
            case_id,
            outcome,
            fixture_sha256,
            output_sha256,
            detail,
            &[],
        );
    }

    /// As [`Emitter::observe_hashed`], with **structured** descriptors — codec, container version,
    /// save mode. Facets keep format identity machine-readable instead of buried in `detail`; no
    /// merge decision reads one.
    #[allow(clippy::too_many_arguments)]
    pub fn observe_faceted(
        &mut self,
        key: &CoverageKey,
        case_id: &str,
        outcome: Outcome,
        fixture_sha256: &str,
        output_sha256: &str,
        detail: &str,
        facets: &[(&str, &str)],
    ) {
        if let Sink::Active(a) = &mut self.sink {
            a.builder
                .observe_hashed(key, case_id, outcome, fixture_sha256, output_sha256, detail)
                .unwrap_or_else(|e| panic!("{e}"));
            let key_text = key.to_string();
            let fields = [
                journal::OBSERVATION_ROW,
                key_text.as_str(),
                case_id,
                outcome.as_str(),
                fixture_sha256,
                output_sha256,
                detail,
            ];
            a.append(
                "observations.csv",
                Some(journal::OBSERVATION_HEADER),
                &journal::record_line(&fields),
            );
            for (name, value) in facets {
                a.builder.facet(case_id, Some(key), name, value);
                let facet_fields = [journal::FACET_ROW, case_id, key_text.as_str(), name, value];
                a.append(
                    "facets.csv",
                    Some(journal::FACET_HEADER),
                    &journal::record_line(&facet_fields),
                );
            }
        }
        self.check_persistence();
    }

    /// Record an executed case whose emitter operation has **no** operation in the closed §4.1
    /// vocabulary. It carries no coverage key, so it qualifies nothing and enters no axis — but the
    /// execution, its outcome, its fixture and the reason no key was assigned all survive.
    #[allow(clippy::too_many_arguments)]
    pub fn observe_unmapped(
        &mut self,
        target: &str,
        emitter_operation: &str,
        case_id: &str,
        outcome: Outcome,
        fixture_sha256: &str,
        unresolved: &str,
        detail: &str,
    ) {
        if let Sink::Active(a) = &mut self.sink {
            a.builder.observe_unmapped(UnmappedObservation {
                target: target.to_string(),
                emitter_operation: emitter_operation.to_string(),
                case_id: case_id.to_string(),
                outcome,
                fixture_sha256: fixture_sha256.to_string(),
                unresolved: unresolved.to_string(),
                detail: detail.to_string(),
            });
            let fields = [
                journal::UNMAPPED_ROW,
                target,
                emitter_operation,
                case_id,
                outcome.as_str(),
                fixture_sha256,
                unresolved,
                detail,
            ];
            a.append(
                "unmapped.csv",
                Some(journal::UNMAPPED_HEADER),
                &journal::record_line(&fields),
            );
        }
        self.check_persistence();
    }

    /// Seal the run as `completed`. Consumes the sink, so the [`Drop`] guard has nothing left to do.
    ///
    /// Returns the sealed run directory, or `None` when observation was off.
    ///
    /// # Panics
    ///
    /// When observation was explicitly enabled and persistence or sealing failed. A run whose
    /// evidence could not be written must not report success — the journal is retained and named.
    pub fn finish(mut self) -> Option<PathBuf> {
        let failure = match &self.sink {
            Sink::Active(a) => a.persist_error.clone(),
            Sink::Disabled => None,
        };
        let state = if failure.is_some() {
            RunState::Failed
        } else {
            RunState::Completed
        };
        let dir = self.seal(state, failure.clone());
        if let Sink::Active(a) = &self.sink {
            if let Some(message) = failure {
                panic!(
                    "qa-evidence: observation was enabled but persistence failed: {message}; the \
                     run was sealed `failed` and the journal is retained at {}",
                    a.journal_dir.display()
                );
            }
            if dir.is_none() {
                panic!(
                    "qa-evidence: observation was enabled but the run could not be sealed; the \
                     journal is retained at {}",
                    a.journal_dir.display()
                );
            }
        }
        dir
    }

    /// Panic on a recorded persistence failure — unless a panic is already unwinding, in which case
    /// the failure is kept for [`Emitter::finish`] and the drop guard. A second panic during
    /// unwinding aborts the process and would destroy the journal being written.
    fn check_persistence(&self) {
        if std::thread::panicking() {
            return;
        }
        if let Sink::Active(a) = &self.sink {
            if let Some(message) = &a.persist_error {
                panic!(
                    "qa-evidence: could not persist an observation: {message}; evidence already \
                     persisted is retained at {}",
                    a.journal_dir.display()
                );
            }
        }
    }

    fn seal(&mut self, state: RunState, failure: Option<String>) -> Option<PathBuf> {
        let Sink::Active(a) = &mut self.sink else {
            return None;
        };
        if a.sealed {
            return None;
        }
        a.sealed = true;
        let ended = utc_now();
        match a.builder.write(&a.root, state, &ended, failure) {
            Ok(dir) => {
                // Link the journal to its sealed run *before* removing it. If the removal fails, or
                // the process dies between the two, the surviving journal names the run holding the
                // same evidence and the loader skips it instead of counting it twice.
                let run_id = dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                a.identity.sealed_run_id = Some(run_id);
                if let Err(e) = a.write_identity() {
                    eprintln!(
                        "qa-evidence: sealed {} but could not link the journal to it: {e}",
                        dir.display()
                    );
                    return Some(dir);
                }
                let _ = std::fs::remove_dir_all(&a.journal_dir);
                Some(dir)
            }
            Err(e) => {
                eprintln!(
                    "qa-evidence: could not seal the {} run: {e}; the in-progress journal is kept \
                     at {}",
                    state.as_str(),
                    a.journal_dir.display()
                );
                None
            }
        }
    }
}

impl Drop for Emitter {
    /// An emitter that never reached [`Emitter::finish`] still has evidence worth keeping. Unwinding
    /// means a case failed; anything else means the emitter returned without finishing.
    fn drop(&mut self) {
        let persist_error = match &self.sink {
            Sink::Active(a) => a.persist_error.clone(),
            Sink::Disabled => None,
        };
        let (state, failure) = if std::thread::panicking() {
            (
                RunState::Failed,
                Some(match persist_error {
                    Some(e) => format!("a case panicked, and persistence had already failed: {e}"),
                    None => {
                        "a case panicked; the run was sealed by the emitter's drop guard".into()
                    }
                }),
            )
        } else {
            (
                RunState::Interrupted,
                Some(match persist_error {
                    Some(e) => {
                        format!(
                            "the emitter returned without finishing; persistence had failed: {e}"
                        )
                    }
                    None => "the emitter returned without calling finish()".into(),
                }),
            )
        };
        // Never panic while unwinding: a failure here is reported and the journal retained.
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            self.seal(state, failure);
        }));
    }
}

impl Active {
    /// Write `journal.json` atomically: a temporary sibling, then a rename over the target. A
    /// half-written identity file would make the whole journal unattributable, and the loader fails
    /// closed on exactly that.
    fn write_identity(&self) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&self.identity).map_err(|e| e.to_string())? + "\n";
        let temp = self.journal_dir.join("journal.json.new");
        super::paths::write_file(&temp, text.as_bytes()).map_err(|e| e.to_string())?;
        super::paths::rename_file(&temp, &self.journal_dir.join("journal.json"))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Append one already-formatted record, writing `header` first if the file does not exist yet.
    ///
    /// **Append-only is the whole point.** Rewriting the file would put every previously persisted
    /// record at risk on every single write; an append can damage only the record it is writing,
    /// and the loader discards that fragment and keeps the rest.
    ///
    /// A failure is recorded once — the *first* one, because it carries the useful cause — and
    /// turns the emitter's workflow into a non-success one.
    fn append(&mut self, name: &str, header: Option<&[&str]>, record: &str) {
        if self.persist_error.is_some() {
            return;
        }
        let path = self.journal_dir.join(name);
        if let Some(header) = header {
            if !path.exists() {
                if let Err(e) = super::paths::append_file(&path, super::csv::row(header).as_bytes())
                {
                    self.persist_error = Some(format!("{}: {e}", path.display()));
                    return;
                }
            }
        }
        if let Err(e) = super::paths::append_file(&path, record.as_bytes()) {
            self.persist_error = Some(format!("{}: {e}", path.display()));
        }
    }
}

// ------------------------------------------------------------------ helpers ------

fn basic(utc: &str) -> String {
    utc.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn nonce(emitter: &str, started_utc: &str) -> u32 {
    let mut hasher = Sha256::new();
    hasher.update(emitter.as_bytes());
    hasher.update(started_utc.as_bytes());
    hasher.update(std::process::id().to_le_bytes());
    let digest = hasher.finalize();
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]])
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "a case panicked with a non-string payload".to_string()
    }
}

/// `YYYY-MM-DDTHH:MM:SSZ` from the system clock. Recorded metadata only (**R-NO-TIME**).
pub fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant's `civil_from_days`, for a days-since-1970 count.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_target_triple_is_never_substituted_with_an_architecture() {
        // The identity is either the real triple the build recorded, or nothing at all.
        if let Some(triple) = toolchain_identity("target-triple") {
            assert!(
                triple.matches('-').count() >= 2,
                "target-triple {triple:?} is not a triple"
            );
            assert_ne!(triple, std::env::consts::ARCH);
        }
        assert_eq!(toolchain_identity("no-such-identity"), None);
    }

    #[test]
    fn the_utc_string_has_the_contract_shape() {
        let now = utc_now();
        assert_eq!(now.len(), 20, "{now}");
        assert!(now.ends_with('Z') && now.contains('T'), "{now}");
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_000), (2022, 1, 8));
    }

    #[test]
    fn a_disabled_sink_writes_nothing_and_seals_nothing() {
        let emitter = Emitter::disabled();
        assert!(!emitter.is_enabled());
        assert_eq!(emitter.journal_dir(), None);
        assert_eq!(emitter.finish(), None);
    }
}

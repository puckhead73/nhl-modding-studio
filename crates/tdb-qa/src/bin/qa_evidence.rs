//! `qa-evidence` — the QA evidence merge, check, build and publication tool.
//!
//! Implements [contract §7.1](../../../../docs/plans/qa-evidence-contract.md):
//!
//! ```text
//! qa-evidence check   [--root <dir>] [--data <dir>] [--repo <dir>]
//! qa-evidence build   --out <dir> [--root <dir>] [--repo <dir>]
//! qa-evidence publish --from <dir> [--data <dir>] [--root <dir>] [--repo <dir>]
//! qa-evidence publish --recover [--data <dir>]
//! ```
//!
//! | Command | Writes | Exit |
//! |---|---|---|
//! | `check` | **nothing** | 0 ok · 2 drift vs the published set · 3 malformed active input |
//! | `build` | `--out` only | 0 ok · 3 malformed active input |
//! | `publish` | the layer-C set in `--data` | refuses unless the staged set checks **and** the merge inputs re-verify |
//!
//! No command writes under `manual/`. Migration — the only path into layer B — is a separate
//! command with its own approval file (contract §7.2), and is not part of this binary's default
//! flow.
//!
//! Running with no arguments prints this usage and exits non-zero; nothing is ever written by
//! accident.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tdb_qa::evidence::emit;
use tdb_qa::evidence::journal;
use tdb_qa::evidence::merge;
use tdb_qa::evidence::model::EvidenceSet;
use tdb_qa::evidence::publish::{self, BuildInputs, PublishOptions};
use tdb_qa::evidence::scopes::Scopes;
use tdb_qa::evidence::store;
use tdb_qa::evidence::validate::validate;
use tdb_qa::evidence::vocab;

const USAGE: &str = "\
qa-evidence — QA evidence merge, check, build and publication

USAGE:
  qa-evidence check   [--root <dir>] [--data <dir>] [--repo <dir>]
  qa-evidence build   --out <dir> [--root <dir>] [--repo <dir>]
  qa-evidence publish --from <dir> [--data <dir>] [--root <dir>] [--repo <dir>]
  qa-evidence publish --recover [--data <dir>]
  qa-evidence migrate --frozen <dir> --out <dir> --id <migration-id> [--repo <dir>]
  qa-evidence migrate-install --from <dir> --approve <file> --id <migration-id> [--root <dir>]

DEFAULTS:
  --repo   the repository root inferred from this crate
  --root   <repo>/docs/audits/evidence      (layers A, B, L)
  --data   <repo>/docs/audits/data          (layer C, published)

EXIT CODES:
  0 ok · 2 drift against the published set · 3 malformed active input · 4 command error

NOTES:
  `check` and `build` never write into the published tree. `migrate` writes only into its
  --out candidate directory. `migrate-install` is the ONLY command that writes under
  <root>/manual, and it refuses without an approval file naming the migration id and
  carrying the exact candidate-set digest — see docs/plans/qa-evidence-contract.md §7.2.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprint!("{USAGE}");
        return ExitCode::from(4);
    }
    match run(&args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("qa-evidence: {message}");
            ExitCode::from(4)
        }
    }
}

struct Options {
    repo: PathBuf,
    root: PathBuf,
    data: PathBuf,
    out: Option<PathBuf>,
    from: Option<PathBuf>,
    recover: bool,
    frozen: Option<PathBuf>,
    approve: Option<PathBuf>,
    id: Option<String>,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let repo_default = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut repo = repo_default.clone();
    let mut root = None;
    let mut data = None;
    let mut out = None;
    let mut from = None;
    let mut recover = false;
    let mut frozen = None;
    let mut approve = None;
    let mut id = None;
    let mut i = 0;
    while i < args.len() {
        let value = |i: usize| -> Result<PathBuf, String> {
            args.get(i + 1)
                .map(PathBuf::from)
                .ok_or_else(|| format!("{} needs a value", args[i]))
        };
        match args[i].as_str() {
            "--repo" => {
                repo = value(i)?;
                i += 2;
            }
            "--root" => {
                root = Some(value(i)?);
                i += 2;
            }
            "--data" => {
                data = Some(value(i)?);
                i += 2;
            }
            "--out" => {
                out = Some(value(i)?);
                i += 2;
            }
            "--from" => {
                from = Some(value(i)?);
                i += 2;
            }
            "--recover" => {
                recover = true;
                i += 1;
            }
            "--frozen" => {
                frozen = Some(value(i)?);
                i += 2;
            }
            "--approve" => {
                approve = Some(value(i)?);
                i += 2;
            }
            "--id" => {
                id = Some(
                    args.get(i + 1)
                        .cloned()
                        .ok_or_else(|| "--id needs a value".to_string())?,
                );
                i += 2;
            }
            other => return Err(format!("unknown argument {other:?}\n\n{USAGE}")),
        }
    }
    Ok(Options {
        root: root.unwrap_or_else(|| repo.join("docs/audits/evidence")),
        data: data.unwrap_or_else(|| repo.join("docs/audits/data")),
        repo,
        out,
        from,
        recover,
        frozen,
        approve,
        id,
    })
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let command = args[0].clone();
    let options = parse_options(&args[1..])?;
    match command.as_str() {
        "check" => check(&options),
        "build" => build_cmd(&options),
        "publish" => publish_cmd(&options),
        "migrate" => migrate_cmd(&options),
        "migrate-install" => migrate_install_cmd(&options),
        other => Err(format!("unknown command {other:?}\n\n{USAGE}")),
    }
}

/// The validated evidence layout every mutating command must present (R2).
fn layout(options: &Options) -> Result<tdb_qa::evidence::paths::Layout, String> {
    tdb_qa::evidence::paths::Layout::new(&options.root).map_err(|e| e.to_string())
}

/// Assemble the evidence set from the tree, and the freshness digests from the repository.
fn load(options: &Options) -> Result<(EvidenceSet, BuildInputs), String> {
    let runs = store::load_runs(&options.root).map_err(|e| e.to_string())?;
    // Incomplete execution evidence. Loaded read-only, never sealed, never selected — and a journal
    // already sealed into a present run is skipped, so nothing is counted twice.
    let journals = journal::load_journals(&options.root, &runs).map_err(|e| e.to_string())?;
    let manual = store::load_manual(&options.root).map_err(|e| e.to_string())?;
    let legacy = store::load_legacy(&options.root).map_err(|e| e.to_string())?;

    // Freshness definitions are optional only in the sense that their absence makes every digest
    // `unknown` — which is not-fresh everywhere. It never makes anything fresh.
    let scopes_path = options.repo.join("defs/qa/scopes.toml");
    let (scopes, scopes_digest) = match std::fs::read_to_string(&scopes_path) {
        Ok(text) => {
            let parsed =
                Scopes::parse(&text).map_err(|e| format!("{}: {e}", scopes_path.display()))?;
            (Some(parsed), store::sha256_prefixed(text.as_bytes()))
        }
        Err(_) => (None, "unknown".to_string()),
    };
    let mut current_scope_digests = BTreeMap::new();
    if let Some(scopes) = &scopes {
        let contents =
            store::collect_scope_contents(&options.repo, scopes).map_err(|e| e.to_string())?;
        // The emitters compute their `implementation_digest` with the identical resolver
        // (`evidence::emit::toolchain_identity`); anything else would make every run stale.
        for scope in &scopes.scopes {
            current_scope_digests.insert(
                (scope.id.clone(), scope.version),
                scopes.digest(&scope.id, &contents, &emit::toolchain_identity),
            );
        }
    }

    let mut merge_input_digests = BTreeMap::new();
    for run in &runs {
        merge_input_digests.insert(
            format!("auto/runs/{}", run.run_id()),
            run.manifest.implementation_digest.clone(),
        );
    }
    for j in &journals {
        merge_input_digests.insert(
            format!("auto/in-progress/{}", j.journal_id),
            j.implementation_digest.clone(),
        );
    }
    merge_input_digests.insert(
        "manual/".to_string(),
        store::manual_digest(&options.root).map_err(|e| e.to_string())?,
    );

    // C0-EMIT1-R2 — every fixture a run attributes is hashed now, once per resolved file, and
    // compared by the merge against what the run captured. The states ride in the merge inputs
    // under `fixture:` so the provenance sidecar records what was compared and `publish`'s A30
    // re-verification refuses if a fixture moved between build and publish.
    let fixtures = store::verify_fixture_inputs(&options.repo, &runs);
    for (path, state) in &fixtures.states {
        merge_input_digests.insert(format!("fixture:{path}"), state.render());
    }
    let mut fixture_verification = BTreeMap::new();
    for run in &runs {
        fixture_verification.insert(
            run.run_id().to_string(),
            merge::run_fixture_verdict(run, &fixtures.states).render(),
        );
    }
    eprintln!(
        "fixtures: {} attributed path(s), {} file(s) hashed, {} byte(s)",
        fixtures.states.len(),
        fixtures.files_hashed,
        fixtures.bytes_hashed
    );

    // Unmapped executions from both layers, kept machine-readable and attributed to their source.
    let mut unmapped = Vec::new();
    for run in &runs {
        for u in &run.unmapped {
            unmapped.push(unmapped_row(
                "auto-run",
                run.run_id(),
                run.state.as_str(),
                u,
            ));
        }
    }
    for j in &journals {
        for u in &j.unmapped {
            unmapped.push(unmapped_row("auto-journal", &j.journal_id, "incomplete", u));
        }
    }

    let set = EvidenceSet {
        journals,
        runs,
        manual,
        legacy,
        implementation: BTreeMap::new(),
        current_scope_digests,
        current_input_digests: fixtures.states,
        declared_universe: vocab::catalog_universe(),
    };
    let inputs = BuildInputs {
        tool_version: format!("qa-evidence/{}", env!("CARGO_PKG_VERSION")),
        merge_input_digests,
        fixture_verification,
        scope_definitions_digest: scopes_digest,
        divergence: Vec::new(),
        unmapped,
    };
    Ok((set, inputs))
}

/// One published unmapped execution. Nothing is inferred: every field is copied verbatim from the
/// record the emitter wrote.
fn unmapped_row(
    source_kind: &str,
    source_id: &str,
    source_state: &str,
    u: &tdb_qa::evidence::UnmappedObservation,
) -> publish::UnmappedRow {
    publish::UnmappedRow {
        source_kind: source_kind.to_string(),
        source_id: source_id.to_string(),
        source_state: source_state.to_string(),
        target: u.target.clone(),
        emitter_operation: u.emitter_operation.clone(),
        case_id: u.case_id.clone(),
        outcome: u.outcome.as_str().to_string(),
        fixture_sha256: u.fixture_sha256.clone(),
        unresolved: u.unresolved.clone(),
        detail: u.detail.clone(),
    }
}

/// Validate every active input and report drift against the published set. Writes nothing.
fn check(options: &Options) -> Result<ExitCode, String> {
    report_in_progress(options);
    let (set, inputs) = match load(options) {
        Ok(loaded) => loaded,
        // A malformed **active** input — including a journal whose identity cannot be established
        // — is exit 3, the contract's code for it, not the generic error code (R-ACTIVE-STRICT).
        Err(message) => {
            eprintln!("qa-evidence: malformed active input: {message}");
            return Ok(ExitCode::from(3));
        }
    };
    let violations = validate(&set);
    if !violations.is_empty() {
        eprintln!(
            "{} validation failure(s) in active inputs:",
            violations.len()
        );
        for violation in &violations {
            eprintln!("  {violation}");
        }
        return Ok(ExitCode::from(3));
    }
    let built = publish::build(&merge::merge(&set), &inputs);
    let mut drift = Vec::new();
    for (name, contents) in &built.files {
        let path = options.data.join(name);
        let published = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        if published != contents.replace("\r\n", "\n") {
            drift.push(name.clone());
        }
    }
    if !drift.is_empty() {
        eprintln!(
            "{} published file(s) differ from a fresh build:",
            drift.len()
        );
        for name in &drift {
            eprintln!("  {name}");
        }
        return Ok(ExitCode::from(2));
    }
    println!(
        "ok: {} row(s), no validation failure, no drift",
        merge::merge(&set).rows.len()
    );
    Ok(ExitCode::SUCCESS)
}

/// Name every unsealed emitter journal. Since C0-S2-INT1-R1 these **are** merge inputs — incomplete
/// execution evidence (`evidence::journal`) — so this is a summary of what the merge just read, not
/// a note about something being ignored. It is still not a claim that the emitter that wrote one is
/// dead: nothing here seals a journal.
fn report_in_progress(options: &Options) {
    let dir = options.root.join("auto").join("in-progress");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    if names.is_empty() {
        return;
    }
    names.sort();
    eprintln!(
        "note: {} unsealed emitter journal(s) under {} — incomplete execution evidence: cited, never selected, and blocking wherever a failure is persisted. Nothing here is sealed, and none of it is treated as a finished run:",
        names.len(),
        dir.display()
    );
    for name in &names {
        eprintln!("  {name}");
    }
}

/// Render the set into `--out`. Never writes into the published tree.
fn build_cmd(options: &Options) -> Result<ExitCode, String> {
    let out = options.out.clone().ok_or("build needs --out <dir>")?;
    if out.starts_with(&options.data) {
        return Err(format!(
            "--out {} is inside the published tree; build is a non-publishing operation",
            out.display()
        ));
    }
    let (set, inputs) = match load(options) {
        Ok(loaded) => loaded,
        // A malformed **active** input — including a journal whose identity cannot be established
        // — is exit 3, the contract's code for it, not the generic error code (R-ACTIVE-STRICT).
        Err(message) => {
            eprintln!("qa-evidence: malformed active input: {message}");
            return Ok(ExitCode::from(3));
        }
    };
    let violations = validate(&set);
    if !violations.is_empty() {
        eprintln!(
            "{} validation failure(s); nothing rendered:",
            violations.len()
        );
        for violation in &violations {
            eprintln!("  {violation}");
        }
        return Ok(ExitCode::from(3));
    }
    let built = publish::build(&merge::merge(&set), &inputs);
    // Boundary enforcement is in `write_to` itself; the CLI check above is a convenience
    // diagnostic, not the guard (R1). R2: the layout is mandatory, so this is the same guard a
    // library caller gets — there is no weaker entry point to reach for.
    let layout = layout(options)?;
    built
        .write_to(&out, &inputs, &layout)
        .map_err(|e| e.to_string())?;
    println!("built {} file(s) into {}", built.files.len(), out.display());
    Ok(ExitCode::SUCCESS)
}

fn publish_cmd(options: &Options) -> Result<ExitCode, String> {
    if options.recover {
        let report = publish::recover(&options.data).map_err(|e| e.to_string())?;
        println!("recover: phase {} — {}", report.phase_found, report.action);
        for file in &report.files {
            println!("  {file}");
        }
        for path in &report.retained {
            println!("  retained: {}", path.display());
        }
        return Ok(ExitCode::SUCCESS);
    }
    let from = options
        .from
        .clone()
        .ok_or("publish needs --from <dir> or --recover")?;
    let (set, inputs) = match load(options) {
        Ok(loaded) => loaded,
        // A malformed **active** input — including a journal whose identity cannot be established
        // — is exit 3, the contract's code for it, not the generic error code (R-ACTIVE-STRICT).
        Err(message) => {
            eprintln!("qa-evidence: malformed active input: {message}");
            return Ok(ExitCode::from(3));
        }
    };
    let violations = validate(&set);
    if !violations.is_empty() {
        eprintln!(
            "{} validation failure(s); refusing to publish",
            violations.len()
        );
        return Ok(ExitCode::from(3));
    }
    let layout = layout(options)?;
    let report = publish::publish(
        &from,
        &options.data,
        &layout,
        &inputs.merge_input_digests,
        &PublishOptions::default(),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "published {} file(s) [id {}]",
        report.installed.len(),
        report.id
    );
    println!(
        "prior generation retained at {}",
        report.retained_generation.display()
    );
    Ok(ExitCode::SUCCESS)
}

/// Freeze the legacy sources and render a migration candidate. Writes **only** into `--out`; never
/// into `manual/`, never into the published tree, and never into the frozen sources.
fn migrate_cmd(options: &Options) -> Result<ExitCode, String> {
    use tdb_qa::evidence::migrate;
    let frozen = options
        .frozen
        .clone()
        .ok_or("migrate needs --frozen <dir>")?;
    let out = options.out.clone().ok_or("migrate needs --out <dir>")?;
    let id = options
        .id
        .clone()
        .ok_or("migrate needs --id <migration-id>")?;
    if out.starts_with(&options.root) || out.starts_with(&options.data) {
        return Err(format!(
            "--out {} is inside the evidence or published tree; a candidate is never auto-installed",
            out.display()
        ));
    }
    if !frozen.join("freeze.json").is_file() {
        // Freezing is part of migrating, but it is a distinct, recorded step.
        migrate::freeze(&options.repo, &frozen, "unknown").map_err(|e| e.to_string())?;
        println!(
            "froze {} source(s) into {}",
            migrate::SOURCES.len(),
            frozen.display()
        );
    }
    let layout = layout(options)?;
    let report = migrate::migrate(&frozen, &out, &id, &layout).map_err(|e| e.to_string())?;
    println!("migration {}", report.migration_id);
    println!("  source cells      {}", report.source_cells);
    println!("  accounting rows   {}", report.accounting_rows);
    println!("  claims            {}", report.claims);
    println!("  unmapped          {}", report.unmapped);
    println!(
        "  manual records    {} (migration never writes one)",
        report.records
    );
    println!("  divergence rows   {}", report.divergence_rows);
    println!("  candidate digest  {}", report.candidate_set_sha256);
    if report.accounting_rows != report.source_cells {
        eprintln!(
            "R-ACCOUNT violated: {} rows for {} cells",
            report.accounting_rows, report.source_cells
        );
        return Ok(ExitCode::from(3));
    }
    Ok(ExitCode::SUCCESS)
}

/// The only path into layer B (R-MIG-SEP). Refuses without a matching approval digest.
fn migrate_install_cmd(options: &Options) -> Result<ExitCode, String> {
    use tdb_qa::evidence::migrate;
    let from = options
        .from
        .clone()
        .ok_or("migrate-install needs --from <candidate dir>")?;
    let approve = options
        .approve
        .clone()
        .ok_or("migrate-install needs --approve <approval.toml>")?;
    let id = options
        .id
        .clone()
        .ok_or("migrate-install needs --id <migration-id>")?;
    let report =
        migrate::install(&from, &approve, &options.root, &id).map_err(|e| e.to_string())?;
    println!(
        "installed {} claim(s) and {} manual record(s) into {}",
        report.claims_installed,
        report.records_installed,
        options.root.display()
    );
    Ok(ExitCode::SUCCESS)
}

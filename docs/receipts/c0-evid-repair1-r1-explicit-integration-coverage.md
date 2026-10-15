# C0-EVID-REPAIR1-R1 — explicit real-registry coverage

**Package:** C0-EVID-REPAIR1-R1, a bounded correction of [C0-EVID-REPAIR1](c0-evid-repair1.md) ·
**Date:** 2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` ·
**HEAD at start and at end:** `ef0fdd74b37a15666d4eba25f66107bdc8948bd8` (unchanged) ·
**Index at start and at end:** empty

> **Result: the conditional-pass integration test is gone.** Real-registry integration is now
> `#[ignore]`d by default with a reason naming the prerequisite, and when explicitly selected it
> **fails** on an absent, unreadable or empty registry. Default runs report it as
> **ignored — an explicit coverage gap**, never as a pass.
>
> **Counts change: 223 passed / 0 failed → 222 passed · 0 failed · 1 ignored**, identical in both
> the CRLF and the all-LF representation. The missing case is the integration test, which
> C0-EVID-REPAIR1 wrongly counted as executed.
>
> Nothing staged, committed or pushed. No production migration change, no fixture import, no
> registry or capability data acquired or committed, no game-data access, no generator against the
> live tree, no CI change, no broad reset, and no prior candidate or evidence deleted.

**Preserved unchanged:** the synthetic A11 case, the synthetic divergence case, the synthetic
builders, and the whole Checkpoint-B comparison repair (`evidence::csv::compare_records`, the eight
guards, the JSON restricted-format argument, the clippy fix). **Live edits this package:
`crates/tdb-qa/tests/evidence_migration.rs` and receipt documentation only.**

---

## 1. Correction to C0-EVID-REPAIR1 (labelled)

Two claims in [c0-evid-repair1.md](c0-evid-repair1.md) are **withdrawn**:

| Withdrawn claim | Correction |
|---|---|
| §3.2 / §1: *"`real_headmesh_registry_integration_is_pending_separate_coverage` is **not a skip**"*, and the §3.4 row calling it "**pending** — unmet at this commit, recorded not skipped" | **Withdrawn.** The case was a **conditional pass**: with the registry absent it asserted only that the registry was absent and then returned green. That is worse than a skip — a skip is at least visible in the run output, whereas this reported `ok` and made an unmet requirement indistinguishable from a satisfied one. It is now `#[ignore]`d, so the gap is reported by the test harness itself |
| §1 / §6.2: *"**223 passed · 0 failed**"* presented as the state of the repaired slice | **Withdrawn as stated.** Those 223 included the conditional-pass case, so the figure implied that real-registry integration had **executed**. It had not, and could not: the registry is not committed at HEAD. The corrected figure is **222 passed · 0 failed · 1 ignored** (§4). Every other number in that receipt — the 6 mutations, the merge distribution, the 12-CSV preservation, the clippy and formatting results — is unaffected and stands |

**What this does not mean.** The synthetic A11 invariant did **not** fail and was not weakened; it
still runs on every default invocation and still proves R-NO-BACKFILL. An ignored integration test
is **an explicit coverage gap, not a failed invariant, and not a licence to invent evidence about
any target.** No qualification, count or status anywhere was changed to compensate for it.

A pointer to this correction was added at the top of `c0-evid-repair1.md`; nothing else in that
receipt was rewritten.

---

## 2. The replacement test

`real_headmesh_registry_integration_over_the_committed_registry`, in
`crates/tdb-qa/tests/evidence_migration.rs`.

**Ignored by default, with the prerequisite in the reason.** The attribute reads:

```
#[ignore = "requires defs/appearance/headmesh-matrix.toml, which is NOT committed at HEAD
            (C0-EVID-PROBE1 §4.1); this coverage is unmet until that registry lands — run with
            `--ignored real_headmesh_registry` to execute it"]
```

so a default run prints the gap and how to close it, verbatim, next to the test name (§4.1).

**When explicitly selected it does all six things the package requires:**

| # | Requirement | How |
|---|---|---|
| 1 | fail clearly if the registry is **absent** | `assert!(matrix.is_file(), …)` naming the full path, the prerequisite, and an explicit instruction **not** to substitute the synthetic fixture |
| 2 | fail clearly if the registry is **unreadable/malformed** | `headmesh_pair_keys(…).unwrap_or_else(|e| panic!(…))` naming the file and the parse error, and stating that a registry which does not parse is a **failure of this coverage, not an absence of it** |
| 3 | require a **non-empty** set of pair keys | `assert!(!keys.is_empty(), …)` — *"An empty registry is a failure here, not coverage"* |
| 4 | verify **distinct identities** and **conversion components** | distinct-key count over the full key strings, and a per-key `conversion.is_some()` with the offending key named. The registry's own pair/destination shape is **printed, not asserted against a fixed number**, so no count from an earlier document is re-imposed on it |
| 5 | exercise the documented **migration/install/merge** path on **sandbox** data | `synthetic_frozen_and_migrated` → `approve_and_install` → `store::load_legacy` → `merge::merge`, all in scratch |
| 6 | prove destination-level claims **do not qualify the real pair keys** | the sandbox `heads` claim is located, asserted `!qualifies()` (R-LEGACY), then every **real** key's merged row is asserted `NotAttempted` / `None` / not `GameVerified` / not `AutomatedQualified` |

**Nothing is acquired.** The test reads the registry *if the tree already has it* and never writes,
copies or fetches it. The destination-level claim it merges against comes from the **synthetic**
sandbox sources, so **no hand-edited capability data is read** — the same boundary C0-EVID-REPAIR1
§3.3 established.

**Separate labelling is preserved.** The synthetic case's doc now says it "always runs" and is
"**not** a substitute for" the real case; the real case's doc says A11 proves the *invariant* while
this one proves it *over the live registry*. The module-level note points at the ignored case by
name.

---

## 3. Load-bearing proof of all four outcomes

Executed in the scratch candidate. P1–P3 place a **clearly labelled scratch-only** file at the
registry path to drive the guards; each was deleted immediately afterwards, and the candidate tree
verified byte-identical (§5). **The real registry was never acquired or copied.**

| Probe | Registry state | Expected | Observed |
|---|---|---|---|
| **P0** | **absent** (a clean checkout of HEAD) | fail, naming the prerequisite | **exit 101** — `real-registry integration requires …headmesh-matrix.toml, which is absent or is not a file. It is not committed at HEAD (C0-EVID-PROBE1 §4.1). Commit the registry, or leave this case ignored — do NOT substitute the synthetic fixture …` |
| **P1** | present but declares **no pairs** | fail — empty is never success | **exit 101** — `… declared no `[[pair]]` entries, so this case proved nothing about pair identity. An empty registry is a failure here, not coverage.` |
| **P2** | present but **malformed TOML** | fail, naming file and error | **exit 101** — `the committed registry … could not be read as pair keys: … TOML parse error at line 2, column 7` |
| **P3** | present and **valid** (2 synthetic pairs on 1 destination) | the **whole body** executes and passes | **exit 0**, `1 passed`, printing `real registry: 2 pair(s) across 1 destination(s)` — so requirements 4–6 really run rather than being unreachable behind the absence guard |

P3 is the one that proves the case is not decorative: the distinctness, conversion,
migration/install/merge and non-qualification assertions all executed against a registry at that
path and passed.

---

## 4. Verification in fresh isolated candidates

Two new candidates from `git archive` of `ef0fdd7` (archive SHA-256
`e35186907d6e8aae24a83dd870b2a2ea0434236099de507557f3a5c0babf95cd`, 1160 files) plus the same
**34-file** overlay and the same two C0 lockfile lines: `H:\c0-evid-repair1-r1\candidate-crlf\` and
`…\candidate-lf\` (the 41 committed generated CSV/JSON artefacts converted to LF), **1186 files
each**. Distinct build-output directories: `target-crlf`, `target-lf`, `target-clippy`. Prior
candidates untouched (§5).

### 4.1 Results

| # | Command | Tree | Exit | Result |
|---|---|---|---|---|
| 1 | `cargo test -p tdb-qa --locked --offline --no-fail-fast` | **CRLF** | **0** | **222 passed · 0 failed · 1 ignored** |
| 2 | `cargo test -p tdb-qa --locked --offline --no-fail-fast` | **LF** | **0** | **222 passed · 0 failed · 1 ignored** — identical |
| 3 | `cargo clippy -p tdb-qa --all-targets --locked --offline -- -D warnings` | CRLF | **0** | **zero diagnostics** |
| 4 | `rustfmt --check` on all 7 files C0-EVID-REPAIR1 touched | live bytes | **0** | clean |
| 5 | explicit `-- --ignored real_headmesh_registry`, registry **absent** | CRLF | **101** | fails with the §3 P0 diagnostic — **retained in `logs\20-crlf-ignored-selected.log`** |

Per-target for check 1: lib **72** · `qa_evidence` 0 · `qa_inventory` 0 · `evidence_acceptance` 32 ·
`evidence_discovery` 13 · `evidence_emit` 7 · `evidence_file_reads` 9 · `evidence_journal` 31 ·
**`evidence_migration` 13 passed + 1 ignored** · `evidence_publication` 36 · `inventory` 9.

The default run prints the gap on its own line:

```
test real_headmesh_registry_integration_over_the_committed_registry ... ignored, requires
defs/appearance/headmesh-matrix.toml, which is NOT committed at HEAD (C0-EVID-PROBE1 §4.1); this
coverage is unmet until that registry lands — run with `--ignored real_headmesh_registry` to execute it
```

### 4.2 The four-emitter capture was **not** repeated

No emitter, comparison helper or production migration code changed in this package — the only edited
source file is `evidence_migration.rs`, which no emitter uses. C0-EVID-REPAIR1 §6.3's capture and
deterministic merge (4 completed runs, byte-identical double build, `ok: 2175 row(s)`, 140
`automated-qualified`, 0 `game-verified`) therefore still describes this tree, and **nothing here
re-establishes it or claims to**.

### 4.3 One fact worth stating plainly

The **live working tree** carries `defs/appearance/headmesh-matrix.toml` as an **untracked
feature-side file**, so in *that* tree an explicit `--ignored` run would execute against the real
registry. The coverage gap is against **HEAD** — a clean checkout has no registry, which is what
check 5 measured. This package ran no test in the live tree, so no result here is a statement about
the real registry's contents, and none is claimed.

---

## 5. Preservation

| Check | Result |
|---|---|
| `git rev-parse HEAD` | `ef0fdd74…` → `ef0fdd74…` — **unchanged** |
| Index | **empty** → **empty** |
| `git status --porcelain=v1 -z -uall` SHA-256 | `7c40bc7285af38e3…` → **`7c40bc7285af38e3…` — byte-identical** (the two files this package writes were already `M`/`??`) |
| Candidate tree (`candidate-crlf`) | **1186 files, 1186 OK, 0 FAILED** after all probes; the scratch registry probes left nothing behind (`headmesh-matrix.toml` absent again) |
| Committed CSVs, both candidates | **12 OK / 12 OK** — none rewritten. `QA_WRITE` unset throughout |
| `defs/appearance/` and `docs/audits/**` in the live tree | **not written**; the hand-edited `capability.csv` still carries only its pre-existing +1 row |
| Prior candidates and evidence | all retained — `c0-evid-probe1` 21 227 files, `c0-evid-repair1` 16 822, `c0-s2-int1` **259 OK / 0 FAILED**, `c0-slice2` 8 580, `c0-can-commit2` 2 292. **Nothing deleted** |
| Publication / migration install / staging / commit / push | **none** |

---

## 6. Remaining gaps

1. **Real-registry integration is unmet at HEAD and now says so in every default run.** It closes
   when `defs/appearance/headmesh-matrix.toml` is committed and the case is run with `--ignored`;
   until then it is a declared gap, and no synthetic case substitutes for it.
2. C0-EVID-REPAIR1 §8's items are unchanged: the `migrate` behaviours referred for **separate**
   review (the hardcoded `nhllegacy-xbox360` target for `capability.frozen.csv`, and `"unknown"` for
   absent registry fields) were **not** touched here; the `heads` divergence disposition remains
   C0.4-owned; Linux/CI is unverified; and no workspace-wide `fmt`/clippy claim is made.
3. **The ignored case is not counted as coverage anywhere.** It contributes no qualification, no
   observation and no evidence row, and the published matrix is unchanged.

---

## 7. Files changed

| File | Change |
|---|---|
| `crates/tdb-qa/tests/evidence_migration.rs` | conditional-pass case replaced by the ignored-by-default integration case; two cross-references relabelled |
| `docs/receipts/c0-evid-repair1.md` | review-correction banner pointing here (no other text altered) |
| `docs/receipts/c0-evid-repair1-r1-explicit-integration-coverage.md` | this record |

**Nothing staged, committed or pushed. Stopping here for Codex review.**

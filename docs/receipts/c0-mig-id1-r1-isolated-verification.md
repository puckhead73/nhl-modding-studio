# C0-MIG-ID1-R1 — isolated verification and accurate execution record

**Package:** C0-MIG-ID1-R1, a verification-only correction of [C0-MIG-ID1](c0-mig-id1.md) ·
**Date:** 2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling`

| Live-tree state | Before | After |
|---|---|---|
| `git rev-parse HEAD` | `ef0fdd74b37a15666d4eba25f66107bdc8948bd8` | identical |
| index (`git diff --cached --stat`) | empty (0 bytes) | identical |
| `git status --porcelain` | 239 entries | **byte-identical** (diff empty) |
| 37 overlay files (SHA-256 each) | captured | **all 37 unchanged** |
| `docs/audits/data/game-qa-matrix.csv` | `5d2dbd3c5a79257c…` | identical |
| `docs/audits/data/asset-capability-matrix.csv` | `3767a2c0143293ce…` | identical |
| `docs/audits/data/assets/nhllegacy/xbox360/capability.csv` | `bffb4ac6c0b74f0c…` | identical |

> **Result.** C0-MIG-ID1's numbers reproduce in isolation. In two fresh candidates built from
> `ef0fdd7` plus an enumerated overlay — never from the dirty tree — `cargo test -p tdb-qa --locked
> --offline` gives **228 passed · 0 failed · 1 ignored** in both the LF and the CRLF candidate, the
> real-registry case is **ignored** in both, and scoped clippy and fmt are clean in both. The three
> neutralization probes reproduce exactly, in disposable copies, with the candidates left
> bit-identical.
>
> **Two corrections came out of building the candidates**, both recorded below: the overlay needed
> one file C0-MIG-ID1 never identified as an input (`defs/qa/scopes.toml`, §2.3), and the workspace
> required a manifest/lock derivation because HEAD does not contain two members the live root
> manifest names (§2.4).
>
> C0-MIG-ID1's receipt now carries a labelled §7 deviation recording what actually ran against the
> live tree, what was temporarily mutated there (including `docs/audits/data/`), which hashes were
> and were **not** captured, and that an external `CARGO_TARGET_DIR` isolated build output only.
>
> **No implementation change of any kind was made by this package.** The only live-repository
> writes are `docs/receipts/c0-mig-id1.md` §7 and this file. No build, test, format, lint or
> temporary mutation was run against live sources. Nothing staged, committed or pushed.

---

## 1. Correction to C0-MIG-ID1 (§1 of the package)

Appended to that receipt as **§7 "Deviation — recorded by C0-MIG-ID1-R1"**, and summarised here.

| Claim in C0-MIG-ID1 | Correction |
|---|---|
| §3 "Every invocation used a dedicated build-output directory outside the repository … the repository's own `target/` was neither read nor written" | True but misleading. `CARGO_TARGET_DIR` isolates **build output only**; every compile read the **live working tree**. There was **no source isolation** |
| §6 "no `docs/audits/data/**` edit" | **Withdrawn.** Three files under `docs/audits/data/` were rewritten in place twice by the CRLF conversion and restored: `game-qa-matrix.csv`, `asset-capability-matrix.csv`, `assets/nhllegacy/xbox360/capability.csv` |
| §4 "the tree at the end … was byte-identical to the tree at its start" | Narrowed. Endpoint equality was verified for the **37 files in the retained manifest**; it is not evidence that no other live file was written, and the intermediate states (CRLF sources, neutralized production code, rewritten audit CSVs) were genuinely live while the runs happened. mtimes changed and were not restored |
| §3/§4 hash claims | Enumerated exactly in §7.3: the 37-file before-manifest and its two `mismatches: 0` restores **were** captured (second one retained; the first was overwritten by the second), `migrate.rs` **was** hash-verified after each probe; **no** pre-normalisation hash of `evidence_migration.rs` and **no** before/after `git status`/HEAD/index capture existed |

Everything in §7 is drawn from this session's retained artefacts —
`…/scratchpad/lf-manifest.txt` (37 entries), `…/scratchpad/lf-snapshot/`,
`…/scratchpad/migrate.rs.fixed` (`cf9851c3…`) and `…/scratchpad/crlf.py`. Nothing was reconstructed
where the evidence no longer exists; where it does not exist, §7.3 says so.

---

## 2. Candidate identity

### 2.1 Directories

| Purpose | Path |
|---|---|
| Base extraction of HEAD | `…/scratchpad/r1/base` (`git archive --format=tar HEAD \| tar -x`, 1160 files) |
| LF candidate | `…/scratchpad/r1/candidate-lf` — **1186 files** |
| CRLF candidate | `…/scratchpad/r1/candidate-crlf` — **1186 files** |
| Build output | `…/r1/target-lf`, `…/r1/target-crlf`, `…/r1/target-probe-{N1,N2,N3}` (all outside every candidate) |
| Probe copies | `…/r1/probe-N1`, `-N2`, `-N3` |

`…/scratchpad` is `H:\TEMP\claude\e--Repositories-nhl-modding-studio\4eaf7e39-…\scratchpad`. The
prior package's target directories (`target-c0-mig-id1`, `-crlf`) and its snapshot/manifest were
**read but never modified or deleted**.

### 2.2 Construction

1. `git archive HEAD` extracted into `base` — no working-tree bytes, no `.git`.
2. `crates/tdb-qa/` **removed wholesale** from the copy, so no HEAD-era file in that crate can
   survive under the overlay.
3. The enumerated overlay copied in from the live tree (§2.3).
4. CRLF candidate only: the 37 files listed in `crlf-converted-membership.txt` converted to CRLF —
   all `.rs`/`.toml` under `crates/tdb-qa/` plus the three `docs/audits/data/` CSVs (the same
   membership C0-MIG-ID1 used).
5. Workspace manifest/lock derivation (§2.4).
6. Full SHA-256 manifest taken of every file in each candidate (`manifest-{lf,crlf}-before.txt`).

**The dirty tree was not copied.** Everything outside the overlay is HEAD. In particular the
candidates carry **HEAD's** `docs/audits/data/assets/nhllegacy/xbox360/capability.csv`
(`4944fb8f2303…`), not the live modified one (`bffb4ac6c0b7…`) — a deliberate membership decision,
since that modification belongs to another package. The counts are unaffected: the migration cases
count cells from the frozen bytes rather than from a hard-coded total.

### 2.3 Overlay membership — 37 files

Full path+hash list: `…/r1/overlay-manifest-before.txt` (re-verified unchanged afterwards as
`overlay-manifest-after.txt`).

| Group | Files |
|---|---|
| Root manifests | `Cargo.toml`, `Cargo.lock` |
| C0-MIG-ID1's two changed files | `crates/tdb-qa/src/evidence/migrate.rs` `71a7e75b4162e3a0c9153fe010ecf684a32a7539e5da1e2a1e095a1f66f90b41` · `crates/tdb-qa/tests/evidence_migration.rs` `5a54c19ee37f101cd0798b736c658bc7f8ebca8008fa5992ab05c0dca7d12ed2` |
| Rest of the evidence slice | `crates/tdb-qa/` — `Cargo.toml`, `build.rs`, `src/bin/{qa_evidence,qa_inventory}.rs`, `src/{lib,catalog,diff,manifest,matrix,schema}.rs`, all 16 `src/evidence/*.rs`, and `tests/{evidence_acceptance,evidence_discovery,evidence_emit,evidence_file_reads,evidence_journal,evidence_publication,inventory}.rs` |
| **Correction** | `defs/qa/scopes.toml` |

**`defs/qa/scopes.toml` was a missing input.** The first candidate build omitted it — it is
untracked, and C0-MIG-ID1 never identified it as a slice input. Four `evidence_discovery` cases then
failed (`Withheld` where `AutomatedQualified`/`Blocked` was expected): with the scope definitions
absent, every freshness digest is `unknown` under R-FRESH-UNKNOWN and nothing can be fresh. That is
a **true isolation finding about the overlay, not about the code** — the file exists in the live
tree, which is why the live runs passed. It was added to the overlay by enumeration (`grep` for the
repo-relative paths the slice reads at runtime), the candidates were rebuilt, and the suite passed.
The only other repo path the slice reads is `defs/appearance/headmesh-matrix.toml`, which is
**deliberately absent** (§3.2).

### 2.4 Required manifest and lock derivation

The live root `Cargo.toml` names two workspace members that **do not exist at HEAD** —
`crates/tdb-headmesh` and `crates/blender-job` (both untracked). Cargo refuses to load the workspace
without them, and importing two whole untracked crates would be copying the dirty tree.

Derivation, applied identically to both candidates:

- those two `members` lines removed from the candidate root `Cargo.toml`
  (`b2fc2b2a8a11615532c35e4b5b11fb45b4e7900601192f0aa3159a9df64dbcb5`);
- the lock minimally re-derived once, offline, from the **live** lock
  (`cargo metadata --offline`), giving `d4c15e1c5f69cdff29931d60f26d2f0ae538d48961fa504fa25bbba18c527351`, copied byte-identical into both candidates.

The derived lock differs from the live lock by **41 lines**, all consequences of the candidate being
HEAD: the `blender-job` and `tdb-headmesh` package entries and their unique transitive deps
(`zune-core`, `zune-jpeg`) are gone, and `tdb-qa` is no longer a dependency of `studio-domain` —
HEAD's `studio-domain/Cargo.toml` does not declare it, the live one does. Nothing else moved; no
external dependency version changed. Retained: `…/r1/lock-before-derivation.lock`,
`…/r1/derived.lock`.

Everything after this point ran `--locked --offline`.

---

## 3. Verification

Commands, run from inside each candidate with `QA_WRITE` and `QA_EVIDENCE_ROOT` **unset**
(`env -u QA_WRITE -u QA_EVIDENCE_ROOT`, verified unset in the shell beforehand):

```sh
cd <candidate> && CARGO_TARGET_DIR=<r1/target-*> \
  cargo test   -p tdb-qa --locked --offline
  cargo clippy -p tdb-qa --all-targets --locked --offline -- -D warnings
  cargo fmt    -p tdb-qa -- --check
```

### 3.1 Results

| Target | LF candidate | CRLF candidate |
|---|---|---|
| `src/lib.rs` unit tests | 72 | 72 |
| `bin/qa_evidence`, `bin/qa_inventory` | 0 + 0 | 0 + 0 |
| `evidence_acceptance` | 32 | 32 |
| `evidence_discovery` | 13 | 13 |
| `evidence_emit` | 7 | 7 |
| `evidence_file_reads` | 9 | 9 |
| `evidence_journal` | 31 | 31 |
| `evidence_migration` | **19 passed · 1 ignored** | **19 passed · 1 ignored** |
| `evidence_publication` | 36 | 36 |
| `inventory` | 9 | 9 |
| doc-tests | 0 | 0 |
| **Total** | **228 passed · 0 failed · 1 ignored** | **228 passed · 0 failed · 1 ignored** |
| `clippy … -D warnings` | clean, exit 0 (`Checking tdb-qa` / `Compiling tdb-qa` present in the log) | clean, exit 0 |
| `fmt -- --check` | clean, exit 0, empty output | clean, exit 0, empty output |

Logs retained: `log-{lf,crlf}-test.txt`, `log-{lf,crlf}-clippy.txt`, `log-{lf,crlf}-fmt.txt`.

`touch crates/tdb-qa/src/lib.rs` was used inside each **candidate** to force a clippy recheck; it
changes mtime only and the manifests hash content, so candidate membership is unaffected (§3.4).

### 3.2 The real-registry case stayed ignored — structurally

Both candidate runs report:

```
test real_headmesh_registry_integration_over_the_committed_registry ... ignored, requires
defs/appearance/headmesh-matrix.toml, which is NOT committed at HEAD (C0-EVID-PROBE1 §4.1) …
```

`defs/appearance/headmesh-matrix.toml` is **absent from both candidates** (confirmed: `ls` fails on
it in `candidate-lf`), because it is untracked and was deliberately excluded from the overlay. The
case was never selected with `--ignored`, and no candidate could have read the live registry even if
it had been. No game data was read by anything in this package.

### 3.3 Neutralization probes, in disposable copies

Each probe is a fresh copy of `candidate-lf` (excluding `anchors/`, `corpus/`, `target/` — none of
which `evidence_migration` reads; the exclusion keeps a probe ~4 MB instead of ~700 MB), mutated
only in its own copy, run as `cargo test -p tdb-qa --test evidence_migration --locked --offline`
with its own target directory, then left in place as evidence.

| Probe | Mutation | Result |
|---|---|---|
| **N1** | `class=head-authored`, `recipe={id_scheme}` restored | **14 passed · 5 failed · 1 ignored** — `a_declared_id_scheme_never_becomes_the_recipe_component`, `the_conversion_class_is_not_inferred_…`, `unknown_conversion_identity_withholds_…`, `distinct_rows_that_collapse_…`, `a11_…_no_destination_claim_is_back_filled` |
| **N2** | collision-rejection loop deleted | **17 passed · 2 failed · 1 ignored** — `exact_duplicate_pairs_…`, `distinct_rows_that_collapse_…` |
| **N3** | absent `target` defaulted to `nhllegacy-xbox360` | **18 passed · 1 failed · 1 ignored** — `a_pair_with_no_target_still_fails_closed` |

Identical to the live-tree results reported in C0-MIG-ID1 §4, case for case.

### 3.4 Candidate membership, measured before and after

Full-tree SHA-256 manifests (excluding the external `target` dirs) taken before and after all
verification:

| Candidate | Files before | Files after | Differing entries |
|---|---|---|---|
| `candidate-lf` | 1186 | 1186 | **0** |
| `candidate-crlf` | 1186 | 1186 | **0** |

No test wrote into a candidate: the suite's sandboxes live under `%TEMP%\tdb-qa-evidence-mig\…`,
build output was external, and `--locked` guaranteed no lock rewrite. Nothing generated, published
or installed; no evidence tree was created inside a candidate.

---

## 4. Live-tree preservation

Captured before any R1 work and re-captured after it, all identical (table at the top of this
receipt): HEAD, the empty index, the 239-entry `git status --porcelain`, the SHA-256 of every one of
the 37 overlay files, and the SHA-256 of the three `docs/audits/data/` inputs. Retained:
`live-head-before.txt`, `live-index-before.txt`, `live-status-{before,after}.txt`,
`overlay-manifest-{before,after}.txt`, `audit-inputs-before.txt`.

**No build, test, clippy, fmt, conversion or neutralization was run against live sources by this
package.** Live reads were limited to `git` queries, `sha256sum`, `grep`/`ls`, and copying overlay
files into the candidates. The only live writes are §7 of `docs/receipts/c0-mig-id1.md` and this
receipt.

---

## 5. What this does and does not establish

**Establishes:** C0-MIG-ID1's implementation and tests behave identically outside the dirty tree,
under `--locked --offline`, in both line-ending representations, on a HEAD base; its three
neutralization claims are reproducible; and its counts are not an artefact of live-tree state.

**Does not establish:**

1. **Real-registry readiness — still not established.** The registry is untracked, absent from both
   candidates, and the integration case remains ignored and unexecuted. Unchanged from C0-MIG-ID1 §5.
2. **U2 is still open.** No `conversion.class` vocabulary exists; `class = unknown` remains the
   honest placeholder and every headmesh pair key remains unqualifiable by construction.
3. **Nothing about the slice's committability.** The candidates required a derived lock and a
   trimmed workspace manifest to build at all (§2.4), and one live-only input (`defs/qa/scopes.toml`)
   had to be enumerated by hand (§2.3). Those are properties of the uncommitted slice, and this
   package did not repair them.
4. **Nothing about the wider workspace.** Only `-p tdb-qa` was built or tested; no workspace-wide
   run, no emitter recapture, no publication.
5. **The C0-MIG-REVIEW1 checkpoint-1 hazard** (the `capability.frozen.csv` → `nhllegacy-xbox360`
   attribution and the unlinked `SOURCES` constant) is untouched and stands as recorded.

---

## 6. Limits honoured

No code repair, no implementation change, no schema change, no feature import, no registry or
game-data access, no real-registry execution, no live migration, publication or install, no CI
change, no generator run, no broad reset, nothing staged, committed or pushed, and no prior
candidate, snapshot, manifest, log or receipt deleted or modified — the C0-MIG-ID1 receipt was
**appended to**, under a labelled heading, with its original text left intact. Stopping here for
Codex review.

# C0-NEXT1 — reconcile current state and prepare evidence activation

**Package:** C0-NEXT1 · **Date:** 2026-09-13 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **HEAD before and after:** `ef0fdd74b37a15666d4eba25f66107bdc8948bd8` ·
**Index before and after:** empty · **`git status --porcelain` before and after:** 269 entries, byte-identical

> **Result.** The evidence system has an **exact, reproducible integration boundary**: HEAD plus
> **35 enumerated files** plus **one lock line** builds and passes `cargo test -p tdb-qa --locked
> --offline` in isolation — **228 passed · 0 failed · 1 ignored** in both an LF and a CRLF candidate —
> with scoped clippy and fmt clean, the committed `tdb-canonical` consumer still green (200 passed)
> against it, the real-registry case reported as an explicit ignored gap, and `qa-evidence build`
> byte-deterministic on a disposable non-empty evidence tree (build twice → identical; publish into
> a disposable data dir → `check` exits 0 → rebuild → identical). Root `Cargo.toml` needs **no**
> change for evidence integration; the untracked `tdb-headmesh`/`blender-job` members and the
> `studio-domain → tdb-qa` dependency belong to other work and are **not** part of the slice.
>
> Status documents corrected in place, dated and attributed: `ef0fdd7` committed the canonical
> contract slice; the evidence slice is implemented, tested and isolation-verified but **not
> committed and not activated**; the external headmesh workstream and its integration dependency
> are recorded in the roadmap (C5) and in CLAUDE.md; the NHL 19 user-reported head-import
> successes are retained at fixture scope and promote nothing.
>
> No live build, test or mutation; no live migration, publication, generator run, game install or
> capability promotion; nothing staged, committed or pushed. Live writes: four documents (§1).

---

## 1. Changes — exact files

| File | Change |
|---|---|
| `docs/plans/project-completion-roadmap.md` | **C0 status block:** header attribution; item 4 row (accepted/committed/activated cells); item 7 row (committed cell); the "nothing beyond items 1–2 is committed" paragraph replaced by a dated, attributed correction. **C5:** a "Direction update (2026-09-13)" block and a "User-reported runtime successes" paragraph appended after the exit criterion. No other section touched |
| `CLAUDE.md` | One paragraph under *Hard rules* recording the external headmesh workstream and the do-not-inspect rule (already a modified tracked file; the user's own template-transfer paragraph above it is left as written) |
| `docs/STATE-OF-THE-REPO.md` | New dated **§7 Status update — 2026-09-13**; §§1–6 untouched |
| `docs/receipts/c0-next1-evidence-activation.md` | This receipt |

No source, manifest, lock, `defs/`, `anchors/`, `docs/audits/data/**` or `docs/validation/` file was
modified. Historical receipts were not rewritten.

### 1.1 Reconciliation — the five levels, as they stand

| Deliverable | Implemented | Tested | Accepted | Committed | Activated |
|---|---|---|---|---|---|
| Canonical contract (`tdb-canonical`, `defs/canonical`, `defs/adapters`) | yes | 200 cases | C0-CAN-COMMIT1/2 | **yes — `ef0fdd7`** | n/a |
| Registry population (adapters/profiles) | 2/28 units, 1/14 families | yes | mixed | those 3 files only | n/a |
| Evidence system (S1–S3, emitters, migration, publication) | yes | 228 cases | S1/S3 yes; S2-INT1 chain no record; MIG-REVIEW1 yes; **MIG-ID1 + R1 pending** | **no** | **no** |
| `defs/qa/scopes.toml` | yes | via discovery/emit cases | with S2 | **no** (untracked) | n/a |
| Real-registry pair keys | fail-closed code | ignored case | — | registry untracked | **blocked** |
| Headmesh conversion | partial (Studio) | fixture-level | — | uncommitted | **external workstream** |

Stale claims corrected: "nothing in C0 beyond items 1–2 is committed" (roadmap); the absence of the
canonical and evidence systems from STATE-OF-THE-REPO. Nothing was promoted.

---

## 2. The evidence-system integration boundary

### 2.1 Candidate base and construction

- **Base:** `ef0fdd7`, extracted with `git archive` (the `…/scratchpad/r1/base` tree from
  C0-MIG-ID1-R1, reused read-only; 1160 files). **Note:** `git archive` applies the repository's
  `core.autocrlf` filter, so this base is a **CRLF checkout representation** for text files, exactly
  as a Windows clone would be. C0-MIG-ID1-R1 labelled its candidates "LF"/"CRLF" by the state of the
  *slice*; the same convention is used here and stated explicitly.
- **Overlay:** `crates/tdb-qa/` removed from the copy and replaced wholesale by the 35 files below,
  copied from the live tree. Nothing else from the dirty tree.
- **Candidates:** `…/scratchpad/n1/candidate-lf` (slice as it sits in the live tree — LF) and
  `…/n1/candidate-crlf` (the same 35 files plus the three `docs/audits/data/` migration CSVs
  converted to CRLF; 38 files listed in `crlf-converted-membership.txt`). **1186 files each**; full
  SHA-256 manifests before and after all verification: **0 differing entries** in both.
- **Build output:** `…/n1/target-lf`, `…/n1/target-crlf`, outside every candidate.
- `QA_WRITE` and `QA_EVIDENCE_ROOT` unset throughout (verified).

### 2.2 Overlay membership — 35 files (`…/n1/overlay-manifest-before.txt`, path · sha256 · size)

| Group | Files | Status vs HEAD |
|---|---|---|
| Crate manifest | `crates/tdb-qa/Cargo.toml` | modified: `+ toml = "0.8"`, `+ [[bin]] qa-evidence` |
| Build script | `crates/tdb-qa/build.rs` | untracked |
| Binaries | `src/bin/qa_evidence.rs` (untracked), `src/bin/qa_inventory.rs` (modified) | |
| Library | `src/{lib,catalog,diff,manifest,matrix,schema}.rs` | modified |
| Evidence module | `src/evidence/{csv,emit,journal,key,merge,migrate,mod,model,observe,paths,publish,scopes,store,testkit,validate,vocab}.rs` (16) | untracked |
| Tests | `tests/evidence_{acceptance,discovery,emit,file_reads,journal,migration,publication}.rs` (7, untracked), `tests/inventory.rs` (modified) | |
| Scope definitions | **`defs/qa/scopes.toml`** | untracked — a required runtime input (R-FRESH-UNKNOWN): without it every freshness digest is `unknown` and 4 discovery cases fail; found by C0-MIG-ID1-R1 |

C0-MIG-ID1's two changed files are in the set at their recorded digests (`migrate.rs`
`71a7e75b…`, `evidence_migration.rs` `5a54c19e…`). All 35 live files are **unchanged** at the end
of this package.

### 2.3 Manifest and lock treatment — what belongs to evidence integration

| Item | Belongs to | Treatment in the candidate |
|---|---|---|
| Root `Cargo.toml` | — | **HEAD's, unchanged.** `tdb-qa` is already a member; evidence integration needs no root-manifest change |
| `Cargo.lock`: `tdb-qa` gains `"toml 0.8.2"` | **evidence integration** | applied as a **one-line insertion** into HEAD's lock (`toml 0.8.2` is already resolved there for `studio-domain`/`tdb-canonical`; no new package enters). Verified by `--locked` |
| Root `Cargo.toml`: `crates/tdb-headmesh`, `crates/blender-job` members | headmesh workstream | **excluded** — the crates are untracked; these lines are why the live root manifest cannot be overlaid onto HEAD |
| `Cargo.lock`: `blender-job`, `tdb-headmesh`, `zune-*`, `image` additions | headmesh workstream | excluded |
| `Cargo.lock`: `studio-domain` depends on `tdb-qa` | unrelated feature work (live `studio-domain/Cargo.toml`) | excluded |
| `Cargo.lock`: `tdb-rx2` dependency changes (live lock drifted again since 2026-09-09) | `mcd_write` / rx2 work | excluded |

This is the bounded integration defect C0-MIG-ID1-R1 could only work around (it trimmed members and
re-derived the whole lock). The precise fix is the one-line lock delta; the candidate proves it is
sufficient.

### 2.4 Other inputs enumerated

Runtime reads by the slice, all present at HEAD except where noted: `defs/qa/scopes.toml` (overlay);
`docs/audits/data/{game-qa-matrix,asset-capability-matrix}.csv` and
`docs/audits/data/assets/nhllegacy/xbox360/capability.csv` (the migration sources — **HEAD's**
versions; the live `capability.csv` is modified by another package and was deliberately not
overlaid); `docs/audits/data/schema/**`, `defs/enums/**`, `defs/relationships/**`,
`crates/{tdb-core,tdb-defs,tdb-relations}/src/**`, `crates/studio-domain/{src/view,src/surfaces,src/capabilities}.rs`,
`crates/studio-domain/tests/surface_matrix.rs` (scope globs — present at HEAD);
`defs/appearance/headmesh-matrix.toml` — **absent by design** (untracked; only the ignored case reads
it). `anchors/**` (read by `inventory.rs` cases). Toolchain identity from `rustc -vV` via `build.rs`.

---

## 3. Verification — commands actually executed and their results

All from inside a candidate, `env -u QA_WRITE -u QA_EVIDENCE_ROOT`, `CARGO_TARGET_DIR` external:

| Command | LF candidate | CRLF candidate |
|---|---|---|
| `cargo test -p tdb-qa --locked --offline` | **228 passed · 0 failed · 1 ignored** (72 unit, 32 acceptance, 13 discovery, 7 emit, 9 file-reads, 31 journal, **19 migration + 1 ignored**, 36 publication, 9 inventory) | **228 · 0 · 1**, identical per target |
| `cargo clippy -p tdb-qa --all-targets --locked --offline -- -D warnings` | clean, exit 0 | clean, exit 0 |
| `cargo fmt -p tdb-qa -- --check` | clean, exit 0, empty output | clean, exit 0, empty output |
| `cargo test -p tdb-canonical --locked --offline` (committed consumer of `tdb-qa`'s catalog) | **200 passed · 0 failed** | not repeated |

Logs: `…/n1/log-{lf,crlf}-{test,clippy,fmt}.txt`, `log-lf-canonical.txt`.

**Ignored / unexecuted, reported as gaps, not passes:**
- `real_headmesh_registry_integration_over_the_committed_registry` — **ignored** in both runs with
  its prerequisite named; the registry is absent from both candidates, so it could not have read the
  live one. Not selected with `--ignored`.
- Private-fixture and mounted-game-tree cases live in other crates (`tdb-rx2`, `studio-domain`) and
  were not run; nothing here claims them.
- Unknown conversion identity stays fail-closed: the seven C0-MIG-ID1 cases pass unchanged
  (`unknown-conversion-identity` withheld by name; collisions refuse; absent target fails).

### 3.1 Deterministic generation / merge / check on a disposable non-empty tree

Tree: a copy of a synthetic sealed run + one manual record left by the discovery suite
(`H:\TEMP\tdb-qa-evidence-discovery\r4-normal-18808-17`, 8 files, copied to `…/n1/evtree`). Data
dir: a disposable copy of the candidate's `docs/audits/data` (`…/n1/evdata`). Nothing live touched.

| Step | Result |
|---|---|
| `qa-evidence build --out build1 --root evtree --repo candidate-lf` | exit 0; 10 files; **2016-row** universe (`qa-summary/2`), 1 unresolved-file header only |
| `build --out build2` (same inputs) | exit 0; `diff -r build1 build2` → **byte-identical** |
| `check --root evtree --data evdata` (nothing published yet) | **exit 2 — drift**, as specified |
| `publish --from build1 --data evdata` | exit 0; 10 files, id `844decb520b7ade1`; prior generation retained |
| `check` again | **exit 0** — "ok: 2016 row(s), no validation failure, no drift" |
| `build --out build3` | **byte-identical to build1** |

The merged row for the synthetic key `nhl15-ps3/db-presentation/read` is instructive and correct:
`automated = pass`, `manual = pass`, yet `qualification = withheld`, `reason = stale-evidence`,
`fresh = unknown` — the manual record carries no `[source]` identity, so its digest is `unknown` and
R-FRESH-UNKNOWN withholds. A pass with unrecorded identity does not qualify.

---

## 4. Activation plan

### 4.1 Remaining work by area — blockers vs. proceed-now

| Area | Can proceed now | Genuine blocker |
|---|---|---|
| **Commit the evidence slice** | The boundary is enumerated and verified (§2). A commit package can stage exactly the 35 files + the one lock line on HEAD | Architect authorization of the commit and of its *order* relative to the headmesh members that the live root manifest names (the slice must land **before** or **independently of** them — it does not depend on them) |
| **Migration** (`mig-c0-bulk1`) | Freeze + render a candidate from the committed CSVs; the two unmapped `uniforms` claims stay `_unmapped/` | **D7** (P3/M1 `assets-jerseys` ownership) before install; **U3/D2** for where `legacy/` lives; approval file = architect act |
| **Publication** | `check`/`build`/`publish` are deterministic on isolated trees (§3.1) | **U3/D2**: evidence-tree location and whether layer-A runs are committed; without it there is no `--data` target to publish into |
| **Consumer transition** (`game-qa-matrix.csv`, `matrix_guard`, emitter CSVs → layer C) | Dual-run design exists (R-TRANSITION) | Publication first; `matrix_guard` additionally waits on the headmesh registry (see 4.3) |
| **CI gate** (`qa-evidence check`) | Command is ready; runs offline | Publication first; Windows CI (C0.2 R6) is separate and still blocked on repo-wide fmt/clippy **at HEAD** |
| **U2** (conversion class/recipe vocabulary) | Representation is honest (`unknown`) | **D8** — architect; it is the only thing that can make pair keys semantically final |
| **Ownership** (FR12) | — | **D1** — architect |

### 4.2 Evidence storage and ownership — recommendations vs. decisions

*Technical recommendations (developer):*
1. Commit **layer B (manual) and layer L (migrated claims)** to git under `docs/audits/evidence/`;
   keep **layer A (run directories) out of git** and republish them from CI/local runs. Layer A is
   regenerable and would produce diff churn; B and L are the durable sign-off and history.
2. Publish layer C into `docs/audits/data/` (the contract's location); `check` becomes the gate that
   refuses drift between committed inputs and the published set.
3. Store the `qa-legacy-freeze` under `docs/audits/evidence/legacy/frozen/` and commit it with the
   first install, so the frozen bytes travel with the claims that cite them.

*Decisions that require the architect or user (not made here):* **D2/U3** (the above is a
recommendation, not a decision), **D1** owners, **D7** `uniforms` ownership, **D8** U2, **D11**
whether activation sits inside C0's boundary and in what order. **D3** (outer-package fixture
acquisition) needs the user.

### 4.3 Effect of the deferred headmesh integration on activation

- **Pair keys stay in the universe and stay unqualifiable.** `headmesh_pair_keys` derives keys from
  the registry only; with the registry untracked and U2 open, every pair key carries
  `class=unknown;recipe=unknown` and is withheld by construction. Activation does **not** wait on
  this: those rows publish as `unqualified`/`withheld`, which is the truthful state.
- **The destination-level `heads` claim migrates as a claim** and is cited, never qualifying
  (R-NO-BACKFILL). Nothing about the external work changes that.
- **`matrix_guard` transition is the one consumer that must wait.** Its pair-promotion assertion
  reads `defs/appearance/headmesh-matrix.toml`; until the registry is committed (or the external
  integration lands one), the old assertion keeps running unchanged and the layer-C assertion is
  added beside it (contract §6.5), not instead of it.
- **No qualification rule is weakened.** Deferral changes *when* pair-specific records can exist,
  not what they must contain (R-IDENTITY, R-CONV, R-GAME).

### 4.4 Adapter / profile population — inventory and the next bounded slice

Evidence already in this repository: **27 physical schema snapshots** (`docs/audits/data/schema/`),
**28 audit documents**, the anchors manifest, `defs/enums/`, `defs/relationships/`. Populated:
**2/28** database adapters (`nhl10-xbox360`, `nhl15-xbox360`), **1/14** asset profiles
(`assets-portraits`, 3/28 target sections, 0 `supported-target-constraint`).

**Next bounded slice — `C0-REG1a`: one adapter, `nhllegacy-xbox360`, from its own evidence only.**
Rationale: it has a snapshot (`xbox360-nhllegacy.json`), the densest audit coverage of any single
unit (`roster-capacity`, `anchor-capacity-and-editable-tables`, `ratings-write-codec`,
`schema-drift-matrix`, `surface-capability-audit`, the Xbox 360 asset pipeline audits), and it is
the migration's own attributed target. Rules: import only facts an audit or snapshot **explicitly
attributes to `nhllegacy-xbox360`**; every block linked; `unknown` where the unit's own evidence is
silent; **no fact copied from `nhl15-xbox360` or any neighbour**. Report `registry_population`
before/after. A second slice can then take the PS4 units, whose snapshots and `ps4-nhl-game-db`
audit exist but whose adapters do not.

---

## 5. Following work queue (short; not implemented here)

**C1 — recorded defects, status in the working tree (spot-checked by grep only; none is claimed closed):**

| Defect | Working tree | Verification status |
|---|---|---|
| 1 — dressed/scratched wired to `ZBac.RREB` (player type) | **changed**: `view.rs`/`session.rs` now read lineup status from `eGlu.hSZj` ("Spike S4") | **unverified** — no named automated case located; needs a focused regression against an anchor |
| 3 — goalie-type codes on `hHbv` are a flagged best guess | no audited mapping located by grep | **still present / unverified** |
| 2 — uncalibrated overall outside NHL 14/15 | `AttributesTab.tsx` gates by era; roster-list OVR column not located | **partly changed / unverified** |
| 4 — companion creation on flip, multi-select, clone naming | not checked | open (STATE-OF-THE-REPO §4 still lists them) |
| 5, 6 — contract audit, field-disposition inventory | not checked | open |

First C1 package: verify 1 and 3 with named cases; do not touch 2/4–6 until then.

**C2/C9 — first exact-target case.** Recorded runtime availability now includes a **build
identity**: NHL 19 PS4, **CUSA11126 / APP_VER 01.60 on shadPS4** (N19 records, 2026-09-10/11),
which also settles FR9's discrepancy on the *installed* side (CUSA11126 is what runs). Proposed
first case: `nhl19-ps4/db-operations/scalar-edit` → deploy `nhl_game.db` into that install →
play → save roster → `nhl19-ps4/save-raw-db/reopen` in Studio. Prerequisites before it can be a
layer-B record: FR6 (record runtime identity in the register — the N19 docs give the values),
`studio_commit`/`implementation_digest` captured at deploy time, output hashes recorded. It is a
**manual game-acceptance candidate**, not a promotion.

**C3/C3a — next headmesh-independent slice.** The shared recoverable-write transaction for
**Ignite `*.big` texture replacement (portraits/logos) with append-and-verify**, building on the
untracked `eb_verify_append.rs` / `eb_find.rs` examples in `tdb-archive-big` and the authorized
`Nhl19BigStudio-Tester/` reference. Scope: one archive family, one operation, full recovery; no
mesh payloads.

---

## 6. Recommended activation sequence

1. **Accept C0-MIG-ID1 (+R1)** and this boundary; authorize **C0-COMMIT-EVID1**: stage the 35 files
   + one lock line on HEAD, verify on the exported staged tree exactly as C0-CAN-COMMIT2 did, commit.
2. **Decide D2/U3 and D1** (storage, ownership). Developer recommendation in §4.2.
3. **C0-REG1a** (§4.4) in parallel — it needs no activation decision.
4. **Decide D7, D8**; then **C0-ACT1**: freeze → migrate → approve → install `mig-c0-bulk1`; first
   real `build`/`publish`; `check` gate in CI; dual-run consumers.
5. **Consumer transition** for `game-qa-matrix.csv` and the emitter CSVs; `matrix_guard` waits on
   the registry.
6. Windows CI (C0.2 R6) once repo-wide fmt/clippy pass at HEAD — separate track.

---

## 7. Limits and preservation

No headmesh conversion or donor-fitting work; no other repository accessed; no live migration,
publication, game installation or capability promotion; nothing staged, committed or pushed; no
formatting, cleanup or reset; no live source built, tested or mutated (all builds ran in
`…/scratchpad/n1/candidate-*`); `docs/audits/data/**` unchanged (hashes verified before/after);
retained prior candidates (`…/r1/*`) read only. Live-tree preservation: HEAD, index and the
269-entry status identical before and after; all 35 overlay files and the root `Cargo.toml`/`Cargo.lock`
unchanged (`Cargo.lock` `91b22103…` — already drifted from R1's `7d0c4876…` by other work before
this package began, untouched here). Stopping for Codex review.

---

## 8. Corrections to §4 — recorded by C0-COMMIT-EVID1 (2026-09-13)

§4 stands as written; the following four statements in it were imprecise and are corrected here
rather than rewritten.

1. **§4.3 "Pair keys stay in the universe and stay unqualifiable."** Overstated. `headmesh_pair_keys`
   has **no production caller**, and an absent registry cannot enumerate pair rows at all. What the
   catalog universe publishes today is **family-level** coverage —
   `<target>/assets-heads/<operation>` rows without a conversion component. **Pair-specific** rows
   (`…?from=…;class=unknown;path=…;recipe=unknown`) will exist only once a committed registry is read
   by a wired caller; that they would then publish as `withheld` is the **planned** behaviour the
   merge rules guarantee, not something any publication has demonstrated.
2. **§4.2 recommendation 1 (layer A outside git).** A proposal, not a mechanism. Activation needs,
   in addition: an **immutable retention/retrieval** store for run directories *or* a **reproducible
   recapture** path; a recorded **selected-run identity** per published generation so `check` can
   name what it verified against; and **clean-checkout CI verification** proving the gate holds
   without local state. Committing layers B, L and C alone does **not** make `check` reproducible —
   §3.1's determinism was shown on one disposable tree, not on a clean checkout.
3. **§5 C2/C9 "build identity … on shadPS4".** `CUSA11126 / APP_VER 01.60` identifies the **game
   build**. The shadPS4 **executable version** is not recorded anywhere in the repository; FR6 is
   not settled by the N19 records, only narrowed. The proposed first case still needs the emulator
   identity captured before it can be a layer-B record.
4. **What `819940a` does and does not commit.** It commits the **evidence core** — the `tdb-qa` crate
   and CLI, its tests, `defs/qa/scopes.toml`, one lock line. It does **not** commit the
   `studio-domain` emitter integration (the `observe` calls in the four matrix tests), so
   **observation capture is not wired at HEAD**: nothing at HEAD produces a layer-A run. That
   integration is a separate, later slice with its own boundary.

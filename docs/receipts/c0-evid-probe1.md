# C0-EVID-PROBE1 — isolated evidence-system slice verification

**Package:** C0-EVID-PROBE1 · **Date:** 2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` ·
**HEAD at start and at end:** `ef0fdd74b37a15666d4eba25f66107bdc8948bd8` (unchanged) ·
**Index at start and at end:** empty

> **Scope of writes.** The only file this package writes inside the repository is this receipt.
> Nothing was staged, committed, pushed, stashed, reset, restored or checked out. No live source,
> configuration, `defs/`, `anchors/`, `app/` or `crates/` file was created, edited or deleted. No
> generator ran against the live tree, no `docs/audits/**` file was touched, no live evidence was
> published, no migration was installed, no consumer was retired, no CI was changed, no game data
> was read and no prior evidence was deleted. **`QA_WRITE` was never set.** Every build, test,
> capture, publication and recovery ran in an isolated scratch tree on another volume.
>
> **This record promotes nothing and decides nothing.** It reports what an isolated candidate did
> when executed. Whether that candidate becomes a commit is the architect's call.

**Dependency treatment, as instructed.** C0-SLICE1's S3 attribution is treated as a **starting
point, not proof**. §2.4 records where execution confirmed it and where it did not.

---

## 1. Result in one paragraph

**The evidence system and its four matrix emitters build, lint and run clean at their own boundary
on top of HEAD — and the slice as enumerated is nevertheless *not* self-contained.** A tree built
from `git archive` of `ef0fdd7` plus **33 enumerated files** and **two added `Cargo.lock` lines**
passes `cargo check --workspace --all-targets --locked --offline` (exit 0), contributes **zero**
clippy findings under `-D warnings`, and runs **209 of 215** `tdb-qa` cases green. The six failures
split cleanly: **two are candidate defects** — `evidence_migration` reads two inputs that are *not*
in the slice and *not* at HEAD (the untracked `defs/appearance/headmesh-matrix.toml`, and the
uncommitted `heads` row in `capability.csv`) — and **four are a pre-existing HEAD defect**,
reproduced on a pure-HEAD control tree with no overlay at all. That same pre-existing defect fails
**all four matrix emitters** as well, and its cause was isolated by experiment: the committed
generated CSVs are **CRLF in any Windows checkout** while the generators emit LF, so every
raw-read drift guard fails from a clean checkout. This is the exact defect class C0-CAN-EOL1
repaired inside `tdb-canonical`, and which its §9 item 4 left open for other crates; **eight tests
at HEAD are affected**. With that one pre-existing factor neutralised in a separate labelled scratch
tree, the capture path reproduces C0-S2-INT1-R1's preserved run3 numbers **exactly** — 4 completed
runs, **952 observations**, 189 unmapped, 1452 facets, a deterministic double build, a clean publish
and a `check` reporting `ok: 2175 row(s)`, with **140 `automated-qualified`** and **0
`game-verified`**.

---

## 2. The candidate

### 2.1 Scratch location, validated before creation

| | |
|---|---|
| Scratch root | `H:\c0-evid-probe1\` |
| Pre-existing? | **No** — confirmed absent before creation |
| Prior roots preserved | `c0-baseline`, `c0-bulk1`, `c0-can-commit1`, `c0-can-commit2`, `c0-can-eol1`, `c0-helper-tests`, `c0-integration`, `c0-lock1`, `c0-repair`, `c0-s2-int1`, `c0-slice2` — **all present, none written, moved or deleted** (§6.3) |
| Inside the live tree? | **No.** Different volume from `E:\Repositories\nhl-modding-studio` |

Layout: `candidate\` (the commit-boundary tree), `control\` (pure HEAD, §4.2), `cand-lf\` (the
labelled newline-diagnostic variant, §4.3), `evroot\` + `evroot-crlf\` (evidence roots), `out1\`,
`out2\` (build outputs), `data-scratch\` (published layer C), `logs\`. **`CARGO_TARGET_DIR` was
external on every invocation**, and all logs live outside every candidate tree.

### 2.2 Base identity

| | |
|---|---|
| Commit | `ef0fdd74b37a15666d4eba25f66107bdc8948bd8` |
| Tree | `824d19da7303d25d1a5443b44405fb44c4b65d03` |
| Archive SHA-256 | `e35186907d6e8aae24a83dd870b2a2ea0434236099de507557f3a5c0babf95cd` |
| Files extracted | **1160** |

The dirty working tree was **never copied**. The 33 overlay files were placed individually from an
explicit manifest; no directory was copied wholesale.

### 2.3 Membership — 33 files, enumerated

**`crates/tdb-qa/` — the evidence system (26 new)**

`build.rs`; `src/bin/qa_evidence.rs`; `src/evidence/` ×16 — `csv.rs`, `emit.rs`, `journal.rs`,
`key.rs`, `merge.rs`, `migrate.rs`, `mod.rs`, `model.rs`, `observe.rs`, `paths.rs`, `publish.rs`,
`scopes.rs`, `store.rs`, `testkit.rs`, `validate.rs`, `vocab.rs`; `tests/evidence_*.rs` ×7 —
`acceptance`, `discovery`, `emit`, `file_reads`, `journal`, `migration`, `publication`.
9 218 lines of module source, 4 840 lines of test source, 545 lines of CLI, 36 lines of build script.

**Definitions (1 new):** `defs/qa/scopes.toml` (120 lines).

**Modified, whole-file (6):** `crates/tdb-qa/Cargo.toml` (`toml = "0.8"` plus the `qa-evidence`
`[[bin]]`), `crates/tdb-qa/src/lib.rs` (**one added line**, `pub mod evidence;`),
`crates/studio-domain/Cargo.toml` (the `tdb-qa` **dev**-dependency), and the four emitters
`crates/studio-domain/tests/{mutation,operation,save,surface}_matrix.rs`.

Every one of the three manifest/module edits was read and is single-purpose: no unrelated hunk
rides along.

Live-tree SHA-256 of all 33, and the SHA-256 of each as placed in the candidate, are recorded in
`logs\01-inputs-before.sha256` and `logs\03-candidate-inputs-crlf.sha256` (the pair differ only by
the newline conversion of §2.6, verified content-identical modulo CR for all 33).

### 2.4 What was deliberately excluded — and one correction to C0-SLICE1

| Excluded | Why |
|---|---|
| `crates/tdb-qa/src/{catalog,diff,manifest,matrix,schema}.rs`, `src/bin/qa_inventory.rs`, `tests/inventory.rs` | C0-INT1 formatting only. **Verified, not assumed:** all seven are identical to HEAD once whitespace, rustfmt trailing commas and `else { … }` semicolons are normalised, and the candidate compiles and runs against HEAD's copies |
| `crates/tdb-canonical/**`, `crates/tdb-headmesh/**`, `crates/blender-job/**`, `defs/appearance/*.toml` | not evidence-system inputs |
| `Cargo.lock` feature regions (`blender-job`, `tdb-headmesh`, `zune-core`, `zune-jpeg`, `image`'s zune deps) and the `Cargo.toml` feature member lines | feature-side (§2.5) |
| `crates/studio-api/**` (C0-DT1), `crates/tdb-rx2/src/fixtures.rs` (C0-FX1), all 147 C0-INT1 repairs, `app/**`, `src-tauri/**`, documentation | out of slice |
| `anchors/**`, mounted game trees, private fixture roots, `Nhl19BigStudio-Tester/` | not read. The candidate's `anchors/` is HEAD's committed copy |

**Correction to C0-SLICE1 §3.7.** It states the four emitters need "only five `studio-domain`
symbols". Measured: `surface_matrix` also imports `Capabilities`, `SurfaceCapabilities`,
`SurfaceCapability`, `SurfaceState` and `tdb_relations::RosterModel`, and `operation_matrix` also
imports `RosterModel`, `Severity` and `CAREER_SKATER_SEASON`. **All of them exist at HEAD**, so the
conclusion stands and is now executed rather than argued — but the count was wrong.

### 2.5 The lockfile — two regions, and both are C0

| Region | Added lines | Attribution |
|---|---|---|
| `studio-domain` dependency list gains `"tdb-qa"` | **1** | **C0** — the dev-dependency that lets the four emitters observe |
| `tdb-qa` dependency list gains `"toml 0.8.2"` | **1** | **C0** — the evidence system's TOML reader |

**No package version was added, removed, upgraded or downgraded**; `toml 0.8.2` is already pinned in
HEAD's lockfile. Both edits preserve CRLF. The patched candidate lock differs from the **live**
lockfile by exactly the four feature regions and nothing else — **zero lines present in the
candidate that the live tree lacks** — so the split was verified in both directions, not assumed.

### 2.6 Newline representation, recorded

| Measurement | Result |
|---|---|
| The 33 inputs **on disk** (live working tree) | **33 LF, 0 CRLF** |
| HEAD's tree as `git archive` renders it | **CRLF** (`core.autocrlf=true`; `.gitattributes` marks only `anchors/**` binaries `-text`) |
| The 33 inputs as placed in `candidate\` | **converted to CRLF**, to match what a commit of this slice would check out on Windows |
| Content equivalence after conversion | **33/33 identical modulo CR** |

This choice is deliberate and is what surfaced §4.2's defect. `cand-lf\` (§4.3) differs from
`candidate\` **only** in the newline form of four committed generated CSVs.

### 2.7 Hooks and inputs inspected before execution

| Checked | Finding |
|---|---|
| `.cargo/config.toml` anywhere in the candidate | **none** — no registry override, runner or rustflags |
| `rust-toolchain{,.toml}` | **none** — no pin, no implicit download |
| Build scripts | **two**: `src-tauri/build.rs` (HEAD, Tauri codegen — see §6.1) and the in-slice `crates/tdb-qa/build.rs`, read in full: it runs `rustc -vV`, reads cargo's `TARGET`, and **emits nothing when either is unresolvable** (no clock, no commit, no network) |
| Fixture environment | **no `NHL_STUDIO_FIXTURES` reference and no absolute path** anywhere in the slice |
| Repo-path literals in the slice | `defs/qa/scopes.toml`, the three legacy CSV source paths, `docs/audits/{data,evidence}` as CLI defaults, and **`defs/appearance/headmesh-matrix.toml`** — the last is the §4.1 defect |
| Ambient environment at run time | `QA_WRITE`, `QA_EVIDENCE_ROOT`, `NHL_STUDIO_FIXTURES`, `CARGO_*`, `RUST*` all **unset**; explicitly `unset` again before every invocation |
| Network | `CARGO_NET_OFFLINE=true` plus `--offline` and `--locked` on every invocation |

**Toolchain:** `cargo 1.96.0 (30a34c682 2026-05-25)` · `rustc 1.96.0 (ac68faa20 2026-05-25)` ·
`clippy 0.1.96`. Host `x86_64-pc-windows-msvc`.

---

## 3. Executed checks

All in `H:\c0-evid-probe1\candidate\` unless stated, `--locked --offline`, external
`CARGO_TARGET_DIR`, `QA_WRITE` unset throughout.

| # | Command | Exit | Result |
|---|---|---|---|
| 1 | `cargo test -p tdb-qa --no-fail-fast` | **101** | **209 passed · 6 failed** across 11 targets. The six are §4.1 (×2, candidate) and §4.2 (×4, pre-existing) |
| 2 | `cargo clippy -p tdb-qa --all-targets -- -D warnings` | **101** | **exactly one** diagnostic, in HEAD's `tests/inventory.rs` — **identical on the control tree** (§4.2) |
| 2b | `cargo clippy -p tdb-qa --all-targets` (no deny, for attribution) | **0** | The whole package produces **one** diagnostic, the same pre-existing one. **The evidence system contributes zero** |
| 3 | `cargo test -p studio-domain --test surface_matrix` | 101 | pre-existing drift failure (§4.2) |
| 4 | `cargo test -p studio-domain --test mutation_matrix` | 101 | same |
| 5 | `cargo test -p studio-domain --test operation_matrix` | 101 | same |
| 6 | `cargo test -p studio-domain --test save_matrix` | 101 | same |
| 7 | `cargo check --workspace --all-targets` | **0** | Whole workspace checks. **10 warning lines, 5 distinct diagnostics, all pre-existing HEAD code** (§4.4); **none** from the evidence system or the four emitters |

Per-target detail for check 1: lib 67 · `qa_evidence` bin 0 · `qa_inventory` bin 0 ·
`evidence_acceptance` 32 · `evidence_discovery` 13 · `evidence_emit` 7 · `evidence_file_reads` 9 ·
`evidence_journal` 31 · `evidence_migration` 10 of 12 · `evidence_publication` 36 · `inventory` 4 of 8.
**Nothing was filtered and nothing was ignored** — this slice declares no `#[ignore]`d case, so no
result here is a silent skip.

---

## 4. Diagnostics, separated

### 4.1 Candidate defects — two, both *missing inputs*, neither in the evidence logic

| Case | Needs | Status of that input at HEAD |
|---|---|---|
| `evidence_migration::a11_pair_keys_come_from_the_registry_and_no_destination_claim_is_back_filled` | `defs/appearance/headmesh-matrix.toml` | **untracked, feature-side** (`defs/appearance/` at HEAD holds only `orphan-heads.toml`). The test panics `NotFound` |
| `evidence_migration::both_duplicate_asset_matrices_are_preserved_and_their_differences_published` | the `heads` row in `docs/audits/data/assets/nhllegacy/xbox360/capability.csv` | **uncommitted working-tree hand edit**, dated 2026-08-06 — C0-SLICE1 §2.3 / E4. HEAD's copy has four rows and no `heads`, so the divergence CSV cannot contain `"heads"` and the assertion fails |

Both were **reported, not repaired**: no source was modified to make the probe pass, no input was
substituted, and neither file was acquired into the candidate. The second is doubly out of bounds —
CLAUDE.md puts hand edits to `docs/audits/data/**` under C0.4's ownership, and E4's disposition is
an open architect decision.

**Consequence for the boundary:** as enumerated, the slice **fails its own test suite**. §8 states
the smallest follow-ups.

### 4.2 Pre-existing HEAD defect — CRLF vs LF in committed generated artefacts

Four `inventory` cases and **all four matrix emitters** fail in the candidate. A control tree —
`H:\c0-evid-probe1\control\`, extracted from the **same archive with no overlay of any kind** —
fails **identically**:

| Test | Candidate | Control (pure HEAD) |
|---|---|---|
| `inventory::anchor_manifest_matches_committed` | FAILED | **FAILED** |
| `inventory::schema_snapshots_match_committed` | FAILED | **FAILED** |
| `inventory::schema_diff_matches_committed` | FAILED | **FAILED** |
| `inventory::qa_matrix_matches_committed` | FAILED | **FAILED** |
| `surface_matrix` / `mutation_matrix` / `operation_matrix` / `save_matrix` | FAILED ×4 | **FAILED ×4**, same assertion each |
| `clippy … -D warnings` on `tests/inventory.rs` | error | **error**, same line |

**Cause, isolated by experiment, not inferred.** These guards regenerate an artefact in memory
(LF-joined) and compare it with `fs::read_to_string` of the committed file. `git archive`/checkout
under `core.autocrlf=true` renders those committed CSVs **CRLF**, so equality is impossible. On the
control tree, converting **one** file — `docs/audits/data/surface-capabilities.csv` — to LF and
re-running turned `surface_matrix` from FAILED to **`ok. 1 passed`**; the file was then restored and
re-hashed. Live-tree copies of these CSVs are **LF**, which is why the defect is invisible in
day-to-day work and appears only in a clean checkout.

This is the **same defect class** C0-CAN-EOL1 repaired inside `tdb-canonical`, and it is exactly the
question its §9 item 4 left open for other crates. It affects **eight tests at HEAD**, is
**independent of this slice**, and would be inherited by any commit. It is Windows-specific: a Linux
checkout yields LF, so CI would not currently show it (Windows CI is C0.2 R6, open).

### 4.3 What the newline factor does to the capture path — stated plainly

`ev.finish()` is called **after** the drift assertion in each emitter. On the candidate as it would
be committed, a capture therefore produces a **`failed`** run. Executed and recorded:

`QA_EVIDENCE_ROOT=…\evroot-crlf cargo test -p studio-domain --test surface_matrix` on `candidate\` →
test FAILED, run sealed **`state = "failed"`**, **all 28 observations retained**, no journal left
behind. That is the emitter contract behaving correctly, and under contract §4.3 such a run
qualifies nothing.

To exercise capture and merge on their own merits, §5 therefore runs on **`cand-lf\`** — the
candidate with **only** the four committed emitter CSVs converted to LF, i.e. the pre-existing
§4.2 factor removed and **nothing else changed**. It is labelled as a diagnostic variant, not as the
commit boundary; its byte difference from `candidate\` is 1 244 bytes of CR.

### 4.4 Pre-existing workspace warnings (check 7), for completeness

`crates/tdb-rx2/src/{pack,playermesh}.rs`, `crates/studio-domain/src/discovery.rs`,
`crates/studio-domain/tests/{nhl07_encodings,read_matrix}.rs` — five distinct diagnostics, all HEAD
code, matching the set C0-SLICE2 §11.2 recorded. **`tdb-qa` and the four emitters contribute none.**

### 4.5 A methodological hazard found, and how it was contained

Two candidate trees sharing one `CARGO_TARGET_DIR` produce the **same cargo unit hash**
(`surface_matrix-4939a25925959ca1.exe`), and the fingerprint did not invalidate on switching trees:
a run launched in `candidate\` silently executed the binary built from `cand-lf\` — which embeds its
own `CARGO_MANIFEST_DIR` — and reported a **false pass**. It was caught because the candidate's tree
hashes had not moved, re-run under a dedicated `CARGO_TARGET_DIR`, and the false result discarded
(§4.3 reports the isolated re-run). Every result in §3 and §5 was taken with the tree's own target
directory. **No conclusion in this receipt rests on the contaminated run**, and it is recorded so
the next package does not repeat it.

---

## 5. Capture, merge, publication and recovery

### 5.1 With `QA_EVIDENCE_ROOT` unset — nothing is created

All four emitters run on `cand-lf\`, variable unset, `QA_WRITE` unset:

| Check | Result |
|---|---|
| Test outcome | **4/4 passed** (7.3 s / 131.5 s / 428.6 s / 77.1 s) |
| Evidence artefacts anywhere in the tree | **none** — no `auto/`, no `observations.csv`, no evidence directory |
| Files in the prepared external root | **0** |
| Tree membership before vs after | **identical, delta 0** |

### 5.2 A real capture into a fresh external root

`QA_EVIDENCE_ROOT=H:\c0-evid-probe1\evroot` (new, outside every repository copy):

| Emitter | Runtime | State | Scope · implementation digest | Declared | Executed | Observations |
|---|---|---|---|---|---|---|
| `surface_matrix` | 6.8 s | **completed** | `db-read` v2 · `sha256:9d4eff57eca9…` | 28 | 27 | 28 — 27 `pass`, 1 `skipped-missing-fixture` |
| `mutation_matrix` | 114.3 s | **completed** | `db-edit` v2 · `sha256:96ff35a167b8…` | 28 | 27 | 175 — 147 `pass`, 27 `refused-clean`, 1 skipped |
| `operation_matrix` | 355.3 s | **completed** | `db-edit` v2 · `sha256:96ff35a167b8…` | 140 | 119 | 302 — 271 `pass`, 10 `refused-clean`, 21 skipped |
| `save_matrix` | 81.4 s | **completed** | `save-containers` v2 · `sha256:b4216118f959…` | 243 | 116 | 447 — 190 `pass`, 74 `refused-clean`, 177 `not-attempted`, 6 skipped |

**952 observations · 189 unmapped · 1452 facets · 4 completed runs · 28 files · 705 410 bytes.**
Every figure **matches C0-S2-INT1-R1's preserved run3 exactly.** Toolchain resolved to a real
`rustc 1.96.0 (ac68faa20 2026-05-25)` and every `implementation_digest` is a real `sha256:` — no
`unknown` substitution in any of the three scopes the emitters declare. No journal survived: each
seal removed its own.

**Declared / executed / unavailable, kept separate:** 439 declared key-slots across the four runs,
389 executed, **29 `skipped-missing-fixture`** (the declared-but-absent units — `legacyrecomp-recomp`
in every run, plus the missing save fixtures) and **177 `not-attempted`** (reached and deliberately
not run). None of the three is merged into another anywhere in the output.

### 5.3 Build, publish, check, recover — all in isolated scratch

| Step | Exit | Result |
|---|---|---|
| `check` before publishing | **2** | 10 published files differ from a fresh build (the scratch data dir was empty) |
| `build --out out1` | 0 | 10 files (11 with `build-inputs.json`) |
| `build --out out2` | 0 | — |
| **`diff -r out1 out2`** | 0 | **byte-identical, 11/11 — determinism (A1) on real inputs** |
| `publish --from out1 --data <scratch>` | **0** | published 10, id `921a38896559cb66`, prior generation retained |
| `check` after publishing | **0** | **`ok: 2175 row(s), no validation failure, no drift`** |
| `publish --recover` | **0** | `phase completed — nothing to do` |
| `build --out <evidence root>/auto` | **4** | **refused**: `path-overlap … resolves inside the evidence root` |

**The published matrix — 2175 rows:**

| Axis | Distribution |
|---|---|
| `qualification` | **140 `automated-qualified`** · 36 `not-applicable` · 159 `withheld` · 1840 `unqualified` · **0 `game-verified`** |
| `automated` | 230 `pass` · 32 `mixed` · 121 `declared-not-executed` · 28 `skipped-missing-fixture` · 1764 `not-attempted` |
| `manual` | **2175 `none`** |
| `fresh` | 411 `fresh` · 1764 `unknown` |
| `applicability` | 978 `required` · 1002 `optional` · 159 `unknown` · 36 `not-applicable` |
| `conflict` / `unresolved` | **empty on every row** |

The 140 qualified rows span **27 distinct targets** across `db-presentation` (27) and
`db-operations` (113). The 159 `withheld` rows are the variant cells carrying
`reason = unknown-applicability` — `save-inner-container`'s among them — and were **left withheld**.
**No freshness scope was widened, no claim was promoted, and nothing unknown or stale was
reinterpreted to reach a target count.** Layer B's provenance digest is `sha256:e3b0c442…` — the
empty-tree digest, so `manual/` was never written (R-SEP, V11).

### 5.4 One boundary observation on `defs/qa/scopes.toml`

As committed, the file declares `headmesh-convert` over `crates/tdb-headmesh/src/**` and three
`defs/appearance/*.toml` recipes, **none of which exist at HEAD**. Under R-FRESH-UNKNOWN that
scope's digest is permanently `unknown` until the feature lands. It is **benign here** — no emitter
declares `headmesh-convert` or `asset-codecs`, and the three scopes in use all produce real digests
(§5.2) — but it is a committed forward reference and the architect should know it is there.

---

## 6. Preservation

### 6.1 The candidate trees, measured before and after

| Tree | Files before | Files after | Bytes before | Bytes after |
|---|---|---|---|---|
| `candidate\` | **1186** | **1190** | 756 250 245 | 756 588 555 |
| `cand-lf\` | 1186 | **1186** | 756 249 001 | 756 249 001 |

**All 1186 original candidate files are byte-identical** after every check (`sha256sum -c` → 1186
OK, **0 FAILED**). The four added files are `src-tauri/gen/schemas/{acl-manifests,capabilities,
desktop-schema,windows-schema}.json`, written by HEAD's `src-tauri/build.rs` during the workspace
check (check 7) — **+338 310 bytes**. They are covered by `.gitignore:17` (`src-tauri/gen/`), so
they could never enter a commit; they are reported because generated and gitignored output is
inside the measurement boundary. External `CARGO_TARGET_DIR`s are **not** treated as a write
boundary and are excluded from these figures by construction.

### 6.2 The live repository — untouched

| Check | Before | After |
|---|---|---|
| `git rev-parse HEAD` | `ef0fdd74…` | `ef0fdd74…` — **unchanged** |
| Index | **empty** (0 staged) | **empty** |
| `git status --porcelain=v1 -z -uall` SHA-256 | `800b860af33efd09…` | **`800b860af33efd09…` — byte-identical** (731 entries both times) |
| The 33 live inputs | hashed pre-copy | `sha256sum -c` → **33 OK** |
| `docs/audits/**`, `anchors/**`, `defs/`, `app/`, `crates/`, `Cargo.*` | — | **not written** |

### 6.3 Prior evidence — preserved

`H:\c0-s2-int1\` was hashed in full before the probe and re-verified after: **259 files, 259 OK, 0
FAILED.** `run1` (20 files) and `run3` (28 files) are intact, and every other prior scratch root
listed in §2.1 is present and unmodified. **Nothing was deleted.** This package's own evidence
(`H:\c0-evid-probe1\`, 8.0 GB including build outputs) is retained for review.

---

## 7. Correction to C0-CAN-COMMIT2 (labelled, as instructed)

**C0-CAN-COMMIT2 §5.2 is wrong on two counts, and this receipt corrects it.** That receipt says of
the lockfile remainder: *"Everything still outstanding there is feature-side — `blender-job`,
`tdb-headmesh`, `zune-core`, `zune-jpeg`, plus `studio-domain`'s `tdb-qa` edge and
`tdb-instance-registry`'s `toml` edge."*

| Claim | Correction |
|---|---|
| `studio-domain`'s `tdb-qa` edge is **feature-side** | **It is C0.** It is the dev-dependency that lets the four matrix emitters observe — the evidence system, C0 item 4 (§2.5) |
| "`tdb-instance-registry`'s `toml` edge" | **The package is misidentified.** `tdb-instance-registry`'s dependency list at HEAD is `["rusqlite"]` and is not modified at all. The `+ "toml 0.8.2"` line belongs to **`tdb-qa`** (`Cargo.lock:3933`), and it too is **C0** — the evidence system's TOML reader |

So of the six outstanding lockfile items §5.2 listed, **four are feature-side and two are C0**, and
one of the two was attributed to the wrong crate. C0-SLICE1 §3.2 had these right (its hunks 3+4 and
7); the regression entered at C0-CAN-COMMIT2. **Nothing else in that receipt is affected** — the
canonical commit's own content, tree hash, gates and preservation figures stand unchanged — and this
correction is recorded here rather than by editing the historical record, as the package directs.

---

## 8. Readiness

**Not ready for commit review as enumerated.** The boundary is sound in every respect measured
except one, and that one is decisive:

| Question | Answer |
|---|---|
| Builds from HEAD without the other uncommitted work? | **Yes — executed.** 33 files + 2 lock lines; workspace check exit 0 |
| Contributes any lint or compiler diagnostic? | **No.** Zero clippy findings under `-D warnings`, zero build warnings, in a package whose only diagnostic is pre-existing |
| Adds third-party surface? | **No.** `toml 0.8.2` is already pinned at HEAD |
| Capture, merge, publication and recovery work on real inputs? | **Yes** — 4 completed runs, deterministic double build, clean publish, `check` exit 0, recovery clean, overlap refused |
| Promotes anything? | **No.** 0 `game-verified`; 159 rows stay `withheld`; `manual/` untouched |
| Self-contained test suite? | **No.** Two `evidence_migration` cases require inputs that are neither in the slice nor at HEAD (§4.1) |
| Green from a clean Windows checkout? | **No** — but that failure is **pre-existing at HEAD** and independent of this slice (§4.2) |

### Smallest follow-ups — for the architect to choose, not for this package

1. **The `a11` headmesh input.** Either add `defs/appearance/headmesh-matrix.toml` to the slice —
   which imports a feature-side definition file into a C0 commit and needs explicit authorization —
   or make the case conditional on the registry's presence, which is a production/test change this
   package is not authorized to make. **No third option was found**: the test asserts exact pair and
   destination counts read from that file.
2. **The `heads` divergence row.** The case depends on an uncommitted hand edit to a generated tree
   whose disposition is already an open question (C0-SLICE1 E4, C0.4-owned). It cannot be resolved
   inside a slice commit.
3. **The CRLF drift guards (§4.2).** Independent of this slice and of `tdb-canonical`; a
   C0.2-class baseline defect affecting eight tests at HEAD. Its natural owner is the repair that
   produced C0-CAN-EOL1, extended to `tdb-qa`'s inventory guards and the four emitters. It is
   **not recorded** in `docs/validation/c0-02-baseline-execution.md` today.

The slice was **not broadened** to absorb any of these, and **no production code was repaired**.

---

## 9. Unresolved questions for the architect

1. Which of §8's two options resolves `a11`, and is importing `headmesh-matrix.toml` into a C0
   commit acceptable? Until one is chosen, this slice cannot commit green.
2. Should the CRLF guard defect (§4.2) be added to the C0.2 baseline-defect register, and does it
   block this slice or proceed as its own repair? It fails at HEAD with or without this candidate.
3. `defs/qa/scopes.toml` ships a `headmesh-convert` scope whose declared paths do not exist at HEAD
   (§5.4). Commit as-is, or defer that scope until the feature lands?
4. Is `H:\c0-evid-probe1\` retained as durable evidence, and should this receipt be mirrored into
   `docs/validation/` if a boundary is accepted?

---

## 10. What was **not** verified

- **No full-workspace test run.** Check 7 is `check` only.
- **No workspace-wide `clippy` or `fmt`** — both fail repo-wide at HEAD (C0.2, open) and were
  deliberately not re-measured. Checks 2 and 2b are `-p tdb-qa` and say nothing about other crates.
- **Nothing is game-verified.** No anchor was modified, no game tree or private fixture was read,
  and the merge produced **0 `game-verified`** rows.
- **Linux/CI behaviour is unverified.** Windows only; §4.2's defect is expected to be invisible on
  Linux, which is itself a reason to state it rather than rely on CI.
- **No claim about any other slice.** S1, S4, S5 and S6 remain exactly as C0-SLICE1 left them.
- The §5 results are evidence about the **evidence system**, not about the commit boundary; the
  commit boundary's own emitter behaviour is §4.3.

---

## 11. Files changed

| File | Change |
|---|---|
| `docs/receipts/c0-evid-probe1.md` | This record — **the only file written in the repository** |

**Nothing staged, committed or pushed. No live publication, no migration installation, no consumer
retirement, no CI change, no game-data access, and no prior evidence deleted. Stopping here for
Codex review.**

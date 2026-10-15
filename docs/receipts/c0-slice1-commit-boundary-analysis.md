# C0-SLICE1 — commit-boundary and dependency analysis

**Package:** C0-SLICE1 (queue item Q0 of [C0-CLOSE1](c0-close1-remaining-work.md) §6) ·
**Date:** 2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` ·
**HEAD at start and at end:** `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` (unchanged) ·
**Index at start and at end:** empty

> **Read-only analysis.** Nothing was staged, committed, pushed, stashed, reset, restored, checked
> out, applied, generated, built or tested. No source, configuration, `defs/`, `anchors/`, `app/` or
> `crates/` file was edited. No generator was run and no `docs/audits/data/**` file was touched. No
> game data was searched or read, and the `Nhl19BigStudio-Tester/` implementation was not traversed.
> **The only file this package writes is this receipt.**
>
> This record **promotes nothing** and **decides nothing**. It proposes candidate boundaries for the
> architect to accept, replace or reject. **No commit order here is approved, and no candidate is
> authorized.** No product question is answered.

**C0-CLOSE1-R1 acceptance, recorded here as instructed:** C0-CLOSE1-R1 is **accepted as a
reconciliation** of C0's status — **not as C0 closure**. Its §6 queue and §7 recommendations remain
**proposals**; no commit order and no product decision in that document is approved. This receipt
does not edit that document or any other to propagate the acceptance.

---

## 1. Result in one paragraph

**No clean C0-only slice of the whole exists, but C0 is not monolithic either.** Of C0's five source
strands, **two are separable today** (`tdb-canonical`; the `tdb-qa` evidence system with its four
matrix emitters), **one is separable only in part** (C0-FX1), **one is hard-blocked by feature work**
(the C0-DT1 DTO drift gate, whose `bindings.rs` names **8 export roots that do not exist at HEAD**),
and **one is a genuinely interleaved rewrite** (the C0-INT1 formatting/lint repair, 27 of whose files
carry in-flight feature work in the same regions). C0's **documentation** strand is separable by
paths and has **zero broken links** at its own boundary, but committing it first would put a HEAD
that **documents four pieces of uncommitted machinery** — an accuracy hazard, not a build one.
A further finding beyond C0-INT1 §7's scope: **10 of the 147 files C0-INT1 repaired have changed
since integration**, so C0-INT1 §7.1's "class A builds" probe **no longer describes the current
bytes** and must not be cited as verification of any boundary proposed here.

---

## 2. Checkpoint 1 — inventory

### 2.1 Repository state as found

| | |
|---|---|
| HEAD | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` |
| Branch | `feat/phase5-asset-tooling` |
| **Index** | **empty** — recorded as found, never altered |
| Staged, deleted, renamed, unmerged entries | **0** of each |

Enumeration used `git status --porcelain=v1 -z --untracked-files=all`, split on NUL. `-uall` is
required: the default listing **collapses wholly-untracked directories**, which is why
[C0-CLOSE1 §5](c0-close1-remaining-work.md) reported "242 entries". That figure is a **collapsed**
count and is not wrong at its own definition; the individual-file count is **750**. Both are recorded
so the two numbers are never mistaken for a discrepancy.

| Measure | Count |
|---|---|
| Tracked modifications (` M`) | **171** |
| Untracked individual files (`??`, `-uall`) | **579** |
| **Total records** | **750** |
| Untracked under `Nhl19BigStudio-Tester/` (not traversed) + its `.zip` | **386** |
| **Untracked excluding that reference material** | **193** |

### 2.2 Attribution method

Attribution is by **evidence, not location**: the C0-INT1 patch-set class lists and post-write hash
manifest (`H:\c0-integration\c0-int1-20260907-183530\evidence\`), per-file `git diff` content, the
package receipts, and symbol/`use`-graph reads. Location alone decided nothing — `crates/tdb-rx2/`
holds C0-only, feature-only and triple-mixed files side by side.

### 2.3 Tracked modifications — 171, classified

| Class | Count | Contents |
|---|---|---|
| **C0-only** | **116** | 96 C0-INT1 class-A `.rs`; `crates/tdb-qa/src/lib.rs`; `crates/tdb-rx2/src/{container,bc45}.rs`; `crates/{tdb-qa,studio-domain,studio-api}/Cargo.toml`; 14 C0-03 documentation files |
| **Mixed** | **29** | 27 C0-INT1 class-B `.rs`; `Cargo.toml`; `Cargo.lock` |
| **Pre-existing feature work** | **26** | 22 `app/src/**`; `crates/studio-api/src/dto.rs`; `crates/studio-domain/src/team_tabs.rs`; `docs/audits/README.md`; `docs/audits/data/assets/nhllegacy/xbox360/capability.csv` |
| **Uncertain attribution** | **0** | — |

The 14 C0-03 documents are `CLAUDE.md`, `README.md`, `docs/{00-vision,01-scope-and-compatibility,
03-tdb-format,04-source-database,05-save-data,DESIGN,EDIT-MODEL,ROADMAP,glossary}.md` and
`docs/plans/{full-game-qa-editability,phase-6-asset-tools,phase-6.1-portraits-logos-spikes}.md`.
Every added line sampled in the three `docs/plans/` files is a completion-roadmap cross-reference
dated 2026-09-05 or 2026-09-07 — C0-03, not Phase-5 content.

`docs/audits/data/assets/nhllegacy/xbox360/capability.csv` carries **one added row** (`heads`, dated
**2026-08-06**). It **predates every C0 package** and is therefore classified as pre-existing feature
work, **not** a C0 side effect. It remains a hand-edit to the generated tree that CLAUDE.md's
evidence rules put under C0.4's ownership; it is **reported, not repaired**, and it is not proposed
for any C0 slice.

### 2.4 Untracked files — 579, classified

| Class | Count | Contents |
|---|---|---|
| **C0-only** | **98** | `docs/receipts/**` 35 + 5 `docs/validation/c0-*` + `docs/STATE-OF-THE-REPO.md` + 3 `docs/plans/` (= 44 docs); `crates/tdb-qa/` 25 (`build.rs`, `src/bin/qa_evidence.rs`, `src/evidence/` ×16, `tests/evidence_*.rs` ×7); `crates/tdb-canonical/**` 21; `defs/{canonical,adapters,qa}` 5; `crates/studio-api/src/{bindings.rs,bin/export_bindings.rs}` 2; `crates/tdb-rx2/src/fixtures.rs` 1 |
| **Pre-existing feature work** | **90** | `crates/tdb-headmesh/**` 40 + `crates/blender-job/**` 5 + `defs/appearance/*.toml` 11 (= 56); `app/src/**` non-binding 16; `app/src/bindings/*.ts` 9 (generated); `crates/{studio-domain,tdb-fhm-store}` sources 5; `docs/audits/*.md` 4 |
| **Uncertain attribution** | **5** | `corpus/ea_attribute_maxima.json`, `corpus/ea_formulas.json`, `corpus/ea_formula_state.json`, `corpus/ea_formula_profiles/{Default,Pucks_Picks}.json` |
| **Supplied reference material** | **386** | `Nhl19BigStudio-Tester/**` + `Nhl19BigStudio-Tester.zip` — **not traversed**, counted from the status listing only |

**The 5 `corpus/` files and the 386 tester files are not covered by `.gitignore`.** `.gitignore`
excludes `*.db` (which is why the ~800 MB SQLite corpus does not appear) but nothing excludes these
JSON profiles, the tester tree or its `.zip`. Whether any of them should ever enter git is a
**policy question for the architect and the user** under CLAUDE.md rule 2 — this package neither
decides it nor includes them in any candidate slice.

### 2.5 Generated / input dependencies

| Path | Kind | Owner |
|---|---|---|
| `app/src/bindings/*.ts` — 1 modified, 9 untracked | **generated** by `ts-rs` from `studio-api` DTOs | feature (see §3.4) |
| `crates/tdb-qa/build.rs` | **build script**, untracked. Emits `QA_TARGET_TRIPLE` / `QA_RUSTC_VERSION`; absence yields the literal `unknown` per R-FRESH-UNKNOWN | C0 |
| `defs/qa/scopes.toml` | test/runtime input read at `repo_root/defs/qa/scopes.toml` | C0 |
| `defs/canonical/project-schema.toml`, `defs/adapters/**` (3 files) | test inputs read from repo root by `tdb-canonical` tests | C0 |
| `defs/appearance/*.toml` (11) | inputs for `tdb-headmesh` | feature |
| `Cargo.lock` | resolver output, hand-attributable per hunk (§3.2) | mixed |

**No `include_str!` or `include_bytes!` exists in either C0 crate.** Every definition file is read at
run/test time from the repository root, so no `defs/` file is a *compile-time* input — but a slice
that ships the crate without its `defs/` files would still **fail its own tests**.

---

## 3. Checkpoint 2 — dependencies

### 3.1 `Cargo.toml` workspace membership — separable

The `members` list gains three entries in **two distinct hunks**, six context lines apart:

| Hunk | Adds | Attribution |
|---|---|---|
| `@@ -6,6 +6,7 @@` | `crates/tdb-canonical` | **C0** |
| `@@ -13,6 +14,8 @@` | `crates/tdb-headmesh`, `crates/blender-job` | **feature** |

Six intervening context lines is comfortably above the three git needs, so these are separable
hunks. **Statically plausible; not verified by an actual split** — patch application is outside this
package's permissions.

### 3.2 `Cargo.lock` — separable, and it splits *against* the naive reading

Eight hunks at `-U0`, each attributable:

| Hunk | Content | Attribution |
|---|---|---|
| 1 | new package `blender-job` | feature |
| 2 | `image` gains `zune-core`, `zune-jpeg` | feature (headmesh JPEG feature) |
| 3+4 | **`studio-domain` gains `tdb-qa` and `toml`** | **C0** — the matrix-emitter dev-dependency |
| 5 | new package `tdb-canonical` | **C0** |
| 6 | new package `tdb-headmesh` | feature |
| 7 | **`tdb-qa` gains `toml`** | **C0** — the evidence system |
| 8 | new packages `zune-core`, `zune-jpeg` | feature |

Note hunks 3+4 and 7: **`Cargo.lock` carries C0 changes to two crates that are otherwise feature
territory.** A "feature-first, C0-later" order that treats `Cargo.lock` as wholly feature-owned
would silently drag C0 resolution in, and the reverse order would drag feature resolution in. It is
per-hunk separable, but only if it is deliberately split; it is not a file either side simply owns.

### 3.3 Crate manifests — all three are single-purpose C0 edits

| File | Hunks | Attribution |
|---|---|---|
| `crates/tdb-qa/Cargo.toml` | 2 — `toml = "0.8"`; `[[bin]] qa-evidence` | **C0, whole file** |
| `crates/studio-domain/Cargo.toml` | 1 — `tdb-qa` dev-dependency for the four emitters | **C0, whole file** |
| `crates/studio-api/Cargo.toml` | 1 — `[[bin]] export-bindings` | **C0, whole file** |

The `studio-domain/Cargo.toml` comment cites `docs/plans/qa-evidence-contract.md`, which is
**untracked**. Committing that manifest without the C0 documentation slice leaves a committed
pointer to a file not at HEAD.

### 3.4 The DTO drift gate — **hard-blocked by feature work**

`crates/studio-api/src/bindings.rs` (untracked, C0-DT1) names **70 DTO export roots**. Comparing
`studio-api/src/dto.rs` at HEAD (115 DTOs) against the working tree (124):

| New in the working tree | An export root in `bindings.rs`? |
|---|---|
| `CloneJerseyResultDto`, `CloneKitTexturesSummaryDto`, `JerseyThumbnailDto`, `LegendsComputeSummaryDto`, `LegendsMemberDto`, `LegendsStatusDto`, `LegendsTeamSummaryDto`, `PlayerManifestSummaryDto` | **yes — 8 of them** |
| `ClonedKitFileDto` | no (reached transitively) |

All nine are **feature** DTOs (Legends and jersey cloning). **A C0-DT1-only commit would not
compile**: `bindings.rs` would reference eight types absent from HEAD's `dto.rs`. The gate therefore
drags in `dto.rs` (+146 lines), the `convert`/`commands` code behind those DTOs, and the exact 10
generated `app/src/bindings/` paths CLAUDE.md already flags. **C0-DT1 and the Legends/jersey feature
work are one inseparable unit.** This is the single largest boundary constraint found.

`crates/studio-api/src/lib.rs` is the mixed file at the centre of it, with four regions:

| Region | Content | Attribution |
|---|---|---|
| `@@ -1,6 +1,10 @@` | doc comment naming `bindings` | C0-DT1 |
| `@@ -11,6 +15,7 @@` | `pub mod bindings;` | C0-DT1 |
| `@@ -100,6 +105,14 @@` | +8 Legends/manifest command registrations | feature |
| `@@ -132,83 +145,10 @@` | +4 jersey/roster command registrations **and** −73 lines removing the old `#[cfg(test)] mod bindings` | **both, in one hunk** |

The feature additions and the C0 removal in the last hunk are separated by exactly **three** context
lines (`]`, `};`, `}`) — the minimum git requires. A split is therefore **statically plausible at the
default context width and no more than that**; it is **not verified**, and the margin is one line.

### 3.5 Untracked crates gated by mixed module roots

| Untracked source | Reachable only through | That root's class |
|---|---|---|
| `crates/studio-domain/src/{legends,player_manifest}.rs` | `crates/studio-domain/src/lib.rs` | **mixed (class B)** |
| `crates/tdb-fhm-store/src/legends.rs` | `crates/tdb-fhm-store/src/lib.rs` | **mixed (class B)** |
| `crates/tdb-rx2/src/fixtures.rs` (C0-FX1) | `crates/tdb-rx2/src/lib.rs` | C0-only |
| `crates/tdb-canonical/**`, `crates/tdb-headmesh/**`, `crates/blender-job/**` | `Cargo.toml` `members` | mixed file, **separable hunks** (§3.1) |
| `crates/studio-api/src/{bindings,bin/export_bindings}.rs` | `crates/studio-api/src/lib.rs` | **mixed (class B)** |
| `crates/tdb-qa/src/{evidence/**,bin/qa_evidence.rs}` | `crates/tdb-qa/src/lib.rs` | **C0-only** — one added line, `pub mod evidence;` |

`crates/studio-domain/src/lib.rs` is the worst case found. Its single 110→123-line module-declaration
hunk contains **both** the feature additions `pub mod legends;` / `pub mod player_manifest;` **and**
the C0-INT1 rustfmt **re-sorting of the entire existing module list** (`equipment_*`, `error`,
`head_submesh`, `kit_preview`, `orphan_heads`, `surfaces` all move). The two changes are interleaved
line-by-line with no context run between them. **This region cannot be hunk-separated, and this
receipt promises no split of it.** Separating it would require hand-authoring a patch — outside this
package's permissions and a change of substance, not of boundary.

### 3.6 `tdb-canonical` — the one C0 crate with no feature entanglement

Its only cross-crate coupling is a **dev-dependency** on `tdb-qa` for one assertion:
`asset_tests.rs:576` compares `ASSET_FAMILY` against `tdb_qa::catalog::FAMILIES`. **`FAMILIES` exists
at HEAD** (`catalog.rs:109`), and `tdb-canonical` references **no** `evidence::` item. It therefore
does **not** depend on the C0 evidence system, on `studio-domain`, or on anything feature-side. Its
test inputs are `defs/canonical/project-schema.toml`, `defs/adapters/nhl15-xbox360.toml`,
`defs/adapters/asset-profiles/assets-portraits.toml` and its own 10 bundled fixtures.

### 3.7 The evidence system and the four emitters

`crates/tdb-qa/src/lib.rs` is modified by exactly one line, `pub mod evidence;` — **pure C0**. The
seven other modified `tdb-qa` sources (`catalog`, `diff`, `manifest`, `matrix`, `schema`,
`bin/qa_inventory`, `tests/inventory`) are all C0-INT1 class A and **hash-identical to their
post-integration bytes**: formatting only. `tests/inventory.rs` contains the string `evidence` only
inside a CSV header literal, not as a dependency.

The four emitters — `crates/studio-domain/tests/{mutation,operation,save,surface}_matrix.rs` — import
`tdb_qa::evidence::{CoverageKey, emit::{file_digest, Emitter}, model::Outcome}` and, from
`studio_domain`, only `EditSession`, `FieldValue`, `Platform`, `Project`, `ProjectRole`. Each of
those five names is present in HEAD's `studio-domain/src/lib.rs`. **Whether they still typecheck
against HEAD's signatures is not verified** — no build was run.

### 3.8 C0-INT1 repair — **10 of 147 files have drifted since integration**

`sha256sum -c` of C0-INT1's `live-after.sha256` against the current tree: **137 OK, 10 FAILED.**

| Drifted file | C0-INT1 class | Cause of the drift |
|---|---|---|
| `studio-domain/tests/{mutation,operation,save,surface}_matrix.rs` | A ×4 | **C0-S2-INT1** emitter integration |
| `tdb-rx2/src/{lib,xpr}.rs` | A ×2 | **C0-FX1** explicit fixture tests |
| `tdb-rx2/src/{pack,playermesh,texlib}.rs` | B ×3 | **C0-FX1**, on top of repair + feature |
| `studio-api/src/lib.rs` | B | **C0-DT1** drift gate |

**Every drift is C0 work landing after integration** — no feature work re-entered the repaired set.
Two consequences:

1. All 96 class-A files remain **C0-only** in attribution (six now carry a second C0 package as well
   as the repair).
2. **C0-INT1 §7.1's probe is stale.** Its `cargo check` exit 0 was measured on `git archive HEAD`
   overlaid with the *then-current* 96 class-A files, six of which have since changed. That result
   **must not be cited** as evidence that any boundary in §4 builds.

Two files modified since C0-INT1 fall outside all three classes and are C0 by content:
`crates/tdb-rx2/src/container.rs` (pure C0-FX1) and `crates/tdb-rx2/src/bc45.rs` (pure rustfmt — the
file C0-INT1 §7.2 identified as already `fmt`-clean in the working tree while unclean at HEAD).

### 3.9 Documentation link integrity at the C0 boundary

A scripted link check over the 15 modified documents for Markdown links resolving to **untracked**
targets:

- The 8 C0-03 documents that link outward resolve **only** to `docs/plans/project-completion-roadmap.md`,
  `docs/plans/completion-design-questions.md`, `docs/STATE-OF-THE-REPO.md` and
  `docs/validation/c0-fx1-explicit-fixture-tests.md` — **all four inside the C0 documentation slice.**
  Zero links escape it.
- `docs/audits/README.md` links **only** to the four untracked `docs/audits/*.md` headmesh audits —
  **all four inside the feature slice.** It is cleanly feature-side.

So the documentation strand is link-clean at its own boundary in both directions. What it is *not* is
statement-clean — see §4 S1.

---

## 4. Checkpoint 3 — candidate slices

Each candidate states its exact paths, its dependencies, its unresolved attribution, and its
verification status in the three required grades:

- **statically plausible** — the dependency graph permits it; nothing was executed;
- **previously tested at that exact boundary** — a recorded run exists at *these* bytes;
- **not yet verified** — no run exists, or the recorded run no longer describes these bytes.

**No candidate in this section is claimed to build.** The only boundary probe on record (C0-INT1
§7.1) is stale by §3.8, and the passing check totals in recent receipts were measured on the **full
dirty tree**, which is not a statement about any subset of it.

---

### S1 — C0 documentation records

**Paths (58).** Untracked: `docs/receipts/**` (35), `docs/validation/c0-{03,04a,05,06,fx1}*.md` (5),
`docs/STATE-OF-THE-REPO.md`, `docs/plans/{project-completion-roadmap,completion-design-questions,
qa-evidence-contract}.md`. Modified, whole-file: the 14 C0-03 documents in §2.3.

**Excluded:** `docs/audits/README.md` and `docs/audits/data/**/capability.csv` (feature, §2.3), and
the four untracked `docs/audits/*.md` headmesh audits.

**Dependencies:** none at build time. Link-closed in both directions (§3.9).
**Mixed-file regions:** none — every path is whole-file.
**Unresolved attribution:** none.

**The catch, stated plainly.** Committing S1 first produces a HEAD whose `CLAUDE.md` documents
**four pieces of machinery that are not at HEAD**: the `export-bindings` binary and the whole
"Generated code — do not hand-edit" contract (C0-DT1, blocked by §3.4); the `NHL_STUDIO_FIXTURES`
`tdb-rx2` workflow (C0-FX1, only partly separable); a **20-crate** table naming `tdb-headmesh` and
`blender-job`; and a C0.2 defect note whose repairs are working-tree-only. Every one of those
statements is *true of the working tree* and **false of the resulting HEAD**. Under CLAUDE.md's own
evidence rule — *documentation never promotes support* — this is the exact failure mode to avoid, so
S1 is **not** recommended first despite being the cleanest slice by paths. Making it accurate at its
own boundary would require **editing those documents**, which is a product/scope change this package
is not authorized to make and does not propose.

**Verification required:** relative-link check across the committed set (re-run of §3.9); a read of
`CLAUDE.md`, `README.md` and `DESIGN.md` against the resulting HEAD for statements about
uncommitted machinery. No build, no test.
**Status: statically plausible. Not yet verified.**

---

### S2 — `tdb-canonical` (C0 item 7, the contract half)

**Paths (26).** Untracked: `crates/tdb-canonical/**` (21), `defs/canonical/project-schema.toml`,
`defs/adapters/{nhl10-xbox360,nhl15-xbox360}.toml`,
`defs/adapters/asset-profiles/assets-portraits.toml`.
**Mixed-file regions:** `Cargo.toml` hunk `@@ -6,6 +6,7 @@` **only**; `Cargo.lock` hunk 5
(`[[package]] tdb-canonical`) **only**.

**Dependencies:** `tdb-qa::catalog::FAMILIES`, **present at HEAD** — no evidence-system dependency
(§3.6). Its three `defs/adapters` + `defs/canonical` inputs are in-slice.
**Unresolved attribution:** none.
**Leaves no unresolved build input** and **no misleading drift gate**: it touches neither
`app/src/bindings/` nor `qa-inventory`.

**Verification required:** `cargo check --workspace --all-targets` and `cargo test -p tdb-canonical`
on a probe tree built from `git archive HEAD` + this slice only (the 191 canonical cases C0-AS3-R2
records). Confirm the `Cargo.toml` and `Cargo.lock` hunk splits actually apply. Note `--locked` fails
at HEAD independently (C0-INT1 §7.1) and must be resolved separately.
**Status: statically plausible. Not yet verified** — the 191-case figure is a full-dirty-tree run and
is not evidence at this boundary.

---

### S3 — the evidence system and its four emitters (C0 item 4, definition half)

**Paths (28).** Untracked: `crates/tdb-qa/build.rs`, `crates/tdb-qa/src/bin/qa_evidence.rs`,
`crates/tdb-qa/src/evidence/**` (16), `crates/tdb-qa/tests/evidence_*.rs` (7), `defs/qa/scopes.toml`.
Modified, whole-file: `crates/tdb-qa/src/lib.rs` (one line), `crates/tdb-qa/Cargo.toml`,
`crates/studio-domain/Cargo.toml`, and `crates/studio-domain/tests/{mutation,operation,save,
surface}_matrix.rs`.
**Mixed-file regions:** `Cargo.lock` hunks 3+4 (`studio-domain` += `tdb-qa`, `toml`) and 7
(`tdb-qa` += `toml`) **only**.

**Dependencies:** the four emitters need only five `studio-domain` symbols, all present at HEAD
(§3.7). `studio-domain/src/lib.rs` — the mixed root of §3.5 — is **not** required, because the
emitters are integration tests in `tests/`, not modules of the library.
**Carried side effect, disclosed:** the four emitter files also carry their C0-INT1 rustfmt repair
(§3.8); committing them commits that repair for those four files. It closes no defect on its own.
**Unresolved attribution:** none.
**Documentation coupling:** `studio-domain/Cargo.toml`'s comment cites the untracked
`docs/plans/qa-evidence-contract.md` (§3.3) — either sequence S1 first or accept a dangling pointer.

**Verification required:** `cargo check --workspace --all-targets` and `cargo test -p tdb-qa` plus
the four `studio-domain` matrix tests on a `git archive HEAD` + slice probe; then one
`QA_EVIDENCE_ROOT` capture run into a temporary root to confirm the emitters still journal, and one
run with it **unset** to confirm no file is created. **`QA_WRITE` must stay unset** — it drives
legacy CSV regeneration, which CLAUDE.md forbids as a side effect.
**Status: statically plausible. Not yet verified.**

---

### S4 — C0-FX1 explicit fixture tests · **separable only in part**

**Cleanly separable part (4 paths):** untracked `crates/tdb-rx2/src/fixtures.rs`; modified
`crates/tdb-rx2/src/{lib,container,xpr}.rs` — all three C0-only.

**Inseparable remainder:** `crates/tdb-rx2/src/{pack,playermesh,texlib}.rs` are class B —
**repair + feature + C0-FX1 in the same files** — and three of C0-INT1's five lint allowances live
in class-B files, one of them (`TAG_FX_RENDERABLE`) in `playermesh.rs`.

**Named risk at the partial boundary:** `fixtures.rs` is `#[cfg(test)] pub(crate)`, and its
constants are consumed by all six files. Committing the clean four leaves the constants used only by
three of them; under `-D warnings` an unused `pub(crate)` const in a `cfg(test)` module can trip
`dead_code`. **Unverified — this is exactly the kind of claim that needs a build, and none was run.**
The partial slice also leaves the `tdb-rx2` share of the silent missing-fixture skips **only partly
closed**, contradicting the CLAUDE.md sentence that says C0-FX1 closes it — so **S4-partial should
not be committed alongside S1** unless that sentence is corrected first.

**Verification required:** `cargo test -p tdb-rx2` (default: synthetic only) and
`cargo clippy -p tdb-rx2 --all-targets -- -D warnings` on a probe tree, specifically to settle the
`dead_code` question; plus a `NHL_STUDIO_FIXTURES` run confirming a selected case **fails** on an
absent fixture rather than returning early.
**Status: partial slice statically plausible with a named unverified risk. Full slice not
separable.**

---

### S5 — C0-DT1 DTO drift gate · **not separable**

**Blocked.** `bindings.rs` names 8 export roots absent from HEAD (§3.4). Any commit of C0-DT1 alone
does not compile. Its minimum viable unit is: `crates/studio-api/src/{bindings.rs,
bin/export_bindings.rs}`, `crates/studio-api/Cargo.toml`, three of the four regions of
`crates/studio-api/src/lib.rs`, **plus** `dto.rs`'s 9 feature DTOs, the `convert`/`commands` code
behind them, the 9 untracked `app/src/bindings/*.ts`, and `app/src/bindings/PlayerAttributesDto.ts`
— i.e. **the Legends and jersey-cloning feature**. In practice that is the whole feature slice.

**Misleading-drift-gate hazard:** the gate validates the **working tree** against the current Rust
DTOs, never against HEAD. Landing it while any generated `.ts` is uncommitted produces a HEAD at
which the gate **passes locally and cannot be reproduced from a clean checkout**. The 10 flagged
binding paths must land in the *same* commit as `bindings.rs`, or the gate is decorative.

**Verification required (when the feature lands):** `cargo run -p studio-api --bin export-bindings --
--check` exit 0 from a **clean checkout** of the resulting commit, not from the working tree; plus
the `bindings::tests::live_bindings_match_the_dto_contract` case. **`--write` must not be run** as
part of verification.
**Status: not separable. Not yet verified.**

---

### S6 — C0-INT1 baseline repair · **not separable, and it closes nothing alone**

96 class-A files are C0-only, but a class-A-only commit **does not close B1 or B2** — C0-INT1 §7.2
measured 413 `fmt` diffs across 26 files and 47 clippy errors, **every one in a class-B file**. The
27 class-B files cannot be split: §3.5 shows `studio-domain/src/lib.rs`'s rustfmt re-sort interleaved
with feature module declarations at line granularity, with no context run between them.
**No hunk separation is promised for this slice.** Six of the 96 have drifted since the probe
(§3.8), so even the "it builds" evidence is stale.

**Verification required (only after the feature lands):** re-run `cargo fmt --all` and
`cargo clippy --workspace --all-targets -- -D warnings` on the merged tree; B1/B2/B3 may be claimed
closed at HEAD only from a clean checkout of that commit.
**Status: not separable. Previously probed at a boundary that no longer exists.**

---

### S7 — the pre-existing Phase-5 feature slice (for completeness; **not C0, not proposed here**)

All 26 feature-classified modifications and 90 feature-classified untracked files, plus the class-B
files and the `studio-domain`/`tdb-fhm-store` module roots that gate them, plus `Cargo.toml` hunk 2
and `Cargo.lock` hunks 1/2/6/8, plus — by §3.4 — the whole of C0-DT1. **Acceptance of this work is
not this package's business** and it is listed only because S5, S6 and the remainder of S4 are
downstream of it.

---

## 5. Inseparable dependencies — the short list

| # | Constraint | Force |
|---|---|---|
| **I1** | **C0-DT1 ⊂ feature.** `bindings.rs` names 8 DTOs absent from HEAD | **hard** — will not compile |
| **I2** | **C0-INT1 class B ⊂ feature.** `studio-domain/src/lib.rs` interleaves the rustfmt re-sort with `legends`/`player_manifest` at line granularity | **hard** — no hunk boundary exists |
| **I3** | **B1/B2 closure ⊂ feature.** All 47 clippy errors and the residual `fmt` diffs live in class-B files (C0-INT1 §7.2) | **hard** |
| **I4** | **C0-FX1 in `pack`/`playermesh`/`texlib` ⊂ feature.** Triple-mixed files | **hard** |
| **I5** | `tdb-headmesh` + `blender-job` ⊂ feature `Cargo.toml`/`Cargo.lock` hunks | **hard**, but those hunks are separable from C0's (§3.1, §3.2) |
| **I6** | `crates/{studio-domain,tdb-fhm-store}/src/legends.rs`, `player_manifest.rs` ⊂ their mixed module roots | **hard** |
| **I7** | S3's `studio-domain/Cargo.toml` comment → untracked `qa-evidence-contract.md` | **soft** — sequence S1 first, or accept a dangling pointer |
| **I8** | S1's `CLAUDE.md` asserts C0-DT1, C0-FX1, 20 crates and the C0.2 note as facts about the tree | **soft but material** — see §4 S1 |

---

## 6. Recommended sequence

**A proposal for the architect. Nothing here is approved or authorized, and no step may be executed
without a separate, explicitly authorized package.** Each step's verification is the plan in §4.

| Step | Slice | Why here | Gate before the next step |
|---|---|---|---|
| **1** | **S2 — `tdb-canonical`** | The only C0 strand with **no** feature coupling and no drift-gate or generated-artefact exposure. Smallest coherent unit that is genuinely C0-only | probe `cargo check --workspace` + `cargo test -p tdb-canonical` from a clean checkout |
| **2** | **S3 — evidence system + 4 emitters** | Second-cleanest; needs only HEAD symbols. Delivers C0 item 4's definition half to HEAD | probe build + `cargo test -p tdb-qa` + one `QA_EVIDENCE_ROOT` capture and one unset run, `QA_WRITE` unset throughout |
| **3** | **S7 — the Phase-5 feature slice**, on its own merits and its own review | **I1–I4, I6** all discharge here. Nothing further in C0 can be separated until it lands | the feature's own acceptance; then `export-bindings --check` exit 0 from a clean checkout |
| **4** | **S5 + S4-remainder + S6**, as one mechanical repair-and-gate commit | After step 3 the class-B/C files are tracked and clean, so `fmt`/clippy/C0-FX1/C0-DT1 become a single non-interleaved change — C0-INT1 §7.4's own recommendation, and it still holds | `fmt --check` and clippy `-D warnings` exit 0 **at HEAD** from a clean checkout; only then may B1/B2/B3 be claimed closed, and only then is C0.2 R6 (Windows CI, Q5) actionable |
| **5** | **S1 — C0 documentation** | Deliberately **last**: by this point `CLAUDE.md`'s statements about `export-bindings`, `NHL_STUDIO_FIXTURES`, the 20-crate list and the C0.2 defect note are all **true of HEAD** rather than only of the working tree | link check + a read-through against the resulting HEAD |

**The single decision that unblocks the most:** whether the Phase-5 feature slice may be reviewed and
landed. Steps 1 and 2 can proceed independently of it; steps 3–5 cannot proceed without it.

**If the architect wants C0 records at HEAD sooner**, the smallest coherent alternative is
**S1 committed early with its four inaccurate statements corrected first**. That correction is a
**documentation edit this package is not authorized to make** and does not propose. It would need
explicit authorization naming the exact statements to change, and it would still be a weaker outcome
than step 5, because those statements would then have to be changed back as steps 3 and 4 land.

**If no slice is authorized at all**, the standing consequence is unchanged from C0-CLOSE1 §5: none
of this work is recoverable from HEAD. This receipt makes no claim about backups.

---

## 7. Unresolved decisions

| # | Decision | Who |
|---|---|---|
| **E1** | Whether any candidate slice is authorized, and in what order — §6 is a proposal | **architect** |
| **E2** | Whether the Phase-5 feature slice may be reviewed and landed. It gates I1–I4 and I6, i.e. **all remaining C0 separability** | **architect** |
| **E3** | Whether `corpus/*.json` (5 files) and `Nhl19BigStudio-Tester/**` + `.zip` (386) should be gitignored, committed, or left untracked. None is covered by `.gitignore` today | **architect + user** (CLAUDE.md rule 2) |
| **E4** | The `heads` row hand-added to `docs/audits/data/assets/nhllegacy/xbox360/capability.csv` on 2026-08-06 — keep, revert, or fold into C0.4's merge. **Not touched by this package** | **architect** (C0.4 owns the merge) |
| **E5** | Whether S1 may be committed early with its four statements corrected, per §6's alternative | **architect** |
| **E6** | The `--locked` failure at HEAD (C0-INT1 §7.1) — a `Cargo.lock` question independent of every slice above | **architect** |

---

## 8. Verification still required

**No build, test, generator or drift check was run by this package.** Nothing in §4 is claimed to
compile. Specifically outstanding:

1. A `git archive HEAD` + slice probe build for **S2** and for **S3**, separately.
2. Confirmation that the `Cargo.toml` (§3.1) and `Cargo.lock` (§3.2) hunk splits apply as predicted.
3. The **S4 `dead_code`** question — the one named unverified risk in a slice this receipt calls
   partially separable.
4. Confirmation that the three-context-line split in `studio-api/src/lib.rs` (§3.4) is real, if S5 is
   ever attempted before the feature lands.
5. `fmt`/clippy **at HEAD** after step 4, from a clean checkout — the only thing that can close
   B1/B2/B3.
6. `export-bindings --check` from a **clean checkout** after step 3 — a working-tree pass is not
   evidence.

**C0-INT1 §7.1's `cargo check` exit 0 is stale for six of its 96 files (§3.8) and must not be reused
as verification for anything here.** Passing totals in recent receipts were measured on the **full
dirty tree** and say nothing about any subset of it.

---

## 9. Preservation checks

| | Result |
|---|---|
| HEAD at start / at end | `a2bbdc4f…` / `a2bbdc4f…` — **unchanged** |
| Index at start / at end | **empty** / **empty** — recorded as found, never altered |
| `git status --porcelain=v1 -z -uall` at start vs end | **byte-identical**, 750 records both times (diff empty) |
| Working-tree entries added or removed | **0** |
| Staging, commit, push, stash, reset, restore, checkout, branch change, worktree, patch application | **none** |
| Source, configuration, `defs/`, `anchors/`, `app/`, `crates/`, `Cargo.*` | **not touched** |
| Generators, `qa-inventory`, `docs/audits/data/**`, `export-bindings`, builds, tests | **not run** |
| `QA_WRITE`, `QA_EVIDENCE_ROOT` | **never set** |
| Game data, mounted installs, `Nhl19BigStudio-Tester/` implementation | **not searched, not read, not traversed** |
| Files written | **1** — this receipt |
| Commits / pushes | **none** |

External read-only inputs: `H:\c0-integration\c0-int1-20260907-183530\evidence\` (C0-INT1 class
lists and `live-after.sha256`), opened read-only; nothing there was modified.

---

## 10. Files changed

| File | Change |
|---|---|
| `docs/receipts/c0-slice1-commit-boundary-analysis.md` | This record — the only file written |

No other document was edited, including to propagate C0-CLOSE1-R1's acceptance (recorded in the
header above, per the package instruction).

**Nothing staged, committed or pushed. Commit and push remain unauthorized. Stopping for Codex
review.**

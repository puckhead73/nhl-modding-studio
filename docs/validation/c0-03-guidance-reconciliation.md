# C0.3 — Guidance reconciliation

> **Package:** C0.3 (roadmap [C0](../plans/project-completion-roadmap.md) item 3).
> **Inputs:** the accepted [C0.1 baseline inventory](c0-01-baseline-inventory.md) section 7
> (conflicts D1–D12), the [C0.2 baseline execution](c0-02-baseline-execution.md) findings and
> limitations, the [completion roadmap](../plans/project-completion-roadmap.md) and the
> [design-question brief](../plans/completion-design-questions.md).
> **Type:** documentation only. No source was changed, no generated evidence refreshed, no QA
> generator run, no capability CSV touched, no fixture accessed, no game file written, and **no
> support status promoted**.
> **Date:** 2026-09-07. **Author role:** developer. Reviewer: Codex (architect).
> **Revision R1, 2026-09-07 (during C0.4a):** two claims in the first draft overstated their
> evidence and were corrected in the documents and here. See section 10.

---

## 1. Authorized commit performed by this package

Only the three accepted C0.2 files were committed, exactly as instructed.

| Item | Value |
|---|---|
| Commit | `2a70b4d0ea8c3a213678f5f76f685da940821434` |
| Subject | `docs(validation): record isolated Windows baseline` |
| Files | `docs/validation/c0-02-baseline-execution.md`, `c0-02-snapshot.ps1`, `c0-02-snapshot.tests.ps1` — three additions, 1,484 insertions, nothing else |
| Prior HEAD | `16825ca64cc70d510855eecfacc52a2b6b17c23f` |
| Index before staging | Verified **empty** (`git diff --cached --name-only` returned nothing). No unrelated staged change existed, so nothing had to be preserved and no commit was blocked. |
| Staged diff inspected | `A` on all three paths only; `git diff --cached --check` exit 0, no whitespace errors |
| Message delivery | `git commit -F <file>` — a message file, avoiding the shell-quoting damage that required an amend in C0.2 |
| Push | **Not performed. Not authorized.** |

**The C0.3 documentation changes below are uncommitted and unstaged.**

---

## 2. Files changed

| File | Change |
|---|---|
| [CLAUDE.md](../../CLAUDE.md) | Rewritten as a concise entry point (see section 4) |
| [README.md](../../README.md) | Status, coverage table, capability list and documentation index |
| [docs/DESIGN.md](../DESIGN.md) | Scope, sequencing pointer, registry/metadata ranges, crate-set reality note, risk wording |
| [docs/01-scope-and-compatibility.md](../01-scope-and-compatibility.md) | Platforms, target counts, evidence-level matrix legend and notes |
| [docs/ROADMAP.md](../ROADMAP.md) | Historical banner extended (counts, sequencing, Legends). **Phase record itself untouched.** |
| [docs/STATE-OF-THE-REPO.md](../STATE-OF-THE-REPO.md) | Titles line; section 6 drift list converted to a disposition table |
| [docs/plans/phase-6-asset-tools.md](../plans/phase-6-asset-tools.md) | Sequencing pointer to milestones C3/C3a/C4/C5/C7; 6.x numbering marked historical |
| [docs/00-vision.md](../00-vision.md), [03-tdb-format.md](../03-tdb-format.md), [04-source-database.md](../04-source-database.md), [05-save-data.md](../05-save-data.md), [glossary.md](../glossary.md), [EDIT-MODEL.md](../EDIT-MODEL.md) | Single-line scope-range corrections (`NHL 10–21` → the actual catalog) and two obsolete phase pointers |
| **This file** | New record |

`docs/plans/full-game-qa-editability.md` was **not** edited: it already carries the completion-roadmap
banner and the correct 17-entry scope line.

---

## 3. C0.1 conflicts — disposition

| # | Conflict | Resolution in this package |
|---|---|---|
| D1 | CLAUDE.md "not been scaffolded yet" | **Fixed.** CLAUDE.md now states the app is implemented (~82k Rust / 24k TS / 115 commands) and points at STATE-OF-THE-REPO.md. |
| D2 | README "pre-release scaffold … no application code committed" | **Fixed.** Replaced with an in-development status that also states no target is game-verified. |
| D3 | Title scope "NHL 10 → NHL 21" | **Fixed** in CLAUDE.md, README.md, DESIGN.md and the six topic docs: 17 catalog entries, NHL 07–21 + Legacy + Recompiled, 27 shipping targets, NHL 07 PS3 research-only. ROADMAP.md keeps its wording as history under a banner that says so. |
| D4 | "Xbox (360/One family)" | **Fixed.** README now reads Xbox 360 and states explicitly that there is no Xbox One target; CLAUDE.md, DESIGN.md and the scope doc repeat the exclusion. |
| D5 | ROADMAP phases stale | **Already banner-flagged; banner extended** with the target counts, an instruction to sequence from C0–C10, and the Legends note. The phase text is preserved verbatim. |
| D6 | Compatibility matrix = inherited reference-app confidence | **Fixed without promoting anything** — see section 5. |
| D7 | Legends shipped with no ROADMAP slot | **Addressed** in the ROADMAP banner and the STATE-OF-THE-REPO disposition table, both pointing at the completion roadmap's Legends workstream. The historical phase list is not retrofitted. |
| D8 | Arena assets in QA scope vs. the stretch decision | Already resolved by the working-tree edit to the QA plan. **Untouched.** |
| D9 | Generated ledger vs. hand-maintained asset CSVs | **Left open — owned by C0.4.** CLAUDE.md now forbids running QA generators or hand-editing capability CSVs as a side effect of other work, and records that the merge design is C0.4's. |
| D10 | `qa-ledger.md` "no numbers are hand-maintained" | **Left open — C0.4.** Not edited here; correcting it belongs with the merge design that gives manual sign-off a home. |
| D11 | Windows required (Q05) vs. Linux-only CI | **Recorded, not implemented.** CLAUDE.md states Windows is the required initial platform; CI implementation is outside this package's limits and remains C0.2 R6. |
| D12 | Clean-room rule vs. the authorized tester directory | **Fixed.** CLAUDE.md hard rule 1 now carries the recorded exception for `Nhl19BigStudio-Tester/` (C3a), stated as specific to that material and relaxing nothing else. |

---

## 4. CLAUDE.md as an entry point

The rewrite keeps it short and pointer-shaped rather than duplicating the roadmap. It now carries:

- **Current architecture paths** — `crates/` (20 crates, named), `app/`, `anchors/`, `defs/`,
  `docs/`, `corpus/`, and an explicit correction that `src-tauri/` is a thin shell, **not** the Rust
  core (the old text located the core there and the frontend at `src/`).
- **Authoritative references** — the completion roadmap as the execution sequence, the question
  brief, DESIGN, EDIT-MODEL, the QA plan, the scope doc, STATE-OF-THE-REPO, `audits/`,
  `validation/`, and ROADMAP.md explicitly labelled historical.
- **Accepted scope**, stated once and briefly, with a pointer for the rest.
- **Hard rules** 1–6, with the human-readable-surfaces rule promoted from Conventions to a numbered
  rule and the tester exception recorded under rule 1.
- **Evidence rules** — documentation never promotes support; the five evidence levels; the explicit
  statement that no cell is game-verified; that `cargo test` totals are not target qualification;
  staleness on parser/schema/codec/validation change; and the prohibition on running QA generators
  or editing capability CSVs incidentally.
- **Generated DTO warning** — `app/src/bindings/*.ts` are generated by `ts-rs` via the
  `export_typescript` test, must never be hand-edited, and have **no drift check** today. **(R1: the
  wording of what C0.2 §4.7 proved was corrected — see section 9.1.)**
- **Architect/developer receipt workflow** — Codex owns architecture, scope and acceptance; work
  arrives as numbered packages with limits; each ends in a short receipt and stops for review; and
  **Codex explicitly authorizes each commit and each push**, with the staging/inspection/message-file
  procedure spelled out.
- **Known open baseline defects** (C0.2 B1–B7) with a warning not to fix them opportunistically.

---

## 5. Compatibility language vs. evidence (D6)

The matrix previously used a confidence legend whose 🟡 was described as the reference app's proven
Legacy/14/15 lineage. It now states an **evidence level** per cell:

| Symbol | Meaning |
|---|---|
| ✅ | game-verified — **held by no cell** |
| 🟡 | implemented + automated-tested against that target's anchor, **for the main-database path only** (R1) |
| 🔬 | research-only (anchor exists, not a shipping target) |
| ⬜ | implemented, no automated coverage yet |
| ⛔ | blocked, blocker recorded |
| ❓ | fixture or runtime missing |
| — | not a catalog combination |

Two cells changed value, and neither is a promotion:

- **NHL 07 PS3: `—` → 🔬.** The old cell claimed the combination does not exist. It does: `anchors/ps3/nhl07`
  is one of the 27 anchors. It is a research copy, not a shipping target, so it is now marked as such.
- **Legacy Recompiled: 🟡 → ⬜.** There is **no independent Recompiled anchor** (the 27 anchors are 10
  PS3 + 10 Xbox 360 + 7 PS4), so no automated case asserts it as its own target. Under the new legend
  🟡 would have overstated it. A note records that sharing Legacy's schema does not inherit a pass.

Deliberate exclusions:

- The **C0.2 Rust totals (667 passed / 0 failed / 7 ignored)** are **not** used anywhere as target
  qualification. The matrix text says so explicitly. C0.2 §4.4 is the reason: 15 cases reported `ok`
  while returning early on an absent fixture, so a green suite is not per-target evidence.
- The **QA ledger's 80 `edit-roundtrip-pass` rows** were **not** mapped onto matrix cells. They are
  feature-family rows, not per-target sign-off, and mapping them is evidence-merge work owned by C0.4.
- No `capability.csv` and no generated ledger artifact was read for a status or written.

---

## 6. Checks run

| Check | Result |
|---|---|
| Index empty before staging | Confirmed |
| Staged diff inspected (`--name-status`, `--stat`) | 3 additions only, as authorized |
| `git diff --cached --check` | Exit 0, clean |
| Relative-link validation across the 12 changed documents | **All resolve.** Every relative Markdown link target was checked against the filesystem; zero broken. |
| Anchor counts asserted in prose | Verified on disk: `anchors/ps3` 10, `anchors/xbox360` 10, `anchors/ps4` 7 = 27, including `ps3/nhl07`, with no Recompiled anchor |
| Shipping-target count | Cross-checked against the completion roadmap's runtime table (10 + 9 + 7 + 1 = 27) and against the matrix's non-`—`, non-🔬 cells = 27 |
| Crate list and layout | Verified against `crates/` (20 directories) and `src-tauri/src/` (`lib.rs`, `main.rs`) |
| Toolchain claims | Verified: `rust-version = "1.96"` in `Cargo.toml`; Node 20 and the four CI steps in `.github/workflows/ci.yml` |
| Residual stale-claim sweep | `scaffold`, `src-tauri` as core, `Xbox One`, `NHL 10–21` swept across `CLAUDE.md`, `README.md`, `docs/*.md` — remaining hits are the intentional historical quotations listed below |

No build, test, formatter, linter, DTO generator or QA generator was run. This package changed only
Markdown.

---

## 7. Intentionally retained historical text

These remaining occurrences are correct as history and are labelled as such at their location:

- **`docs/ROADMAP.md`** keeps its entire NHL 10–21 phase record, Phase 0's scaffold description and
  Phases 9/10 as written. The banner states the scope wording is stale and that sequencing comes
  from C0–C10. Retained because the original phase record is required to be preserved.
- **`docs/DESIGN.md`** and **`docs/plans/phase-6-asset-tools.md`** still refer to Phase 6.x sub-phase
  numbers. The phase-6 plan's new banner maps that work onto milestones C3/C3a/C4/C5/C7 and marks the
  numbering historical.
- **`docs/EDIT-MODEL.md`** retains its NHLViewNG-parity framing; only the scope-range sentence and one
  obsolete phase pointer changed. The binding contract itself is untouched.
- **`docs/rebuild-design.md`**, **`nhlviewng-functional-report.md`**,
  **`studio-vs-nhlviewng-comparison.md`** and **`docs/audits/**`** are prior-app reference material
  and recorded evidence. Not edited, by rule.
- **`docs/validation/c0-01…`** and **`c0-02…`** record the state at their own dates and are not
  retro-edited; this document supersedes their open items where it says so.

---

## 8. Still open

| Item | Owner | Note |
|---|---|---|
| D9/D10 — generated vs. manual QA evidence merge; the `qa-ledger.md` "nothing hand-maintained" line | **C0.4** | Untouched here. Manual sign-off still has nowhere to live in the generated ledger. |
| Mapping ledger rows / asset `capability.csv` evidence onto per-target cells | **C0.4** | Deliberately not attempted; would require reading and reconciling generated evidence. |
| B1 — `cargo fmt --all -- --check` fails (1,231 hunks, 122 files, 19 crates) | open | Recorded in CLAUDE.md; repair is C0.2 R1, not this package. |
| B2 — `cargo clippy … -D warnings` fails (174 diagnostics, 158 spans) | open | C0.2 R2. |
| B3 — vacuous assertion at `crates/studio-domain/tests/read_matrix.rs:305` | open | C0.2 R3. |
| B4 — 15 cases report `ok` on absent fixtures (texlib descriptor/resize, XPR2 geometry, template head substitution unproven) | open | C0.2 R4; also the reason totals are not qualification here. |
| B5 — hardcoded absolute fixture paths, one into `E:\Repositories\nhl-database-studio` (clean-room) | open | C0.2 R4. |
| B7 — tests leave temp artefacts (`H:\TEMP\…`) | open | — |
| D11 — Windows required (Q05) vs. Linux-only CI | open | C0.2 R6; CI implementation is outside this package's limits. |
| DTO drift gate | open | C0.2 R5. CLAUDE.md warns that the bindings are generated and unguarded. |

**No accepted document was found to genuinely contradict another about a feature.** Every conflict
addressed here was a stale statement superseded by a recorded decision, not a live disagreement.

---

## 9. R1 corrections (2026-09-07, raised during C0.4a review)

Two statements in the C0.3 first draft claimed more than the evidence supports. Both are corrected in
the documents themselves; this section records what changed and why.

### 9.1 The DTO binding claim (CLAUDE.md)

**Was:** "the committed bindings match only because they were regenerated (C0.2 §4.7 found all 124
byte-identical)."

**Wrong because:** C0.2 §4.7 compared the bindings **inside the C0.2 working-tree snapshot** before
and after `export_typescript` ran. That snapshot deliberately includes modified and untracked files,
so the 124 files it compared are the **working-tree** bindings, not the ones committed at HEAD. The
check therefore says nothing about whether HEAD's bindings match HEAD's Rust DTOs. At the time of
this correction, 10 paths under `app/src/bindings/` are modified or untracked relative to HEAD.

**Now:** CLAUDE.md states that nothing enforces binding/DTO agreement, describes exactly what
§4.7 established and about which tree, and records the 10 divergent paths. The `--check` gate
remains open work (C0.2 R5).

### 9.2 The automated-coverage claim (`01-scope-and-compatibility.md`)

**Was:** "Automated coverage means the anchor matrices (`mutation_matrix`, `operation_matrix`,
`save_matrix`, `surface_matrix`) executed for that target."

**Wrong because:** it reads as though all four matrices cover every 🟡 target, and it lets a
main-database result be mistaken for container coverage. Neither holds:

- `save_matrix` emits **no rows for PS3 NHL 07** (verified in `docs/audits/data/save-matrix.csv`);
  the other matrices cover 27 units.
- **No matrix covers Legacy Recompiled at all** — it has no anchor. (This is why C0.3 moved that cell
  to ⬜; the reasoning was right but the surrounding sentence contradicted it.)
- Main-DB coverage and save-container coverage are different operation classes. Outer-console
  packaging — **required under Q04** — is unproven everywhere, and Xbox 360 STFS is recorded blocked.

**Now:** the matrix legend says 🟡 covers the **main-database path only**; a per-matrix table states
each matrix's operation class and unit coverage, including the PS3 NHL 07 and Recompiled gaps; and
explicit notes say main-DB coverage implies neither container nor packaging nor asset coverage. The
research-only and Recompiled distinctions are preserved unchanged.

Neither correction promoted or demoted a support status. Both narrowed a claim.

---

## 10. What was not done

No source file, test, CI workflow, dependency, generated binding, capability CSV, QA-ledger artifact,
audit record or game file was changed or read for mutation. No QA generator was run. No fixture was
accessed. No support status was promoted, and no evidence was merged. Nothing in this package is
staged or committed, and nothing was pushed.

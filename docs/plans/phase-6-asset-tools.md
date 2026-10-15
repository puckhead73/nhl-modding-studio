# Phase 6 — Production asset tools (implementation plan)

> **Status:** 📋 scoped 2026-07-26. Sequenced by dependency; each sub-phase is gated by *verified*
> round-trip per (title, platform) — **proven-first, never big-bang**. Follows Phase 5 (Roster
> Builder + manual construction). Architecture: [../DESIGN.md](../DESIGN.md); original roadmap slot:
> [../ROADMAP.md](../ROADMAP.md) Phase 6.
>
> **Sequencing update (2026-09-07):** current execution order for this work lives in the
> [project completion roadmap](project-completion-roadmap.md) — milestones C3/C3a (shared asset
> writes and Ignite archive-backed texture editing), C4 (2D workflows), C5 (mesh, Appearance Studio,
> Hockey Shop, headmesh) and C7 (audio). The sub-phase content below remains the detailed plan; its
> 6.x numbering is historical. Nothing here is game-verified.

## Cross-cutting principles (apply to every sub-phase)

1. **Format facts verified, not guessed** (CLAUDE.md rule 3). Every unproven (title, platform) asset
   layout gets a discovery spike landing in `docs/audits/` with provenance + confidence before pipeline
   code depends on it. Mark unverified specifics.
2. **Proven-first gating** (rule 4). An asset (title, platform) flips to supported only on verified
   round-trip + in-game check — same discipline as the DB matrix. Track status alongside
   [01-scope-and-compatibility.md](../01-scope-and-compatibility.md).
3. **Human-readable surfaces** (rule). Every user-facing label resolves to real names (team, player,
   slot, portrait subject); raw ids/slugs only behind a details affordance.
4. **Clean-room** (rule 1). Prior-app conversion knowledge (esp. the NHL24→Legacy headmesh process) is
   consulted **only** when the user explicitly opens that door per-instance; until then build from
   format facts placed in-repo.
5. **Codec reuse.** `tdb-rx2` already carries the X360 portrait/texture chain (BIGF→refpack→XPR2/DDS,
   GNF for PS4). Extend it; do not fork per feature.

---

## Phase 6.1 — Portraits & Logos *(first — 2D, no Blender dependency)*

The highest-value / lowest-risk workstream and the only one with no mesh dependency, so it ships first.

### 6.1 spikes

Detailed briefs: [phase-6.1-portraits-logos-spikes.md](phase-6.1-portraits-logos-spikes.md). Note the
read/decode/export side is **already built** (`assets.rs` + `tdb-rx2`); the spikes target the
encode/replace direction and the matcher, not decoding.

- **P-S1 — Import/replace round-trip per (title, platform).** Prove encode is the exact inverse of the
  existing decode (X360 re-tile→XPR2→RefPack→BIGF; PS4 flat-PNG; PS4 BC7 GNF), byte-identical on a
  vanilla raster then in-game load on a modified one. Output: per-target support rows + fallthrough
  (import-disabled where unproven). *Gates the converter + `portrait_import`/`logo_import`.*
- **P-S2 — Portrait source index + match review.** Reuse the implemented
  `tdb-core::dev_key` universal asset slug (`lastname_firstname_dd_mm_yyyy`) and existing EA/FHM
  bridge. Build the searchable source index and ambiguity/manual-resolution policy; do not reopen
  slug normalization. *Gates `portrait_search` + the matcher UX.*

### 6.1 backend
- **Portrait/Logo asset DB** (SQLite sidecar): index imported source assets with parsed slug fields.
- **Portrait matcher/review:** exact universal-slug matches first, then ambiguity/unmatched review and
  manual browse, resolved to real subject names. It calls the existing bridge-key implementation.
- **On-import converter:** source (DDS/PNG/…) → the **selected game's format** (P-S1), including
  mip/atlas/pool-vs-bespoke placement. Round-trip proven per target before enabling.
- **Logo icon generator:** derive each game's required icon/variant set from a source logo, honouring
  per-title iconography (sizes, atlas layout, colour-key/alpha rules).

### 6.1 service + frontend
- 6-point command registration (DTO → convert → registry → `#[tauri::command]` → `command_handler!` →
  `export_typescript`). Commands: `portrait_search(query)`, `portrait_import(...)`, `logo_import(...)`,
  `logo_generate_icons(...)`.
- Portraits surface: match list (high→low confidence), "review all slug matches", manual browse, format
  preview pre/post-convert, replace. Logos: import + generated-icon preview per target.

*Gate:* import a portrait + a logo into ≥1 proven (title, platform), format-convert, save, verify
in-game; matcher ranks a known subject first; no unverified target format enabled.

---

## Phase 6.2 — Mesh + Blender pipeline foundation *(shared prerequisite for 6.3–6.5)*

Built once; consumed by appearance CAP meshes (6.3), hockey shop (6.4), and headmesh porting (6.5).

### 6.2 spikes
- **M-S1 — Headless-Blender bundling + invocation.** Decide bundle layout (size, licensing, platform),
  the headless invocation contract (script + args + stdio/JSON marshalling), and failure/timeout
  handling. Output: a `blender-job` harness design.
- **M-S2 — rx2 mesh IO round-trip.** Confirm `tdb-rx2` (or a new `tdb-mesh`) reads/writes the mesh
  containers we need lossless-round-trip on, on the proven titles first.

### 6.2 deliverables
- A **job harness**: invoke bundled Blender headless, marshal geometry/UV/skin in and results out,
  capture logs, verify output, surface progress as a Studio job. Deterministic + testable (golden
  round-trip on a fixture mesh).
- Mesh IO round-trip in the engine crate set.

*Gate:* a fixture mesh round-trips through the harness byte-stable; a UV-projection sample job returns
verifiable geometry; harness errors surface as Studio job failures, never silent.

---

## Phase 6.3 — Player Appearance

- **Enum editing:** hair / facial-hair / eye-color / skin-tone. Vocab from RTTI where available, but the
  **on-disk offset is verified vs anchor per (title, platform)** — RTTI gives labels, not the codec
  (see the `enum-rtti-not-field-codec` audit; watch 0- vs 1-based and the wrong-field trap).
- **Premodeled CAP mesh library:** curated meshes selectable + imported via the 6.2 harness. In-Studio
  geometry **editor deferred** — import/selection now, editing later.
- Head/hair 3D preview reusing the 6.2 pipeline.

*Gate:* edit each appearance enum on a proven title, save, verify in-game; import a CAP mesh onto a
player and confirm it renders correctly in-game.

---

## Phase 6.4 — Hockey Shop / Equipment

- Per-slot scan (sticks / skates / gloves / goalie equipment) → 3D preview → store → clone → install.
- **UV conversion via the 6.2 Blender projection pipeline** — the reason 6.2 precedes this.
- Codec reuse for equipment containers (rx2/rpsgl per DESIGN inventory).

### 6.4 spike
- **E-S1 — UV projection recipe per slot/target.** Establish the projection/atlas mapping that yields a
  correct in-game result per equipment slot and target game. Output: audited per-slot recipes.

*Gate:* clone + install one item per slot family on a proven title, save, verify correct UV/appearance
in-game.

---

## Phase 6.5 — Headmesh porting *(moved in-scope 2026-07-26)*

Sequence the proven **NHL24→Legacy** conversion first, then generalize toward **any-to-any**; each
(source, target) pair gated independently. Uses the 6.2 harness for skin transfer / re-projection.

> **Clean-room note.** The NHL24→Legacy conversion knowledge is referenced only when the user
> **explicitly opens that door** for that instance (CLAUDE.md rule 1). Until then this is built from
> format facts placed in-repo (and the user has committed to pointing at the source when we reach it).

### 6.5 spikes / stages
- **H-S1 — NHL24→Legacy conversion recipe.** Capture the source→target head-mesh transform (rig/skin,
  UV, scale/quantisation — note the multi-skin denormal-copy + re-quantisation tolerances already
  audited in `nhl09-multiskin-denormal-copies`). Output: an audited, tested one-pair conversion.
- **6.5a** — ship NHL24→Legacy end-to-end (import → convert → install → in-game verify).
- **6.5b** — generalize the recipe into a source/target matrix, adding pairs one verified pair at a
  time. Never assume an untested pair matches a neighbour.

*Gate (per pair):* convert a real head mesh, install, save, verify in-game on the target title.

---

## Phase 6.6 — Audio *(preview + replace parity)*

Matches the reference-app surface — **no in-Studio waveform authoring** in the RC.

- Slot list + preview + **replace/export** for menu / in-game / **PBP** / **PA**.
- Bundled codecs: EALayer3 / Speex (+ `sx.exe`, `vgmstream`) per the DESIGN inventory.

### 6.6 spike
- **A-S1 — Audio bank layout + codec per (title, platform).** Confirm slot/bank structure and the
  encode path for replace on the proven titles first. Output: audited bank map + a decode↔encode
  round-trip proof.

*Gate:* preview + replace one asset in each of menu / in-game / PBP / PA on a proven title, save,
verify in-game; replaced audio round-trips through the bundled encoder.

---

## Dependency order (summary)

```
5 (Roster Builder + 5.5b manual) ─▶ 6.1 Portraits & Logos      (no Blender)
                                     6.2 Mesh + Blender harness ─┬─▶ 6.3 Appearance (CAP)
                                                                 ├─▶ 6.4 Hockey Shop
                                                                 └─▶ 6.5 Headmesh porting
                                     6.6 Audio                   (independent; last visual→audio)
```

## Deferrals (out of this phase)

In-Studio mesh **editor** (import-only for now); full audio **authoring** (trim/mix/normalize);
any-to-any headmesh pairs beyond those individually verified; the exclusive-path head-*import* research
([../MESH_IMPORT_EXCLUSIVE_PATH.md](../MESH_IMPORT_EXCLUSIVE_PATH.md)) — distinct from 6.5 porting.

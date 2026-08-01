# Phase 5 — Historical (FHM) + import: Roster Builder (implementation plan)

> **Status:** 📋 scoped 2026-07-17 (Stages 1–2 shipped; **Stage 3 / Roster Builder not started**).
> Authored after Phase 4 ([phase-4-roster-ops-transactions.md](phase-4-roster-ops-transactions.md),
> complete 2026-07-14), whose entity ops + capacity normalization are this phase's write surface.

ROADMAP Phase 5 is *"FHM corpus build, Historical Teams/Players, Formulas, Roster Builder
(preview/commit pipeline + instance registry)"*. Three of the four shipped incrementally; this plan
covers **only the remaining one — the Roster Builder / two-phase import**, the first surface that
*writes* corpus data into an EA project and the first to bridge both databases.

| Stage | Deliverable | Status |
|---|---|---|
| 1 — Corpus + Browse | `tdb-fhm` / `-build` / `-store`, CSV→SQLite (schema v6), Historical Players + Teams | ✅ shipped 2026-07-05 |
| 2 — Formula engine | `tdb-formula`, computed-ratings tab, Formulas page + profiles | ✅ shipped 2026-07-05 |
| **3 — Roster Builder / import** | cart → match → `preview_change_set` / `commit_change_set` → instance registry | **📋 this plan** |
| **3b — Manual roster construction** | player-select slot-fill for teams absent from the FHM corpus (same cart/preview/commit spine, different source) | **📋 §5.5b below** |

**Goal.** Import FHM player-seasons (and whole team-seasons) into the open EA project via a
cart → preview → commit pipeline, with preview **byte-identical** to commit, respecting every EA-DB
invariant.

**Decisions already made (carried forward):** cart item = **a specific player-season**; on a
same-person match, **ask per cart item** (update-existing vs create-new).

---

## What already exists (build on this — do not rebuild)

Verified against the tree at `535d1e9`. **These citations correct the prior planning notes**
(`~/.claude/plans/rename-the-database-primary-eager-hennessy.md`, 2026-07-05), which predate Phase 4
and got several seams wrong — see "Corrections" below.

**Write surface.** `EditTransaction::create_player(team_id, position) -> (new_wBIz, OpOutcome)`
([txn.rs:514](../../crates/tdb-edit/src/txn.rs#L514)) atomically stages the whole cluster — `ubPc`
bio (minted `wBIz`, `vjla`, `dXSB`) → kind-correct companion pair (`dSvy`+`xmja` skater / `mHuy`+`fMVl`
goalie, goalie iff `position == 4`) → `ZBac` index row → `eGlu` slot — inside
`with_repairing_transaction`. It is **block-coherent by construction**: `eglu_placement`
([txn.rs:480](../../crates/tdb-edit/src/txn.rs#L480)) reuses a vacant in-block slot or inserts at the
team's block boundary, never appends at table end. `Workspace::create_player`
([workspace.rs:499](../../crates/studio-domain/src/workspace.rs#L499)) wraps it as a dual-create
guarded by `fully_aligned()`, rolling both files back on divergence.

**The audit.** `with_repairing_transaction` ([txn.rs:384](../../crates/tdb-edit/src/txn.rs#L384))
snapshots, runs the op, then audits four db-only predicates in order — `single_captain` →
`companion_presence` → `towx_coherence` → `dup_on_team` — restoring the snapshot on any violation.
Referential/enum predicates run at **save** time only. The import composes ops; it does not re-implement
these invariants, it **surfaces them in preview**.

**Capacity (Phase 4.4).** `Workspace::normalize_roster_capacity`
([workspace.rs:187](../../crates/studio-domain/src/workspace.rs#L187)) grows the roster save's
`max_records` to the audited ceiling at open. This is what makes a bulk import viable at all —
without it every insert into a packed save pays per-record growth.

**Corpus reads.** `FhmCorpus::player_fhm_values(person_id, season_year) -> BTreeMap<String, f64>`
([tdb-fhm-store/src/lib.rs:1021](../../crates/tdb-fhm-store/src/lib.rs#L1021)) is the import's primary
read; `person_identity` (:1001), `person_played_years` (:341), `attribute_maxima` (:1090) support it.
Slugging: `tdb_fhm::slug::{person_slug, normalize_name, normalize_dob}`
([slug.rs:16/26/51](../../crates/tdb-fhm/src/slug.rs#L16)).

**Ratings.** `studio_domain::formula::compute_person_ratings(corpus, person_id, season_year, profile,
maxima) -> Option<ComputedRatings>` ([formula.rs:33](../../crates/studio-domain/src/formula.rs#L33))
is the entry point (not the raw `tdb_formula::compute`). `compute_person_career_ratings` (:101) yields
`best_season_year`/`peak3` — directly useful for a "which season?" affordance in the cart.

### Corrections to the prior planning notes (verified 2026-07-17)

1. **`EditTransaction::set_field` is private** ([txn.rs:152](../../crates/tdb-edit/src/txn.rs#L152)).
   The import writes through **`Workspace::set_field`**
   ([workspace.rs:216](../../crates/studio-domain/src/workspace.rs#L216)), which validates + encodes
   against metadata and runs the repairing txn. (Beware the unrelated legacy
   `tdb_edit::EditSession::set_field` at [tdb-edit/src/lib.rs:135](../../crates/tdb-edit/src/lib.rs#L135)
   — a raw-id staging builder, *not* this path.)
2. **`Capabilities` gates nothing on the write path.** It is a read-only probe
   ([capabilities.rs:23/61](../../crates/studio-domain/src/capabilities.rs#L23)) surfaced to the
   frontend to hide UI. Writing a field a title lacks fails as `UnknownField`
   ([txn.rs:360](../../crates/tdb-edit/src/txn.rs#L360)); **the importer must consult `Capabilities`
   itself** before writing optional per-title fields (draft, `year_ufa`, `player_style`, …).
3. **`ubPc.dXSB` and `eGlu.dXSB` are different encodings.** On the **bio** it is **1-indexed with
   `0` = free agent** ([view.rs:320](../../crates/studio-domain/src/view.rs#L320), :521-523); on the
   **slot** it is the raw team id used directly ([entity.rs:45](../../crates/tdb-relations/src/entity.rs#L45)).
   An importer writing a bio team must apply the +1. **This is exactly the class of off-by-one that
   silently corrupts** — cover it with a test.
4. **Birthdate is 0-indexed on disk.** `qFnd` = year + 1900, `JKLp` = month + 1, `Kswi` = day + 1
   ([view.rs:1053-1055](../../crates/studio-domain/src/view.rs#L1053)). FHM's `YYYY-MM-DD` must
   subtract 1 from month/day. `qFnd` is **overloaded** — on career tables it is the season-end year,
   not a birth year (view.rs:1063).
5. **Do not source field ids from [phase-3-ea-db-core-views.md:95](phase-3-ea-db-core-views.md#L95)** —
   it lists character-reversed forms (`pkRG`/`hleL`/`sFgQ`) that are stale/wrong against the real
   defs (`GRkp`/`Lelh`/`QgFs`). Source ids from `defs/enums/*.toml` + `tdb-relations::entity::ids`.
6. **`person_seasons` is private** ([tdb-fhm-store/src/lib.rs:298](../../crates/tdb-fhm-store/src/lib.rs#L298))
   — reachable only via `person_detail`. Needs `pub` to read season rows without the full detail.

---

## Phase 5.0 — Discovery spikes (gate the rest; do before coding the dependent stages)

Each spike lands its finding in `docs/audits/` with provenance + confidence (CLAUDE.md rule 3), and
names the sub-phase it unblocks.

> **✅ All four spikes resolved 2026-07-26** (proofs in `crates/studio-domain/tests/` + findings in
> `docs/audits/`):
> - **S1** → [ratings-write-codec.md](../audits/ratings-write-codec.md) — write is byte-identical
>   inverse of read using the target field's `FieldView.edit_offset` (36 modern / **50 gen6** / 0 PS4);
>   a fixed offset (the `EaRating.raw_offset` shape) silently corrupts. No `rating_offset` visibility
>   change needed. `ratings_write_codec.rs`.
> - **S2** → [ea-match-key.md](../audits/ea-match-key.md) — DOB is ~universal (≥99.4% every anchor),
>   so name+DOB is a viable *primary* key, but **not unique** (0.1–6.1% residual, mostly multi-pool
>   duplicates) → `match_player` returns candidates, per-item Match/Create is first-class.
>   `ea_match_key.rs` (`#[ignore]`). *nhl10/ps3 meta restored 2026-07-26 (platform-invariant, from the
>   committed Xbox360 nhl10 meta; verified by `nhl10_meta_smoke.rs`) — its S2 row fills on the next
>   `--ignored` sweep.*
> - **S3** → [import-history-cost.md](../audits/import-history-cost.md) — per-op writes retain N
>   full-DB snapshots (≈2–8 GB for a mirrored team-season); **one import commit = one transaction/one
>   undo entry**. New 5.2 seam: an import-scoped batch. `import_history_cost.rs` (`#[ignore]`).
> - **S4** → [capacity-full-policy.md](../audits/capacity-full-policy.md) — boundary-insert is
>   mechanically safe (grows a full block + re-parses), but the write path grows **past** the ceiling
>   with no check → the ceiling gate lives in the 5.2 preview; storage blocker, never silent drop.
>   Blocker wording pinned. `capacity_boundary_insert.rs`.

### Spike S1 — Ratings write codec (`raw_offset`) ⟵ **gates 5.2**

**Partly resolved at scoping time (2026-07-17) — the seam is narrower than the prior notes implied.**
The read side already derives the offset **data-driven from the target field's own metadata range**:
`attributes::rating_offset(studio_table, meta_name, range_low, range_high)`
([attributes.rs:90](../../crates/studio-domain/src/attributes.rs#L90)) returns `Some(36)` for `(0,63)`
6-bit storage (PS3/X360), `Some(0)` for `(0,99)` storage (**PS4 — the stored value *is* the rating**),
`None` for non-ratings. So "PS4 uses 0" is **already handled**, and no new `Capabilities` flag is
needed — the prior notes' framing ("apply per `Capabilities`") is misleading.

**The real hazard:** `EaRating.raw_offset` ([tdb-formula/src/lib.rs:58](../../crates/tdb-formula/src/lib.rs#L58))
is computed by the formula engine **with no knowledge of the target project**, so it can *disagree*
with the loaded title's actual storage (formula says 36; a PS4 target stores `0..99`). **The import
must derive the offset from the target field's metadata via `rating_offset` — never trust
`EaRating.raw_offset`.**

*Resolve:* (a) widen `rating_offset`'s visibility (`pub(crate)` today) for the import module;
(b) confirm write is the exact inverse of read by round-tripping computed ratings on ≥2 titles + the
PS4 anchor; (c) pin the `_ => None` fallthrough policy — a rating field whose range matches neither
pair must be **skipped and reported**, never written with a guessed offset. **Output:** an audited
`human → raw` rule + a `raw_offset`-disagreement test.

### Spike S2 — EA match key: DOB coverage + ambiguity ⟵ **gates 5.1**

Matching is by normalized **name + DOB**, but nothing indexes EA players this way today
(`person_slug` is corpus-only). Unknown: how many EA bios carry a **usable** DOB vs a sentinel/zero,
and the resulting ambiguity rate (same name, no DOB).

*Resolve:* over the committed anchors, compose `qFnd`/`JKLp`/`Kswi` and measure: % with a plausible
DOB, % zero/sentinel, and the collision rate of `normalize_name(last, first)` alone. **Output:** the
match key + a documented tie-break/ambiguity policy (the cart asks per item, so ambiguity is a UI
state, not a failure — but the rate decides how prominent that UI must be).

### Spike S3 — Bulk-import history cost ⟵ **gates 5.2 design**

`EditSession::run` clones the **whole working buffer** per op onto the undo stack
([session.rs:300-307](../../crates/studio-domain/src/session.rs#L300)), ×2 on a mirrored workspace.
A cart of N players × (create + ~15 field writes) is **O(N × 16 × full-DB)** clones — a 700-player
team-season import could mean thousands of multi-MB copies.

*Resolve:* measure a realistic cart (1 player, 25 players, a full team-season) against an anchor.
**Output:** the batching decision — one undo entry per *commit* (not per field write), via either a
batch/transaction-scoped session mode or an import-specific path that stages many edits under a single
`with_repairing_transaction`. **This likely needs a new seam in `EditSession`; scope it here, not
mid-5.2.**

### Spike S4 (minor) — Capacity-full policy ⟵ **gates 5.2 blockers**

Carried from the prior notes as an open item. Default proposal: **boundary-insert** to grow a full
destination block when still under the vanilla ceiling (Phase 4.4 normalization means headroom
usually exists); otherwise surface a **storage blocker** in preview — never a silent drop.
*Resolve:* confirm the insert path is safe for a full block at the ceiling, and pin the exact
preview-blocker wording.

---

## Phase 5.1 — Backend: `EaSlugIndex` / `match_player` ⟵ gated on S2 — ✅ shipped 2026-07-27

> Implemented in `crates/studio-domain/src/import.rs` (`EaSlugIndex` + `match_player`), exposed via
> `EditSession::ea_slug_index()`; tests in `tests/ea_slug_index.rs`. Returns ambiguity-aware
> `MatchCandidate`s (name+DOB high-confidence → name-only review), human-readable name + team. Index
> **caching** is deferred to 5.4 (the accessor builds fresh today).

A lazy index over the **open EA project's** players keyed by normalized name + DOB, reusing
`tdb_fhm::slug::{normalize_name, normalize_dob}` so both sides fold identically. Composes EA DOB from
`qFnd`/`JKLp`/`Kswi` with the decode in Correction 4. Returns ambiguity-aware candidates
(`Vec<MatchCandidate>`, not a single id) so the cart can offer Match vs Create per item.

*Verify:* unit tests over an anchor — a known EA player matches their corpus person; a DOB-less EA
bio degrades to a name-only candidate flagged ambiguous; a corpus person absent from the anchor
returns no candidate. Index build is lazy + cached (do not rebuild per cart item).

## Phase 5.2 — Backend: `studio-domain::import` — the two-phase pipeline ⟵ gated on S1/S3/S4

> **Progress (2026-07-27):**
> - **5.2a — batch seam ✅** (`EditTransaction::apply_import` + `EditSession::apply_import`;
>   `create_player` split into unaudited `create_player_inner` + audited wrapper → a whole cart is one
>   audited transaction / one undo entry, Spike S3). `ImportOp`/`ImportWrite`/`ImportTarget`/`ImportCell`
>   are the lowered edit form. `tests/import_batch.rs`: one-undo-entry, byte-identical lowering, atomic
>   rollback.
> - **5.2b lowering core ✅** — `PersonSeason` (corpus-extracted seam) + `lower_create` (CreateNew) +
>   `lower_update` (Match, no allocation), sharing `person_writes`. Bio identity + height (+54) + weight
>   (base) + birthdate (probed 0/1-index) + jersey + ratings (S1 `edit_offset`), skip-and-report on
>   out-of-range. `tests/import_lower.rs` round-trips every field on nhl14 **and nhl10** (the 1-index
>   drift).
> - **5.2 two-phase pipeline ✅** — `EditSession::preview_import` (non-mutating dry-run: player-delta,
>   audit over the previewed state, skip report, exact commit bytes) + `commit_import`.
>   `tests/import_preview.rs`: preview doesn't mutate, and **commit is byte-identical to preview** on a
>   mixed Create+Match cart.
> - **5.2b corpus adapter ✅** — `person_season_from_corpus` (person_detail bio + season height/weight
>   + dob + EA position code + ratings via `compute_person_ratings`). The formula's `ea_key` display
>   names are bridged to on-disk field suffixes by an **explicit, verified `ea_key→suffix` table**
>   (25 skater + 18 goalie; the 3 AI biases + goalie "Angles" deferred, not guessed). **Proven
>   end-to-end** against the real `corpus/historical.db` (`tests/import_corpus_e2e.rs`, `#[ignore]`): a
>   corpus person lowers into nhl14 with bio + physicals + 25 ratings, 0 skips, on the roster.
> - **5.2b cart orchestration ✅** — `ImportItem { person_id, season_year, target_team_id, identity:
>   CreateNew | Match(wBIz), jersey? }` + `lower_change_set` map a whole cart → encoded ops through the
>   corpus adapter + `lower_create`/`lower_update`, returning a `LoweredChangeSet { ops, skipped, items }`
>   with a per-item `ItemOutcome` trail (Created / Matched / **PersonMissing** / **NoSuchPlayer** /
>   NoBioTable). Un-lowerable items are non-fatal **blockers** (no op, failing outcome) so preview can
>   surface them; `EditSession::lower_change_set` bridges the working state → the same
>   `preview_import`/`commit_import` ops (byte-identical). `tests/import_change_set.rs` (synthetic
>   in-memory corpus, **CI**): mixed Create+Match+2 blockers → 2 ops, right outcomes, preview==commit.
> - **5.2b enum identity + style ✅** — the adapter now resolves **handedness** (`ubPc.GRkp`, corpus
>   "Left"/"Right" → enum raw), **nationality** (`ubPc.Lelh`, corpus country name → enum raw, via the
>   verified `defs/enums/nationality.toml` with one FHM spelling alias `Czechia`→`Czech Republic`; an
>   unmapped country is skipped, never guessed), and skater **style** (`dSvy.QgFs`, the archetype
>   classifier's 1-based `ea_raw`; `None` for goalies). Resolution goes through the loaded, verified
>   enums (`EnumRegistry::for_field`/`raw_of`), so it is title-aware and hardcodes no code. **Proven on
>   real data** (`import_corpus_e2e`): Joel Åhgren → hand 0 (Left), nat 69 (**Sweden**), style 9
>   (Two-Way Forward). CI: `import_change_set` asserts GRkp+Lelh land in the create op.
> - **Remaining (5.2b-next):** **birthplace** (`ubPc` birthcity string + `QvxI` birth-state — needs the
>   postal-code→alphabetical bridge; deferred, not guessed) and the three AI **biases** (0..15, no
>   verified formula source → still deferred); **full per-table capacity projection + S4 blockers** and
>   per-item **diffs** + 5.1 match candidates in the preview; ledger to 5.3 on commit. **Verify (open):**
>   the bio-team **+1** (Correction 3) — roster membership is confirmed via `eGlu`; confirm `ubPc.dXSB`
>   reads back to the target team before relying on it.

**There is no dry-run/preview anywhere in the stack today** — `commit()`
([txn.rs:1784](../../crates/tdb-edit/src/txn.rs#L1784)) is a CRC finalizer, not a two-phase gate.
This sub-phase builds it.

- **`ImportChangeSet`** = `Vec<ImportItem>`, each `{ person_id, season_year, target_team_id,
  identity: Match(ea_player_id) | CreateNew, jersey?, profile_id }`.
- **`lower(plan, session) -> LoweredEdits`** — the **single shared path** that makes preview and
  commit byte-identical (this is the invariant the whole design turns on):
  - `CreateNew` → `create_player` → bio fields from the corpus person-season (name, height/weight,
    birthdate, birthplace, nationality, position, handedness, bio team **+1**) → ratings from
    `compute_person_ratings` (human→raw per **S1**) → style `QgFs` (archetype; `None` for goalies) →
    potential/traits → jersey `sVRt`.
  - `Match(id)` → `set_field` updates to the matched player's existing bio/companion records; **no
    allocation**.
  - Fields absent in the loaded title are **skipped per `Capabilities`** (Correction 2).
- **`preview_change_set(plan) -> ChangeSetPreview`** — run `lower()` against a snapshot and collect
  per-item **diffs** (old→new for matches, new record for creates), **validation findings** (the four
  audit predicates), **capacity projection** per table, **storage blockers** (S4), **same-person
  matches** (5.1), and portrait status (report "no match" only — portrait replace is the Assets phase).
- **`commit_change_set(plan)`** — re-run the **same** `lower()` for real through `Workspace`
  (roster-mirror aware), then the normal save spine; on success, ledger each `(person, season, target)`
  to the instance registry (5.3).

*Verify:* a **golden test that `preview` and `commit` lower to byte-identical edits** (the headline
gate); tests for capacity blockers, `Capabilities` skipping on a title lacking a field, the bio-team
+1 (Correction 3), and the birthdate 0-index (Correction 4), against synthetic fixtures.

## Phase 5.3 — Backend: `crates/tdb-instance-registry` (new SQLite sidecar)

A **non-blocking** ledger of imported `(person_id, season_year, target game/db id, new ea_player_id)`.
Preview **warns** (never blocks) when a cart item was already imported to this target.

> **Done (2026-07-28).** `tdb-instance-registry` crate: `InstanceRegistry` over a single `imports`
> table keyed `(person_id, season_year, target)` — `target` an opaque, caller-chosen EA-db identity.
> `record` upserts (re-import overwrites the same key); `lookup` is the dedup-warning source;
> `entries_for_target` lists a target's history. **Graceful degradation is structural:** `open` never
> returns an error — a missing file is created, and a corrupt file / version skew / unwritable path
> opens **disabled** (reads empty, writes silent no-ops), so the ledger can never block an import. The
> crate is clock-free (`imported_at` supplied by the caller). Unit tests cover round-trip+dedup,
> re-import overwrite, the disabled no-op, and corrupt-file→disabled. **Wiring into
> preview/commit is 5.4** (the service layer owns the target id + ledger lifetime).

*Verify:* round-trip a ledger entry; a re-preview of the same item surfaces the dedup warning; a
missing/corrupt sidecar degrades to "no ledger" rather than failing the import. ✅

## Phase 5.4 — Service layer: DTOs, convert, commands

Per the 6-point registration pattern (DTO → convert → registry method → `#[tauri::command]` →
`command_handler!` at [lib.rs:28](../../crates/studio-api/src/lib.rs#L28) → `export_typescript` at
[lib.rs:108](../../crates/studio-api/src/lib.rs#L108); **missing the macro = silently unavailable at
runtime**).

- DTOs: `ImportItemDto`, `ChangeSetPreviewDto` (diffs / validation / capacity / blockers / matches),
  `MatchCandidateDto`, `ImportResultDto`.
- Commands: `import_preview(items)`, `import_commit(items)`, `match_ea_player(personId, seasonYear)`,
  `roster_builder_capacity(teamId)`.
- These take **both** a `projectId` (EA target) **and** read the corpus — the **first commands to
  bridge both databases**. `with_workspace` ([registry.rs:503](../../crates/studio-api/src/registry.rs#L503))
  and `with_corpus` (:1008) are separate today; bridging them is a real seam, not a formality.

> **Done — core slice (2026-08-01).** DTOs `ImportItemDto` (in; `match_ea_player_id: null ⇒ create`),
> `MatchCandidateDto`, `ImportItemResultDto`, `ChangeSetPreviewDto`, `ImportResultDto` (+ convert +
> `ts-rs` bindings regenerated). Commands `match_ea_player(projectId, personId)`,
> `import_preview(projectId, items)`, `import_commit(projectId, items)` — all registered in
> `command_handler!` + `export_typescript`. The bridge is real: `import_preview`/`commit` resolve the
> active profile + maxima, then hold the **corpus + workspace** locks together to lower → preview/commit.
> Commit is **roster-mirror aware** (`Workspace::commit_import` fans the cart to the mirror with the
> full-alignment guard, rolling both files back on divergence). The **ledger** opens beside the corpus
> (`ea_import_ledger.db`) on `open_corpus`; commit records each lowered item (best-effort), preview
> flags `already_imported`. A registry test drives preview → commit → **re-preview shows the dedup
> warning** on the `nhllegacy` anchor with a synthetic corpus. ✅
>
> **Follow-up — capacity gate ✅ (2026-08-01).** The **S4 ceiling gate** now lives in the preview, as
> the audit demanded. `capacity::project_capacity(db, game_id, PendingCreates)` projects each of the 7
> roster tables' fill after the cart's creates: `ceiling = max(gen7_ceilings(game_id), db.max_records)`,
> `over = enforced && current + adding > ceiling`, with `enforced=false` when no reliable ceiling exists
> (unknown title on a packed file) so it never falsely blocks. A create adds to the shared
> index/bio/slot tables + the **kind-correct** AI/equip companions (`count_creates` splits skaters vs
> goalies). Surfaced through `Workspace::roster_capacity` → `ChangeSetPreviewDto.capacity[]` +
> `capacity_blocked`; **preview** renders per-table bars (current‖adding vs ceiling, over flagged) and
> disables Commit, and **`import_commit` hard-refuses** an over-capacity write (never a silent
> over-ceiling save). `tests/capacity_projection.rs`: a 22-player cart fits every nhl14 ceiling
> (ubPc=6873), 100k skaters overflow bio, unknown-title packed file stays unenforced.
>
> **Polish — created `wBIz` ✅ (2026-08-01).** `EditTransaction::apply_import` now returns
> `ImportApplied { outcome, created }` — the new `wBIz` per `Create` op, in op order (collected via a
> captured side-channel so the `OpOutcome`-fixed repairing-transaction harness is untouched). Threaded
> through `EditSession`/`Workspace::commit_import` (the canonical db is the authority; an aligned mirror
> mints the same ids) → `import_commit` maps each created id back to its Created item, **ledgers the
> real id** (no more `0` placeholder) and echoes it in `ImportItemResultDto.ea_player_id`. The registry
> test asserts the surfaced id is a real player on the roster.
>
> **Still deferred:** `roster_builder_capacity(teamId)` as a standalone command (folded into preview);
> per-item **diffs** and embedded 5.1 **match candidates** in the preview.

*Verify:* `cargo test -p studio-api` regenerates bindings; a registry test drives preview → commit
end-to-end on an anchor. ✅

## Phase 5.5 — Frontend: Roster Builder

- New **Historical → Roster Builder** subtab (the cart; FE state, persisted).
- **"Send to Builder"** on the historical **season row** (person timeline) and on **team-detail roster
  rows** (a whole team-season).
- Each cart item picks a **target team** (from the project's `list_teams`), shows the auto-detected
  **match** (toggle Match/Create), an optional jersey, and a profile.
- Cart → **Preview** (render `ChangeSetPreviewDto`: diffs, validation, capacity bars, blockers,
  matches — **reuse the Dashboard finding renderer**, which Phase 4.6 already extended with repair
  affordances) → **Commit**.
- Requires an open EA project (guaranteed by `AppShell`) **and** an open corpus (`CorpusGate`).
- **Human-readable surfaces rule:** every cart/preview line resolves team + player names; raw ids only
  behind a details affordance.

> **Done — first slice (2026-08-01).** New **Historical → Roster Builder** subtab
> (`app/src/pages/RosterBuilder.tsx`), route under `CorpusGate`, nav entry. A localStorage-persisted
> cart store (`app/src/lib/rosterCart.tsx`, provider in `main.tsx`) survives reloads and dedups by
> `(person, season)`. **"Send to Builder"** lands on the Historical player's **season-timeline rows**
> (gated on an open project; shows *Queued* once added). The cart page: per-row **target-team** picker
> (`list_teams`), **Match/Create** dropdown fed by `match_ea_player` (per-person, lazy), jersey; then
> **Preview** (renders `ChangeSetPreviewDto` — per-item outcome pills, player-delta, blockers, the
> audit summary, the ledger *already imported* flag) → **Commit** (disabled while `blocked > 0`; clears
> the cart + invalidates the DB views on success). `npm run build` clean. **Human-readable surfaces:**
> team + player **names** throughout; the `wBIz` rides only inside the Match option value.
>
> **Whole team-season send ✅ (2026-08-01).** The Historical **team-detail** roster tab now carries a
> "Send team-season to Builder (N)" button that queues every roster member of that constituent's season
> into the cart (dedup by `(person, season)`; N counts the not-yet-queued; gated on an open project).
>
> **Deferred (next FE slice):** capacity bars already landed with the 5.4 follow-up; remaining is a nav
> **cart-count badge** and per-item **diffs** in the preview.

*Verify:* `npm run build` clean; the cart survives a reload; preview blockers disable Commit. ✅

## Phase 5.5b — Manual roster construction (added 2026-07-26)

**Motivation.** Many teams have no whole-team-season in the FHM corpus, but the *individual players*
do exist there (per-season, name+DOB identity). Roster Builder must also let a user **hand-fill named
roster slots** by picking individual player-seasons — an original, curated roster rather than a corpus
team-season replay.

**Reuse, don't rebuild.** This is a **second source feeding the same spine**, not a new write path:
- Source: individual player-season picks (from Historical Players search across all seasons / the
  person timeline), each dropped onto a chosen **target slot** on a target team.
- The item lowers through the **identical** `ImportItem` → `lower()` → `preview_change_set` /
  `commit_change_set` pipeline (§5.2), so it inherits byte-identical preview↔commit, the four audit
  predicates, capacity projection, `Capabilities` skipping, the bio-team **+1** and birthdate 0-index
  corrections, and the instance-registry ledger (§5.3) for free.

**New surface (small):**
- A **slot-target picker** in the cart: choose team → position/line slot (vs. the team-season flow's
  "whole roster"). Slot occupancy resolves via `eGlu` (respect `vfEq==0` occupancy key — see the
  `eglu-occupancy-key-vfeq0` note; `XSWT==0` reads vacant).
- A **cross-season player search** source that emits the same per-item shape as "Send to Builder"
  (person_id + chosen season_year), with the `compute_person_career_ratings` "which season?" affordance.

*Verify:* a cart mixing a whole team-season **and** hand-picked slot-fills lowers to one coherent
change set; preview↔commit byte-identical; filling an occupied slot surfaces a diff (update) while an
empty slot creates; a full block at the ceiling surfaces the S4 storage blocker, never a silent drop.
**Human-readable surfaces:** slot/target/player lines resolve to real names; raw ids behind details.

> **Done — slot-fill slice (2026-08-01).** The cart row's placement control now unifies both sources:
> an **optgroup "Same person (suggested)"** (the `match_ea_player` name+DOB candidates, 5.5a) and an
> **optgroup "Replace on `<team>`"** — every current occupant of the chosen target team (from
> `list_players`, filtered by `team_id`, minus anyone already offered as a same-person match). Picking a
> "Replace" occupant sets `matchEaPlayerId` ⇒ the item lowers as an **Update** in that slot; "Add as a
> new player" ⇒ **Create**. Occupants render **name · position · #jersey** (human-readable). Changing the
> target team clears a slot-fill chosen against the old team. A **cross-season quick-add** ("＋ Builder")
> now sits on the Historical Players **search rows** (latest season; gated on an open project) as the
> fast curated-roster path, complementing the detail-timeline per-season add. `npm run build` clean.
>
> **Deferred:** capacity blockers still surface at **commit** (not preview) until the 5.4-follow-up
> capacity projection lands; whole **team-season** "Send to Builder" and a season-chooser on the
> quick-add are still to come.

## Phase 5.6 — Final verification (gate sign-off)

1. **Functional E2E:** add a historical player-season to the cart → pick a target team → preview (new-
   player diff + capacity) → commit → the player appears in the Game-Database Team detail with
   era-plausible ratings and a **clean validation Dashboard** → save → reload stable; re-preview the
   same item shows the instance-registry dedup warning.
2. **Tests:** `cargo test --workspace` + `cd app && npm run build` clean; **zero new clippy warnings**.
   (Note: this environment intermittently hits a Windows Application-Control block — `os error 4551` —
   on freshly-linked test binaries; `touch` the test file to force a relink. Not a test failure.)
3. **Anti-pattern grep:** no import path bypassing `with_repairing_transaction` or writing raw ids
   around `Workspace::set_field`; no hardcoded rating offsets (must trace to the S1 audit); no
   hand-edited bindings.
4. **Format-fact honesty:** S1/S2 outputs recorded with provenance/confidence; nothing unverified
   asserted as in-game-true.

---

## Risks & deferrals

- **S1 de-risked at scoping, but the failure mode is silent.** The offset is already data-driven from
  target metadata, so the work is plumbing + a round-trip proof, not reverse-engineering. The residual
  risk is that trusting `EaRating.raw_offset` (or guessing on the `None` fallthrough) writes a
  **wrong-by-36** rating on every imported player with no error. Prefer skip-and-report over any
  guess; if the round-trip proof resists, ship bio + identity only and leave ratings to the manual
  Attributes tab.
- **History cost (S3)** could force a new `EditSession` batching seam. That is a change to a Phase-2
  spine that every op depends on — treat it as its own reviewed change, not an import detail.
- **Deferred:** portrait replace (Assets phase — Stage 3 only *reports* match status); whole-league
  import; undo *across* a commit (the commit is one history entry, not N).
- **Out of scope (separate follow-up):** wiring the historical **team view** to the SCHEMA-v5 franchise
  lineage (list = franchises + standalone non-NHL teams) — a Stage-1 loose end, unrelated to import.

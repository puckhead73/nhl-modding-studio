# League overnight run — consolidated receipt (2026-09-17)

> **Corrected by LEAGUE-R1 (2026-09-17, [review](../validation/league-overnight-review-and-r1-dispatch.md))** —
> three overclaims in the original wording are struck below: not every title/code has a confirmed
> label (gen7 labels are candidate or corroborated per `defs/leagues`, several codes stay raw);
> decoding the gen7 dictionaries does **not** establish which dictionary key a gen7 league code
> resolves to; and the NHL 19 / NHL 21 dictionaries are **missing from the supplied fixtures**, not
> absent from the shipped games. The checkpoint hash listed in §2 was stale (the file had already
> been rewritten to its closed-run pointer): the correct value is
> `2ddc047fd27613010bbd7dce316510a803d3c2a7fd480c790e251a95e32b8de2`.

**Plain-language summary.** Studio now has a real Leagues section: every game shows its leagues
with the best label established for that title — confirmed from the game's own league table on
PS4, measured-but-unconfirmed ("candidate"/"corroborated") on the older consoles, and a raw code
where nothing is established (the old one-size-fits-all label list, which was wrong for most games,
is gone). On the PS4 titles a league's name key can be edited, reviewed and saved into a copied
game folder — including straight into a copy of the game's archive — then restored exactly. The
Pending Edits page now lists what actually changed (which team, league or player, which field,
before → after) instead of just a count, and saving in place now really clears the "unsaved" state.
Studio can also read every supplied language dictionary (PS4 and the older consoles); on PS4 that
turns league keys into on-screen words once the title's dictionary is attached, while on the older
consoles the dictionaries decode but which entry a league code maps to is still unknown. What
remains unavailable: nothing here has been run inside a game; the older PS3 / Xbox 360 titles have
no league name to edit in the database (only localization, a later phase); the NHL 19 and NHL 21
dictionaries were not among the supplied fixtures, so their keys are unverified here; and none of
the app screens were exercised in a running desktop build (type-checked and built only).

---

## 1. Package, disposition, scope

* **Brief:** [league-overnight-execution-2026-09-17](../plans/league-overnight-execution-2026-09-17.md)
  (user-authorized autonomous run; all eight queue items O1–O8 attempted).
* **Disposition:** O1–O8 **implemented and developer-verified**; nothing independently reviewed,
  accepted, staged, committed, pushed or installed live. **Phase 1.1 is not closed**: runtime
  (in-game) verification is not part of this run for any target.
* **Baseline:** `feat/phase5-asset-tooling` at `8d60a6d`; the pre-existing dirty tree (unrelated
  streams, untracked reference material) was preserved and not touched.

## 2. Changed paths and hashes (SHA-256 prefix; 70 files)

```text
ecab957e451cc2ff  Cargo.toml                                   (workspace member tdb-loc)
0c44a9b102c9df8a  Cargo.lock
503e893d9c157d18  crates/tdb-loc/Cargo.toml                    (new crate)
d7188ce925368a46  crates/tdb-loc/src/lib.rs                    (new)
73b2433cbd30e07c  crates/tdb-loc/tests/dictionaries.rs         (new)
66c2854b36fa4cf6  crates/tdb-archive-big/src/stage.rs          (20-byte rows)
8b8a459b53b993f8  crates/tdb-archive-big/src/stage/tests.rs
bfd80cbf59454b0e  crates/tdb-archive-big/tests/ps4_real_archives.rs (new, #[ignore], env-gated)
8fb6632966e60fc6  crates/tdb-enums/tests/enums.rs
da17134bedc46629  crates/studio-domain/Cargo.toml
5b8c7331c2935d19  crates/studio-domain/src/lib.rs
75dd54c7dd00ae70  crates/studio-domain/src/leagues.rs          (new)
d776d0d4bce919d3  crates/studio-domain/src/pending.rs          (new)
7bd31e1fc72ede27  crates/studio-domain/src/project.rs
65938d3e2f119f76  crates/studio-domain/src/session.rs
5fc2cbfa307e64f0  crates/studio-domain/src/workspace.rs
5c54125564f9dc51  crates/studio-domain/src/view.rs
e92219fd06c15fc3  crates/studio-domain/src/field_profile.rs
bc78b369359216f9  crates/studio-domain/src/error.rs
33d6800826e0980d  crates/studio-domain/src/audit.rs
65c06354581efbe7  crates/studio-domain/src/import.rs
1cbe0f3c44f6e995  crates/studio-domain/tests/leagues.rs        (new)
c5317b747c283a56  crates/studio-domain/tests/pending.rs        (new)
4d90bff65c9350c5  crates/studio-domain/tests/view.rs
8a842f3e06d9e1fb  crates/studio-domain/tests/field_profile.rs
2dff2acbe63bb32f  crates/studio-domain/tests/read_matrix.rs
1929260886e6570f  crates/studio-api/src/dto.rs
ff7a09d1c3a84837  crates/studio-api/src/convert.rs
14b82c3b74b6c20d  crates/studio-api/src/registry.rs
21a8625f3ff3b77c  crates/studio-api/src/registry/copied_target.rs
e5a1362124446637  crates/studio-api/src/commands.rs
8d9e5305e7a3ff9e  crates/studio-api/src/lib.rs
641bf3eef03cfeb9  crates/studio-api/src/bindings.rs            (76 → 80 export roots)
f906bfac9401a070  crates/studio-api/src/bin/edit_qa.rs
a7febcc85d4cf95b  crates/studio-api/tests/leagues.rs           (new)
ed5df6f35ddd5d37  crates/studio-api/tests/league_copied_target.rs (new)
79e98cdc6276dbdd  crates/studio-api/tests/league_offline_campaign.rs (new)
f34f5e01e1e6cd02  crates/studio-api/tests/copied_target.rs     (closure arity only)
19bdac2f7bf3296b  defs/leagues/README.md                       (new directory, 8 files)
a04d5337323cc685  defs/leagues/gen6-nhl07.toml
44d8ec106ea65b70  defs/leagues/gen7-nhl08.toml
0a2bb129b15185dd  defs/leagues/gen7-nhl09.toml
626c792de2b88b0d  defs/leagues/gen7-nhl10.toml
b9dd90bc5883e141  defs/leagues/gen7-nhl11.toml
56422b99d7c78d42  defs/leagues/gen7-nhl12-15.toml
6784a030a800de35  defs/leagues/gen7-nhllegacy.toml
   (deleted)      defs/enums/league.toml, defs/enums/conference.toml, defs/enums/division.toml
d7dd10b1d53b1350  app/src/lib/client.ts
4f99189f1f02f2dd  app/src/nav.ts
0ba121e0ff6bdff1  app/src/routes.tsx
16b5cecce798a2b3  app/src/AppShell.tsx
80096d034816543b  app/src/pages/Leagues.tsx                    (new)
76009acf464c2b24  app/src/pages/LeagueDetail.tsx               (new)
285ba8f157ee5f84  app/src/pages/PendingEdits.tsx               (rewritten)
3421cfe56e82a95b  app/src/components/TeamHeader.tsx
c6437f2443e0ca13  app/src/components/CopiedTargetPanel.tsx     (new)
bec321ddae16da68  app/src/bindings/TeamViewDto.ts              (generated, --write)
fbc6c0d61253bfdc  app/src/bindings/LeagueCatalogDto.ts         (generated)
a2b1b0eb308b70d3  app/src/bindings/LeagueSummaryDto.ts         (generated)
a1bd70a938a7d2fc  app/src/bindings/LeagueViewDto.ts            (generated)
e5ae3f9e02525ae9  app/src/bindings/DictionaryRefDto.ts         (generated)
3014277ee2b12328  app/src/bindings/PendingReviewDto.ts         (generated)
930aa7cc07e6df73  app/src/bindings/PendingFileDto.ts           (generated)
9baacc930edf1281  app/src/bindings/PendingCellDto.ts           (generated)
2692cb0494ec1f59  app/src/bindings/PendingTableDto.ts          (generated)
5a663e3d0116ade4  app/src/bindings/PendingIntegrityDto.ts      (generated)
fed00e2ce0369eb9  app/src/bindings/CopiedTargetRootDto.ts      (generated)
859c5daad209ff5b  docs/formats/ps4-eb-big.md
1c6857fb8d393ef4  docs/plans/league-identity-parity-matrix.md  (corrections banner + two claims)
066137951ed20730  docs/validation/league-identity-offline-campaign.md (new)
be732c6907263669  docs/receipts/league-overnight-checkpoint.md (superseded by this receipt)
```

Generated bindings were installed only through `export-bindings -- --write`; `--check` reports
"Bindings are in sync" (80 export roots). No Codex-owned roadmap/review record was altered.

## 3. What was built, by package, and the decisions taken

### O1 / L1.1-B — target-specific league identity (implemented + checked)

* `defs/leagues/*.toml`: one vocabulary per **measured code layout** (gen6 NHL 07, gen7 NHL 08,
  09, 10, 11, 12–15, Legacy/Recompiled), each with a schema fingerprint (league table/field/width,
  required/forbidden fields and tables) and per-entry confidence (`corroborated` when a PS4
  `stockleagues` table names the same layout on an identical team distribution — NHL 12–15 ↔ NHL 15
  PS4, Legacy ↔ NHL 16 PS4; `candidate` when inferred from member abbreviations). Unknown codes
  (NHL 09 "CEN", NHL 10 "NAJ/WPR", NHL 07 "DAV") have no entry and show raw.
* `studio-domain::leagues`: `LeagueCatalog` — PS4: rows of `stockleagues` joined on `leagueid`
  (record index is a handle, never an identity); gen6/gen7: derived codes labelled only when the
  fingerprint **and** the session's game id select exactly one vocabulary; otherwise codes stay raw
  with the reason (the NHL 12–15 / Legacy fingerprint is shared, so a session without a game id
  shows raw codes rather than a neighbour's labels). Group ids (`leaguegroup`/`conferencegroup`/
  `divisiongroup`) are a separate namespace; the only labelled ones are the NHL conference/division
  ids on the Legacy layout, scoped to league code 0.
* The global `league.toml` / `conference.toml` / `division.toml` enums were **removed**: they
  labelled every title with the Legacy layout (matrix §3.3). Enum-membership validation on `xMjj`
  goes with them; metadata range validation remains.
* Team surfaces share the catalog: `TeamSummary.league_label`, `TeamView.{league,league_label,
  conference_label,division_label}`, the Team Data "League" cell now binds the real `league`
  (`xMjj`) field with the catalog's options, and `leaguegroup` is exposed as "League group";
  `TeamHeader` and Transactions read the same labels. The game id is now carried on the sessions
  (`EditSession::set_game_id`, set by `Workspace::set_game_id`).
* No league→team relationship def was added: on PS4 the catalog reports team codes that reference
  no `stockleagues` row explicitly; on gen7 there is no native table to relate to (the brief forbids
  imposing PS4 rules there). No new topology operations.

### O2 / L1.1-C — league API and editing surface (backend checked; UI type-checked, not executed)

* `list_leagues` / `league_view` commands (`LeagueCatalogDto`, `LeagueSummaryDto`,
  `LeagueViewDto`), Leagues navigation, list (search by name/key/code, keyed by native code) and
  detail (banner, editable `stockleagues` row through the existing `FieldRow`/`set_field` path,
  member teams linked). Keys display as keys when unresolved; gen7 titles show code/member
  information with the name-edit blocker text.
* Constraints enforced server-side and exercised: byte-length (26 ASCII bytes and 13 two-byte
  characters refused; 12 two-byte characters accepted), immutable `leagueid`, undo/redo, a
  re-pointed key keeps its route (code).
* `edit-qa --spec` gained `set_league_field` addressing a league by `code:N` or, as a convenience,
  `key:<name key>` that **refuses ambiguous keys** (NHL 21's three `CUSTOM_TEAMS` rows).

### O3 / L1.1-D — pending-change review (implemented + checked)

* `studio-domain::pending`: baseline (`Project::db_bytes`) ↔ working diff per record/field through
  the metadata, entities resolved (team, player, league, roster slot → team, strategies → team),
  structural changes (row counts, capacity, layout, tables on one side) reported separately,
  derived integrity bytes (file/table headers, other bytes) summarised, decoded cells capped with an
  exact total, and a parse failure reported as an error — never an empty "no changes".
* **Correction 1 applied:** the baseline is the acknowledged loaded/saved image, not `undo[0]`.
  `Workspace::save_in_place` now adopts the written files as the baseline through the same verified
  rebind a copied-target commit uses (it needed `&mut self`); the app shell's Save and the Pending
  Edits "Save (in place)" call `save_workspace`; "Save as" keeps its legacy no-rebind behaviour and
  says so.
* `pending_changes` command + `PendingReviewDto` (primary and roster roles separately) and the
  rewritten Pending Edits page (entity / table / field / before → after, structural notes,
  integrity line, truncation note, error state).

### O4 / L1.1-E — copied-target review/apply/recovery UI (API loop checked; UI type-checked)

* `CopiedTargetPanel` on Pending Edits: root discovery (`copied_target_root_for` walks up from the
  primary file to the nearest marked root — never designates an unmarked directory), explicit root
  creation (`copied_target_create_root`, refuses existing paths), status with guidance, review →
  artifact table (destination, prior/output identities, qualification items, bound inputs) → apply
  → outcome (state, destinations, baseline; "written to the copied tree as unqualified artifacts,
  not an in-game installation"), cancel while applying, discard, recovery when the root is
  interrupted, restore per restorable transaction (disabled while a review is prepared), reload
  when a baseline went stale. A review made stale by an edit/undo/save disables Apply and says why;
  the previous handle is discarded on re-review, panel unmount and project change.
* Checkpoint evidence (API): league edit → pending → review → apply → allowed-difference audit →
  reopen in a fresh registry → byte-exact restore on a copied NHL 21 fixture; changed-after-review
  refusal with the destination untouched and the root freed; explicit discard; stale-handle refusal;
  root-creation refusal.

### O5 / L1.1-F — offline campaign (implemented + checked)

`crates/studio-api/tests/league_offline_campaign.rs` → [league-identity-offline-campaign](../validation/league-identity-offline-campaign.md):
all 7 PS4 anchors run the reversible equivalent operation (semantic row selection → stable id →
edit → pending → review → apply → audit → reopen → restore == anchor hash); all 20 gen6/gen7 anchor
rows verify browse/classify and the truthful name-edit blocker; Legacy Recompiled is a blocked row;
NHL 07 PS3 supplemental. 28 rows.

### O6 / L1-support-LOC — localization reader (implemented + checked)

* New crate `tdb-loc`: bounded reader for the single-table Huffman-pool dictionary (`verify` of
  the four CRC scopes, `LocDictionary::parse`, `loc_hash`). Ported from the user's NHL 21 handoff
  (`reader.mjs`/`crc.mjs`, read only, originals untouched) plus the repo's TDB core.
* PS4 fixtures agree with the reference reader (row counts 24 251 / 27 468 / 30 799 / 33 396 /
  37 734; all CRCs; `NHL`→"NHL®", `TXT_ROW_LEAGUE`→"Rest Of World"; `AHL`/`HUT`/`CUSTOM_TEAMS`
  absent). **Big-endian pass:** all 20 PS3/Xbox 360 dictionaries decode with the same layout
  (integers in file byte order, 2-byte text-length prefix still big-endian): zero invalid UTF-8,
  every stored hash equals `loc_hash(key)`, keys distinct — recorded as measured, with the caveat
  that no independent reference reader exists for them. Corrupt/truncated/out-of-range inputs fail
  with errors.
* Integration: `AttachedDictionary` (CRC-verified, identified by path/locale/SHA) attached per
  workspace — automatically from the install's own discovered dictionary on onboarding, or
  explicitly via `attach_dictionary` — resolves PS4 `stockleagues` keys to display text; keys absent
  from the dictionary stay keys with the note that the game's own fallback is unverified; gen7
  catalogs never consult a dictionary (their code→key binding is not in the database); nothing is
  shared across projects (leak test). The Leagues page shows which dictionary labelled the names or
  offers to attach one.

### O7 / L1-support-EB — PS4 20-byte-row archive replacement (implemented + checked)

* Measured on the installed NHL 21 (`H:\Emulators\ShadPS4\Games\CUSA18182\databigs`, read only):
  `+0x10` = djb2 of the full path on every row of `boot.big` (7 365), `overlays_21.big` (257) and
  `inxp_00.big` (2 950); `+0x04` = 0; `+0x0C` is **path-derived but of unknown formula** (rows sorted
  by it; siblings differing only in a trailing character share it; content-identical deduplicated
  payloads at different paths do not; no simple hash family matched). Recorded in
  [ps4-eb-big.md](../formats/ps4-eb-big.md).
* `tdb-archive-big::stage` generalised from a fixed 16-byte row to `planned_row_len(version)`
  (16 for `0x0000_0400`, 20 for `0x0011_0400`; anything else refused). The `+16` word is the
  path-hash fact on 20-byte rows; the `+12` word is an **unresolved** qualification item preserved
  verbatim on same-path replacement, so a staged PS4 archive is never deployable-qualified by
  structure alone. Synthetic cases: bind, fit-in-slot, append-and-retarget, unchanged rows and
  digests, verify; real-archive cases (env-gated `#[ignore]`): bind facts on three real archives and
  a same-length in-place replacement of the raw `nhl_game.db` member (entry 2652, 11 209 672 bytes
  = the anchor's length) in a staged copy of `inxp_00.big` with all 2 949 other rows/digests
  unchanged and re-verification (output removed afterwards).
* API: `copied_target_review_archive_member(project, root, archive_relative, member_path)` stages
  the project's packaged primary database into an existing **raw** member of a copied archive in the
  same transaction (refuses missing, deduplicated or compressed members, paths outside the root);
  the panel offers it for PS4 projects with the measured defaults. Checkpoint: NHL 21 league edit
  applied to both the loose file and the archive member of a PS4-shape copied archive, other member
  untouched, exact restore of both.

### O8 — consolidation (this receipt)

Regressions fixed along the way: none outstanding. Two clippy warnings in new code fixed (boxed
enum variant, type alias). Interim edit scripts and probes removed (section 6).

## 4. Checks actually executed (developer, this host; none are independent review)

All with `--target-dir local/runs/league-overnight-2026-09-17/target`.

| Command | Result |
|---|---|
| `cargo test -p tdb-enums -p tdb-loc -p tdb-archive-big -p studio-domain` (full) | 48 `test result: ok` lines, 0 failures (includes the anchor-wide matrices; `studio-domain` incl. new `tests/leagues.rs` 6, `tests/pending.rs` 3; `tdb-archive-big` 53 lib + 1 ignored; `tdb-loc` 2 + 4) |
| `cargo test -p studio-api` (full) | 10 `test result: ok` lines, 0 failures (drift gate, copied_target 15, leagues 3, league_copied_target 3, league_offline_campaign 1, registry, roster_builder, bindings) |
| `NHL_STUDIO_PS4_BIGS=… NHL_STUDIO_PS4_OUT=… cargo test -p tdb-archive-big --test ps4_real_archives -- --ignored --nocapture` | 2 passed (real `boot.big`, `overlays_21.big`, `inxp_00.big`; 167 s) |
| `cargo run -p studio-api --bin export-bindings -- --check` | "Bindings are in sync." (80 roots) |
| `cargo clippy -p tdb-loc -p tdb-archive-big -p studio-domain -p studio-api --tests` | no warnings in the changed crates after the two fixes |
| `rustfmt --check` on the 10 new Rust files | clean (they were formatted in place); touched pre-existing files were edited surgically and not reformatted (repo-wide `cargo fmt --check` is a recorded pre-existing failure, C0.2) |
| `cargo check -p nhl-studio` (Tauri shell) | compiles with the 7 new commands |
| `npx tsc --noEmit -p app` / `npx vitest run` / `npx vite build` | clean / 33 passed / built |
| Anchor integrity | `git status anchors` empty; NHL 21/15/Legacy/14 main DBs and NHL 20/Legacy dictionaries re-hashed unchanged |

**UI evidence vs API evidence.** Every league/pending/copied-target behaviour above is proven at
the Registry/API seam (DTO-level). The Leagues, League detail, Pending Edits and Copied-target
pages are **type-checked and bundled only**; no desktop or browser session was run, so no
screenshot exists and no UI interaction is claimed. Real UI automation was not available in this
environment (the app is a Tauri desktop shell; no driver is configured).

**Untested claims / not run:** no game or emulator; no live installation; no archive placed into
a real install; NHL 21's own dictionary (missing fixture); loose-file vs archive precedence on
PS4; the `+0x0C` PS4 row word's formula; whether the game falls back to a literal name key.

## 5. Remaining blockers and limitations (per target)

* **All targets — runtime:** nothing is game-verified. PS4 in-game persistence now has a staged
  path (edited `nhl_game.db` in a copied `inxp_00.big`) but its acceptance by the game and archive
  precedence are unproven; the staged archive is unqualified by construction.
* **NHL 21 / NHL 19 PS4:** no dictionary fixture — `TXT_ROW_LEAGUE`'s presence and every label are
  unverified for these titles; labels show as keys unless the user attaches the install's own
  `eng_us.db`.
* **NHL 19 / NHL 20 PS4:** identical anchor bytes, provenance unresolved — candidate rows.
* **PS3 / Xbox 360 (NHL 07–15, Legacy):** no league name in the main database; the Leagues page
  browses/classifies with per-title vocabularies (candidate or corroborated) and states the blocker.
  Their dictionaries now decode, but the code → dictionary-key binding lives outside the database
  (UI data/executable) and remains open — Phase 1.3.
* **Legacy Recompiled:** no fixture; untested on every row.
* **Group ids** (`leaguegroup`/`conferencegroup`/`divisiongroup`) are named nowhere in the
  database; only Legacy-layout NHL conference/division labels are shown (candidate).

## 6. Temporary artifacts, processes and cleanup

Package scratch root: `E:\Repositories\nhl-modding-studio\local\runs\league-overnight-2026-09-17\`
(untracked, outside `anchors/`).

| Item | Purpose | Disposition |
|---|---|---|
| `target/` (≈14 GB) | build/test cache for every check above | **retained for the Codex review of this receipt** (rerunning the suites needs it; ≈30 min to rebuild); owner: this run; remove after review |
| `edit_o1_wiring.py`, `edit_o1_tests.py`, `edit_o2_api.py`, `edit_o3_wiring.py`, `edit_o3_api.py`, `edit_o4_api.py`, `edit_o6_wiring.py`, `edit_o7_stage.py`, `edit_o7_api.py` | one-shot source edit scripts | **removed** (their effect is the source diff) |
| `eb20_probe.py`, `eb20_probe_a.py`, `loc_probe.py`, `__pycache__/` | read-only measurement probes (PS4 row words, dictionary key grep) | **removed**; findings are in `docs/formats/ps4-eb-big.md`, `tdb-loc` and the tests |
| `campaign-1.log`, `studio-api-tests-1.log`, `final-domain-tests.log`, `final-api-tests.log`, `changed-files.txt`, `changed-hashes.txt` | test transcripts and the hash inventory | **removed** after transcription into this receipt and the campaign record |
| `ps4-out/` | staged real-archive output (777 MB, deleted by the test itself) | **removed** (was empty) |
| `H:\TEMP\…\scratchpad\l11a\` (L1.1-A scripts, adopted for O1) | mapping verification | **removed** after O1 (its mapping became `tests/leagues.rs`) |
| Test temp roots `%TEMP%\nhlms-league-*`, `nhlms-pending-*`, engine staging under `%TEMP%\nhl-modding-studio\copied-target-staging\` | per-test copied roots and staging | removed by each test's `Drop`/settle; staging root checked empty at closeout |
| Processes | `cargo`, `python`, `node`/`vite` ran to completion in the foreground/background; no app process was launched | none running |
| `%TEMP%
hlms-p3-op-24084` | observed at closeout; not created by this run's tests (its prefix belongs to a pre-existing operation-matrix case, unknown process) | **not claimed, not deleted** — reported for Codex |

No `git clean`/`reset`; no shared `local`/`target` tree deleted; installed games read only.

## 7. Best next task for Codex review

Review this receipt and the campaign record; independently rerun
`cargo test -p studio-domain --test leagues --test pending` and
`cargo test -p studio-api --test leagues --test league_copied_target --test league_offline_campaign -- --nocapture`
(and, with the two env vars, the ignored `ps4_real_archives` cases). The natural follow-ups, in
order: (a) a desktop smoke run of the Leagues / Pending / Copied-target pages (the only unexercised
layer), (b) the NHL 21 in-game test of the copied `inxp_00.big` (loose vs archive precedence is
the open runtime question), (c) Phase 1.3's gen7 code → dictionary-key consumer trace now that the
BE dictionaries decode. Stopping here; no further package started.

# P1-C — PS4 competition-group integration (NHL 15–21)

Developer record, 18 September 2026. Package dispatched by
[p1a-r2-acceptance-and-p1c-dispatch.md](p1a-r2-acceptance-and-p1c-dispatch.md); consumes research
handoff 17 ([17-ps4-groups-and-consumer-leads.md](../research/overnight-2026-09-18/17-ps4-groups-and-consumer-leads.md),
`data/ps4-competition-groups.json`) with its 2026-09-18 provenance corrections. **Corrected by
P1-C-R1 (18 September 2026, section 8 below)** after the [P1-C review](p1c-review-and-r1-dispatch.md):
sections 1-7 are the original P1-C record and stand where section 8 does not override them. Codex
review of R1 pending; nothing here is game-verified.

## Plain-language summary

On the PS4 titles (NHL 15 through NHL 21) Studio now names conferences, divisions and tournament
pools in the game's own words instead of showing bare numbers, and the "move a team" card offers
only the groups that league really uses on that title — the same editor the Xbox 360 / PS3 titles
already had. Where the research could not establish a name (two groups on NHL 19–21) or could only
infer the mapping from how the database uses the ids (part of NHL 17, the Spengler Cup on NHL 18),
the screens say so and never dress a guess up as a name. What the game does on screen with a moved
team is still a question for the user's runtime cards; nothing was deployed or launched.

## 1. Package, disposition, scope

- **Package:** P1-C. **Claimed disposition:** delivered, developer-verified offline; Codex review
  pending.
- **Scope done:** per-title PS4 catalogues; provenance-bound selection; per-row evidence; label
  resolution through the title's dictionary with disclosed fallbacks; membership integration
  through the existing global-groups editor (pending / undo / redo / save-to-copy / reopen /
  restore); league structure and membership cards; bindings; tests on all seven PS4 anchors, the
  real NHL 20 install database and the NHL 19 base + `patch_inxp_56` databases; gen7 regressions;
  matrix and user cards.
- **Not done / out of scope (per dispatch):** no live deployment, image write, game launch,
  quarantine, staging, commit or push; no P1-D / Phase 2; no schedule rewriting inferred from a
  group change; the NHL 17 id mechanism and the names of ids 77/78 stay research questions.

## 2. Changed paths (baseline `8d60a6d` on `feat/phase5-asset-tooling`, dirty tree preserved)

| Path | Change | SHA-256 (16) |
|---|---|---|
| `defs/leagues/groups/ps4-nhl15.toml` … `ps4-nhl21.toml` (7 new) | per-title PS4 group tables baked from research 17 | `d09ab1c917d94fce` `90813e3039a2d9f0` `e18885267df0a9c6` `bcb70b22b0ac4ff9` `82ee5e1168e84de2` `ce98f932fd171d71` `f9c838413cac1a66` |
| `defs/leagues/README.md` | PS4 group tables, selection binding, evidence vocabulary | `90364e26984374dc` |
| `crates/studio-domain/src/competition_groups.rs` | provenance-bound selection, `FitReport`, per-row evidence, multi-league blocks, pools, `LeagueShape`, own-id arrangement | `a27054a6818b5031` |
| `crates/studio-domain/src/leagues.rs` | `group_table_for`, `attach_competition` (fit check + `competition_blocker`), `LeagueStructure.{pools,shared_with,witness_notes}`, `LeagueView.structure_blocker`, `LeagueSource: Copy` | `357fe4a09da92601` |
| `crates/studio-domain/src/membership.rs` | PS4 with a fitting table → shared `GlobalGroups` path; `Choice.evidence`; `LeagueGroups.league_group_evidence`; pool / own-id choices; disclosure notes; raw `Ps4` model only without a fitting table | `cf942652e3ae42f1` |
| `crates/studio-domain/src/session.rs` | `structure_blocker` on the league view | `e660bbeb5fe4b93e` |
| `crates/studio-domain/tests/ps4_groups.rs` (new) | 8 cases (1 `--ignored`, private fixtures) | `ccdc77f494a04b06` |
| `crates/studio-domain/tests/membership.rs` | PS4 case rewritten for the table-backed path | `cf89502b1a442b01` |
| `crates/studio-domain/tests/leagues.rs` | the "no PS4 group labels" assertion inverted | `dd29de6c4e0aed4d` |
| `crates/studio-api/src/dto.rs`, `convert.rs` | `CompetitionGroupDto.{league_codes,evidence,note,hypothesis_key}`, `LeagueStructureDto.{pools,shared_with,witness_notes}`, `LeagueViewDto.structure_blocker`, `MembershipChoiceDto.evidence`, `LeagueGroupsDto.league_group_evidence` | `3720fe848bd7df8a` `ac46e8337484382c` |
| `crates/studio-api/tests/membership.rs` | registry-seam PS4 case (NHL 18) | `91c0935e99645073` |
| `app/src/bindings/{CompetitionGroupDto,LeagueGroupsDto,LeagueStructureDto,LeagueViewDto,MembershipChoiceDto}.ts` | regenerated (`export-bindings --write`; `--check` in sync) | `86cea4c338a27eae` `f19570aa8e61b821` `f8ea2819c9b39910` `327dfe2fc26b4a0d` `3e8cb8c34b7b8252` |
| `app/src/components/LeagueMembershipCard.tsx` | evidence pills, inferred/unknown choice text, target-evidence note, unknown single-group note | `14e22ab55d52fb0e` |
| `app/src/components/LeagueMembershipCard.test.tsx` (new) | 4 jsdom cases | `1e83c6d9c9191b2d` |
| `app/src/pages/LeagueDetail.tsx` | structure card: pools, block sharers, per-row evidence, hypothesis disclosure, witness notes, blocker card | `025f611e9b6eaea5` |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | §1.2 PS4 cells, P1-C package row | — |
| `docs/receipts/overnight-development-2026-09-18/MORNING-REVIEW.md` | P1-C user review cards (PS4 15–21) | — |

The pre-existing dirty tree (asset registry, canonical adapters, other overnight work) is
untouched.

## 3. Behaviour and decisions

### 3.1 Catalogues (`defs/leagues/groups/ps4-<game>.toml`)

Baked by a package-owned script from `data/ps4-competition-groups.json`, one file per title:
55 / 54 / 61 / 77 / 79 / 79 / 79 rows (NHL 15 … 21). Each file carries the witness (CUSA id,
`eboot.bin` / base `nhl_game.db` / `eng_us.db` SHA-256, `param.sfo` versions, provenance notes),
the three team-row field ids (`iIBl` / `OwTu` / `CGMK`), and per row: `key` (empty when unknown),
`kind`, `leagues` (every league code whose rows carry the block), `used`, `evidence`, `note`,
`hypothesis_key`, `logo_index` (NHL 15/16 art id only).

`kind` / `leagues` are derived from each title's own witnessed team-table usage (the JSON's
`team_table_groups_by_league`), not from the neighbouring title: an id used as `leaguegroup` heads
a block; a used sub-group belongs to the head the **majority** of its rows use (so the three NHL 17
custom rows that carry the ECHL Western Conference do not make the custom league an ECHL sharer);
an unused sub-group joins a used sibling of its key family (Group B–H after Group A) or the
preceding head when that head demonstrably has sub-groups (tournament blocks); otherwise it has no
parent (NHL 18+ `TXT_PLACEMENT_POOL` / `TXT_QUALIFYING_POOL`, which follow Allsvenskan in the
array). Ids carried in both the conference and the division slot are `pool`s (Pool A/B, Group
A–H, Cattini/Torriani); a `*Conference` id carried in a division slot stays a conference (the
All-Star tolerance the gen7 tables already have).

Evidence per row, verbatim from the research classes: **traced-static** (array order == usage),
**inferred** (NHL 17 ids 43–60 — the executable stores the ECHL block before the tournament block,
so array position is not the runtime id there; NHL 18 ids 74–76 — absent from its array, keys from
NHL 19–21's order), **unknown** (NHL 19–21 ids 77 and 78 — no static string run names them; the
research hypothesis `TXT_LEAGUE_NHL_ALUMNI` / `TXT_LEAGUE_PROAM` is carried as `hypothesis_key`).

### 3.2 Selection bound to provenance

`LeagueVocabularies::group_table_for(game, source, platform)`: a table applies only when its
`applies_to` names the game id, its `source` matches the league source the open database presents
(`stockleagues` for PS4 tables; derived codes for the gen7 tables, which keep applying exactly as
before), and — when the platform is known — its `platforms` list allows it. The PS4 and Xbox/PS3
NHL 15 tables share a game id and are told apart by both.

After selection the table is checked against the open database's team rows (`FitReport`): rows
whose `leaguegroup` is that league's group in the table (fit) must outnumber rows naming another
league's group (conflict); the report also counts ids the table lacks and league codes without a
group. A refused table leaves `competition = None` with `competition_blocker` naming the numbers
("the nhl15 group table does not describe this database: 66 of 402 team rows …; the database was
opened as nhl15 …"), and the membership card / league page fall back to raw ids with that reason.
Without a game id, or on a schema without the group fields, the reason says that instead. All
seven anchors, the real NHL 20 database and the NHL 19 base and patch databases fit with zero
conflicts and zero unknown ids.

Evidence class per target stays as before: `traced` on the witnessed (game, platform) — every PS4
title is its own witness — `by-reference` otherwise (a PS4 table selected without a platform).

### 3.3 Labels

Dictionary text for the key when the title's own dictionary is attached (exact or by hash),
otherwise the observed English research text, otherwise the key itself, each with `label_source`.
NHL 18's dictionary has no text for `TXT_GROUP_CATTINI` / `TXT_GROUP_TORRIANI`: shown as the key.
An unknown group is "Unnamed group 77" with `label_source = "unknown"`; the hypothesis is a
separate field and the UI prints it as "research hypothesis (not a name)". No sibling-title label
is ever promoted.

### 3.4 Membership (one editor, no second one)

`membership::model_for` already picked `GlobalGroups` whenever the catalog carries a competition
table, so a PS4 database with a fitting table takes the same path as NHL 12 → Legacy: plan →
`set_cells_int` → one transaction, one undo entry, pending review, roster mirror. Changes to that
shared path:

- **Shapes.** `Single` (own id in all three fields), `Pools` (own id or a pool in both slots:
  Spengler Cup, Champions Hockey League), `Structured { own_id_allowed }` — the own-id arrangement
  is accepted in a structured block only when no row carries any of its conferences/divisions
  (tournament blocks whose Conference A/B and Division A–D exist but are unused; the 21-team
  International league at 43/43/43). On the NHL block it is refused as before. This relaxes gen7
  only for Legacy's Tournament block, whose rows are 38/38/38.
- **Choices** carry `evidence`; pool-only leagues list the league's own id first ("no sub-group").
- **Shared blocks.** `leagues = [...]` lets several `stockleagues` rows share one block (Custom
  Teams / Prospect / EASHL / Mascot / 3-on-3 on 53; Alumni and All-Time on 77; NHL 15/16's second
  NHL row on the NHL block). `LeagueStructure.shared_with` discloses the sharers.
- **Unknown target.** A move into Alumni / All-Time / Pro Am writes 77 / 78 — the id every row of
  those leagues already carries — and the plan notes say the name is unknown.
- **Refusals** (bytes, history, revision unchanged, asserted): a group of another league's block;
  a non-pool under a pool-only league; the own id under a league whose rows use conferences; a
  league with no established block (no member rows: CHL, USA, Russia, ROW, HUT, the empty Custom
  Teams rows) — "has no group in this title's competition table"; a dangling league code.
- **Raw model** (`GroupModel::Ps4`) remains only for a PS4 database no table describes; its notes
  carry the blocker.

### 3.5 Surfaces

League page: pools column, block sharers, per-row *inferred* / *name unknown* pills, hypothesis
line, witness notes; a "Competition structure" card with the blocker when no table applies. Team
page membership card: evidence pills on the current values, "(inferred)" / "(name unknown)" in the
option text, a target-evidence line, the unknown-id note on single-group targets, the notes list
(witness, fit summary, unknown ids, inferred count, leagues without an established block).
Rename preview (Phase 1.3) now names PS4 group consumers of a key through the same table.

### 3.6 Requested corrections — none outstanding from the dispatch; each numbered task above maps:
1 catalogues + integration (3.1, 3.4); 2 provenance (3.2, 3.3); 3 unknown/inferred (3.1, 3.3, 3.4);
4 validated choices / read-only reasons (3.4); 5 verification (§4); 6 matrix + cards (§2).

## 4. Checks executed (developer results; no prior independent review of this package)

| Command | Result |
|---|---|
| `cargo test -p studio-domain --test ps4_groups` | **7 passed, 1 ignored** (private fixtures) |
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test ps4_groups -- --ignored` | **1 passed** — real NHL 20 (`1e200478…`, dictionary `0fc95ec8…`: fit, "Group Cattini" from the dictionary, Pro Am 80 teams on 78, a move to AHL North and undo) and NHL 19 base (`265e4a68…`, Pro Am 80) + `patch_inxp_56` (`61332e66…`, Pro Am 84; same ids) |
| `cargo test -p studio-domain --test membership --test leagues` | **4 + 11 passed** (gen7 Legacy / NHL 11 / NHL 07 cases unchanged; the PS4 case rewritten; one Legacy assertion inverted) |
| `cargo test -p studio-api --test membership` | **4 passed** (registry seam: NHL 18 evidence DTOs, pools, sharers, rename disclosure "5 competition groups", move / pending 4 cells / save to copy / reopen / undo / byte-identical restore / wrong-title fallback) |
| `cargo run -p studio-api --bin export-bindings -- --write` then `--check` | 5 files written; "Bindings are in sync." |
| `cargo test -p studio-domain` (full crate) | see §4.1 |
| `cargo test -p studio-api` (full crate, incl. the bindings drift test) | see §4.1 |
| `cd app && npx tsc --noEmit` | clean |
| `cd app && npx vitest run` | **12 files, 92 passed** (4 new in `LeagueMembershipCard.test.tsx`) |

Fixture identities: the seven committed PS4 anchors (`anchors/ps4/nhl{15..21}/databases`;
NHL 19 and 21 carry no `eng_us.db`, the nhl20 anchor's is the NHL 19 dictionary `62260005…`), the
Xbox 360 NHL 15 / Legacy / NHL 11 anchors for the gen7 regressions, and the private fixtures
above. Temp copies: `%TEMP%\nhl-studio-p1c-<pid>` and `nhl-studio-p1c-api-<pid>`, removed by the
tests.

Untested claims: the game's runtime reading of any PS4 group id (unchanged from research 17: no
consumer tracing); the NHL 15 patch database's group usage (not measured); NHL 16/17 art ids
(`logo_index`) are carried but consumed by nothing new.

### 4.1 Full-crate runs

- `cargo test -p studio-domain`: **342 passed, 0 failed, 9 ignored** across 44 test binaries
  (`ps4_groups` 7/1, `membership` 4, `leagues` 11, `schedule_edit` 12/1 — the 26-target
  import/generation parity case, which moves a team through `set_team_membership`, still passes).
  The 45th binary, `zz_diag`, is a pre-existing **untracked scratch diagnostic** (not part of this
  package) that iterates every anchor; it had consumed ~53 CPU-minutes without finishing and was
  stopped so the run could close. It is not a regression signal and was not modified.
- `cargo test -p studio-api`: **91 passed, 0 failed, 4 ignored** (includes
  `bindings::tests::live_bindings_match_the_dto_contract`).
- `cargo clippy -p studio-domain -p studio-api --tests`: no warnings in the files this package
  touched (two `unnecessary_to_owned` hints in the new test were fixed); the repo-wide baseline
  defects (C0.2) remain and were not repaired.

## 5. Remaining blockers and limitations

- Ids 77 / 78 (NHL 19–21) remain unnamed; the runtime-append hypothesis is research-only.
- NHL 17: the mechanism producing the runtime ids for 43–60 is unknown; rows are inferred from
  usage and marked.
- Leagues with no member rows have no established block and cannot be a move target (refused with
  the reason) — intentionally not "repaired" from sibling titles.
- The NHL 17 custom league's three rows carrying the ECHL Western Conference browse with that
  label; a move into that league offers only its own block.
- Placement pool / qualifying pool (NHL 18+) have no established parent league.
- Whether standings / schedules / mode initialization follow a moved team is a runtime question
  (user cards); no schedule rewriting is inferred from a group change.

## 6. Temporary artifacts and cleanup

| Path | Purpose | Owner | Disposition |
|---|---|---|---|
| `H:\TEMP\claude\e--Repositories-nhl-modding-studio\9774228d-315f-4c14-867e-e652ad21c147\scratchpad\p1c\bake_ps4_groups.py` | the catalogue bake script (reproduces the seven TOML files from the research JSON) | P1-C | **retained** until Codex review (reproducibility of the baked tables); then either promote to `scripts/` on request or delete |
| `…\scratchpad\p1c\patch_*.py` (8 files), `domain-tests.log` | scripted source transformations, test log | P1-C | obsolete after review; delete at closeout |
| `%TEMP%\nhl-studio-p1c-<pid>`, `%TEMP%\nhl-studio-p1c-api-<pid>` | test copies of the NHL 21 / NHL 18 anchors | tests | removed by the tests on success; four directories left by earlier failing iterations (`103460`, `16536`, `86220`, `api-108984`, each holding only the anchor copy) were resolved by literal path and removed at closeout; none remain |
| `target\debug` | shared build cache reused | shared | retained (not package-owned) |

No processes left running (the stopped `zz_diag` binary was this session's own test run); no
anchors, live installs, images or private fixtures modified (fixture hashes asserted by the ignored
case). **Not package-owned, reported only:** `%TEMP%
hl-studio-p1a-26528` (`nhl_game.db`) and
`%TEMP%
hl-studio-p1a-api-35316` (`nhlschedule.json`), created 13:51 / 14:02 on 18 Sep by the
earlier P1-A session's test iterations; left in place for the coordinator's closeout decision.

## 7. Next review action

Codex: review this record and the receipt; verify the seven catalogues against the research JSON
(`bake_ps4_groups.py` reproduces them byte-for-byte), rerun `ps4_groups` (plus `--ignored` with
`NHL_STUDIO_FIXTURES`), `membership`, `leagues` and the app suite; decide whether the bake script
is promoted or the scratch is deleted. Nothing is staged or committed.

## 8. P1-C-R1 correction (18 September 2026)

### 8.0 Plain-language summary

Three review findings are fixed. (1) A team can now only be given a conference-and-division
combination that the title's own stock rows actually use — Western with Pacific or Central, not
Western with Atlantic — and the special All-Star rows that carry a conference in the division slot
stay as they are but are never offered to an ordinary club. (2) Which title a database belongs to
is decided by its recorded identity or by its structure (which no edit changes), not by how many
rows happen to agree: the NHL 21 database opened under any other PS4 title's name is refused and
told what it is, while an edited or patched file of the right title keeps working. (3) The
membership card no longer pushes an old project's summary into the app after you switch projects.
The seven PS4 catalogues (and the witnessed structure of the five Xbox/PS3 ones) are now produced
by a repository tool with a drift check, and the notes name the keys each database really uses.

### 8.1 Disposition of the findings

| Finding | Disposition |
|---|---|
| **F1 — parent combinations** | Fixed. Each table now carries the witnessed `[[pairs]]` per league block (derived from the witness database by `competition_groups::witness_pairs`, shared by the tool and the runtime). `validate_membership` accepts only a *regular* witnessed (conference, division) pair of the target league's block; a conference id in the division slot, a group of another block, a cross-pool triple or a pair no row carries is refused with the witnessed alternatives named; `exceptional` pairs (the All-Star squads' `(West, West)` / `(East, East)` on NHL 12–15/Legacy and PS4 15–17 — team ids 31 / 30; NHL 17's three custom rows carrying ECHL West) are recorded with their rows, browse unchanged and are refused by name. The reproduced NHL 21 case `(BOS, 0, 6, 10)` is asserted refused with bytes/history/revision unchanged (`nhl21_membership_offers_only_witnessed_pairs_and_survives_save_reopen_restore`); the UI filters divisions by the chosen conference through `LeagueGroupsDto.pairs` and discloses `exceptional`. Own-id arrangements are no longer a heuristic: `(head, head)` is offered only where rows carry it (International 43/43/43); a division-only league (QMJHL: own id in the conference slot + one of three divisions on every title) is a regular witnessed shape. **gen7 audit:** the same model now applies to NHL 12–15/Legacy from their Xbox anchors — the previous "any conference id in the division slot" tolerance and the unwitnessed Tournament `ConferenceA/DivisionA` choices are gone; `leagues.rs` / `membership.rs` assertions were corrected accordingly (Western + Atlantic, Western + East and the All-Star pair are refused on Legacy). No PS4 assumption was substituted: every gen7 pair set is measured on that title's own anchor. A side finding: the original P1-C tests and receipt used AHL "West / Atlantic" as their example — on NHL 18–21 the AHL's Western Conference pairs with Central/Pacific and its Eastern with Atlantic/North; that example was itself an invalid combination and is now asserted refused. |
| **F2 — provenance** | Fixed. `FitReport::accepted` is gone; row agreement is reported as compatibility only. Each table carries a `[signature]` (table count, SHA-256 of the sorted table-id set, team and stockleagues `max_records` — measured on every anchor and the real NHL 20: all twelve differ, including NHL 19 vs real NHL 20 (111 tables/519/29 vs 114/517/31) and NHL 19 base vs its `patch_inxp_56` (same signature, different identity)) and `[[known_databases]]` (base witness, NHL 19 patch, and the nhl20 anchor as `excluded`). `attach_competition` verifies a recorded identity first (`Provenance::Recorded`), otherwise requires the signature to match the claimed title and no other baked title of the same source (`Structural`; `Ambiguous` demotes evidence to by-reference — no such case exists among the twelve tables), and refuses everything else naming what the structure identifies as. The reproduced NHL 21-as-16/17/18/19 cases, every anchor opened as NHL 21, NHL 12 opened as NHL 15, the nhl20 anchor opened as nhl20 (refused as the recorded NHL 19 base copy), an edited NHL 21 file (structural, still traced) and the NHL 19 patch (recorded `patch`) are asserted in `cross_title_claims_are_refused_by_structure_not_by_row_agreement`, `an_edited_database_keeps_its_title_by_structure_…` and the `--ignored` fixture case. A table without a signature (none remains) would fall back to the old row gate with demoted evidence. |
| **F3 — summary ownership** | Fixed in `LeagueMembershipCard.tsx`: a `ctxRef` mirrors the current `{projectId, teamId}`; after the awaited `project_summary` the global `update` runs only if that project is still open, the result only in the same context and generation, and a late summary error is shown only there. Four deferred jsdom cases added (same-context completion; project+team switch → no update, no result, no notice; team-only switch on the same project → summary applied, result hidden; late error shown only in its context and never globally). |
| **Bake script** | Promoted to `crates/studio-api/src/bin/competition_groups.rs` (`cargo run -p studio-api --bin competition-groups -- --check` / `--write`, optional `--fixtures <root>` or `NHL_STUDIO_FIXTURES`): documented inputs/outputs, deterministic LF/UTF-8 output, `--check` refuses drift without writing (exit 1), `--write` renders every file first and replaces each through a temporary sibling + rename; gen7 files are transformed only below a marker line so the maintained P1.2-X metadata is never rewritten. Inference (majority head, sibling family) is stated in the generated header, separately from the research key-lookup evidence. The Python script is deleted. |
| **Source notes** | The research JSON's "22 = TXT_LEAGUE_NHL_ALL_TIME" is NHL 21's key only: NHL 19 base, its patch and the real NHL 20 key league 22 `TXT_LEAGUE_NHL_ALUMNI`. The unknown-group notes now append the keys the witness database actually uses ("… stockleagues keys of the leagues carrying it on this witness database: 21 = …, 22 = …"), asserted per title; block-sharing notes are generated from each database. NHL 15's patch database: no fixture is held — explicitly untested (witness note). NHL 19 base/patch difference recorded (Pro Am 80 → 84, same ids, same signature, distinct recorded identities). |

### 8.2 Changed paths (R1; hashes SHA-256/16)

`defs/leagues/groups/*.toml` (12, regenerated: gen7 files gain the witnessed-structure section) `f9a229b34b25743c` `1ede464dad6ad569` `9b050951c5188e93` `4b27512be827a447` `305df3ca107a82da` `cc08709258db1dac` `b4e55084e1c3a38f` `a4ee0938b99b1637` `396bb1e2d9a27cb4` `07bf7115c5595ece` `c3d1005ca8d49fa6` `819c2200d2cb285a`; `defs/leagues/README.md` `b1d5cf514289a35a`; `crates/tdb-core/src/lib.rs` (`TdbFile::bytes` accessor) `9b5a185ad1ad576b`; `crates/studio-domain/src/competition_groups.rs` `2eac1805335f0d50`, `leagues.rs` `4e13cd1dbd130f2a`, `membership.rs` `acb1e7397453cd65`, `Cargo.toml` (sha2) `b1b31d5cf765bc1e`; `crates/studio-domain/tests/ps4_groups.rs` (rewritten, 10 cases) `1bc917d6350b22d9`, `membership.rs` `24be47cff7ff1fad`, `leagues.rs` `26f66fe4dbd0552b`; `crates/studio-api/Cargo.toml` (bin, tdb-defs, sha2) `5315dc5c737dbdb6`, `src/bin/competition_groups.rs` (new) `4e7d8bf0e8561e01`, `src/dto.rs` `c5c8d236171105d4`, `src/convert.rs` `1449bde19da3200d`, `tests/membership.rs` `56fda27d8e2ce52c`; `app/src/bindings/{LeagueGroupsDto,LeagueStructureDto}.ts` `392229135a397830` `bee3725f1e16fff2`; `app/src/components/LeagueMembershipCard.tsx` `1660caed7e5fe2e9`, `.test.tsx` `f5504e123f4aafa9`; `app/src/pages/LeagueDetail.tsx` `a4a62e9b65924a5a`; this record; the coverage matrix and MORNING-REVIEW cards.

### 8.3 Checks executed (developer results; the independent review's own runs are in the dispatch)

| Command | Result |
|---|---|
| `cargo run -p studio-api --bin competition-groups -- --write --fixtures E:/nhl-studio-fixtures` then `--check` (with and without `--fixtures`) | 12 written; "in sync (12 rendered, 0 skipped)"; without fixtures "in sync (11 rendered, 1 skipped)" — nhl20 reported skipped, never as verified |
| `cargo test -p studio-domain --test ps4_groups` | **9 passed, 1 ignored** |
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test ps4_groups -- --ignored` | **1 passed** (real NHL 20 recorded base; refused as nhl19 by structure; NHL 19 patch recorded `patch`; base recorded `base`) |
| `cargo test -p studio-domain --test membership --test leagues` | **4 + 11 passed** (gen7 assertions corrected to the pair model) |
| `cargo test -p studio-api --test membership` | **4 passed** (seam: pairs/exceptional/provenance DTOs; West + North refused on NHL 18, East + North written; Legacy West + Atlantic refused) |
| `cargo test -p studio-api` (full, incl. bindings drift test) | **91 passed, 0 failed, 4 ignored** |
| `cargo test -p studio-domain --lib --test <every target except zz_diag>` | see 8.3.1 |
| `cargo run -p studio-api --bin export-bindings -- --write` / `--check` | 2 files rewritten; "Bindings are in sync." |
| `cd app && npx tsc --noEmit`; `npx vitest run` | clean; **12 files, 97 passed** (9 in `LeagueMembershipCard.test.tsx`) |
| `cargo clippy -p studio-domain -p studio-api --tests --bins` | no warnings in the files this package touched; repo-wide baseline defects untouched |

Fixture identities as in §4 plus the NHL 19 patch DB `61332e666a6b4afd…` and meta `c67f4bfd3d152bfb…`.

#### 8.3.1 Full studio-domain run

**344 passed, 0 failed, 9 ignored across 44 binaries** (lib + every integration target except zz_diag; `ps4_groups` 9/1, `membership` 4, `leagues` 11, `schedule_edit` 12/1 — the 26-target parity case moves a team through `set_team_membership` and still passes). `zz_diag` (a pre-existing untracked scratch diagnostic, not part of
this package) was excluded explicitly by listing every other test target; this is not a full clean
domain run and is accounted as such.

### 8.4 Disk-space deviation

The E: volume reached 24 KB free mid-package (the shared `target\debug` tree measured 156 GB: 55 GB
`incremental`, 95 GB `deps`), and no build could link. The single exact path
`E:\Repositories\nhl-modding-studio\target\debug\incremental` (a regenerable compiler cache; verified
not a reparse point, no cargo/rustc running) was removed by literal path; nothing else in the
shared tree was touched. 44 GB free afterwards. Recorded here because the shared cache is not
package-owned.

### 8.5 Scratch and cleanup (R1)

Package scratch root `H:\TEMP\claude\e--Repositories-nhl-modding-studio\9774228d-315f-4c14-867e-e652ad21c147\scratchpad\p1c\`:
the Python bake script and every `patch_*.py` / `r1_*.py` / `domain-tests*.log` / `r1_section.md`
are obsolete now that the tool is in the repository — deleted at closeout by literal path after
confirming containment in that root (8.6). Test temp dirs `%TEMP%\nhl-studio-p1c-<pid>` are removed
by the passing tests (none remain). The P1-A directories `%TEMP%\nhl-studio-p1a-26528` /
`nhl-studio-p1a-api-35316` are not package-owned and were not deleted: ownership is the earlier
P1-A session's and no recovery dependency was checked by this package — left for the coordinator.

### 8.6 Cleanup verification

Verified at closeout: the 17 files of the package scratch root were resolved individually (no
reparse points, all inside the root), removed by literal path, and the empty root removed —
`Test-Path` false. Two `%TEMP%
hl-studio-p1c-*` directories left by this session's own failing R1
iterations (`516`, `api-37088`, created 22:08 / 22:12 today, each holding only an anchor copy)
were removed the same way; none remain. No cargo/rustc processes remain. The P1-A directories
(`nhl-studio-p1a-26528`, `nhl-studio-p1a-api-35316`) remain untouched. The shared `target\debug`
tree remains (minus the `incremental` cache, 8.4).

### 8.7 Remaining limitations after R1

- Names of ids 77/78 and NHL 17's id mechanism: research questions (unchanged).
- NHL 15 patch database: untested (no fixture).
- Structural identification is as strong as the twelve measured signatures: a future title or
  patch with an unmeasured structure is refused, not guessed; a patch with the base's structure but
  an unrecorded identity is accepted as *structural* (edited-or-unrecorded), which the notes say.
- Groups the executable lists but no stock row carries (NHL 18 Torriani, Tournament Conference
  A/B, Division A–D, Champions Group B–H) are never offered: their parent relationship is
  unresolved, disclosed on the league page as unused rows.
- Runtime effect of any group change remains a user card.

### 8.8 Next review action

Codex: review §8 and the diff; rerun `competition-groups -- --check` (with `--fixtures` for the
NHL 20 file), `ps4_groups` (+ `--ignored`), `membership`, `leagues`, `studio-api membership`, the app
suite. Nothing is staged or committed.

## 9. P1-C-R2 correction (19 September 2026)

### 9.0 Plain-language summary

When a team is moved into a league that has no conferences or divisions of its own (Liiga, Norway,
the Alumni list…), the editor writes the league's own group id into all three group cells. Before
this correction, if a caller supplied the right conference id *and* a wrong division id, the wrong
division was silently thrown away and the move went through as if the caller had asked for the
own-id arrangement. Now each supplied value is checked on its own: any supplied conference or
division that is not the league's own id is refused by name ("the division given (6) is not that
id"), and nothing is written. Leaving a value out still selects the own-id arrangement, as before.
The membership card already omits both values for these leagues, so nothing changes on screen; the
change is to what the backend accepts from any client.

### 9.1 Disposition of the finding

| Finding | Disposition |
|---|---|
| F1 (P2) — `conference.or(division)` validated only the first supplied slot on a single-group league; `set_team_membership(BOS, Liiga, Some(21), Some(6))` on NHL 21 was accepted and written as 21/21/21 | **Fixed.** `plan()` in `crates/studio-domain/src/membership.rs` checks the conference and the division independently before any write is planned; a conflicting value in either slot is a validation refusal naming the slot and the value. Omission still selects the own id. Witnessed-pair and exceptional-row handling (R1) is untouched: the `Structured` branch and `validate_membership` are unchanged. |

### 9.2 Changed paths (R2; hashes SHA-256/16)

`crates/studio-domain/src/membership.rs` (the single-group branch only) `241a92e631d0fe3e`;
`crates/studio-domain/tests/ps4_groups.rs` (+1 case) `208acd7ee2959a1c`;
`crates/studio-domain/tests/membership.rs` (Legacy case extended) `100687c79222a3ed`;
`crates/studio-api/tests/membership.rs` (+1 case) `7d91045d5af8ace7`; this record; the coverage
matrix P1-C row. No other code, binding, catalogue or document changed.

### 9.3 Regressions added (automated-tested)

- `ps4_groups::single_group_league_validates_conference_and_division_independently` — NHL 21
  anchor, Boston → Liiga (own id 21). Refused with bytes, revision, undo depth, redo depth, dirty
  flag and pending-cell count asserted unchanged: (21, 6) the reproduced case; (6, 21); (None, 6);
  (6, None); (6, 8). Accepted, each as four writes carrying 21 three times and undone/redone
  cleanly: (None, None), (21, 21), (21, None), (None, 21). The witnessed NHL pair refusal
  (West + Atlantic) and acceptance (West + Pacific) are asserted alongside.
- `membership::legacy_moves_a_team_with_validated_global_group_ids` (gen7 shared path, Legacy
  Xbox 360, Norway own id 19) — added (19, 3), (1, 19), (None, 3) refusals with bytes, revision,
  undo/redo depth and pending count unchanged; (19, 19), (19, None), (None, 19) accepted.
- `studio-api membership::single_group_slots_are_validated_independently_through_the_registry` —
  Registry seam on NHL 18 PS4 (Liiga) and Legacy Xbox 360 (Norway): the three refusals above with
  `workspace_revision`, dirty/undo/redo and pending count unchanged; the four accepted forms
  each undone back to the original revision.
- Each new assertion was run once against the pre-fix branch (temporarily restored, then the fix
  reinstated): all three targets failed there — the seam case reported the reproduced acceptance
  `[("xMjj", 3), ("iIBl", 21), ("OwTu", 21), ("CGMK", 21)]` — and pass with the fix.

### 9.4 Checks executed (developer results)

| Check | Result |
|---|---|
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test ps4_groups --test membership --test leagues -- --include-ignored` | ps4_groups 11 passed (incl. real NHL 20 + NHL 19 patch); membership 4 passed; leagues 11 passed; 0 failed |
| `cargo test -p studio-api --test membership` | 5 passed, 0 failed |
| `cargo clippy -p studio-domain -p studio-api --tests` | no diagnostics in the four touched files (repo-wide baseline warnings unchanged, C0.2) |
| `cargo fmt -- --check` | pre-existing whole-file diffs in the test files (C0.2 baseline); not repaired here |
| `git diff --cached` | empty — nothing staged |

Not rerun for R2 (unchanged by it): the catalogue generator check, the full `studio-api` and
`studio-domain` suites, bindings drift, the app suite. No DTO, binding, catalogue or frontend file
changed.

### 9.5 Scratch and cleanup (R2)

One package-owned file: a copy of the fixed `membership.rs` in the session scratchpad
(`H:\TEMP\claude\…\scratchpad\membership.rs.fixed`) used to swap the old check back in for the
regression proof, removed by literal path afterwards; the scratchpad is empty. No temp directories,
processes, images or fixtures were created. The shared `target` tree and the P1-A temporary
directories were not touched (the R1 removal of `target\debug\incremental` is acknowledged as an
ownership deviation; no shared-cache cleanup was performed in R2).

### 9.6 Remaining limitations

Unchanged from 8.7.

### 9.7 Next review action

Codex: review §9 and the four-file diff; rerun `ps4_groups` (+ `--ignored`), `membership`,
`leagues` and `studio-api membership`. Nothing is staged or committed. P1-BS-1 is next once P1-C
closes; the defaults writer was not started.

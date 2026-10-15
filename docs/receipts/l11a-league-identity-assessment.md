# L1.1-A — League identity mapping and existing-workflow assessment (receipt)

**Plain-language summary.** League names in NHL 21 are not plain text in the database: the league
table stores a *dictionary key* (for example `TXT_LEAGUE_DEL`, or even `NHL`, which the game renders
as "NHL®"), and the real words live in the localization file. So the only league rename Studio can
make today without localization work is to point a league at a *different existing* label; a new
name needs the Phase 1.3 localization step. On the twenty older PS3 / Xbox 360 / Recompiled targets
there is no league name in the database at all — only a number on each team — so for them league
identity is entirely a localization matter. Studio currently has no Leagues screen, shows wrong
league labels for most titles (the one label list was measured on NHL Legacy and reused
everywhere), has no list of pending changes, and its copied-target save exists only as an API. No
code was changed and no game was run; this package delivers the map and the proposed first
implementation slice.

---

## 1. Package, disposition, scope

* **Package:** L1.1-A per [s2c-r4-review-and-l11a-dispatch](../validation/s2c-r4-review-and-l11a-dispatch.md).
* **Disposition claimed:** assessment complete; **Phase 1.1 is not implemented or qualified on any
  target.** Nothing implemented, staged, committed or pushed; no live installation; no game run.
* **Scope executed:** dispatch steps 1–6. Exclusions respected (no feature code, no membership/
  rules/localization/asset work, no Projects/Builds, no Texture Lab, no head-converter work, no
  edits to Codex-owned roadmap/review records).

## 2. Changed paths and hashes

| Path | Status | SHA-256 |
|---|---|---|
| `docs/plans/league-identity-parity-matrix.md` | **new** (deliverable) | `91fbf782a32271e51904d1d94794bbb424cf1ca00524d2fc65f15dd97e75367b` |
| `docs/receipts/l11a-league-identity-assessment.md` | **new** (this receipt) | — |

Baseline `8d60a6d` on `feat/phase5-asset-tooling`; the pre-existing dirty tree (383 `git status`
entries at session start) is untouched. No source, binding, defs, anchor or audit file changed.

## 3. Findings and decisions

Full detail with per-target rows is in the matrix. The decisions that shape L1.1-B:

1. **NHL 21 league identity = `stockleagues` (`fsLK`)**: `leagueid` (key, 5-bit, 0..30), `name`
   (25-byte inline string holding a *localization key*), `artid`, `renderid`, `orderid`, two RGB
   colours. `exhibitionteams.league` (`kOtt.xMjj`) is a verified FK into it (433 team rows
   cross-tabulated; every populated code's cardinality matches its named league).
   `leaguegroup`/`conferencegroup`/`divisiongroup` are a separate, unnamed group-id namespace.
2. **`name` is a dictionary key, verified on the repository's own NHL 15/18/20 `eng_us.db`** with
   the handoff's reference reader (all CRCs match): `NHL`→"NHL®", `elitserien`→"SHL",
   `TXT_LEAGUE_EBEL`→"Erste Bank Eishockey Liga", `TXT_ROW_LEAGUE`→"Rest Of World"; nine keys
   (`AHL`, `HUT`, `CUSTOM_TEAMS`, …) are absent from every dictionary, so a literal fallback is
   *plausible but unverified*. NHL 21's own dictionary is a missing fixture. Studio has no
   varchar/Huffman reader, so it cannot show any localized league name today.
3. **Only the seven PS4 titles carry a directly editable league identity value.** All gen6/gen7
   targets (NHL 07–15, Legacy on PS3/X360; Recompiled by reference) store only the per-team code;
   their league names live in the big-endian `nhl_eng_us.db`, which nothing in the repo (nor the
   LE-only handoff reader) decodes. Recorded as **blocked pending the 1.3 reader**, not as
   unsupported.
4. **Existing defect (Hard rule 6):** `defs/enums/league.toml` is a single, untitled vocabulary
   whose codes ≥7 match only the Legacy / NHL 16 layout (its own note cites a 252-team DB — the
   Legacy anchor). Codes shift per title (five distinct gen7 layouts measured; PS4 ids move between
   15/16/17/18+), so Team headers and the Transactions league picker are mislabelled on most titles
   and unlabeled above 15 on PS4. Also, the Team Data "League" cell binds `leaguegroup` on NHL 12+
   while the header shows `xMjj`.
5. **Chosen NHL 21 change:** `stockleagues[8]` (`Allsvenskan`, 14 teams): re-point `name` to the
   existing key `TXT_ROW_LEAGUE`. Keys and topology untouched; expected in-Studio result and the
   (untested) in-game prediction are in matrix §4, with the semantic equivalent per target
   (resolve by name, never by raw id; gen7 equivalents are localization-only).
6. **Workflow gaps:** no Leagues surface; no per-change pending list (undo history is whole-DB
   snapshots, so a metadata-driven baseline↔working diff can provide it without new state); S2-C
   copied-target review/apply/restore has no UI; `edit-qa --spec` cannot address a league row; no
   `xMjj → leagueid` relation definition. Proposed bounded L1.1-B package and acceptance checks are
   in matrix §5.
7. **PS4 runtime persistence is blocked for all Phase 1 DB edits**: `nhl_game.db` lives in
   `inxp_00.big` (20-byte-row EB), the S2-A stager writes 16-byte rows only, and loose override
   precedence is unproven. This is recorded, not solved.

No accepted-document conflict was found. One clarification for Codex: the roadmap's step 1.1
"identity fields that exist in each target" resolves, for twenty targets, to *none in the main DB*;
the matrix records that per target rather than treating it as scope reduction.

## 4. Checks actually executed (developer, this session)

All read-only; anchors opened read-only; no Rust build was run (a documentation assessment — the
workspace was not rebuilt).

| Check | Result |
|---|---|
| `sha256sum` of all 27 main-DB anchors + PS4 dictionaries | PS3/X360 pairs byte-identical per title (10 pairs); NHL 19 = NHL 20 PS4; values recorded in the matrix |
| Python TDB v6 reader (mirrors `tdb-core` parse/`read_bits`/`read_inline_str`) over all 27 anchors: table inventory, `kOtt` league fields, `xMjj` distributions, `fsLK` rows on 7 PS4 anchors | outputs retained (`team_league_fields.txt`, `xtab_gen7.txt`, `stockleagues_ps4.txt`) |
| Team-abbreviation cross-tab per league code on every anchor | basis for the per-title code maps (candidates on gen7, verified against `fsLK` names on PS4) |
| Handoff `tool.mjs verify` on `anchors/ps4/{nhl15,16,17,18,20}/databases/eng_us.db` | rows 24 251 / 27 468 / 30 799 / 33 396 / 37 734; all four CRCs match on each |
| Handoff `tool.mjs export` + key lookup of every `stockleagues.name` token on NHL 15/18/20 | table in matrix §1.4 |
| Handoff reader on `anchors/ps3/nhllegacy/databases/nhl_eng_us.db` (BE) | fails (LE-only reader) — gen7 dictionaries remain undecoded |
| Code trace: `tdb-defs` updatable rule, `tdb-enums` (no title gating), `view.rs` team summary/label, `field_profile.rs:414`, `session.rs` `set_field`/`prepare_field_edit`/undo model, `TeamHeader.tsx`, `Transactions.tsx`, `PendingEdits.tsx`, `routes.tsx`, `edit_qa.rs` spec resolver, S2-A/S2-C receipts | findings §3 |

**Not executed / not claimed:** no `cargo test`, no `export-bindings --check` (no DTO change), no
game or emulator run, no save decode of NHL 21 `Season0`/`BeAGM0`, no archive extraction. Prior
independent review results (S2-C-R4) are not restated as fresh.

**Fixture identity:** NHL 21 `nhl_game.db` `90ee1ec4…8e9f9`; meta `0b262e4a…4991d`; dictionaries
NHL 15 `fa65b9af…5017`, NHL 18 `12f1af46…159f`, NHL 20 `62260005…a0f2` (NHL 19/20 provenance
unresolved — candidate rows).

## 5. Remaining blockers and limitations

* NHL 21 and NHL 19 `eng_us.db`: missing fixture (NHL 20's used as the nearest proxy).
* Gen6/gen7 dictionaries (`nhl_eng_us.db`, BE Huffman pool): no reader in Studio or in the handoff.
* PS4 in-game persistence for main-DB edits: PS4 EB writer absent / loose override unproven.
* Legacy Recompiled: no anchor, no adapter — untested on every row.
* Runtime unknowns listed in matrix §6 (literal fallback, consuming screens, group-id naming,
  `renderid`, two unidentified gen7 codes).

## 6. Temporary artifacts, processes and cleanup

Package-owned scratch root (session scratchpad, outside the repository):
`H:\TEMP\claude\e--Repositories-nhl-modding-studio\39435b70-83f0-4d5f-989f-713865ad26da\scratchpad\l11a\`

| Item | Purpose | Disposition |
|---|---|---|
| `tdb.py`, `meta_dump.py`, `dump_leagues.py`, `league_tables.py`, `team_league_fields.py`, `xtab.py` (≈11 KB, hashes below) | evidence scripts | **retained for Codex review** (owner: L1.1-A; re-runnable from the repo root with the system Python 3.12) |
| `team_league_fields.txt`, `xtab_gen7.txt`, `stockleagues_ps4.txt` (≈78 KB) | raw outputs behind matrix §3 | **retained for Codex review**; disposable after acceptance |
| `eng_us_nhl15.json`, `eng_us_nhl18.json`, `eng_us_nhl20.json` (12.8 MB) | dictionary exports | **removed** (regenerable in seconds: `node "<handoff>/tool.mjs" export <anchor eng_us.db> <out>`) |
| `__pycache__/` | Python bytecode | **removed** |
| Builds | none (no cargo invocation) | — |
| Processes | `python` and `node` ran to completion in the foreground; none spawned in the background | verified: no `node`/`python` process with a package path in its command line remains |

Script hashes: `tdb.py` `3af5f1cb…7dc7b`, `meta_dump.py` `689ace2a…8465`, `dump_leagues.py`
`8a5e215e…6855`, `league_tables.py` `1509ef15…c712`, `team_league_fields.py` `21395aa9…8d30`,
`xtab.py` `c1e502b6…13d4`; outputs `team_league_fields.txt` `c116a8ef…566f`, `xtab_gen7.txt`
`8cc6f395…f848`, `stockleagues_ps4.txt` `a9287b4c…e8c`.

External read-only input used (user-authorized via the knowledge-gap register): the NHL 21
localization handoff folder under `C:\Users\puckh\Documents\ChatGPT\NHL 21 Legends Project\audit\localization-developer-handoff\`
(`tool.mjs`, `reader.mjs`, `README.md`, `RESEARCH-AUDIT.md`). Nothing there was modified or copied
into the repository.

Removal was done with literal-path PowerShell operations after checking containment in the scratch
root and the absence of reparse points. No `git clean`/`reset`; no shared `local`/`target` tree
touched.

## 7. Next review action

Codex: review the matrix (§3.3 defect, §3.4 per-target dispositions, §4 chosen edit, §5 proposed
L1.1-B slice and acceptance checks) and issue the implementation prompt. Stopping here; no further
package started.

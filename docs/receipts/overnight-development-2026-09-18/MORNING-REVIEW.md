# Morning review — developer lane, overnight 2026-09-18

> **Corrected under [OVERNIGHT-DEV-R2](../../validation/overnight-2026-09-18-review-and-r2-dispatch.md)
> F3 (18 September, developer).** Phase 1 is *partially* implemented, not closed: the per-operation,
> per-target status is the [Phase 1 coverage matrix](../../validation/phase-1-coverage-matrix-2026-09-18.md).
> The **disc-image writer must not be used live** until Codex has reviewed the R2 F1/F2 corrections
> (receipt [08](08-r2-write-recovery-and-image-ownership.md)); every card below that named the
> Disc-image panel's Write is **withheld** and marked so. The cards are regrouped: **A** = the
> existing candidate identity tests (unchanged from the completion record), **B** = the new
> membership / localization / asset surfaces to look at in Studio (no placement), **C** = withheld.

**How to read this.** Everything below is *implementation complete / automated verified / user
review pending* unless marked otherwise. Nothing was launched, deployed live, quarantined or
committed overnight; no original install, anchor or disc image was written (real-file checks were
read-only or on temp copies). "Ready" means the Studio workflow exists and its automated checks
pass — it never means the game accepted anything. NHL 21 (NHL21-E1) stays accepted and closed; no
repeat is requested.

Receipts (plain-language summary first, then detail): [01 R1 corrections](01-r1-corrections.md) ·
[02 disc-image route](02-xbox360-iso-route.md) · [03 handoffs + archive containers + NHL 12 PS3](03-handoffs-and-archive-containers.md) ·
[04 Phase 1.2](04-phase-1-2-membership-groups-schedules.md) · [05 Phase 1.4 / 1.5](05-phase-1-4-league-logos-and-1-5-audio.md) ·
[06 free text in place](06-loc-w2-free-text-in-place.md) · [07 team text (2.3 opening slice)](07-phase-2-3-team-text.md) · [STATUS](STATUS.md). Research lane deliverables: `docs/research/overnight-2026-09-18/`.

## 0. Corrections first (LEAGUE-PARITY-COMPLETION-R1, F1–F5)

All five fixed with regressions that reproduce the defects; **Codex review pending** (receipt 01).
Nothing in tomorrow's cards depends on a known-failing correctness fix.

| Finding | What changed | Your check |
|---|---|---|
| F1 save split DB/dictionary | preflight of the whole set; temp+rename publish; verified rollback; split state named | none (automated) |
| F2 reload dropped edits/dictionary | reload prepares the dictionary too; explicit **detach** button on Leagues | optional: try detach on any title |
| F3 late rename responses | summaries adopted only for the open project; previews bound to inputs/dictionary | none |
| F4 quarantine interruption | per-file plans; restore works from any partial state; 27 fixture cases | reflected in the NHL 19 card below |
| F5 `traced` shown on PS3 | evidence per target: `traced` / `by-reference` / `hypothesis` / `native` | Leagues page pills read differently now |

## 1. What is new tonight (one paragraph each)

- **Xbox 360 disc images and packed archives are a Studio input.** Open a `.iso` (or a tree's
  `cacheboot.big` / `cache.big`): Studio reads the database and dictionary out into a staging folder,
  you edit/save there, and "Pending edits → Disc image / packed archive → Write" patches exactly the
  file's bytes at their offset, with a backup per range and a per-record Restore. All seven X: images
  and their trees were located, read and plan-only preflighted (never written). **R2: the writer's
  failure recovery and review/ownership were corrected (receipt 08) and are awaiting Codex review —
  do not write a live image yet; open/inspect/stage/edit/save in the staging folder is fine.**
- **League identity evidence is per target.** Every Xbox executable's code→key table is traced
  (research); PS3 twins are by reference. NHL 09 renames must use key `DEL`.
- **Phase 1.2 (partial).** Conference/division names on NHL 12 → Legacy; move a team between leagues
  with validated groups; browse and validate every league's stock schedule. *Not delivered:* a
  schedule editor/import/generation workflow, rules/season/mode-initialization coverage, PS4 group
  names (matrix §1.2).
- **Phase 1.4 (partial).** League logo index and package files per title with thumbnails and a
  loose-file Replace exercised on one real Legacy XPR2 package. *Not delivered:* RW4 / PS3 DDS /
  PS4 GNF league-package replaces, archive/disc placement of a replaced logo, non-logo league
  imagery (matrix §1.4).
- **Phase 1.5 (open).** Research found no *separately named* league audio bank on the Xbox titles;
  league cues inside generic banks, PS3 and PS4 are unknown. Not an absence claim for the step.
- **Free text in place (1.3 / 2.3 prerequisite).** The rename panel's second mode re-encodes a
  key's own text with the title's compression tree when it fits the old string (e.g. Norway →
  "Nordic"); exact refusals for unsupported characters (NHL 10: upper-case only) and over-length
  text. Works through every existing save/placement route. Runtime gate: the same tile check as a
  rename; a free-text label is the stronger visual proof that the dictionary is what the screen reads.
- **Team text (Phase 2.3 opening slice).** The Team Data tab lists the team's nine dictionary keys
  (by `artabbr`) with their text beside the database strings; each edits through the dictionary
  panel. Runtime question to settle when convenient: rename `NHLTeamName_TOR` only, then
  `fullname` only, and note which screens follow which (P2.3-X §"Consequence").

## 2. Cards by title, then step

Common rules: emulator closed before any placement; hash before/after; G0 (stock observation) before
any modified boot; restore before leaving a title. Exact identities are in the receipts and
`local/runs/league-parity-batch/manifests/*.json`. Placement helpers: `deploy.ps1` (rows) and
`quarantine.ps1` (NHL 19 only). **Studio's Disc-image panel Write is withheld (group C).**

Grouping per card: **A** existing candidate identity tests (the completion record's cards, unchanged
identities); **B** new surfaces to look at in Studio only — membership, schedule, logo, free text,
team text — no placement suggested; **C** withheld until R2 F1/F2 review.

### NHL 21 PS4 — closed (E1) for the identity rename. **B — 1.2 groups (P1-C):** the same
structure/membership cards as NHL 20/18/17/16 below apply here (NHL 21 is the developer-verified
anchor for the move / undo / save-to-copy / reopen / restore chain); Studio only, no placement.

### NHL 20 / 18 / 17 / 16 PS4 — 1.1 rename
**A.** Unchanged from the completion record's cards 1–4 (archive candidates `2d295097…`, `c6773f3c…`,
`3dd607f7…`, `fc64342c…`; rollback identities recorded). Gate: visual — the renamed tile.
**B.** *1.2/1.4 additions to look at while the title is open:* League page shows the stockleagues
structure fields, the schedule (31 × 82 on 18+), and the logo card (`kettle_assets` paths; shown
as "not extracted" unless the PS4 tree is extracted). *P1-A (18 Sep):* the schedule card's edit
row / Export / Import… / Generate… work here too (6-bit slots on 18+; the four zero-game division
squads are a warning, never a refusal). NHL 20's own database (not the NHL 19-base anchor) was
exercised offline: Austrian 270 rows / 11 clubs edits and re-imports, generation blocked with the
numbers.
**B — 1.2 groups/membership (P1-C, 18 Sep — Studio only, no placement).** With the title's own
`eng_us.db` attached: League page → Competition structure names the NHL's two conferences and four
divisions in the dictionary's words, pill "group table traced on this target"; the Spengler Cup
page (18+) lists its two pools (on NHL 18 marked *inferred* and shown as keys — its dictionary has
no text for them; on 19–21 traced, "Group Cattini / Group Torriani"); the Alumni / All-Time / Pro
Am pages (19–21) show "Unnamed group 77 / 78" with a *name unknown* pill and the research
hypothesis labelled as such, never as the name. Team page → League membership card (*R1*): pick
the AHL, then a conference — the division list shows only the divisions that conference's stock
rows carry (AHL Eastern → Atlantic / North; Western → Central / Pacific on 18–21); move Boston to
AHL Eastern / North (4 cells, one undo step), to the Spengler Cup choosing a pool (the same pool
in both slots), to Alumni (single group, id 77 written as the league's rows already carry it). An
NHL division under the AHL, or Western + Atlantic, is refused with nothing changed; on NHL 15–17
the "stock arrangements not offered" fold names the two All-Star squads. The league page's
structure card shows "Title provenance: recorded identity (base database …)" on a stock file and
"structure identifies this title … file identity not recorded" once you have saved an edit. Open
the same database under another PS4 title's game id and both cards fall back to plain numbers
with the reason ("… it identifies as nhl21"). Undo everything before leaving. Whether the game's
standings / schedule screens follow a moved team is **not** established — that observation is the
card's runtime gate if you place anything later.

### NHL 15 PS4 — 1.1 (two experiments, restore between)
**A.** Unchanged (base `5d011a06…` / patch `33bcce79…`).
**B — 1.2 groups (P1-C).** As above; NHL 15's table is its own numbering (six AHL divisions, OHL at
26, Custom Teams 54) — a value seen on NHL 16+ never applies here. The patch database's group ids
were not separately measured (disclosed on the structure card).

### NHL 19 PS4 — 1.1 with quarantine (user decision)
**A.** `quarantine.ps1 -Mode status` now prints a **per-file** plan and continues from any interrupted
state in either direction; unknown bytes are never overwritten (27 fixture cases re-run under R2;
the batch package's `SHA256SUMS.txt` is refreshed). Then cards 6a/6b as recorded.
Expected: `status` after quarantine shows both files `away`; after restore both `home`.

### NHL Legacy — Xbox 360 (X:\…\NHL Legacy)
- **A — 1.1 rename (traced binding), tree route only.** The loose `fe/loc/nhl_eng_us.db` via deploy
  row `nhllegacy-xbox360-loose` (replacement identity `6e80ecf3…`, rollback recorded), or the packed
  `cacheboot.big` candidate (`nhllegacy-xbox360-packed`, `67c0b46d…`). Observation: NORWAY tile →
  REST OF WORLD. Restore: deploy rollback. *Caveat:* a loose copy may shadow the archive or vice
  versa — if the label does not change, note which route was used and try the other after
  restoring. Whether Xenia launches the image or the folder is still unrecorded (research R0 §4).
- **C — withheld: image route.** The image `NHL Legacy Edition (…).iso` write (expected range
  `[4316217024, 4317976068)`, prior `f00926ce…` → `6e80ecf3…`) and the Studio archive-panel write of
  `cacheboot.big` (offset 1 959 616) wait for the R2 F1/F2 review. Inspect/stage/edit/save in the
  staging folder may be tried; do not press Write.
- **B — 1.2 groups/membership.** League page → Competition structure: two conferences, four
  divisions with the dictionary's names (traced). Team page → League membership card: move e.g.
  Boston to Western/Pacific (one undo step) — *no placement*; observe in Studio only.
- **B — 1.2 schedule (P1-A, 18 Sep — Studio only, no placement).** League page → Source schedule:
  opener Oct 7 TOR vs MTL, 82 per club, the two All-Star squads disclosed. Then: (a) *edit* row 0 →
  swap home/away → Preview shows 1 row / 2 cells, "valid" → Apply → one undo step, Pending Edits
  names `nhlschedule` row 0 (`home`/`away`); set the day to Feb 30 → Preview refuses ("Feb 30 does
  not exist"), Apply stays disabled. (b) *Export CSV* to a scratch file, *Import…* the same file →
  "0 cells", keys all `auto`; edit the file so one key reads `club9` → the mapping table asks for
  an explicit destination. (c) *Generate…* on the NHL table shows the blocker (1 230 games / 30
  clubs is no whole round-robin); on the SHL league page it suggests 4 round-robins and a preview
  shows 364 games, 52 per club. Undo everything before leaving. Expected surfaces are
  developer-verified on the Legacy anchor; nothing here is game-verified.
- **B — 1.4 logo.** League page → League logo: Norway index 15, four packages with thumbnails from
  the X: tree. A replace writes the loose tree file with a backup; the disc's archive copy is not
  touched (runtime precedence unknown; archive placement not implemented — matrix P1-F). Visual gate
  if you try it: the Norway tile art.
- **1.5.** Nothing built to test; the step is open (matrix P1-H).

### Legacy Recompiled (H:\Emulators\games\XBOX\NHL Legacy Decomp Test\game)
`_compiled` does **not** exist (research). **A.** The existing candidate row `legacyrecomp`
(`cacheboot.big` candidate `67c0b46d…`, dictionary `6e80ecf3…`) via `deploy.ps1`. **C — withheld:**
opening `game\cacheboot.big` as a packed archive in Studio and pressing Write (same identities:
`b558b4f6…` archive, `f00926ce…` → `6e80ecf3…`). Binding shown `by-reference`. Observation as Legacy Xbox.

### NHL Legacy PS3 — 1.1 (hdd0 first, then disc)
**A.** Unchanged cards 10; binding now shown `by-reference`. **B.** 1.2/1.4 surfaces as on Xbox (by
reference; PS3 logo packages are DDS — replace path not exercised on a league package, matrix P1-E).

### NHL 15 / 14 / 13 / 12 — Xbox 360 (X: trees + images) — **newly available**
- **C — withheld: every 1.1 write on these titles.** They have no `deploy.ps1` rows; the only
  Studio placement routes are the disc-image range patch (ranges: 15 `[4317081328, 4318810988)`
  `86167614…→b94c951c…`; 14 `[3845237152, 3846940356)` `e5aff828…→3af0b8a5…`; 13 `[476474864, 477627908)`
  `b93fa83f…→bd7fa83c…`; 12 `[499701536, 500638316)` `354a1905…→71fcec76…`) and the archive-panel
  write (`cacheboot.big` on 14/15, `cache.big` on 12/13) — both wait for the R2 F1/F2 review. The
  loose tree file could be placed by hand (copy with hash before/after) if you want a runtime probe
  before that review; test label: LEGEND TEAMS → REST OF WORLD. First establish whether Xenia runs
  the image or the folder (research R0 §4: not recorded anywhere; Xenia `cache:` may shadow — check
  `Documents\xenia\cache` before booting NHL 09 in particular). Binding **traced** per title.
- **B — 1.2.** Groups table per title (12/13: six named NHL divisions; 14: Division A–D; 15: A–D with
  Pacific/Central/Atlantic/Metropolitan text). Schedules 1 231 rows incl. the All-Star row; the
  P1-A edit/export/import/generate surfaces as on Legacy (NHL 15 and NHL 12 anchors exercised).
- **B — 1.4.** Logo indices traced; packages loose in the trees (XPR2; 12–15 share most art).

### NHL 11 — Xbox 360
**C — withheld:** 1.1 image range `[3896064976, 3896876340)` `82520ab3…→4b01f1c1…` (dictionary in
`boot.big`, database in `fe.big`) — binding traced, test `PROSPECT_TEAMS ← TXT_ROW_LEAGUE`. **B.**
1.2: local conference/division numbers (names untraced) — the membership card writes them raw.
Schedule 1 231 rows; P1-A editing exercised on the NHL 11 anchor, and the OHL page is the
record-order check: slot 8 must read Mississauga (stored after Owen Sound but ranked by index).
1.4: index traced; RW4 packages — replace **not implemented** on this era (matrix P1-E).

### NHL 10 — Xbox 360
**C — withheld:** 1.1 image range `[3876976640, 3877644636)` `f1a9a1dd…→f97aced5…` (DB and
dictionary are **loose on the disc**); binding traced, test `TXT_LEAGUE_SWISS_LEAGUE ← CUSTOM_TEAMS`.
**B.** New labels: code 8 Historic Teams, code 9 Prospect Teams. Schedule 1 230 rows (no All-Star row);
P1-A: the AHL page's slots 27/28 must read Wilkes-Barre / Worcester (the index-order exception).

### NHL 09 — Xbox 360
**B (Studio only).** 1.1 traced; **rename must target key `DEL` for code 4** (not `TXT_LEAGUE_DEL`).
Tree only (no image); no deploy row exists, so a runtime probe would be a hand-placed loose file.
Before any boot: research found Xenia's `cache:` holds an NHL 09 install copy and the tree
has `player.big.bak` + a modified head texture — decide whether to move the cache copy aside.
New label: code 8 Historic Teams.

### NHL 08 — Xbox 360
**B (Studio only).** 1.1 hypothesis (literal block, order-inferred). Tree only; no deploy row. Test
label SMLiiga → National if hand-placed.

### NHL 07 — Xbox 360
**Blocked** for renames: league labels are not in its dictionary and the `CHTM_LEAGUE_*` keys' source
is unresolved (not a proof of absence). Membership editing refused (thin index). *P1-A (18 Sep):*
the four NHL length variants (`nhlschedulesmall/medium/longleap/longnoleap`) and the three `intl*`
tables now resolve their teams through the stock table (slot 0 = Anaheim; Pacific rivals 8×) and
edit/generate like any other title; which variant the game actually reads is a runtime question.

### NHL 15 / 14 / 13 / 11 / 10 / 09 / 08 — PS3
**A.** Unchanged cards 11–12 (loose dictionary candidates); bindings now `by-reference` to the Xbox
trace. **B.** 1.2/1.4 surfaces available by reference (12–15) or as local numbers (08–11).

### NHL 12 — PS3 — **disposition changed**
The registered image is an **encrypted** redump ISO with no disc key on this machine: no in-image
route exists and RPCS3 cannot boot it either. Action (yours): register the extracted folder
`X:\…\NHL 12\[BLUS 30791] NHL® 12\` in RPCS3, then **A**: use the loose dictionary candidate
`a4e710dc…` as on the other PS3 rows.

## 3. Restoration summary

Every route above is reversible with a verified restore: `deploy.ps1 -Mode rollback`;
`quarantine.ps1 -Mode restore`; texture replaces write a `.bak` beside the file (`import_texture`
with backup). The Disc-image panel's Restore (records under `<staging>\records\NNNN\`, byte-exact,
interrupted writes included after R2) belongs to the withheld route. Staging folders and their
records are disposable once a restore verified `prior`.

## 4. Open questions for research / you

1. Which source Xenia launches per Xbox title (image vs folder), and loose-vs-archive precedence.
2. Whether a moved team (1.2) affects standings/schedule generation in a new Season.
2a. **P1-A runtime card (when a title is open anyway, loose-file route only):** swap the dates of
   two `nhlschedule` games in Studio, save, start a new Season and check the calendar shows the
   swap — this proves the source table *and* the rank-by-index slot rule in one shot (research 12
   §4). Also: does the engine accept a shorter table (Studio refuses to change row counts until
   this is known), and which of NHL 07's four length variants the game reads.
3. Which screen draws which league-logo family; NHL 07 label source.
4. Runtime effect of the enum-only league codes (CHL, HUT, …) when assigned to a team.

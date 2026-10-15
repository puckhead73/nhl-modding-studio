# Handoff R88 — correction checkpoint for the coordinator review of 24 September (F1–F4): the NHL 19 / NHL 20 rows of R86 / R87 re-measured on witnessed sources with a hash for every input (the anchor `ps4/nhl20` retained only as a mislabeled NHL 19 base measurement); the stat-table membership audited (the "one row per NHL club" statement is withdrawn — season / playoff stat rows cover NHL **and** AHL subsets, the PS4 21 career tables cover Swiss / other European teams); "PS4 18–21 can host only a non-NHL team" withdrawn in favour of measured headroom plus a proposed allocation experiment; the R86 re-key proposal downgraded to a read-only shared-art-id inspection plus an unfinished contract with the destination-occupancy, sharing, atomicity and PS4 gaps named; the audio closeout re-bounded (the `ABKC` 0x78 tree unblocks indexing, not replacement; the NHL 21 single-block decoder disagreement carried; the u8 frame-length domain corrected)

Research lane, 24 September 2026, 13:17 → 13:25 MDT (real clock; the probe inputs were extracted at 13:17) (review: [research-20260924-campaign-review.md](../../validation/research-20260924-campaign-review.md)).
Claim `research-r88-correction-checkpoint`. Read-only over `anchors/`, the CUSA11126 / CUSA15439 installs (two members extracted to the
session scratchpad, hashed, deleted) and this lane's JSONs. Nothing game-verified; no writer proposed; no earlier evidence replaced —
the superseded statements are marked in place with dated notes and their original files are unchanged apart from those notes.

Machine-readable: [`data/r88-correction-checkpoint.json`](data/r88-correction-checkpoint.json) (28 sources — **27** per the 2026-09-25 direction audit and the JSON itself (R97 §5): the 25 anchors + NHL 19
patch 56 + the real NHL 20 install, each with db / meta SHA-256 and a provenance label; the F2 membership audit per table; the F3
occupancy check). Probe `r88_correction_sources.py` (≈ 3 min; reuses the R86 / R87 code paths unchanged except for the class labels).

## Plain-language summary

Four corrections. First, two of this campaign's tables quoted a "NHL 20" database that is really a copy of NHL 19; the affected rows
are now measured on the genuine NHL 20 install and on NHL 19's last patch, and every input carries its hash. Second, the team-creation
analysis had turned what the shipped files happen to contain into requirements: it said every NHL club needs a statistics row and that
only a non-NHL team could be added. The audit shows those tables hold rows for a season's NHL and AHL participants (and, in the PS4 21
career tables, Swiss and other European teams), so the statement is withdrawn; what remains is the measured spare capacity per table and
a proposed experiment. Third, the art-id re-key proposal is not a finished contract: on NHL Legacy 156 of the "free" ids already have
art files, sharing inside archives was not measured, and the undo story was inadequate. It is downgraded to a read-only inspection until
those are closed. Fourth, the audio closeout over-reached in three places: decoding the bank tree helps indexing, not replacement; one
sample where the two Speex decoders disagree strongly was hidden in a range; and the frame-length grammar printed a value it cannot encode.

## F1 — provenance of the NHL 19 / NHL 20 rows (R86 §1–2, R87 §1–3)

| Source | `nhl_game.db` SHA-256 | meta SHA-256 | provenance |
|---|---|---|---|
| `ps4/nhl19-base` (anchor) | `265e4a68…f408` | `c67f4bfd…90b3` | NHL 19 base `inxp_00.big` member (CUSA11126) |
| `ps4/nhl19-patch56` | `61332e66…b58d` | base meta (patch archives carry no meta member; table sets identical) | CUSA11126 `patch_inxp_56.big` entry 504 |
| `ps4/nhl20-install` | `1e200478…a57b` | `2fac0e73…1f93` | CUSA15439 `inxp_00.big` entries 13224 / 7621 (01.00, no patch archives); byte-identical to the hash-bound copies under `local/runs/league-parity-batch/sources/nhl20-ps4/` |
| `anchors/ps4/nhl20` | `265e4a68…f408` | `c67f4bfd…90b3` | **mislabeled** — NHL 19 base copy; every "PS4 19 / 20" figure in handoffs 04 §1 / 05 §1 and the R86 / R87 JSONs is an NHL 19 base measurement, retained as such |

The other 23 anchors' hashes are in the JSON (`sources.<label>.db.sha256` / `.meta.sha256`); gen7 X360 / PS3 pairs are byte-identical
per title (NHL 08 `587204c3…`, 09 `0f7c826b…`, 10 `e87b8c80…`, 11 `461f8d64…`, 12 `94b31ff2…`, 13 `64460a15…`, 14 `84e1bdff…`, 15
`028ae06a…`, Legacy `105d2186…`); PS4 15 `ae9d1d56…`, 16 `7d95f1db…`, 17 `de48b02b…`, 18 `65eaa6e8…`, 21 `90ee1ec4…`.

**Recomputed rows (only the affected ones):**

| Measurement | NHL 19 base | NHL 19 patch 56 | **real NHL 20** | NHL 21 (unchanged) |
|---|---|---|---|---|
| `exhibitionteams` rows / capacity (headroom) | 419 / 519 (100) | 423 / 519 (96) | **416 / 517 (101)** | 433 / 533 (100) |
| free `index` values (0–1 023) | 605 | 601 | **608** | 591 |
| `exhibitionarena` rows / capacity | 426 / 526 | 431 / 526 | **428 / 529** | 450 / 550 |
| `stockteamjerseys` rows / capacity | 1 532 / 2 037 | 1 583 / 2 037 | **1 652 / 2 254** | 1 792 / 2 411 |
| `exhibitionrostertable` rows / capacity | 8 328 / 15 567 | — | **8 613 / 15 735** | 9 090 / 16 576 |
| `stockgoalieequipment` rows / capacity | 998 / 1 708 | — | **1 005 / 1 713** | — |
| `stockleagues` rows / capacity | 29 / 29 | 29 / 29 | **31 / 31** | 31 / 31 |
| schedule tables | as handoff 05 | same | **adds `worldchampschedule` 0 / 100, `worldjuniorschedule` 0 / 60; `ahlschedule` 1 150 / 1 210; `echlschedule` 936 / 996; `austrianschedule` 270 / 300; `whlschedule` 748 / 808** | — |
| `artid` distinct / duplicate ids | 307 / 37 | 308 / 38 (adds 39 × 4) | **304 / 42** (0–29 × 2–3; 36 × 4; 39 × 4; 48, 49 × 2; 50 × 3; 53 × 2; 345 × 10; 346 × 11; 347 × 2; 500, 501 × 2; 600 × 40) | 312 / 37 |
| `stockteamjerseys.teamartid` coverage (values outside) | 0.971 (5) | — | **0.958 (7)** | 0.949 |
| `*resort*extras.teamartid` coverage | 0.936 (2) | — | **0.936 (2)** | 0.936 |
| `league` membership | 23 values, league 23 = 80 | league 23 = 84 | **23 values; league 23 = 80, 15 = 26, 7 = 11, 22 = 4** | 23 values |

Patch 56 versus base: + 4 team rows (all league 23), + 5 arenas, + 51 kit rows, one more shared art id (39 × 4); the loader's
base-versus-patch precedence stays unverified (2026-09-18 record). **Omitted, stated:** NHL 07 (gen6 schema, no `exhibitionteams`),
Legacy Recompiled (no independent anchor; not measured), NHL 19 patches 51–55 (not extracted), the NHL 15 PS4 patch DB (`abe36f58…`,
not re-measured); "every gen7 main database" in handoffs 04 / 05 means the 18 X360 / PS3 08–Legacy anchors, nothing else.

## F2 — capacity is not support; stock distributions are not requirements (R87)

**Terminology fixed.** "Headroom" = record slots declared by the table header and unused in the inspected file. It establishes that a
row *could be stored without growing the table*; it does not establish that Studio can serialize one there, that the archive placement
succeeds, that the game loads it, or that a mode initializes with it. A zero-headroom finding scopes a refusal to **a no-growth in-place
allocation on the inspected file** — whether a full table can grow is the open engine question (Q-R87-1), not a refusal. The "index
reference" class in the R87 JSON was assigned from numeric coverage ≥ 0.95; it is relabelled **candidate index reference (coverage;
not a foreign-key proof)** in the R88 JSON, and the per-team row counts are relabelled **observed stock distribution, not a requirement**.

**Membership audit (who actually holds rows; `team` → `exhibitionteams.index` → `league`, names checked):**

| Table | NHL 08 | NHL 09–Legacy (X360 / PS3), PS4 15–17 | PS4 18 | NHL 19 base / patch 56 | real NHL 20 | NHL 21 |
|---|---|---|---|---|---|---|
| `stockseasonteamstats` (1 row per team) | 30 NHL | 30 NHL | 30 NHL + 30 AHL | 31 NHL + 30 AHL | **31 NHL + 31 AHL** | 31 NHL + 31 AHL |
| `stockplayoffteamstats` (1 row per team) | 16 NHL | 16 NHL + 16 AHL | 16 + 16 | 16 + 16 | 16 + 16 | 16 + 16 |
| `stockcareerseason*stats`, `stockcareerplayoff*stats` (PS4 20 / 21) | — | — | — | — | 32 teams: league 10 (21), league 6 Swiss (9), league 11 (2) — **no NHL team** | same 32 (playoff goalie: 31) |
| `stockteamhistory` (PS4 20 / 21) | — | — | — | — | 118 teams: leagues 2–6, 10–12 + one NHL club | 119 |
| `stockrecordstable` (PS4 21) | — | — | — | — | — | 32 NHL-league rows |
| `stockawardstable` | 36 (NHL 10) | 99–100 (NHL + AHL + 3 European) | 96 | 99 | 100 | 101 |

Consequence: the handoff 05 statement "one row per **NHL** club; a new NHL club would need a slot that does not exist on any anchor" is
**withdrawn**. The rows are the participants of shipped seasons (16 playoff teams per league, 30 / 31 season teams for NHL *and* AHL;
European career tables on PS4 20 / 21). Whether a team without a stat / history row is tolerated, ignored or required by any mode is
**not a data fact** — it is a runtime question. Career / history rows likewise cannot be made mandatory from their presence.

**Withdrawn:** "PS4 18–21 can host only a non-NHL team". **Replacement statement:** on PS4 18–21 and on the real NHL 20 the team, arena,
kit, roster, goalie-equipment, retired-number and per-league schedule tables have unused slots (§F1 table, handoff 05 §1); the
season / playoff stat tables and the PS4 20 / 21 career / history / record tables have none in the inspected files. What a new team of
any league needs at runtime is unresolved. **Proposed allocation experiment (not run; developer + user, staged so each layer is
separately falsifiable):** (1) Studio serialization — write one `exhibitionteams` row into an unused slot of a *copied* NHL 21 DB with
the current-count word incremented, plus 2 `stockteamjerseys`, 2 `stockgoalieequipment` and ≥ 7 `exhibitionrostertable` rows referencing
existing players, using an existing art id (shared-key semantics, handoff 04 §1); verify the file re-parses and every untouched byte is
unchanged; (2) archive placement through the developer's copied-target route (EB replace, append-and-retarget) on the disposable copy;
(3) game loading — exhibition team select shows the team (or not; or crashes); (4) mode initialization — a season including the league
with the extra team (schedule generator bounded by the league table's spare rows: 60 on `ahlschedule` for real NHL 20). Each stage's
failure is a separate finding; none is a creation contract. **No league-creation contract is inferred** from the team census; the
`stockleagues` table is full on every PS4 anchor and on the real NHL 20 (31 / 31), and league identity on gen7 is executable-resident
(2026-09-18 handoff 23).

## F3 — the R86 re-key proposal is not yet a complete, reversible contract (handoff 04 §4)

Downgraded to: **read-only shared-art-id inspection (usable now)** + **an unfinished writer contract with the following gaps named**.

- **Destination occupancy across the whole graph.** Handoff 04 tested only whether team rows hold b. Measured on Legacy X360 from the
  2026-09-18 join JSON (`nhllegacy-team-asset-joins.json`, SHA-256 `d2a7e0a2…`): 237 art ids have files in the team-art families
  (crest 101, small crest 101, rink logo 130, centre logo 35, puck 32, FE jersey 73, nameplate 84, kit textures 85) without any team
  row; **156 of the 32 550 "free" ids are occupied by such files**; the lowest ids free of both team rows and files are 54–65 and
  100–107. Every artid-keyed DB column's values lie inside the team set on Legacy (0 outside); on PS4 the spare-kit values (255,
  1000–1004) and the custom-team block occupy ids outside it. The contract must therefore check b against every `*artid*` column (not
  only team tables), every loose family on the title, every archive member name and — unmeasured — every internal reference; **default
  refuse** on any hit; never overwrite unrelated bytes.
- **Sharing.** The sentence "a file a's rows share with rows of another id cannot exist by construction" is **withdrawn**: path keying
  shows only that the *name* carries one id. Shared spans (BIG4 dedup, PS4 EB duplicate-block sharing), cross-key references inside
  members (kit → nameplate → crest), and the path-derived `word_12` of PS4 EB rows (formula unknown) were not measured. A rename inside
  an archive needs the name table, the row's path-derived fields and any internal id updated — none enumerated; the existing
  member-byte stager (`stage.rs`) replaces payloads and is **not** a rename route.
- **Atomicity and restore.** "One undo step per table" and a reverse b → a operation do not restore the baseline: a copy leaves the
  original files in place, a rename does not, and a partial cascade leaves a mixed state. The contract needs one logical operation
  across DB and files with a journal, failure / restart recovery and **exact baseline restoration verified by the SHA-256 of every
  touched file and of the DB**. Not designed here.
- **Cross-title cascade.** §3's family table is compiled from the Legacy X360 join (measured) plus earlier family-level witnesses
  (R56 / R76 kit joins; PS4 R7 vault joins). It is **not** an exhaustive cross-title enumeration; each title needs its own measured
  family census before a cascade is proposed there.
- **PS4:** refused consistently — vault `Team` references (texpacks, spotlight / BAP logos) are unresolved and unwritable; the
  existence of packs for b changes nothing.
- **T-ARTID-1 narrowed.** A successful run supports the tested surfaces (FE crest, rink art, kits) on that title and file set; it does
  not establish completeness of the graph. Missing art has competing explanations (placement, cache, consumer behaviour, a family not in
  §3) that the card cannot separate.

## F4 — audio closeout boundaries (handoffs 02 / 03, final receipt)

- **The `ABKC` 0x78 tree** (queue row 11, package R89): decoding it can give sound → sample index and stream → `.ast` block address —
  **indexing and stream discovery**. It is not "the last unknown before same-size replacement". Two replacement classes, each with its
  own prerequisites (all currently unmet unless marked): **(a) same-length encoded-payload substitution** — entry indexing + record
  grammar (met, handoff 01), tree pointers (open, R89), an encoded payload of the same codec / rate / channels with identical byte length
  and sample count (needs a qualified encoder or a donor entry), any duration mirror (`.DUR`-like fields, unknown for `.abk`), archive
  re-wrap for archived banks (RefPack / chunkzip; encoder fidelity), loader acceptance (runtime); **(b) arbitrary audio import** — all of
  (a) plus a qualified encoder for EALayer3 v1 / EA-XMA / EA-Speex, changed-size table rewrite and absolute pointer update. Handoff 03 §5
  already lists these elements; the receipt's sentence is superseded.
- **Speex decoder disagreement, exact scope.** Handoff 02 A.3's last row: PS4 NHL 21 `rink_ann_samples.sbs` (base bank, 1 record,
  single block, 12 frames) — both decoders return 12 × 640 samples, but their outputs differ by an rms ratio 0.89 with correlation
  **0.46**, against 0.07–0.29 / ≥ 0.96 on the other 95 vectors. It is the only single-block vector and the only NHL 21 base-bank vector;
  the cause (single-block handling, the base-bank encode, or a decoder difference) is not identified. Carried into queue row 3, the final
  receipt and the developer contract: on this class neither decoder is validated; sample-count agreement is not sample fidelity.
- **The 509-sample start** is a measured count relationship (first block declares 11 × 640 − 509; single block 12 × 640 − 509 − trim)
  and the pinned reference decoder's rule — **not** a game trace; "the game drops 509 samples" in handoff 02's summary is superseded by
  that wording.
- **u8 frame-length domain.** The parser accepts a length byte of **1..255** (0 is refused — measured by the malformed control; 255 is
  the largest encodable value; **256 is not encodable** — "1..0x100" in handoff 02 A.1 / A.4 was wrong); observed lengths 15..110 bytes.

> **Note 2026-09-25 14:20 (R97 §5, [overnight-2026-09-25/10-correction-checkpoint-direction-audit.md](../overnight-2026-09-25/10-correction-checkpoint-direction-audit.md)):** §F3 "every `*artid*` column" is narrowed to the **team-art namespace** (`exhibitionteams.artid`
> + `leagueteams` / `customteams` copies, `stockteamjerseys` / `customteamjerseys.teamartid`, PS4 awards / resort-extras `teamartid`); player
> portrait / head ids, `arenaartid` / `centericelogo`, `coachartid` and any league-art column are other namespaces — an equal number there is
> not collision evidence; multi-namespace file families (`centerlogo_{centericelogo}`) and in-member references stay unresolved (Q-R88-2).

## Consequence for the ledger

- Blocker map rows for Teams 2.1 (`artid`) and Leagues 1.2 / Teams 2.2 (creation) revert to **contract-blocked**: inspection inputs
  ready, writer proposals not accepted; the measured capacities stand with corrected provenance.
- Audio rows unchanged in substance; the replacement-boundary wording and the outlier are corrected in the queue and receipt.
- Claims `research-r83…`, `research-r86…`, `research-r87…` carry a `revision` note pointing here; `research-r88-correction-checkpoint`
  is `ready`.

```
python local/runs/overnight-research-2026-09-18/probes/eb_list.py "H:\Emulators\ShadPS4\Games\CUSA11126\databigs\patch_inxp_56.big" --extract nhl/data/common/gamemodes/database/t3db/nhl_game.db <p56.db>
python local/runs/overnight-research-2026-09-18/probes/eb_list.py "H:\Emulators\ShadPS4\Games\CUSA15439\databigs\inxp_00.big" --extract nhl/data/common/gamemodes/database/t3db/nhl_game.db <n20.db>
python local/runs/overnight-research-2026-09-18/probes/eb_list.py "H:\Emulators\ShadPS4\Games\CUSA15439\databigs\inxp_00.big" --extract nhl/data/common/gamemodes/database/t3db/nhl_game-meta.xml <n20-meta.xml>
python local/runs/overnight-research-2026-09-24/probes/r88_correction_sources.py <p56.db> <n20.db> <n20-meta.xml> docs/research/overnight-2026-09-24/data/r88-correction-checkpoint.json
```
Executed twice (the second run excludes the arena-keyed families from the F3 union); the extracted members were deleted afterwards.

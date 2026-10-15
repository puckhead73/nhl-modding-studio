# League identity parity matrix (Phase 1.1)

> **Current authority (2026-09-17, after L1.1-Q1 and its correction L1.1-Q1-R1):** section 0 below
> is the current per-target state. Everything after it (§1–§6) is the **historical L1.1-A
> assessment** written before any league code existed, plus the overnight corrections; it is kept
> dated and unedited except for the "Dated assessment" notes. Where §1–§6 disagree with §0, §0 wins.
> Reviews: [L1.1-Q1 review](../validation/league-l11-q1-review-and-r1-dispatch.md);
> receipts [league-l11-q1](../receipts/league-l11-q1.md), [league-l11-q1-r1](../receipts/league-l11-q1-r1.md).
> Runtime experiments: [league-identity-runtime-qualification](league-identity-runtime-qualification.md).
> **No edited target is game-verified.** NHL21-E0 stock G0 is accepted for Play Now / Select Team and Rosters / Edit Player on the recorded fork; restart/re-entry remains unobserved. See [E0 review](../validation/league-nhl21-e0-review-and-e1-prep-dispatch.md).

> **Historical banner — L1.1-A (2026-09-17, morning, pre-implementation).** Package L1.1-A of
> [s2c-r4-review-and-l11a-dispatch](../validation/s2c-r4-review-and-l11a-dispatch.md); developer
> assessment only; baseline `feat/phase5-asset-tooling` at `8d60a6d`, dirty tree preserved (383
> entries). *At that time* nothing was implemented, staged, committed or pushed and no game was run;
> every "expected in-game" statement in §1–§6 was, and still is, a prediction. The league code
> described in the receipts has since been implemented (still uncommitted); the runtime statements
> are still untested.

> **Historical banner — overnight corrections (2026-09-17, per the
> [overnight brief](league-overnight-execution-2026-09-17.md), L1.1-A accepted as planning input):**
> (1) the pending review compares the **acknowledged baseline** (`Project::db_bytes`, loaded or
> verified-saved) with the working bytes — never an undo snapshot; (2) "every other byte unchanged"
> is replaced by an **allowed-difference audit**: only the edited record differs at record level,
> derived integrity bytes (headers/CRCs) may move, restore reproduces the whole-file hash; (3)
> `TXT_ROW_LEAGUE` was at that time verified in the NHL 15/16/17/18/20 dictionaries only — the NHL 19
> and NHL 21 dictionaries were *not among the supplied fixtures* (they do ship with the games; NHL
> 21's has since been located and verified, §0.3); (4) gen7 labels keep their candidate/corroborated
> provenance per `defs/leagues/*.toml`, raw codes are shown where uncertain, and no neighbouring
> dictionary is ever reused; (5) the absence of a name field in a main DB does not prove every
> identity consumer is localization-only — what was inspected is the main DB, its metadata and
> Studio's own surfaces; UI/executable consumers stay open. Implementation status is in
> [league-overnight-2026-09-17](../receipts/league-overnight-2026-09-17.md); the 27 explicit
> per-target execution rows are in
> [league-identity-offline-campaign](../validation/league-identity-offline-campaign.md).

---

> **Batch preparation note (2026-09-18, LEAGUE-PARITY-BATCH, pending Codex review).** The
> consolidated per-target matrix and the ordered runtime checklist for every installed title now
> live in [league-parity-batch-2026-09-18](../validation/league-parity-batch-2026-09-18.md). Two
> provenance corrections from that batch are **folded into the NHL 19 / NHL 20 rows of §0.2 (REG-1)**:
> (1) the `anchors/ps4/nhl20` database, metadata and dictionary are the **NHL 19 base** members
> (the NHL 20 install carries a different 31-row `stockleagues` database and dictionary; NHL 20 was
> missing-fixture), so every "NHL 20" measurement in §0.2/§1.4 is an NHL 19 base measurement;
> (2) the PS3 NHL 08/09 dictionary anchors are the Xbox 360 files. Also new there: the gen7
> code→key binding traced in the Legacy Xbox 360 executable, NHL 15 PS4's unsupported archive
> layout, NHL 19's seven shipped databases and stale loose tree, and the LOC-W1 / EB-V1 packages.
> **Completion queue (2026-09-18, pending Codex review):** EB-V1 (NHL 15 PS4 archives bind and stage) and
> LOC-W1 (dictionary rename route for PS3 / Xbox 360 / Recompiled, desktop-exercised) are implemented;
> the refreshed per-target matrix and runtime cards are in
> [league-parity-completion-2026-09-18](../validation/league-parity-completion-2026-09-18.md).

## 0. Current state per target (L1.1-Q1, corrected by L1.1-Q1-R1, 2026-09-17)

> Read-only reconciliation after LEAGUE-RESP-1 acceptance. Durable evidence for the NHL 21
> provenance chain: [`docs/validation/evidence/league-l11-q1/`](../validation/evidence/league-l11-q1/).
> In particular the following §1–§6 statements no longer describe the code: §1.4 "Studio cannot
> read any dictionary today", §1.5 "the PS4 20-byte-row writer does not exist", §2 "no league
> entity view / no Leagues page / no per-change pending list / no copied-target UI", §3.3's
> `league.toml` defect, and all of §5 — superseded by the implementation recorded in the receipts
> cited in each row below.

### 0.1 Evidence classes used in this section

| Column | Meaning | Who established it |
|---|---|---|
| **Schema** | where league identity lives on this title and how it was measured | anchor measurement (L1.1-A, `tests/leagues.rs`) |
| **Editor** | offline read/write evidence in Studio: edit → pending → review → apply → allowed-difference audit → reopen → byte-exact restore, on a **copied fixture** | `league_offline_campaign` (developer-run; **independently rerun by Codex**, 28 rows, [overnight review](../validation/league-overnight-review-and-r1-dispatch.md)) |
| **Dictionary** | whether the **title's own** localization dictionary is available, captured, and verified to contain the keys the edit uses. "Decodes" means the reader parses it; it does **not** say which key any consumer looks up | `tdb-loc` tests on committed anchors; L1.1-Q1 for NHL 21 |
| **Packaging (offline artifact)** | whether Studio can **produce** an artifact in the shape the game ships (loose file / archive member) and verify it structurally. This is offline artifact evidence only — it says nothing about whether the game reads that location, accepts the artifact, or which location wins | S2-A/S2-C stagers; `ps4_real_archives` (env-gated, developer-run only); desktop archive apply on a copy |
| **Desktop** | the real Tauri build exercised over CDP on copied fixtures (developer observation; **never independently rerun**) | SMOKE-1 / SMOKE-R1 / RESP-1, screenshots under [`docs/validation/evidence/`](../validation/evidence/) |
| **In-game** | game-verified per the CLAUDE.md vocabulary, for the exact build/environment/surface recorded | **nothing, on any target** |

"Identical bytes" means two anchors hash the same and therefore yield one measurement; it is
**not** title provenance and not a shared verdict. Rows that share a measurement say so and link
to the same measurement id (`M-…`); each shipping target keeps its own verdict columns so a
future run can fill one without touching the other.

### 0.2 Per-target rows (27 shipping targets; NHL 07 PS3 research-only)

**PS4 — a direct identity field exists (`stockleagues.name`).**

| Target | Schema | Editor (offline) | Dictionary | Packaging (offline artifact) | Desktop | In-game |
|---|---|---|---|---|---|---|
| **NHL 21 PS4** | **verified**: `stockleagues` 31 rows, `name` 25-byte key, `kOtt.xMjj` FK (§1). Anchor `90ee1ec4…` is **byte-identical to the installed `inxp_00.big` member 2652** (§0.3, independently re-hashed by Codex) | **automated-tested**: campaign row (edit `Allsvenskan`→`TXT_ROW_LEAGUE`, output `343404153ae5…`, restore = anchor); `league_copied_target` 7 cases; referential gate; independently rerun | **available from the install, hash-bound, not a repository fixture** (coordinator decision: not committed in this package): member 1156 of the installed `inxp_00.big` (raw, 3 921 060 B, SHA-256 `24693e2f…`, payload identity independently re-hashed by Codex); verified with `tdb-loc` in L1.1-Q1 (developer): 4/4 CRCs, 55 766 rows, `Allsvenskan`→"HockeyAllsvenskan", `TXT_ROW_LEAGUE`→"Rest Of World" (§0.3) | **artifact produced and structurally verified**: synthetic stager tests; same-length raw member replacement on a copy of the real archive (developer-run `ps4_real_archives`, not independently rerun); desktop apply/restore on the real 777 MB copy (SMOKE-1 #23–26, #49; RESP-1). The artifact keeps two **unresolved** words (`+0x0C`, header `0x10`) verbatim. **Whether the game reads either location, accepts the artifact, or prefers one location is unknown** | developer observation on copied fixtures: SMOKE-1 §3.1–3.3, §3.6; SMOKE-R1 §4; RESP-1 (screenshots 00–29, 40–53 + resp-1 set) | **Stock G0 accepted** (NHL21-E0, two screens); stock restart with the existing profile **observed** 2026-09-18 (F12 ×2, Edit Player + re-entry). **E1 archive-only, user-observed (2026-09-18):** candidate `4876cd2b…` live as `databigs/inxp_00.big` → user reported REST OF WORLD in the former HockeyAllsvenskan slot on Rosters/Edit Player across two cold boots; verified restore (E5) → HockeyAllsvenskan again (user-confirmed). **No screenshots of the modified/restore boots — user decision**; developer-attested user observation, not screenshot-backed. Environment: modified shadPS4 fork `aeb04892…` only. See [league-nhl21-e1](../receipts/league-nhl21-e1.md) |
| NHL 20 PS4 | **verified from the install (REG-1, 2026-09-18)**: CUSA15439 01.00 `inxp_00.big` #13224 `nhl_game.db` `1e200478…` (31 `stockleagues` rows, NHL 21-family layout), meta `2fac0e73…`. **The committed `anchors/ps4/nhl20` files are NHL 19 base bytes** (`265e4a68…`); NHL 20 has no anchor — sources are private fixtures (`databases/nhl20-ps4-cusa15439/…`) | automated-tested on the **install's own** bytes (LEAGUE-PARITY-BATCH row `nhl20-ps4`, output `3eee9daa…`; `league_private_sources` ignored case). The earlier campaign row measured NHL 19 base bytes | the install's own `eng_us.db` #6245 `0fc95ec8…` (40 535 rows; `Allsvenskan`→"HockeyAllsvenskan", `TXT_ROW_LEAGUE`→"Rest Of World", `TXT_USA_LEAGUE`→"USA") — private fixture, not committed | candidate archive `2d295097…` (517 362 304 B) produced and structurally verified; original `10306c2f…` | none | not executed (batch: ready, preflight PASS) |
| NHL 19 PS4 | **base attributed (REG-1)**: anchor `265e4a68…` = CUSA11126 (APP_VER 01.60) base `inxp_00.big` #2089; the install also ships `patch_inxp_51..56.big` with six further databases (patch 56: `61332e66…`, 10 318 668 B — only the Pro-Am member count differs from base, 80→84); which one the game loads is unverified. Adapter promotion pending (canonical package) | automated-tested: base row (output `eda86b60…`) and patch-56 row (`3d271b06…`) in LEAGUE-PARITY-BATCH; `league_private_sources` ignored case | base dictionary #936 `62260005…` (= the file committed under nhl20); patch-56 dictionary #209 `fbb8adcf…` (38 001 rows) — both verified with `tdb-loc`; private fixtures | two candidate archives: base `9100d5fe…` (445 758 880 B), patch-56 `4e69f9c3…` (128 855 568 B) | none | not executed; **refused by the helper** while the stale loose game tree (2026-07-19) sits in the install — quarantine plan in the batch record |
| NHL 18 PS4 | verified (anchor `65eaa6e8…`) | automated-tested (campaign row; R2 referential regressions on this anchor); independently rerun | committed anchor `12f1af46…` (attributed); desktop-attached from the install layout on a copy | none on an NHL 18 archive | developer observation: SMOKE-1 §3.4 (#30–34: dictionary attach, unknown key stays a key) | not executed |
| NHL 17 PS4 | verified (`fsLK` 24 rows, `orderid` appears) | automated-tested (campaign: `TXT_LEAGUE_NORWAY`→`TXT_ROW_LEAGUE`); independently rerun | committed anchor (30 799 rows; `TXT_LEAGUE_NORWAY`→"Norway") | none | none | not executed |
| NHL 16 PS4 | verified (`fsLK` 22 rows; no `orderid`/colours) | automated-tested (campaign: `TXT_LEAGUE_NORWAY`→`TXT_ROW_LEAGUE`) | committed anchor (27 468 rows; `TXT_LEAGUE_NORWAY`→"Norway") | none | none | not executed |
| NHL 15 PS4 | verified (`fsLK` 20 rows; no `orderid`/colours/`renderid`) | automated-tested (campaign: `TXT_LEGEND_TEAMS`→`TXT_ROW_LEAGUE`) | committed anchor (24 251 rows; `TXT_LEGEND_TEAMS`→"Legend Teams") | none | none | not executed |

**PS3 / Xbox 360 / Recompiled — no direct identity field.** Shared measurements (one per title,
because the PS3 and Xbox 360 anchors are byte-identical): `M-legacy` (`gen7-nhllegacy.toml`,
corroborated by NHL 16 PS4 on an identical team distribution), `M-12-15` (`gen7-nhl12-15.toml`,
corroborated by NHL 15 PS4), `M-11`, `M-10`, `M-09`, `M-08` (candidate vocabularies; NHL 09 code 8
and NHL 10 code 9 raw), `M-07` (gen6 `HqzU.xMjj` uint 3, `gen6-nhl07.toml`, code 5 raw). On every one
of these titles the main DB stores a league **code** on the teams and no name; the game's own
label source is **unresolved** — `nhl_eng_us.db` is the obvious candidate (it decodes with
`tdb-loc`: CRCs, UTF-8, hashes; no reference reader), but which key, if any, a given code resolves
to, and whether some labels come from UI data or the executable, has not been traced. An identity
*edit* therefore does not exist on these targets today; their identity experiment is a Phase 1.3
question. Packaging (offline artifact): PS3 loose `USRDIR/nhlng.db` (S2-C) and the 16-byte
`patch.big` stager (S2-A) are automated-tested on synthetic/PS3 material; Xbox 360 and Recompiled
packaging is **not established**. Desktop: NHL 14 PS3 only (SMOKE-1 §3.5, #36–39, copied fixture).

| Target | Shared measurement | Editor (offline) | Packaging (offline artifact) | Desktop | In-game |
|---|---|---|---|---|---|
| NHL Legacy PS3 | `M-legacy` | browse/classify automated-tested; no identity edit (blocked → 1.3) | PS3 loose + `patch.big` stager (synthetic/PS3 tests) | none | not executed |
| NHL Legacy Xbox 360 | `M-legacy` | same | not established | none | not executed |
| Legacy Recompiled | by reference to `M-legacy`; **missing-fixture** | blocked row | not established | none | not executed |
| NHL 15 PS3 | `M-12-15` | browse/classify automated-tested; no identity edit | PS3 loose path | none | not executed |
| NHL 15 Xbox 360 | `M-12-15` | same | not established | none | not executed |
| NHL 14 PS3 | `M-12-15` | same | PS3 loose path | developer observation (SMOKE-1 §3.5) | not executed |
| NHL 14 Xbox 360 | `M-12-15` | same | not established | none | not executed |
| NHL 13 PS3 | `M-12-15` | same | PS3 loose path | none | not executed |
| NHL 13 Xbox 360 | `M-12-15` | same | not established | none | not executed |
| NHL 12 PS3 | `M-12-15` | same | PS3 loose path | none | not executed |
| NHL 12 Xbox 360 | `M-12-15` | same | not established | none | not executed |
| NHL 11 PS3 | `M-11` (candidate) | same; save acceptance open | PS3 loose path | none | not executed |
| NHL 11 Xbox 360 | `M-11` (candidate) | same | not established | none | not executed |
| NHL 10 PS3 | `M-10` (candidate; code 9 raw) | same; save acceptance open | PS3 loose path | none | not executed |
| NHL 10 Xbox 360 | `M-10` (candidate; code 9 raw) | same | not established | none | not executed |
| NHL 09 PS3 | `M-09` (candidate; code 8 raw) | same | PS3 loose path | none | not executed |
| NHL 09 Xbox 360 | `M-09` (candidate; code 8 raw) | same | not established | none | not executed |
| NHL 08 PS3 | `M-08` (candidate) | same | PS3 loose path | none | not executed |
| NHL 08 Xbox 360 | `M-08` (candidate) | same | not established | none | not executed |
| NHL 07 Xbox 360 | `M-07` (candidate; code 5 raw) | same | not established | none | not executed |
| NHL 07 PS3 | research-only (`M-07`) | supplemental campaign row | — | — | out of scope |

### 0.3 NHL 21 provenance chain (verified read-only in L1.1-Q1; payload identities independently re-hashed by Codex)

| Item | Identity |
|---|---|
| Installed game | `H:\Emulators\ShadPS4\Games\CUSA18182`, `param.sfo`: TITLE_ID `CUSA18182`, CONTENT_ID `EP0006-CUSA18182_00-NHLICEHOCKEY2021`, APP_VER `01.00`, VERSION `01.01`; no update/patch folder found beside the game folder |
| `databigs/inxp_00.big` | 777 373 328 B, SHA-256 `1c41f578a595dbd5aae7ad921445626636f3a842d8fb869bc054d35e3e849e87` (= the SMOKE-1/RESP-1 copy) |
| member 2652 `nhl/data/common/gamemodes/database/t3db/nhl_game.db` | raw, 11 209 672 B at 726 230 496, `90ee1ec49d36…` **= anchor** |
| member 1479 `…/t3db/nhl_game-meta.xml` | raw, `0b262e4abafd…` = anchor |
| member 1156 `nhl/data/kettle/loc/eng_us.db` | raw, 3 921 060 B at 746 617 088, `24693e2f1ae87bde23066d083458bae2efea9a633269518f4fede2354fbac969` = `databigs/Extracted/…/loc/eng_us.db` = the localization handoff's recorded fingerprint |
| member 2888 `nhl/data/kettle/loc/eng_us-meta.xml` | raw, `54febbf9471b…` = anchor |
| Occurrence | `nhl_game.db` and `loc/eng_us.db` occur **once** across all 16 `databigs/*.big` (front tables scanned, developer) |

NHL 21's own dictionary vs the NHL 20/18 proxies used in §1.4 — **the proxy was wrong on four
league keys**, which is why no sibling dictionary may stand in for a title:

| key | NHL 21 (own) | NHL 20 / NHL 18 anchors |
|---|---|---|
| `TXT_LEAGUE_EBEL` | "ICE Hockey League" | "Erste Bank Eishockey Liga" |
| `TXT_LEAGUE_NHL_ALL_TIME` | "NHL Alumni All-time" | absent |
| `TXT_USA_LEAGUE` / `TXT_RUSSIA_LEAGUE` | "USA" / "RUS" | absent |
| `Allsvenskan` / `TXT_ROW_LEAGUE` | "HockeyAllsvenskan" / "Rest Of World" | same |
| `AHL`, `CUSTOM_TEAMS`, `HUT`, `TXT_LEAGUE_MASCOT`, `TXT_LEAGUE_3ON3`, `TXT_LEAGUE_PROAM` | absent | absent |

A schema-level **lead, not a trace**: `boot.big` member 3730
`nhl/data/common/gamemodes/service-defs/view/roster/leagues_and_teams.json` declares a UI roster
service output `LeagueList[]{LeagueId, LeagueName "localized string for league name",
LeagueArtId, OrderId, TeamGroupList[]{GroupName ≤20, ItemList[]{ItemId, ItemArtId, ItemName}}}`
(max 20 leagues). A schema that *declares* a localized string does not say where or how the
localization happens, which key it uses, or whether `stockleagues.name` is the input. It is
recorded as a starting point for the Phase 1.2/1.3 consumer trace and nothing more.

## 1. NHL 21 PS4 — what "league identity" actually is

### 1.1 Fixtures used

| File | SHA-256 | Notes |
|---|---|---|
| `anchors/ps4/nhl21/databases/nhl_game.db` | `90ee1ec49d36dbb0f6d9cefe75e259d9661e3aa05338d03347b4842e33c8e9f9` | little-endian TDB v6, 115 tables, size and header CRC consistent |
| `anchors/ps4/nhl21/databases/nhl_game-meta.xml` | `0b262e4abafdf5fa166eb7fb99c7c5419d521379de63f37aa97f6654e8a4991d` | field names/ranges/update flags |
| `anchors/ps4/nhl21/databases/eng_us-meta.xml` | `54febbf9471b9a08270923426442852b1a4a04c8837f2e1236401568f000749b` | schema only — **`eng_us.db` is absent for NHL 21** ([anchors-manifest](../audits/anchors-manifest.md)) |
| `anchors/ps4/nhl20/databases/eng_us.db` | `62260005fcb02a1b5fc6dfeea9b98b25d49e28a1d92a0fbb829ebf8fb167a0f2` | nearest dictionary fixture (37 734 rows); NHL 19/20 anchor provenance unresolved (C0-REG4-R1) |
| `anchors/ps4/nhl18/databases/eng_us.db` | `12f1af46906c064670f3b536ef3876b6fb8921ac620e88785d359ff17ea7159f` | attributed dictionary fixture (33 396 rows) |
| `anchors/ps4/nhl15/databases/eng_us.db` | `fa65b9af56b51ab6fa10253dcb0d9b5cca48ea930d64ea6352ea916b35945017` | attributed dictionary fixture (24 251 rows) |

No NHL 21 ROSTER save is committed (missing-fixture, C0-REG4). `Season0`/`BeAGM0` PS4 saves exist
but were not opened for this package (mode-local league copies are Phase 1.2 scope).

### 1.2 The league entity: `stockleagues` (`fsLK`) — verified

NHL 21 has a real league table. Measured on the anchor (table block `0x63edc4`, record block
`0x63ee5c`, 36-byte records, 31/31 rows, `savegroups="allloaded"` only, no `maxinsert`):

| Field | Meta name | Kind / width | Bit offset | Meta flags | Classification |
|---|---|---|---|---|---|
| `aQrQ` | `leagueid` | uint 5 | 253 | `key="True"` | **primary key**, 0..30, equals row index on the anchor |
| `vIys` | `name` | inline string, 200 bits = **25 bytes** | 48 | (no `update` attr) | **localization key**, not display text (see 1.4) |
| `lOnr` | `artid` | uint 5 | 258 | — | asset id (league logo/tile family — Phase 1.4) |
| `kzyu` | `renderid` | uint 5 | 263 | — | asset/presentation id, semantics **unknown** |
| `XMjH` | `orderid` | uint 5 | 248 | — | menu ordering candidate (values are a permutation of 0..30) |
| `uhyr` | `primarycolor` | uint 24 | 24 | — | RGB colour |
| `hGlL` | `secondarycolor` | uint 24 | 0 | — | RGB colour |

Full row dump (anchor order = `leagueid`):

```text
id name                      order art render   id name                      order art render
 0 NHL                          0   0   0       16 CUSTOM_TEAMS                21   5  13
 1 AHL                          1   1   1       17 TXT_EASHL                   22   5  30
 2 elitserien                   3   2   2       18 TXT_LEAGUE_MASCOT           19   0   0
 3 SMLIIGA                      4   3   3       19 TXT_LEAGUE_3ON3             20  22   0
 4 TXT_LEAGUE_DEL               5   6   4       20 TXT_SPENGLER_CUP            10  21  24
 5 TXT_LEAGUE_O2_EXTRALIGA      6   7   5       21 TXT_LEAGUE_NHL_ALUMNI       17  25   0
 6 TXT_LEAGUE_SWISS_LEAGUE      7  10   6       22 TXT_LEAGUE_NHL_ALL_TIME     18  25   0
 7 TXT_LEAGUE_EBEL              8  18  23       23 TXT_LEAGUE_PROAM            23   5  13
 8 Allsvenskan                 11  19  22       24 CUSTOM_TEAMS                24   5  13
 9 TXT_CHAMPIONS_LEAGUE         9  20  21       25 CUSTOM_TEAMS                25   5  13
10 TXT_INTERNATIONAL           12   4   7       26 CHL                         26  14  11
11 OHL                         13  12   8       27 TXT_USA_LEAGUE              27  23  17
12 QMJHL                       14  13   9       28 TXT_RUSSIA_LEAGUE           28  24  18
13 WHL                         15  11  10       29 TXT_ROW_LEAGUE              29   5   7
14 PROSPECT_TEAMS              16  14  11       30 HUT                         30   5  20
15 TXT_ECHL                     2  17  16
```

Studio's metadata layer treats every `fsLK` field as updatable (`tdb-defs` sets
`updatable = attribute("update") != "False"`; these fields carry no attribute) and refuses the key
(`prepare_field_edit` rejects `meta.key`). The anchor-capacity audit's `ro` mark on this table means
the *game* declares no insert/update/delete diff caps, not that Studio refuses it.

### 1.3 References to the league — verified on the anchor

* **`exhibitionteams.league` (`kOtt.xMjj`, uint 5, `update="True"`) is a foreign key to
  `stockleagues.leagueid`.** Cross-tabulating the 433 team rows by `xMjj` against `fsLK` gives
  real-league cardinalities under the matching name for every populated code: 0 NHL (35 rows =
  31 clubs + all-star/extra rows), 1 AHL (31), 2 elitserien (14), 3 SMLIIGA (15), 4 DEL (14),
  5 Extraliga (14), 6 Swiss (12), 7 EBEL (11), 8 Allsvenskan (14), 9 Champions (8),
  10 International (21), 11 OHL (20), 12 QMJHL (18), 13 WHL (22), 14 Prospects (2), 15 ECHL (26),
  17 EASHL (12), 18 Mascot (2), 19 3on3 (2), 20 Spengler (6), 21 NHL Alumni (33), 22 All-Time (4),
  23 Pro-Am (97). Codes 16, 24–30 have no team rows on the anchor.
* **`kOtt.leaguegroup` / `conferencegroup` / `divisiongroup` (`iIBl` / `OwTu` / `CGMK`, uint 7)**
  form a separate hierarchical *group-id* namespace (e.g. NHL teams: leaguegroup 1, conferences 6/7,
  divisions 8–11; EBEL teams: 62/62/62). **No table in `nhl_game.db` names these groups** (searched
  every table and field name for `group`); their labels are a localization/UI concern (1.2/1.3),
  and they are *not* the league identity.
* Other tables carrying `xMjj`: the mode-local `leagueteams` (`RGbs`, 0 rows on the anchor, also
  carries `leagueindex` `bnSF` 5-bit −1..30) and the career-stats tables (1-bit NHL/AHL flag,
  `defs/enums/career-league.toml`). Mode-local copies are Phase 1.2.
* `stockleagues.artid` / `renderid` bind to presentation assets (1.4); `primarycolor` /
  `secondarycolor` are presentation values (1.4/5.x). Consumers not traced here.

### 1.4 `name` is a dictionary key — verified offline, runtime unverified

> **Dated assessment (L1.1-A).** The NHL 20/18/15 columns below were a proxy; NHL 21's own dictionary is now verified (§0.3) and differs on four league keys. "Studio cannot read any dictionary today" was true when written; `tdb-loc` and `attach_dictionary` exist since the overnight run.

The `name` column mixes literal-looking values (`NHL`, `AHL`, `OHL`, `elitserien`) with obvious
tokens (`TXT_LEAGUE_DEL`). The user's NHL 21 localization handoff (cited by the
[knowledge-gap register](builder-knowledge-gap-register.md), K14) already established on the
CUSA18182 snapshot that the main DB's league name is looked up as a **bare dictionary key**:
`TXT_LEAGUE_NHL_ALUMNI` → "NHL Alumni Association", `NHL` → "NHL®".

This package re-derived that on the repository's own PS4 dictionaries with the handoff's reference
reader (`tool.mjs verify/export`; all four CRCs match on NHL 15/16/17/18/20):

| `stockleagues.name` | NHL 20 `eng_us.db` | NHL 18 | NHL 15 |
|---|---|---|---|
| `NHL` | NHL® | NHL® | NHL® |
| `elitserien` | SHL | SHL | SHL |
| `SMLIIGA` | Liiga | Liiga | Liiga |
| `TXT_LEAGUE_DEL` | DEL | DEL | DEL |
| `TXT_LEAGUE_O2_EXTRALIGA` | Extraliga ledniho hokeje | same | same |
| `TXT_LEAGUE_SWISS_LEAGUE` | National League | same | same |
| `TXT_LEAGUE_EBEL` | Erste Bank Eishockey Liga | same | (absent — league not in NHL 15) |
| `Allsvenskan` | HockeyAllsvenskan | same | (absent) |
| `TXT_CHAMPIONS_LEAGUE` | Champions Hockey League | same | (absent) |
| `TXT_INTERNATIONAL` | International | same | same |
| `OHL` / `QMJHL` / `WHL` / `CHL` | OHL / QMJHL / WHL / CHL | same | same |
| `PROSPECT_TEAMS` | Prospect Teams | same | same |
| `TXT_ECHL` | ECHL | ECHL | (absent) |
| `TXT_EASHL` | EA SPORTS Hockey League | same | (absent) |
| `TXT_SPENGLER_CUP` | Spengler Cup | same | (absent) |
| `TXT_LEAGUE_NHL_ALUMNI` | NHL Alumni Association | (absent) | (absent) |
| `TXT_ROW_LEAGUE` | Rest Of World | same | same |
| `TXT_LEGEND_TEAMS` / `UnitedStates` / `Russia` / `WorldCup` / `TXT_LEAGUE_NORWAY` | present | present | partly |
| `AHL`, `CUSTOM_TEAMS`, `HUT`, `TXT_LEAGUE_MASCOT`, `TXT_LEAGUE_3ON3`, `TXT_LEAGUE_NHL_ALL_TIME`, `TXT_LEAGUE_PROAM`, `TXT_USA_LEAGUE`, `TXT_RUSSIA_LEAGUE` | **absent as keys** | absent | absent |

Consequences:

* The displayed league name on NHL 21 is **localization-driven**. Writing a new literal such as
  `Legends League` into `name` only shows on screen if the runtime falls back to the literal when
  the key is missing — **unknown**. The absent keys above (`AHL`, `HUT`, …) suggest a literal
  fallback exists, but the NHL 21 dictionary is not in the repo and nothing was run in game.
* A real rename therefore lands in **Phase 1.3** (edit the text under the existing key, or add a key
  and point `name` at it). Phase 1.1 can only (a) re-point `name` at another *existing* key, or (b)
  change the non-text identity fields (`orderid`, colours, `artid`/`renderid`).
* `name` is 25 bytes on disk; dictionary keys may be up to 100 bytes. A key that does not fit in 25
  bytes cannot be referenced from this table.
* Studio cannot read any dictionary today: `tdb-core` decodes inline strings (kind 0) only; the
  kind 13/14 Huffman varchar pool (`03-tdb-format.md` "port faithfully") is **not implemented**.
  The handoff's `reader.mjs` is the verified reference for the little-endian PS4 layout.

### 1.5 Runtime location and deployment — verified location, blocked writer

> **Dated assessment (L1.1-A).** The PS4 20-byte-row stager now exists (`tdb-archive-big::stage`, O7) and was exercised on a copy of the real archive; the output is still structurally unqualified and loose-vs-archive precedence is still unproven (§0.2, runtime cards).

`nhl_game.db` lives inside `inxp_00.big` under `nhl/data/common/gamemodes/…`
([ps4-eb-big](../formats/ps4-eb-big.md)); the dictionaries under `nhl/data/kettle/loc/`. The S2-A
stager writes **16-byte-row** `EB\0\3` archives (PS3 Legacy `patch.big` layout) only; the PS4
20-byte-row writer does not exist, and loose-file override precedence on PS4 is unproven
(N19-H1 spec). So on every PS4 target an edited `nhl_game.db` can be produced and saved to a copied
tree, but **installing it for an in-game check is blocked** until either the PS4 archive writer or
a proven loose-override path exists. This blocker applies to all Phase 1 PS4 work, not just leagues.

---

## 2. Studio today — how league identity flows through the code

> **Dated assessment (L1.1-A, pre-implementation).** The gaps in this table (no league view, no Leagues page, no pending list, no copied-target UI, global `league.toml`) were closed by the overnight run and its corrections; see §0 and the receipts.

| Layer | What exists | League-specific behaviour |
|---|---|---|
| Metadata (`tdb-defs`) | per-field name/range/key/updatable from `-meta.xml` | `fsLK` fields updatable except key; `kOtt.xMjj` updatable, range 0..31 on NHL 21 |
| Enums (`tdb-enums`, `defs/enums/league.toml`) | one global `(kOtt, xMjj)` vocabulary, **no title gating** | labels 0..15 taken from a 252-team DB = **the Legacy anchor** (NHL 14 has 246). Correct for Legacy and NHL 16 PS4 only — see §3.3 |
| Domain read (`view.rs`) | `team_summary_at` reads `xMjj` → `league_label` via the enum; `TEAM_IDENTITY` lists `leaguegroup/divisiongroup/conferencegroup`; `entity_fields(table, record)` is generic | **no league entity view**; `fsLK` is never read |
| Domain edit (`session.rs`) | generic `set_field(studio_table, record, studio_field, value)` → validated, audited, undoable; undo stack = whole-DB byte snapshots | would accept `fsLK.vIys` (≤25 bytes) and the numeric fields; refuses `aQrQ` (key). Nothing calls it for leagues |
| Field profile (`field_profile.rs:414`) | Team Data "Classification → League" cell binds `["leaguegroup","league"]` (first present wins) | on NHL 12+ (all targets with `iIBl`) the cell edits **`leaguegroup`**, an unlabeled group id — while the header shows the **`xMjj`** enum label. Two fields under one name |
| API (`studio-api`) | `set_field`, `list_teams`, `team_view`, `undo/redo`, `save_project` (save-as), `save_workspace` (in place), S2-C `copied_target_*` (review → apply → status/recover/restore/reload) | no league commands; `edit-qa --spec` resolves player/team entities only |
| UI (`app/src`) | `TeamHeader` classification = `xMjj · OwTu · CGMK` enum labels; `Transactions` league picker groups teams by `xMjj` with `league_label ?? "League N"`; `PendingEdits` shows dirty flag + undo/redo depth, Save in place, Save as | **no Leagues page**, no per-change pending list, no copied-target UI (S2-C shipped API only) |

---

## 3. Target matrix — all 27 shipping targets

### 3.1 Byte identity and fixture reality (verified)

* Every PS3 and Xbox 360 main-DB anchor pair is **byte-identical** per title (NHL 07–15, Legacy;
  already recorded in [anchor-capacity-and-editable-tables](../audits/anchor-capacity-and-editable-tables.md)).
  Each pair therefore yields one measurement and two targets; runtime placement, packaging and
  in-game behaviour still need separate evidence per platform.
* NHL 19 and NHL 20 PS4 anchors are byte-identical with unresolved provenance (C0-REG4-R1): their
  rows below are **candidates**.
* NHL 07 PS3 is research-only. Legacy Recompiled has **no anchor**; it inherits Legacy Xbox 360's
  schema by reference only and is **untested** on every row.

### 3.2 Where league identity lives, per schema generation

| Generation / targets | League table | Team league code | Team conference / division | League display name | Studio disposition today |
|---|---|---|---|---|---|
| **gen6 — NHL 07 X360** (PS3 research-only) | none | `stockteamtable.league` (`HqzU.xMjj`, uint 3, no update attr) — 0 NHL 32, 1 National 21, 2 SEL 12, 3 SM-liiga 13, 4 DEL 14, 5 "DAV" 1 (candidate: Spengler/Davos); `customteamtable` (`fLEY`) carries the same field, 0 rows | `HqzU.conference` 1-bit, `kOtt.division` 3-bit | not in the DB; `nhl_eng_us.db` (big-endian dictionary) — undecoded | team-only read; classification cell edits `league` raw on the stock table; **no league name to edit** |
| **gen7 early — NHL 08 PS3/X360** | none | `kOtt.xMjj` uint 3 — 0 NHL 32, 1 AHL 29, 2 SEL 12, 3 SM-liiga 14, 4 National 21, 5 blank-abbr 30 (custom slots) | `rasz` conference 1-bit, `xgeX` division 3-bit | not in the DB; `nhl_eng_us.db` undecoded | same |
| **NHL 09 PS3/X360** | none | `kOtt.xMjj` uint 4 — 0 NHL, 1 AHL, 2 SEL, 3 SM-liiga, 4 DEL 15, 5 Extraliga 14, 6 Russian/KHL 20 (candidate from abbrs MVD/CSKA/DYN), 7 National 21, 8 "CEN" 1 (unknown), 9 blank 30 | same | same | same |
| **NHL 10 PS3/X360** | none | uint 4 — 0 NHL, 1 AHL, 2 SEL, 3 SM-liiga, 4 DEL, 5 Extraliga, 6 NLA 12, 7 National 21, 8 "CEN" 1, 9 NAJ/WPR 2 (unknown pair), 10 blank 30 | same | same | same |
| **NHL 11 PS3/X360** | none | uint 4 — 0 NHL, 1 AHL 30, 2 SEL, 3 SM-liiga, 4 DEL, 5 Extraliga, 6 NLA, 7 National, 8 OHL 20, 9 QMJHL 18, 10 WHL 22, 11 Top Prospects 2 (TPR/TPW), 12 blank 30 | same | same | same |
| **NHL 12 / 13 / 14 / 15 PS3/X360** and **NHL 15 PS4** | none on gen7; **`fsLK` (20 rows) on NHL 15 PS4** | uint 4 — 0 NHL, 1 AHL, 2 SEL, 3 SM-liiga, 4 DEL, 5 Extraliga, 6 NLA, 7 National, 8 OHL, 9 QMJHL, 10 WHL, 11 Top Prospects, 12 Legends (1–3 rows), 13 blank 30, 14 Winter Classic (2–4 NHL rows). NHL 15 PS4 `fsLK` names agree: 7 `NATIONAL`, 8 `OHL`, 12 `TXT_LEGEND_TEAMS`, 13 `CUSTOM_TEAMS`, 14 `NHL` (art 0), 15 `CHL`, 16 `UnitedStates`, 17 `Russia`, 18 `TXT_ROW_LEAGUE`, 19 `HUT` | `iIBl`/`OwTu`/`CGMK` uint 6 group ids (from NHL 12) | gen7: `nhl_eng_us.db` undecoded; NHL 15 PS4: `eng_us.db` present (keys verified §1.4) | gen7: team-only; NHL 15 PS4: `fsLK.name` editable via generic path, no UI |
| **NHL Legacy PS3/X360**, **Legacy Recompiled** (by reference, untested), **NHL 16 PS4** | none on Legacy; **`fsLK` (22 rows) on NHL 16** | uint 4 (Legacy) / uint 5 (NHL 16) — 0 NHL, 1 AHL, 2 SHL 14, 3 Liiga 15, 4 DEL, 5 Extraliga, 6 NLA, **7 Norway 2**, 8 National 21, 9 OHL, 10 QMJHL, 11 WHL, 12 Top Prospects, 13 blank 30, 14 Winter Classic 4, 15 EASHL 2. NHL 16 `fsLK` agrees (7 `TXT_LEAGUE_NORWAY`, 15 `TXT_EASHL`, 16 `CHL` … 21 `TXT_LEGEND_TEAMS`) | uint 6 group ids | Legacy: `nhl_eng_us.db` undecoded; NHL 16: `eng_us.db` present | **this is the layout `league.toml` labels correctly** |
| **NHL 17 PS4** | `fsLK` 24 rows | uint 5 — 0..12 as NHL 16, 13 `TXT_ECHL` 27, 14 `WorldCup` 8, 15 `CUSTOM_TEAMS`, 16 `NHL`, 17 `TXT_EASHL` 5, 18 `CHL`, 19 US, 20 Russia, 21 ROW, 22 `HUT`, 23 `TXT_LEGEND_TEAMS`; `orderid` appears | uint 6 group ids; `teamtype` (`edFE`, no update attr) appears | `eng_us.db` present | as NHL 15 PS4 |
| **NHL 18 PS4** | `fsLK` 26 rows | uint 5 — 0 NHL 35, 1 AHL 30, 2 SHL, 3 Liiga, 4 DEL, 5 Extraliga, 6 NLA, **7 EBEL 12, 8 Allsvenskan 14, 9 Champions 8**, 10 International 21, 11 OHL, 12 QMJHL, 13 WHL, 14 Prospects, 15 ECHL 27, 16 Custom, 17 EASHL 8, 18 Mascot 2, 19 3on3 2, 20 Spengler 6, 21 CHL, 22 US, 23 Russia, 24 ROW, 25 HUT; `secondarycolor`/`primarycolor` present | uint 7 group ids | `eng_us.db` present | as NHL 15 PS4 |
| **NHL 19 PS4 / NHL 20 PS4** (identical bytes, provenance unresolved → candidate) | `fsLK` 29 rows | as NHL 18 plus 21 `TXT_LEAGUE_NHL_ALUMNI` 33, 22 `TXT_LEAGUE_NHL_ALL_TIME` 5, 23 `TXT_LEAGUE_PROAM` 80; `kOtt` 419/519 rows | uint 7 | NHL 19: **`eng_us.db` missing-fixture**; NHL 20: present | as NHL 15 PS4 |
| **NHL 21 PS4** | `fsLK` 31 rows (§1) | uint 5, 0..23 populated (§1.3) | uint 7 | **`eng_us.db` missing-fixture** (NHL 20's used as proxy) | as NHL 15 PS4 |

Cardinalities are anchor measurements; the league *identities* attached to gen7 codes are
**candidates** inferred from member-team abbreviations (e.g. code 6 on NHL 09 = MVD/CSKA/DYN/AMUR
→ Russian league) and are only *verified* where a PS4 `stockleagues.name` on the same layout names
the code (NHL 12–15 ↔ NHL 15 PS4; Legacy ↔ NHL 16 PS4).

### 3.3 Existing defect: `defs/enums/league.toml` mislabels most titles (verified)

The enum's vocabulary (7 GET-Ligaen, 8 National, 9 OHL, 10 QMJHL, 11 WHL, 12 Top Prospects,
13 Custom, 14 Winter Classic, 15 EASHL) matches **Legacy / NHL 16 PS4** and is wrong for:

| Titles | Wrong labels today (examples) |
|---|---|
| NHL 08 | 4 National shown as "DEL"; 5 custom shown as "Extraliga" |
| NHL 09–10 | NHL 09: 6 Russian league shown as "NLA"; both: 7 National shown as "GET-Ligaen", 8 "CEN" shown as "National Teams"; NHL 10: 9 NAJ/WPR shown as "OHL", 10 custom shown as "QMJHL" |
| NHL 11 | 8 OHL shown as "National Teams", 9 QMJHL as "OHL", 10 WHL as "QMJHL", 11 Prospects as "WHL", 12 custom as "Top Prospects" |
| NHL 12–15 (PS3, X360) and NHL 15 PS4 | 7 National shown as "GET-Ligaen", 8 OHL as "National", 9 QMJHL as "OHL", 10 WHL as "QMJHL", 11 Prospects as "WHL", 12 Legends as "Top Prospects" |
| NHL 17 PS4 | 13 ECHL as "Custom", 14 World Cup as "Winter Classic", 15 Custom as "EASHL", 16–23 unlabeled (`League 16` … in the Transactions picker) |
| NHL 18–21 PS4 | 7 EBEL as "GET-Ligaen", 8 Allsvenskan as "National", 9 Champions as "OHL", 10 International as "QMJHL", 11 OHL as "WHL", 12 QMJHL as "Top Prospects", 13 WHL as "Custom", 14 Prospects as "Winter Classic", 15 ECHL as "EASHL", 17–30 unlabeled |

Surfaces affected: Team header classification line, Transactions league picker, any
`league_label` consumer. On PS4 titles the correct label source is the target's own
`stockleagues.name` (plus the dictionary for display text); on gen7 titles it must be a per-title
vocabulary. This is a Hard-rule-6 (human-readable surfaces) defect and is the first thing L1.1-B
should fix, because the roadmap's "appears on the intended surfaces" check cannot be read from a
mislabelled picker.

### 3.4 Per-target disposition for a **direct league identity edit** (Phase 1.1 scope)

> **Dated assessment (L1.1-A).** "Save/reopen path — no UI" and "PS4 archive writer / loose override unproven; dictionary missing-fixture" describe the pre-overnight state. Now: Leagues page + pending review + copied-target UI exist; the PS4 stager exists (unqualified); the NHL 21 dictionary is available from the install (§0.3) but not a committed fixture. The runtime blocker itself (precedence/acceptance) is unchanged.

| Target | Directly editable identity value in the main DB | Save/reopen path in Studio | In-game persistence path | Disposition |
|---|---|---|---|---|
| NHL 21 PS4 | `fsLK.name` (key re-point), `orderid`, colours, `artid`/`renderid` | generic `set_field` → `save_project`/`save_workspace`/S2-C copied-target (API) — no UI | **blocked**: PS4 archive writer / loose override unproven; dictionary missing-fixture | implementable in Studio (DB side); runtime **blocked** |
| NHL 20, 18, 17, 16, 15 PS4 | same fields (NHL 15 lacks `orderid`/colours/`renderid`; NHL 16 lacks `orderid`/colours) | same | same blocker; NHL 15 ROSTER decode blocked (irrelevant to leagues) | same; NHL 19/20 rows are candidates until provenance is resolved |
| NHL 19 PS4 | same | same | same + dictionary missing-fixture | candidate / blocked |
| NHL Legacy PS3, X360 | **none** — league name is not stored in `nhlng.db`; only the team-side `xMjj` code (membership = 1.2) | n/a for identity; team edits save today (`USRDIR/nhlng.db` PS3 layout per S2-C) | display name in `nhl_eng_us.db` (BE Huffman pool, undecoded) | **blocked pending 1.3 reader**; record as "no direct identity field" |
| Legacy Recompiled | as Legacy by reference | no anchor, no adapter | unknown | **untested** |
| NHL 15, 14, 13, 12, 11, 10, 09, 08 PS3 / X360 | none (as Legacy) | as Legacy | as Legacy; NHL 10/11 save acceptance open (K-register) | **blocked pending 1.3 reader** |
| NHL 07 X360 | none (`HqzU.xMjj` code only) | team edits via stock-table join | `nhl_eng_us.db` undecoded | blocked pending 1.3 reader |
| NHL 07 PS3 | research-only | — | — | out of scope |

Bottom line: a **directly editable league identity value exists only on the seven PS4 titles**
(`stockleagues`). On the twenty gen6/gen7 targets (PS3, Xbox 360, Recompiled) the league is a
code plus a dictionary string, so Phase 1.1's "identity edit" collapses into Phase 1.3 there. That
is a finding about the games, not a scope reduction; it must be recorded per target as
"identity is localization-only".

---

## 4. The concrete NHL 21 change (step 3 of the dispatch)

**Chosen league:** `leagueid 8` — `Allsvenskan` (record 8 at `0x63ef7c`, 14 member teams, records
265–278, all with `leaguegroup 63`). Chosen because its `name` is a literal-looking key whose
dictionary text differs from the key ("HockeyAllsvenskan"), so a display change is unambiguous, and
because it has no conference/division sub-structure to disturb.

Record bytes (before): `1717173264d4416c6c7376656e736b616e00000000000000000000000000000b4d0b0000`
(`secondarycolor 0x171717`, `primarycolor 0xD46432`, `name "Allsvenskan"`, `order 11`, `id 8`,
`art 19`, `render 22`).

**Minimal before/after identity edit that Phase 1.1 can make safely (no localization):**
re-point `fsLK[8].name` from `Allsvenskan` to the *existing* key `TXT_ROW_LEAGUE` ("Rest Of
World" on NHL 15/18/20). Keys unchanged (`leagueid 8`), topology unchanged (14 teams keep
`xMjj = 8`), no other field touched.

* Expected Studio behaviour: the league row shows name key `TXT_ROW_LEAGUE`; the pending list shows
  one cell change `stockleagues[8].name: Allsvenskan → TXT_ROW_LEAGUE`; save → reopen shows the new
  value; the allowed-difference audit passes (only `fsLK` record 8 differs at record level;
  header/CRC bytes may move; length unchanged); restore returns the anchor hash. *(Implemented and
  developer-verified offline in the overnight run — see the campaign record.)*
* Expected in-game behaviour (**prediction, unverifiable in this package**): wherever the league
  label for the Allsvenskan teams is rendered from `stockleagues.name` (league selection tiles, team
  select filter), "Rest Of World" appears instead of "HockeyAllsvenskan". Any screen that still
  says "HockeyAllsvenskan" is a consumer *not* driven by this field (schedule/mode tables, UI data,
  or the `leaguegroup` namespace) and must be recorded for 1.2/1.3.
* Why not write `Legends League` directly: it would test literal fallback, which is unknown; a
  first run should separate "field is consulted" from "literal fallback exists". The literal test is
  the natural *second* run once the first passes.
* The desired *real* rename (any new text) depends on **Phase 1.3** (edit the dictionary text under
  `Allsvenskan`, or add a key ≤25 bytes and point `name` at it).

**Equivalent operation per target (semantics, never raw ids):**

| Target family | Equivalent of "re-point league 8's name key" |
|---|---|
| NHL 18/19/20 PS4 | same league: `stockleagues` row whose `name == "Allsvenskan"` (id 8 on these anchors, but resolve by name, not by 8) → `TXT_ROW_LEAGUE` |
| NHL 17 PS4 | no Allsvenskan; equivalent league = `TXT_LEAGUE_NORWAY` (id 7, a 2-team second-tier league) → `TXT_ROW_LEAGUE` |
| NHL 16 PS4 | `TXT_LEAGUE_NORWAY` (id 7) → `TXT_ROW_LEAGUE` |
| NHL 15 PS4 | no second-tier European league; nearest reversible row = `TXT_LEGEND_TEAMS` (id 12) → `TXT_ROW_LEAGUE` |
| Legacy PS3/X360/Recomp | Norway league = team code 7 (STAV, VAL). **No name field to edit**; the equivalent is a dictionary edit (1.3) — blocked |
| NHL 12–15 gen7 | no Norway/Allsvenskan; nearest = Legends code 12 — same blocker |
| NHL 08–11, NHL 07 | no equivalent league; same blocker |

---

## 5. Existing-workflow assessment (step 4) — gaps and smallest slice

> **Dated assessment (L1.1-A, pre-implementation) — kept as history.** The proposed L1.1-B package below was executed as O1–O7 of the overnight run and corrected by R1/R2/SMOKE-1/SMOKE-R1/RESP-1. Current per-target status is §0.2; the remaining open items are the runtime cards.

| Workflow stage | Exists today | Gap for a league identity edit | Smallest slice |
|---|---|---|---|
| Browse | Players / Teams / Arenas pages; no Leagues | no way to see `stockleagues` rows; team league labels wrong on most titles (§3.3) | `list_leagues` (PS4: `fsLK` rows with `leagueid`, name key, resolved label, member-team count; gen7: derived rows from distinct `xMjj` with per-title label) + a Leagues page (list + detail via `entity_fields`) |
| Edit | generic `set_field` (domain + API) | no UI cell for `fsLK`; Team "League" cell edits `leaguegroup` under the name "League" | league detail uses the existing generic field cells; rename the Team cell to "League group" and add a real `league` (`xMjj`) picker fed by `list_leagues` |
| Validate | metadata range/updatable/key checks; structural audit (`validate()`) | none for leagues — but `xMjj` has no FK relation def, so an out-of-set league code is not caught | add a `kOtt.xMjj → fsLK.aQrQ` relation on PS4 titles (`defs/relationships`), gated on table presence |
| Pending review | dirty flag + undo/redo depth only | **no list of what changed** — required by "one pending-changes review" | `pending_changes`: diff the **acknowledged baseline** (`Project::db_bytes`) vs `working` per record/field through the metadata, rendered with resolved names/labels; no new state *(corrected from the original `undo[0]` wording — undo snapshots are not the baseline; save → undo must read as pending)* |
| Save | `save_project` (save-as), `save_workspace` (in place), S2-C copied-target review/apply (API only) | copied-target has **no UI**; PendingEdits "Save as" writes a bare path | wire PendingEdits to `copied_target_review` → show plan → `copied_target_apply`; keep save-as |
| Reopen | `copied_target_reload`, `reload_from_disk`, rebind on apply | none | reuse |
| Restore | `copied_target_restore(root, txn)` (API) | no UI | reuse via the same page |
| Evidence scripts | `edit-qa --spec` (player/team entities only) | cannot address a league row | add an `entity: "league:<name key>"` resolver (script evidence only — not UI functionality) |

**Proposed bounded implementation package (L1.1-B, for Codex to issue):**

1. **Per-title league vocabulary** (Hard rule 6 defect): PS4 → labels from `stockleagues.name` with
   a dictionary-key → display-text map where a dictionary fixture exists, else the key token shown
   as a key (never as a name); gen7/gen6 → per-layout vocabularies (`NHL 08`, `NHL 09`, `NHL 10`,
   `NHL 11`, `NHL 12–15`, `Legacy/NHL 16`) selected by measured layout, with unverified codes shown
   as raw. Automated test: every anchor's populated codes resolve to a label consistent with §3.2.
2. **League read surface**: `LeagueSummaryDto` / `LeagueViewDto`, `list_leagues`, `league_view`,
   Leagues page (list + detail with generic field cells; member teams linked).
3. **Pending-changes list**: `pending_changes` command + PendingEdits table (table, record resolved
   to entity name, field name, before → after with labels), plus undo per entry is *not* required.
4. **Copied-target UI**: PendingEdits gains review → apply → status/restore using the S2-C API
   unchanged.
5. **Team cell correction**: "League" → `xMjj` picker; "League group" → `iIBl` raw.
6. **Evidence run** (scripted, PS4 anchors only): apply §4's edit to a copied NHL 21 tree, prove
   save/reopen and byte-level unrelated-data preservation, restore; repeat the equivalent row on
   NHL 15/16/17/18/20 (19/20 as candidates). Record the gen6/gen7 targets as
   "identity is localization-only — blocked pending 1.3" with the exact code/team evidence above.

Acceptance checks for that package: (a) `cargo test -p tdb-enums -p studio-domain -p studio-api`
including a new anchor-wide league-label test; (b) `export-bindings -- --check` in sync;
(c) copied-target apply → reopen → restore on the NHL 21 anchor copy with hashes recorded;
(d) screenshots or DTO dumps of the Leagues page and pending list for NHL 21 and one gen7 title;
(e) no in-game claim.

---

## 6. Unresolved questions (for research or Codex)

> **Status after L1.1-Q1:** 1 and 6 are the runtime experiments in [league-identity-runtime-qualification](league-identity-runtime-qualification.md); 2 has a schema-level lead (§0.3, `leagues_and_teams.json`); 3 stays Phase 1.4; 4 stays Phase 1.3 (the dictionaries decode, the code→key binding does not live in the database); 5 unchanged.

1. Does NHL 21 fall back to the literal `stockleagues.name` when the key is absent from the
   dictionary (`AHL`, `HUT`, `CUSTOM_TEAMS` are absent yet presumably display)? Runtime test needed.
2. Which screens consume `stockleagues.name` vs. the `leaguegroup`/`conferencegroup`/`divisiongroup`
   ids, and where are those group ids named (UI data outside the DB?).
3. `renderid` semantics and the `artid` → asset member binding (1.4).
4. Gen7 dictionary keys for league names (`nhl_eng_us.db`, big-endian Huffman pool): no reader in
   the repo and the handoff reader is LE-only — the 1.3 prerequisite for twenty targets.
5. NHL 09 code 8 "CEN" and NHL 10 code 9 "NAJ/WPR": unidentified small groups.
6. PS4 in-game persistence path for any DB edit (archive writer vs loose override).

# League-name parity batch — consolidated review matrix and runtime checklist (2026-09-18)

> **Package:** LEAGUE-PARITY-BATCH (developer preparation; no live replacement, no game launch,
> no staging/commit/push, no anchor replacement). Receipt:
> [league-parity-batch](../receipts/league-parity-batch.md). Package files:
> `local/runs/league-parity-batch/` (hash list `SHA256SUMS.txt`).
> **Superseded for readiness, candidates and cards (2026-09-18, completion queue):** the refreshed
> matrix, the 21 candidate rows and the ordered runtime cards are in
> [league-parity-completion-2026-09-18](league-parity-completion-2026-09-18.md); §1–§3 below remain
> the inventory and provenance source. REG-1 corrected the NHL 19/20 and PS3 08/09 rows; EB-V1
> unblocked NHL 15 PS4; LOC-W1 gave the gen7 rows a rename route.
> **NHL21-E1 is accepted as the comprehensive pass by the user's decision; nothing here reopens
> it.** Every other target below is *prepared*, not run. The runtime pass is a separate, user-driven
> session; this document is its input.
> Evidence vocabulary is CLAUDE.md's: *implemented / automated-tested / game-verified / blocked /
> missing-fixture*. **No target in this batch is game-verified.**

## 0. Plain-language summary

Every NHL title installed on this machine was identified from its own package data, and the exact
database, metadata and dictionary the game ships were located and hashed — inside the PS4
archives, on the PS3 discs and game-data installs, and in the Xbox 360 / recompiled trees. That
inventory changed the picture in four places: the repository's "NHL 20" database and dictionary
anchors are actually NHL 19's base-game files (so NHL 20 had no fixture at all until this batch
extracted its own); NHL 19 ships seven different databases (base plus six patches) and a stale
loose game tree; NHL 15 PS4 uses an older archive layout Studio cannot open and also ships a
patched database; and the PS3 NHL 08/09 dictionary anchors are the Xbox 360 files, not the PS3
disc's.

For the seven PS4 titles the same one-cell league rename that NHL21-E1 used (re-point one league's
name key at the existing "Rest Of World" key) was produced through Studio's own save path from
each title's own files, verified byte by byte, retained as a candidate archive with hashes, and the
copied working trees were restored exactly. Eight candidate archives exist (NHL 19 has two: base
and patch). NHL 15 PS4 has a loose candidate only, because its archives are the unsupported layout.
A single manifest-driven install/rollback helper (the NHL21-E1 state machine, unchanged; 33 fixture
cases pass) is ready for each of them, and its read-only status mode was run against every live
install: six are ready to place; NHL 19 is refused until a pre-existing loose game tree is
resolved.

For the twenty PS3 / Xbox 360 / Recompiled targets the league name is not in the database at all.
This batch traced where it comes from on NHL Legacy Xbox 360: a fixed table in the game
executable maps each league code to a dictionary key, and the dictionary supplies the text. The
exact same minimal operation as on PS4 is therefore a *dictionary* edit — point one key's text at
another existing string — which needs a writer Studio does not have. That missing capability is
specified as one bounded package (§7), with a byte-exact dry plan per installed title. Nothing was
written to any dictionary.

## 1. Installed-title identity inventory (read-only)

Sources: PS4 `sce_sys/param.sfo` (parsed), PS3 `PARAM.SFO`, RPCS3 `config/games.yml`, the archive
front tables (`EB\0\3`, both row layouts), and SHA-256 of every located database/metadata/dictionary.
Emulators: shadPS4 `H:\Emulators\ShadPS4\shadPS4.exe` (`aeb04892…`, the NHL21-E0/E1 fork — re-hashed
`PASS` by the helper today), RPCS3 `H:\Emulators\RPCS3\rpcs3.exe`, Xenia `H:\Emulators\Xenia\xenia.exe`
(build `master@95a5c3ee2`, 2026-02-18 per its log), recompiled `nhllegacy.exe` (`7264bec6…`).

### 1.1 PS4 (shadPS4 `H:\Emulators\ShadPS4\Games\<TITLE_ID>`)

| Title | TITLE_ID / CONTENT_ID | APP_VER · VERSION | Archive layout | DB member (`t3db/nhl_game.db`) | Dictionary member (`kettle/loc/eng_us.db`) | Base/patch |
|---|---|---|---|---|---|---|
| NHL 15 | CUSA00561 · EP0006-CUSA00561_00-NHLICEHOCKEY2015 | 01.00 · 01.06 | `0x00100400`, **16-byte rows** — **not a layout Studio binds** (`KNOWN_VERSION_TAGS` = `0x00110400`, `0x00000400`) | `inxp00.big` #0 raw 16 981 156 B `ae9d1d56…` **= anchor**; `patch_inxp00.big` #0 raw 16 981 204 B `abe36f58…` (**different, 48 B longer**) | `inxp00.big` #146 raw `fa65b9af…` = anchor; `patch_inxp00.big` #264 raw 1 658 908 B `1e873d7e…` (different) | **ambiguous**: `patch_*.big` ship in the base package; which DB the game loads is unverified |
| NHL 16 | CUSA01909 · EP0006-…-NHLICEHOCKEY2016 | 01.00 · 01.00 | `0x00110400`, 20-byte rows, 234 files | `inxp_00.big` #218 raw `7d95f1db…` = anchor | #86 raw `cb1f5d56…` = anchor | none (no patch archives) |
| NHL 17 | CUSA04326 · UP0006-…-NHLICEHOCKEY2017 | 01.00 · 01.00 | `0x00110400`, 1 185 files | `inxp_00.big` #1053 raw `de48b02b…` = anchor | #441 raw `035b3b30…` = anchor | none |
| NHL 18 | CUSA07569 · UP0006-…-NHLICEHOCKEY2018 | 01.00 · 01.00 | `0x00110400`, 1 354 files | `inxp_00.big` #1200 raw `65eaa6e8…` = anchor | #503 raw `12f1af46…` = anchor | none |
| NHL 19 | CUSA11126 · UP0006-…-NHLICEHOCKEY2019 | **01.60** · 01.00, CATEGORY `gp` | `0x00110400`; base `inxp_00.big` (2 329 files) **+ `patch_inxp_51..56.big`** | base #2089 raw `265e4a68…` **= the anchor labelled nhl19 *and* nhl20**; patch_51 `82d66970…`, 52 `7ed8b5fb…`, 53 `997ea05b…`, 54 `ea6f39e0…`, 55 `fcb4bfe8…`, **56 `61332e66…`** (10 318 668 B) — seven distinct databases | base #936 raw `62260005…` **= the anchor labelled nhl20**; patch_51..56 each carry their own (`a3a08ce7…` … `fbb8adcf…`) | **ambiguous**: the loader's patch precedence is unverified; **plus a stale loose game tree** (§2.4) |
| NHL 20 | CUSA15439 · UP0006-…-NHLICEHOCKEY2020 | 01.00 · 01.00 | `0x00110400`, 14 567 files | `inxp_00.big` #13224 raw 10 226 516 B `1e200478…` — **matches no anchor** (31 `stockleagues` rows, NHL 21-family layout) | #6245 raw 2 675 972 B `0fc95ec8…` — matches no anchor (40 535 rows) | none (13 archives, no patch) |
| NHL 21 | CUSA18182 · EP0006-…-NHLICEHOCKEY2021 | 01.00 · 01.01 | `0x00110400`, 2 950 files | `inxp_00.big` #2652 raw `90ee1ec4…` = anchor (E1 chain unchanged) | #1156 raw `24693e2f…` (hash-bound local source, not a fixture) | none |

Also present, out of catalog scope: CUSA26280 NHL 22, CUSA31046 NHL 23, CUSA37933 NHL 24 (+patch), CUSA02138 FIFA 16; duplicate copies under `H:\Emulators\games\PS4\` (same `param.sfo` hashes for 20/21). None used.

### 1.2 PS3 (RPCS3; disc trees under `X:\Games\Emulation\Roms\Sony Playstation 3\`, game-data under `H:\Emulators\RPCS3\dev_hdd0\game\`)

| Title | Disc id (games.yml) | Disc `USRDIR/db/nhlng.db` | Disc `USRDIR/fe/loc/nhl_eng_us.db` | `dev_hdd0/game/<id>/USRDIR` copy | Base/patch |
|---|---|---|---|---|---|
| NHL 08 | BLUS30062 (`NHL 08/`) | `587204c3…` = anchor; meta `d2b6feb2…` = anchor | **`b9fb3361…` (265 236 B) ≠ anchor `308021e7…`** (the anchor equals the Xbox 360 file) | none (no game-data install) | disc only; `PS3_UPDATE` holds a firmware PUP, no title update |
| NHL 09 | BLUS30168 (`NHL 09/`) | `0f7c826b…` = anchor (disc ships no `nhlng-meta.xml`; anchor meta used) | **`537319ce…` (511 324 B) ≠ anchor `d9e0463b…`** (= Xbox 360 anchor) | none | disc only |
| NHL 10 | BLUS30345 (`NHL 10/`) | `e87b8c80…` = anchor (no disc meta; anchor meta) | `954d1617…` = anchor | none | disc only |
| NHL 11 | BLUS30568 (`NHL 11/`) | `461f8d64…` = anchor; meta `a83aae92…` = anchor | `48e3cd43…` = anchor | `0.ver` only | disc only |
| NHL 12 | BLUS30791 — **registered as the `.iso`**; extracted folder `NHL 12/[BLUS 30791] NHL® 12/` inspected | `94b31ff2…` = anchor; meta = anchor | `3327bc3a…` = anchor | `0.ver` only | disc (ISO) only; the folder copy equals the anchors |
| NHL 13 | BLUS31003 (`NHL 13/`) | `64460a15…` = anchor | `69cc4f75…` = anchor | **`USRDIR/db` + `fe/loc` present**, byte-identical to the disc | **ambiguous**: game-data install duplicates the disc DB and dictionary; which copy the game reads is unverified (hypothesis: the hdd0 copy) |
| NHL 14 | BLUS31179 (`NHL 14/`) | `84e1bdff…` = anchor | `d7bf2a6f…` = anchor | present, identical to disc | same |
| NHL 15 | BLUS31436 (`NHL 15/`) | `028ae06a…` = anchor | `5f3f8f20…` = anchor | present, identical to disc | same |
| NHL Legacy | BLUS31540 (`NHL Legacy - Vanilla/`; `NHL Legacy - Modded/` and an `.iso` also exist) | `105d2186…` = anchor | `70814551…` = anchor | **`db/nhlng.db` = `7d1e2b8e…` — MODIFIED** (`db/backup/nhlng.db` and `nhlng.db.bak` = anchor); dictionary = anchor; a second `BLUS31540 Empty` data dir holds `0.ver` only | **ambiguous + dirty**: the probable live DB is a prior user edit, not vanilla |

NHL 07 PS3 (research-only) is not installed. RPCS3 instance `H:\Emulators\RPCS3 - PureSports` also
carries a `BLUS31540` data dir (not inventoried further; not the registered instance).

### 1.3 Xbox 360 (Xenia) and Legacy Recompiled

| Target | Tree | DB | Dictionary | Executable |
|---|---|---|---|---|
| NHL Legacy Xbox 360 — loose tree | `H:\Emulators\games\XBOX\NHL Legacy\` (no `.big`; `db/`, `fe/loc/` loose) | `db/nhlng.db` `105d2186…` = anchor; meta = anchor | `fe/loc/nhl_eng_us.db` `f00926ce…` = **Xbox 360** anchor (+7 other locales) | `default.xex` `d818eb15…` |
| NHL Legacy Xbox 360 — packed tree | `H:\Emulators\games\XBOX\NHL Legacy - Vanilla\` (`.big` set + `_compiled/` extraction); `NHL Legacy - Modded\` also present | `cacheboot.big` #50 raw `105d2186…` (Studio binder: `0x00000400`, 16-byte rows, 61 files, path hashes verified) | `cacheboot.big` #3 raw `f00926ce…` | same `default.xex` |
| Legacy Recompiled | `H:\Emulators\games\XBOX\NHL Legacy Decomp Test\` — `nhllegacy.exe` `7264bec6…` + `game\` (`.big` set, `default.xex` `d818eb15…`, no loose `db/`) | `game\cacheboot.big` #50 raw `105d2186…` (byte-identical archive `b558b4f6…`) | `game\cacheboot.big` #3 raw `f00926ce…` | recompiled binary |
| NHL 07–15 Xbox 360 | **not installed** (anchor-only) | — | — | — |

Which Xbox 360 tree Xenia launches is not recorded anywhere inspected (its log names only
`game:\default.xex`); the runtime checklist makes the user identify the root by hash first.

## 2. Provenance resolutions

### 2.1 NHL 19 / NHL 20 duplicate anchors — **resolved: both are NHL 19 base**

`anchors/ps4/nhl19/databases/nhl_game.db` = `anchors/ps4/nhl20/databases/nhl_game.db` = `265e4a68…` =
the NHL 19 install's **base** `inxp_00.big` member 2089; `anchors/ps4/nhl20/databases/eng_us.db`
`62260005…` = NHL 19 base member 936; both `nhl_game-meta.xml` anchors `c67f4bfd…` = NHL 19 base member
1179. The NHL 20 install (CUSA15439, no patches) carries a *different* database (`1e200478…`,
10 226 516 B, 31 `stockleagues` rows including `TXT_USA_LEAGUE`/`TXT_RUSSIA_LEAGUE` and three
`CUSTOM_TEAMS`, i.e. the NHL 21-family layout; `kOtt` differs too) and dictionary (`0fc95ec8…`,
40 535 rows) that match no anchor. Consequences:

* **NHL 20 PS4 was missing-fixture until this batch.** Every earlier "NHL 20" row (offline
  campaign, parity matrix §0.2, `tdb-loc` fixture tests, the L1.1-A §1.4 proxy columns) measured NHL 19
  base bytes. This batch's NHL 20 row uses the install's own members (hash-bound sources under
  `local/runs/league-parity-batch/sources/nhl20-ps4/`, not committed).
* The NHL 19 row's "candidate" status resolves to **verified base provenance**, with the separate
  question of which of the seven shipped databases the game loads (§2.2).
* Anchor relabelling / a real NHL 20 anchor is a coordinator decision (C0-REG4); nothing was moved.

### 2.2 NHL 19 base vs patch — **identified, unresolved at runtime**

APP_VER 01.60 (`gp`) with `patch_inxp_51..56.big` each carrying `t3db/nhl_game.db` and
`loc/eng_us.db`; the databases differ across patches (lengths 10 347 860 → 10 337 184 → 10 318 300 →
10 318 668) and from base (e.g. `stockleagues` still 29 rows but EBEL 11→12 members, ECHL 26→27,
Pro-Am 80→84 on patch 56). The precedence rule (highest patch wins? mount order?) is a runtime
hypothesis. The batch prepared **two** candidates — base (`inxp_00.big`) and `patch_inxp_56.big` — so
the runtime pass can discriminate with one placement each. Each uses the dictionary from the same
archive as its database.

### 2.3 NHL 15 PS4 — **layout blocker + base/patch ambiguity**

All `databigs/*.big` are version `0x00100400` with 16-byte rows (`[block][word_4][size][path_hash]`,
verified by the batch's fallback parse: the DB member's `path_hash` `0x00429ca7` equals the djb2
value the 20-byte layouts carry at `+0x10`). Studio's binder refuses the version (`unexpected EB
version tag 0x00100400`). Two archives also declare `archive_size` smaller than their file length
(`game20.big`, `inxp20.big`, `patch_inxp00.big`) — a header difference the current reader would
have to characterise before this layout is supported. `patch_inxp00.big` ships a different
`nhl_game.db` (48 B longer) and dictionary. Studio's loose route works (offline row below).

### 2.4 NHL 19 stale loose game tree — **runtime blocker until resolved by the user**

`H:\Emulators\ShadPS4\Games\CUSA11126\nhl\` holds a loose extraction dated 2026-07-19: 
`…/t3db/nhl_game.db` (= base `265e4a68…`), `…/t3db/nhl_game-meta.xml`, roster `.db` files, and
`kettle/loc/*.db` (eng_us = base `62260005…`), plus a `.nhlmodproj` sidecar from a prior tool (not
read). With loose files present, an archive placement cannot be attributed (loose-vs-archive
precedence is the N19-H1 open question), so the deploy helper **refuses** NHL 19 installs
(`check no loose game-tree DB dir FAIL`) by design. Resolution is the user's: move the loose
`nhl\data\common\gamemodes` (and `kettle\loc`) aside before the NHL 19 placement, or accept an E2-style
loose experiment instead. Nothing was moved.

### 2.5 PS3 NHL 08 / NHL 09 dictionary anchors — **Xbox 360 files under a PS3 label**

The PS3 discs ship `nhl_eng_us.db` `b9fb3361…` (08) and `537319ce…` (09); the repository's
`anchors/ps3/nhl08|nhl09/databases/nhl_eng_us.db` are byte-identical to the Xbox 360 anchors
(`308021e7…`, `d9e0463b…`). From NHL 10 on, the PS3 and Xbox 360 dictionary anchors differ and the PS3
ones equal the discs. Consequence: any PS3 NHL 08/09 dictionary work must use the disc file; the
anchors stay as they are (coordinator decision).

### 2.6 NHL Legacy PS3 game-data DB is modified

`dev_hdd0/game/BLUS31540/USRDIR/db/nhlng.db` = `7d1e2b8e…` with the vanilla bytes kept as
`db/backup/nhlng.db` and `db/nhlng.db.bak`. Studio opens it (16-league catalog, same vocabulary
fingerprint), but a Legacy PS3 runtime baseline must first decide which DB is live and whether the
modified one is acceptable as a baseline. Not touched.

## 3. Native league-name editing route per target family

### 3.1 PS4 (NHL 15–21): `stockleagues.name` (`fsLK.vIys`) — existing route, reproduced

Route (Studio, unchanged from NHL21-E1): open the title's own `nhl_game.db` + `-meta.xml` → attach
the title's own `eng_us.db` → resolve the league **by key** → `set_field(fsLK, record, vIys)` →
pending review (exactly one cell) → `copied_target_review_archive_member` (same-length raw member
replacement, `+0x0C`/header `0x10` preserved verbatim, unqualified) → apply → allowed-difference
audit → restore. The dictionary member is also raw and unshared on every PS4 title (§1.1), so a
future dictionary edit can use the same member route.

### 3.2 PS3 / Xbox 360 / Recompiled: code → executable table → dictionary key → text — **traced on Legacy Xbox 360**

* **Binding (traced, Legacy Xbox 360 `default.xex`, decompressed image base `0x82000000`):** a static
  pointer table (file offset `0x19aa64c` region, 8 identical copies) lists the league keys **in code
  order**: 0 `NHL`, 1 `AHL`, 2 `elitserien`, 3 `SMLIIGA`, 4 `TXT_LEAGUE_DEL`, 5 `TXT_LEAGUE_O2_EXTRALIGA`,
  6 `TXT_LEAGUE_SWISS_LEAGUE`, 7 `TXT_LEAGUE_NORWAY`, 8 `NATIONAL`, 9 `OHL`, 10 `QMJHL`, 11 `WHL`,
  12 `PROSPECT_TEAMS`, 13 `CUSTOM_TEAMS`, **14 `NHL`** (the Winter Classic rows), 15 `EA_SPORTS_HOCKEY_LEAGUE`,
  16 `CHL`, 17 `UnitedStates`, 18 `Russia`, 19 `TXT_ROW_LEAGUE`, 20 `HUT`, 21 `TXT_LEGEND_TEAMS` — the
  same 22-key list NHL 16 PS4 stores in `stockleagues.name`, and consistent with `gen7-nhllegacy.toml`
  (code 14 = Winter Classic NHL rows; 15 = EASHL). The table sits in an enum-name block (preceded by
  difficulty names, followed by trophy names). **This establishes that the code→key list is in the
  executable, not in any data file, on this title.** Which surfaces call it is not traced.
* **Dictionary (verified on every gen7 anchor and every installed dictionary):** the keys resolve
  in the title's own `nhl_eng_us.db`, and the lookup is evidently by `hashid` (case-folded: `AHL`
  resolves through the row keyed `ahl`, `CUSTOM_TEAMS` through `Custom_Teams`, `HUT` through `hut`),
  which Studio's exact-key lookup does not do (small labelling gap, §7 P4). Per era the key set
  matches the code vocabulary: NHL 15/12–14 (all keys), NHL 11 (no `TXT_LEGEND_TEAMS`), NHL 10 (no
  `OHL`/`TXT_ROW_LEAGUE`), NHL 09 (no Swiss; `TXT_LEAGUE_SUPER_LEAGUE` → "Superliga" corroborates code 6
  = Russian Superliga), NHL 08 (only `ahl`/`elitserien`/`SMLIIGA`/`National`), NHL 07 (only `National`;
  gen6 league names are not dictionary keys → label source unresolved).
* **Not traced:** the PS3 executables (encrypted `EBOOT.BIN`; RPCS3's cache holds compiled objects,
  not a decrypted ELF), the Xbox 360 executables of NHL 07–15 (not installed), and the recompiled
  binary. The PS3 binding is *expected* to be the same static table (identical DB and dictionary
  key set) — an expectation, not evidence.
* **Equivalent operation:** because the key list is baked into the executable, the exact analogue of
  the PS4 edit is a **dictionary text re-point**: make key A's text pointer equal key B's text pointer
  (4 bytes in A's record + the body CRC at EOF; length and pool untouched; restore = write the old
  pointer back). Dry plans were computed read-only per installed title (§4.2). **Minimum
  localization support needed:** a `tdb-loc` pointer re-point writer plus a Studio route that stages
  a dictionary (loose PS3 `USRDIR/fe/loc/nhl_eng_us.db`; Xbox 360/Recomp `cacheboot.big` member 3 or
  the loose `fe/loc/` file) as a copied-target artifact with pending review and restore — package
  **LOC-W1** (§7). Reader support alone (`tdb-loc`, `attach_dictionary`) does **not** establish edit
  support, and no dictionary was written in this batch.
* **Packaging (offline, structural):** PS3 loose `USRDIR/db|fe/loc` (S2-C style; hdd0 vs disc copy
  per §1.2); Xbox 360 packed `cacheboot.big` binds through the existing S2-A stager (`0x00000400`,
  16-byte rows, both members raw and unshared — `reports/xbox360-legacy-cacheboot-bind.txt`); the Xenia
  loose tree needs no archive at all; Recompiled uses the same `game\cacheboot.big`.

## 4. Offline results (developer-run, this host, 2026-09-18)

### 4.1 PS4 — one controlled rename per title through Studio (`reports/<target>.txt`)

Common outcome on every row: pending review = exactly one cell; review handle → apply
`committed` (`applied_unqualified = true`, `live_install_allowed = false`); record-level audit = only
the edited `fsLK` record; byte-level = the name bytes inside that record + one 4-byte `prior_crc`
word in the next table's header; header CRC and size valid; length unchanged; archive: 0 metadata
rows differ, prefix identical, every differing byte inside the replaced member; copied root
restored byte-exactly (`restored`, 0 leftovers, apply allowed); candidate reopened in a fresh
registry with the key re-pointed and the member-team identities/counts unchanged; restored root
reopened with the original key.

| Row | Edit (resolved by key) | Label before → after (title's own dictionary) | Loose DB out (SHA-256) | Candidate archive out (SHA-256, bytes) | Note |
|---|---|---|---|---|---|
| nhl21-ps4 | `fsLK[8].name` `Allsvenskan` → `TXT_ROW_LEAGUE` (14 members) | HockeyAllsvenskan → Rest Of World | `343404153ae5117708e737ec69c9824a816bc77560c1b7a6ea825833da59b569` | `inxp_00.big`: `4876cd2b3f4b51878fcbf08f8a56249df2eecbc6c086340b869fc9eada9da582` (777 373 328) | **identical to the accepted E1 candidate** (deterministic reproduction) |
| nhl20-ps4 | `fsLK[8].name` `Allsvenskan` → `TXT_ROW_LEAGUE` (14) | HockeyAllsvenskan → Rest Of World | `3eee9daa7baf0c90d13f6a60aae709654a940e49cdb1d2021f9c23d78825f3ee` | `inxp_00.big`: `2d295097f1d9070b123d0cd3c9817063fa6724ef26a98ed909cb35fa3fca4197` (517 362 304) | first NHL 20 measurement on NHL 20 bytes |
| nhl19-ps4-base | `fsLK[8].name` `Allsvenskan` → `TXT_ROW_LEAGUE` (14) | HockeyAllsvenskan → Rest Of World | `eda86b60409fbd0449c3e54c76788dead8cba8c8482cccd4608f834bad278a8e` | `inxp_00.big`: `9100d5fe5e0c67872af11c5e3896a5ff0d4e3bb3772d5abf2c6e97d11c5da386` (445 758 880) | loose hash = the offline campaign's "nhl19/nhl20" row |
| nhl19-ps4-patch56 | same row on the patch-56 DB (`61332e66…`) | HockeyAllsvenskan → Rest Of World (patch-56 dictionary `fbb8adcf…`, 38 001 rows) | `3d271b069e3cd99b3e7e2a1883f94dc921f8bc2985a3d422de64ed550c6e4a4e` | `patch_inxp_56.big`: `4e69f9c3e5bf6d0fb4ca6a77964243c8b4213fbf6a4e4e316bbb56b8e05be6ce` (128 855 568) | precedence hypothesis |
| nhl18-ps4 | `fsLK[8].name` `Allsvenskan` → `TXT_ROW_LEAGUE` (14) | HockeyAllsvenskan → Rest Of World | `7cd6b886f32418f828879d4222c3a5f9afad4ef50fb6f7684361edb343a0ec4b` | `inxp_00.big`: `c6773f3c1d53497eba86e212ab340577c8670cc574701814dc1dcc4e1f08b4fb` (390 197 792) | |
| nhl17-ps4 | `fsLK[7].name` `TXT_LEAGUE_NORWAY` → `TXT_ROW_LEAGUE` (2) | Norway → Rest Of World | `03a01ed35158e5dbd0bc38179cfc07484f4b40f7d08ce5c40392d8719cf8a57b` | `inxp_00.big`: `3dd607f7b6ca31f482f300e68c6e24781c03eaab810c5d6ab3d9d84353c0e5fc` (273 371 840) | |
| nhl16-ps4 | `fsLK[7].name` `TXT_LEAGUE_NORWAY` → `TXT_ROW_LEAGUE` (2) | Norway → Rest Of World | `f36a3b7a3965c68ec46f9d0b0fccc898a45c180ac16d0173b49261eca0f747ea` | `inxp_00.big`: `fc64342c2dfe5c25671e9e1cb94a2ac29d1489e762c613ac04ca7a3dcd1e8b55` (299 883 104) | |
| nhl15-ps4 | `fsLK[12].name` `TXT_LEGEND_TEAMS` → `TXT_ROW_LEAGUE` (1) | Legend Teams → Rest Of World | `48ef259fd68ac3dea825830d13fecd43be188995f7a9266399f0e5b12762323e` | **none** — binder refuses `0x00100400` | loose-only; archive route blocked |

Every loose hash that had an earlier value (15/16/17/18/19-base/21) reproduced it exactly.
Original archive identities (the pre-run values the helper demands): NHL 21 `1c41f578…`, NHL 20
`10306c2f33769c09f5a00fb038cdf85ee042d8ee6c173b6ab5457e7de1032380`, NHL 19 base
`6efa9096866bd2300c286c5b151db19c620938f5d2c3e1017309b5deff38ea7e`, NHL 19 patch-56
`0ac608d1a8a943d320bcf6095c0530300746f6783487d7f76dc1df69670326a8`, NHL 18
`48aab4a93150d3eb5dbc43c2e9b58d3584192ec828189c9a5bcf37d6966ff369`, NHL 17
`3f659305de18bb5a796c8198d4f1c953535dfad776c90fec06a11e8f47a6f366`, NHL 16
`0db9701aae0a3f0f6f5d5b0d9b30aafa02374b20daf3ca5d9ea3e93eabe1408f` (full values also in
`manifests/*.json`).

### 4.2 PS3 / Xbox 360 — browse/classify on the install's own DB + dictionary re-point dry plan (`reports/gen7-*.txt`)

Catalog through the Registry on the install's own database (derived codes, per-title vocabulary,
name-edit blocker reported — no edit exists); then the read-only dry plan of the equivalent
dictionary edit on the install's own dictionary (source key ← donor key's text; the 4-byte slot,
old/new pointer, and the body-CRC scope that would move). No bytes were written.

| Row | DB used (install) | Catalog | Dictionary used | Dry plan (A ← text of B) |
|---|---|---|---|---|
| nhllegacy-ps3 (disc) | `105d2186…` | 16 leagues, all labelled (`gen7-nhllegacy`) | disc `70814551…` | `TXT_LEAGUE_NORWAY` ("Norway", rec 13305, slot `0x3400c`, ptr `0x000a9a5d`) ← `TXT_ROW_LEAGUE` ("Rest Of World", ptr `0x0013349f`); body CRC at `0x1ad400` |
| nhllegacy-ps3 (hdd0, modified DB) | `7d1e2b8e…` | 16 leagues (same fingerprint) | hdd0 `70814551…` | identical plan |
| nhl15-ps3 (hdd0 = disc) | `028ae06a…` | 15 leagues (`gen7-nhl12-15`) | `5f3f8f20…` | `TXT_LEGEND_TEAMS` ("Legend Teams", slot `0x56c1c`, `0x0011cfcc`) ← `TXT_ROW_LEAGUE` (`0x0012eb67`) |
| nhl14-ps3 (hdd0 = disc) | `84e1bdff…` | 15 | `d7bf2a6f…` | `TXT_LEGEND_TEAMS` (slot `0x5389c`, `0x001158bb`) ← `TXT_ROW_LEAGUE` (`0x00126c53`) |
| nhl13-ps3 (hdd0 = disc) | `64460a15…` | 15 | `69cc4f75…` | `TXT_LEGEND_TEAMS` (slot `0x3f1ec`, `0x000b2fa6`) ← `TXT_ROW_LEAGUE` (`0x000bf164`) |
| nhl12-ps3 (folder copy) | `94b31ff2…` | 15 | `3327bc3a…` | `TXT_LEGEND_TEAMS` ("LEGEND TEAM", slot `0x387ac`, `0x0008d43e`) ← `TXT_ROW_LEAGUE` ("REST OF WORLD", `0x00096d12`) |
| nhl11-ps3 (disc) | `461f8d64…` | 13 (`gen7-nhl11`) | `48e3cd43…` | `PROSPECT_TEAMS` ("PROSPECT TEAMS", slot `0x3482c`, `0x0007ea56`) ← `TXT_ROW_LEAGUE` ("REST OF WORLD", `0x00081ca9`) |
| nhl10-ps3 (disc) | `e87b8c80…` | 11, codes 8/9 raw | `954d1617…` | `TXT_LEAGUE_SWISS_LEAGUE` ("NATIONAL LEAGUE", slot `0xa43c`) ← `CUSTOM_TEAMS` ("CUSTOM TEAMS", `0x000547b0`) — no ROW key on NHL 10 |
| nhl09-ps3 (disc) | `0f7c826b…` | 10, code 8 raw | disc `537319ce…` (≠ anchor) | `TXT_LEAGUE_O2_EXTRALIGA` ("O2 Extraliga", slot `0x1221c`) ← `Custom_Teams` ("Custom Teams", `0x00040afe`) |
| nhl08-ps3 (disc) | `587204c3…` | 6 (`gen7-nhl08`) | disc `b9fb3361…` (≠ anchor) | `SMLIIGA` ("SMLiiga", slot `0x26ec`) ← `National` ("National", `0x00009030`) |
| nhllegacy-xbox360 (loose tree) | `105d2186…` | 16 | `f00926ce…` | `TXT_LEAGUE_NORWAY` (rec 13299, slot `0x33fac`, `0x000a9bda`) ← `TXT_ROW_LEAGUE` (`0x001337f7`); body CRC at `0x1ad740` |

The Xbox 360 / Recompiled packed copies were bound through Studio's stager (§3.2); NHL 07 Xbox 360
and NHL 08–15 Xbox 360 are not installed (anchor-only, offline campaign rows stand).

## 5. Consolidated review matrix (27 shipping targets + NHL 07 PS3 research copy)

Columns: **Original → intended label** (title's own dictionary) · **Native field/key** · **Source
provenance** (what the batch used, and how it relates to the anchors) · **Offline result** ·
**Candidate / rollback identities** · **Runtime readiness** · **Precise blocker**. Identical fixture
bytes never merge rows.

| # | Target | Original → intended label | Native field / key | Source provenance | Offline result | Candidate / rollback identities | Runtime readiness | Precise blocker |
|---|---|---|---|---|---|---|---|---|
| 1 | **NHL 21 PS4** | HockeyAllsvenskan → Rest Of World | `stockleagues[8].name` `Allsvenskan` → `TXT_ROW_LEAGUE` | install CUSA18182 01.00 `inxp_00.big` #2652 = anchor `90ee1ec4…`; dictionary #1156 `24693e2f…` (hash-bound local, not a fixture) | automated-tested (E1-prep reproduced; batch row OK) | candidate `4876cd2b…` (777 373 328); rollback = live `.e1-original` (rename) / `copied\nhl21-ps4\…\inxp_00.big` `1c41f578…` | **done — NHL21-E1 accepted (user-observed)**; candidate available if a repeat is ever requested | none for E1 scope; E2/E4 not prepared (out of this batch) |
| 2 | **NHL 20 PS4** | HockeyAllsvenskan → Rest Of World | `stockleagues[8].name` `Allsvenskan` → `TXT_ROW_LEAGUE` | install CUSA15439 01.00 members: DB `1e200478…`, meta `2fac0e73…`, dictionary `0fc95ec8…` — **no anchor matches** (repo "nhl20" anchors are NHL 19 base) | automated-tested (batch row OK; NHL 20-specific hashes new) | candidate `2d295097…` (517 362 304); original `10306c2f…`; loose evidence `3eee9daa…` | **ready** (status: S0-stock, all automated checks PASS) | G0 (stock boot + surface) not yet observed for NHL 20 |
| 3 | **NHL 19 PS4** (base placement) | HockeyAllsvenskan → Rest Of World | `stockleagues[8].name` `Allsvenskan` → `TXT_ROW_LEAGUE` | install CUSA11126 01.60 base `inxp_00.big` #2089 = anchor `265e4a68…`; dictionary #936 `62260005…` (= repo "nhl20" dictionary anchor) | automated-tested (batch row OK; loose = campaign hash `eda86b60…`) | candidate `9100d5fe…` (445 758 880); original `6efa9096…` | **refused by the helper** (loose game tree present) | §2.4 stale loose tree; base/patch precedence unresolved (§2.2) |
| 3b | NHL 19 PS4 (patch-56 placement) | same | same row on patch-56 DB `61332e66…` | `patch_inxp_56.big` #504; dictionary #209 `fbb8adcf…` | automated-tested (batch row OK) | candidate `4e69f9c3…` (128 855 568); original `0ac608d1…` | refused (same loose-tree check) | same; which archive the game reads is the experiment |
| 4 | **NHL 18 PS4** | HockeyAllsvenskan → Rest Of World | `stockleagues[8].name` | install CUSA07569 01.00 #1200 = anchor `65eaa6e8…`; dictionary #503 = anchor `12f1af46…` | automated-tested (OK) | candidate `c6773f3c…` (390 197 792); original `48aab4a9…` | **ready** | G0 not yet observed |
| 5 | **NHL 17 PS4** | Norway → Rest Of World | `stockleagues[7].name` `TXT_LEAGUE_NORWAY` → `TXT_ROW_LEAGUE` | install CUSA04326 01.00 #1053 = anchor `de48b02b…`; dictionary #441 = anchor `035b3b30…` | automated-tested (OK) | candidate `3dd607f7…` (273 371 840); original `3f659305…` | **ready** | G0 not yet observed |
| 6 | **NHL 16 PS4** | Norway → Rest Of World | `stockleagues[7].name` | install CUSA01909 01.00 #218 = anchor `7d95f1db…`; dictionary #86 = anchor `cb1f5d56…` | automated-tested (OK) | candidate `fc64342c…` (299 883 104); original `0db9701a…` | **ready** | G0 not yet observed |
| 7 | **NHL 15 PS4** | Legend Teams → Rest Of World | `stockleagues[12].name` `TXT_LEGEND_TEAMS` → `TXT_ROW_LEAGUE` | install CUSA00561 01.00/01.06 `inxp00.big` #0 = anchor `ae9d1d56…`; dictionary #146 = anchor; `patch_inxp00.big` carries a different DB/dictionary | automated-tested **loose only** (`48ef259f…`); archive route **blocked** | loose evidence only; no archive candidate | **blocked** | EB version `0x00100400` (16-byte rows) unsupported by Studio's binder/stager → package **EB-V1** (§7); base/patch ambiguity |
| 8 | NHL Legacy PS3 | (Norway → Rest Of World would be the equivalent) | none in DB; code 7 → exe key `TXT_LEAGUE_NORWAY` (traced on X360, expected on PS3) → dictionary text | disc BLUS31540 DB = anchor; **hdd0 DB modified** `7d1e2b8e…`; dictionary = anchor `70814551…` | browse/classify OK; dry plan computed | none (no writer); nothing to roll back | **blocked** (identity edit does not exist) | **LOC-W1** dictionary re-point writer + route; live-DB choice (disc vs modified hdd0) |
| 9 | NHL Legacy Xbox 360 | same | code 7 → **traced** exe table → `TXT_LEAGUE_NORWAY` → `nhl_eng_us.db` | loose tree DB = anchor; dictionary = X360 anchor `f00926ce…`; packed `cacheboot.big` members bind in Studio | browse/classify OK; dry plan; packaging binds | none | **blocked** | LOC-W1; which Xenia tree is launched (loose vs packed) to be identified by hash |
| 10 | Legacy Recompiled | same | as Xbox 360 (same `default.xex`, same `cacheboot.big`) | `game\cacheboot.big` `b558b4f6…` = Vanilla tree; **no separate anchor** (was missing-fixture; now has an install-bound source) | packaging binds; catalog by reference to the identical DB | none | **blocked** | LOC-W1; recompiled loader behaviour unknown |
| 11 | NHL 15 PS3 | (Legend Teams → Rest Of World) | code 12 → key `TXT_LEGEND_TEAMS` (expected) → dictionary | hdd0 BLUS31436 = disc = anchors | browse OK; dry plan | none | blocked | LOC-W1; hdd0-vs-disc live copy |
| 12 | NHL 15 Xbox 360 | same | same | **not installed** (anchor-only) | campaign row (anchor) | none | not installed | LOC-W1 + install |
| 13 | NHL 14 PS3 | same | same | hdd0 BLUS31179 = disc = anchors | browse OK; dry plan | none | blocked | LOC-W1; hdd0-vs-disc |
| 14 | NHL 14 Xbox 360 | same | same | not installed | campaign row | none | not installed | LOC-W1 + install |
| 15 | NHL 13 PS3 | same | same | hdd0 BLUS31003 = disc = anchors | browse OK; dry plan | none | blocked | LOC-W1; hdd0-vs-disc |
| 16 | NHL 13 Xbox 360 | same | same | not installed | campaign row | none | not installed | LOC-W1 + install |
| 17 | NHL 12 PS3 | (LEGEND TEAM → REST OF WORLD) | same | registered `.iso`; folder copy = anchors | browse OK; dry plan | none | blocked | LOC-W1; ISO vs folder (the ISO is what RPCS3 runs) |
| 18 | NHL 12 Xbox 360 | same | same | not installed | campaign row | none | not installed | LOC-W1 + install |
| 19 | NHL 11 PS3 | (PROSPECT TEAMS → REST OF WORLD) | code 11 → `PROSPECT_TEAMS` (expected) | disc BLUS30568 = anchors | browse OK; dry plan | none | blocked | LOC-W1; save acceptance open (K-register) |
| 20 | NHL 11 Xbox 360 | same | same | not installed | campaign row | none | not installed | LOC-W1 + install |
| 21 | NHL 10 PS3 | (NATIONAL LEAGUE → CUSTOM TEAMS as the test label) | code 6 → `TXT_LEAGUE_SWISS_LEAGUE` (expected); no ROW key on NHL 10 | disc BLUS30345 = anchors (no disc meta) | browse OK (codes 8/9 raw); dry plan | none | blocked | LOC-W1; codes 8 "CEN" / 9 "NAJ/WPR" unidentified (lead: 9 = Top Prospects pair, unverified) |
| 22 | NHL 10 Xbox 360 | same | same | not installed | campaign row | none | not installed | LOC-W1 + install |
| 23 | NHL 09 PS3 | (O2 Extraliga → Custom Teams as the test label) | code 5 → `TXT_LEAGUE_O2_EXTRALIGA` (expected); code 6 → `TXT_LEAGUE_SUPER_LEAGUE` "Superliga" corroborates the Russian-league candidate | disc BLUS30168 = anchor DB; **disc dictionary `537319ce…` ≠ anchor (X360 file)** | browse OK (code 8 raw); dry plan on the disc dictionary | none | blocked | LOC-W1; code 8 "CEN" unidentified; PS3 dictionary anchor mislabelled |
| 24 | NHL 09 Xbox 360 | same | same | not installed (anchor dictionary `d9e0463b…` is the X360 file) | campaign row | none | not installed | LOC-W1 + install |
| 25 | NHL 08 PS3 | (SMLiiga → National as the test label) | code 3 → `SMLIIGA` (expected) | disc BLUS30062 = anchor DB; **disc dictionary `b9fb3361…` ≠ anchor** | browse OK; dry plan | none | blocked | LOC-W1; PS3 dictionary anchor mislabelled |
| 26 | NHL 08 Xbox 360 | same | same | not installed | campaign row | none | not installed | LOC-W1 + install |
| 27 | NHL 07 Xbox 360 | — (no dictionary key evidence; only `National` exists) | `stockteamtable.league` code only; label source **unresolved** (not the dictionary key family) | not installed | campaign row (browse, code 5 raw) | none | not installed | label source untraced (gen6); LOC-W1 does not apply as specified |
| — | NHL 07 PS3 (research-only) | — | — | not installed | campaign row | — | out of scope | — |

## 6. Ordered runtime checklist (user-driven navigation; one title at a time; nothing placed before Codex approval and the user's go)

Rules carried over unchanged from the accepted card (§1 of
[league-identity-runtime-qualification](../plans/league-identity-runtime-qualification.md)): cold
start; hash before and after; the E1 evidence class (F12 whole-screen where the user chooses to;
otherwise a developer-attested user report, labelled as such); G0 before any modified boot; E5
restore closes each title. Helper: `local\runs\league-parity-batch\deploy.ps1 -Target <row> -Mode
status|install|rollback [-ExpectedExeSha aeb04892… ] [-ConfigPre <pre-run config.toml>]` — same
state machine and sidecars as E1, per-target manifest. Record the emulator version line from
`%APPDATA%\shadPS4\log\shad_log.txt` per boot and copy each boot's log beside the run.

0. **Session start (once).** Emulator closed. Copy `%APPDATA%\shadPS4\config.toml` to
   `local\runs\league-parity-batch\runtime\config.pre-batch.toml` (then pass it as `-ConfigPre`);
   note `Profile0`/`cache\CUSA*.zip` present per title; `deploy.ps1 -Target <each> -Mode status` for
   all seven archive rows and keep the transcript.
1. **NHL 18 PS4 (row 4)** — first, because provenance is clean and the edit is the E1 edit on the
   same layout family. G0: stock boot → Rosters / Edit Player league tile "HOCKEYALLSVENSKAN" (and
   Play Now / Select Team if reachable); leave/re-enter; exit. Install (`-Mode install`, expects live
   `48aab4a9…` → `c6773f3c…`). Cold boot ×2 → the tile reads "REST OF WORLD"; unrelated tiles (SHL,
   DEL, LIIGA, NLA, NHL) unchanged; one Allsvenskan team still listed. Rollback (`-Mode rollback`,
   verifies `48aab4a9…`, removes the used candidate). E5 stock boot → "HOCKEYALLSVENSKAN".
2. **NHL 17 PS4 (row 5)** — same procedure; baseline tile "NORWAY" (2 teams: Stavanger, Vålerenga)
   → "REST OF WORLD"; hashes `3f659305…` → `3dd607f7…` → `3f659305…`.
3. **NHL 16 PS4 (row 6)** — same; "NORWAY" → "REST OF WORLD"; `0db9701a…` → `fc64342c…` → `0db9701a…`.
4. **NHL 20 PS4 (row 2)** — same; "HOCKEYALLSVENSKAN" → "REST OF WORLD"; `10306c2f…` → `2d295097…` →
   `10306c2f…`. This is also the first stock boot of NHL 20 on this fork: record profile/first-boot
   behaviour as E0 did for NHL 21.
5. **NHL 19 PS4 (rows 3/3b)** — **precondition, user decision:** move the stale loose tree
   `CUSA11126\nhl\data\common\gamemodes` (and `nhl\data\kettle\loc`) aside (rename the `nhl\data\common`
   subtree to a sibling outside the game folder; record the move) — or decide to keep it and run an
   E2-style loose experiment instead (not prepared here). Then `status` must report `no loose
   game-tree DB dir PASS`. G0 stock boot. Placement order: **patch-56 first** (`-Target
   nhl19-ps4-patch56`; `0ac608d1…` → `4e69f9c3…`): tile change ⇒ the patch member is consumed;
   unchanged ⇒ rollback, then **base** (`-Target nhl19-ps4-base`; `6efa9096…` → `9100d5fe…`). Never
   place both at once (identical edits in two locations establish nothing). Rollback + E5 after
   each. Restore the loose tree only after E5 if the user wants it back.
6. **NHL 21 PS4 (row 1)** — **no action**: E1 accepted. Optional only if the user asks: the
   identical candidate `4876cd2b…` is present for a repeat with F12 captures.
7. **NHL 15 PS4 (row 7)** — no placement (blocked). Optional G0 only: stock boot, record the
   "LEGEND TEAMS" tile and whether the base or patch database is evidently loaded (the patch DB has
   the same league rows, so leagues cannot discriminate; note it as unobservable from this surface).
8. **PS3 / Xbox 360 / Recompiled (rows 8–27)** — **no files are replaced** (identity edit blocked
   pending LOC-W1), so nothing to roll back. Optional G0-only corroboration boots, in this order,
   each recorded with the title id, the emulator build line and the tree/DB hash actually launched:
   Legacy Xbox 360 (Xenia; identify the launched root by hashing its `db\nhlng.db` or `cacheboot.big`
   first) → Legacy Recompiled → Legacy PS3 (state whether RPCS3 read the modified hdd0 DB or the
   disc; the labels are the same either way) → NHL 15/14/13 PS3 → NHL 12/11 PS3 → NHL 10 PS3 (read
   the labels the game shows for codes 8 and 9) → NHL 09 PS3 (code 8) → NHL 08 PS3. These boots
   corroborate or refute `defs/leagues/*.toml`; they are mapping evidence, not edits.
9. **Close.** All PS4 installs re-hashed to their pre-run values; no `.e1-*` sidecars; helper logs,
   boot logs, screenshots/user reports and the per-title result tables filed under
   `docs/validation/evidence/league-parity-batch-runtime/<row>/`.

## 7. Missing capabilities — exact gaps and bounded packages (not started)

| Id | Missing capability (exact) | Bounded package | Unblocks |
|---|---|---|---|
| **LOC-W1** | `tdb-loc` is read-only: no way to change a dictionary row's text pointer (or any bytes) and recompute the body CRC; no Studio artifact kind for a dictionary; no pending-review cell for it | (a) `tdb-loc::repoint_text(bytes, key, donor_key) -> Result<Vec<u8>>` — locate both rows by exact key then by `loc_hash`, write the 4-byte pointer in the source row's text slot (endian-aware), recompute only the body CRC scope, verify by re-parse (`verify` all-match, `text(key)` == donor text, every other row unchanged), refuse null pointers/absent keys/mismatched lengths; (b) a Studio route staging the title's own dictionary as a copied-target artifact (`dictionary` kind) at its native location (PS3 loose `USRDIR/fe/loc/nhl_eng_us.db`; Xbox 360/Recomp `cacheboot.big` member via the existing 16-byte-row stager, or the loose `fe/loc` file) with pending review (one cell: `<locale> / <key> : "<old text>" → "<donor text>"`), apply, allowed-difference audit (4 + 4 bytes), reopen, restore; (c) automated cases on every gen7 dictionary anchor **and** on the PS3 08/09 disc dictionaries (hash-bound local sources) using the §4.2 plans; (d) no pool rewrite, no key insertion, no free-text edit — those stay Phase 1.3 | rows 8–26 identity edits (equivalent operation) |
| **EB-V1** | Studio's EB reader/stager rejects version `0x00100400` (NHL 15 PS4): 16-byte rows plus at least three archives whose header `archive_size` is smaller than the file length | add `0x0010_0400` to `KNOWN_VERSION_TAGS` with `entry_size 16`; characterise the header on all 17 CUSA00561 archives (row order, `archive_size` semantics on `game20`/`inxp20`/`patch_inxp00`, path-hash word position `+0x0C`); extend the layout report and the S2-A planner's same-length replacement; env-gated real-archive test on the install; no other behaviour change | row 7 archive route; NHL 15 PS4 asset packaging generally |
| **REG-1** (coordinator) | `anchors/ps4/nhl20/**` is NHL 19 base content; NHL 20 has no anchor; `anchors/ps3/nhl08|nhl09/…/nhl_eng_us.db` are the Xbox 360 files | decision + relabel/capture under C0-REG4; until then every consumer (campaign test, `tdb-loc` fixture test, parity matrix) must read "nhl20 anchor = NHL 19 base"; the batch's hash-bound sources under `local/runs/league-parity-batch/sources/` are the only NHL 20 files on hand | rows 2, 23, 25 provenance |
| **P4** (small) | Studio resolves dictionary keys by exact string; the game evidently resolves by `hashid` (case-folded), so `AHL`, `HUT`, `CUSTOM_TEAMS` show as keys although `ahl`/`hut`/`Custom_Teams` rows exist | `LocDictionary::text_by_hash(key)` fallback in the catalog label resolution, evidence string says "resolved by hash"; anchor test | human-readable labels on every PS4/gen7 catalog |
| **VOC-1** (research) | gen7 vocabularies for NHL 10 codes 8/9 and NHL 09 code 8 remain raw; gen7 code→key binding is traced on Legacy Xbox 360 only | research partner: PS3 executables (decrypted ELF) or the G0 boots in §6 step 8; add the traced key names to `defs/leagues/gen7-*.toml` as `key = "…"` with provenance once corroborated | rows 8–26 label confidence |

## 8. Artifacts and cleanup manifest

Retained under `E:\Repositories\nhl-modding-studio\local\runs\league-parity-batch\` (every file
listed with SHA-256 in `SHA256SUMS.txt`; **dependency: the runtime pass and Codex's review**):

| Item | Size | Why retained |
|---|---|---|
| `artifacts/<row>/{<archive>.X-candidate, nhl_game.db.X-loose-evidence}` (8 rows; 7 candidate archives) | 2.8 GB | the placement inputs (§4.1 identities) |
| `copied/<row>/<TITLE_ID>/` restored copied roots incl. engine journals | 5.5 GB | secondary rollback source per manifest (verified original bytes) |
| `sources/<row>/{nhl_game.db, nhl_game-meta.xml, eng_us.db}` | 131 MB | the install's own members the rows were built from (hash-bound; the only NHL 20 / NHL 19-patch files on hand) |
| `manifests/<row>.json`, `deploy.ps1`, `tests/{run-tests.ps1, results.txt}` | small | the install/rollback method (33/33 fixture cases) |
| `reports/*.txt` (per-row transcripts, gen7 rows, batch log, cacheboot bind) | small | offline evidence |
| `prep/`, `probe/`, `gen7-offline.sh` (sources) | small | reproduce the batch |

Removed at close (temporary): the package build directory `target/` (1.4 GB), the scratch
extraction/inventory scripts and extracted members in the session scratchpad (`sfo.py`, `eb.py`,
`ps3inv.py`, `ext/`), and the fixture roots `tests/fx/`. Free space after cleanup: see receipt.
Nothing in `anchors/` was touched (all 27 main-database anchors re-hashed unchanged by the
provenance checks); no live installation file was modified, renamed or created; no emulator ran.

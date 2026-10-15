# League-name parity completion — refreshed matrix, candidates and ordered test cards (2026-09-18)

> **Queue:** [league-parity-batch-review-and-completion-dispatch](league-parity-batch-review-and-completion-dispatch.md)
> — REG-1, EB-V1, LOC-W1, batch integration. Receipt:
> [league-parity-completion](../receipts/league-parity-completion.md). Supersedes the per-target
> table and checklist of [league-parity-batch-2026-09-18](league-parity-batch-2026-09-18.md)
> (whose inventory §1–§3 stays the source of the identities quoted here).
> Package files: `local/runs/league-parity-batch/` (`SHA256SUMS.txt`). **Nothing here is
> game-verified beyond NHL21-E1, which is closed.** Every candidate is a Studio-produced,
> structurally verified, *unqualified* artifact; a `status` result is preflight, not acceptance.

## 0. Plain-language summary

The four packages are done. **REG-1:** the record now says what the mislabelled anchors are (the
"NHL 20" files are NHL 19's base game; the PS3 NHL 08/09 dictionaries are the Xbox 360 files),
the correctly identified files are registered as private fixtures with fail-loud tests, and every
consumer of the old labels is annotated. **EB-V1:** Studio reads and stages the NHL 15 PS4 archive
layout after measuring all 17 archives, including the three whose declared size is not their
length; both NHL 15 databases (base and day-one patch) have candidate archives. **LOC-W1:** older
titles can be renamed through Studio by pointing a dictionary key at another key's text — preview,
hold, pending review, copied-target apply, reopen, restore — as a loose file or an archive member,
with the desktop app exercised end to end on NHL Legacy PS3. Thirteen dictionary candidates exist
for the installed PS3 / Xbox 360 / Recompiled titles, each labelled by how well its league-code →
key binding is known (traced only on NHL Legacy Xbox 360's executable). **Integration:** one
manifest-driven install/rollback helper covers all 21 candidate rows (38 fixture cases), NHL 19's
loose tree has a hash-bound, reversible two-file quarantine plan (tested on fixtures, not run),
and the ordered test cards below are ready for the joint runtime pass. NHL 07 stays blocked with a
specific result: its league names are not in its dictionary at all.

## 1. Package dispositions

| Package | Disposition | Where |
|---|---|---|
| REG-1 provenance | **done** (records + private fixtures + tests + consumer audit; adapter promotion deferred to the coordinator) | [anchors-manifest §Provenance corrections](../audits/anchors-manifest.md), `defs/adapters/nhl19-ps4.toml` / `nhl20-ps4.toml` headers, `crates/tdb-loc/tests/private_dictionaries.rs`, `crates/studio-api/tests/league_private_sources.rs`, parity matrix §0.2 rows |
| EB-V1 NHL 15 archives | **done** (17-archive characterisation → reader tag + stager rules + synthetic and real-archive tests → two candidates) | [ps4-eb-big §NHL 15 variant](../formats/ps4-eb-big.md), `crates/tdb-archive-big/src/{eb,stage}.rs`, `tests/ps4_real_archives.rs` |
| LOC-W1 dictionary rename | **done** (writer + workspace + transaction + archive route + UI; 13 candidates; desktop exercised) | `crates/tdb-loc/src/repoint.rs`, `crates/studio-domain/src/dictionary_edit.rs`, `workspace.rs`, `copied_target.rs`, `txn.rs`, `leagues.rs`; `crates/studio-api/src/registry*.rs`, `commands.rs`, `dto.rs`; `app/src/components/DictionaryRenamePanel.tsx`, `LeagueDetail.tsx`, `PendingEdits.tsx`, `CopiedTargetPanel.tsx`; `defs/leagues/gen7-*.toml` (`dictionary_key` / `key_evidence`); evidence `docs/validation/evidence/league-locw1-desktop/` |
| Batch integration | **done** (manifests refreshed, helper generalised + manifest loader tests, quarantine plan, this record) | `local/runs/league-parity-batch/{deploy.ps1, quarantine.ps1, manifests/, tests/}` |

## 2. What changed in the evidence since the batch

* **NHL 20 PS4:** the committed anchors are NHL 19 base; NHL 20's own files are private fixtures
  (`databases/nhl20-ps4-cusa15439/…`) and the row's candidate was built from them.
* **NHL 19 PS4:** base attribution has a witness; two candidates (base, patch-56) — the ignored
  case shows the two databases differ only in the Pro-Am member count (80 → 84) at league level.
* **NHL 15 PS4:** unblocked. Base `inxp00.big` and patch `patch_inxp00.big` candidates; the patch
  archive's declared size stays 178 432 B below its length, preserved verbatim (unresolved item).
* **gen7 (PS3 / Xbox 360 / Recompiled):** a rename route exists (dictionary text-pointer
  re-point). Labels on the Leagues page now come from the title's own dictionary through the
  vocabulary's `dictionary_key` when a dictionary is attached (the app attaches the discovered one
  on open); the evidence string names the binding class. Hash-based (case-folded) key resolution
  matches the game's own lookup (`AHL` via `ahl`, `HUT` via `hut`).
* **NHL 07 (Xbox 360; PS3 research-only):** its dictionary (3 952 rows) carries **none** of the
  league-key family (`NHL`, `AHL`, `elitserien`, `SMLIIGA`, `TXT_LEAGUE_*`, `NATIONAL` as a league
  key) — only `NHLCityName_*` / `NHLTeamName_*` and UI strings. The league labels' source is
  therefore not this dictionary: executable literals or `fe` UI data are the candidates, neither
  inspected (the title is not installed). **Result: blocked; LOC-W1 does not apply as specified.**

## 3. Consolidated review matrix (27 shipping targets + NHL 07 PS3)

Columns as in the batch record. *Binding* is the league-code → dictionary-key evidence class for
the rename, **resolved per target** (R1 F5): `native` (PS4 `stockleagues.name` is the key),
`traced` (read from *this target's* executable static table — NHL Legacy Xbox 360 only),
`by-reference` (the mapping is inherited from that witness: same database and key set, this
target's executable not inspected — NHL Legacy PS3 and Legacy Recompiled), `hypothesis` (key
family present in the title's own dictionary; binding untraced). Before R1 the shared Legacy
vocabulary displayed `traced` on PS3 and Recompiled as well; that was a display defect (F5), not
evidence, and the rows below are corrected. Readiness reflects today's read-only `status` run
(`reports/status-all-2026-09-18.txt`): all automated checks pass unless a blocker is named; every
row carries the manual gates (emulator build line; PS3/Xbox/Recomp overlay state; pre-run config).

| # | Target | Original → intended label | Native field / key · binding | Source provenance | Offline result (Studio) | Candidate / rollback identities | Runtime readiness | Precise blocker |
|---|---|---|---|---|---|---|---|---|
| 1 | NHL 21 PS4 | HockeyAllsvenskan → Rest Of World | `stockleagues[8].name` · native | install = anchor; own dictionary `24693e2f…` | reproduced E1 identities (`343404…`/`4876cd2b…`) | **closed — NHL21-E1 accepted**; batch candidate retired (E1 evidence durable) | no action | — |
| 2 | NHL 20 PS4 | HockeyAllsvenskan → Rest Of World | `stockleagues[8].name` · native | **install's own** DB `1e200478…`, dict `0fc95ec8…` (private fixtures; anchors are NHL 19 base) | OK: 1 cell; loose `3eee9daa…`; audited; restored | archive X `2d295097f1d9070b123d0cd3c9817063fa6724ef26a98ed909cb35fa3fca4197` (517 362 304); original `10306c2f33769c09f5a00fb038cdf85ee042d8ee6c173b6ab5457e7de1032380`; secondary = restored copied root | **ready** (S0-stock, PASS) | G0 not yet observed |
| 3 | NHL 19 PS4 — base | HockeyAllsvenskan → Rest Of World | `stockleagues[8].name` · native | base member = anchor `265e4a68…` (witnessed); dict `62260005…` | OK; loose `eda86b60…` | X `9100d5fe5e0c67872af11c5e3896a5ff0d4e3bb3772d5abf2c6e97d11c5da386` (445 758 880); original `6efa9096866bd2300c286c5b151db19c620938f5d2c3e1017309b5deff38ea7e` | **refused until the loose tree is quarantined** (`quarantine.ps1`, §5) | loose `t3db/nhl_game.db` + `loc/eng_us.db`; base/patch precedence |
| 3b | NHL 19 PS4 — patch 56 | same | same row on patch-56 DB `61332e66…` · native | patch member; dict `fbb8adcf…` | OK; loose `3d271b06…` | X `4e69f9c3e5bf6d0fb4ca6a77964243c8b4213fbf6a4e4e316bbb56b8e05be6ce` (128 855 568); original `0ac608d1a8a943d320bcf6095c0530300746f6783487d7f76dc1df69670326a8` | refused (same gate) | same; tested independently of base, restore between |
| 4 | NHL 18 PS4 | HockeyAllsvenskan → Rest Of World | `stockleagues[8].name` · native | = anchors | OK; loose `7cd6b886…` | X `c6773f3c1d53497eba86e212ab340577c8670cc574701814dc1dcc4e1f08b4fb` (390 197 792); original `48aab4a93150d3eb5dbc43c2e9b58d3584192ec828189c9a5bcf37d6966ff369` | **ready** | G0 not yet observed |
| 5 | NHL 17 PS4 | Norway → Rest Of World | `stockleagues[7].name` · native | = anchors | OK; loose `03a01ed3…` | X `3dd607f7b6ca31f482f300e68c6e24781c03eaab810c5d6ab3d9d84353c0e5fc` (273 371 840); original `3f659305de18bb5a796c8198d4f1c953535dfad776c90fec06a11e8f47a6f366` | **ready** | G0 not yet observed |
| 6 | NHL 16 PS4 | Norway → Rest Of World | `stockleagues[7].name` · native | = anchors | OK; loose `f36a3b7a…` | X `fc64342c2dfe5c25671e9e1cb94a2ac29d1489e762c613ac04ca7a3dcd1e8b55` (299 883 104); original `0db9701aae0a3f0f6f5d5b0d9b30aafa02374b20daf3ca5d9ea3e93eabe1408f` | **ready** | G0 not yet observed |
| 7 | NHL 15 PS4 — base | Legend Teams → Rest Of World | `stockleagues[12].name` · native | `inxp00.big` #0 = anchor; dict = anchor | OK (EB-V1); loose `48ef259f…` | X `5d011a068b6c87c072af84ad1ad7045e5af62a2c12248cbed82eee45fd55b422` (266 430 512); original `b6f6c245e2135284f33baa3a7b47a91ae5a719181f25332fbee9842bb6235d22` | **ready** | which DB the game loads (base vs patch) |
| 7b | NHL 15 PS4 — patch | same | same row on the patch DB (`abe36f58…`, 16 981 204 B) · native | `patch_inxp00.big` #0; its own dict `1e873d7e…` | OK; loose `eced9bb3…`; declared archive_size preserved (unresolved) | X `33bcce79123117a1f97102cb0120352dee45f4f3ae12e3ece4f85dbd230b7b65` (872 223 088); original `629091b6786cd687…` (full value in the manifest) | **ready** | same; test independently, restore between |
| 8 | NHL Legacy PS3 | Norway → Rest Of World | dictionary `TXT_LEAGUE_NORWAY` ← `TXT_ROW_LEAGUE` · **by-reference** (mapping traced on Xbox 360; PS3 executable not inspected) | hdd0 dict = disc = anchor `70814551…`; hdd0 DB is a prior user edit (preserved) | OK (LOC-W1): 1 pending dictionary key, 7 B differ (slot + body CRC), restored; **desktop exercised** (§6) | dict X `b24daa9fa80d139f…` (1 758 212) for **both** rows `nhllegacy-ps3-hdd0` and `-disc`; original `70814551…` | **experimental candidate ready**; live copy to decide first (hdd0 vs disc) | binding hypothesis; which copy RPCS3 reads |
| 9 | NHL Legacy Xbox 360 | Norway → Rest Of World | same key · **traced** (`default.xex` static table) | loose tree dict = X360 anchor `f00926ce…`; packed `cacheboot.big` `b558b4f6…` #3 | OK: loose row and packed row (member + loose replaced in one transaction) | loose dict X `6e80ecf388a1f593…` (1 759 044); packed archive X `67c0b46d6dcb993b…` (31 705 408) | **candidate ready** (two rows: identify the tree Xenia launches by hash first) | which tree Xenia runs |
| 10 | Legacy Recompiled | Norway → Rest Of World | same key · by-reference (recompiled from the traced executable; recompiled loader not inspected) | `game\cacheboot.big` = Vanilla tree's | OK (archive member) | archive X `67c0b46d6dcb993b…`; original `b558b4f6…` | **experimental candidate ready** | recompiled loader behaviour unknown |
| 11 | NHL 15 PS3 | Legend Teams → Rest Of World | `TXT_LEGEND_TEAMS` ← `TXT_ROW_LEAGUE` · hypothesis | hdd0 = disc = anchor | OK (LOC-W1) | dict X `846b3e5678865627…`; original `5f3f8f20…` | experimental candidate ready | binding hypothesis; hdd0 vs disc |
| 12 | NHL 15 Xbox 360 | — | same · hypothesis | **not installed** | campaign row (anchor) | none | not installed | install |
| 13 | NHL 14 PS3 | Legend Teams → Rest Of World | same · hypothesis | hdd0 = disc = anchor | OK | dict X `4ae64f3f0a7127b5…`; original `d7bf2a6f…` | experimental candidate ready | as 11 |
| 14 | NHL 14 Xbox 360 | — | — | not installed | campaign row | none | not installed | install |
| 15 | NHL 13 PS3 | Legend Teams → Rest Of World | same · hypothesis | hdd0 = disc = anchor | OK | dict X `8ce5103da1e56300…`; original `69cc4f75…` | experimental candidate ready | as 11 |
| 16 | NHL 13 Xbox 360 | — | — | not installed | campaign row | none | not installed | install |
| 17 | NHL 12 PS3 | LEGEND TEAM → REST OF WORLD | same · hypothesis | folder copy = anchor; **RPCS3 registers the `.iso`** | OK | dict X `a4e710dcc9b8a80e…`; original `3327bc3a…` | **blocked for placement**: the live source is the ISO (no loose file to swap) | ISO-registered title |
| 18 | NHL 12 Xbox 360 | — | — | not installed | campaign row | none | not installed | install |
| 19 | NHL 11 PS3 | PROSPECT TEAMS → REST OF WORLD | `PROSPECT_TEAMS` ← `TXT_ROW_LEAGUE` · hypothesis | disc = anchor | OK | dict X `b64086fd1a17d0f5…`; original `48e3cd43…` | experimental candidate ready | binding hypothesis; save acceptance open |
| 20 | NHL 11 Xbox 360 | — | — | not installed | campaign row | none | not installed | install |
| 21 | NHL 10 PS3 | NATIONAL LEAGUE → CUSTOM TEAMS (test label; no ROW key on NHL 10) | `TXT_LEAGUE_SWISS_LEAGUE` ← `CUSTOM_TEAMS` · hypothesis | disc = anchor | OK | dict X `b07e03f6799f6434…`; original `954d1617…` | experimental candidate ready | codes 8/9 unidentified |
| 22 | NHL 10 Xbox 360 | — | — | not installed | campaign row | none | not installed | install |
| 23 | NHL 09 PS3 | O2 Extraliga → Custom Teams (test label) | `TXT_LEAGUE_O2_EXTRALIGA` ← `Custom_Teams` · hypothesis | disc DB = anchor; disc dict `537319ce…` (**≠ anchor**, private fixture) | OK | dict X `b991c8716528c50e…`; original `537319ce…` | experimental candidate ready | code 8 "CEN" unidentified |
| 24 | NHL 09 Xbox 360 | — | — | not installed | campaign row | none | not installed | install |
| 25 | NHL 08 PS3 | SMLiiga → National (test label) | `SMLIIGA` ← `National` · hypothesis | disc DB = anchor; disc dict `b9fb3361…` (≠ anchor, private fixture) | OK | dict X `0233dbc16556fc5c…`; original `b9fb3361…` | experimental candidate ready | — |
| 26 | NHL 08 Xbox 360 | — | — | not installed | campaign row | none | not installed | install |
| 27 | NHL 07 Xbox 360 | — | **no dictionary key family** (§2) | not installed | campaign row (browse, code 5 raw) | none | **blocked** | label source untraced (executable / UI data); LOC-W1 does not apply |
| — | NHL 07 PS3 (research-only) | — | — | not installed | campaign row | — | out of scope | — |

Full identities for every row: `local/runs/league-parity-batch/manifests/<row>.json` and the
per-row reports (`reports/<row>.txt`, `reports/gen7-<row>-candidate.txt`).

## 4. The rename route on older titles — what it is and is not

* The edit is a 4-byte text-pointer reassignment in the title's own `nhl_eng_us.db` plus the body
  CRC word (independently re-verified per file — all 25 committed dictionaries and every installed
  one move exactly one CRC scope); the source key keeps its hash and record; no key is added, no
  text is encoded, the pool is untouched, the length is unchanged.
* Consequences the preview and the pending review show: hash collisions (none on any row here),
  other keys sharing the old or the new string (none on any row here). What no offline check can
  show: **which screens read the key.** The league-code → key table is traced only in NHL Legacy
  Xbox 360's `default.xex`; every PS3 row and the recompiled row carry that as a hypothesis, and a
  label change on screen would corroborate it — a label *not* changing establishes nothing about
  the dictionary write.
* Registered launch sources, determined without launching: RPCS3 `games.yml` registers the disc
  trees for NHL 08–11, 13–15, Legacy and the **ISO** for NHL 12; game-data installs under
  `dev_hdd0/game/` duplicate the disc `db/` + `fe/loc/` for 13/14/15/Legacy (which copy the game
  reads is unverified — hdd0 first is the hypothesis; the checklist tests hdd0 first, disc second).
  Xenia's launch root is not recorded anywhere inspected (two trees exist); the Recompiled build
  runs `NHL Legacy Decomp Test\game\`.

## 5. Ordered runtime test cards (user-driven; nothing placed before Codex approval and the user's go)

Common rules: the card §1 of [league-identity-runtime-qualification](../plans/league-identity-runtime-qualification.md)
(cold start, hash before/after, G0 before any modified boot, E5 closes each title, F12 or attested
user report as the user chooses). Helper: `deploy.ps1 -Target <row> -Mode status|install|rollback`
(`-ExpectedExeSha aeb04892…` for PS4 rows; `-ConfigPre <pre-run config.toml>`); every row's
`status` today: `reports/status-all-2026-09-18.txt`. Manual gates the helper cannot automate are
printed as `MANUAL` and must be recorded by hand: the emulator build line per boot, the
PS3/Xbox/Recomp patch/mod state, the pre-run config. **A `status` PASS is preflight only.**

0. **Session start.** Emulators closed. Snapshot `%APPDATA%\shadPS4\config.toml` to
   `local\runs\league-parity-batch\runtime\config.pre-batch.toml`; record RPCS3 / Xenia / recomp
   build identities (about box, log line). Run `status` for every row and file the transcript.
1. **NHL 18 PS4** (`nhl18-ps4`). G0: Rosters / Edit Player tile HOCKEYALLSVENSKAN (+ Play Now / Select
   Team if reachable), re-entry, exit. Install (`48aab4a9…` → `c6773f3c…`). Two cold boots: REST OF
   WORLD in that slot; SHL/DEL/LIIGA/NLA/NHL tiles unchanged; one Allsvenskan team listed. Rollback →
   `48aab4a9…`. E5 stock boot.
2. **NHL 17 PS4** (`nhl17-ps4`): NORWAY (Stavanger, Vålerenga) → REST OF WORLD; `3f659305…` → `3dd607f7…` → `3f659305…`.
3. **NHL 16 PS4** (`nhl16-ps4`): NORWAY → REST OF WORLD; `0db9701a…` → `fc64342c…` → `0db9701a…`.
4. **NHL 20 PS4** (`nhl20-ps4`): HOCKEYALLSVENSKAN → REST OF WORLD; `10306c2f…` → `2d295097…` →
   `10306c2f…`. First stock boot of NHL 20 on this fork: record profile/first-boot behaviour.
5. **NHL 15 PS4** — two discriminating experiments, restore between them, never both at once:
   5a `nhl15-ps4-base` (`b6f6c245…` → `5d011a06…`): LEGEND TEAMS → REST OF WORLD? 5b
   `nhl15-ps4-patch` (`629091b6…` → `33bcce79…`). One changing and the other not says which
   archive is loaded for this member; both unchanged says nothing about either (record only).
6. **NHL 19 PS4** — first the quarantine (user decision): `quarantine.ps1 -Mode status` → `-Mode
   quarantine` moves exactly `nhl\data\common\gamemodes\database\t3db\nhl_game.db` (`265e4a68…`) and
   `nhl\data\kettle\loc\eng_us.db` (`62260005…`) to
   `H:\Emulators\ShadPS4\Games\CUSA11126-loose-quarantine\`, hash-verified, manifest written; the
   `.nhlmodproj`, the metadata and the other 2 333 loose files stay. Then `status` must show the
   loose gates PASS. G0 stock boot (with the two files quarantined — record whether anything looks
   different; if the stock boot itself changes, stop and restore the quarantine before any placement).
   6a `nhl19-ps4-patch56` (`0ac608d1…` → `4e69f9c3…`), rollback, E5; 6b `nhl19-ps4-base`
   (`6efa9096…` → `9100d5fe…`), rollback, E5. Precedence is read from which experiment changes the
   label, never from the archive numbering. Finally `quarantine.ps1 -Mode restore` (byte-verified).
7. **NHL 21 PS4** — no action (closed).
8. **NHL Legacy Xbox 360** — identify the tree Xenia launches (hash its `db\nhlng.db` /
   `cacheboot.big` against rows 9's identities) → G0 (league tiles NORWAY) → `nhllegacy-xbox360-loose`
   (`f00926ce…` → `6e80ecf3…`) **or** `-packed` (`b558b4f6…` → `67c0b46d…`) accordingly → two cold
   boots → rollback → E5. This is the only gen7 row whose binding is traced: a changed label here
   is a positive for the dictionary route *and* the executable table.
9. **Legacy Recompiled** (`legacyrecomp`, archive member): same procedure; record the recompiled
   build identity (`nhllegacy.exe` `7264bec6…`).
10. **NHL Legacy PS3** — decide the live copy: run `nhllegacy-ps3-hdd0` first (RPCS3 game data;
    note its `nhlng.db` is a prior edit `7d1e2b8e…` — the baseline label is still NORWAY); if the
    label does not change, rollback and run `nhllegacy-ps3-disc`. Restore between; E5 each.
11. **NHL 15 / 14 / 13 PS3** (`nhl15-ps3`, `nhl14-ps3`, `nhl13-ps3`; hdd0 game data): LEGEND
    TEAMS → REST OF WORLD; rollback; E5. (If hdd0 proves inert on Legacy, use the disc copies —
    additional manifests can be minted from the same candidates by path.)
12. **NHL 11 PS3** (`nhl11-ps3`): PROSPECT TEAMS → REST OF WORLD. **NHL 10 PS3** (`nhl10-ps3`): the
    Swiss league tile → CUSTOM TEAMS (also read the labels the game shows for codes 8 and 9).
    **NHL 09 PS3** (`nhl09-ps3`): Extraliga tile → Custom Teams (read code 8). **NHL 08 PS3**
    (`nhl08-ps3`): SM-liiga tile → National. Each: rollback + E5.
13. **NHL 12 PS3** — no placement (ISO-registered). Optional G0 only, reading the league labels.
14. **Not installed / blocked, no action:** NHL 07–15 Xbox 360 (not installed); NHL 07 (label
    source untraced).
15. **Close.** Every live file re-hashed to its pre-run identity; no `.e1-*` sidecars; NHL 19
    quarantine restored; boot logs, transcripts, screenshots / attested reports filed under
    `docs/validation/evidence/league-parity-runtime/<row>/`.

## 6. Desktop verification of the new workflow (developer observation, not game acceptance)

`docs/validation/evidence/league-locw1-desktop/` — the real Tauri build driven over CDP on a
copied RPCS3-shaped root (NHL Legacy PS3 anchors; the app attached the discovered dictionary on
open): `03/04` Leagues list with dictionary labels (NHL®, SHL, Liiga, DEL, …, Norway), `05` league
7 detail with the key pill — **the screenshot shows `traced` on PS3, which R1 F5 corrected to
`by-reference`; the image is kept as the historical record of that defect, not rerun**, `06` rename preview (Norway → Rest Of World, slot/pointers,
warning), `07` held, `08` Pending edits with the dictionary card (7 bytes; slot + body checksum),
`09/10` copied-target review → apply (dictionary on disk `b24daa9f…`), `11` league 7 now "Rest Of
World", `12/13` restore (`70814551…`, "Norway"). Driver: `cdp-driver.mjs` (no dependencies).

## 7. Remaining blockers and open decisions

| Item | Owner |
|---|---|
| Promote `nhl19-ps4` adapter blocks to established on the recorded witness; relabel/capture NHL 20 and the PS3 08/09 dictionaries (REG-1 §7 of the batch record) | coordinator (C0-REG4) |
| NHL 19 quarantine (two files) — user decision in the runtime session | user |
| NHL 12 PS3 placement (ISO-registered) — an extracted-folder registration or an ISO member route | coordinator / user |
| Xbox 360 NHL 07–15 not installed; NHL 07 label source untraced | user (installs) / research |
| gen7 code → key bindings other than Legacy Xbox 360 | corroborated or refuted by the runtime cards 8–12 |

## 8. Artifacts and cleanup

Retained under `local/runs/league-parity-batch/` (`SHA256SUMS.txt`): `artifacts/<row>/` (3.2 GB:
9 archive candidates, 13 dictionary candidates, loose evidence), `copied/<row>/` (6.4 GB restored
roots = secondary rollback sources; every transaction `restored`), `sources/` (167 MB install-own
members), `manifests/` (21), `deploy.ps1`, `quarantine.ps1`, `tests/` (38 + 8 cases, results),
`reports/`, `prep/` + `probe/` sources, `gen7-offline.sh`. Retired: the NHL 21 batch candidate and
copied root (E1 closed; evidence durable), `local/runs/league-nhl21-e1/` (superseded scratch; its
helper and hashes live in `docs/validation/evidence/league-nhl21-runtime/e1/`), the package build
dir, fixture roots, session scratch. Private fixtures stay at `E:\nhl-studio-fixtures` (outside the
repo; layout in the two private-fixture test files).

# Research lane — STATUS (overnight 2026-09-18; R1 session 2026-09-18 afternoon appended at the end)

Lane: RESEARCH. Dispatch: [overnight-two-agent-campaign-2026-09-18](../../plans/overnight-two-agent-campaign-2026-09-18.md).
Owned dirs: `docs/research/overnight-2026-09-18/` (findings, data, previews) and
`local/runs/overnight-research-2026-09-18/` (probes, logs, scratch, isolated `pylib/`).
Shared: `local/coordination/overnight-2026-09-18/claims/`. **Morning index: [00-ATLAS.md](00-ATLAS.md).**

Evidence vocabulary: **observed** (measured tonight on the named copy), **inferred** (derived, stated as such),
**by-reference**, **unknown**. Nothing is game-verified; no game/emulator was launched; nothing under `X:`, `H:`,
`anchors/`, application code, defs, bindings or fixtures was modified. The only writes outside this lane's
directories were the claim files.

## Claims (all research claims are `ready`; no research claim is active)

| Claim | Ready | Handoff |
|---|---|---|
| `r0-xbox360-source-inventory` | 05:19 | [01](01-xbox360-source-inventory.md) (+ [02 PS3 cross-check](02-ps3-container-cross-check.md)) |
| `p1.1-xbox360-league-code-key-tables` | 05:00 | [10](10-phase1-league-identity-xbox360.md) |
| `p1.2-competition-groups-and-schedules` | 05:34 | [12](12-phase1-competition-structure-and-schedules.md) |
| `p1.3-localization-capacity` | 06:18 | [13](13-localization-free-text-capacity.md) |
| `p1.4-xbox360-league-logo-assets` | 05:25 | [14](14-phase1-league-logo-assets.md) |
| `p2.7-audio-banks-xbox360` | 05:42 | [15](15-audio-banks-xbox360.md) |
| `p2-team-asset-joins-xbox360` | 05:39 | [20](20-phase2-team-identity-and-asset-joins.md) |
| `p2.3-team-localization-keys` | 05:49 | [23](23-phase2-team-localization-keys.md) |
| `p3-portrait-equipment-thumbs-xbox360` | 05:42 | [30](30-phase3-4-portraits-equipment-thumbnails-xbox360.md) |
| `codec-chunklzx-framing` | 06:16 | [40](40-chunklzx-framing.md) |

Developer claim seen: `p1.1-xbox360-iso-member-route` (ready). Developer STATUS (05:32) records R0 and P1.1-X
as consumed.

## Headline findings

1. All ten Xbox 360 titles are installed (extracted trees + ISOs for 10–15/Legacy); every DB/meta/dictionary
   equals the repo anchor. Corrects the "not installed" rows. The inspected Recompiled install has no `_compiled` dir (user-supplied convention; candidate roots in 01 §5).
2. League code → dictionary-key tables **traced** in the NHL 09–15 and Legacy executables (XEX2 decrypt + LZX,
   validated against the user's Legacy dump); exceptions: NHL 09 `DEL`, NHL 09/10 codes 8/9, NHL 11 code 13,
   NHL 12–15 codes 15–19. NHL 07's European league labels were not found in its English dictionary or in plain-text loose ION/db files
   (compressed package members not inspected).
3. Source schedules are DB tables; slot = the team's rank within its league ordered by the global team `index`
   (**corrected R1**; verified on the Legacy NHL opener, the Legacy AHL 68/76-game split and the NHL 10 AHL /
   NHL 11 OHL out-of-order rows — not the raw `index`); 0-indexed dates;
   All-Star row on 30-team titles; conference/division = per-title executable group table (12–Legacy), local
   fields on 08–11, DB-less group ids on PS4.
4. League logos: code → executable logo-index table → `leaguelogos*/l{idx}.big`; the index is the family-wide
   "league art id" (= PS4 `stockleagues.artid`). Team assets: `artid` joins FE/rink/centre-ice logos; arena
   `bouncemap`/`centerline`/`puck` join ice/puck textures 100 %; jersey triple = asset triple (698/698).
5. Team text: `kOtt.artabbr` is the dictionary key suffix for `NHLTeamName_`/`NHLCityName_`/`TEAMLINE1/2_`/…
   (220/222); free-text writes face zero spare rows and per-title Huffman alphabets.
6. Audio: per-id `{id}_goal/_period/_chant.abk` banks in `nocache.big`; music/PBP bank paths confirmed 12–Legacy.
7. Portraits: `nocache.big` (large) / `cache.big` (small) on 13–Legacy, `nocacherender.big` on 12, `fe.big` on 10,
   `data0.big` flat on 08 — closes AS7's open item. Equipment thumbnail counts per title recorded.
8. `chunklzx` framing pinned and validated byte-for-byte (three plaintexts); Studio's walker has three named
   defects; rink logos now decodable (previews rendered; `a`/`b` = current/vintage set and variant slot, inferred).

## Blockers / open (all recorded in the atlas §3)

Recompiled `_compiled` semantics; NHL 07 `CHTM_LEAGUE_*` label source; PS3 executables (encrypted); Xenia
`cache:` precedence; horn fallback; `fe/nhlsched.bin`; PS4 group table (in `eboot.bin`).

## Checks run (all probes are read-only; logs in `local/runs/overnight-research-2026-09-18/logs/`)

`xbox360_inventory.py` (60 MB JSON), `xiso_list.py` (7 ISOs; members extracted only for hashing, deleted),
`xex_image.py` (10 images; Legacy == user's dump), `ptr_table.py` / `league_tables.py` / `group_table.py` /
`league_logo_map.py`, `tdb.py` + `schema_dump.py` (hash resolution matches `tdb-loc`), `eb_list.py` /
`big4_list.py` / `member_map.py` / `asset_grep.py` / `fe_pkg_probe.py`, `refpack.py` + `xpr_preview.py` +
`rx2_preview.py` (previews), `schedule_survey.py` / `sched_slots.py`, `team_asset_joins.py` / `audio_joins.py` /
`team_loc_join.py` / `loc_families.py` / `loc_capacity.py`, `ion_grep.py`, `chunklzx.py` (+ `chunk_struct.py`,
`lzx_try*.py` experiments).

## Owned artifact manifest (final)

- `docs/research/overnight-2026-09-18/**` — 13 Markdown handoffs, `data/` (77 MB: JSON/CSV incl. the 60 MB
  inventory), `previews/` (league-logo and rink-logo PNGs + contact sheets). Durable.
- `local/runs/overnight-research-2026-09-18/probes/*.py` — ~30 probes (durable, ~320 KB).
- `local/runs/overnight-research-2026-09-18/pylib/` — pycryptodome (isolated pip target, 7.8 MB; disposable).
- `local/runs/overnight-research-2026-09-18/logs/` — 45 KB.
- `local/runs/overnight-research-2026-09-18/scratch/assets/` — 45 extracted specimens, 2.6 MB (disposable; the
  chunklzx vectors describe how to re-extract them). `scratch/xex/` (252 MB) and `scratch/iso/` were **deleted**
  at 06:20 (regenerable). No processes left running (stray decoder processes were stopped).
- Disk: E: 7.6 GB free at 06:20 (was 18.1 GB at start; this lane holds ~330 MB of it).

## Restart (if the lane resumes)

Python 3.12 (`python`), Git Bash tools; probes import `../pylib` themselves. Run from the repo root:
`python local/runs/overnight-research-2026-09-18/probes/<probe>.py …`. To re-run the executable-table probes,
first regenerate the images: `python …/xex_image.py "<X:\…\default.xex>" local/runs/overnight-research-2026-09-18/scratch/xex/<title>.bin`.
Environment note: heredocs through the Bash tool corrupt backslash escapes — write scripts as files.
Next useful research (none claimed): PS4 FE logo packages and conference/division names; NHL 07 gen6 team
structure; equipment tables ↔ thumbnail counts (Phase 4); RW4 previews for ice/jersey texlibs.

---

## R1 session (2026-09-18, ~11:30–13:10) — dispatch [overnight-research-review-and-next-dispatch](../../validation/overnight-research-review-and-next-dispatch.md)

### Plain-language summary

The four review corrections are applied and the vector set is now a single consistent regression reference:
50 archive members from six Xbox titles decode with the final chunklzx decoder to exactly the loose files EA
shipped, with every hash and offset recorded. The schedule "slot" rule was wrong for non-NHL leagues and is now
stated (and proven on three leagues) as the team's rank inside its league. Over-broad "nothing exists" wording
about audio, PS3 equivalence and NHL 07 was narrowed to what was actually inspected, and the Recompiled
`_compiled` note now describes a user-supplied path convention with the candidate roots. Three new handoffs
follow the dispatch's Phase 1 list: league-logo packaging on every platform (eight container chains, with
concrete replace/repack routes), the PS4 conference/division table (found in all seven executables and bound
to the anchors' group ids) plus the PS4 UI data contracts and audio-bank census, and localization coverage per
locale/platform (PS3 dictionaries differ from Xbox; EA's own PS4 patches grow the dictionary).

### Correction dispositions (R1 items)

| # | Item | Disposition |
|---|---|---|
| 1 | chunklzx vectors | **Done.** `data/chunklzx-vectors/manifest.json` + README regenerated (50 vectors, NHL 11/12/13/14/15/Legacy; archive/input/output SHA-256, entry/offset/size, chunk & sub-frame counts, last-frame sizes, decoder identity, plaintext equality on all 50); superseded padded/unpadded rows and the sibling `.sha256` removed; handoff 40 §2 rewritten, §2b lists required behaviour + malformed cases; the probe's own alignment defect (step-by-4 never terminating on `zsize ≡ 2 mod 4`) found and fixed. No family-wide claim beyond the six sampled titles' `attribdb`/`rendering/logo` members. |
| 2 | schedule semantics | **Done.** 12 §1.2 rewritten: slot = rank within league by global `index`; evidence = Legacy NHL opener, Legacy AHL 68/76-game Pacific split (BAK/ONT/SD/SJ/STO), NHL 10 AHL (WBS/WOR) and NHL 11 OHL (Mississauga) out-of-order rows where rank-by-index ≠ record order; PS4 layouts corrected; §1.4 editing/import/generation requirements added; atlas/STATUS updated; `data/schedule-slots/` (25 anchors). |
| 3 | absence/equivalence claims | **Done.** 15 (audio: names only; generic banks and PS4 unmeasured — PS4 banks now located in 17), 02 (PS3: layout/families only; dictionary differences now measured in 13 §2), 10 (NHL 07: exact inspected data), atlas/STATUS wording. |
| 4 | `_compiled` | **Done.** 01 §5 reworded: user-supplied convention, inspected-instance observations, candidate root table with provenance; no user investigation demanded; atlas/STATUS updated. |

### Freshly executed checks (this session) vs inherited results

Executed now: `sched_slots_all.py` on 25 anchors + NHL 07 via `stockteamtable` + `sched_pairs.py` (NHL 11 OHL, NHL 10 AHL, NHL 07);
`chunklzx_vectors.py` twice (pass 1 without, pass 2 with archive hashes — 50/50 plaintext-equal);
`leaguelogo_pkg_xplat.py` (77 Xbox/PS3 packages) and `ps4_leaguelogos.py` (712 PS4 members decoded, GNF T#
parsed); `ps4_group_table.py` on 7 eboots + `ps4_groups_build.py` (every anchor group id resolved except
77/78); `ps4_service_defs.py` (8 areas), `ps4_member_grep.py` (audio census); `loc_locales.py`,
`loc_capacity_files.py` on 27 PS3 + 43 Xbox + 9 PS4 dictionaries; DB identity of NHL 17/18 anchors vs the
H: installs. Inherited (not rerun): the overnight executable-table traces (10, 14 §1), the team/asset joins
(20, 23), portrait/thumbnail counts (30), the R0 inventory hashes.

### New handoffs / data

16 league-logo packaging (all platforms), 17 PS4 groups + consumer leads, 13 §2 localization coverage,
12 §1.2/§1.4 rewritten; data: `chunklzx-vectors/manifest.json`, `schedule-slots/*.json`,
`leaguelogo-packages-xplat.json`, `ps4-leaguelogo-members.json`, `ps4-competition-groups.json`,
`ps4-groups/*.json`, `ps4-nhl21-service-defs-digest.json`, `ps4-nhl21-audio-banks.csv`,
`localization-locales.json`, `localization-capacity-{ps3,xbox360-locales,ps4-extra}.json`.
Claims: `r1-corrections-and-chunklzx-vectors`, `p1.4-league-logo-packaging-xplat`,
`p1.2-ps4-groups-and-consumer-leads`, `p1.3-localization-coverage` (all ready); `codec-chunklzx-framing`
result updated.

### Unresolved gaps

Atlas §3 items 9–12 (PS4 ids 77/78 keys and the NHL 17 array-order anomaly; PS4 hornbank slot ids / horn
`Filter`; loader precedence for every dual-copy case; gen7 acceptance of a grown dictionary). Still no
consumer *tracing* on PS4 (static tables + data contracts only). Not covered: PS3 `RW4ps3` texlib layout
(08–11 logos), the runtime meaning of schedule `round`/`status`, PS4 non-English dictionaries on 16–18/20.

### Cleanup

Scratch: `local/runs/overnight-research-2026-09-18/scratch/assets` (2.8 MB, overnight specimens — kept; owner
research, regenerable) — nothing else; session scratchpad holds only `clzx/pass2.log`. No decrypted
executables, no extracted PS4 members retained (dictionaries extracted for measurement were deleted). E: free
space 5.5 GB (unchanged by this session within 0.1 GB; the 79 MB `data/` tree grew by ~2 MB). No processes
left running (one stalled overnight-probe process from the first vector pass was killed after the alignment
fix). Nothing modified under `X:`, `H:`, `anchors/`, application code or developer-owned paths; no staging,
commits or pushes.

---

## Follow-up session (2026-09-18, ~13:15–14:20) — dispatch [research-r1-acceptance-and-phase1-followup](../../validation/research-r1-acceptance-and-phase1-followup.md)

### Plain-language summary

Four bounded items, three delivered as new handoffs and one as a correction: (1) the early PS3 texture
format is now pinned — RenderWare texlibs on PS3 are plain row-major ARGB8 / DXT5 with no swizzle, and every
one of 36 league logos decodes pixel-identical to its Xbox counterpart, so a small PS3 reader/writer is a
bounded task; (2) the PS4 group mapping now cites each install's own executable and database, the NHL 20
rows were rebuilt from the real NHL 20 database (the repo anchor is an NHL 19 copy), and the two unresolved
ids plus the NHL 17 ordering anomaly are stated as unknowns with the search that failed to resolve them;
(3) goal-horn numbers are shown to be the same on Xbox and PS4 for NHL 15 and renumbered from NHL 18, the
PS4 custom-horn fields are catalogued, the single PS4 horn bank's header is read but its index is not, and
the minimal in-game tests are listed; (4) dictionary growth is specified as twelve measured invariants, a
transformation, per-family placement, ten test cases and four separate gates.

### New handoffs / data / claims

18 RW4ps3 texlib layout (`data/rw4ps3-leaguelogo-survey.json`); 19 goal-horn ids + hornbank
(`data/ps4-goalhorn-survey.json`); 20 dictionary growth (`data/localization-invariants.json`); 17 provenance
section + `data/ps4-install-witnesses.json` + rebuilt `data/ps4-competition-groups.json` (witness blocks);
12 NHL 20 note + `data/schedule-slots/slots-ps4-nhl20-install-1e200478.json` (the mislabeled
`slots-ps4-nhl20.json` removed). Claims (ready): `codec-rw4ps3-texlib`,
`p1.2-ps4-groups-provenance-and-unknowns`, `p2.7-goalhorn-ids-hornbank-static`,
`p1.3-dictionary-growth-invariants`.

### Actual checks (all freshly executed, read-only)

`rw4ps3_survey.py` — 36 PS3 logos (NHL 08/09/10/11 × 4 families × l0/l1/l5) decoded under the linear
hypothesis and compared pixel-wise with the Xbox `RW4xb2` decode: MAD 0.0 on all 36; Legacy PS3 rink texlib
`texlib_27_0_0.rpsgl` rasters 1–2 MAD 0.0, raster 0 MAD 0.47 vs the Xbox chunklzx-decoded texlib; mip-chain
sums equal slot sizes on all six rasters. `ps4_install_witnesses` — eboot/DB SHA-256 for seven installs;
anchor-vs-install hash comparison (NHL 20 anchor = NHL 19 base confirmed; others equal). `ps4_string_runs.py`
— every RELATIVE-reloc string-pointer run (stride 8–32) in the NHL 21 eboot searched for `ALUMNI`/`PROAM`.
`ps4_groups_build.py` rebuilt from install DBs; NHL 19 base vs `patch_inxp_56` group usage diffed (only Pro Am
team count differs). `sched_slots_all.py` on the real NHL 20 DB. `ps4_horn_survey.py` on NHL 15/18/21 install
DBs; NHL 15 Xbox vs PS4 per-arena horn pair comparison (39/39 equal). `hornbank.sbr` header/DSET dump.
`loc_invariants.py` on six dictionaries. Inherited: everything in the R1 and overnight sections.

### Remaining unknowns

PS4 ids 77/78 and the NHL 17 mechanism; SBR DSET record format (id → sample); goal-horn `Filter`; runtime
acceptance of grown dictionaries per family; PS3 `RW4ps3` beyond logos; PS3 RefPack trailing-pad tolerance.
Each has a bounded user test or next research item recorded in its handoff.

### Cleanup

Scratchpad extractions (PS4 DBs/dictionaries 117 MB, hornbank tail, RW4 specimens) deleted after
measurement; no game bytes retained outside `local/runs/overnight-research-2026-09-18/scratch/assets`
(2.8 MB overnight specimens, unchanged). No originals modified; nothing staged/committed/pushed; no launches.
E: free 5.5 GB (unchanged).

---

## Growth-contract clarification (2026-09-18, ~14:25–14:50) — dispatch [research-followup-review-and-growth-contract-dispatch](../../validation/research-followup-review-and-growth-contract-dispatch.md)

Handoff 20 rewritten (rev. 2) as an implementation-ready LOC-W3 contract; handoff 18 gains §1b (exact
sample coverage: 6/8/10/12 rows, absent combinations named, no full-grid claim); new census
`data/localization-order-census.json` (95 dictionaries). Fresh checks: `loc_order_census.py` on 95 files
(descents with row numbers, duplicates, nulls, shared pointers, reserved words). Corrections to rev. 1:
null text pointers exist on every title (13–37 per file), not only PS4; anomalous order = NHL 08/09/10 (all
locales) and `lng_lg`; reserved record bytes are always zero. Contract points: insertion only into
strictly-ascending files at count(hash <) with existing order preserved (class-B insertion refused, text
growth allowed); preserving serializer with byte accounting vs canonical rebuild with semantic proof and an
explicit allowed-difference set; Studio-touched inputs (repoint/retext) supported with sharing preserved;
row-identity duplicate-hash checks; runtime gates R-A (existing queried row reads appended text) vs R-B
(new-key lookup via `stockleagues.name` / `artabbr`-derived keys); placement keeps patch-copy, image
same-length and launch-source uncertainties. Claim `p1.3-dictionary-growth-contract-rev2` (ready). No
scratch created; nothing modified outside research paths.

---

## LOC-W3 revision 3 (2026-09-18, ~15:05–15:25) — review [loc-w3-revision2-review](../../validation/loc-w3-revision2-review.md)

Handoff 20 corrected to rev. 3 from the existing 95-file census and six-file invariants survey (no new
extraction, no probes run beyond re-reading the JSON): (1) class A = nondecreasing hashes / no descent, equal-hash
runs (the PS4 `TXT_STATS_RANK` pair) kept intact, new-key collision refusal kept as its own rule; (2) preserving
serializer appends after the physical end of the original pool (slack included) before the relocated EOF CRC,
with `db_size`, `max/current_records`, record shifts, pool start shift and the four CRC words tabulated;
(3) canonical rebuild proved by decoded row equality + preserved opaque metadata + individually accounted
differences (no universal byte-identity; C2 compares to the requested result; expected ranges derived
independently of the serializer); (4) baseline validation excludes exactly the requested rows/sharers, T5/T7/T8/T9
reworked, existing-key refusal scoped to insertion; (5) B5 = nulls in 93/95 files, none in the two NHL 07 anchors;
census composition stated (25 anchors, 27 PS3, 43 Xbox; PS4 NHL 21 via the invariants file). Atlas row 20 and a
corrections entry updated. Claim `p1.3-dictionary-growth-contract-rev3` (ready). No scratch created; scratchpad
empty; nothing staged/committed/pushed; no launches.

---

## P1-B research (2026-09-18, ~15:40–17:30) — dispatch [loc-w3-r3-acceptance-and-p1b-research-dispatch](../../validation/loc-w3-r3-acceptance-and-p1b-research-dispatch.md)

New handoff **21** (competition rules and mode initialization) + `data/competition-rules-inventory.json`
(29 main DBs incl. the real NHL 20 install and NHL 19 patch; all 46 committed Season/Dynasty/Be-A-GM saves —
inner TDB extracted read-only from every container style: raw/zlib/PS3 multi-part/PS4 nested LZ4/NHL 15
chunkzip; PS4 eboot + Legacy image identifier sets). Findings: no points/tiebreak/qualification/playoff/cap/
calendar data in any main DB (settings + executable); mode saves copy the source schedule verbatim (slots →
`leagueteams.index`, AHL +30/+31, All-Star re-numbered, Franchise adds generated preseason and merges leagues
into one date-sorted index), calendar derived from the copied first/last dates (start −3/−20, end −1, deadline
−45/−41) and fixed month/days; `leaguestates` = (type, tier, value) store with title-specific numbering, labels
inferred by value signature only; tournament tables are phase templates (knockout placeholder slots; CHL's 32
participants are not a league); expansion placeholder observed in the NHL 18 GM save. Six user cards. Atlas
family map reconciled (PS3 dictionaries not identical; PS4 layouts/logo formats title-specific), row 21,
question 14. Probes: `p1b_mode_init_survey.py`, `save_db.py`, `p1b_lib.py`, `p1b_sources.py`; `lz4` added to
`pylib`. Claim `p1b-competition-rules-and-mode-init` (ready). Scratch (`scratchpad/p1b`, 264 MB of extracted
inner DBs) deleted after the survey — nothing retained; nothing staged/committed/pushed; no launches.

---

## P1-B-R1 (2026-09-18, ~17:45–18:20) — review [p1b-review-and-r1-research-dispatch](../../validation/p1b-review-and-r1-research-dispatch.md)

Handoff 21 rewritten as rev. 2: observed / inferred / unresolved separated (schedule-copy correspondence and
calendar offsets are stock-data measurements; the initialization mechanism and any edited-input behaviour are
not established; "+1" summary error fixed to −1); coverage reconciled with a new reproducible comparison
(`data/p1b-schedule-copy-comparison.json`, `p1b_schedule_compare.py`: exact candidate sources, compared fields,
unmatched tuples with multiplicity, `leagueteams`-based translation, positional/`index` equality) — 46 saves
opened, 44 with a copy incl. the three NHL 07/08 raw `ULcv` tables, 2 NHL 10 Franchise specimens without a
copy; exceptions recorded (All-Star home/away swapped on 13–15 + PS4 15, NHL 08 All-Star at 59/60, NHL 18 GM
`ahl31` placeholder id 61); absence claims narrowed to "no identified main-DB representation" with the search
method — the recorded scan surfaced a mode-setup settings catalogue (`stocksettings` + per-context defaults) in the NHL 15 PS4 main DB, the one identified main-DB representation of rule *defaults*; rule editing kept in scope as research-blocked with discriminating next steps; group edits reconciled
with the matrix; tournament phase readings marked inferred; six cards rewritten with controls and bounded
verdicts. Inventory JSON annotated (`notes`). Atlas row 21, Q14 and a corrections entry updated. No scratch
created (in-memory extraction); nothing staged/committed/pushed; no launches.

---

## P1-B-R2 (2026-09-18, ~18:30–18:50) — review [p1b-r1-review-and-r2-dispatch](../../validation/p1b-r1-review-and-r2-dispatch.md)

Handoff 21 rev. 3: plain summary reconciled (leads with the NHL 15 PS4 `stocksettings` catalogue — defaults,
not player-selected values; 46 processed / 44 with a populated regular-season comparison / 2 NHL 10 Franchise
specimens without a copy; three exception classes; "generated preseason" labelled inferred); processing coverage
(43 metadata-named + 3 raw `ULcv`) separated from correspondence outcomes, each specimen with an explicit
`disposition`; `p1b_schedule_compare.py` no longer slices residuals — the comparison JSON holds every unmatched
tuple with multiplicity plus an exhaustive characterisation (NHL 18 GM: all 82 copy residuals involve the unmapped
id 61, all 82 source residuals slot 30); card verdicts narrowed with named alternatives and next controls
(CRC/structure validity, active placement); World Cup slot binding kept inferred/read-only with a new card P1B-7;
phase names not authorised for UI; P1-A accepted offline, P1-C delivered-under-correction per the matrix/dispatch.
Atlas row 21 + corrections entry. Temporary patch script in the scratchpad removed; scratchpad empty; nothing
staged/committed/pushed; no launches.

---

## P1-B-SETTINGS (2026-09-18, ~21:55–22:45) — dispatch [p1b-r2-acceptance-and-settings-research-dispatch](../../validation/p1b-r2-acceptance-and-settings-research-dispatch.md)

### Plain-language summary

The NHL 15 PS4 settings catalogue is now fully mapped: five tables with exact layouts, every join checked, 57
screen/context ids named by the numbering that NHL 16 and NHL 21 still use, per-screen defaults for the league
and competition settings, and the changes EA itself made to these tables in the 01.06 patch. Four columns store
"raw plus minimum" values rather than signed numbers — established three independent ways and confirmed row by row
against a compiled copy of the same catalogue inside the game executable — and Studio's parser currently reads
those four columns wrongly (a finding handed to the developer, not fixed here). Displayed choice labels are bound
from the executable's label arrays (order inferred) and, on NHL 17–21, from the game's own explicit choice lists.
The same catalogue exists as CSV files inside `inxp_00.big` on every PS4 title from NHL 16, as a compiled table in
the NHL 10–15/Legacy executables (no per-screen defaults), and not at all on NHL 07–09. A narrow one-cell
defaults-editing contract for NHL 15 PS4 is proposed; where a player's chosen value is stored remains unknown
(user card P1BS-1 prepared, not executed).

### New handoff / data / claim

Handoff **22** ([22-settings-catalogue-defaults-and-cross-title-equivalents.md](22-settings-catalogue-defaults-and-cross-title-equivalents.md));
data: `p1bs-nhl15-ps4-settings-catalogue.json` (302 KB), `p1bs-settings-cross-title-map.json` (550 KB),
`p1bs-ps4-setting-csv-catalogues.json` (1.3 MB), `p1bs-gen7-setting-tables.json` (233 KB); atlas row 22, Q14
extension, corrections entry. Claim `p1bs-settings-catalogue-defaults` (ready). Probes: `p1bs_catalogue.py`,
`p1bs_eboot_runs.py`, `p1bs_eboot_setting_table.py`, `p1bs_ps4_xtitle.py`, `p1bs_gen7_setting_table.py`,
`p1bs_csv_catalogue.py`, `p1bs_build_map.py` (all read-only; `p1bs_eboot_runs.py` inlines the loader of
`ps4_string_runs.py` because that probe runs `main()` on import — the older probe is untouched).

### Checks actually executed (fresh, this session)

`p1bs_catalogue.py` on the NHL 15 PS4 anchor (`ae9d1d56…`) and on the patch 01.06 DB extracted read-only from
`patch_inxp00.big` (`abe36f58…`, size-consistency 213/223 under offset vs 0/223 two's complement on both);
row-level base/patch diff; `p1bs_eboot_setting_table.py` on the NHL 15 eboot (223 records at `0x3d60080`; id
equality 222/223 offset, 0/223 raw; 186/223 rows equal on all six fields); `p1bs_eboot_runs.py` windows/around
for the label arrays and the 796-entry schema-name list; `p1bs_ps4_xtitle.py` on the six NHL 16–21 eboots (FE
names present on 16 only, no descriptor table, label arrays on all; `stocksettings` names absent); member-name
grep over all `databigs` of the seven PS4 installs (`setting_csv` found on 16–21, not on 15); extraction +
chunkzip decode of the 9–12 `setting_csv` members per title (plain SHA-256 recorded) and `p1bs_csv_catalogue.py`;
`xex_image.py` on the ten Xbox `default.xex` files (hashes in the gen7 JSON) + `p1bs_gen7_setting_table.py`
(tables on 10–15/Legacy; none of the FE names in 07/08/09); `ptr_table.py`/string search on the Legacy image;
NHL 15 PS4 `eng_us.db` key lookups; `grep` of the Legacy loose `fe/` tree for the identifiers (none) and a
header read of `fe/profile/defaultprofile.dat`; `leaguestates` cross-reference from the existing inventory JSON.
Inherited, not rerun: handoff 21's inventory/comparison, handoff 17's witnesses. No launches; no live source or
image written; no application/defs/bindings edits; nothing staged, committed or pushed.

### Unknowns carried forward

Consumer tracing (DB vs executable copy on NHL 15; screen vs global default precedence); chosen-value
persistence (profile container / mode save) — card P1BS-1; `leaguestates` type bindings; `BaseLeague` 11 vs 14,
`NumberOfTeams` 122 5 vs 4 label-count mismatches; gen7 per-context defaults and labels; NHL 07–09 system;
whether `tdb-defs`' two's-complement default is wrong for every EA negative-rangelow field (developer question);
NHL 19 patch archives / real NHL 20 install CSV members not diffed.

### Temporary-artifact manifest and cleanup

Owned scratch root `H:\TEMP\claude\…\scratchpad\p1bs` (ten regenerated Xbox flat images 274 MB, the extracted
NHL 15 patch DB 17 MB, extracted `setting_csv` text members, intermediate JSON/CSV/log files — 121 files,
286 426 253 bytes) — **removed** after containment and reparse-point checks (0 files outside the root, 0
reparse points); scratchpad now empty. No processes left running (the one background image batch completed).
Nothing retained outside `docs/research/overnight-2026-09-18/**`, the probes directory and the claim file. Disk:
E: 36 GB free, H: 33 GB free at closeout (the research data tree grew by ~2.4 MB).

---

## P1-BS-R1 (2026-09-18, ~23:00–23:30) — review [p1bs-review-and-contract-correction-dispatch](../../validation/p1bs-review-and-contract-correction-dispatch.md)

### Plain-language summary

Handoff 22 is corrected to revision 2 from the existing data (no new extraction): the executable agreement is
stated with its exceptions (222 of 223 rows on the setting identity and minimum columns, 186 of 223 on all six
numeric columns, every difference listed); wording that promised what a screen offers or which default wins is
withdrawn; the defaults-editing contract now names the 20 eligible rows and excludes the text, ambiguous and
unresolved ones, preserves stock anomalies, rejects only defects an edit would introduce, keeps a second edit
eligible, and requires bit-level preservation inside the edited record with every checksum byte accounted
individually; the user test card gains profile/placement controls, a second distinct treatment and verdicts that
never treat an unchanged field as proof; and a four-site metadata table shows the affected fields exist only in
the NHL 15 PS4 metadata (base and patch identical), with a field-id collision that makes table-and-field keying
necessary for the queued parser fix.

### Checks actually executed

Re-read of `data/p1bs-nhl15-ps4-settings-catalogue.json` (exception enumeration: 26 `default`, 14 `max`, 12
`size`, 1 `min`, 1 `id`/`style`, record 222 empty in the executable); metadata scan of 55 `*-meta.xml` files
(27 anchor main DBs + NHL 07 + loose localization metadata + private fixtures NHL 08/09 PS3, 19/20/21 PS4) for the
five table ids and four field ids — only NHL 15 PS4 carries them, `CkjP` also names `leaguesimpopups.id` there;
the NHL 15 patch `nhl_game-meta.xml` read out of `patch_inxp00.big` is byte-identical to the anchor metadata
(`d9cf1be4…`); NHL 16 `setting.csv` keys re-read for the eligibility table. No probes re-run, no launches, no
writes outside research paths, nothing staged/committed/pushed. Data files unchanged.

### Cleanup

Two throw-away edit scripts in the session scratchpad removed; scratchpad empty; no processes. Claim
`p1bs-settings-catalogue-defaults` updated (rev. 2, ready).

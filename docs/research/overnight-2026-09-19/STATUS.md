# Research lane status — overnight 2026-09-19

## R1 — P1-BS-CSV (2026-09-19, ~03:45–04:30) — dispatch [p1bs-r1-acceptance-and-csv-parity-dispatch](../../validation/p1bs-r1-acceptance-and-csv-parity-dispatch.md)

### Plain-language summary

The NHL 16–21 (PS4) settings catalogue lives as plain CSV text inside one archive per title; the earlier census was
re-verified against the real NHL 20 install and all twelve NHL 19 patch archives (none carries a copy), so each title
has one stored copy. The accepted NHL 15 "edit one stored default" operation now has a per-title equivalent for
NHL 16–21: which cell(s) hold the default, which values each title allows (from its own files, not NHL 15's), and
what is refused. Editing a cell means re-compressing the text file; the compressed size often changes even when
the text length does not, and Studio's archive planner already handles both the "fits" and "must be appended"
cases on a copied target. What is missing is a compressor that frames and encodes exactly like EA's files (standard
zlib level 9 in 128 KiB chunks reproduces every stock member byte-for-byte; Studio's current one does not) and an API
route that accepts a text member. Whether the game shows or uses an edited default remains a user test.

### Package, disposition, scope

P1-BS-CSV — delivered as research handoff **01** (researcher-verified, pending Codex review). Scope: dispatch items
1–5 (source gaps, per-title contract, CSV preservation + packaging, vectors/cards, handoff-22 clarifications).
Research-owned paths only.

### Changed paths

- New: `docs/research/overnight-2026-09-19/{00-ATLAS.md, STATUS.md, 01-ps4-settings-csv-defaults-parity-and-packaging.md}`,
  `data/p1bs-csv-sources.json` (111 KB), `data/p1bs-csv-defaults-contract.json` (484 KB);
  `local/runs/overnight-research-2026-09-19/probes/{p1bs_csv_sources.py, p1bs_csv_contract.py}`;
  `local/coordination/overnight-2026-09-19/claims/research-p1bs-csv.json`.
- Revised (wording only, measured data unchanged): `docs/research/overnight-2026-09-18/22-settings-catalogue-defaults-and-cross-title-equivalents.md`
  (rev. 3, §11 lists the edits), `docs/research/overnight-2026-09-18/data/p1bs-nhl15-ps4-settings-catalogue.json`
  (`readings.value` string only; 309 117 → 309 336 bytes; still valid JSON), `docs/research/overnight-2026-09-18/00-ATLAS.md`
  (row 22 → rev. 3, §4 correction entry).
- Not changed: application code, defs, bindings, tests, coverage matrix, developer files, installs, anchors.
- Baseline: branch `feat/phase5-asset-tooling`, HEAD `8d60a6d`; `docs/research/` is untracked in the working tree
  (as before this session).

### Checks actually executed (fresh, read-only)

`p1bs_csv_sources.py` over 78 `databigs/*.big` of the six installs (65 `setting_csv` members, 65/65 census plain
hashes equal, 0 copies, NHL 19 patch archives 0 hits; eboot SHA-256 = handoff 17 witnesses on all six);
`p1bs_csv_contract.py` (per-title contract, integrity counts, vectors with before/after hashes);
ad-hoc python over the catalogue JSON: eligible defaults ∈ list values on 128/137/171/177/177 rows, `id == value` on
the ten eligible lists, `SHOOTOUT_NUMBER_OF_ROUNDS` value witness, `setting.default_value == contextsetting[-2]` on
32/32 rows per title (18–21), 0 duplicate keys, NHL 16 88 empty defaults (none eligible), no `is_enabled 0` /
`is_visible 0` on eligible rows; chunkzip header/padding byte dump on four members; zlib level-9 native-framing
re-encode byte-identical on 53/53 stock members; throwaway `flate2 1.1.9` (`miniz_oxide`) probe on 26 chunks of
NHL 16/18/21 → 1/26 byte-identical (build deleted). Read-only source inspection of `tdb-archive-codec/src/lib.rs`,
`tdb-archive-big/src/{eb.rs, stage.rs}`, `studio-domain/src/txn.rs`, `studio-api/src/registry/copied_target.rs`,
`Cargo.lock` (flate2 → miniz_oxide). **Not executed:** any Studio binary, `cargo test`, any `stage`/`verify` run,
any launch or write to an install.

### Remaining blockers / limitations

Runtime consumption, context precedence, loader acceptance of re-encoded/retargeted members and selected-value
storage are unknown (cards only). D1/D2/D3 are developer dependencies (handoff 01 §5.3). Xbox 360 / PS3 /
Recompiled gaps unchanged (no data-file representation found in the sources handoff 22 inspected).

### Temporary artifacts / cleanup

- `local/runs/overnight-research-2026-09-19/p1bs-csv/` — `csv-sources.json`, `csv-contract.json` (regenerable copies
  of the published data), `sources.log`, `contract.log`, `tables.md`, `rev3_edit.py` (the handoff-22 edit script,
  kept as the exact record of the wording edits) — owner: research; disposition: delete after Codex review
  (regenerable by the commands in the atlas).
- Session scratchpad `deflate-probe/` (Rust build tree for the miniz_oxide comparison) — **removed** this session.
- No extracted members, no bulk copies, no running processes.

### R1 checkpoint 2 (~04:20) — review addendum reconciled, scratch cleaned

[p1bs-csv-review-and-overnight-addendum](../../validation/p1bs-csv-review-and-overnight-addendum.md) accepted the
inventory/mappings/dependency analysis and bound six clarifications. Applied as **handoff 01 rev. 2** (change list §12;
script `p1bs-csv/rev2_edit.py`). No measured value changed; the JSON provenance note now records `zlib 1.3.1 /
CPython 3.12.10`. New check executed: `global.consistent` on 16/16 eligible rows on NHL 18–21 from the JSON, and the
size-delta count over all 22 edit vectors (11 changed / 11 unchanged; the nine displayed are 5 / 4). Cleanup:
`csv-sources.json`, `csv-contract.json`, `tables.md` and `probes/__pycache__` removed (containment checked, all
regenerable); `sources.log`, `contract.log`, `rev2_edit.py`, `rev3_edit.py` retained (small, record of execution
and of the exact wording edits). Claim `research-p1bs-csv` stays `ready` (rev. 2 noted). **Identities after rev. 2** (for the developer's receipt 02, which
records the contract hash): `data/p1bs-csv-defaults-contract.json` SHA-256 `090184c00eb4ea12083b04cbebe60a1452c040e9f7dd43c03ecbe45ac98086cc`;
`data/p1bs-csv-sources.json` `5a3ee148e9967cac7a1d2f3c8e206f78ff7578b7ab2a83f584415332faa8eb38` (unchanged by rev. 2).

### Next (superseded by R2 below)

Per the overnight brief the queue continues with **R2** (packaging evidence gaps across measured families) after
this checkpoint; Codex review of handoff 01 is requested in the morning. Claim `research-p1bs-csv` → `ready`.

## R2 — packaging size-change requirements (~04:25–04:50) — handoff [02](02-packaging-size-change-requirements.md)

### Plain-language summary

Measured, on every installed archive family, what has to change around a member whose compressed length changes.
The early titles keep a second index file (`.bh`) that must be patched together with the archive (NHL 07 even packs
copies of those indices inside `boot.big`); the inner logo packages have a fixed 64-byte-aligned layout that Studio's
package builder does not reproduce, so a span-preserving re-plan is needed; Studio's RefPack encoder is always longer
than EA's **[superseded by R7: 6/38 unchanged-content re-encodes still fit their slot; placement is decided per actual
encoded replacement]**; EA's chunkzip members everywhere (PS3 and PS4) are exactly zlib level 9 in
128 KiB chunks with a store-if-not-smaller rule; on PS4 16–21 most league-logo members share bytes with other paths,
which Studio's stager currently refuses; and EA's own PS4 archives already contain appended entries beyond a stale
declared size (NHL 15 ×3 and NHL 16 ×1 — four in total; R7 read every hole of the four in full). Everything is packaged as masks, refusals, vectors and three runtime cards.

### Changed paths

New: `02-packaging-size-change-requirements.md`, `data/r2-packaging-survey.json` (619 KB),
`data/r2-encoder-vectors.json`, probes `viv4_index.py`, `pkg_size_change_survey.py`, claim `research-r2-packaging`;
atlas rows/corrections. Not changed: application code, developer files, installs.

### Checks actually executed

Survey (5 parts, read-only) over 9 BIG4 + 118 EB archives, 116 inner packages, 250 RefPack and 240 chunkzip members;
ad-hoc python for `.bh`/BIG4 trailers, `boot.big` Viv4 scans, the PS4 declared-size anomalies, the NHL 21 stored-chunk
rule, S-B shared-span groups; a throwaway Rust binary linking the working-tree `tdb-archive-codec` for 68 encoder
vectors. **Not executed:** Studio binaries, `cargo test`, `stage`/`plan`, launches, writes to installs.

### Cleanup

Survey part files (`local/runs/…/r2-packaging/`) deleted after merging into `data/`; extracted vector members and the
Rust probe build tree deleted; `probes/__pycache__` removed. No processes running.

### Next (done — see R3 below)

R3 — non-logo league imagery (P1-G).

## R3 — non-logo league imagery, P1-G (~04:50–05:05) — handoff [03](03-non-logo-league-imagery.md)

### Plain-language summary

Instead of guessing from file names, the lane read the two front-end asset registries (Xbox/PS3 ActionScript helper,
PS4 JSON asset-path table) and mapped every league/competition-shaped image type to its files, ids, dimensions and
on-screen consumers. League-keyed non-logo art is a short list (playoff-tree conference/league logos, championship
trophies, the PS4 large/collection/Memorial-Cup/Champions logos), scouting-region logos on PS4 are the same bytes as
league logos, and nothing tile- or banner-shaped is keyed by league. Contact sheets are attached for the user.

### Changed paths

New: `03-non-logo-league-imagery.md`, `data/r3-league-imagery.json` (5.3 MB), `previews/contact-*.png` (17),
probes `league_imagery_names.py`, `league_imagery_survey.py`, claim `research-r3-league-imagery`; atlas row/questions.

### Checks actually executed

Name census over every archive of 26 sources; family survey (4 335 gen7 members decoded to texture headers + hashes);
PS4 registry parse ×7 and 3 062 member headers; consumer greps (Xbox Legacy/NHL 11 ION packages; PS4 UI text ×7);
17 contact sheets (XPR2/DDS decode). **Not executed:** launches, writes, BC7/RW4ps3 previews.

### Cleanup

`local/runs/…/r3-imagery/` (survey parts, extracted PS4 registry text files, names.json, log) deleted after merging.

### Next (done — see R4 below)

R4 — league audio consumers (P1-H).

## R4 — league audio consumers, P1-H (~05:05–05:12) — handoff [04](04-league-audio-consumers.md)

### Plain-language summary

The lane read the commentary, crowd, UI-sound and music definitions of every installed family. No league bank
exists, but the definitions contain league-keyed records by name (AHL/NHL/CHL cup lines, per-league matchup intros
on PS4 20–21, World Cup crowd contexts, a CHL UI sound); the league reaches them through compiled condition scripts,
and the Nordic music bank is chosen by locale, not league. Replacement is blocked on the undecoded bank index that
the horn work also needs; extraction and name-level reading are possible now.

### Changed paths

New: `04-league-audio-consumers.md`, `data/r4-league-audio-consumers.json`, probe `league_audio_survey.py`, claim
`research-r4-league-audio`; atlas row/questions.

### Checks actually executed

String scan of every text-shaped audio member (≤ 3 MB) across 17 sources with in-memory wrapper decode; NHL 10 XML
condition listing; GAS header check; Xbox-vs-PS3 Legacy audio member comparison. **Not executed:** launches, writes,
audio decoding/playback.

### Cleanup

`local/runs/…/r4-audio/` deleted after copying the JSON to `data/`.

### Next (done — see R5 below)

R5 — remaining Phase 1 blockers, targeted (developer claims re-read first).

## R5 — targeted Phase 1 gaps (~05:12–05:37) — handoff [05](05-phase1-targeted-gaps.md)

### Plain-language summary

NHL 07's three European league names have no text anywhere in the shipped data or executable (only the keys), so
that cell stays blocked with evidence and a runtime observation card; the NHL 07 texture-container variant that
blocked previews is decoded (contact sheet attached); two coverage-matrix cells lag research that already exists.

### Changed paths

New: `05-phase1-targeted-gaps.md`, `data/r5-nhl07-label-grep.json`, `data/r5-nhl07-texlib-variant.json`,
`previews/contact-nhl07-x360-league-families.png`, probes `nhl07_label_grep.py`, `rw4xb2_nhl07.py`, claim
`research-r5-targeted-gaps`.

### Checks actually executed

RefPack-decoded grep of 18 968 NHL 07 members + loose trees; `default.xex` decrypt (image deleted after the grep);
dictionary row scan; 13 texlib decodes. Developer claims re-read at the boundary (D3 active, D4 next = P1-F; handoff
02 is its input). **Not executed:** launches, writes.

### Cleanup

`local/runs/…/r5/` deleted after copying to `data/`; decrypted image and the scratchpad `refpack-probe/` removed.

### Next (done — see R6 below)

R6 — research ahead: Teams (identity/joins, localization, assets, arenas/ice, kits, horns/PBP/music), starting with
the operations research 20/23 left unresolved.

## R6 — Teams step 1, PS4 field map and asset joins (~05:37–05:48) — handoff [06](06-ps4-team-identity-and-presentation-map.md)

### Plain-language summary

The PS4 team, arena and jersey tables were mapped across all seven titles with their value spaces and joined to the
front-end asset registry: team logos, arena pictures, jersey thumbnails and the Creation-Zone catalogues that back
the presentation id fields all resolve; the team row absorbs arena presentation from NHL 18 on; on-ice textures are
named by slug with no DB field carrying it, which is the next Teams question.

### Changed paths

New: `06-ps4-team-identity-and-presentation-map.md`, `data/r6-ps4-team-field-map.json` (3.4 MB),
`data/r6-ps4-jersey-joins.json`, probe `ps4_team_field_map.py`, claim `research-r6-ps4-teams`; atlas row/questions.

### Checks actually executed

Table/field/value survey over six anchor DBs + the real NHL 20 install DB and its own metadata; registry joins;
jersey triple joins per title and the fourth-component field test; row-count and `fevariant == variant`
verification. **Not executed:** launches, writes.

### Cleanup

`local/runs/…/r6/` (incl. the extracted NHL 20 metadata) deleted after copying the two JSON files to `data/`.

### Late addition (05:55)

Handoff 06 §4b: the PS4 on-ice slug binding is located — `boot.big:nhl/data/common/database/attrib/gccollections.bin`
(`ErtS` collection database, 10–25 MB decoded, id-keyed `centerlogo_{artid}_texpack` / `centerline_{id}_texpack`
collections plus slug-named stadium/jersey/mascot collections; format undecoded). Xbox `attribdb` carries no such
names (gen7 keys art by id in file names).

### Next

R6 Teams step 2: decode the `ErtS` collection container (bounded; NHL 21 specimen SHA `d668bbe1…`, NHL 15
`0684ef97…`), then PS4 team localization keys (`artabbr`-suffixed, cf. handoff 23) and the gen7 cross-title join
table from the existing `xbox360-assets-*.csv`. Not started.

## R7 — correction checkpoint (2026-09-19, review F1–F4) — dispatch [overnight-research-2026-09-19-review](../../validation/overnight-research-2026-09-19-review.md)

### Plain-language summary

The morning review found that several of last night's conclusions went further than the measurements. This checkpoint
re-derives the numbers from the published data and the source archives and rewrites the affected sentences: Studio's
current RefPack encoder does *not* make every logo replacement a size-changing one — six of the 38 unchanged-content
test members still fit their slot, the other 32 do not, and an edited image compresses to an unrelated length anyway,
so the planner must decide from the actual bytes. There are four (not ten) PS4 archives whose declared size is stale,
and every gap in those four was now read in full and is zero. Gap bytes count as free space only after they are read
and verified zero. The package survey holds 129 rows (102 distinct byte contents), not 116. The NHL 07 "no text
anywhere" result is bounded to what was searched (32 members over 8 MB were skipped), the NHL 07 texture variant is
proven on 13 specimens only, "tiles/banners n/a" is a registry-only absence, the runtime cards no longer claim to rule
out causes, and the `ErtS` collection file is a candidate resolver whose binding is still unresolved.

### Changed paths

- Revised (wording only; measured data unchanged): handoffs `02` → rev. 2 (§11 change list), `03`, `04`, `05`, `06`
  (rev. 2 wording marked inline), `00-ATLAS.md` (rows 02–06, Q-R2-3, Q-R6-1, §3 correction entry, §4 commands).
- New data: `data/r2-fit-audit.json` (per-vector fit vs growth, slack bytes verified zero, anomalies, package
  populations, sample denominators, skipped cases), `data/r2-anomaly-hole-scan.json` (every hole of the four
  anomalous archives read in full).
- New probes: `probes/r2_fit_audit.py`, `probes/r2_hole_scan.py`; exact edit records `probes/rev2_handoff02_edit.py`,
  `probes/rev2_handoffs03_06_edit.py` (re-runnable, no-op once applied).
- Claim: `local/coordination/overnight-2026-09-19/claims/research-r7-correction-checkpoint.json` (`ready`).
- Not changed: application code, defs, bindings, matrix, developer files, installs, anchors. No commits or pushes.

### Checks actually executed (read-only)

`r2_fit_audit.py` (≈ 2 s): 68 vectors joined to their slots by (archive, entry index, offset, stored size) with the
package hash re-verified for the 18 inner vectors; slack bytes read on every slot — all zero; result RefPack 6/38 fit
(X-A outer `leaguelogossmall/l0` NHL 10/09/08 +25/+15/+15 B vs 38/39/30 B; inner `leaguelogossmall` X-B NHL 11 +19 vs 60,
P-B NHL 10 +18 vs 61, P-C Legacy +30 vs 42), 32/38 do not (15 inner textures whose 64–1 024 B package growth exceeds the
0–15 B outer slot, 12 BIG4 members with 14–55 B slack, 5 X-A outer packages); chunkzip 21/30 fit under the pre-D4
encoder and 30/30 are byte-identical under the native zlib-9 re-encode (survey rows). `r2_hole_scan.py` (≈ 40 s): NHL 15
`game20.big` 24 853 holes / 5.26 MB, `inxp20.big` 1 308 / 3.06 MB, `patch_inxp00.big` 526 / 0.08 MB, NHL 16 `game_90.big`
42 069 / 3.20 MB — 0 non-zero bytes; first entry beyond the declared word starts exactly at it and the last entry ends
exactly at the file length on all four. Directory read of the seven NHL 07 archives: 15 338 top-level members, 32 above
8 MB (`gamedata/anim/anim.viv`, 31 `rendering/stadium/stadium_*_container_0.rx2`). Population counts from the survey
JSON: 129 package rows, 102 distinct SHA-256, 94 inner RefPack textures, 29 outer wrappers, 6 raw P-A; 118 EB archives
(90 `0x00110400`, 17 `0x00100400`, 7 + 4 `0x00000400`); 250 RefPack and 240 chunkzip sampled members. **Not executed:**
Studio binaries, `cargo test`, `stage`/`plan`, launches, writes to installs.

### Limitations

The fit audit covers unchanged stock plaintext only; it bounds nothing about edited content. The hole scan covers the
four anomalous archives only; other archives' holes remain sampled. The "116" figure could not be reconciled with any
subset of the array and is recorded as not reproducible rather than explained.

### Cleanup

No scratch produced (probes read directly from the installs and write only the two JSON files). `p1bs-csv/rev2_edit.py`
and `rev3_edit.py` were deleted at the end of the session (item 4 of the dispatch) — see the second closing receipt.

## R7b — `ErtS` → the PS4 Attrib/VLT collection registry, parsed (2026-09-19, dispatch item 2) — handoff [07](07-ps4-vault-collections-and-slug-binding.md)

### Plain-language summary

The "ErtS collection database" handoff 06 flagged is the string half of EA's Attrib/VLT vault (`gccollections.vlt` records +
`gccollections.bin` strings/heap, described by `gcschema`). It was parsed to attribute level on all seven PS4 titles with one
reader: the record layout, the two-segment pointer table, the schema's class/attribute descriptors and the key hash (Jenkins
`lookup2`, seed `0xABCDEF00`) are pinned, and every typed reference in the Teams-relevant collections resolves inside the vault.
The binding handoff 06 could only see as strings is explicit data: each database arena row has a `ge_level` whose `ID` equals its
`arenaartid` (every row, every title); each centre-ice logo pack carries the team art id as a typed `Team` attribute and points at
the texture collection holding the slug-named file (66 of NHL 21's 348 packs point at a file whose number is not the pack's —
the reference, not the name, is the binding); each jersey texture pack carries `Team`/`UniformVariation`/`JerseyFit` equal to the
database's `teamartid`/`variant`/`style` and references ten slug-named textures. Open: the selection rule where a triple has two
packs, runtime consumption (unobserved), most attribute names, and the collection keys themselves. No writer is proposed.

### Changed paths

- New: `07-ps4-vault-collections-and-slug-binding.md`, `data/r7-ps4-vault-collections.json` (10.7 MB), probe
  `probes/vlt_collections.py`, claim `research-r7-vault-collections` (`ready`); atlas row 07, Q-R6-1 marked resolved at the data
  level, Q-R7-1..4 added, §4 command.
- Not changed: application code, defs, bindings, matrix, developer files, `docs/audits/` (the NHL 21 head audit is cited, not
  edited), installs, anchors. No commits or pushes.

### Checks actually executed (read-only)

`vlt_collections.py` over the seven extracted `attrib/` trees + six anchors + the real NHL 20 install DB (≈ 5 min): chunk tables,
`ExpN`/`TofC`/`PtrN` consistency (TofC == ExpN order on 100 % of rows; all pointer slots 8-aligned and zero; targets all in the
`.bin`), class-key resolution 258/258 … 476/476, type keys 185/185, 81 attribute names, reference accounting (0 dangling on every
title), the four DB joins per title. Ad-hoc in the session scratchpad (deleted): the hash-function search, ≈ 180 k-candidate
attribute-name brute force, record/heap dumps, collection-key and numbered-vault name negatives, the reference-into-sibling-vault
test, the `(teamartid, variant, style)` triple join on 15/17/18/21. **Not executed:** launches, writes, any Studio binary.

### Limitations

Static correspondence only; nothing about the loader. Attribute names 81/2 828; head/hair classes not decoded; the numbered
sibling vaults inventoried by name and size only.

## Closing receipt — research lane, 2026-09-19 (04:00–05:55)

**Delivered (researcher-verified, pending Codex review; nothing game-verified):** handoff 01 rev. 2 (addendum
reconciled), handoffs 02 (packaging size-change requirements), 03 (non-logo league imagery, P1-G), 04 (league
audio consumers, P1-H), 05 (targeted Phase 1 gaps: NHL 07 labels, NHL 07 texlib variant, matrix lag), 06 (PS4 team
identity/arena/kit map + collection-DB finding); 10 data files under `data/`, 18 contact sheets under `previews/`,
10 read-only probes, six `research-*` claims (all `ready`).

**For the developer (D4 active on P1-F):** handoff 02 §3.2 (chunkzip store-if-not-smaller rule, PS3 identity 240/240),
§4.1 (shared-span private-copy append; NHL 16 `game_90.big` declared-size acceptance), §4.2 (span-preserving BIGF
re-plan; `ArchiveBuilder` does not reproduce stock), §4.3 (`.bh` sidecar patching), §5 (RefPack encoder always
longer). Post-rev-2 identities of the CSV contract data are recorded under R1 checkpoint 2.

**Coverage-matrix wording proposals:** P1-G (handoff 03 §5), P1-H (handoff 04 §4), 1.3 locales / NHL 07 cells
(handoff 05 §3).

**User cards (tomorrow, no review requested now):** P1BS-CSV-1 (rev. 2 controls), R2-BIG4-1, R2-LZX-1, R2-EB-1,
R3-IMG-1/2, R4-AUD-1, R5-07-1.

**Cleanup verified:** `local/runs/overnight-research-2026-09-19/` holds only `probes/` (10 scripts) and
`p1bs-csv/` (two logs, two edit scripts); no extracted members, decrypted images or build trees remain; session
scratchpad empty; no processes running; no writes to installs, anchors, archives, images, saves or application code;
no commits or pushes.

**Reproducible commands:** atlas §4. **Next queue entry:** R6 Teams step 2 (above).

## Second closing receipt — research lane, 2026-09-19 (12:30–13:35), correction checkpoint + R7b

**Delivered (researcher-verified, pending Codex review; nothing game-verified):** the correction checkpoint (handoffs 02–06 rev. 2,
`data/r2-fit-audit.json`, `data/r2-anomaly-hole-scan.json`, atlas/STATUS reconciled — see R7 above) and handoff 07 (PS4 Attrib/VLT
collection registry parsed on NHL 15–21; arena/centre-logo/jersey → slug binding as typed data; `data/r7-ps4-vault-collections.json`).
Claims: `research-r7-correction-checkpoint`, `research-r7-vault-collections` (both `ready`).

**Independently reproducible checks:** atlas §4 — `r2_fit_audit.py` (≈ 2 s), `r2_hole_scan.py` (≈ 40 s), `vlt_collections.py`
(≈ 5 min); the two `rev2_*_edit.py` scripts are re-runnable no-ops once applied. Developer-side items are untouched: the review's
"audit D4 against F1/F2" is input for the developer's independent review, not done here.

**Cleanup verified (dispatch item 4):** `local/runs/overnight-research-2026-09-19/p1bs-csv/rev2_edit.py` and `rev3_edit.py` deleted
(their edits are recorded textually in handoff 01 §12 and handoff 22 §11; `sources.log`, `contract.log` retained); `probes/__pycache__`
removed in both lane directories; session scratchpad emptied (exploration scripts `vlt.py`, `schema.py`, `decode.py`, `t1–t9.py`,
`r7-test.json` deleted after the durable probe reproduced them). **Retained:** `probes/` (10 earlier + `r2_fit_audit.py`,
`r2_hole_scan.py`, `rev2_handoff02_edit.py`, `rev2_handoffs03_06_edit.py`, `vlt_collections.py`), `p1bs-csv/{sources.log,contract.log}`,
the three new `data/` files. **Dependencies of the retained probes:** the 2026-09-18 probes `big4_list.py`, `eb_list.py`, `tdb.py`,
`schema_dump.py`, `ps4_leaguelogos.py` (chunkzip decode) and this lane's `ps4_team_field_map.py`; the installs on `X:`/`H:` and the
extracted `attrib/` trees; the committed anchors. No extracted members, build trees or running processes; no writes to installs,
anchors, archives or application code; no commits or pushes.

**Stop for coordinator review** (dispatch item 4). Queued next: PS4 team-localization keys / gen7 join comparison (research 20/23),
`ge_nhl_skaterhead*` decode with the R7 reader (bounded), numbered-vault naming key.

# C0-AS7 — asset-evidence measurement batch D1 + D4 + D3 — receipt

> **Closed by C0-AS7-CLOSE (section H below, 2026-09-16): committed as `8d60a6d`.** Codex review
> passed after P2/P3 were closed in place (§G); the 25-path boundary of §E was committed
> byte-for-byte, on `9e02acf`, with an empty index before and after. Not pushed.

**Package:** measure the file side of asset discovery and identity on the two copies the C0-AS6
receipt (§D) already identified: the Legacy Xbox 360 rendering archives' member tables (D1), a
per-id census of the PS3 Legacy portrait folders joined to the committed anchor (D4), and the PS3
`patch.big` (D3). Carry evidence into the registry only at the scope it establishes.
**Baseline:** `9e02acfe7e50e0737970fccf3b7d4321bef00f4b` (C0-AS6-CLOSE). **Date:** 2026-09-16.
**Status:** committed as `8d60a6d` (section H); nothing pushed; QA activation parked; no
schema version added; no manifest or lock change; no source copy written.

**Result in one paragraph.** All three archives named by the batch are `EB\0\3` archives of the
16-byte-entry variant the committed reader already handles, so every member table was read through
`tdb_archive_big::eb` at the baseline: `cacherender.big` (12 845 members, all under `rendering/`),
`nocacherender.big` (3 237, all under `rendering/`) and the PS3 `patch.big` (459 members: 83
portrait `BIGF`s, 8 localization databases, 328 `rendering/*.rpsgl`, 13 create-player thumbnails, 15
jersey thumbnails). Names are literal (NUL-terminated ASCII in the archive's own tables); nothing is
inferred, and no member was decoded except the `chunkzip` and raw stored forms the committed
`decode_payload` handles for the PS3 archive. Joined to the byte-identical Legacy anchor, the member
tables give the **file half** of the logo, jersey and equipment identities on an **attributable**
corpus for the first time: every one of the 698 rendered `xjgw` composite ids names a jersey texlib
member, a nameplate member and a sock member; every stored id of **all eight** measured equipment
fields names a member under the HEAD grammar (the 74 stick texlibs are exactly the 74 enum ids);
180 of the 218 stored logo art ids name the declared `texlib_{id}_0_0` member and all 218 name some
`texlib_{id}_{a}_{b}` (348 distinct first components, 130 stored by no team row). Two findings run
the other way: a **ninth** comparison — the goalie-stick folder against `xmja.wXFx`, the field HEAD
`SLOT_SPECS` says the goalie stick *shares* with the skater stick — is contradicted by the listing
(members 6..43 against stored ids 35..112, one in common; a finding about that shared-field claim,
not one of the eight fields failing), and the D4 join shows the portrait identity is **not total on the PS3 copy** — 4 500 of the
5 449 non-zero anchor ids have a loose `playerheads` file and 949 do not (350 outside the observed
regular ranges, 599 inside them), with `patch.big` supplying 33 of the missing and 916 in neither.
**No block was promoted to established** — every candidate that gained its file half still bundles
an unmeasured field, a single-root discovery that cannot cover two archives, or the unobserved
runtime lookup — and **no operation's readiness changed**. Portraits are in neither render archive
(the other eleven Xbox 360 archives were not opened, by package scope), and the committed `chunkzip`
codec fails on 257 of the 329 `chunkzip` members of `patch.big`, all multi-chunk.

---

## A. What was measured

Everything below was read from the two copies C0-POR1 identified, **read-only**, by two examples
built from the baseline tree (`git archive 9e02acf`), and written as records under
`docs/receipts/data/` with specimen attribution, digests and the tool's own provenance (the tool's
and the reader's source hashed at run time; `source_commit = 9e02acf`). Each archive record was produced
**three times** from the copies (before and after the tools were formatted, and again with the
P2-corrected tool — section G) and the census twice; every CSV came out byte-identical every time,
and only the embedded tool hashes and limitation text moved.

### A.1 D1 — Legacy Xbox 360 rendering archives (attributable corpus: the title root's own copy)

| | `cacherender.big` | `nocacherender.big` |
|---|---|---|
| SHA-256 (tool; cross-checked by `sha256sum`, equal) | `e05179835ff2099b01a3085c335f6b8d6cb8de52cd5cf7ecee23e948d586d6d2` | `649cc6f223b147daeb8147e7a834fb5c2eda786d9447063e6a25a6d1da225cfd` |
| size / header `archive_size` | 1 458 552 880 (equal) | 1 938 855 744 (equal) |
| version word / entry stride | `0x00000400` / 16 bytes | same |
| name slot / dir slot / dirs | 35 / 24 / 42 | 25 / 19 / 8 |
| members / distinct paths | 12 845 / 12 845 | 3 237 / 3 237 |
| stored form | 12 844 `chunklzx` + 1 raw XML (`rendering/colorcube/colorcube_pausemenu.xml`) | 3 237 `chunklzx` |
| last payload end = `archive_size` | yes | **no** — ends 6 bytes short (1 938 855 738); offsets all 16-aligned |
| top-level directory | `rendering/` only | `rendering/` only |
| record | `c0-as7-x360-cacherender-members.{csv,json}` | `c0-as7-x360-nocacherender-members.{csv,json}` |

**No `fe/` member in either.** The portrait, crest and create-player thumbnail families are
therefore in one of the eleven archives the package did not scope; the recorder's summary names
`cache.big` / `nocache.big`. That is now a measured absence, not a located member.

**Name shapes** (all-digit components → `N`; `docs/receipts/data/c0-as7-member-shapes.csv`, 424
rows over the three archives). The grammar the profiles declare and what the listing shows:

| Family / slot | Directory (archive) | Members | Shapes the listing shows | Declared template covers |
|---|---|---|---|---|
| logos, rink texlib | `rendering/logo` (cache) | 1 926 | `texlib_N_N_N.rx2` only | `texlib_{id}_0_0`: 307 members |
| jerseys, bundle | `rendering/jersey` (nocache) | 1 869 | 905 `texlib_N_N_N`, 904 `name_N_N_N_cm`, 60 `jersey_N_N_N_N_N_N` | `texlib_{composite}`: 905 |
| jerseys, socks | `rendering/sock` (**cache**, not nocache) | 954 | 946 `sock_N_N_N_cm` + 8 stragglers | `sock_{variant}_cm`: 946 |
| sticks | `rendering/playerstick` (cache) | 74 | `texlib_N` | all 74 |
| skates | `rendering/skate` (cache) | 32 | `texlib_N` | all 32 |
| helmets | `rendering/helmet` (cache) | 348 | ten shapes; 19 `texlib_N` | `texlib_{id}`: 19 |
| gloves | `rendering/glove` (cache) | 575 | `glove_N_{am,nm,sm}` 159, `glove_N_N_{dm,tm}` 212, `feglove_*` 204 | `glove_{id}_{variant}`: 159 |
| goalie sticks | `rendering/goaliestick` (cache) | 78 | `goaliestick_N_{dm,tm}` 76 (+2 `goaliestick_goaliestick_*`) | 76 |
| goalie pads | `rendering/goaliepad` (cache) | 650 | `goaliepad_N_{nm,sm}` 134, `goaliepad_N_N_{dm,tm}` 258, `fegoaliepad_*` 258 | 134 |
| mask paint | `rendering/goaliehelmet` (cache) | 575 | `goaliehelmet_N_N_cm` 539 + mesh/texlib shapes | 539 |
| blockers (prose grammar) | `rendering/blocker` (cache) | 1 296 | `blocker_N_N_{nm,sm}` 264, `blocker_N_N_N_{dm,tm}` 516, `feblocker_*` 516 | — (unspellable, C0-AS6 P2) |
| trappers (prose grammar) | `rendering/trapper` (cache) | 1 288 | 264 / 512 / 512 | — |
| (not a profiled family) | `rendering/player` (cache) | 1 589 | 904 `player_N`, 685 `texlib_N` | recorded in the shape histogram only; not joined (heads are the external workstream) |

**Joins to the Legacy anchor** (`docs/receipts/data/c0-as7-x360-member-join.csv`, 21 rows,
re-derived on every test run from the member CSVs + `anchors/xbox360/nhllegacy/databases/nhlng.db`,
SHA-256 `105d2186…1230` = the PS3 anchor):

| Grammar | DB ids | with member | without | member ids without a row |
|---|---|---|---|---|
| `logo/texlib_{team-id}_0_0` (declared) | 218 (`kOtt.lOnr`, incl. 0) | **180** | **38** (`68;131-139;141-145;148-159;161-165;197;332;361;396;401;410`) | 127 (307 members / 307 distinct first components) |
| `logo/texlib_{team-id}_{a}_{b}` (observed) | 218 | 218 | 0 | 130 (1 926 members / 348 distinct first components, incl. `20000-20085`) |
| `jersey/texlib_{style}_{art}_{variant}` (declared) | 698 rendered `xjgw` | **698** | 0 | 207 |
| `jersey/name_{…}_cm` (observed) | 698 | 698 | 0 | 206 |
| `sock/sock_{…}_cm` (observed, in `cacherender.big`) | 698 | 698 | 0 | 248 |
| `playerstick/texlib_{id}` | 19 (`xmja.wXFx`) | 19 | 0 | 55 |
| same vs `defs/enums/stick-model.toml` | 74 | 74 | 0 | **0** — the member set equals the enum |
| `skate/texlib_{id}` | 8 (`AOOe`) | 8 | 0 | 24 |
| `helmet/texlib_{id}` | 10 (`YUZp`) | 10 | 0 | 9 |
| `glove/glove_{id}_{variant}` | 14 (`NmUR`) | 14 | 0 | 39 |
| `goaliestick/goaliestick_{id}_{variant}` | 19 (`xmja.wXFx`, per HEAD `SLOT_SPECS`) | **1** | **18** | 37 (member ids 6-43) |
| `goaliepad/goaliepad_{id}_{variant}` | 29 (`fMVl.mIUA`) | 29 | 0 | 38 |
| `goaliehelmet/goaliehelmet_{variant}_{id}_cm` | 173 (`fMVl.JOlt`, incl. the 36-row value 0) | 173 | 0 | 107 |
| `blocker/blocker_{model}_{id}_{map}` | 29 (`Kmxz`) | 29 | 0 | 39 |
| `trapper/trapper_{model}_{id}_{map}` | 26 (`OTJR`) | 26 | 0 | 40 |
| `playerheads/p{id}` (both archives) | 5 450 | 0 | 5 450 | — (directory absent) |

Three things this does and does not say. (1) A member name matching a stored id is a *specimen
observation on an attributable corpus* — the file half of an identity, which no committed record
held before. It is not the runtime's lookup, and it is not a decode. All **eight** measured
equipment fields have matching members. (2) The goalie-stick row is a **ninth** comparison and a
**contradiction of HEAD source**: `SLOT_SPECS` says the goalie stick shares `xmja.wXFx` with the
skater stick; the members are named 6..43 and that field stores 35..112. Either another field keys
it or the id space is remapped; the profile records the mismatch and proposes nothing. (3) The logo
template `texlib_{id}_0_0` is not the whole grammar: the trailing pair varies per id, 38 stored art
ids have no `_0_0` member (307 members, 127 unstored first components under that spelling), and the
broader `texlib_{id}_{a}_{b}` shape — 1 926 members over 348 distinct first components — matches
all 218 stored ids with 130 extras, including a `20000-20085` block no team row stores.

### A.2 D4 — PS3 portrait census and anchor join (the C0-POR1 installed copy)

`docs/receipts/data/c0-as7-ps3-portrait-census.csv` — 18 663 rows, one per file under
`fe/ion/artassets/{playerheads,playerheadssmall}/<bucket>/`, each with its label (`current` /
`backup-labelled`), the numeric id read from the name, size, SHA-256, and the `BIGF` member count,
member-3 name, stored size and RefPack declared size (nothing decompressed). Counts reproduce
C0-POR1-R1's exactly: 9 228 + 103 and 9 229 + 103; buckets 3 087 / 3 196 / 3 044 / 4 and
3 089 / 3 196 / 3 043 / 4; all 18 663 are `BIGF` with 6 members and a RefPack member 3; no `other`
file. Observed regular id range 1-12401 plus silhouettes 60000-60003 in both folders; no id has two
current files; no `p0`; every `.bak` has a current sibling.

**Join to `ubPc.lOnr`** (`docs/receipts/data/c0-as7-ps3-portrait-join.csv`, 78 measures, re-derived
by `ps3_portrait_census_joins_to_the_legacy_anchor_and_the_join_record_matches`). Reported apart,
as the package asked:

| Measure | `playerheads` | `playerheadssmall` |
|---|---|---|
| anchor rows / distinct values / zero rows / shared ids | 5 632 / 5 450 / 180 / `2x2;7x2;12x2` | same |
| distinct non-zero ids | 5 449 | 5 449 |
| **ids with a loose current file** | **4 500** | **4 501** |
| ids without | **949** | **948** |
| — of which in **12403..12814** (the 350; *outside the observed regular ranges*, not "above the silhouette range", which starts at 60000) | 350 | 350 |
| — of which **inside 1..12401** (listed by id in the record) | **599** | **598** |
| current files whose id no row stores | 4 728 | 4 728 |
| ids whose current file has a backup-labelled sibling | 103 | 103 |
| `patch.big` portrait members (D3) | 58 | 25 |
| — naming an anchor id / no anchor id | 45 / 13 (`8323;12508;12816;12818;12824-12832` — **eleven above the anchor's maximum**) | 12 / 13 |
| — also present loose | 12 | 11 |
| — in 12403..12814 / above 12814 | 29 / 11 | 1 / 11 |
| **anchor ids only in `patch.big`** | **33** (`5176;7255;8375;9329;10459;12633-12648;12650-12651;12653-12658;12660-12662;12666` — 28 among the 350) | 1 (`11692`) |
| anchor ids in **neither** | 916 (322 among the 350) | 947 (all 350) |

What this establishes: correspondence *in this specimen* between an anchor id and a file name for
most rows, and its absence for 949 — so the binding `lOnr = N ↔ p{N}.big` is **not** an
inspected-corpus invariant here, which is why `[identity]` on `nhllegacy-ps3` stays a candidate
(now with its file half measured) rather than becoming the first established identity. It does not
prove the runtime looks ids up that way, and it says nothing about whether this copy — recorded by
R1 as not established pristine, and carrying a third-party sidecar file at USRDIR root — is complete.

### A.3 D3 — PS3 `patch.big`

`H:…/BLUS31540/USRDIR/patch.big`: SHA-256 `d2d26273a742cc21667b56e483b089ae15c23b462c335bea2dcf38405dc9bbe0`,
60 596 976 bytes, `EB\0\3` version `0x00000400`, 16-byte entries, 459 members / 459 distinct paths,
28 dirs, last payload end = archive size. **The committed reader inspects it without modification.**
Record: `c0-as7-ps3-patch-members.{csv,json}`, with a loose-tree comparison at the same relative
paths; summary in `c0-as7-ps3-patch-overlap.csv`.

| Family (top of path) | Members | Stored form → committed `decode_payload` | Loose file at same path | Content |
|---|---|---|---|---|
| `fe/loc` | 16 | 8 `DB\0\x08` TDB databases (`nhl_{cze,eng_us,fin,fre,ger,lng,rus,swe}.db`) + 8 `-meta.xml`, raw | 16 | 9 equal (the 8 `-meta.xml` and `nhl_lng_lg.db`), 7 `.db` differ |
| `fe/ion/artassets/playerheads` | 58 | raw `BIGF` | 12 | **all 12 differ** |
| `fe/ion/artassets/playerheadssmall` | 25 | raw `BIGF` | 11 | **all 11 differ** |
| `fe/ion/artassets/jerseys` | 15 (`jersey_{s}_{a}_{v}.big`) | raw `BIGF` | 4 | differ |
| `fe/ion/artassets/createplayer` | 13 (`blocker/gloves/pads/trapper`) | raw `BIGF` | 0 | — |
| `fe/ion/game/screens/…` | 2 | raw | 2 | differ |
| `fe/credits` | 1 | `chunkzip` inflated | 1 | differs |
| `rendering/{blocker,glove,goaliehelmet,goaliepad,goaliestick,helmet,jersey,logo,pant,player,sock,trapper}` | 328 `.rpsgl` | **72 `chunkzip` inflate, 257 fail** | 63 | **0 equal** (neither stored nor inflated bytes): 18 inflated members differ from their loose file (14 of the 63 loose files share a decoded member's byte length, so the loose `.rpsgl` are stored unwrapped), 45 undecoded |
| `sku/na.ini` | 1 | raw (one byte) | 0 | — |

**Codec finding, recorded not fixed.** Every failing member carries the padded header
(`dummy = 2`) with `n_chunks ≥ 2`; every single-chunk member inflates; the codec's error is
`chunkzip header is truncated: needed N bytes, have M` from the padded-chunk walker (the record
keeps the header words and the exact message per member). This is a limitation of the committed
`decompress_chunkzip` on this variant's multi-chunk payloads, not a statement about the bytes; it
belongs to C3 and is not repaired here. `chunklzx` stays unsupported by the public path.

**Overlap, stated without precedence.** `patch.big` holds 83 portrait members at exactly the loose
tree's relative paths; 23 of those paths also exist loose and **in every one the loose bytes
differ from the member's** (no `.bak` sibling exists for any). 60 portrait members have no loose
counterpart, including 28 of the 350 outside-range anchor ids and eleven ids above the anchor's
maximum. Nothing here says which copy the game loads: `[runtime_lookup]` keeps `state = unknown`
with no value, and `[discovery]` stays a candidate — both notes now say what was found.
**For the stick slot** (the one family with an in-game observation) `patch.big` holds **no**
`rendering/playerstick` or `createplayer/sticks` member on this copy.

---

## B. Registry changes — what moved and what did not

**Promotions to `established`: none.** Justification per block that gained evidence:

| Block | Why it stays a candidate / unknown |
|---|---|
| portraits `nhllegacy-ps3` `[identity]` | the join is not total (949 of 5 449 anchor ids have no loose file; 916 in neither loose nor patch); the runtime's lookup is unobserved. Candidate note rewritten with the join; provenance cites the census, join, patch record and case |
| portraits `nhllegacy-ps3` `[discovery]` / `[runtime_lookup]` | `patch.big` is now read and holds a second source for 83 ids with differing content at 23 colliding paths; which the game loads is untraced. `runtime_lookup` gains provenance and a note; no value |
| portraits `nhllegacy-xbox360` `[discovery]` / `[runtime_lookup]` | measured absence in the two render archives; members not located |
| logos / jerseys / equipment `nhllegacy-xbox360` `[discovery]` candidate | upgraded from `asserted` / `specimen-observation` (a summary line) to **`measured` / `inspected-corpus-invariant`** (the whole member table). Still a candidate: `DiscoveryValue` is one root per target, and each family's declared path families sit in two archives (jerseys in `nocacherender.big`, socks in `cacherender.big`) and one unobserved place (every `fe/` family). A root of one archive would misdescribe the other family |
| logos `[identity]` | file half observed for 180 / 218 under the declared spelling — not total; `ambiguity` is a policy; crest half still prose |
| jerseys `[identity]` candidate | file half observed for all 698 (bundle, nameplate, sock); still bundles `ambiguity`, 207 members carry ids no row stores, D7 undecided |
| equipment `[identity]` | all eight measured fields' file halves observed; the ninth (goalie-stick vs the shared-`wXFx` claim) contradicted; one `[identity]` per target still cannot carry eight sources (the recorded schema gap, not extended) |

**Carried as the contract already allows** — `[[path_family.observed_bucket]]` entries
(`scope = inspected-corpus-invariant`, `corpus = xbox360-shipping-archives`, census = the member
record by digest) on every Xbox 360 render family that now has a located directory: logos
`rendering/logo`; jerseys `rendering/jersey` and `rendering/sock`; equipment `rendering/playerstick`,
`skate`, `helmet`, `glove`, `goaliestick`, `goaliepad`, `goaliehelmet` (the blocker/trapper
directories have no path family to attach to — the C0-AS6 P2 gap). On the PS3 portrait families
the eight observed-bucket entries now cite the **committed per-id census** (`c0-as7-ps3-portrait-census.csv`,
same buckets, same ranges) instead of the uncommitted C0-POR1 range summary.

Profile versions: portraits 5 → 6, logos 2 → 3, jerseys 3 → 4, equipment 1 → 2 — each with a
history line naming this package. Recompiled stays by reference; nothing specimen-level was written
on `legacyrecomp-recomp`. The inventory CSV's six affected rows gain the records in
`evidence_source` and the measured facts in `blocker`; four rows' `committed_or_working_tree`
cell still said "working tree (C0-AS6, uncommitted)" after `9e02acf` and now say committed.

**Readiness: unchanged.** All 6 operations on every declared target of all four profiles remain
`BlockedByUnknownConstraints`; `target_required_constraints` stays 0; nothing became operative.
The AS6 tripwire `the_logo_and_jersey_profiles_represent_their_families_and_establish_nothing`
pinned the discovery candidate at `specimen-observation` ("a summary line is not a corpus
invariant") and was **deliberately** updated to pin `inspected-corpus-invariant` / `measured` with
a C0-AS7 record cited — the only existing case changed.

---

## C. Implementation

| Path | Role |
|---|---|
| `crates/tdb-archive-big/examples/as7_eb_members.rs` (A) | D1/D3 tool: reads an EB member table through the committed reader; per entry the literal path, offset, size, the two uninterpreted words, stored magic and SHA-256, chunk-header words, what `decode_payload` yields (form, size, SHA-256, or the codec's error text), and optionally the loose file (+`.bak`) at the same path through `loose::read_loose` — a non-`NotFound` read failure aborts the run with the path before any record is written, and `--loose-root` is validated first. Self-contained SHA-256 with a FIPS known-answer self-check at start-up (the crate has no digest dependency and the manifest is untouched); every archive digest additionally cross-checked against `sha256sum` |
| `crates/tdb-archive-big/src/loose.rs` (A), `src/lib.rs` (M, `pub mod loose;`) | P2: loose-file lookup with `validate_loose_root` (exists, is a directory, listable — checked before any member) and `read_loose_with` / `read_loose` (`NotFound` is the **only** absent; every other error carries the path); the reader is injectable and 5 unit cases cover a readable file, a missing file, injected `PermissionDenied` / `Other` / `InvalidData`, a real below-a-file path, an invalid root (absent, and a file), and the error text |
| `crates/tdb-canonical/src/asset_census.rs` (A) | pure record and join logic: portrait name classification, `BIGF`/RefPack peek, census CSV render/parse (parser refuses a row whose label disagrees with its name), member-record parser (refuses disorder), name shapes with a one-component wildcard, grammars, per-folder portrait join, per-grammar member join, renderers |
| `crates/tdb-canonical/examples/as7_portrait_census.rs` (A) | D4 tool: walks the two folders sorted, hashes, peeks, writes the census + metadata |
| `crates/tdb-canonical/src/asset_census_tests.rs` (A) | 13 cases: **9 synthetic** (ranges, name classification, a synthetic six-member `BIGF` with 3- and 4-byte RefPack headers, census round trip + refusal, the portrait join over a built case incl. shared/zero/backup/patch/multi-bucket, member-record round trip incl. the quoted error cell + disorder refusal, shapes/wildcards, grammar matching + join); **4 specimen** (records match their own metadata — schema, `source_commit`, CSV digest, row count; D4 join re-derived and compared byte-for-byte; D1 join re-derived and compared; shape histograms and the D3 overlap re-derived and compared). A missing record **fails** and prints the regenerated text; nothing skips |
| `crates/tdb-canonical/src/lib.rs`, `asset_tests.rs` (M) | module registration; the one deliberate tripwire update |

---

## D. The pre-existing C0-POR1 dependency

Uncommitted and pre-existing: `docs/receipts/data/c0-por1-{portrait-specimens,census,specimen-hashes}.json`
and `c0-por1-tools/`, plus the POR1/R1 receipts. This package **does not read** any of them (the
D4 census is a fresh per-id measurement of the same copy, and its counts agree with R1's). Where the
portrait profile cited the POR1 range summary for the PS3 observed buckets, it now cites the
committed AS7 census. Citations that remain to POR1 material — the `identity` and `container`
provenance, the discovery candidate's provenance, the AS6 test comment — are unchanged and stay an
**uncommitted dependency of the profile**, reported here and not resolved: committing them is
Codex's call (C0-AS6 unresolved question 2).

---

## E. Verification

**Candidate:** `git -c core.autocrlf=false -c core.eol=lf archive 9e02acf` (1 231 files) + the 25
boundary files below, CRLF-normalised to LF → 1 248 files; fresh `CARGO_TARGET_DIR` (`target3`); `QA_WRITE`,
`QA_EVIDENCE_ROOT`, `NHL_STUDIO_FIXTURES` unset; `9e02acf`'s `Cargo.toml` / `Cargo.lock`, `--locked
--offline`. Scratch: `…/scratchpad/as7/{candidate,ref,target2,logs,boundary.txt,compare.py}`.

| Check | Result | Log |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **552 passed · 0 failed · 0 ignored** (539 at `9e02acf` + 13) | `logs/r1-test-canonical.txt` |
| `cargo test -p tdb-archive-big --locked --offline` (builds the new example) | **28 passed · 0 failed · 1 ignored** (23 at `9e02acf` + the 5 `loose::tests` error-path cases; the ignored one is the baseline's own `#[ignore = "LZX bitstream framing not yet pinned"]`) | `logs/r1-test-archive-big.txt` |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0 | `logs/r1-clippy-canonical.txt` |
| `cargo clippy -p tdb-archive-big --all-targets --no-deps … -D warnings` | exit 0 (the new module and example are lint-clean) | `logs/r1-clippy-archive-big-nodeps.txt` |
| same **with** deps | fails on **13 lints in `tdb-archive-codec`** (`dds.rs`, `lib.rs`, `lzx.rs`, `refpack.rs` — `manual_div_ceil`, `manual_swap`, `manual_range_contains` …) — the recorded C0.2 baseline defect, untouched; **known exception, preserved** | `logs/r1-clippy-archive-big-full.txt` |
| `cargo fmt -p tdb-canonical -- --check` | exit 0 | `logs/r1-fmt-canonical.txt` |
| `cargo fmt -p tdb-archive-big -- --check` | fails on **baseline files only** (`src/eb.rs`, `examples/{big_ls,eb_extract,eb_layout}.rs` at `9e02acf`) — **known exception, preserved**; `rustfmt --check` on the new `src/loose.rs` and the example: exit 0 | `logs/r1-fmt-archive-big.txt` |
| non-boundary files vs the `9e02acf` archive | 1 223 compared: **0 changed · 0 missing · 0 extra**; all 8 modified boundary files differ from the archive, all 17 new ones absent from it | `logs/compare-r1.txt` |
| measurement determinism | the three archive records byte-identical across three independent runs (the third with the P2-corrected tool), the census across two | `out/`, `out2/`, `out3/` |
| archive digests | tool = `sha256sum` on all three archives | §A |

**Boundary (SHA-256 of the LF-normalised bytes git would store; M = modified vs `9e02acf`, A = new):**

```
98d7f0fafec18ab47be30fae1f6a622137a3d0ca0eae5cd03f805824858bb20a  M  crates/tdb-canonical/src/lib.rs                       +3
738af02cd115353be8b17901187391908c81f98905761306c017602c0e7ebb12  M  crates/tdb-canonical/src/asset_tests.rs               the one tripwire
65d8fe4f80fc98e13f0627b14e9641d4ae7a86d3e8a8377872875801a6146eb5  A  crates/tdb-canonical/src/asset_census.rs              (= module_sha256 in the D4 record)
ef62f7f4c0b2752474f643433292c806da97b4e17d14a5008b3195fbe50f5732  A  crates/tdb-canonical/src/asset_census_tests.rs        13 cases
914f083c99a3743e4d05347a556f428cf192a4f0eaf5000be17751e3a14d46f1  A  crates/tdb-canonical/examples/as7_portrait_census.rs  (= example_sha256)
6705fa23ee7c89c1839fa310e1c8ae8157df005505062632ac5ffa16ba8065db  M  crates/tdb-archive-big/src/lib.rs                     +1 (`pub mod loose;`)
2d6e313876f16926e5427f98d954756aceefd9d86bd2def7211472ddcf92c34b  A  crates/tdb-archive-big/src/loose.rs                   P2 module + 5 cases
4d8578eec2349ed223fbb98cf8c2f84ccf7c42bc3a663aa1c855d641fed71103  A  crates/tdb-archive-big/examples/as7_eb_members.rs     (= tool_sha256 in the D1/D3 records)
4870f9718125b1471597d40b909e6ee1f8e22f5c592f45a9a00b51db8c598537  M  defs/adapters/asset-profiles/assets-portraits.toml    profile_version 6
a86eb98ffb17deb744ccd7efdc85bc6073aba601e44694703f20879093222f4b  M  defs/adapters/asset-profiles/assets-logos.toml        profile_version 3
e15c8fb0e76ed76587b43a3a5ce29d944010dac74cf9bbd1ab4ba21abfbd9024  M  defs/adapters/asset-profiles/assets-jerseys.toml      profile_version 4
025196d3d6f9eb155d2fda962f54aeab713141b9108f5c727832508309dc3083  M  defs/adapters/asset-profiles/assets-equipment.toml    profile_version 2
c4404e38db5c1bb39a9ed9882a3de73ecbb7b06219a79952eb2670279c3754d1  M  docs/receipts/data/c0-reg2-asset-mapping-inventory.csv  8 cells
46cb5f1c1dde59d3c1b2ef20552987dd562564ddd60f5c78e9c13b92da3b2732  A  docs/receipts/data/c0-as7-ps3-portrait-census.csv     18 663 rows (2.5 MB)
b60fc027814f4e21f57d612b742357e7a2221e8ac5f1a275c93c2d89d2ba8d4a  A  docs/receipts/data/c0-as7-ps3-portrait-census.json
f281a5916a442fbed95e251626235204df8b6bb29c3598030fe7a0165b268a65  A  docs/receipts/data/c0-as7-x360-cacherender-members.csv  12 845 rows (2.3 MB)
f221e312bfabc9de83a033be3334baa02fc02a38ffb5a954a939125578b65199  A  docs/receipts/data/c0-as7-x360-cacherender-members.json
ffded0db30cf2539b50f434cb79c567cd4be08fbdc2d254550a80370aeb883bd  A  docs/receipts/data/c0-as7-x360-nocacherender-members.csv  3 237 rows
55fcbc084024fec63c251b4e9c26d37ece3f3fa9f7bb8f00016e6f8471cbdfc5  A  docs/receipts/data/c0-as7-x360-nocacherender-members.json
13dc4d4377ad9f22c11344fc0f062d39c530876a4f73f24a11e12ee45f7bf300  A  docs/receipts/data/c0-as7-ps3-patch-members.csv       459 rows
09a33d2023a0cab2de792289f69be662c6ed633060ff3885052ea5c0b4a931ef  A  docs/receipts/data/c0-as7-ps3-patch-members.json
85189df0d0bff1dc3df8aa038df7aed9efd1ae460248b9400c4432c1bc74d542  A  docs/receipts/data/c0-as7-ps3-portrait-join.csv       re-derived by a case
01b9ffb7c8173cff35658e0fd72a5be6b779dac0920c333e45f12a26baf6570b  A  docs/receipts/data/c0-as7-x360-member-join.csv        re-derived by a case
d278d94d026f106d675f3f0ceba6942ac58f3d9467e59e86391f7e47469e85ca  A  docs/receipts/data/c0-as7-member-shapes.csv           re-derived by a case
92b90bf98ad8453051742ee1bcfaa7c9435bc66576817b8a344221aa08f162af  A  docs/receipts/data/c0-as7-ps3-patch-overlap.csv       re-derived by a case
```

Outside the boundary (documentation, read by no test): this receipt;
`docs/STATE-OF-THE-REPO.md`; `docs/plans/project-completion-roadmap.md`;
`docs/validation/c0-06-canonical-project-and-adapters.md` (§12) — all four already untracked or
modified by earlier packages. `Cargo.toml` / `Cargo.lock` untouched.

**Size.** The twelve records total 6.4 MB, 4.8 MB of it the two per-row tables. Codex accepted
the size at review (section G): all twelve records and the per-member hashes are retained.

Proposed commit, when authorized: one commit over the 25 paths,
`feat(canonical): measure the file side of Legacy asset identities — render-archive member tables, PS3 portrait census, patch.big` (expected 552 canonical + 28 archive cases).

---

## F. Limits, unresolved questions, next step

**Limits.** Two copies, both previously identified; nothing acquired; no source byte written (every
open read-only; the D1/D3 tool decodes in memory and drops); the eleven Xbox 360 archives outside
the batch not opened; no member of either render archive decoded (`chunklzx`); no head, arena or
audio work; the separate headmesh repository not inspected; no QA generator run, `docs/audits/data/**`
untouched; no schema version; no manifest/lock change; per-slot identity, D7, `SLOT_SPECS.max_id`
not touched; nothing committed or pushed; index empty. Build identification of both copies remains
incomplete (a title id and version word is not a build), and the PS3 copy is not established pristine.

**Unresolved questions for the architect.**
1. **Per-family discovery.** Three families' declared path families now have *measured* locations in
   two different archives and one unobserved place; `DiscoveryValue` is one root per target, so
   the block cannot be established without misdescribing a family. Options: a discovery per path
   family, or a `[[discovery.root]]` list; either is a schema change, not made here.
2. **Goalie-stick binding.** HEAD `SLOT_SPECS` keys the goalie stick to `xmja.wXFx`; the listing does
   not support it (1 of 19). A C1/C5 source finding; which field (if any) keys `goaliestick_{id}` is
   unmeasured.
3. **Portrait identity is not total.** 949 anchor ids without a loose file on the only attributable
   portrait corpus. Whether that is the copy (incomplete), the game (silhouette fallback), or the
   anchor (ids the roster never displays) is unknown; a load test or trace decides it, a census
   cannot.
4. **`patch.big` precedence** is now a live two-source question with content that differs at 23
   paths; only runtime evidence settles it.
5. **Committed `chunkzip` codec** fails on multi-chunk padded payloads of this variant (257 / 329) —
   a C3 defect to schedule, recorded with the header words per member.
6. **POR1 records** stay an uncommitted dependency of the portrait profile (§D).

**Smallest next step.** List `cache.big` and `nocache.big` with the same tool (an hour; the tool,
the reader and the join cases exist): it settles where the portrait, crest and thumbnail members are
inside the Xbox 360 title, closes FR14 for every family at once, and lets the per-family discovery
question (1) be decided on complete evidence. Real destination output still needs C3a's write
planner and the LZX decode, which no listing substitutes for.

---

## G. Review findings P2 / P3 — disposition (2026-09-16, for re-review)

**P2 — read failures became absence evidence.** *Was:* `loose_file` in the example mapped every
`std::fs::read` error to `loose_present = false`. *Fix:* a new `tdb_archive_big::loose` module —
`validate_loose_root` (must exist, be a directory, be listable; run before any member is looked
up) and `read_loose_with` / `read_loose` (`io::ErrorKind::NotFound` is the **only** absent; any
other error is `LooseError::Read { path, kind, source }`). The example validates `--loose-root`
first and **fails the run** with the path on any non-`NotFound` failure, before a record is
written; its metadata limitation now states that meaning of `loose_present = false`. Five
deterministic unit cases (scratch temp dir + injected reader, no private installation): readable
file present with bytes; missing file absent, and only `NotFound` absent; injected
`PermissionDenied` / `Other` / `InvalidData` and a real below-a-file path are errors carrying the
path; an invalid root (absent, or a file) is refused; the error text names the path. Verified live:
`--loose-root <nonexistent>` aborts before creating the output directory.

*Re-measurement.* The three archives were re-recorded read-only with the corrected tool from the
isolated candidate against the same copies (`out3/`): **all three CSVs byte-identical** to the
earlier runs — every `loose_present = false` in the `patch.big` record was a genuine `NotFound`,
so no measured fact changed and no absence assertion in the join records or tests was weakened.
The three `.json` metadata files were regenerated (tool hash
`d4b8558a…5e311f` → `4d8578ee…71103`, limitation text); the old results were not retained under
the new hash.

**P3 — summaries contradicted the records.** *Corrected everywhere:* (a) all **eight** measured
equipment fields have matching members; the goalie stick is a **ninth** comparison against HEAD
`SLOT_SPECS`' claim that it shares `xmja.wXFx`, and it is that shared-field claim the listing
contradicts — not one of the eight fields failing (equipment profile note and identity note,
inventory row 25, receipt §Result / §A.1 / §B, STATE, roadmap, c0-06 §12). (b) The broad logo
grammar `texlib_{id}_{a}_{b}` has 348 distinct first components, 218 matched, **130** extras; the
**127** figure belongs to the restricted `_0_0` grammar (307 members / 307 distinct) — logo profile
path-family note, inventory row 8, receipt §A.1 table and prose. (c) The `xbox360-shipping-archives`
corpus limitation in `assets-logos.toml`, `assets-jerseys.toml` and `assets-equipment.toml` no
longer says no member name, offset or digest was recorded: it now distinguishes the **two measured
render archives** (member tables committed, by record and digest) from the **eleven uninspected
archives** (headers and a summary line only; every `fe/` family is in one of them). No unrelated
history was rewritten; the C0-AS6 §D text and earlier receipts are untouched.

**Record size:** retained in full per Codex (no slimming).

**Validation, repeated on a fresh candidate** (`git archive 9e02acf` + the 25-path boundary,
fresh `target3`, QA variables unset, baseline manifest/lock): canonical **552 / 0 / 0**; archive
**28 / 0 / 1 ignored** (23 + 5 new error-path cases; the ignored case is the baseline's labelled
LZX one); clippy `-D warnings` clean on `tdb-canonical` and on `tdb-archive-big --no-deps`; the
**known exception** of 13 baseline `tdb-archive-codec` lints under `-D warnings` with deps, and the
**known exception** of four baseline `tdb-archive-big` files failing `cargo fmt --check`, both
preserved and unchanged; the new module and example are rustfmt-clean; non-boundary files 1 223
compared, 0 changed / missing / extra. Nothing staged, committed or pushed; index empty; no source
copy written; no block promoted; the `chunkzip` defect not touched; no further archive listed.

**Stop for Codex re-review. Nothing staged, committed or pushed.** *(superseded by section H)*

---

## H. C0-AS7-CLOSE — review closure and the commit

Codex review passed: P2 (read failures as absence) and P3 (summaries vs records) closed in place
(§G), no remaining findings; all 25 §E hashes independently verified; the isolated validation of
§E / §G (canonical 552 / 0 / 0; archive 28 / 0 / 1 baseline ignore; scoped clippy and fmt clean
with the documented baseline exceptions — 13 `tdb-archive-codec` lints under `-D warnings` with
deps, four baseline `tdb-archive-big` files under `cargo fmt --check` — preserved) is the
validation relied on; no rerun, since the committed bytes are the validated bytes.

| Item | Value |
|---|---|
| Pre-checks | HEAD `9e02acf`; index empty; all 25 §E LF-normalised SHA-256 hashes match the working tree — **no drift** |
| Staged | exactly the 25 boundary paths (8 M, 17 A) by explicit path; every staged blob's SHA-256 equals its §E hash; `git diff --cached --check` clean; +39 465 / −113 |
| Staged tree | `ed31cf164fd1d68a9c52f4909c52cc17f3fea803` (`git write-tree` before the commit = `HEAD^{tree}` after it) |
| Commit | **`8d60a6d582c1bf208e0a5bc590b28d5add7d5051`** — `feat(canonical): measure Legacy asset members and portrait identity joins`; parent `9e02acf`; message file; Co-Authored-By line |
| Post-checks | committed path set = boundary; all 25 committed blobs match §E; index empty afterwards |
| Preservation | `Cargo.toml` / `Cargo.lock`, the POR1 records and every other working-tree change untouched; all twelve evidence records with per-member hashes committed; nothing pushed |

**Remaining blockers, unchanged:** per-family discovery is a schema gap (one root per target vs
families split across two archives and an unobserved place); the goalie-stick field is unknown
(HEAD's shared-`wXFx` claim contradicted); the portrait identity is not total on the PS3 copy (949
anchor ids without a loose file) and its runtime lookup unobserved; `patch.big` precedence needs
runtime evidence; the committed `chunkzip` codec fails on multi-chunk padded payloads (C3); the
POR1 records stay an uncommitted dependency of the portrait profile; portraits, crests and
thumbnails are in one of the eleven unlisted Xbox 360 archives. No block is established beyond the
seven already on `nhllegacy-ps3`; nothing is game-verified.


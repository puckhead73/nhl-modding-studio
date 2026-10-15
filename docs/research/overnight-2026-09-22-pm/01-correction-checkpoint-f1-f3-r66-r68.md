# Handoff R69 — Correction checkpoint F1–F3 (review of R66–R68): the X360 per-level contract H1 is restated as a candidate with partial static support, the EA-MP3 preview contract is bounded to what 24 vectors show, the PS4 venue counts are made exact, and the R60 extent data is marked invalid for its intended slots

Research lane, 22 September 2026 pm (extended continuation), first package of this campaign. Claim
`research-r69-corrections-f1-f3-r66-r68`. Applies [the coordinator's delivery review of 2026-09-22 (R66–R68)](../../validation/research-20260922-delivery-review.md)
to the morning campaign ([overnight-2026-09-22](../overnight-2026-09-22/00-ATLAS.md)) and to the pm-campaign handoff 17 (R60).
**No raw evidence is changed**: every JSON, probe and measurement stays as published; the corrections are dated notes, restated conclusions,
exact counts and rewritten developer-facing text. No application file, matrix or developer receipt is touched. The developer-facing
claim is published with this handoff; no acknowledgement is awaited.

## Plain-language summary

Three of this morning's results said more than their tests support, and one older data set needs a plainer status.

1. **Xbox 360 mip positions (R66).** The search that located each smaller mip copy decided "found, decisively" by comparing the best
   match only against the crowd of other candidates (the 5th percentile and the median). It never checked that the best position was
   *unique* or that the second-best position was clearly worse, it rounded the scores it kept, and it counted only the disagreements
   against the rule by hand — three were then explained away as flat textures. One of those three (`clipboard_0_0_am`) does not fit that
   explanation as given: its candidate crowd scored 63.75 at the 5th percentile and 255 at the median, so the "near-zero crowd" reading
   is wrong; its best score of 0.000 comes from a *constant parent target*, which any constant candidate matches exactly. The rule H1
   therefore stands as a **candidate with substantial partial static support**, not as a write contract for a whole mip chain, and the
   published agreement totals are restated as "under the R66 predicate". A stricter classifier with independent controls is this
   campaign's next package (R70), and it reclassifies agreements and disagreements alike.
2. **PS4 speech preview (R67).** The block grammar, the end-to-end decode and the sample arithmetic stand on 24 vectors. But the two
   decoders used were ffmpeg's fixed-point and floating-point MP3 decoders — a useful consistency check inside one code base, not two
   independent implementations — and the role of the 1 152 embedded PCM samples is open, so a preview built on this contract is a
   **diagnostic preview whose first 24 ms are unresolved**; it must say so, and must not silently pick one splice and call the result
   the game's audio. The hashes in the JSON belong to ffmpeg 8.1.1 with the recorded options, not to every conforming decoder.
3. **PS4 venue folders (R68).** The summary and queue said "one level per title ships no folder". Exactly: NHL 15 (`carolinageneric`)
   and NHL 18–21 (`team_canada_training`) — **NHL 16 and NHL 17 have none**. While making the counts exact, a residual the handoff did
   not state was found: level *ids* are not unique to a level name on NHL 16–21 (ID 0 is shared by `anaheim` and `common_ice_int` on
   18–21, by 27 names on NHL 17 and by five on NHL 16; NHL 16 also shares ID 18 and ID 28 between two names each), so the two-hop chain
   from an arena row can land on more than one name. The name match is a **listing with provenance**; it does not show that the game
   selects a venue by that name, and re-pointing an arena's art id needs a separate consumer / cascade contract before any write UI.
4. **R60 extent data.** Already withdrawn this morning; now carries an explicit **invalid / superseded** status: its windows were read
   4 KiB early, so they are not measurements of the intended slots at all, and even correctly placed they were lower bounds. The raw JSON
   stays as the record of what ran. No allocation or layout statement may cite it.

## F1 — R66 classification and writer acceptance (handoff 02, queue row 3, claim `research-r66-rw4xb2-level-addresses`)

**What R66 measured, restated exactly.** For 142 DXT1 / DXT3 / DXT5 descriptors (plus 36 DXN descriptors that were walked but have no
colour endpoints) on NHL 09 / 10 / 13 / Legacy, the probe scored every candidate offset of each level ≥ 1 by the mean absolute
difference between the level's block-mean endpoint colours (gathered through the Xenos tiled address) and the 2 × 2 box reduction of the
parent level as found. It kept the best offset, its score rounded to three decimals, the median and 5th-percentile candidate scores
(rounded), the runner-up *offset* without its refined score, and the score one 4 KiB page later.

**Why the published classification is not sufficient.**

| Defect | Where | Consequence |
|---|---|---|
| "Decisive" = `score < 0.5 · p05 ∧ score < 0.35 · median`. No unique-optimum test; no runner-up margin; the runner-up's refined score is not recorded | `r66_contract_check.py::decisive`, handoff 02 §1 step 4 | flat or repeated content (every constant candidate scores 0 against a constant target; a periodic texture repeats its optimum) can be labelled decisive |
| Scores rounded to 3 decimals before classification | `r66_rw4xb2_level_addresses.py` (`round(…, 3)`) | the classification boundary is not reproducible from the JSON |
| Three disagreements excluded *after* the fact as "flat textures" | handoff 02 §2 evidence table footnote | the exclusion rule was applied to mismatches only, never to agreements |
| `clipboard_0_0_am` (64², DXT1, level 1): score 0.000, p05 **63.75**, median **255.0** | `r66-contract-check.json` (nhl09 disagree) | the footnote's "near-zero p05" explanation does not describe this record; the 0.000 comes from a constant *target* (parent box-reduction), which the constant candidate at offset 0 matches exactly, while non-constant candidates score high — the level is *uninformative*, not "found at 0" and not "found at 8 192" |
| DXN levels were walked and their disagreements landed in the contract check's grand totals (`disagree packed 69` includes DXN), while the handoff's table excluded them by hand | `r66-contract-check.json` `grand` vs handoff 02 §2 table | the general JSON and the DXT totals disagree; the "DXN excluded" statement in §1 step 5 was not implemented in the check |

**Withdrawn as stated** (dated 2026-09-22 pm; the original text stays in handoff 02 under a correction note):

| Withdrawn statement | Replacement |
|---|---|
| "H1 … agrees on **386 / 387** own-tile and **76 / 78** packed decisive levels" as a classification result | "… under the R66 predicate (best < 0.5 · p05 and < 0.35 · median), which does not test uniqueness, margin or content informativeness"; the exact totals are regenerated by R70 with one rule for agreements and disagreements |
| "H1 is the per-level address contract the review asked for, down to 4 × 4-block levels" (§4) | H1 is a **candidate with substantial partial static support** on levels ≥ 4 × 4 blocks. The last levels, 16-wide strips, DXN / CTX1 / `8_8_8_8` and cubes are unresolved. It is not a full-chain write contract |
| "Acceptance test: run the r66 search on the *written* file and require every level ≥ 4 × 4 blocks to be found at H1's offset with a decisive score" (§4) | Necessary, not sufficient: a file generated to layout H1 and scored against H1 cannot alone establish compatibility with the *shipped* layout. Required in addition: independent controls — known misplacement (the H1 offset ± one tile / ± one page must score clearly worse), tied / flat content (the classifier must return *uninformative*, never *found*), wrong byte order (little-endian endpoint reads must not beat big-endian), and independently decoded per-level content (full block decode compared with the decoded parent, not only endpoint means) where a decoder exists |
| "Endpoint mean … compared with the 2 × 2 box reduction of the parent" as the texture test | Endpoint means and box reduction are **approximate** evidence of level content; they are not an exact reconstruction and cannot show that a level's texels are right, only that its coarse colour field is in the expected place |
| "Refuse (or leave the donor tail untouched and say so) for the unestablished cases" (§4) | **Do not offer donor-tail preservation as a general safe resize fallback.** Changed dimensions, offsets, mip counts or updated image content can invalidate a preserved tail. Require a demonstrated identity-preserving case (same dimensions, same mip count, same offsets, tail bytes unchanged and re-located by the search) or refuse the unsupported mutation. Independently established operations (single-level replacement in place, base-level `DW12` rule of the pm handoff 02 §1) stay usable |
| "The old writer comment in `texlib.rs` ('players rendered debug-blue at gameplay distance') is consistent with a tail placed elsewhere than H1" | The comment is **historical context**, not a verified runtime result of this campaign and not proof of the cause |

**Preserved, with accurate labels:** the per-descriptor found offsets and their scores (`r66-rw4xb2-level-addresses.json`), the
untiling validation of the base level, the `DW12` = level-1 tile base observation (141 / 142), the packed-position table (bytes past the
tail base), the r60 read-offset defect note. All are R66 *observations under its predicate*; R70 re-derives the classification from
the same descriptors with unrounded decision values.

**Developer text (queue row 3 becomes rev. 1a).** H1 is a candidate placement rule with partial static support; do not enable full
mip-chain writes from it; do not implement donor-tail preservation as a general fallback; independently established operations remain;
audit `replace_with_mip_chain` against H1 as a *hypothesis check*, not against a contract; wait for R70's controls and totals before
treating any level class as established.

## F2 — EA-MP3 preview contract bounded (handoff 03, queue row 1, claim `research-r67-ea-mp3-sized-frames`, card A-MP3-1)

**Kept (24 / 24 vectors, NHL 15 / 16 / 17):** the sized-frame grammar (`u16le size │ 0x8000`, optional `u32 pcm_samples` + PCM on the
first frame), exact tiling of every block, nine frames per full block, per-frame bitrate, `main_data_begin` ≠ 0 across block boundaries,
concatenated MPEG frames decoding to frames × 1 152 samples at 48 000 Hz mono, `num_samples` = frames × 1 152 − last-block trim.

**Restated:**

| Statement in handoff 03 | Bounded form |
|---|---|
| "two independent decoders" / "two separate MP3 decoders" | ffmpeg 8.1.1's `mp3` (fixed-point) and `mp3float` decoders: a **fixed / float consistency check** inside one code base. Their lineage / independence was not established and is not claimed. They do not settle EA's embedded-PCM semantics |
| "Preview contract established up to one open item" | **Diagnostic preview contract**: the MPEG-only PCM is verified; the *audible start* is unresolved. A preview may be presented only as "incomplete at the start (first 24 ms unresolved)"; candidate A (embedded PCM replaces decoded frame 0) must not be selected silently and the result must not be described as verified game output |
| "Both decoders' PCM SHA-256 per vector … can be checked against them bit-for-bit (fixed) or within ± 1 (float)" | The hashes apply to **ffmpeg 8.1.1, decoders `mp3` / `mp3float`, `-f wav -c:a pcm_s16le`, no resampling**. A different conforming decoder may differ by rounding; agreement with these hashes is a consistency check for *that* decoder, not a conformance requirement |
| The contract's single validation list | Validation is recorded in **three separate layers**: (i) extracted MPEG PCM — frame count, rate, channels, sample count = frames × 1 152, hash / ± 1 LSB against the JSON; (ii) candidate-A output — a separately hashed rendering, labelled candidate; (iii) tail trim — `num_samples` ≤ frames × 1 152, trim = last-block declared deficit, never applied before (i) is verified |
| Robustness | Not yet verified: malformed / truncated sizes, `pcm_samples` bounds (only 1 152 was seen), flag on a later frame, reservoir continuity across a dropped block, final counts on a truncated stream. **R71** (this campaign) adds these checks and the embedded-PCM investigation before any contract revision is handed over |

**Developer text (queue row 1 becomes rev. 1a).** Prepare a bounded parser and diagnostic decode for codec `0x0b` (NHL 15–17): grammar
+ concatenated decode + count arithmetic as in handoff 03 §1–2; label the output "diagnostic preview — first 24 ms unresolved"; do
not choose candidate A silently; no encoder, no replacement, no full-fidelity claim. The listing text stays "EA-MP3 (codec 0x0b):
diagnostic preview available; replacement not supported".

## F3 — arena mapping scope and counts (handoff 04, queue row 2, claim `research-r68-ps4-venue-dir`, card T-VEN-1)

**Exact counts (from `r68-ps4-venue-dir-by-level-name.json` and the r7 `levels` set), replacing "one level per title ships no folder":**

| Title | levels | distinct `ID` | level names = dir | level names without a dir | dirs without a level name | `ID` shared by > 1 name |
|---|---|---|---|---|---|---|
| NHL 15 | 33 | 33 | 32 | `carolinageneric` | `common` | none |
| NHL 16 | 41 | 35 | 41 | **none** | `common`, `common_boards_international`, `common_props_roof2` | ID 0 (5 names: `illuminating_cjt_detroit`, `illuminating_cjt_colorado`, `common_ice_edmonton`, `anaheim`, `common_ice_int`), ID 18 (`nybarclays`, `nyislanders`), ID 28 (`vancouver`, `custom_vancouver`) |
| NHL 17 | 79 | 53 | 79 | **none** | `common`, `common_boards_international`, `common_ice_edmonton`, `common_props_roof2` | ID 0 (27 names: 25 `lighting_custom_*`, `common_ice_int`, `anaheim`) |
| NHL 18 | 65 | 64 | 64 | `team_canada_training` | `common`, `common_boards_international`, `common_props_roof2`, `unused` | ID 0 (`common_ice_int`, `anaheim`) |
| NHL 19 | 70 | 69 | 69 | `team_canada_training` | + `vancouvertest` | ID 0 (`common_ice_int`, `anaheim`) |
| NHL 20 | 74 | 73 | 73 | `team_canada_training` | + `vancouvertest` | ID 0 (`common_ice_int`, `anaheim`) |
| NHL 21 | 76 | 75 | 75 | `team_canada_training` | + `vancouvertest` | ID 0 (`common_ice_int`, `anaheim`) |

So "every venue directory is named by a level" holds as a *name* statement, but the **id → name hop is not injective** on NHL 16–21;
handoff 04 §2's chain `arenaartid → ge_level.ID → DebugName → dir` reaches a *set* of names for the shared ids. Whether any arena row
uses a shared id, and what the level's other attributes (`ArenaID`, `RinkId`, `CenterLogo`, `CenterLine`, `IceLine`) add, is measured in
**R73** (this campaign), which publishes the bounded listing contract.

**Restated:**

| Statement in handoff 04 | Bounded form |
|---|---|
| "closes Q-R64-1" | Closes the *directory-naming* half of Q-R64-1 (which shipped folder carries a level's name). The *runtime-selection* half — that the game selects the venue by this name / id — is **open** as **Q-R69-1** |
| "so the Arena page can now find its venue folder in two hops with no guessing" | A case-normalised directory-name match supports **navigation and listing with provenance** ("vault level name = folder name, observed on every shipped title"); it does not trace a runtime consumer |
| "Re-pointing an arena at another venue would be a DB `arenaartid` edit validated against the level set" (§3) | Proposes a write; **withdrawn as a proposal**. `arenaartid` reassignment needs a relationship / cascade contract (what else the level record binds: ice texpacks, rink, arena id) and a consumer contract (what the runtime reads) before any write UI. Until then: read-only discovery |
| "the only unnamed directories are the shared `common*` / `unused` / `vancouvertest` folders" | Kept, and they stay **visible** in the listing as shared / unresolved entries; unresolved levels (`carolinageneric`, `team_canada_training`) are listed with "no folder shipped", never with an invented path |

**Developer text (queue row 2 becomes rev. 1a).** Read-only venue discovery on PS4: list the level(s) whose `ID` equals the arena's
`arenaartid` with their `DebugName`s, the folder match per name (exact, case-normalised, with provenance) and the exact residuals
(no folder / shared id / shared folder); no reassignment UI; label the key "vault level name (observed, not a documented selector)".

## R60 — explicit status (pm handoff 17, claim `research-r60-rw4xb2-slot-extent`)

Handoff 17 now carries a top-line status: **INVALID / SUPERSEDED (2026-09-22)** — the extent tables were read 4 KiB before the intended
slots (header-relative offsets) and, even where correctly placed, are lower bounds inside an already-bounded slot. The raw JSON and the
probe are retained as the historical record. No developer allocation or layout assertion may cite them as qualifying evidence; the
successor evidence is R66 as bounded by F1 above and R70.

## Changed paths

Dated notes only, no deletions: `docs/research/overnight-2026-09-22/` handoffs 02, 03, 04 (top correction notes), `DEVELOPER-QUEUE.md`
rows 1 / 2 / 3, `00-ATLAS.md` (header note, rows 02–04, Q-R64-1 / Q-R65-1 / Q-R67-1), `STATUS.md` (closing note), `USER-CARDS.md`
(A-MP3-1 / T-VEN-1 notes); `docs/research/overnight-2026-09-21-pm/17-…md` (status line) and its `DEVELOPER-QUEUE.md` row 17 (status
pointer). New: this campaign's `00-ATLAS.md`, `STATUS.md`, `DEVELOPER-QUEUE.md`, `USER-CARDS.md`, this handoff, claim
`research-r69-corrections-f1-f3-r66-r68`. Unchanged: every `data/*.json`, every probe, every other handoff.

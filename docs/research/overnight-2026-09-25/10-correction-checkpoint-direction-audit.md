# Handoff R97 — correction checkpoint for the 25 September direction audit: the chant-selection overclaims withdrawn against a whole-database census (the "0 arenas reference a club bank" row of R90 was a transcription error — the raw probe recorded arena chant id 61 on every tree, and that arena's goal-horn and period-horn ids are 61 as well); the Speex outlier reworded to "near-silent output under both decoders"; the framing / decode / control denominators restated exactly; the research / production boundary restated; the R88 source count corrected to 27; the `*artid*` destination check scoped to namespaces; the unregistered R96 decode-matrix run (13:36–13:38 MDT) inventoried and summarised

Research lane, 25 September 2026, 13:56 → 14:20 MDT (real clock). Audit: [research-20260925-misrouted-prompt-audit.md](../../validation/research-20260925-misrouted-prompt-audit.md);
the earlier review [research-20260924-campaign-review.md](../../validation/research-20260924-campaign-review.md) was already answered
by R88 ([handoff 06](../overnight-2026-09-24/06-correction-checkpoint-campaign-review.md)) — re-read, its four corrections stand; nothing
there is re-derived here. Claim `research-r97-correction-checkpoint-audit`. Read-only over `anchors/` (18 gen7 main databases) and this
lane's own JSONs. No developer path, application code, `defs/`, binding, anchor, game file or fixture touched; nothing staged, committed
or pushed; no launch, no listening. Every earlier document keeps its text; superseded sentences carry a dated note pointing here.

Machine-readable: [`data/r97-chant-id-field-census.json`](data/r97-chant-id-field-census.json) (probe `r97_chant_id_field_census.py`,
9 s, 18 anchors with SHA-256) and the previously unregistered [`data/r96-decode-matrix.json`](data/r96-decode-matrix.json)
(probe `r96_decode_matrix.py`, 110 s, 106 vectors; see §6).

## Plain-language summary

Five corrections and one inventory. First, the club-chant story was stated too strongly: the earlier sentence "no database value in
55–66 except one Finnish arena" was only ever measured on two fields of two tables; a census of every integer field of every table on
the eighteen older-console databases shows the number 61 sits in that arena's goal-horn, period-horn and chant fields together, that a
few Swedish clubs' announcer ids and a few players' name-call ids also fall in 55–66, and that the earlier table row saying "no arena
references a club bank" contradicted the probe's own output, which had recorded the Finnish arena on every title. Second, the odd
PlayStation 4 announcer sample is described as what was measured — near-silent under both decoders, useless for judging either — not as
a "silent placeholder" whose purpose is known. Third, the counts behind "every stream", "every title" and "15 / 15 controls" are
restated with their real denominators (four PlayStation 3 trees, not the whole 11–Legacy span; 14 detected corruptions plus one that
cannot be detected structurally; 14 + 106 decoded vectors, not every stream). Fourth, the line between research parsers and Studio code
is restated, and a manifest is said to be a hash list, not proof that a writer recovers. Fifth, a source count of 28 was really 27, and
the art-id destination check must not treat every column whose name contains "artid" as one namespace. Finally, a decode-validation
matrix that ran earlier today but was never written up is inventoried and its results summarised — it confirms the counts and the
restart continuity on 106 more vectors, and it sharpens one claim: the 47-sample PCM frame matches the decoder only when the frame is
loud enough to compare.

## 1. Chant selection (audit correction 1) — what was actually inspected, and the census that replaces the overclaim

### 1.1 The actual inspected population behind R90 / R95

| Level | What R90 / R95 inspected | What was claimed | Status |
|---|---|---|---|
| database, arena-keyed | `exhibitionarena.chantaudioid`, every row, 11 anchors, against the loader-listed bank numbers | "no arena's chantaudioid ever equals 55–66" (handoff 08 title / table row / atlas / queue row 13 / receipt) | **wrong as transcribed** — the R90 JSON field `club_banks_referenced_by_any_arena_chantaudioid` is `[61]` on **11 / 11** trees (15 "Hartwall Areena" rows on 10–12 without an `arenaid` link; one row, index 60–62, on 13–Legacy with 14–15 Finnish tenants + Finland). The handoff's "0" row was written from the SHL-tenant sub-question (every SHL club's arena → 45) and mislabelled as the whole-arena result. |
| database, club-keyed | integer fields of the `exhibitionteams` rows whose `fullname` matches one of the 12 club aliases, and of their arena row (13–Legacy), tested for equality with that club's bank number; "keyish" = field names containing `audio` / `chant` / `arenaid` / `horn` | "no team / arena audio field carries those ids" | **true for those rows only** (0 keyish hits; two coincidental hits: `exhibitionteams.arenaid` 61 for Linköping on NHL 15 and 59 for HV71 on Legacy, `goaliemask6_starter`) — never a whole-database statement |
| database, whole | nothing (R95 inspected the loader, the banks, the module table and the control bank, not the database) | "the only database value in 55–66 anywhere is the Finnish arena's chantaudioid 61" (R95 / STATUS / cards / handoff 09) | **withdrawn** — replaced by §1.2 |
| consumer | `aemscrwd.csi` name table (+ 12 KB undecoded body, u16 probe), `controlCenter.abk` node graph, `na_en_loadfile.xml` | "no database field and no bank data selects them", "the key, if any, is code-side", "selected by the compiled module" | **withdrawn as conclusions** — what is established: the module and the control bank *name* the pickers; the loader lists the banks by number; **no selecting consumer has been traced**; a selector can derive an id, combine fields, read another data source or load content indirectly |

### 1.2 R97a census — every integer field of every table, 18 gen7 anchors (`data/r97-chant-id-field-census.json`)

Population: X360 and PS3 NHL 08–Legacy main databases (`nhlng.db`, SHA-256 per anchor in the JSON), 76–136 tables, 1 498–2 917 integer
fields per anchor, 17–20 of them **audio-named** (name contains audio / chant / horn / sound / music / song / anthem / crowd / voice /
speech). Omitted, stated: NHL 07 (gen6 schema, no chant field), PS4 (no gen7 `.abk` club banks), Legacy Recompiled (no anchor).

| Audio-named field with a value in 55–66 | NHL 08 | 09 | 10 | 11 | 12 | 13 | 14 | 15 | Legacy | what the rows are |
|---|---|---|---|---|---|---|---|---|---|---|
| `exhibitionarena.chantaudioid` = **61** | 15 rows | 15 | 15 | 15 | 15 | 1 | 1 | 1 | 1 | "Hartwall Areena ™" (08–15) / "EA SPORTS ™ Arena" (Legacy); the 15 rows on 08–12 are duplicate arena rows (no `arenaid` link on those titles); tenants on 13–Legacy: the Finnish league (14–15 clubs) + Finland |
| `exhibitionarena.goalhorn` = **61** | 15 | 15 | 15 | 15 | 15 | 1 | 1 | 1 | 1 | **the same rows** |
| `exhibitionarena.periodhorn` = **61** | 15 | 15 | 15 | 15 | 15 | 1 | 1 | 1 | 1 | **the same rows** |
| `exhibitionarena.arenaaudioid` / `cityaudioid` = 61 | — | 15 / 15 | 10 / 10 | — | — | — | — | — | — | the same rows on 09; 10 of those 15 rows on NHL 10 (fields exist on 09 / 10 only) |
| `exhibitionteams.audioid` ∈ 64–66 | 0 | 0 | 0 | 1 | 1 | 1 | 3 | 2 | 1 | SHL clubs' announcer ids: AIK 66, Leksands IF 65, Örebro 64 (NHL 14); Leksand 65 / Örebro 64 (15); Örebro 64 (Legacy) — **none of these clubs has a club bank** (the 12 banks are Brynäs … Timrå), so a team `audioid` equal to a bank number is not a club-bank reference |
| `exhibitionplayerbiotable.audioid` ∈ 55–66 | 9 | 8 | 10 | 9 | 8 | 5 | 6 | 4 | 3 | player name-call ids (Bell 60, Bergenheim 65, Bergeron 66, Lauritzen 61 …) — a different id space |
| every other audio-named field (`audioid2`, `leaguearena.*`, `leagueteams.*`, `playernames.audioid` …) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | — |
| **non-audio integer fields** holding 55–66 | 179 fields / 12 680 rows | 178 / 14 121 | 212 / 15 967 | 364 / 23 525 | 400 / 24 391 | 403 / 26 381 | 400 / 21 947 | 391 / 23 121 | 396 / 22 689 | numerical coincidence rate of a 12-value window; why "equals a bank number" is never a binding by itself |

Consequences:

- The one arena whose chant id addresses a club bank carries **61 in all three of its audio fields** (goal horn, period horn, chant; plus
  the arena / city audio ids on 09 / 10). Horn bank 61 does not exist on any title (R82 §4 residual `goalhorn` ids include 61), chant
  bank `061_chant.abk` does (Linköping). The value therefore reads at least as plausibly as **the arena's audio-set number, authored
  once and copied into every audio field**, as it does as a deliberate reference to the Linköping bank. Both remain candidates:
  (a) the chant loader keys on `chantaudioid` and bank 061 loads at that arena (authoring inconsistency, club bank reachable);
  (b) 61 is an unshipped set number and the match with the club-bank block is coincidental (bank 061 unreachable from the database);
  (c) a selector derived from other fields / data / code. Nothing offline separates them.
- "The only database path to a club bank" → **"the only value of an inspected arena chant field that equals a club-bank number"**.
- The 12 club banks: **club association is by name inside the bank and in the module / control-bank name tables** (measured);
  **numerical candidate lookup**: bank number vs the census above (measured, no club's own arena or team row carries its bank number);
  **selection binding: none traced** (unchanged as a negative, now with its population stated).

### 1.3 T-CHANT-1 rev. 3 (observations unchanged from rev. 2; outcomes reworded — [USER-CARDS.md](USER-CARDS.md))

Hearing a Linköping-specific chant at the Finnish arena is **consistent with (a)** but does not prove `chantaudioid` caused it (the same
rows carry 61 in the horn fields; a code path could load by any of them or by none). Hearing a Frölunda-specific chant at Frölunda's home
shows **a club-chant path this census did not trace** — not "a code-side key". Hearing nothing shows neither that the content is
unreachable nor unused (generic chants can mask, the trigger may be situational). Next discriminating control, **not authorised now**:
an explicit treatment of the candidate input on a disposable copy (set one non-Finnish arena's `chantaudioid` to 61 with its horn fields
untouched, verify the active source, hold everything else constant) — the only test that would attribute an effect to that field.

### 1.4 Page wording (developer; replaces CHANT_SELECTION.md §4 and queue row 5 of this package)

"Twelve numbered chant banks (055–066) are club chants named inside the bank and in the crowd module / control-bank name tables.
Chant banks are loader assets listed by number. Of the audio-named database fields (17–20 per title, all tables), only the Finnish
arena's `chantaudioid` equals a club-bank number (61), and that arena's goal-horn and period-horn ids are 61 as well; no club's own
arena or team row carries its bank number. No consumer that selects a club bank has been traced. Listing only; no edit route."

## 2. Speex outlier (audit correction 2) — measurement kept, intent removed

R93's numbers stand: PS4 NHL 21 base `rink_ann_samples` single-block record, both decoders' output peaks 5 / 4 LSB, rms 0.40 / 0.90 LSB,
max abs difference 2 LSB, 10 / 12 frames Speex NB mode 1; the same title's `_inxp` control corr 0.98. **Wording replaced everywhere**
(STATUS, DEVELOPER-QUEUE row 3, handoff 09, the 2026-09-24 notes, card A-SPX-1): "silent placeholder record" / "silence" →
**"near-silent output under both tested decoders; unsuitable for validating onset or decoder fidelity"**. Not suppressed, not labelled
game-silent, no authoring or consumer intent inferred, no runtime qualification. The class label for the listing: "near-silent decoded
output (1 record, 12 frames)".

## 3. Scope and controls (audit correction 3) — exact denominators

| Statement as published | Exact statement (binding) |
|---|---|
| "PS3 11–Legacy" (queue row 1, handoff 09) | PS3 **NHL 11, 14, 15 and Legacy** (4 loose trees) + X360 NHL 10, 11, 12, 13, 14, 15 and Legacy (7 loose trees). Archive-backed PS3 09 / 10 / 12 / 13 and X360 09 were **not** measured by R91 — they are measured by R98 (this session, separate handoff), whose result does not retroactively widen R91. |
| "every stream" | every stream entry of every loose `.abk` on those 11 trees (29 962 referenced + 146 loop bodies) — structural framing only |
| "controls 15 / 15 named" | 15 named mutations per platform; **14 detected + 1 not detectable structurally** on each platform (PS3: main-data byte flip; X360: packet-body byte flip) — AST_FRAMING.md §3. Handoff 09's "14 / 13 detected" is a typo for 14 / 14. |
| "decoded" | R92: **14 vectors**; R96 (§6): **106 further vectors**, decoder acceptance only. Neither is "every stream"; COVERAGE.md's `decoded (diagnostic)` means "a member of that class was decoded", nothing per stream. |
| restart bit-identical → "the state-continuity requirement" | a property of the **reference decoder** (libavcodec) on the rebuilt frames; it says nothing about the console's own decoder state, trim or start behaviour |

## 4. Research / production boundary (audit correction 4)

Everything under `local/runs/overnight-research-2026-09-25/` and `docs/research/**` is research-owned: standalone parsers, probes,
fixtures and previews. No Studio code, no shared fixture, no `defs/`, no binding was changed by R91–R97. The developer's `tdb-snd`
crate and the archived-bank route are the developer's; the research probes do not depend on them and are not a proposal to vendor
anything. MANIFEST.md / `data/manifest.json` are **hash lists of inputs and outputs** — they are not evidence that any writer recovers,
and no writer exists in this lane. Retained review dependencies: the 51 WAV previews (62 MB, named by PREVIEW_REVIEW.md and cards
A-AST-1 / A-LOOP-1 / A-SPX-1) and the 288 representative block maps (46 MB, regenerable by the CHECKPOINT commands; kept only until the
review packet closes, then to be deleted as regenerable duplicates). `_scratch*` directories were deleted at the 06:25 closeout.

## 5. Two reporting corrections from the audit

- **R88 source count:** `data/r88-correction-checkpoint.json` holds **27** sources (25 anchors + NHL 19 patch 56 + the real NHL 20
  install). Handoff 06 line 8 and atlas row 06 said 28 — corrected by dated notes; no evidence changes.
- **`*artid*` destination occupancy (R88 §F3):** "check b against every `*artid*` column" is narrowed to **the team-art namespace**:
  `exhibitionteams.artid` (+ `leagueteams` / `customteams` copies where present), `stockteamjerseys` / `customteamjerseys.teamartid`,
  the PS4 awards / resort-extras `teamartid` columns (handoff 04 §2). Columns of other namespaces — player portrait / head ids,
  `arenaartid` / `centericelogo` (arena), `coachartid` (NHL 08 / 09), any league-art column — are **not** collision evidence for a team
  art id; an equal number there means nothing by itself. Unresolved and to be marked as such in the eventual contract: whether any file
  family is keyed by more than one namespace (the `centerlogo_{centericelogo}` arena-side key is the known case), and every reference
  inside archive members (Q-R88-2).

## 6. R96 — decode-contract validation matrix, run 13:36–13:38 MDT today, previously unregistered

Found at this session's open: `probes/r96_decode_matrix.py`, `data/r96-decode-matrix.json`, `r96-run.log` (exit 0), written after the
06:25 closeout by a continuation that left no STATUS entry, claim or handoff (the direction audit's "finite queue" continuation, queue 1).
Inventoried here rather than discarded; it is lane-owned research output. Bounded summary from its JSON (106 vectors selected from the
R91 summary so that every platform × codec × rate × channel-config × loop × block-count class has a decoded member, plus named boundary
cases; ffmpeg 8.1.1 as the only decoder; no new WAV):

| Check | Result | Reading |
|---|---|---|
| V1 declared vs decoded | EAL3: decoded = granules × 576 on 59 / 59 sub-streams; the identity decoded + PCM − 1152 − trim = Σ samples holds on 49 / 49 non-stub parts and **does not apply** to the 10 loop **stubs** (one block, output = warm-up only — the stub's own rule is V6); XMA: decoded = Σ block samples on 105 / 105, the RIFF header +1000 control changes nothing on 62 / 62 multi-block streams | counts confirmed on 106 more vectors |
| V3 restart from a middle block | bit-identical tail after 2 granules (EAL3, 36 / 36) / immediately (XMA, 62 / 62) | decoder-side continuity (see §3, last row) |
| V4 PCM frame vs decoder output at 1105 | on the **14 sub-streams whose PCM frame is loud enough to compare** (rms ≥ 20): planar reading ≥ 0.95 on 12 / 14, minimum 0.858 (one cc 3 loop body); interleaved reading median ≈ 0 on stereo. On the 45 quiet frames (rms < 20; every cc 5 frame among them) the correlation is uninformative (median 0.65–0.92) | **refines R92's "corr ≥ 0.995 on every non-silent vector"**: the match is established on loud PCM frames only; quiet frames are not evidence for or against; cc 5 remains an inference |
| V5 terminal flush (decoded beyond Σ samples + start) | median rms 6.6 but **15 / 49 flushes carry audio (rms > 50; max 3 574)** | the terminal trim discards non-silent decoder output on those streams — consistent with encoder padding trimmed by the sample word, not proof of what the console renders |
| V6 loops | a = stub samples and stub + body = num_samples on 8 / 8; body first delta 1152 / 384, body tail trims recorded | R91 loop reading confirmed on the sampled loops |
| V7 `mp3` (integer) vs `mp3float` | max abs difference 1 LSB on 50 / 59, identical on 9 | **not independent** (same libavcodec family) — stated in the JSON |

> **Note 2026-09-25 22:07 (R102, [14-correction-checkpoint-r97-r101-review.md](14-correction-checkpoint-r97-r101-review.md) §F3):** V4 "established on loud PCM frames only" reads **"12 / 14 loud frames ≥ 0.95; two
> exceptions — PS3 NHL 14 `crowds.abk` record 0 loop body (cc 3), sub-stream 1 at 0.858 and sub-stream 0 at 0.930; 45 quiet frames
> uninformative"**.

Nothing in R96 is listening or runtime evidence; it adds no replacement capability. Queue row added; claim `research-r96-decode-matrix`
created at this checkpoint with the run's real timestamps.

## 7. Where the dated notes were placed

`overnight-2026-09-25/`: CHANT_SELECTION.md (§1 rows, §2, §4), USER-CARDS.md (A-SPX-1 rev. 3, T-CHANT-1 rev. 3), DEVELOPER-QUEUE.md
(rows 1, 3, 5, 6 + rows 7 / 8 added), STATUS.md (corrections list, log), CHECKPOINT.md / TASKS.md (rewritten), handoff 09 (§1 rows,
§2), REPLACEMENT_READINESS.md (boundary line), COVERAGE.md (scope line; `gen_docs.py` patched to keep it on regeneration).
`overnight-2026-09-24/`: handoff 08 (title row, table row, §1, §2), handoff 06 (count), 00-ATLAS.md (rows 06 / 08), DEVELOPER-QUEUE.md
(rows 9 / 13), USER-CARDS.md (T-CHANT-1), FINAL-RECEIPT.md (R90 addendum), STATUS.md (R90 paragraph).

```
python local/runs/overnight-research-2026-09-25/probes/r97_chant_id_field_census.py docs/research/overnight-2026-09-25/data/r97-chant-id-field-census.json
python local/runs/overnight-research-2026-09-25/probes/r96_decode_matrix.py docs/research/overnight-2026-09-25/data/r96-decode-matrix.json   # as run at 13:36 MDT; not re-run here
```
Executed: r97 × 2 (the first run missed the `DBOFIELDTYPE_INTEGER` type string of the 08–13 metas and reported 0 integer fields there —
fixed, re-run; the JSON is the second run).

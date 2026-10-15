# Handoff R99 — Q-R88-2 answered for the gen7 EB archives: **no archive member shares bytes with another** (0 duplicate spans, 0 overlapping spans across 242 429 rows on six trees), so a team-art rename inside these archives moves exactly the members it names; the 16-byte entry row is `block | 0 | stored size | djb2(path)` on 242 429 / 242 429 rows (the path hash **must be recomputed** on rename — the only path-derived field), and the team-art families are re-enumerated from the real member grammars (rink logo `texlib_{id}_{a}_{b}`, socks `sock_…`, nameplates `name_…` as a candidate); PS4 NHL 21's 20-byte row carries `djb2(path)` too, but its `word_12` is a sorted, non-hash-of-25-variants value that stays unknown

Research lane, 25 September 2026, 14:40 → 15:05 MDT (real clock). Blocker-map item Teams 2.1 / 2.6 (re-key and kit repoint contracts),
handoff 06 §F3 "sharing" gap and Q-R88-2. Claim `research-r99-team-art-archive-spans`. Read-only over the EB archives of X360 Legacy,
PS3 Legacy, X360 NHL 15, PS3 NHL 15, PS3 NHL 12, PS3 NHL 13 (every `*.big` under each root) and the PS4 NHL 21 install archive
(`H:\Emulators\ShadPS4\Games\CUSA18182\databigs\inxp_00.big`, EB `0x00110400`). No member decoded beyond 16 header bytes for the `w4`
test; nothing written; no writer proposed; nothing game-verified.

Machine-readable: [`data/r99-team-art-archive-spans.json`](data/r99-team-art-archive-spans.json) (probe `r99_team_art_archive_spans.py`,
≈ 2 min; per archive: rows, duplicate / overlapping spans, family counts, sharing lists, hash test, `w4` distribution).

> **Note 2026-09-25 22:07 (R102, [14-correction-checkpoint-r97-r101-review.md](14-correction-checkpoint-r97-r101-review.md) §F1):** the title's "a team-art rename inside these archives moves exactly the members it names"
> and §1 / §5's "the name-table string and that one hash word and nothing else" are **withdrawn**. Measured facts kept: no shared or
> overlapping payload spans and `path_hash` = djb2 on the six trees (four titles, 71 archives) as shipped. A rename additionally meets
> fixed-width name slots sized to the longest basename, payload-offset row order, the unexplained header word `0x10`, dir-table entries,
> hash-collision checks, byte-identical loose aliases (Legacy / NHL 15) and in-payload references — plus destination occupancy, other
> titles, recovery and runtime lookup. Same-length rename and changed-length rebuild are separate, undesigned operations. The PS3 12
> "2 ids (0 / 1)" sock row is the two-file sock form read with a single-file regex (R56 / R74), not an id finding.

## Plain-language summary

The re-key contract of 24 September could not say whether renaming a club's art files inside the console archives might drag other
clubs' bytes along, because archives can store one payload once and point several names at it. Measured on six titles' archives
(242 429 entries), they never do: no two entries share a byte range and none overlap. Each entry is a block offset, a zero, a stored
size and a hash of the path, and the hash is the plain djb2 of the lowercase path on every row — so a rename inside an archive is the
name-table string, that one hash word and nothing else at the archive level; the payload stays where it is. The families that carry a
team art id in their file names were re-counted from the actual names (the rink-logo and sock grammars in the older handoff were
placeholders, not the real names), and the nameplate family is flagged as a candidate because which of its three numbers is the team
id has not been established. What remains unmeasured is inside the payloads — whether a jersey texture refers to a nameplate or crest
by name — and the PlayStation 4 archive's extra word, which sorts the table but matches no hash tried.

## 1. Entry row and path hash (rename cascade input)

| Title / archive format | rows | `w4` | `path_hash` = djb2-add over the lowercase `/`-separated path (`h = h·33 + c`, 32-bit, seed 5381) |
|---|---|---|---|
| X360 Legacy, 13 EB `0x00000400` archives | 44 602 | 0 on 44 602 | 44 602 / 44 602 |
| PS3 Legacy, 13 | 44 471 | 0 on all | 44 471 / 44 471 |
| X360 NHL 15, 13 | 40 837 | 0 | 40 837 / 40 837 |
| PS3 NHL 15, 13 | 40 705 | 0 | 40 705 / 40 705 |
| PS3 NHL 12, 8 | 35 568 | 0 | 35 568 / 35 568 |
| PS3 NHL 13, 11 | 36 246 | 0 | 36 246 / 36 246 |
| PS4 NHL 21 `inxp_00.big`, EB `0x00110400` (20-byte rows) | 2 950 | (not in this row layout) | 2 950 / 2 950 (`word_16`, as the 2026-09-19 handoff 01 stated) |

Stored paths are already lowercase (the "as-is" and "lower" variants hit identically). The `w4` word is **zero on every row** of the
gen7 archives (it is not the decompressed size: chunkzip / chunklzx members carry their decompressed size at member offset +8 and `w4`
stays 0). Consequence for a rename `a → b` inside a gen7 EB archive: the name-table string and the row's `path_hash` are the only
entry-level fields derived from the name; the block offset and size are untouched; whether the loader also keys by the hash at runtime
is not observable here (it is the only plausible reason for the hash to exist — stated as inference).

## 2. Spans: nothing is shared

| Title | archives | rows | duplicate spans (same offset + size) | partially overlapping spans | team-art rows | team-art rows sharing a span with another id / with a non-team member |
|---|---|---|---|---|---|---|
| X360 Legacy | 13 | 44 602 | 0 | 0 | 6 438 | 0 / 0 |
| PS3 Legacy | 13 | 44 471 | 0 | 0 | 6 440 | 0 / 0 |
| X360 NHL 15 | 13 | 40 837 | 0 | 0 | 5 300 | 0 / 0 |
| PS3 NHL 15 | 13 | 40 705 | 0 | 0 | 5 302 | 0 / 0 |
| PS3 NHL 12 | 8 | 35 568 | 0 | 0 | 7 355 | 0 / 0 |
| PS3 NHL 13 | 11 | 36 246 | 0 | 0 | 5 877 | 0 / 0 |

Handoff 06 §F3's "shared spans (BIG4 dedup, PS4 EB duplicate-block sharing) … not measured" is now measured for the gen7 EB archives:
**absent**. (The BIG4 archives of NHL 07–10 and the PS4 EB archives' block sharing are not in this measurement — stated.)

## 3. Team-art families by their real member grammar (Legacy X360 / PS3, NHL 15, PS3 12 / 13)

| Family (archive; measured stem) | id position | X360 Legacy | PS3 Legacy | X360 15 | PS3 15 | PS3 12 | PS3 13 | status of the grammar |
|---|---|---|---|---|---|---|---|---|
| FE crest `cache.big: fe/ion/artassets/teamlogos/t{id}.big` | the number | 318 (318 ids, 0–20 087) | 318 | 313 | 313 | 302 | 305 | established (handoff 04) |
| FE small crest `s{id}.big` | — | **0** (none in any archive; none loose either: 320 `t*`, 0 `s*`) | 0 | 0 | 0 | 0 | 0 | **not on these titles** — handoff 04 §3's "small `s{artid}`" row does not apply to Legacy / 15 / 12 / 13 |
| rink logo `cacherender.big: rendering/logo/texlib_{id}_{a}_{b}.rx2` | first number (handoff 04's `texlib_{artid}_a_b`, a / b placeholders) | 1 926 (348 ids, 0–20 085) | 1 928 | 1 784 | 1 786 | 1 701 | 1 700 | established grammar; **id position per handoff 04, not re-verified here** |
| centre-ice logo `cacherender.big: rendering/icesurface/centerlogo_{id}_cm.rx2` | the number | 231 (231 ids, 0–701) | 231 | 226 | 226 | 213 | 218 | established; arena-side key (`centericelogo`), listed for completeness — **not** a team-art rename target |
| puck `puck_{id}` | the number | 249 (0–777) | 249 | 244 | 244 | 212 | 216 | established |
| kit jersey `nocacherender.big: rendering/jersey/texlib_{style}_{id}_{variant}.rx2` | second number | 905 (222 ids, 0–421) | 905 | 683 | 683 | 967 | 865 | established (R56 / R76) |
| kit pant `cacherender.big: rendering/pant/texlib_{style}_{id}_{variant}.rx2` | second | 959 (221 ids) | 959 | 684 | 684 | 979 | 865 | established |
| kit sock `cacherender.big: rendering/sock/sock_{style}_{id}_{variant}_cm.rx2` | second (kit convention — **candidate**) | 946 (221 ids, 0–421) | 946 | 683 | 683 | **2 012 rows, 2 "ids" (0 / 1)** | 865 | Legacy / 15 / 13 consistent with the kit convention; **PS3 NHL 12's sock names follow another grammar** (the second number is 0 / 1 — the NHL 11 / 12 mirror-pair titles, R74 / R75) — id position on 12 not established |
| nameplate `nocacherender.big: rendering/jersey/name_{a}_{b}_{c}_cm.rx2` | **unknown** (second number gives 221 ids, 0–421 — matches the kit id set, so it is the candidate) | 904 | 904 | 683 | 683 | 969 | 843 | **candidate** — handoff 04's `rendering_name_…` was a placeholder; the id position must be established (a DB join against `stockteamjerseys.teamartid` would do it) before a rename touches it |
| create-team logos `rendering/czlogo/logoct_{a}_{b}_{nm|sm|tm}.rx2`, `logoct_{a}_dm` | — | 1 065 | — | — | — | — | — | not classified as team-art (create-a-team block); listed so it is not mistaken for a keyed family |

Distinct-id sets: the kit families (jersey / pant / sock / nameplate-candidate) share one id set per title (221–228 ids, max 421 / 505),
the FE crest and rink-logo sets reach 20 085–20 087 (the create-a-team placeholder block, handoff 04), the puck set reaches 777.
Per-title family censuses for the other gen7 titles (08–11, 14) remain to be measured before a cascade is proposed there (unchanged).

> **Note 2026-09-25 15:30 (R101, [13-archived-stream-decode-and-kit-name-key-positions.md](13-archived-stream-decode-and-kit-name-key-positions.md) §2):** the nameplate candidate is **settled** — `name_{style}_{teamartid}_{variant}_cm` (position 2 = `teamartid` on 6 / 6 titles); the rink logo is `texlib_{artid}_{style}_{variant}` (per style and variant); PS3 NHL 12 socks are a four-number grammar (`sock_{1}_{2}_{3}_{4}_cm.rpsgl`, 1 946 + 62 members) whose id position is **not** established — the "2 ids (0 / 1)" row above is that grammar read with the wrong regex, not a finding about ids.

## 4. PS4 NHL 21 `word_12` — bounded negative result

Sorted non-decreasing along the table (0 negative deltas), 2 395 distinct values on 2 950 rows, range `0x4e890`–`0xff7c9128`; not the
block offset, not `offset >> 4`, not a cumulative size (6 relations tested, 0 hits); not constant per directory (38 / 152 directories);
equal for names that differ only in a digit or the extension in 160 of 256 multi-row groups (`…_v01.cba` … `_v04.cba`; `3908523078.bin`
/ `.vlt`; `sc12b` / `sc13b`); **no 32-bit hash of 25 path variants × 5 algorithms (djb2 add / xor, FNV-1 / 1a, CRC-32) and no
8 / 16 / 24-bit fold matches** on 1 500 rows. Reading: a sort key computed from a canonicalised name (digits and extension folded)
by a function not among those tried — formula **unknown**; for any PS4 rename it must be treated as opaque (preserved verbatim is not
an option for a renamed row, so **PS4 archive renames stay refused**, consistent with handoff 06 §F3).

## 5. Consequence for the re-key / repoint contracts (Teams 2.1 / 2.6; still inspection-only, not Codex-accepted)

- Archive-level rename cascade on gen7 EB archives (Legacy, 15, 12, 13): per member, **name-table string + `path_hash` = djb2(new path)**;
  payload, offset, size unchanged; no member shared with any other name, so no collateral movement at the archive level.
- Still **unmeasured**: references *inside* payloads (a kit `.rx2` naming its nameplate / crest by path or by hash) — the F3 "cross-key
  references inside members" gap stays open; the nameplate id position; the sock grammar on NHL 11 / 12; every other title's family
  census; the runtime's use of the hash. **Default remains refuse** until those are closed; the read-only shared-art-id inspection of
  handoff 06 is unchanged.
- PS4: `word_12` unknown → archive renames refused (unchanged); the vault `Team` references (R73 / R7) are a separate, unresolved layer.

```
python local/runs/overnight-research-2026-09-25/probes/r99_team_art_archive_spans.py docs/research/overnight-2026-09-25/data/r99-team-art-archive-spans.json --ps4 "H:/Emulators/ShadPS4/Games/CUSA18182/databigs/inxp_00.big"
```
Executed × 2 (the first run's rink-logo / sock / nameplate regexes used handoff 04's placeholder grammars and matched nothing; the family
table above is from the second run with the measured stems). Three scratch follow-up scripts (family stems, `w4` vs decoded length, PS4
`word_12` groups / hash variants) ran once each and were deleted; their results are the sentences above.

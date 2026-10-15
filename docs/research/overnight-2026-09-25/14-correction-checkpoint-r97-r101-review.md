# Handoff R102 / R103 — correction checkpoint for the R97–R101 review (F1–F3): the archive-rename sentence narrowed to measured entry fields and per-archive payload spans, with the EB layout constraints a rename would meet now measured (rows in payload-offset order, fixed-width name slots sized to the longest name, header word `0x10` unexplained, loose copies that are byte-identical aliases of the archive members); R101's "settled" wording withdrawn — nameplates resolve per row by `(style, teamartid, variant)` and win every discriminating row, rink logos do **not** (16–119 rows unresolved, ~1 000+ unmatched tuples, and `jerseyindex` beats `variant` on NHL 13's discriminating rows); the NHL 11 / 12 sock "four-number grammar" is the two-file form R56 / R74 established and the developer's resolver already implements; the X360 NHL 09 decode explanation narrowed to three vectors with a byte-identical control; the two informative R96 PCM-frame exceptions carried by name; the developer queue split into current rows and preserved history

Research lane, 25 September 2026, 21:58 → 22:07 MDT (real clock). Review: [research-20260925-r97-r101-review.md](../../validation/research-20260925-r97-r101-review.md).
Claim `research-r102-correction-checkpoint-r97-r101`. Read-only over the EB archives and loose trees on `X:` and over `anchors/`; the
developer's kit resolver (`crates/studio-domain/src/kit_files.rs`) and EB reader (`crates/tdb-archive-big/src/eb.rs`) were **read, not
edited**. No application / defs / bindings / source-asset change; no launch; nothing staged, committed or pushed. Earlier handoffs keep
their text; each superseded sentence carries a dated note pointing here.

Machine-readable: [`data/r102-eb-layout-constraints.json`](data/r102-eb-layout-constraints.json) (probe `r102_eb_layout_constraints.py`),
[`data/r103-nameplate-rink-discriminating-join.json`](data/r103-nameplate-rink-discriminating-join.json) (probe
`r103_nameplate_rink_discriminating_join.py`; per-member SHA-256 of every nameplate / rink-logo copy),
[`data/r103b-loose-archive-copy-identity.json`](data/r103b-loose-archive-copy-identity.json) (probe `r103b_loose_archive_copy_identity.py`),
[`data/r100b-x360-09-rewrite-pcm-identity.json`](data/r100b-x360-09-rewrite-pcm-identity.json) (probe `r100b_x360_09_rewrite_pcm_identity.py`).

## Plain-language summary

Three corrections. First, yesterday's archive result said a rename inside an archive touches "the name and one hash word and nothing
else"; that went beyond what was measured. The archive layout was measured instead: each file name sits in a fixed-width slot exactly as
wide as the longest name in that archive, the entries are ordered by where their data lies, one header word is still unexplained, and
every nameplate and rink-logo file on the Legacy and NHL 15 discs also exists as an identical loose file. A rename therefore has to deal
with slot width, ordering, the unexplained word, the second copy and anything inside the payloads that names the file — none of which is
designed. Second, the claim that nameplate and rink-logo names were "settled" came from comparing sets of numbers. Testing each database
row against the files, on the rows where rival columns disagree, confirms the nameplate naming but not the rink-logo naming, which leaves
many rows and files unexplained. The NHL 11 / 12 sock names were never an open question: the earlier kit research and Studio's own
resolver already read them. Third, the NHL 09 audio decode is now stated for exactly the three streams tested, with a byte-for-byte
comparison, and the two loud audio frames that fell below the match threshold are named instead of summarised away.

## F1 — archive rename: what is measured, and what a rename would still have to handle

**Narrowed statement (replaces handoff 12's title / §1 consequence / §5 first bullet):** on the six title / platform trees inspected
(X360 and PS3 Legacy, X360 and PS3 NHL 15, PS3 NHL 12, PS3 NHL 13 — four distinct titles; 71 EB `0x00000400` archives; 242 429 rows),
no two rows share or partially overlap a payload span, and the fourth row word equals djb2 of the stored lowercase path on every row.
That is a statement about **entry fields and payload spans of those archives as shipped**. It is not a statement about other gen7 EB
files, NHL 07–10 BIG4 archives, PS4 archives, or any archive after an edit, and it says nothing about logical aliases or references
inside payloads.

R102 measured the structures a rename would meet (per archive, JSON `trees.*.archives.*`):

| Constraint | Measured on the six trees | Consequence for a rename proposal (none is designed here) |
|---|---|---|
| Name storage | fixed-width slots, `u16 dir index | basename NUL-padded`; **slot width = 2 + longest basename + 1 on 71 / 71 archives** (no spare byte for the longest name); dir table of `dir_count × dir_slot` after the name table (16-aligned); zero gap to the first payload on 71 / 71 | a **same-byte-length** basename change stays inside its slot; a **longer** name fits only while it stays within the archive's longest name (team-art basenames +3 digits fit on 6 366 / 6 438 X360 Legacy … 5 837 / 5 877 PS3 13 rows); a name longer than the slot means a **name-table rebuild** (slot width, dir table start, and — if the tables grow into the first payload — every payload offset); the two cases are separate operations |
| Row order | rows follow **payload offset** on 71 / 71 archives; path-hash order on only one small archive each on PS3 12 / 13 (coincidental size); **not** hash-sorted as on PS4 NHL 15 (developer `eb.rs` note) | a rename that changes the hash need not re-sort on these archives **if** the loader does not search by hash; whether it does is unobserved (runtime) |
| Directory index | the slot's `u16` selects a directory string; renames inside one directory keep it | a rename into another directory needs a dir-table entry (present or added — growth = rebuild) |
| Header word `0x10` | matches none of name end, dir start / end, first payload, `file_count × name_slot`, `dir_count × dir_slot` on 71 / 71 | unknown field; preserve verbatim is the only safe rule, and whether a rebuild must change it is unknown |
| `archive_size` | equals the file length on 71 / 71 | a rebuild that moves payloads must update it (the PS4 NHL 15 anomaly the developer recorded does not occur here) |
| Path-hash collisions | 0 inside any archive, 0 across each tree's archives | a destination name must also be checked for a hash collision, not only a path collision |
| **Aliases (R103b)** | every nameplate and rink-logo member on X360 / PS3 Legacy and X360 / PS3 NHL 15 **also exists as a loose file whose bytes equal the unwrapped member** (904 + 1 926 X360 Legacy; 904 + 1 928 PS3 Legacy; 683 + 1 784 / 1 786 NHL 15); PS3 12 / 13 have archive copies only | a rename is at least **two** logical objects per asset on those titles (archive member + loose file); which one the game reads is unobserved |
| Destination occupancy, team-art namespace | per family, the ids with members, in the JSON (`occupancy`); 746–761 ids in 0–1 023 have a member in **no** inspected family per tree | occupancy only, in the inspected archives; loose-only files, the DB team-art columns (handoff 10 §5), other archives / titles and runtime caches are separate checks. **PS3 NHL 12's sock occupancy is wrong in this count** (R99's single-file regex reads the two-file sock form as ids 0 / 1), so its free-id figure is overstated |
| Transaction / recovery | not designed | unchanged requirement (handoff 06 §F3) |

Runtime lookup (by name, by hash, by index or by offset) remains unobserved. PS4 archive renames stay **refused** (`word_12` unknown,
vault references unresolved). The CHECKPOINT's "in-payload references = the last unmeasured element" is withdrawn: in-payload
references are one requirement among destination occupancy, other-title coverage, naming / layout rules, transactional recovery and
runtime behaviour.

> **Note 2026-09-26 01:15 ([overnight-2026-09-26/02-ps3-12-13-payload-references.md](../overnight-2026-09-26/02-ps3-12-13-payload-references.md) §2):** the F1 table's "PS3 NHL 12's free-id figure is overstated" is withdrawn as a conclusion: recounted like-for-like with the two-file sock grammar (sock ids 2 → 228), the free-id count stays 761 (NHL 13 757). The sock input was wrong; the count was not affected.

## F2 — naming associations, per row, with the discriminating rows

**Already established by independent evidence (kept, not weakened):** R56 (2026-09-21 pm handoff 14) joined every `stockteamjerseys`
row to its jersey / pant / sock textures by `(style, teamartid, variant)` — 627–704 / 627–704 per title on NHL 11–Legacy, both consoles,
PS3 12 / 13 by archive listing — and recorded `fevariant = variant` on every row and `(style, teamartid, jerseyindex)` resolving only
457–534. R74 / R76 established the NHL 11 / 12 two-file sock form `sock_{style}_{0|1}_{artid}_{variant}_cm` (mirror pairs). The
developer's resolver implements exactly these stems (`kit_files.rs` `file_stem` / `parse_stem`: single-file and two-file sock grammars)
and resolves the required components on the real trees (`studio-domain/tests/kit_selector_join.rs`). **R101's "PS3 NHL 12 socks — id
position not established" was a gap in R101's regex, not an open mapping; withdrawn.** The PS3 12 members in R101 / R99 counted as
"2 ids (0 / 1)" are the two-file sock form.

**New: per-row joins with competing columns (R103).** Each row's own values are looked up as a full tuple in the member names; the
"discriminating" rows are those where the alternative's tuple differs from the reference tuple.

| Family | Title | reference resolves rows | alternative → rows resolved; on its discriminating rows: this / reference | members without a row (ref.) | rows without a member (ref.) |
|---|---|---|---|---|---|
| nameplate `name_{a}_{b}_{c}_cm` vs ref. `(style, teamartid, variant)` | X360 / PS3 Legacy | **703 / 703** | `islightcolor`: 437 (104 / 370); `isdefault`: 462 (52 / 293); `jerseyindex`: 495 (207 / 415); `fevariant`: identical to `variant` on every row | 206 | 0 |
| | X360 / PS3 NHL 15 | **659 / 659** | 413 (94 / 340); 450 (44 / 253); 418 (157 / 398); identical | 26 | 0 |
| | PS3 NHL 12 | **676 / 676** | 424 (98 / 350); 427 (40 / 289); 574 (324 / 426); identical | 295 | 0 |
| | PS3 NHL 13 | **684 / 684** | 416 (88 / 356); 431 (32 / 285); 457 (210 / 437); identical | 163 | 0 |
| rink logo `texlib_{a}_{b}_{c}` vs ref. `(teamartid, style, variant)` | X360 / PS3 Legacy | 687 / 703 | `islightcolor`: 513 (188 / 362); `isdefault`: 497 (95 / 285); `hastie`: 553 (149 / 283); `jerseyindex`: 588 (306 / 405); `fevariant` identical | 1 244 / 1 246 | 16 |
| | X360 / PS3 NHL 15 | 567 / 659 | 454 (181 / 294); 455 (69 / 181); 475 (161 / 253); 539 (298 / 326); identical | 1 219 / 1 221 | 92 |
| | PS3 NHL 12 | 676 / 676 | 560 (234 / 350); 555 (168 / 289); 588 (204 / 292); 576 (326 / 426); identical | 1 027 | 0 |
| | PS3 NHL 13 | 565 / 684 | 475 (203 / 293); 491 (104 / 178); 488 (187 / 264); **572 (336 / 329) — `jerseyindex` beats `variant`** | 1 139 | 119 |

Readings:
- **Nameplate:** the full triple `(style, teamartid, variant)` names a member for every row on six title / platform trees and resolves
  every discriminating row where each alternative fails most of them → **naming association established on the rows**: `name_{style}_{teamartid}_{variant}_cm`,
  same key as the jersey. `variant` vs `fevariant` cannot be distinguished (identical columns); 26–295 members per title have no row
  (spare names, as R56's spare kits). This is a naming association, **not** runtime selection (the game's nameplate choice is unobserved).
- **Rink logo:** **not established.** The `(teamartid, style, variant)` reading leaves 16–119 rows without a member and ~1 000–1 250
  member tuples without a row; alternatives resolve large shares of the discriminating rows, and on PS3 NHL 13 `jerseyindex` resolves
  more discriminating rows than `variant`. Position 1 is the team art id (R99 / handoff 04; not re-tested here); positions 2 and 3 are
  **candidates**. The file set is roughly 2.5× the kit rows, so the rink logo is likely keyed by something other than kit rows
  (per-team logo sets, arena usage) — untested. Card-worthy only after a static hypothesis exists; none is proposed.
- **Multiplicity:** members are one per archive; Legacy / NHL 15 add a loose copy (F1 aliases); PS3 12 / 13 archive only.

## F3 — diagnostic controls and residuals, exact

- **X360 NHL 09 decode (R100 → R100b):** three vectors (`playercalls` 254, `crowds` 41, `colors1` 69) decode through the unchanged R92
  XMA route with the declared counts; the ffmpeg `xma2` output is **byte-identical** under the shipped headers, the synthesised XMA2
  headers and fully zeroed header words (`data/r100b-…json`: 0 differing samples on each). Scope: these three vectors, ffmpeg 8.1.1.
  The shipped headers already carry frame-offset 0, so the zeroed control changes only the top six bits and the metadata / skip bits —
  whether the decoder uses the offset field is **untested**. The sentence "the decoder ignores the constant header word" is replaced by
  "on these three vectors this decoder's output did not depend on the header's frame-count / metadata bits". The RIFF sample-count
  control of R92 tests a different field and is not evidence for packet headers. The 87 residual middle blocks of R98b stay open;
  selected decode success does not extend structural acceptance.
- **R96 PCM-frame match:** 14 loud PCM frames (rms ≥ 20), 12 at planar correlation ≥ 0.95; **two informative exceptions**, both the
  PS3 NHL 14 `crowds.abk` record 0 loop **body** (cc 3): sub-stream 1 at 0.858 (per-channel 0.873 / 0.854; rms 60) and sub-stream 0 at
  0.930 (0.942 / 0.846; rms 86). The 45 quiet frames (rms < 20; every cc 5 frame) are uninformative. "Established on loud frames" is
  replaced by "12 / 14 loud frames ≥ 0.95, two exceptions named, 45 quiet frames uninformative". The start assembly stays a diagnostic
  candidate.

## Developer queue

[DEVELOPER-QUEUE.md](DEVELOPER-QUEUE.md) now holds **current rows only**, each with its corrected consumption wording and a link to the
preserved text; the full earlier table and its dated notes moved verbatim to [DEVELOPER-QUEUE-HISTORY.md](DEVELOPER-QUEUE-HISTORY.md).
The one change to the history text is the repair of row 6, whose first cell had been duplicated by a note-insertion script at 14:20
(the duplicated fragment is recorded in the history file's header).

```
python local/runs/overnight-research-2026-09-25/probes/r102_eb_layout_constraints.py docs/research/overnight-2026-09-25/data/r102-eb-layout-constraints.json
python local/runs/overnight-research-2026-09-25/probes/r103_nameplate_rink_discriminating_join.py docs/research/overnight-2026-09-25/data/r103-nameplate-rink-discriminating-join.json
python local/runs/overnight-research-2026-09-25/probes/r103b_loose_archive_copy_identity.py
python local/runs/overnight-research-2026-09-25/probes/r100b_x360_09_rewrite_pcm_identity.py docs/research/overnight-2026-09-25/data/r100b-x360-09-rewrite-pcm-identity.json --scratch <scratch dir>
```
Executed once each (R103b first ran as a scratch script and was promoted unchanged into the lane's probes; its JSON is copied to `data/`).

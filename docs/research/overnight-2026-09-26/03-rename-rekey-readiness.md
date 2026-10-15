# Handoff R109 (P3) — bounded readiness assessment for a gen7 team-art rename / re-key: six separable layers (outer archive name, loose alias, internal raster names, shared team-art rows, destination occupancy, transaction); what is measured, which conditions would permit further **offline design**, and which questions stay **runtime**; same-length and changed-length operations kept apart; no writer authorised

Research lane, 26 September 2026, 01:14 → 01:16 MDT (real clock). Review package 3. Claim `research-p3-rename-rekey-readiness`. Scope: gen7
X360 / PS3 NHL Legacy and NHL 15, PS3 NHL 12 / 13 (the trees with measured evidence). **Excluded:** NHL 07–11 and 14 (no measured family
census), PS4 (renames refused: `word_12` unknown, vault references unresolved — no gen7 rule transfers). New measurement: R109 destination
pool ([`data/r109-destination-pool.json`](data/r109-destination-pool.json)); everything else cites earlier handoffs.

> **Correction 2026-09-26 02:35 (review `research-20260926-r107-r112-review.md` finding 3; raw results unchanged).** (1) The R109 pool
> is **absence from the enumerated sources only**: the DB columns `exhibitionteams.artid`, `leagueteams.artid`, `customteams.artid`,
> `stockteamjerseys.teamartid`, `customteamjerseys.teamartid` (where present) and the team-art asset families jersey, nameplate, pant,
> sock, "rink logo", FE crest (`teamlogos/t{id}.big`) and puck, in the EB archives and loose. It is not "no art file uses the id". (2) The
> **centre-ice logo family** (`rendering/icesurface/centerlogo_{id}_cm`) is **excluded** because it is keyed by the arena-side
> `centericelogo` value, a separate namespace (2026-09-25 handoff 10 §5, handoff 12 §1). That is a boundary, not a finding: whether a
> destination id also names a centre-ice logo is not checked, and a re-key that also moves centre logos is a separate operation. Other
> `*artid*` namespaces (`arenaartid`, `coachartid`, league art) are excluded likewise. (3) The DB byte mask in §2 item 4 now includes
> the required **TDB CRC updates**, which change bytes outside the art-id cells. (4) A same-length offline rename **design** is not a
> working team re-key: the six layers stay separate, and the internal raster names (layer C) and the runtime consumer / lookup questions
> remain unresolved.

## Plain-language summary

Renaming or re-numbering a team's art is not one change but six, and they do not all have the same status. The archive entries, the
loose copies and the file-internal picture names are now measured well enough that a careful offline design could be written for the
simplest case — a new name of exactly the same length, in the same folder, on one installation. What cannot be settled from the files is
how the game finds these files at run time (by name, by hash, by position), which copy it prefers when both exist, whether it cares about
the internal picture names, and whether anything caches art by number. A longer or shorter name is a different, larger operation that
rebuilds the archive's name table and moves data. ~~The measured pool of numbers that neither the database nor any art file uses is
roughly 750 per installation~~ *(Corrected 2026-09-26 02:35:)* About 750 numbers per installation appear in none of the database
columns and art families that were checked (the centre-ice logos, keyed by arena, were left out and not checked). "Unused in the checked
files" is not proof the game has no use for a number, and a same-length rename design is not a working team re-key.

## 1. Layers

| Layer | Measured (evidence) | Offline-designable now? | Runtime / unobservable |
|---|---|---|---|
| **A. Outer archive name** (EB 0x00000400) | fixed-width slot `{u16 dir | basename}`, slot width = longest basename + 3 on 71 / 71 archives; row = `{block, 0, size, djb2(lowercase path)}`; rows in payload-offset order; no shared / overlapping spans; no path-hash collisions; header word `0x10` unexplained; `archive_size` = file length (R99, R102) | **Same-length, same-directory rename: yes, as a design** — rewrite the basename bytes in place and the row's hash; preserve every other byte (offsets, sizes, order, `0x10`, `archive_size`). **Changed length: no** — needs a name-table rebuild whose effect on `0x10`, the dir table and payload offsets is unmeasured | lookup key (name / hash / index / offset); whether a stale order matters; caches |
| **B. Loose alias** | every nameplate / logo member (full family) and 12 sampled pairs per other family equal their loose file after unwrap (Legacy, NHL 15); PS3 12 / 13 kits archive-only, FE crests loose on 12 / 13 too (R103b, R104, R109) | yes, as a rule: a rename must address **both** copies consistently (rename both, or refuse) | which copy the game reads; whether the other is ignored |
| **C. Internal raster names** (`0xEC0010`) | every member names its rasters with its own tuple, except 4 NHL 13 members; one of them (`name_1_138_2_cm`) is DB-referenced (R104, R108) | yes, as a **choice to be made explicit**: keep (byte-preserving) or rewrite (dictionary strings are NUL-terminated inside the chunk; same-length rewrite is in-place, changed length resizes the chunk) — design both, default keep | whether the engine resolves rasters by these names (card K-NAME-1 tests the shipped counterexample) |
| **D. Shared team-art rows** | `artid` is shared by a club and its variant rows (1–45 rows per id; handoff 04); kit rows key by `(style, teamartid, variant)` (R56); custom-team rows share 20 054 | yes: the operation's unit is **all rows holding a** plus their kit rows; refuse re-keying one row of a shared id silently; refuse a = 20 054 / spare ids | code-side tables keyed by art id (not observable) |
| **E. Destination occupancy** (target installation) | DB team-art columns (`exhibitionteams.artid`, `stockteamjerseys.teamartid`, `customteamjerseys.teamartid` on these titles) + every team-art family in archives + loose files: 212–218 DB ids, all with assets; 137–149 **asset-only** ids; **746–761 ids in 0–1 023 free in both** per tree (R109); other `*artid*` namespaces excluded (handoff 10 §5); **boundary (2026-09-26 02:35):** the arena-keyed centre-ice logo family is excluded and unchecked | yes: check b against DB columns, every family (archive + loose) and path-hash collisions of every renamed path; default refuse on any hit | runtime caches; code tables; saves that mirror team tables (roster saves carry `exhibitionteams` copies) |
| **F. Transaction** | not designed | yes, design work: one logical operation over DB + archive(s) + loose files, journal, SHA-256 of every touched file before / after, exact baseline restore | power-loss atomicity (never claimed) |

## 2. Conditions that would permit further offline design (a narrow proposal, not a writer)

1. Target = one installation of one measured tree (Legacy X360 / PS3, NHL 15 X360 / PS3, PS3 12 / 13); same-length, same-directory
   outer rename only: b drawn from the R109 pool **with the same number of decimal digits as a** (so every renamed basename keeps its
   byte length), re-checked on the target.
2. Both aliases handled (B), internal names kept byte-for-byte by default (C), all rows of a moved together (D), occupancy and hash
   collision checks (E), journal + restore (F).
3. Every assumption listed as unresolved: runtime lookup key, alias precedence, internal-name use, caches / code tables, save mirrors.
4. Byte-preservation obligations stated per file: archive — only the renamed slot bytes and the row hash change; loose — rename only,
   bytes unchanged; DB — ~~only the art-id cells~~ the art-id cells **plus the TDB CRC fields the existing commit path must recompute**
   (inner-DB CRC-32/MPEG-2 scopes; container checksums where the target is a save), nothing else (corrected 2026-09-26 02:35).
5. A design meeting 1–4 is still not a working team re-key (six layers; layer C and runtime lookup / consumers unresolved).

Nothing here authorises implementation; runtime acceptance stays the user's.

## 3. Refusals kept

Changed-length names (rebuild unmeasured); PS4 (all); NHL 07–11 / 14 (no family census); a = 20 054 or spare ids; any b hit in DB,
archive, loose or hash space; partial alias handling; a single row of a shared id.

```
python local/runs/overnight-research-2026-09-26/probes/r109_destination_pool.py docs/research/overnight-2026-09-26/data/r109-destination-pool.json
```

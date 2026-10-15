# Handoff R108 (P2) — parsed references of team-art / kit members on the archive-only PS3 NHL 12 / 13, from their own chunkzip members: 6 775 + 5 356 members, all chunkzip with declared size = decoded size, 0 refusals, empty subreference / atom tables and the three constant external-arena values everywhere; NHL 12's socks read under the established two-file grammar; the R102 free-id figure survives the sock correction unchanged; **one shipped counterexample to "internal names match the outer name"**: NHL 13's `name_1_138_2_cm` (referenced by a Hershey Bears kit row) is a pixel copy of `name_1_138_1_cm` whose raster is still named `name_1_138_1`, and the unreferenced kit `(1, 421, 2)` is a pixel copy of Sudbury's `(1, 400, 2)` with internal names for art id 400

Research lane, 26 September 2026, 01:11 → 01:14 MDT (real clock). Review package 2. Claim `research-p2-ps3-12-13-payload-references`.
Read-only over the PS3 NHL 12 / 13 EB archives on `X:` (members decoded in memory) and `anchors/ps3/nhl13`. Parser: R104's RW4 reader
imported unchanged. Sock grammar: R56 / R74 two-file form on NHL 12 (as the developer's `kit_files.rs` `TwoFile` grammar). Nothing
game-verified; loose / member precedence not inferred (these titles ship the kit and logo families as archive members only; their FE crests do also exist loose — R109).

Machine-readable: [`data/r108-ps3-12-13-payload-references.json`](data/r108-ps3-12-13-payload-references.json),
[`data/r108b-occupancy-reconciliation.json`](data/r108b-occupancy-reconciliation.json), [`data/r108c-ps3-13-cross-named-kits.json`](data/r108c-ps3-13-cross-named-kits.json).

## Plain-language summary

The two PlayStation 3 titles that keep their kit and logo files only inside archives were read member by member. Each archive member
unpacks to exactly the size its wrapper declares, and every file's internal reference tables are empty, as on Legacy and NHL 15. Earlier
counting had misread NHL 12's two-file sock names; recounting with the correct pattern leaves the pool of unused art ids unchanged. The
new finding is that EA's own data breaks the rule that a file's internal picture names repeat its outer name: on NHL 13 one nameplate
used by a real Hershey Bears kit is a copy of another Hershey nameplate and still carries the old internal name, and one unused kit file
is a copy of a Sudbury kit that still carries Sudbury's number inside. If that Hershey kit shows its nameplate correctly in the game, the
game does not depend on those internal names — a question only a short in-game look can answer.

## 1. Coverage and provenance

| | PS3 NHL 12 | PS3 NHL 13 |
|---|---|---|
| members (jersey / nameplate / pant / sock / logo / centre logo) | 967 / 969 / 979 / 1 946 (two-file) / 1 701 / 213 = **6 775** | 865 / 843 / 865 / 865 (single-file) / 1 700 / 218 = **5 356** |
| wrapper · declared (chunkzip u32 +12) = decoded size | chunkzip 6 775 · equal 6 775 | chunkzip 5 356 · equal 5 356 |
| refused (decode or RW4) | 0 | 0 |
| subreferences / atoms (decoded counts) | 0 / 0 on all | 0 / 0 on all |
| external-arena list | `04000000 ffb00000 04000000` on all (meaning not decoded) | same |
| internal names = member's own tuple | all (jersey 5 798, nameplate 969, pant 2 937, sock 1 946, logo 5 102, centre logo 213) | all but **11 names on 4 members** (§3) |
| other-member stems / djb2 path hashes in non-payload bytes | 0 / 0 (random expectation 27.9) | 0 / 0 (18.3) |
| sock names outside the kit grammars (residuals, not forced) | `sock_#_#_#_#.rpsgl` 40, `_am` 18, `sock_#_#_nm` / `_sm` 4 + 4, `sock_#_nm` / `_sm` 2 + 2 | `playersock_#_#` 16 (+ 8 `_am`), `goaliesock_#_#` 4 (+ 1), `sock_#_nm / _rm / _sm` 2 each |

Per-member stored and decoded SHA-256 for one example per family, and the archive / row identities, are in the JSON (`examples`).

## 2. Occupancy reconciliation (R102 used the superseded single-file sock pattern)

Like-for-like with R102's families (FE crest, logo, pant, puck, sock, nameplate, jersey — stored per-family id sets), replacing only the
sock ids: NHL 12 sock ids 2 → **228**, ids 0–1 023 free in every family **761 → 761**; NHL 13 sock ids 215 → 215, 757 → 757. The sock
ids lie inside the other kit families' ids, so R102's free-id figure stands; the note in handoff 14 §F1 ("overstated") is withdrawn as a
conclusion — the input was wrong, the count was not affected. (A first comparison in R108 that omitted the FE-crest and puck families gave
773 / 772; it is not like-for-like and is superseded by R108b.)

## 3. Shipped internal-name mismatches (NHL 13; R108c)

| Outer member | Internal raster names | Pixel payload | DB reference (anchor `ps3/nhl13`) |
|---|---|---|---|
| `rendering/jersey/texlib_1_421_2` | `font_1_400_2_*`, `jersey_1_400_2_*` | identical to `texlib_1_400_2` | **no row** `(1, 421, 2)`; art id 421 = Sherbrooke Phoenix; `(1, 400, 2)` = Sudbury Wolves (team 167, jerseyindex 2) |
| `rendering/pant/texlib_1_421_2` | `pant_1_400_2_*` | identical to `pant/texlib_1_400_2` | no row |
| `rendering/sock/sock_1_421_2_cm` | `sock_1_400_2_cm` | identical to `sock_1_400_2_cm` | no row |
| `rendering/jersey/name_1_138_2_cm` | `name_1_138_1_cm` | identical to `name_1_138_1_cm` | **row `(1, 138, 2)` exists** — Hershey Bears (team 42), jerseyindex 2, not default |

NHL 12 has no mismatch. Readings: the `(1, 421, 2)` set is an unreferenced copy (a spare, no runtime evidence either way). The Hershey
nameplate is a **DB-referenced shipped asset whose internal raster name differs from its outer name** — static evidence that such a state
exists in retail data. It does not show that the engine ignores internal names: the row is not the default kit, and whether that
nameplate renders was not observed.

## 4. Card K-NAME-1 (bounded runtime observation)

PS3 NHL 13: choose Hershey Bears, select the kit whose `jerseyindex` is 2 (the third kit; not default), view a player's back in any
replay / edit-lines view. **Outcome:** the nameplate lettering renders → the renderer resolved `name_1_138_2_cm` despite its internal name
`name_1_138_1_cm` (evidence that internal names are not a lookup key on this path); missing / placeholder lettering → internal names may
matter, or another cause (kit unavailable in that mode). Control: kit jerseyindex 0 (default, internal name matches).

## 5. Developer-facing contract

Inspection only: the archive-backed kit members parse under the same RW4 reading as Legacy / NHL 15 with the same empty reference tables;
the two-file sock grammar applies on NHL 12 (as implemented); list internal raster names behind disclosure; flag outer / internal name
mismatches (4 members on NHL 13) as "internal name differs" without treating them as errors. No rename or re-key operation follows.

```
python local/runs/overnight-research-2026-09-26/probes/r108_ps3_12_13_payload_references.py docs/research/overnight-2026-09-26/data/r108-ps3-12-13-payload-references.json
python local/runs/overnight-research-2026-09-26/probes/r108b_occupancy_reconciliation.py docs/research/overnight-2026-09-26/data/r108b-occupancy-reconciliation.json
python local/runs/overnight-research-2026-09-26/probes/r108c_ps3_13_cross_named_kits.py docs/research/overnight-2026-09-26/data/r108c-ps3-13-cross-named-kits.json
```
Executed once each (R108 after moving the chunkzip declared-size read from +8, the chunk count, to +12).

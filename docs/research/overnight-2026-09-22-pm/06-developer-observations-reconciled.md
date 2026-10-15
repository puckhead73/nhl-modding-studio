# Handoff R74 — The developer's ten observations (2026-09-22 `OBSERVATIONS-FOR-RESEARCH.md`) reconciled: the NHL 11 / 12 sock `0 / 1` sub-index is a **mirrored pair** (the `_1_` file is the horizontal mirror of the `_0_` file on 1 163 / 1 729 pairs, identical on 277, symmetric or near-mirror on the rest — one kit, two files, never a selector between kits); `seat*zone*` fields are reclassified as **colour**; PS3 NHL 12 / 13 kit members are `.rpsgl` (never byte-identical to X360 `.rx2`); the remaining rows are counting-rule or reading differences, each answered

> **Correction note — 2026-09-23 (coordinator review item 4; [R75](07-correction-checkpoint-r69-r74-review.md)).** The O-2 sock classes stay separate wherever summarised: clear mirror 459 / 704, identical (symmetric) 153 / 124, mirror-like or other 120 / 119, flat 24 / 26 (NHL 11 / NHL 12). "Every pair is a mirror" is not claimed; both files remain required and their runtime assignment is not inferred (Q-R74-1).

Research lane, 22 September 2026 pm (prompt item 4: Teams 2.5 / 2.6, Players 3.4, Equipment 4.x, Misc 5.4 — reconciliation of the
developer lane's measured departures before further parity work). Claim `research-r74-developer-observations`. Read-only over the X360
NHL 11 / 12 loose `rendering/sock` trees and the PS3 NHL 12 / 13 archive member listings; the rest is documentation against the existing
JSONs. Nothing game-verified; no runtime claim.

Machine-readable: [`data/r74-nhl11-12-sock-pairs.json`](data/r74-nhl11-12-sock-pairs.json) (per title: pairs, class counts, dims,
the "other" examples with their direct / mirrored / flipped block-mean distances). Measured inline (numpy over the r42 parser and the
r66 tiled gather; block-mean images of the 512 × 512 DXT1 base level, mean absolute difference 0–255).

## Plain-language summary

The developer lane published ten places where its own measurements departed from, or added to, this lane's handoffs. Each is answered
here so the two lanes carry one reading. Two needed new measurements. The socks on NHL 11 and NHL 12 come in two files per kit, `_0_` and
`_1_`, and nobody knew what the second selects: comparing the two images shows the second is the **mirror image** of the first on two out
of three kits (a symmetric design makes the two identical on a further sixth, and the rest are mirror-like with small differences), so the
pair is one sock seen from the two sides, not two alternative kits — a kit re-pointing on those titles must move both files, and
completeness rightly requires both. The seat-colour fields the developer flagged are re-classed from "capacity" to "colour" in this
lane's inventory (their values are plainly 24-bit RGB, as the developer measured). And the PS3 NHL 12 / 13 kit textures inside the
archives are a different file type from the Xbox 360 ones, so they can share the *naming* contract but never the bytes. The other seven
rows are differences of counting rules or of reading, each stated with the rule this lane now adopts.

## Row by row

| # | Developer observation | Research answer | Status |
|---|---|---|---|
| O-1 | PS4 `0XTC` condition list = array of `u64` pointers (440 / 440), not a flat array | **Accepted.** The pm handoff 15 documented the 24-byte condition struct but not the record → condition addressing; the developer's pointer-array reading reproduces the census through the parser and is the reading of record. Dated note on handoff 15. | reconciled |
| O-2 | NHL 11 / 12 socks: two files per kit, `sock_<style>_<0│1>_<artid>_<variant>_cm`; meaning of `0 / 1` unmeasured | **Measured.** NHL 11: 1 512 `_cm` files = 756 complete pairs (1 582 files with the `am` / `nm` siblings), NHL 12: 1 946 `_cm` files = 973 complete pairs (2 016 with siblings) — both counts equal the developer's; every pair is 512 × 512 DXT1 × 10 mips on both files; bytes never identical. Base-level block-mean comparison: `_1_` = **horizontal mirror** of `_0_` (mirror MAD ≤ 1.5, ≥ 4 × better than direct) on 459 / 756 and 704 / 973; identical content (symmetric design) 153 / 124; flat 24 / 26; "other" 120 / 119 — mirror-like with detail differences (mirror MAD 2–6 vs direct 5–16) or fully symmetric textures where direct = mirror. **Reading:** the sub-index selects the mirrored counterpart of the same sock; NHL 13+ ships one file per kit (the mirroring moved out of the texture). Consequence: one kit = both files; completeness requires both (as the developer implemented); re-pointing must copy both; no "select 0 or 1" affordance. Runtime handedness (which file dresses which leg) is not claimed. | measured |
| O-3 | Teams-with-rows: 213 / 216 / 216 vs R56's 218 / 222 / 222 on NHL 12 / 14 / 15 | **Counting-rule difference, developer's rule adopted.** R56 counted distinct `teamartid` values among `stockteamjerseys` rows; the developer counts team-table rows owning ≥ 1 kit row through `(index, artid)`. Where several team rows share an art id (or a kit art id has no team row), the two differ. The team-row count is the one a Team page needs; R56's figure is relabelled "distinct kit art ids". Dated note on handoff 14 §1. | reconciled |
| O-4 | Spare pool contains incomplete triples once every component is required | **Accepted; wording changed.** R56 §2's "files without a row" becomes "complete candidates (all required components present) vs incomplete leftovers"; the developer's per-title counts (20 / 22 / 1 / 2 / 0 / 58 incomplete of 117 / 277 / 208 / 317 / 54 / 286) are the record. Dated note on handoff 14 §2. | reconciled |
| O-5 | `seat*zone*` classed `capacity` by name; measured value space is 24-bit RGB | **Reclassified in the research inventory:** `seat1zone1–6` / `seat2zone1–6` (12 per PS4 signature NHL 17–21; values such as `0xdfddd7`, `0x878787`, max `0xfd9581`, `distinct` 1–12) → class **`colour`**. The R51 name-based rule mis-fired on "seat"; the class is now the measured one and the developer-side override can be retired at the developer's discretion. Dated note on handoff 09; the JSON keeps the original class with the note pointing here. | reconciled |
| O-6 | PS3 NHL 12 / 13 kit textures live in `cacherender.big` / `nocacherender.big`; are the members byte-identical to the X360 loose files? | **No, by construction.** The archive members are `.rpsgl` (PS3 NHL 12: 5 022 kit-family members, NHL 13: 3 564 — every kit member has that extension; X360 files are `.rx2`). "X360 = PS3" in R56 meant the *name* grammar and the row counts, never the bytes. An archive-member join on PS3 12 / 13 can reuse the triple contract (names identical apart from the extension) but needs the EB-archive listing (`eb_list.parse`) for presence and the PS3 `.rpsgl` route for any thumbnail; the loose-tree resolver cannot cover those two titles. | answered |
| O-7 | NHL 09 / 11 carry no `kOtt.buDa` home-arena link; team-keyed families resolve for 152–165 of 174–187 arenas on tenanted titles | **Accepted.** R62 §2's "show through the home team" is scoped to titles with the link (13 / 15 / Legacy); on 09 / 11 the centre logo / banners have no key to run through and must be listed as "no tenant link on this title". The residual arenas whose tenant art id ships no logo / banner are listed by the developer's join; this lane adopts those counts. Dated note on handoff 19. | reconciled |
| O-8 | Portrait bucket count must exclude `playerheads/silhouettes/`; a third class *other* is needed beside bespoke / pool | **Accepted.** R53's bucket counts included the four silhouette stand-ins on NHL 09 (2 924 / 218 with them excluded); the developer's three-class model (bespoke / pool / other = head id 0 or below the floor) is adopted, and the two Legacy rows flagged without a file (Gergo Nagy, Maurizio Colella) are the record. Dated note on handoff 11. | reconciled |
| O-9 | `blocker_0_am` / `trapper_0_am` are style-level files, not id 0 (68 / 66 ids, not 69 / 67); NHL 13 mask paint 260 has no shell-0 file | **Accepted.** R52 keyed the id at the first numeric position; the developer's second-position keying is consistent with the family's templates, so 68 / 66 are the counts of record and the single-number stems are style-level files. NHL 13 mask 260 without shell 0 is the one exception to "shell 0 always" on that title. Dated note on handoff 10. | reconciled |
| O-10 | Distinct composers one fewer per title when the `None` row's empty composer is excluded and names are trimmed / case-folded | **Counting-rule difference, developer's rule adopted** (134 / 132 / 128 / 96): R54 counted the empty composer as a value. Dated note on handoff 12. | reconciled |

## Not claimed

Which leg each NHL 11 / 12 sock file dresses at runtime; whether the "other" pairs (mirror-like with detail differences) are authored
asymmetries or a different mirroring pipeline; anything about the `.rpsgl` payload beyond its extension.

## Reproduce

```
python local/runs/overnight-research-2026-09-22-pm/probes/r74_nhl11_12_sock_pairs.py docs/research/overnight-2026-09-22-pm/data/r74-nhl11-12-sock-pairs.json   (≈ 2 min)
```
Reads the two loose `rendering/sock` trees only. Executed twice (an inline development version on 24 then all pairs, then the probe
file above, which regenerated the JSON with identical counts). The PS3 member listing is `r56_gen7_kit_selector_join.member_names` on
`ps3-nhl12` / `ps3-nhl13` (inline, once). The seat-zone value space was read from `r51-arena-venue-field-inventory.json` (inline, once).

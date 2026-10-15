# Handoff R76 — PS3 NHL 12 / 13 kit triples from each title's own anchors and render-archive listings: 676 / 684 stock rows all complete, every component a `chunkzip`-wrapped `.rpsgl` member (jerseys only in `nocacherender.big`, pants and socks only in `cacherender.big`), no member appears in both archives or twice in one, spare pools reconciled exactly to the developer's per-team rule (255 / 22 and 208 / 1) once the rule is stated, and a fixture contract with member identities and stored-byte hashes for the archive-backed inspection and thumbnail route

Research lane, 23 September 2026 (brief item 1; Teams 2.6). Claim `research-r76-ps3-archive-kit-join`. Reconciled against developer
receipts [03](../../receipts/overnight-development-2026-09-23/03-r4-ps3-archive-kits.md) and
[04](../../receipts/overnight-development-2026-09-23/04-r4-archive-member-preview.md) (read only). Read-only over
`anchors/ps3/nhl12|nhl13/databases/nhlng.db` and the two render archives' front tables; the stored bytes of the eight fixture members
were read for hashing; nothing decoded, nothing written; nothing game-verified.

Machine-readable: [`data/r76-ps3-archive-kit-join.json`](data/r76-ps3-archive-kit-join.json) — per title: archive identities
(version, file count, kit-family member counts), every stock row with its members (archive, member path, table index, stored size),
the class and missing components, spare candidates / incomplete leftovers (global rule), members in both archives / twice in one
(none), payload magic per family, sibling files not keyed, and the fixture contract. Probe `r76_ps3_archive_kit_join.py` (≈ 1 min).

## Plain-language summary

The developer's two receipts of this morning already list and preview the PlayStation 3 NHL 12 and 13 kits from their archives. This
package re-derives the join from the research side — the anchors' kit rows against the archives' file tables — so that every number the
developer published has an independent source. They agree exactly: 676 and 684 rows, all complete, 2 704 and 2 052 components taken from
the archives, jerseys in one archive and pants / socks in the other. Two things the receipts did not state are now recorded: the
members are wrapped as `chunkzip`, not `chunklzx` as receipt 04's summary says (Studio's member decoder handles both, so the preview
route is unaffected, but the "no encoder" limitation named there is the `chunklzx` one and does not describe these members); and the
spare counts differ between the lanes only because the rules differ — counted globally the research pool is 289 / 21 and 184 / 2,
counted per team under the team's art id (the developer's rule) it is 255 / 22 and 208 / 1 on both sides. A small fixture contract
(one stock row and one complete spare per title, with archive, member path, index, offset, size, hash and magic) is provided for the
developer's archive-backed inspection test.

## 1. Archive and member identities

| Title | archive | version | files | kit-family members | payload magic (all kit members) |
|---|---|---|---|---|---|
| PS3 NHL 12 | `nocacherender.big` | `0x00000400` | 9 710 | jersey 2 010 | `chunkzip` 2 010 / 2 010 |
| PS3 NHL 12 | `cacherender.big` | `0x00000400` | 12 141 | pant 996, sock 2 016 | `chunkzip` 3 012 / 3 012 |
| PS3 NHL 13 | `nocacherender.big` | `0x00000400` | 10 173 | jersey 1 782 | `chunkzip` 1 782 / 1 782 |
| PS3 NHL 13 | `cacherender.big` | `0x00000400` | 10 969 | pant 882, sock 900 | `chunkzip` 1 782 / 1 782 |

- **No kit member path appears in both archives, and none appears twice in one** (0 / 0 on both titles): there are no override
  candidates to resolve, and archive precedence is therefore not a question this data raises. (Stated, not inferred: if a future title
  shipped duplicates, precedence would need a runtime contract.)
- Keyed triples (style, teamartid, variant): NHL 12 jersey 967, pant 979, sock **973 complete pairs** (`_0_` + `_1_`); NHL 13 jersey /
  pant / sock 865 each — identical to the pm R56 archive listing and to the X360 loose trees (NHL 13 X360: 864 each; the one extra PS3
  key is in the spare pool).
- Sibling files not keyed by the triple grammar (`_am` / `_nm` maps, style-level stems): NHL 12 jersey 1 043 / pant 17 / sock 70; NHL 13
  jersey 917 / pant 17 / sock 35 — listed, not part of the kit contract.

## 2. Join and reconciliation with receipts 03 / 04

| Quantity | research (this handoff) | developer receipt 03 / 04 | status |
|---|---|---|---|
| stock rows NHL 12 / 13 | 676 / 684 | 676 / 684 | equal |
| complete / incomplete / unknown | 676 / 0 / 0 and 684 / 0 / 0 | same | equal |
| components from archives | 2 704 (676 jersey from `nocacherender`, 2 028 pant + socks from `cacherender`) / 2 052 (684 + 1 368) | 2 704 / 2 052 | equal |
| NHL 13 family split | jersey 1 782 `nocacherender`; pant 882 / sock 900 `cacherender` | 1 782 / 882 / 900 | equal |
| sock components NHL 12 | 1 352 (both files per kit) | 1 352 | equal |
| spare candidates, **developer's rule** (per team with rows: triples under the team's art id no row of that team uses) | **255 / 22** and **208 / 1** (recomputed here under that rule) | 255 / 22 and 208 / 1 | equal |
| spare candidates, **global rule** (any keyed triple no stock row uses) | 289 / 21 and 184 / 2 | — | different rule; both published, labelled |
| payload wrapper | `chunkzip` on 100 % of kit members | receipt 04 §1 says "`chunklzx` payloads"; §5 names the missing `chunklzx` encoder | **wording discrepancy**: the members are `chunkzip`; Studio's `decode_payload` handles `chunkzip` / `chunklzx` / raw, so the preview route is unaffected; the §5 limitation as worded does not describe these members (no write is proposed here) |
| member byte identity with X360 | not applicable (`.rpsgl` vs `.rx2`; R74 O-6) | not claimed | consistent |

The global-rule spare pool is larger on NHL 12 (34 triples sit under art ids no team with rows owns) and smaller on NHL 13 (the
per-team rule counts a triple once per team sharing an art id). Neither is wrong; a Team page needs the per-team rule, an inventory
needs the global one, and both should be labelled.

## 3. Fixture / probe contract for archive-backed inspection (developer input; not Codex-accepted)

For each title the JSON's `fixture_contract` names two kits — stock row 0 (`teamid` 0, key (0, 0, 0)) and the first complete spare —
with, per component: `archive`, `member_path`, table `index`, `offset`, `stored_size`, `path_hash`, `magic` (`chunkzip`) and the
SHA-256 of the **stored** bytes. Example (NHL 13, stock row 0): `nocacherender.big:rendering/jersey/texlib_0_0_0.rpsgl` (index 915,
1 255 445 bytes, `5d4a7bf8…`), `cacherender.big:rendering/pant/texlib_0_0_0.rpsgl` (index 6 212, 244 058 bytes, `d76103df…`),
`cacherender.big:rendering/sock/sock_0_0_0_cm.rpsgl` (index 9 057, 24 740 bytes). An inspection test can assert: the member is found by
exact path in the named archive at the named index; its stored size and hash match; its magic is `chunkzip`; NHL 12 rows carry both
sock members. Names identify members; they say nothing about payload compatibility — that stays with the decode route.

## 4. Residuals (exact)

None for the join itself (no row without a component, no unknown archive). Open by construction: which of two sock files the runtime
uses where (Q-R74-1); no runtime verification that the console draws a listed member; no claim about the sibling `_am` / `_nm` maps'
role; no write route.

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-22-pm/probes/r76_ps3_archive_kit_join.py docs/research/overnight-2026-09-22-pm/data/r76-ps3-archive-kit-join.json   (≈ 1 min)
```
Executed once, plus one inline recomputation of the spare pool under the developer's per-team rule (recorded in §2). Inputs: the two PS3
anchors' `nhlng.db` + meta, the archives' front tables, and the eight fixture members' stored bytes.

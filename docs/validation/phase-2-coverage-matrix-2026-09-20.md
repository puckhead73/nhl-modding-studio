# Phase 2 (Teams) — provisional operation-by-target coverage matrix

Opened 20 September 2026 under [OVERNIGHT-DEV-20260920-R1](overnight-dev-20260920-r1-corrections.md)
F3 as an **accounting** of the Phase 2 work the 20 September campaign already delivered
(receipts [07](../receipts/overnight-development-2026-09-20/07-c8-team-identity-colours.md),
[08](../receipts/overnight-development-2026-09-20/08-c9-team-logo-families.md),
[09](../receipts/overnight-development-2026-09-20/09-c10-big4-binder-and-ps4-team-text.md)) plus the
pre-existing team surfaces. It is **provisional**: the R1 review did not accept the campaign in full,
so **no cell here is independently accepted**; opening the matrix is not permission to expand
Phase 2 and implies nothing about Phase 1 being closed (see the Phase 1 matrix's remaining-work
table). The [Phase 1 matrix](phase-1-coverage-matrix-2026-09-18.md) vocabulary applies, with one
addition that this matrix keeps explicit in every cell:

| Evidence class (second word of every cell) | Meaning |
|---|---|
| **dev** | developer-verified only: a named automated case, run by the developer lane, asserted it |
| **listing** | a *listing / capability / identity* case only (a table exists, a family is enumerated, a hash matches) — this is **not** edit / save / restore parity and is recorded as such |
| **by identity** | not executed on that family; carried over because the files are the same by recorded identity (Recompiled = Legacy Xbox 360) |
| **acc** | independently accepted by Codex — **no Phase 2 cell carries this today** |
| **game** | game-verified with a recorded run — **none** |

Target families as in the Phase 1 matrix; where a receipt split an era the split is kept.
"Runtime" = which screen consumes the value; every runtime question is a user card, never a cell.

## 2.1 Identity data

| Operation | X360 07 | X360 08–10 | X360 11–15 / Legacy | PS3 08–10 | PS3 11–15 / Legacy | PS4 15 | PS4 16 | PS4 17–21 | Recomp |
|---|---|---|---|---|---|---|---|---|---|
| Team names (full / abbr / short / art abbr) through the field profile | impl+tested, dev (stock/custom row: `field_profile.rs::nhl07_team_name_cell_targets_the_editable_stock_row`) | impl+tested, dev (`profile_generalizes_across_anchors`, `every_anchor_exposes_only_addressable_profile_fields`) | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev (+ nickname) | impl+tested, dev (city / team name / nickname) | by identity |
| Team colours (one `set_cells_int` per colour, one undo step) | n/a (evidence) — no colour stems on the NHL 07 row (receipt 07 §3) | n/a (evidence) — no colour fields on the team row, measured (receipt 07 §3) | impl+tested, dev — 2 triples (`field_profile.rs::team_colour_and_name_cells_match_the_measured_per_title_shapes`; `registry.rs` `set_cells_int` seam; `ColorField.test.tsx`) | — (as X360 08–10) | impl+tested, dev — 2 triples | impl+tested, dev — 2 triples | impl+tested, dev — 6 triples | impl+tested, dev — 9 packed words (`packed_colour_word_is_r_g_b_big_endian_order`) | by identity |
| Colour draft bound to its team / project / field (R1 F2) | — | — | impl+tested, dev (`ColorField.test.tsx`, `FieldProfile.test.tsx`: same-colour team/project/field switch, in-flight completion, unmount, Enter+blur) | — | ← | ← | ← | ← | ← |
| `artid` / `index` re-identification with consumer cascade | not impl — cascade contract not accepted (research 2026-09-20 handoff 07 §1: reference census) | ← | ← | ← | ← | ← | ← | ← | ← |
| Runtime: which screen reads which name form / colour (`forceteamcolors`, presentation vs front end) | research unknown → cards T-1 / T-2 | ← | ← | ← | ← | ← | ← | ← | ← |

## 2.2 Additional database entries

Only the **surface capability probe** exists as evidence here (receipt 09 §1: `read_matrix.rs` over
the 27 anchors); it establishes that the tables exist and the surfaces build, **not** that an edit
saves, reloads and restores on those titles. No 2.2 package has been opened.

| Operation | X360 07 | X360 08–10 | X360 11–15 / Legacy | PS3 08–10 | PS3 11–15 / Legacy | PS4 15–21 | Recomp |
|---|---|---|---|---|---|---|---|
| Lines / depth charts, strategies, captaincy, contracts (surfaces build; `Editable` reported) | impl, **listing** | impl, **listing** | impl, **listing** | impl, **listing** | impl, **listing** | impl, **listing** | by identity |
| Draft picks, team jerseys tables | n/a (evidence) — no `qHav` / `xjgw` table on 07–10; whether the era stores the data elsewhere is a research question | n/a (evidence), same | impl, **listing** | n/a (evidence), same | impl, **listing** | impl, **listing** | by identity |
| Team equipment table | n/a (evidence) — absent on NHL 07 | impl, **listing** | impl, **listing** | impl, **listing** | impl, **listing** | impl, **listing** | by identity |
| League membership / move between leagues | see Phase 1 matrix 1.2 (impl+tested there) | ← | ← | ← | ← | ← | ← |
| Roster membership, player-index links, numbers / retired numbers, affiliations / rivals, supported team settings — edit, save, reload, restore parity | not established — no named case per family; **not** covered by the listing probe | ← | ← | ← | ← | ← | ← |

## 2.3 Localization (team text)

| Operation | X360 08–15 / Legacy | X360 07 | PS3 08–15 / Legacy | PS4 15–21 | Recomp |
|---|---|---|---|---|---|
| Team text card: resolve the team's dictionary keys by `artabbr`, edit through the dictionary route (LOC-W1/W2) | impl+tested, dev (`membership.rs::team_text_resolves_by_artabbr_and_edits_through_the_dictionary_route`, gen7 anchors) | blocked — labels not in the dictionary (Phase 1 matrix 1.1/1.3) | impl+tested, dev (by the same case on PS3 anchors) | impl+tested, dev — the NHL 15 committed anchor (`ps4_team_text_resolves_by_artabbr_on_the_committed_anchor`) and, `--ignored`, every extracted PS4 English dictionary base + patch, 14 files (`ps4_team_text_resolves_on_every_install_dictionary`) | by identity |
| Long / short forms agree across menus and gameplay | research unknown → user card | — | research unknown | research unknown | research unknown |

## 2.4 Assets (art-id-keyed logo families)

| Operation | X360 07 | X360 08–11 | X360 12–15 / Legacy | PS3 08–10 | PS3 11 | PS3 12–15 / Legacy | PS4 15 | PS4 16–21 | Recomp |
|---|---|---|---|---|---|---|---|---|---|
| List the team's logo families (census of all 27 sources) | impl+tested, **listing** (4 families) | impl+tested, **listing** (6 / 5) | impl+tested, **listing** (5 / 6) | impl+tested, **listing** | impl+tested, **listing** | impl+tested, **listing** | impl+tested, **listing** (5) | impl+tested, **listing** (5 … 15) | by identity |
| Preview + loose replace, same dimensions | impl+tested, dev (`team_logos.rs`, C9 sweep: 284 members, `--ignored` on the private `teamlogos/` fixtures) | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | by identity |
| Different dimensions | blocked (RW4xb2 descriptor) | blocked (RW4xb2) | impl, test missing — C6 XPR2 64/256-wide path exists for 1:1 families; team packages **not swept** | blocked (RW4ps3) | blocked | impl, test missing — C6 DDS path, not swept on team packages | impl, dev (PNG, no descriptor) | blocked (GNF) | by identity |
| Archive placement (copied-target save + restore) | blocked by policy (NHL 07 `.bh` mirror cascade) | by identity (C4 BIG4 / C3 EB routes proven on league logos only) | impl+tested, dev — **real Legacy `cache.big`** (`t10.big`; [evidence log](evidence/overnight-development-2026-09-20/logs/c9_real_teamlogos.log)); 12–15 by identity | by identity (C4) | loose only (no archive holds the family) | impl+tested, dev — **real PS3 Legacy `cache.big`**; 12–15 by identity | by identity (C2) | by identity (C2) | by identity |
| Contradictory whole-group / private scopes on one deduplicated block refused (R1 F1) | — | — | impl+tested, dev (engine `stage::tests`, domain `asset_member_undo.rs`, API `asset_member_route.rs`) | — | — | ← | ← | ← | ← |
| Which screen draws which family; rebrand after restart | research unknown → card T-3 | ← | ← | ← | ← | ← | ← | ← | ← |

## 2.5 Arena details and ice

| Operation | X360 (all) | PS3 (all) | PS4 15–21 | Recomp |
|---|---|---|---|---|
| Arena DB identity and team → arena assignment (profile `arena` cell, arena views) | impl+tested, dev — reads and reference checks (`view.rs::team_views_resolve_identity_and_arena`, `arena_views_resolve`, `read_matrix.rs` arena reference checks, `registry.rs::teams_and_arenas_through_registry`); an **assignment edit with save / reload / restore per family is not a named case** | same | same | by identity |
| Shared-arena users disclosed on an assignment | not established — no named case | ← | ← | ← |
| Centre-ice marks, rink / ice textures (`rendering/*` members) | blocked — gen7 `rendering/*` members are `chunklzx`: framing pinned by research (2026-09-18 handoff 40, 50 members byte-exact by the research probe), Studio's `decompress_chunklzx` **not yet corrected to it** (public pass-through still reports unsupported; oracle test `eb.rs::chunklzx_decodes_against_extracted_oracle` is `#[ignore]` and mount-gated); **no encoder**; loader tolerance of a `chunkzip` member in place of `chunklzx` is **unknown** (research 2026-09-19 handoff 02 §8, card R2-LZX-1 — receipt 10 §9's "boot-write probes crashed on raw substitution" has no recorded evidence, see the R1 corrections record §4) | blocked — same members, same codec state | blocked — the PS4 binding is **research-parsed, not implemented**: typed vault links (arena `ArenaID` packs, centre / spotlight / BAP logos by `Team`, jersey triples → `ge_texture` slug; research 2026-09-19 handoff 07, 2026-09-20 handoff 11) exist as research probe output (`vlt_collections.py`), with no Studio reader, no writer and no complete runtime selector (`IsTopScorer`, override groups, GameMode names unresolved) | by identity |
| Venue parameters beyond identity | research unknown — not inventoried in Studio terms | ← | ← | ← |

## 2.6 Jerseys, pants and kits

| Operation | X360 / PS3 11–15 / Legacy | X360 / PS3 07–10 | PS4 15–21 | Recomp |
|---|---|---|---|---|
| Jersey / kit tables (existing jersey surfaces; full-kit preview pipeline) | impl, **listing** (surface probe; kit preview pre-dates this campaign and is not a Phase 2 case) | n/a (evidence) — no `xjgw` table on 07–10 (receipt 09 §1) | impl, **listing** | by identity |
| Kit entries: ordering / selection / components / fonts / numbers / nameplates / recolouring — edit + save + restore | not impl — research claim `r8-jersey-triple-contract` (2026-09-20 handoff 01) delivered the static join; **not entered** by the developer lane | not impl | not impl — the two-pack discriminator is a vault attribute (`IsTopScorer`), not a DB field | not impl |

## 2.7 Audio

| Operation | X360 (all) | PS3 (all) | PS4 (all) | Recomp |
|---|---|---|---|---|
| Goal horns, team PBP / PA calls, arena music | blocked (format) — the SND10 bank **index is decoded by research** (2026-09-20 handoff 02: `SBKR` / `ABKC` on gen7) and the gen7 music box (handoff 10); **no Studio reader** consumes it; sample encoders (EALayer3 / XMA) absent; DB `goalhorn` → hornbank id open; event → sentence link only name-joined (handoff 13, R19/R20 review F3) | blocked (format), same | blocked (format) — `SBle` index decoded, NHL 21 sentence → sample chain complete to the sample (handoff 02); team / arena / chant audio ids are `pbp_keywords.bin` keyword ids (handoff 07 §3: reassignment would be a DB edit — **not implemented**); EA-MP3 / Opus-shaped encoders absent | blocked (format) |

## What this matrix does not say

- Nothing is accepted: every `dev` cell awaits Codex review; every `listing` cell is explicitly
  **not** parity evidence.
- Phase 1 is still open (its matrix's remaining-work table and the R1 corrections record §4).
- The Phase 2 acceptance foci of the roadmap ("consistent across roster and selection views",
  "resolves correctly … after restart", "correct venue and ice appear") are runtime statements;
  no cell reaches them, and the user cards T-1 … T-3 are where they would be observed.

## Addendum — 21 September 2026 overnight developer campaign (developer-verified; nothing accepted)

Rows changed by the receipts under
[docs/receipts/overnight-development-2026-09-21/](../receipts/overnight-development-2026-09-21/);
the cells above stand as the 20 September accounting and are superseded only where this table
says so. The vocabulary is the one at the top of this matrix.

| Row | Was | Now (21 Sep) | Receipt |
|---|---|---|---|
| 2.1 `artid` / `index` re-identification | not impl — cascade contract not accepted | **impl+tested, dev** — index renumbering **refused** with per-table consumer counts on every family; art id changes only by a **swap** between two existing teams as one multi-table undoable step (jersey / award rows follow, roster-mirrored), refused for spare-kit ids and shared ids; bare art-id cell edits refused at the write layer (`workspace.rs::artid_swap_cascades…` NHL 14 PS3; `team_reid.rs` Legacy X360 + NHL 21 PS4). Contract = research 2026-09-21 handoff 10 (a **proposal**, consumed at the risk of rework). Runtime: card T-4 | 06 |
| 2.5 centre-ice marks, rink / ice textures (gen7 `rendering/*` `chunklzx` members) | blocked — decoder not corrected, no encoder | **read: impl+tested, dev** — the decoder is promoted (50 / 50 research vectors byte-exact on the real archives; the BIGF reader decodes too). **Write: impl+tested, dev, loader acceptance NOT established** — the store route (LZX uncompressed blocks in the same container; encoder = the research's sizes exactly on 16 synthetic + 50 real members) through the EB member route on a synthetic archive around a real package; card **R30-LZX-1** is the discriminating runtime test; the `chunkzip` substitution (R2-LZX-1) stays the fallback candidate | 04, 08 |
| 2.5 PS4 rink / ice binding | blocked — research-parsed, not implemented | unchanged | — |
| 2.7 team PBP / PA name calls (gen7) | blocked (format) — no Studio reader | **read-only listing: impl+tested, dev** — Team page "Name calls": the `team_ID` graffiti tables of the team's audio id → sample ids → bank resolution, KWD0 mention count on 15 / Legacy; research counts 207 / 211 (NHL 15) and 206 / 212 (Legacy) reproduced on both platforms' anchors. Titles outside the measured coverage (07–10, PS3 12 / 13, PS4) refuse with the reason. Decoding / playing / writing: still blocked (format) | 05, 07 |
| 2.7 league / cup commentary catalogue (gen7) | — (no row) | **read-only listing: impl+tested, dev** — Leagues → Commentary (events, sentences sharing an event's name, phrases → candidate samples → bank); trace figures exact on NHL 15 / Legacy through the API (new crate `tdb-speech`, research model reproduced on 10 loose trees) | 05 |
| 2.2 / 2.3 group labels (PS4 pseudo-leagues 77 / 78) | "Unnamed group 77 / 78" | **impl+tested, dev** — labelled by rule from the leagues that carry them (`derived-from-leagues`), evidence class unchanged | 09 |

Read-side defects fixed that touch team art: Xbox 360 texlib heights above 2 048 or not a
multiple of 8 were misread (13-bit `DW9` read); PS3 DXT1 / DXT3 rasters (14 000 of 26 000 on
Legacy) were unreadable — both under receipt 11.

### Correction R1 — 21 September 2026 (coordinator review R1, F2; receipt 16)

The review reproduced a failed art-id swap from one project displaying its error on another
project's card for the same team id (`TeamIdentityCard` `onError` compared the team only). Row 2.1's
domain / API claims above are unchanged; the **UI cell** is corrected: every completion path of the
swap — success, failure, the owning project's summary refresh and its failure — now decides by the
immutable dispatch context (project, team, partner, card generation) carried as the mutation's
variables, and "pending" belongs to the dispatching card only. `TeamIdentityCard.test.tsx`: 10 cases
(cross-project same-team failure — the reviewer's reproduction — cross-team failure, away-and-back
success and failure, unmount, summary-refresh failure on the owning card and after a project move,
exact dispatched arguments, the owning project's refresh, one-step undo wording). Runtime card T-4 is
unchanged (not run).

### Addendum — 22 September 2026 (campaign OVERNIGHT-DEV-2026-09-22)

Rows changed by [receipts 04 and 05](../receipts/overnight-development-2026-09-22/). Nothing here is
Codex-accepted; no cell is game-verified.

| Row | Was | Now (22 Sep) | Receipt |
|---|---|---|---|
| 2.5 venue parameters beyond identity | **not inventoried in Studio terms** | **inventoried: impl+tested, dev** — every field of the open title's arena table listed with a candidate class from `defs/arena/venue-fields.toml` (159 fields, transcribed from research R51), its constancy measured over *that file's* arena rows, the 24-bit colour zones rendered as colours, and unclassified fields refused for editing with the reason on the row. Tested on four era signatures (PS3 NHL 07 gen6 thin table, PS3 NHL 08 rink-dressing set, X360 Legacy gen7 core, PS4 NHL 21 colour zones + horn builder). **No arena field has a traced runtime consumer** and none is claimed — the card says so in its own text | 05 |
| 2.7 commentary contexts (PS4) | not covered — the card was gen7 only | **read-only listing: impl+tested, dev** on **PS4 NHL 15–21** through the little-endian `0XTC` walk (research R58): all seven titles reproduce the research census exactly (records, triggers, speech / context items, conditions) through the registry seam, with each item's name checked against the title's MOIR table. Condition **argument values** and the `.gas` bytecode stay undecoded and are named as such | 04 |

| 2.7 PS4 `goalhorn` → catalogue | raw id ("DB `goalhorn` → hornbank id open") | **→ catalogue closed: impl+tested, dev** — `goalhorns.csv` joined by research R50 §2's measured rule on the real NHL 17 / NHL 21 installs (150 and 46 club horns, matching the handoff), with the missing New York Rangers row and the 690–693 sentinels as **named refusals**; the rule is on the row behind disclosure. `goalhorn` → **sound** stays open (R50 §3); `periodhorn` is deliberately not joined | 07 |
| 3.4 player head pool classification (X360 / PS3 **NHL 09–13**) | pool range unrecognised — Studio's fixed 60 000 floor found no pool heads | **impl+tested, dev** — the floor is measured from the open file (60 000 when present, else 10 000, per research R53 §1); pool populations on five anchors match the research, NHL 15 (5 275) and Legacy (5 385) exactly | 06 |

| 2.6 gen7 kit entries: ordering / selection / components | rows decoded, **no join to the textures** | **read-only join + Uniforms card: impl+tested, dev** — `(style, team art id, variant)` → jersey / pant / sock under a **title / platform contract** (NHL 11 / 12 two sock files per kit, NHL 13 → Legacy one; NHL 09 / 10 no selector table; PS4 and 07 / 08 unmeasured), every required component listed with present / missing / unknown evidence and **one completeness rule** for rows and spare candidates (coordinator F1 corrected: an absent `sock/` directory yields *incomplete*, never complete). Resolves **100 % of rows** on ten real trees — X360 NHL 11 / 12 / 13 / 14 / 15 / Legacy (627 / 676 / 684 / 704 / 659 / 703) and PS3 NHL 11 / 14 / 15 / Legacy (`.rpsgl`) — reproducing research R56; spare candidates listed with their own verdict (complete 97 / 255 / 207 / 315 / 54 / 228, plus 20 / 22 / 1 / 2 / 0 / 58 incomplete). PS3 NHL 12 / 13 (in-archive kits) **not covered**. Custom-team rows and art-id mismatches refused with their reasons. Uniforms tab shows the card (contract, tree evidence, the selected kit's components with thumbnails, spare summary, why re-pointing is not offered). **No writer**: reassignment unsupported, and on NHL 11 / 12 the sock 0 / 1 sub-index meaning is unmeasured | 08, **10** |

These 2.x rows are inspection surfaces: none opens a write path, so none adds a runtime card. The
3.4 row corrects a classification, not a write path.

### R3 — 22 September 2026 (fresh-session dispatch; receipts [10](../receipts/overnight-development-2026-09-22/10-r3-kit-completeness-correction-and-uniforms.md), [12](../receipts/overnight-development-2026-09-22/12-r3-arena-assets-card.md); developer-verified, nothing accepted)

| Row | Before | After | Receipt |
|---|---|---|---|
| 2.5 venue model / jumbotron (gen7 `rendering/stadium`, `rendering/jumbotron`) | not joined to the arena row | **read-only join: impl+tested, dev** — `arenaartid` names the venue model on 100 % of rows of X360 NHL 09 / 11 / 13 / 15 / Legacy and PS3 Legacy, and the jumbotron on 12+; spare venues per title equal research R62's residuals (4 / 10 / 10 / 12 / 12; jumbotrons 12 / 14 / 14); Arena page card | 12 |
| 2.5 centre-ice logo / banners on the arena | team surface only | shown on the arena **through the home team** with the provenance stated; NHL 09 / 11 state that the team table has no home-arena link | 12 |
| 2.5 bounce map / other ice layers | — | bounce map a **candidate** (100 % of rows on 13 / 15 / Legacy name a file); centre-line / line art / lightmap / hotspots / named ice bases **no established binding** — labelled, never promoted | 12 |
| 2.5 PS4 rink / ice binding | blocked — research-parsed, not implemented | **listing, dev** — venue directories and `stadiums/common/` families as shipped evidence under a *vault reference Studio does not read*; the vault reader stays open | 12 |
| 2.6 gen7 kits | see the row above (22 Sep addendum) | corrected and surfaced — receipt 10 | 10 |
| 2.4 different-dimension replacement — gen7 team-logo packages | impl, test missing — team packages **not swept**; 07–11 blocked (RW4xb2 / RW4ps3) | **swept: 168 packages** ([receipt 13](../receipts/overnight-development-2026-09-22/13-r3-team-logo-dimension-sweep.md)) — X360 07–11 and PS3 08–11 **refuse by name** ("same-size only", the package forms); X360 12–Legacy and PS3 12–Legacy (+ Legacy cache) **impl+tested, dev**: small / wide / reflection resize on every package, teamlogos and dynasty on the packages whose Apt quad is the texture's own size (9 / 14, 5 / 10 per console), calendar refuses as a display crop everywhere; every success keeps every other member byte-identical and the Apt size words follow | 13 |
| 2.7 team songs — PS4 17–21 (`goalsongid` / `winsongid` / `powerplaysongid` / `introsongid`) | cells raw; no listing of the selectable set | **read-only catalogue listing: impl+tested, dev** ([receipt 16](../receipts/overnight-development-2026-09-22/16-r3-song-catalogue.md)) — `songs.csv` + `songgenres.csv` on the real NHL 17 / 18 / 19 / 21 installs (191 / 245 / 637 / 422 rows, R54 §1); the cells stay raw with "no catalogue join" (R54 §2); NHL 15 / 16 and gen7 refuse with the reason; no preview, no writer | 16 |


### R4 — 23 September 2026 (coordinator review of receipts 10–17, items 2 / 3; receipts [02](../receipts/overnight-development-2026-09-23/02-r4-title-text-keys.md), [03](../receipts/overnight-development-2026-09-23/03-r4-ps3-archive-kits.md); developer-verified, nothing accepted)

| Row | Before | After | Receipt |
|---|---|---|---|
| 2.7 PS4 song catalogue — title / genre keys | keys shown raw; the card prettified genres by parsing the key | **resolved through the open project's own attached dictionary: impl+tested, dev** — `exact` / `by-hash` / `absent` / `no-dictionary` per key, the raw key kept and shown when unresolved, the reply stamped with the dictionary's locale / sha / rows; seam test across attach, detach, re-attach, project switch (NHL 17 / 18 / 15 anchor dictionaries); no sibling dictionary is ever consulted | 02 |
| 2.5 PS4 `goalhorn` → catalogue → name | `$NHLTeamName_ANA (AnaheimDucks)` | the club's text through the project's own dictionary, the key and its lookup state disclosed; unresolved keys stay keys | 02 |
| 2.6 gen7 kits — PS3 NHL 12 / 13 | not covered (kits in `cacherender.big` / `nocacherender.big`) | **read-only join from the archives' file tables: impl+tested, dev** on the real trees (676 / 684 rows, 100 % complete under the accepted contract rule; 2 704 / 2 052 components from the archives; both sock files on NHL 12); presence is listing evidence — members are not decoded or previewed; unlistable archive → unknown, never absent | 03 |
| 2.6 gen7 kits — PS3 NHL 12 / 13 component preview | present by archive listing only (receipt 03) | **archive-member preview: impl+tested, dev** — `archive_member_texture_preview` (front tables + one member, `chunklzx` decoded); the real NHL 13 jersey member decodes at 1024²; the kit card shows the thumbnail; no archive write | [04](../receipts/overnight-development-2026-09-23/04-r4-archive-member-preview.md) |
| 2.5 PS4 venue binding (NHL 15–21) | venue directories listed as shipped evidence under a vault reference Studio does not read (R3 row above) | **read-only listing by vault level name: impl+tested, dev** — research R73's join as a developer-owned table (`defs/arena/ps4-venue-levels.json`); every level carrying the arena's `arenaartid` with live folder status, texpack keys resolved into the measured sets, rink id; classes unambiguous / ambiguous / level-without-folder / no-level / unmeasured; ambiguities named (Honda Center id 0; NHL 16 Barclays / Rogers) and never resolved silently; shared content once; **no reassignment** (R68's write proposal not implemented; cascade stated) | [06](../receipts/overnight-development-2026-09-23/06-r4-ps4-venue-listing.md) |
| 2.6 gen7 kits — NHL 11 / 12 sock pair | "what the 0 / 1 sub-index selects is not measured" | research R74: the `_1_` file is the horizontal mirror of `_0_` — one kit, both files; contract text and re-pointing reason updated; no selector affordance | STATUS 23 Sep |

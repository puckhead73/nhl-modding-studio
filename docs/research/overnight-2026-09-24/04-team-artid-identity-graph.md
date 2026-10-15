# Handoff R86 — Team `artid` re-identification (Teams 2.1): the identity space and the complete static dependency graph, measured on 25 anchors — **`artid` is a shared art key, not a row identity** (NHL-club art ids are held by 2–4 rows from NHL 12 on: the Winter Classic / Alumni / Circuit variants of a club; 30 create-a-team rows share one placeholder; the EASHL rows share 255), the DB columns keyed by it are exactly `stockteamjerseys.teamartid` / `customteamjerseys.teamartid` (+ `stockawardstable.teamartid`, `*resort*extras.teamartid` on PS4 19–21), and the asset families keyed by it are the logo / rink-logo / centre-logo / puck / kit families already joined by earlier handoffs — a narrow refuse-or-cascade contract follows, with every unknown named

Research lane, 24 September 2026, 05:41 → 05:44 MDT (clock corrected against file times) (brief item R4, Team scope; Q: 2026-09-20 handoff 07 §1's "contract input").
Claim `research-r86-team-artid-graph`. Read-only over `anchors/` and this lane's earlier JSONs. Nothing game-verified; no writer built.

Machine-readable: [`data/r86-team-artid-graph.json`](data/r86-team-artid-graph.json) — per anchor the identity statistics of
`exhibitionteams` / `leagueteams` / `customteams`, every `*artid*` column with its coverage by the team art-id set, and the free-id
counts. Probe `r86_team_artid_graph.py` (≈ 1 min, 25 anchors: X360 08–Legacy, PS3 08–Legacy, PS4 15–21; NHL 07's gen6 schema skipped).

## Plain-language summary

A team's "art id" is the number that names its logo, rink art, puck and kits on disk and in the jersey tables. Measured on every
shipped database, that number is not unique per team row: from NHL 12 on, four NHL clubs (Rangers, Flyers, Penguins, Capitals) have a
second row — their Winter Classic team — with the same art id, the thirty blank "create a team" rows all share one placeholder id, the
two EASHL rows share another, and on the PlayStation 4 titles every NHL club's Alumni row (and the Circuit rows on NHL 21) reuses the
club's id. So re-numbering the art id of one row would either split a club from its variants or leave them pointing at art that no
longer exists. The safe operation is therefore defined on the art id, not the row: move every row and every keyed file and table
entry from id a to a free id b, or refuse. The table below lists exactly what "every keyed file and table entry" means per title
family, what has been measured, and what has not (runtime caches and user saves).

## 1. Identity space (measured; `exhibitionteams`)

| Anchors | rows | `artid` field | distinct | duplicates (rows sharing) | free ids in range | `index` unique |
|---|---|---|---|---|---|---|
| X360 / PS3 NHL 08 | 138 | 8-bit, 0–255 | 108 | **0 × 31** (31 rows with no art) | 148 | yes |
| X360 / PS3 NHL 09 | 188 | 15-bit, 0–32 767 | 158 | 0 × 31 | 32 610 | yes |
| X360 / PS3 NHL 10 | 182 | 15-bit | 153 | **20 054 × 30** (create-a-team rows, league 13, blank names) | 32 615 | yes |
| X360 / PS3 NHL 11 | 242 | 15-bit | 213 | 20 054 × 30 | 32 555 | yes |
| X360 / PS3 NHL 12 | 243 | 15-bit | 212 | 23, 29 × 2 (Winter Classic rows, league 14); 20 054 × 30 | 32 556 | yes |
| X360 / PS3 NHL 13 | 248 | 15-bit | 213 | 19, 21, 23, 29 × 2; **505 × 3** (Legends / Ruggiero / Wickenheiser, league 12); 20 054 × 30 | 32 555 | yes |
| X360 / PS3 NHL 14 / 15, PS4 15 | 246 | 15-bit | 213 | 19, 21, 23, 29 × 2; 20 054 × 30 | 32 555 | yes |
| X360 / PS3 Legacy, PS4 16 | 252 | 15-bit | 218 | same + **255 × 2** (EASHL 1 / 2, league 15) | 32 550 | yes |
| PS4 17 | 256 | 15-bit | 256 | none | 32 512 | yes |
| PS4 18 | 296 | 15-bit | 289 | 0 × 7; 36 × 2 | 32 479 | yes |
| PS4 19 / 20 | 419 | 15-bit | 307 | **0–29 × 2–4** (club + Alumni league 21 [+ others]); 345 × 10 / 346 × 11 / 347 × 2; 36 × 2; 500 / 501 × 2; **600 × 40** | 32 461 | yes |
| PS4 21 | 433 | 15-bit | 312 | 0–29 × 2–3 (club + Alumni; Anaheim also the Circuit 7 row, league 23); 345 × 35; 346 × 7; 600 × 45 | 32 456 | yes |

The PS4 8-bit-looking values (255 on Legacy / 16) are real ids inside a 15-bit field; 20 054 is a placeholder shared by the create-a-team
block on every gen7 title from NHL 10 (blank names, `league` 13); `exhibitionteams.index` is unique on 25 / 25. `coachartid` (NHL 08 / 09)
is a coach art id, not a team key (13 / 15 values outside the team set on 09).

## 2. DB columns keyed by the team art id (measured by coverage of non-zero values; everything else with "artid" in its name is a player / arena / league art id and is listed apart in the JSON)

| Column | Titles | coverage by the team art-id set | values outside |
|---|---|---|---|
| `stockteamjerseys.teamartid` | X360 / PS3 11–Legacy, PS4 15–21 | 1.000 on gen7 / PS4 15–16; 0.949–0.989 on PS4 17–21 | PS4: the spare-kit ids (255, 1000–1004 — 2026-09-20 handoff 01) |
| `customteamjerseys.teamartid` | same | 1.000 on gen7 / PS4 15–16; 0.0 on PS4 17–21 (1–4 values: the custom-team art block) | — |
| `leaguecustomteamjerseys.teamartid`, `leagueteamjerseys.teamartid` | same | empty tables on the anchors (mode copies) | — |
| `stockawardstable.teamartid` | PS4 21 | 0.994 (1 value outside) | — |
| `exhibitionresortgoalieextras.teamartid`, `exhibitionresortskaterextras.teamartid` | PS4 19–21 | 0.936 (2 outside) | — |

`exhibitionteams.index`-keyed columns (roster, bio, equipment, retired numbers, awards, affiliates, rivals, schedule slots) are those of
the 2026-09-20 handoff 07 §1 and are untouched by an art-id change.

## 3. Asset families keyed by the team art id (from the earlier joins; per family the witness title, coverage and the source handoff)

| Family (path grammar) | Titles | measured coverage (Legacy X360 unless noted) | where |
|---|---|---|---|
| FE crest `fe/ion/artassets/teamlogos/t{artid}.big`, small `s{artid}` | X360 / PS3 08–Legacy | 217 / 218 rows matched, 101 asset-only ids (unreferenced art) | 2026-09-18 handoff 20 (`nhllegacy-team-asset-joins.json`); developer `team_logos.rs` census on 27 sources |
| rink logo `rendering/logo/texlib_{artid}_a_b.rx2` | X360 / PS3 08–Legacy | 218 / 218; 130 asset-only | same |
| centre-ice logo `centerlogo_{teamartid}` | gen7 | 196 / 218 matched, 22 rows without, 35 asset-only | same |
| puck `puck_{teamartid}` | gen7 | 217 / 218; 32 asset-only | same |
| kits `rendering/{jersey,pant,sock}/texlib_{style}_{teamartid}_{variant}` (+ FE jersey, nameplate `rendering_name_…`) | X360 / PS3 11–Legacy (PS3 12 / 13 by archive listing) | 698 / 698 kit rows matched on Legacy; per title in R56 / R76 | 2026-09-21 pm handoff 14, 2026-09-22 pm handoff 08 |
| PS4 vault packs keyed by `Team`: `centerlogo` texpacks (`Team`), jersey texpacks `(Team, UniformVariation, JerseyFit)`, spotlight / BAP logos | PS4 15–21 | `team.artid_vs_centerlogo.CenterLogo` and jersey pair joins in `r7-ps4-vault-collections.json` | 2026-09-19 handoff 07 |
| PS4 logo `.png` / GNF families by art id | PS4 15–21 | developer census (27 sources) | Phase 2 matrix 2.4 |
| localization keys | all | keyed by `artabbr`, **not** by art id — unaffected | 2026-09-18 handoff 23 |

## 4. Contract (proposal; researcher-verified structure, not Codex-accepted, nothing runtime)

**Operation: re-key team art id a → b on one title.**

- Applicability: a is held by ≥ 1 `exhibitionteams` row of the open database; b is inside the field's declared range and held by no
  row (§1: 32 000+ free ids on every 15-bit title; 148 on NHL 08); b ≠ 0 and b ∉ {20 054, 255 on Legacy / 16, 1000–1004 on PS4}
  (placeholder / spare ids — refuse); a ≠ 20 054 (the create-a-team block — refuse; those rows have no art).
- Lookup identity: **every** `exhibitionteams` row with `artid == a` (1–45 rows; the UI must show the set — club + Winter Classic /
  Alumni / Circuit variants — and never re-key one row of a shared id silently); the `leagueteams` / `customteams` copies where present.
- Allowed changes: `exhibitionteams.artid` (all rows of a), the §2 columns' rows equal to a, and — per family present on the title — a
  rename or copy of the §3 files from a to b (loose) or the copied-target archive route the developer already has for members; PS4 vault
  `Team` attributes are **not** rewritable (no vault writer; R73 / R7 read-only) → on PS4 the operation is **refused** unless b's packs
  already exist, i.e. it is not implementable on PS4 today.
- Byte / bit preservation: DB cells only, through the existing field route (one undo step per table); no other cell touched; file
  renames preserve bytes (hash equality before / after); archive members through the existing span-preserving stager only.
- Shared-resource scope: a file family shared by the rows of a moves with them (that is the point of keying on a); a file a's rows share
  with rows of another id cannot exist by construction (families are keyed by a single art id) — except the `centerlogo_{centericelogo}`
  arena-side key (arena-keyed, untouched) and the PS4 spotlight / BAP packs (unmeasured sharing → refuse on PS4, as above).
- Repeat edits: idempotent per (title, a → b); a second a → c after a → b finds no rows with a → refuse with "no row holds a".
- Refusal conditions: b held; b a placeholder / spare; a = placeholder; any §3 family present on the title whose files for a cannot be
  enumerated (archive unlistable, member undecodable) → refuse the whole operation (no partial cascade); PS4 (vault) → refuse.
- Round-trip / restore: the reverse operation b → a on the same files and rows; backups of every renamed file and archive; DB undo.
- **Unknown (must be disclosed, not inferred):** what the runtime caches by art id (Xbox 360 / PS3 render caches, `cache.big` naming);
  user saves that mirror the team table (roster saves carry `exhibitionteams` copies — the operation does not touch saves, so a save
  loaded afterwards may reference a, and what the game then does is untested); PS4 vault selection; any code-side table of NHL club
  ids 0–29 (the `proteam` / `draftteam` third id space of the 2026-09-20 handoff 07 is index-based, not art-id-based, but a code table
  keyed by art id 0–29 cannot be excluded statically — cards T-ARTID-1).

**Not proposed:** creating a team row (allocation of `index`, roster / bio / schedule references — a separate contract), deleting one,
or re-keying `index`.

## 5. Card (deferred)

- **T-ARTID-1** — on X360 NHL 15 (loose tree), re-key one non-NHL club with a single row and a complete file set (e.g. an AHL club) to a
  free id, moving its `t{artid}` / `s{artid}` / `texlib_{artid}_*` / puck / kit files; observe the FE crest, rink art and kits in an
  exhibition game; control: an untouched club; restore from backups. Outcome: art follows the id → the static graph is complete for that
  family set on that title; art missing → a family or cache not in §3.

```
python local/runs/overnight-research-2026-09-24/probes/r86_team_artid_graph.py docs/research/overnight-2026-09-24/data/r86-team-artid-graph.json
```
Executed once; the duplicate-row listing (§1 examples) was an inline query on the Legacy X360, NHL 13 X360 and PS4 NHL 21 anchors.

> **Correction note 13:25 MDT (R88, [handoff 06](06-correction-checkpoint-campaign-review.md) §F1 / §F3):** (1) The "PS4 19 / 20" row of §1
> and the PS4 19–21 columns of §2 were measured on `anchors/ps4/nhl20`, which is the NHL 19 base copy; the real NHL 20 install
> (`1e200478…`) has 416 rows, 304 distinct art ids, 42 shared ids (adds 36 × 4, 39 × 4, 48 / 49 × 2, 50 × 3, 53 × 2) and
> `stockteamjerseys.teamartid` coverage 0.958 (7 outside) — handoff 06 §F1; every input hash is in `data/r88-correction-checkpoint.json`.
> (2) §4 is **downgraded from a proposal to an unfinished contract**: destination occupancy must be checked across every `*artid*`
> column, every loose family, every archive member and internal reference (on Legacy X360, 156 of the "free" ids already carry
> asset-only files; default refuse); the sentence "cannot exist by construction" is withdrawn (shared spans / cross-key references /
> PS4 `word_12` unmeasured; the member stager is not a rename route); "one undo per table" and a reverse b → a are not a restore
> contract (one logical operation with journal, restart recovery and hash-verified baseline restoration is required); §3 is a compiled,
> not exhaustive, cross-title list; PS4 stays refused regardless of existing b packs. **Retained as usable now:** the read-only
> shared-art-id row-set inspection (§1) and the keyed-column listing (§2). Card T-ARTID-1 narrowed (USER-CARDS).

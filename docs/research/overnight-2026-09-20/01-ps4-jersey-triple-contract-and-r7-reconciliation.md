# Handoff R8 — the durable three-part jersey join on PS4 15–21, the duplicate-pack discriminator, and the R7 evidence contract closed

Research lane, 20 September 2026 (Priority 0 of the overnight research campaign). Claim `research-r8-jersey-triple-contract`.
Consumer: Phase 2.6 (jerseys/kits) and the NHL21-E1 successor work; reconciles handoff [07](../overnight-2026-09-19/07-ps4-vault-collections-and-slug-binding.md) §4.
Nothing here is game-verified; no install, archive, vault or anchor was written; no game launched; **no writer is proposed.**

Machine-readable: [`data/r8-jersey-triple-joins.json`](data/r8-jersey-triple-joins.json) (16.7 MB; per title: source identities,
denominators, the full residual on both sides, every multiple-match triple with every candidate collection's typed attributes and
texture paths, every matched single-pack triple with its DB rows and pack, attribute census, discriminator test). Probe:
`local/runs/overnight-research-2026-09-20/probes/r8_jersey_triples.py` (read-only; imports the R7 reader `vlt_collections.py` and the
R6 DB helpers; ≈ 4 min for all seven titles).

**Evidence vocabulary:** **observed** = read from the named files; **inferred** = a reading whose producing mechanism is not traced;
**unknown** = not established. Static correspondence between two shipped data sets only — nothing about the loader.

## Plain-language summary

Yesterday's handoff 07 said that a jersey in the PS4 database is keyed by three numbers (team art id, variant, style) and that the
texture collection for it carries the same three numbers, but that on NHL 17–21 dozens of those triples had *two* collections and the
rule for choosing between them was unknown; and its durable probe only recorded the two-number join. This handoff makes the three-number
join durable on all seven titles, with every unmatched and every doubly-matched triple listed, and it identifies the discriminator: in
every doubly-matched triple but one, the two collections differ in exactly one typed attribute — an on/off flag whose name hashes to
`IsTopScorer` — and the "on" collection is the `*topscorer*`-named pack (the leading-scorer jersey of the Finnish, Swiss and Champions
Hockey League clubs). So the static key of a jersey texture pack is *four* values, `(Team, UniformVariation, JerseyFit, IsTopScorer)`,
and with the fourth value the join is unique on every title except one Malmö pair on NHL 19. The database does not carry the fourth
value: its `variantleadingscorer` column equals `variant` on every stock row, so which pack the game draws for a leading scorer is a
runtime decision, not a database one. The rev. 2 packaging corrections from the 19 September review are restated at the end for the
developer, unchanged.

## 0. Sources (observed)

Vault, schema and database identities per title are in the JSON `sources` block (full SHA-256). Prefixes:

| Title | Install | `gccollections.vlt` / `.bin` | `gcschema.vlt` / `.bin` | Database |
|---|---|---|---|---|
| 15 | CUSA00561 | `d1ec595c…` / `0684ef97…` | `5ca05db6…` / `0223c45c…` | `anchors/ps4/nhl15/databases/nhl_game.db` `ae9d1d56…` |
| 16 | CUSA01909 | `202035cf…` / `b1217f4a…` | `304d2f87…` / `ac1d8014…` | anchor `7d95f1db…` |
| 17 | CUSA04326 | `df39aeaa…` / `2da87ee8…` | `ad1edb36…` / `31312e80…` | anchor `de48b02b…` |
| 18 | CUSA07569 | `0a5e019d…` / `cf0cfab2…` | `e6232c25…` / `464ae277…` | anchor `65eaa6e8…` |
| 19 | CUSA11126 | `6e537bb7…` / `aece3f7a…` | `96e01d98…` / `1df5e941…` | anchor `265e4a68…` |
| 20 | CUSA15439 | `31a2727f…` / `e5266477…` | `84b89a5c…` / `c102d023…` | real install `inxp_00.big:nhl/data/common/gamemodes/database/t3db/nhl_game.db` `1e200478…` |
| 21 | CUSA18182 | `dd4ceff0…` / `d668bbe1…` | `04d0b43c…` / `772ccd15…` | anchor `90ee1ec4…` |

Same files handoff 07 read (identical prefixes). The DB table is `stockteamjerseys`; the vault class is `ge_nhl_skaterjerseytexturepack`.

## 1. Denominators (observed; exact)

"Row" = one DB record; "pack" = one vault collection record; "triple" = a distinct `(teamartid, variant, style)` / `(Team,
UniformVariation, JerseyFit)` value. Every DB row and every pack on every title has a complete triple (0 incomplete on both sides).

| Title | DB rows | DB distinct triples | DB triples with duplicate rows | Packs | Pack distinct triples | Pack triples with 2+ packs | **Matched triples** | matched with 1 pack | **matched with 2 packs** | DB-only triples | Pack-only triples |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 15 | 659 | 657 | 2 | 1 014 | 1 005 | 4 | **657 / 657** | 657 | 0 | 0 | 348 |
| 16 | 703 | 698 | 3 | 798 | 735 | 63 | **696 / 698** | 641 | 55 | 2 | 39 |
| 17 | 813 | 810 | 3 | 920 | 855 | 65 | **802 / 810** | 749 | 53 | 8 | 53 |
| 18 | 1 030 | 998 | 10 | 1 150 | 1 124 | 26 | **996 / 998** | 972 | 24 | 2 | 128 |
| 19 | 1 532 | 1 091 | 271 | 1 318 | 1 227 | 91 | **1 069 / 1 091** | 980 | 89 | 22 | 158 |
| 20 | 1 652 | 1 151 | 271 | 1 330 | 1 240 | 90 | **1 092 / 1 151** | 1 004 | 88 | 59 | 148 |
| 21 | 1 792 | 1 250 | 280 | 1 464 | 1 374 | 90 | **1 176 / 1 250** | 1 088 | 88 | 74 | 198 |

Duplicate DB rows for one triple (NHL 19–21: 271–280 triples) differ in `teamid`, `orderid`, `iscasual`, `jerseyindex`, `isdefault`,
`locstring`, `variantlegend` (20/21) — several teams (`teamid`) sharing one art id share the kit rows; the join is by art id, so those
duplicates are expected and do not multiply packs. No pack triple has more than two packs on 16–21; NHL 15 has one placeholder triple
`(999, 0, 1)` with seven packs (`jersey_rbk_template_home` + six Kometa Brno / Abbotsford packs parked on `Team` 999) and three
`(284, 2..4, 0)` pairs (Slavia Praha / Litvínov packs sharing art id 284) — none of the four is a DB triple.

**Reconciliation with handoff 07 §4.** The two-part join regenerated here equals handoff 07's row exactly on all seven titles
(565/565, 599/600, 703/711, 837/839, 878/900, 880/937, 955/1 027). The three-part numbers handoff 07 reported for 15/17/18/21 from the
deleted exploratory scripts (657/657; 802/810 · 53; 996/998 · 24; 1 176/1 250 · 88) are reproduced exactly by the durable probe; the
three titles it marked "(not run)" are now 696/698 · 55 (16), 1 069/1 091 · 89 (19), 1 092/1 151 · 88 (20). Nothing in handoff 07 is
contradicted; its "selection rule unknown" is replaced by §2.

## 2. The duplicate-pack discriminator: `IsTopScorer` (observed, with the name hash-resolved)

Every `ge_nhl_skaterjerseytexturepack` record on NHL 16–21 carries, besides `Team`/`UniformVariation`/`JerseyFit`, one inline enum
attribute with key `a0f106bf` of type `EA::AttribSys::Enums::OnOff` and one inline `Int32` with key `bb93351e` (NHL 15: the Int32 key is
`354f452f` and the OnOff attribute is absent). `lookup2("IsTopScorer", 0xABCDEF00) == 0xa0f106bf` — the name is resolved by the same
hash that handoff 07 verified for 81 other attributes (found in a ≈ 25 000-candidate word search this session; the Int32 key did not
resolve).

| Title | Matched triples with 2 packs | pairs where `IsTopScorer` is (0, 1) | pairs where it is (0, 0) | `DebugName` differs | texture path sets differ |
|---|---:|---:|---:|---:|---:|
| 16 | 55 | **55** | 0 | 55 | 24 (the other 31 pairs reference the *same* ten textures) |
| 17 | 53 | **53** | 0 | 53 | 24 |
| 18 | 24 | **24** | 0 | 24 | 24 |
| 19 | 89 | **88** | **1** | 89 | 89 |
| 20 | 88 | **88** | 0 | 88 | 88 |
| 21 | 88 | **88** | 0 | 88 | 88 |

- The `IsTopScorer == 1` pack comes in two authoring shapes. (i) **Named** `*topscorer*` packs: NHL 16/17 Liiga clubs (art ids 82–99;
  31 / 29 pairs, every one of which references the **same ten textures** as its `IsTopScorer 0` twin — the pack differs only by name and
  flag) and NHL 19–21 CHL/Champions clubs (64 pairs each, own `*hometopscorer*`/`*awaytopscorer*` slug sets). (ii) **Unnamed**: the
  twelve Swiss NLA clubs on every title 16–21 (24 pairs each: `jersey_nineties_<club>_home`/`_away` are `IsTopScorer 0`, the paired
  `_two`/`_three` packs are `IsTopScorer 1` with their own textures) — the name carries no `topscorer` token, so **the flag, not the
  name, is the discriminator**. Population of `IsTopScorer == 1` packs: 65 / 65 / 26 / 90 / 90 / 90 on 16–21 (named 39 / 39 / 0 / 64 /
  64 / 64; unnamed Swiss 26 in every title); the ones not in a matched pair sit on pack-only triples (10 / 12 / 2 / 2 / 2 / 2).
- **The one exception:** NHL 19 `(127, 0, 1)` Malmö Redhawks — `jersey_rbk_malmo_home_texpack` and `jersey_rbk_malmo_two_texpack`, both
  `IsTopScorer 0`, different texture sets, one DB row (`isdefault 1`). Selection between these two is **unknown** (a stock authoring
  duplicate; NHL 20/21 carry no such pair).
- `bb93351e` (`354f452f` on 15) is the constant `-312571348` (`0xED5E8F2C`) on every pack except three NHL 16 and three NHL 17 packs
  where it is 0 — it never discriminates a pair; opaque (not `lookup2` of any of the 25 000 candidates, not a class or collection key
  in the vault).
- The DB column `variantleadingscorer` (NHL 17–21; absent on 15/16) **equals `variant` on every stock row** (583 / 684 / 1 035 / 1 126 /
  1 252 non-zero rows, 0 rows where it differs). It therefore does not select the topscorer pack statically; the vault flag is the only
  static discriminator, and *whether* a player is drawn in the topscorer pack is a runtime decision (inferred: the league's leading
  scorer wears it — the Liiga/NLA/CHL practice — which is what the name says, not what was observed).

## 3. Residuals (observed; full lists in the JSON)

**DB triples without any pack** (`db_triples_without_pack`): 0 / 2 / 8 / 2 / 22 / 59 / 74. They are the spare-kit and placeholder art
ids — `255` (16–21), `30220`/`30221` (17), `1000`–`1004` (19–21: 3 / 4 / 4 / 9–37 / 4 rows) — plus on NHL 20/21 thirteen `(variant 10,
style 2)` rows and a handful of `(0..4, style 2)` rows for art ids 1, 23, 29 whose `adidas`-fit pack does not exist. What the game draws
for these rows is unknown (a fallback pack, a generic kit, or nothing).

**Pack triples without a DB row** (`pack_triples_without_db`): 348 / 39 / 53 / 128 / 158 / 148 / 198. Concentrated on `Team` values no
stock row uses — NHL 15: 341/342 (16 each), 193, national teams; 16–21: 51, 505 (6 each), 30/31 (8 each, 18+), 469, 900 (30 on NHL 21 —
outdoor/Lake-Tahoe kits, cf. handoff 06 `outdoor_*` levels), placeholders 999/1000/1001 — i.e. authored kits the stock DB does not
expose (custom-team and Creation-Zone material by name). Not "orphans": the collection is reachable by its own key.

## 4. Contract for the developer (static; inputs, joins, validation, refusals) — proposed, not authorised

- **Key.** A jersey texture pack is identified by `(Team, UniformVariation, JerseyFit, IsTopScorer)`; the DB row supplies the first three
  as `(teamartid, variant, style)`; `IsTopScorer` is a property of the pack, not of the row. Present the two packs of a matched pair as
  "kit" and "leading-scorer kit" of the same DB row. Do not treat `DebugName` or the slug numbers as the key (handoff 07 §3 item 2).
- **Validation.** A row whose triple has no pack is disclosed (§3 list), not refused — stock ships such rows. A triple with two
  `IsTopScorer 0` packs (NHL 19 Malmö only) is disclosed as ambiguous. Titles: 15 has no `IsTopScorer` attribute — every triple is
  unique there.
- **Allowed edits (scope statement only).** Replacing the bytes of a texture referenced by a pack (the `ge_texture.FileRef` slug →
  archive member) is a Phase 2.6 asset replacement that needs no vault write; changing which pack a row uses, or adding a pack, needs a
  vault writer, which does not exist and is not proposed (handoff 07 §8; collection keys are opaque, Q-R7-1).
- **Preserved bytes.** The vault is read-only in every operation proposed so far.
- **Reopen check.** After any DB `variant`/`style` edit, re-run the join for the edited row and disclose "no pack" / "two packs".
- **Runtime.** Whether the game consumes the vault, and when it selects the `IsTopScorer 1` pack, is unobserved (card below).

## 5. Rev. 2 packaging corrections carried forward for the developer (unchanged from the 19 September checkpoint)

For P1-F and every later changed-length asset write — consume [`overnight-2026-09-19/data/r2-fit-audit.json`](../overnight-2026-09-19/data/r2-fit-audit.json)
as **source-bound regression evidence**, not as a placement rule:

1. **6 of the 38** measured unchanged-content RefPack re-encodes fit their measured slots (3 X-A outer packages NHL 08/09/10
   `leaguelogossmall/l0`, 3 inner `leaguelogossmall` textures on X-B NHL 11 / P-B NHL 10 / P-C Legacy); 32 do not. The **actual edited
   bytes and their encoded size** decide placement against the measured, verified-zero slack — the unchanged-content census is not an
   edit-placement guarantee in either direction.
2. **Four** measured archives have stale declared sizes (NHL 15 `game20.big`, `inxp20.big`, `patch_inxp00.big`; NHL 16 `game_90.big`);
   "ten" is superseded.
3. The full-hole zero findings (`r2-anomaly-hole-scan.json`) apply **only to those four scanned archives**; other archives' gaps remain
   sampled and are reusable capacity only after being read and verified zero for that slot.
4. The inner-package population is **129 rows / 102 distinct byte contents**; "116" is superseded and not reproducible.

## 6. Unknowns (explicit)

1. Runtime: whether the vault is the consumer-side binding and when the `IsTopScorer 1` pack is drawn.
2. The NHL 19 Malmö `(127, 0, 1)` pair.
3. The constant Int32 (`bb93351e` / `354f452f`) and the fallback for DB rows without a pack.
4. `IsTopScorer` on NHL 15 (attribute absent; the four NHL 15 multi-pack triples are placeholder/shared-id authoring, not stock rows).

## 7. User card (deferred; no review requested now)

- **R8-KIT-1 (NHL 21 PS4, observation only, no edit):** in a Champions Hockey League or Liiga exhibition on a stock profile, note whether
  one skater per club wears a visibly different jersey (the `*topscorer*` pack). Presence is consistent with the runtime selecting the
  `IsTopScorer 1` pack; absence identifies no cause (the mode, the roster's scoring state, or the pack never being consumed all remain
  open). Next control: the same club in a mode that shows league leaders.

## 8. Reproduce, checks, cleanup

```
python local/runs/overnight-research-2026-09-20/probes/r8_jersey_triples.py docs/research/overnight-2026-09-20/data/r8-jersey-triple-joins.json [--titles nhl21]
```

Executed (read-only): the probe over all seven titles (≈ 4 min, log in the session scratchpad, deleted at closeout); ad-hoc python over
the JSON for the pair patterns, the `variantleadingscorer` test, the residual breakdowns and the attribute-name search
(`lookup2` over ≈ 25 000 prefix/stem/suffix candidates → `IsTopScorer` only). **Not executed:** launches, writes, any Studio binary.
No scratch retained beyond the probe and the JSON.

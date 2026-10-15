# Handoff R15 — Players (PS4 15–21): the `headid` → head collection binding is a typed attribute in the vault; hair likewise; what that changes for the head-asset programme

Research lane, 20 September 2026 (Priority 3 of the overnight campaign: 3.1 / 3.4 / 3.5). Claim `research-r15-players`. Consumer: developer
Phase 3.4 (portraits / heads / hair), the NHL 21 cyberface programme ([docs/audits/nhl21-ps4-head-asset-binding.md](../../audits/nhl21-ps4-head-asset-binding.md)
§8 — **its "no binding found" conclusion is superseded at the data level by this handoff; the audit file itself is not edited**), and
the external head-converter integration. Builds on handoff [07](../overnight-2026-09-19/07-ps4-vault-collections-and-slug-binding.md)
(reader) and today's [07](07-teams-reference-census-vaults-and-audio-keywords.md) (reference census, audio). Nothing here is
game-verified; nothing written; no launch; **no writer proposed**.

Machine-readable: [`data/r15-player-asset-binding.json`](data/r15-player-asset-binding.json) — per PS4 title: vault identities, the head /
head-texture-pack / hair-texture-pack record counts and attribute coverage, DB `headid` / `hairid` coverage by the vault values, the
bespoke-name agreement check with every mismatch listed, witnesses with texture paths. Probe: `r15_player_asset_binding.py` (≈ 5 min).

**Evidence vocabulary:** observed / inferred / unknown as before.

## Plain-language summary

The head-asset audit of the NHL 21 program ended with "no id → name binding was found anywhere in the shipped data" after eight
negative probes and pointed at eboot disassembly as the only avenue left. That conclusion rested on reading the head collections'
*index* records; the attribute-level reader built yesterday reads their *instance* data, and there the binding is: every head
collection (`ge_nhl_skaterhead`) and every head texture pack (`ge_nhl_skaterheadtexturepack`) carries one typed enum attribute of
type `SkaterNameAttribute` whose value **is the database `headid`** — 766 records, 766 distinct values on NHL 21, and 604 of the
605 distinct `headid` values in the database resolve (the missing one is 0 = "no head"); the same holds on every PS4 title back to
NHL 15 (495 / 439 …). Where the collection name can be checked against the player's name, 364 agree exactly and the other 31 are the
same person under a nickname, hyphen or typo variant — the audit's name-drift cases, now explained as harmless. Hair works the same way
through `GenericHairStyle` (58 / 58 database hair ids on NHL 21; the class first appears in NHL 17). Consequences: (1) reading a
player's head/hair assets is a parsed join, not a name heuristic; (2) reassigning a player to any *existing* head is a database edit
(`headid`), as the audit already allowed; (3) minting a **new** head still needs a new vault collection, and the vault's collection
keys remain underived (Q-R7-1) — so the 606-slot ceiling of the audit stands for *creation*, while its "binding unknown" reason is gone
and the eboot-disassembly avenue is no longer needed for reading. Portraits are unchanged (`artid` names `playerportraits/p<artid>`,
audit §6; `artid == headid` on 394 rows).

## 1. The binding (observed on 7 / 7 PS4 titles)

| Title | `ge_nhl_skaterhead` records = distinct `SkaterNameAttribute` values | `…headtexturepack` records | DB `headid` distinct → in vault | Bespoke ids checked by name: exact / drift | `hairid` distinct → in vault (`GenericHairStyle`) | `artid == headid` rows |
|---|---:|---:|---|---|---|---:|
| 15 | 495 / 495 | 495 | 439 → 438 (missing: 10 391) | 226 / 31 | class absent (hair ids unbound here) | 258 |
| 16 | 544 / 544 | 544 | 448 → 448 | 217 / 30 | class absent | 247 |
| 17 | 608 / 608 | 608 | 483 → 483 | 250 / 33 | 54 → 54 | 283 |
| 18 | 638 / 638 | 638 | 492 → 491 (missing: 0) | 255 / 30 | 54 → 54 | 339 |
| 19 | 698 / 698 | 698 | 536 → 535 (0) | 296 / 30 | 58 → 58 | 326 |
| 20 (install DB) | 743 / 743 | 743 | 590 → 589 (0) | 348 / 32 | 59 → 59 | 380 |
| 21 | 766 / 766 | 766 | 605 → 604 (0) | 364 / 31 | 58 → 58 | 394 |

- Attribute: key `0x4028769e`, enum type `EA::AttribSys::Enums::SkaterNameAttribute`, present on 100 % of the head records; its *name* did
  not resolve from ~50 candidates (`HeadId`, `HeadGeoId`, `SkaterName`, …) — the type name is what identifies it; opaque name, typed value.
- Value bands (NHL 21): 352 values < 8 000 (early bespoke ids), 203 in 8 000–59 999 (later bespoke), 211 ≥ 60 000 (the generic pool,
  `head_generic_player_<n>` — 239 collections carry `generic` in their name incl. female/referee). Vault values not used by the DB: 162
  on NHL 21 (spare / retired heads — addressable without a new collection).
- Chain (NHL 21 witness): `headid 20` → `ge_nhl_skaterhead` key `…` (SkaterNameAttribute 20) and `ge_nhl_skaterheadtexturepack`
  `head_nik_antropov_texpack` (same value) → `ge_texture` `rendering\characters\player\head_nik_antropov_cm`, `hair_nik_antropov_cm`,
  `head_common_normal`. Kevin Shattenkirk `headid 8443` → `head_kevin_shattenkirk_texpack`; the head record also references a
  `ge_nhl_skaterhelmetplacement` and its `ge_gameobject` (the mesh binding of the audit §3.3).
- Name drift (all 31 NHL 21 cases are the same person): `Marc-Edouard Vlasic` → `head_marcedouard_vlasic`, `Alex Ovechkin` →
  `head_alexander_ovechkin`, `Corey Perry` → `head_cory_perry`, `Gabriel Landeskog` → `head_gabriel_lendeskog`, `Valtteri Filppula` →
  `head_valterri_filppula`, `Mitchell Marner` → `head_mitch_marner`, … (full list in the JSON) — the audit's heuristic misses, resolved by
  the attribute.
- The one NHL 15 miss (`headid 10391`) has no head record in the NHL 15 vault — a DB reference to a head that title does not ship
  (inferred: falls back at runtime; unobserved).

## 2. What this changes (for the audit's §8 and the programme)

| Audit statement | Status now |
|---|---|
| "no bespoke head id appears as a string / no numeric attribute on the `head_<name>` collections" | true of the index records and the string pool; **false at the instance level** — the typed attribute holds the id |
| "the collection key is not a function of the head id" | unchanged (keys still opaque, Q-R7-1) |
| "known-good addressable head slots: 606" | unchanged for *creation* (a new head = a new collection pair + textures + mesh, no writer, keys underived); for *reassignment* every value in the vault (766 on NHL 21, incl. 162 unused by the DB) is addressable by `headid` |
| "remaining avenue: disassemble the `HeadGeoId` consumer in eboot" | not needed to *read* the binding; still the only route to learn how a *new* key is formed, together with the vault-key question |
| "name matching is a heuristic, not the binding" | confirmed and superseded: the attribute is the binding, the name is documentation |

Operation scope (proposal): **reassign** = DB `headid` (and `hairid`) edit within the existing editor, validated against the vault's
value set of the title (refuse values with no record; disclose generic-pool vs bespoke); **replace textures/mesh of an existing head**
= the texture paths from the pack + the mesh from the head's `ge_gameobject` (audit §3.3), archive placement per handoffs 02/03; **new
head** = blocked on the vault writer (collection keys). Runtime card **R15-HEAD-1 (NHL 21 PS4):** set one skater's `headid` to another
existing bespoke id (e.g. Antropov's 20) — the other face appears = the DB id drives the vault lookup; unchanged = no cause identified
(cache, the pool fallback, another copy) — next control: the same edit to a generic-pool id (60000+).

## 3. Other Player items (status)

- **Identity / roster FKs:** handoff 07 §1 — `exhibitionplayerbiotable.team` / `.juniorteam` → team index; `draftteam` / `proteam` are
  NHL club numbers (0–31), `artid` is the player art id (portrait), `stockawardstable.playerartid` likewise.
- **Name audio:** open (handoff 07 §3 — `audioid` / `surnameid` are neither PBP keyword ids nor discriminable sample ids); next: the
  `pbp_speecheventsubscr` layer or the `rink_announcer` banks' DSET naming.
- **Localization:** player names are direct DB text (`firstname` / `lastname`), not dictionary keys (handoff 23, unchanged).
- **Gen7:** `ubPc.lOnr` = portrait id, `pPaD` = head id, bespoke iff equal (memory / handoff 30 of 2026-09-18); no vault — the gen7
  head assets are id-named files; not re-measured tonight.

## 4. Unknowns (explicit)

1. The attribute's name (hash `0x4028769e`); the second Int32 on head records (`0x63eefc24`, constant `1958391428`). 2. Collection-key
derivation (new heads). 3. Runtime fallback for a `headid` without a record. 4. Player-name audio. 5. Facial hair (`ge_nhl_skaterfacialhairtexturepack`:
1 record, `FacialHairColor`) and eyebrows — not measured here (audit §5 says direct id link).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-20/probes/r15_player_asset_binding.py docs/research/overnight-2026-09-20/data/r15-player-asset-binding.json   (≈ 5 min)
```
Executed once (read-only) plus the exploratory class-attribute dump and the attribute-name hash test in the session. **Not executed:**
launches, writes, Studio binaries.

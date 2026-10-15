# Handoff 08 (28 Sep) — PS4 Creation Zone name keys resolve through each title's own dictionary (goalie masks, skater gear)

Research lane, 28 September 2026, published 06:47 MDT (system clock). Prompt package 6 — the "next research task" drafted for receipt 5,
taken immediately. Claim `research-q6c-cz-name-keys`. Read-only:
- each title's `nhl/data/kettle/loc/eng_us.db`, decoded by the research TDB reader;
- the Creation Zone CSVs of handoffs 02 / 02b.

Machine-readable: [`data/r141-cz-name-keys-dictionary.json`](data/r141-cz-name-keys-dictionary.json). Probe:
`local/runs/overnight-research-2026-09-28/probes/r141_cz_name_keys_loc.py` (`8b9f42de…`).

## Plain-language summary

Studio's new PS4 gear and mask cards (developer receipts 52 / 54) show names as raw keys such as `$bauer4500`. Every one of those keys
is a string in the game's own English text file: "BAUER 4500", "Bauer Nexus 2N PRO", "Aviator", "Starting BAUER" and so on. About one
key in five differs only in upper and lower case from the stored key. So Studio must look keys up the way the game's key hash does,
ignoring case, not by exact spelling. The only keys that name nothing are two placeholder words on some newer mask rows.

## 1. Results (English dictionary of each title; the last archive copy in patch order)

| Title | Dictionary (archive, SHA-256, rows) | Exact key | Case-insensitive (hash) match | Unresolved |
|---|---|---|---|---|
| NHL 17 | `inxp_00.big` `035b3b30…`, 30 799 | 154 | 19 | 0 |
| NHL 18 | `inxp_00.big` `12f1af46…`, 33 396 | 140 | 34 | 0 |
| NHL 19 | `patch_inxp_56.big` `fbb8adcf…` (also in `inxp_00` and patches 51–55), 38 001 | 229 | 49 | 20 |
| NHL 20 genuine | `inxp_00.big` `0fc95ec8…`, 40 535 | 363 | 61 | 44 |
| NHL 21 | `inxp_00.big` `24693e2f…`, 55 766 | 437 | 64 | 50 |

- The counts are cells across `goaliemaskinfo`, `goaliemaskdisplayinfo`, `skaterhelmetstyle`, `skaterglovestyle`, `skatedisplayinfo`,
  `stickdisplayinfo` and `skatervisortypes`.
- **Unresolved = only `GoalieMaskPattern` and `GoalieMaskBrandAbbr`.** These are the placeholder cells of the NHL 19+ mask rows whose
  columns are shifted (handoff 02 §5). Those rows' `name` keys (e.g. `$TXT_THE_STAG`) resolve.
- 3–4 dictionary rows per title have a stored hash ≠ the hash of their key. That is a dictionary fact, not a lookup failure.

**Examples (genuine NHL 20):**

| Key | Text |
|---|---|
| `$bauer4500` (hash) | BAUER 4500 |
| `$bauerreakt100` | BAUER RE-AKT 100 |
| `$ccmfitlite` | CCM Fitlite |
| `$bauernexus2n` (hash) | Bauer Nexus 2N PRO |
| `$TXT_VISOR_AVIATOR` | Aviator |
| `$none` (hash) | None |
| `$bauer` (hash) | BAUER |
| `$yellow` | Yellow |
| `$TXT_STANDARD_MASK_DESC` | `%1s %2s` (a template: style name, then pattern name, e.g. "BAUER Yellow") |

**Mask display keys** (`goaliemaskdisplayinfo.displayname`, `maskstyle` 0 / 1 / 2 / 6): "Starting BAUER", "Starting Cage",
"Starting Cage 2", "Starting CHL Mask" (and "Backup …"). These are the title's own labels for helmet types 0 / 1 / 2 / 6. Studio may
show them because they are the title's text, not invented brand names.

## 2. For the developer (receipts 52 / 54)

1. **Resolve through the open title's own `eng_us.db`** (the last archive copy in patch order; bind its SHA-256). Strip the leading `$`,
   then look up **by `tdb_loc::loc_hash(key)`**. The hash is case-insensitive for ASCII. `LocDictionary::text` (exact key) misses
   19–64 cells per title.
2. **Mask names:** `name` is a `%1s %2s` template on standard masks. Fill it with the resolved `stylename` and `patternname` only when both
   resolve; otherwise show the keys. Show named masks (`$TXT_THE_STAG`, …) as resolved.
3. **Unresolved placeholders** (`GoalieMaskPattern`, `GoalieMaskBrandAbbr`): show nothing for those cells, and never the placeholder word.
4. **Other languages:** use the matching locale file if Studio has a locale setting. English is what was measured.
5. **Wording:** no brand is invented; every string is the title's own text.

## 3. Reproduce

```
python -B local/runs/overnight-research-2026-09-28/probes/r141_cz_name_keys_loc.py <out.json>   (≈ 3 min)
```

# Handoff 13 (28 Sep) — gen7 front-end Attrib vault holds no create-player picture lists (bounded negative); NHL 14 trapper / glove picture checks do not add a join

Research lane, 28 September 2026, published 15:19 MDT (system clock). After-receipt-6 prompt, "bounded equipment research" (RECEIPT-6
§8). Claim `research-q12-gen7-feattribdb-and-fallback`. *([14](14-after-receipt7-corrections.md) K17: this package also relied on no delegation; the delegated search was package 2 of this run, checkpoint 12 §2.)* The claim file was written at publication; the work started without it, which is
recorded as a process slip. Read-only on the installs. Nothing launched.

## Plain-language summary

- ~~The front-end vault is not where the picture lists live. … in any encoding.~~ *(Narrowed 18:44, [14](14-after-receipt7-corrections.md) K16.)* No picture-id list was found in the **tested encodings** (ordered u8 / u16 BE / u32 BE) or in the 40 / 47 records that parse. NHL 15 and Legacy ship byte-identical vaults with different complete catalogues, so different complete lists cannot be encoded solely in those bytes. A shared superset, filtering outside the vault, indirection or runtime-built lists remain possible.
- **The NHL 14 glove and trapper comparisons add nothing usable.** One glove picture looks more like its neighbour's design.
- The create-player picture join therefore stays what handoff 11 (as corrected in 12 §3) says: a candidate for pads and blockers on
  sampled titles. It is not a data edge.

## 1. Vault structure (measured on X360 Legacy; NHL 15 identical)

Probe `r144_gen7_feattribdb_vault.py` (SHA-256 `66ba0aa4…`); data [`data/r144-gen7-feattribdb-vault.json`](data/r144-gen7-feattribdb-vault.json)
(`bb721a44…`).

- **`.vlt` chunk chain** (big-endian `tag | u32 size`): `Vers` 16, `StrN` 16 (**empty**: names are hashes only), `DepN` 80, `DatN`
  15 568, `ExpN` 1 152, `PtrN` 5 488, `EndC`.
- **`.bin`:** `StrE` (empty), `EndC`, then an unframed heap.
- **`ExpN`:** u32 0, u32 count = **47**, then 24-byte rows `key u64 | type u64 | size u32 | offset u32`. All 47 rows carry one type key
  (`ad303b8f42b3307e`). Keys are **64-bit**, unlike PS4's 32-bit lookup2 keys, so PS4 offsets and hashes were not transplanted.
- **`DatN` record:**
  - collection key u64 | class key u64 | u64 0 | u32 nA | u32 nT | u32 nA | u16 | u16 | 8 B 0 | nT × 16 B type rows;
  - then nA × 16 B attribute rows: `attr key u64 | value u32 | flags u32`.
- **Consistency:** 40 of 47 records parse consistently (nA repeated, rows fit the size). 32 classes. Flags: `0x4000` inline 389,
  `0x0200` 275, and others. The other 7 records have a different layout, which was not pursued.
- **Title identity:** `feattribdb.vlt` SHA-256 `e1ad72be…` and `.bin` `394d7381…` are **identical on X360 NHL 15 and Legacy**. Every
  other title's `.vlt` differs.

## 2. Candidate searches and discriminators

| Test | Result |
|---|---|
| S1: each title's create-player id list per family (pads, blocker, trapper, gloves, helmet, skates, sticks, goalie stick, mask), as ordered u8 / u16 BE / u32 BE, whole list and first 8 ids, in `.vlt` and `.bin` | **0 hits** on Legacy and NHL 15. Controls (shuffled, ids + 1) also 0 |
| S2: inline values equal to a family's picture count | scattered small-integer coincidences that disagree between the two titles, **even though the vaults are identical**. That confirms they are coincidences |
| S3: ascending runs of 8 or more values (step 1–3) in the `.bin` heap, u8 / u16 / u32 | **none** |
| **Discriminator:** vault identity vs catalogue difference | NHL 15 and Legacy have the same vault bytes, but different catalogues: pads 87 vs 92, sticks 101 vs 106, goalie sticks 48 vs 54, trapper 86 vs 91. **the identical vault bytes cannot by themselves encode the different complete per-title lists** *(narrowed 18:44, [14](14-after-receipt7-corrections.md) K16: superset / filtering / indirection / runtime construction remain possible)* (compare `equipatrib.bin`, handoff 11 §1) |

**Result.** Two independent approaches failed: the id-list search, and the identity-versus-catalogue discriminator. This is a bounded
negative. The picker's `ASSET_ID` arrays are not in `feattribdb`. They come from native code or another source not examined here. No
disassembly was attempted.

## 3. Fallback: NHL 14 glove and trapper picture checks (R143e sheets; recorded in `data/r143d-visual-checks.json`)

| Family | Ids (N − 1 / N / N + 1 controls) | Match at N | Plausible at N | Undetermined | Neighbour signal |
|---|---|---|---|---|---|
| gloves (`glove_<N>_0_tm`) | 20, 24, 29, 40, 48, 55 | 40 ("EASTON" lettering) | 55 | 20, 24, 29 | **48: cuff lettering resembles mask 47** |
| trapper (`trapper_0_<N>_0_tm`) | 12, 24, 33, 44, 55, 65 | — | 12, 44 | 24, 33, 55, 65 | none |

- **Gloves:** the sample does not support a picture join, and one counter-signal exists. Studio should not present glove pictures as
  matching the stored value.
- **Trappers:** tint masks are dominated by the net and discriminate weakly. Trappers stay at "plausible".
- **Expansion stopped**, as the prompt directs.
- The glove sheets are kept in the review set (`H:\TEMP\nhlms-research-c38-visual-2026-09-28\nhl14-gloves*.png`, hashes in the JSON).
  The trapper sheet was deleted.
- PS3 archive-held equivalents were **not** checked. No established reader renders them, and equal id sets would not prove identity.

## 4. Consequence for C38

- **Unchanged:** pads / blocker candidate on the sampled titles; trapper plausible.
- **Changed:** gloves move to **"not supported"**, following this counter-signal. Sticks, helmets and skates keep set-only confidence.
- Queue C38 carries a dated note.

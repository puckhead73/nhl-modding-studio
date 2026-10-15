# Research receipt 7 (consolidated) — continuation after receipt 6, 28 September 2026

Research lane. Prompt: [`research-20260928-after-receipt6.md`](../../plans/research-20260928-after-receipt6.md). Published 15:21 MDT (system
clock). Current authority: [BETA-HANDOFF.md](BETA-HANDOFF.md). Log: [STATUS.md](STATUS.md). Queue notes on C32, C37 and C38.
- No staging, commits, publication, launches, listening or cross-chat messages.
- No installed, anchor or fixture writes.
- No app / defs / binding / developer-test / coordinator-record edits.
- One read-only delegated source search inside this session.
- Every contract here awaits independent review.

## Summary

- **Board-ad validator corrected.** The coordinator found three admission faults, and all three are fixed:
  - an overlap hidden behind an enclosing span;
  - DXT5 admitted;
  - zero sizes normalised to one.

  The corrected validator passes all 29 test cases. Re-checking every real PS3 arena package changed nothing.
- **Recompiled mapped operation by operation.** Research now names which Studio operations work on Legacy Recompiled and which still
  look for folders that install does not have. Two need attention: the portrait / head containment card claims an archive check it
  does not make, and Hockey Shop accepts Recompiled but could only write loose files.
- *(Corrected 18:44, [14](14-after-receipt7-corrections.md): C37 accepted for implementation (K15); vault result narrowed to the tested encodings and parsed records (K16); the delegated search and the late claim are recorded process deviations (K17); C38 stays candidate-only, review folder removed (K18).)*
- **Equipment pictures.** The front-end vault does not hold the create-player picture lists; this is proven by identical vaults
  across two titles with different catalogues. Glove pictures showed a counter-signal, so gloves are not supported.

## 1. Corrections first

- **C37 / handoff 10 §1:** superseded for writer admission by **§8** (R142b):
  - pairwise overlap refusal (enclosing, non-adjacent, equal-start, duplicate), with boundary-touching spans allowed;
  - DXT5 identified by discovery but refused at admission;
  - width, height and level count must be above zero, with bounded levels;
  - format words must be equal.
- **C38 / handoff 11:**
  - confidence is stated per title and family;
  - equal id sets are not picture identity;
  - a missing mask picture reads "no picture available", never "team mask";
  - gloves are not supported (13 §3).
- **BETA-HANDOFF:** r64 instead of r59; F1 closed in its tested scope; F2 and temporary-file ownership recorded as active developer
  corrections (row D1); Recompiled blocker split into named routes (B14); cards mapped to r64 (B9).
- **Process slip:** package 3's claim was written at publication, not before work began. This is recorded in the claim and in 13.

## 2. Outputs

| Package | Output | Result | Queue |
|---|---|---|---|
| Validator correction | [10](10-ps3-board-raster-writer-contract.md) §8; probe `r142b_c37_admission_validator.py` (`9709b0f7…`); [`data/r142b-c37-admission-validator.json`](data/r142b-c37-admission-validator.json) (`4c2343fa…`) | 14 R142 + 15 new vectors, 29 / 29; census edges 243 / 243 / 243 / 232 / 226 / 136, no edge-set change; NHL 11's 3-level chain kept | C37 note |
| Reconciliation | [12](12-reconcile-after-receipt6.md) | developer status table; Recompiled route per operation (16 rows, file:line); C38 confidence; r64 card map; review set outside the repo, manifest [`r143f`](data/r143f-visual-review-set.json) | C32, C38 notes |
| Equipment | [13](13-gen7-feattribdb-negative-and-fallback.md); probe `r144_gen7_feattribdb_vault.py` (`66ba0aa4…`); [`data/r144-…`](data/r144-gen7-feattribdb-vault.json) (`bb721a44…`) | vault structure measured (64-bit keys, 47 exports); 0 id-list hits; identical vault on NHL 15 and Legacy vs different catalogues (decisive negative); NHL 14 gloves: one counter-signal; trappers plausible only | C38 note |

## 3. Developer-consumable scope (priority)

1. **C37 (after coordinator review):**
   - PS3 DXT1 / DXT3, same size, whole declared chain, behind §8.1;
   - pair = one review and recovery scope; archive-only = member route; stager required;
   - DXT5, X360 and X360 NHL 10 refused.
   - The developer owns the independent decode of `compress_bc` output and the transaction tests.
2. **Recompiled routes (12 §2):**
   - correct the containment card's disclosure, or pass it the `ArchiveIndex`;
   - O12-1: Hockey Shop on `legacyrecomp` should refuse or disclose its loose-only write;
   - then the speech catalogue member route, the speech-preview `game/` root, the preview member routes, and the `legacyrecomp` id rows
     for imagery, kits and venue art.
3. **C38:**
   - fix the two prefixes;
   - labelled candidates for pads / blocker (trapper plausible) on the sampled titles only;
   - no glove join;
   - neutral missing-picture text.

## 4. Checks actually run

- **R142b, one run** (about 2 min, read-only census over 370 packages): 29 / 29 vectors.
- **R143f:** rebuilt 5 review sheets with source hashes.
- **R144, one run** on X360 Legacy and NHL 15. Plus `sha256sum` of `feattribdb.vlt` on X360 NHL 07 → Legacy: only NHL 15 and Legacy
  are equal.
- **R143e sheets:** NHL 14 gloves (6 ids plus one enlarged) and trappers (6 ids).
- **Source reading:** 16 Recompiled consumers (delegated read-only search). Research re-read the key lines:
  - `speech_catalogue.rs:150`;
  - `registry.rs:325–329`;
  - `player_containment.rs:142, 209–217`;
  - `registry.rs:3609–3627`;
  - `defs/league_imagery.toml`.
- **Nothing rerun without a changed premise:** no real-file round trips, no LZX, no audio.

## 5. Remaining October 1 blockers (owner → smallest action)

| Blocker | Owner | Action |
|---|---|---|
| F2 cache revalidation; temporary-file ownership | developer | the receipt-65 dispatch |
| Board-ad replacement | coordinator → developer | review 10 §8, then the PS3 writer |
| Recompiled named routes; O12-1 | developer | 12 §2 rows |
| User acceptance | user | r64 cards V-45 → V-95, A-XMA-2 → 8 |
| Austrian phases | user → coordinator | P1-A |
| Mask slot vs personal mask | user | G-MASK-1 |
| Drawn heads | user | P34-1 / R15-HEAD-1, P34-G08 |
| Archive re-key, X360 board gates | coordinator | withheld under their existing contracts |
| Release: MIT holder; artwork replacement before external release | user / developer | not research |

## 6. Cleanup and retention

- **Session scratchpad:** emptied (sheets, specs and helper scripts). The retained exception is below.
- **Kept outside the repo:** `H:\TEMP\nhlms-research-c38-visual-2026-09-28\` (7 PNG, 1.75 MB), owned by research.
  - Successor dependency: the coordinator's review of C38 (this receipt).
  - It is deleted after that review.
  - Manifest: `data/r143f-visual-review-set.json` and `data/r143d-visual-checks.json`.
- **Kept in the repo:** probes R142b, R143f and R144; the data files above.
- **Bytecode:** none created (all runs used `-B`).
- **Not touched:** shared caches, bundles, toolchain, fixtures, other H:\TEMP folders.
- **Disk:** E: has about 9.5 GB free.
- **Claims:** q10, q11 and q12 are `ready`.

## 7. Next research task

**Resolve O12-1's premise statically.** Does the Recompiled executable's install contain any loose-override search for `rendering/`
beyond the `update:` / `cache:` prefixes R128 noted? Use only the recompiled program's own logged or config evidence in the registered
install. No disassembly, no launch.
- If nothing static exists, publish that the loader precedence stays a user runtime card.
- Then write that card: a copied Recompiled install, one changed loose file, and the observed result.

# Handoff R45 — NHL 11 / 12 `CTX0` v3.1 script ids are the `.gas` blob header id words (a keyed lookup, not an index): 100 % resolved on all six sets, 59 unreferenced play-by-play blobs listed

Research lane, 21 September 2026 pm, roadmap step 1.5 / 2.7 / 3.5 gen7 (the "exact next task" item 1 of the 2026-09-21 closing receipt;
prompt step 4 "census NHL 11 / 12 CTX0 v3.1 condition / script ids against CSIS blob headers, preserving unresolved references; do not assume
the v4.5 index rule"). Claim `research-r45-ctx0-v31-gas-ids`. Read-only over the loose `audio/context` files of X360 NHL 11 / 12 and PS3
NHL 11; nothing game-verified; no writer; the `.gas` instruction set stays out of scope.

Machine-readable: [`data/r45-ctx0-v31-script-ids-vs-gas.json`](data/r45-ctx0-v31-script-ids-vs-gas.json) (per tree / set: file hashes,
id-word census, condition and expression joins with the full unresolved list — empty —, unreferenced blob ordinals, blob header shapes by
reference class). Probe `r45_ctx0_v31_script_ids_vs_gas.py` (≈ 25 s; imports the 2026-09-21 `r41` parser and the 2026-09-20 `r22` `.gas`
reader).

## Plain-language summary

On NHL 13 and later, a context record points at its condition script by position in the script file. NHL 11 and 12 use a different rule that
handoff 16 could not name: the number in the record is not a position. Tonight's census shows what it is — every script blob in the NHL 11 /
12 `.gas` files starts with its own id word (non-sequential, unique), and every condition and expression id in the context records is one
of those id words: 11 702 / 11 702 condition references and 2 438 / 2 438 expression references on NHL 11 (13 099 / 13 099 and 2 638 /
2 638 on NHL 12; the crowd sets 5 113 + 1 020 each) resolve, with **no** unresolved reference on any set. So the developer's NHL 11 / 12
context listing can show the script for each condition exactly as on 13+, by looking the id up in the blob table instead of indexing it.
Fifty-nine play-by-play blobs on each of NHL 11 and 12 are referenced by no context (listed by ordinal); every crowd blob is referenced.
What a blob *computes* is still not decoded and is not needed for the listing.

## 1. Measured (three trees; `pbp_eng` and `crowds`; PS3 NHL 11 = X360 NHL 11 on every number)

| Set | `.gas` blobs (none empty) | id words: distinct / min / max / injective / sequential | condition ids resolved (refs) | expression ids resolved (refs) | ids used by both | blobs unreferenced |
|---|---|---|---|---|---|---|
| NHL 11 `pbp_eng` | 1 739 | 1 739 / 5 / 2 250 / yes / **no** (896 order inversions) | 1 525 / 1 525 (11 702 / 11 702) | 155 / 155 (2 438 / 2 438) | 0 | **59** (ordinals 14, 43, 86, 94, 124, 157, 191, 248, 250, 332 … in the JSON) |
| NHL 11 `crowds` | 588 | 588 / 1 / 1 922 / yes / no (307) | 316 / 316 (5 113) | 272 / 272 (1 020) | 0 | 0 |
| NHL 12 `pbp_eng` | 1 975 | 1 975 / 5 / 2 505 / yes / no (999) | 1 758 / 1 758 (13 099) | 158 / 158 (2 638) | 0 | **59** (ordinals 16, 49, 91, 101, 129, 169, 211, 274, 276, 367 …) |
| NHL 12 `crowds` | 586 | 586 / 1 / 1 922 / yes / no (305) | 316 / 316 (5 113) | 270 / 270 (1 020) | 0 | 0 |

Rule (v3.1): **`script id` in a condition or expression = the `u32` at offset 0 of the referenced `.gas` blob**; the blob is found by
scanning the blob table for that id word (a map with no collisions on any set). On v4.5 the same word equals the blob's ordinal, so the
v4.5 index rule is the special case where the map is the identity — the developer can implement one lookup (`id word → blob`) for both
versions and drop the index assumption. The condition-blob and expression-blob id sets stay **disjoint** on every set (as on v4.5).

## 2. Blob header shape (observed, not decoded)

Every blob begins `u32 id │ u32 w1 │ u32 w2 │ u32 w3 │ u32 ptr │ …` with `w1 = 0` and **`ptr = blob offset + 28 + 4·w2 + 8·w3`** on every
shape seen (`[0,0,0] → +28`, `[0,1,0] → +32`, `[0,0,1] → +36`, `[0,1,1] → +40`, `[0,0,4] → +60`, `[0,2,3] → +60`, `[0,4,3] → +68`, `[0,0,8]
→ +92` …): `w2` counts 4-byte entries and `w3` counts 8-byte entries that precede the body the pointer addresses. Condition blobs are
dominated by `[0,1,0]` (480 / 637) and expression blobs by `[0,0,4]` / `[0,1,1]` on `pbp_eng`; the 59 unreferenced blobs are `[0,0,0]` (34),
`[0,2,3]` (16), `[0,4,3]` (5), `[0,0,6]` (4) — shapes that also occur among referenced blobs, so shape alone does not mark them as dead.
This is a header-layout observation for the listing ("script k, w2 / w3 entries"); the entries' meaning and the body's instruction set are
**not** decoded (out of scope for this campaign).

## 3. Contract consequence (handoff 03 rev. 2b → 2c, this campaign)

`CTX0` listing on NHL 11 / 12: each condition shows its inline name **and** its script (blob ordinal found by id word, blob size, `w2` /
`w3` entry counts); each expression parameter likewise. Refusal: an id word absent from the blob table is reported per condition (0
observed on the six sets — keep the check); a `.gas` whose id words are not injective is refused as a whole (0 observed). The
unreferenced blobs are listed under the file, labelled "no context references this script" — not "unused" (another module or the
executable could reference them; not tested).

## 4. Not claimed

Blob semantics; why 59 pbp blobs are unreferenced; whether NHL 12 PS3 (archive-only `audio/context` in `audioboot.big`) is identical
to X360 (not extracted here; NHL 11 PS3 = X360 was measured); the w0 / w1 / w2 record header words (handoff 05 of this campaign).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r45_ctx0_v31_script_ids_vs_gas.py docs/research/overnight-2026-09-21-pm/data/r45-ctx0-v31-script-ids-vs-gas.json   (≈ 25 s)
```
Inputs (SHA-256 in the JSON): `audio/context/{pbp_eng,crowds}_{contexts.bin,contextgasscript.gas}` on `X:\…\NHL 11\NHL 11`, `X:\…\Sony
Playstation 3\NHL 11\PS3_GAME\USRDIR`, `X:\…\NHL 12\NHL 12`. Executed once (final) after one development run.

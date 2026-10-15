# Handoff R10 — a source-bound placement oracle for changed-length members (BIGF inner re-plan validated on 129/129, BIG4 + `.bh`, EB, NHL 07 cascade, unknown fields)

Research lane, 20 September 2026 (Priority 1B of the overnight campaign). Claim `research-r10-placement-oracle`. Consumer: developer
**P1-F** (gen7 BIGF/BIG4/`.bh` re-plan — "not impl" in the coverage matrix), P1-E, and every later asset step that rewrites a
compressed gen7 member. Builds on handoff [02 rev. 2](../overnight-2026-09-19/02-packaging-size-change-requirements.md) (rules §4.1–4.3)
and its data; corrects one of its layout statements (§2 below). Nothing here is game-verified; nothing was written under `X:`/`H:`
(every "expected file" is a hash of bytes composed in memory); no game launched; the chunklzx/loader questions of handoff 02 §9 are
unchanged.

Machine-readable: [`data/r10-placement-oracle.json`](data/r10-placement-oracle.json) (0.85 MB) — for every case: source identity
(path, length, SHA-256 where hashed, entry index/offset/size, stored SHA-256), the replacement input, the exact words to patch (file
offset, old bytes, new bytes) and the expected SHA-256 of every output file. Probe:
`local/runs/overnight-research-2026-09-20/probes/r10_placement_oracle.py` (read-only; ≈ 12 min with hashing, `--skip-hash` ≈ 1 min;
imports the 2026-09-18 `big4_list`/`eb_list`/`refpack` and the 2026-09-19 `viv4_index` readers).

**Replacement input, defined:** `payload' = stock payload ++ N zero bytes`, declared length `S + N`. This keeps a RefPack stream
decodable (the decoder stops at the terminator) but is a **test construction** — no stock member carries bytes after its terminator
(handoff 02 §3.1) — so the oracle exercises placement arithmetic, not encoder output. Hashes are what a correct planner must produce for
exactly this input; a real replacement's bytes differ and its length decides fit vs growth per handoff 02 rev. 2.

## Plain-language summary

The developer has the rules for moving the small logo packages around when a texture changes size, but no way to check an
implementation except by writing to a game install. This handoff gives that check: for the 129 measured packages it re-derives the
package from its own bytes with the proposed re-plan and reproduces every one byte-for-byte (the rules were tightened in the process:
the 16-byte tail is not zero but a per-asset constant, and zero-length markers that share the texture's offset stay with it); for
1-, 63- and 64-byte growth it lists the exact table words that change and the resulting package hash (96 of 129 absorb one byte
without growing). For the six early-title archives that carry a `.bh` mirror index it gives the fit-in-slot, append and ordered-reflow
outcomes as exact word patches plus the hash of the whole patched `.big` and `.bh`; for the five later EB archives the append case
(their logo slots have zero slack). It also pins the NHL 07 two-level cascade (`boot.big` holds RefPack'd copies of the six `.bh`
files, with 227–2 036 zero bytes after each) and inventories the words nobody has explained, so a writer knows exactly what to copy
untouched. One genuine ambiguity is recorded rather than decided: on the early four-member packages the record after the texture
starts exactly at the texture's end, which is always 64-aligned, so "tight" and "64-aligned" cannot be told apart from the source — the
oracle gives both hashes.

## 0. Sources (observed)

The 129 inner packages and their archives are those of the 2026-09-19 survey (`r2-packaging-survey.json` `packages`, 129 rows / 102
distinct contents; 45 in BIG4 archives, 60 in EB archives, 24 loose PS3 files). Whole-file SHA-256 of the six BIG4 `.big`/`.bh` pairs
and the five EB archives are in the JSON (`source_sha256`, `bh_sha256`).

| Outer case | Archives | Entry (index-0 `leaguelogos` package) |
|---|---|---|
| BIG4 + `.bh` | Xbox NHL 07 `data0.big` (284 526 259 B), NHL 08 `data0.big` (2 000 405 120), NHL 09 `fe.big` (207 665 664), NHL 10 `fe.big` (247 494 656); PS3 NHL 08 `data0.big` (2 000 883 200), NHL 10 `fe.big` (249 900 992) | i 610 / 1165 / 1292 / 1608 / 1136 / 1608; S 5 358 / 8 228 / 8 216 / 7 853 / 17 696 / 8 048; slack 18 / 28 / 40 / 19 / 32 / 16 — all zero |
| EB `0x00000400` | Xbox NHL 11 `fe.big`, NHL 12 / 15 / Legacy `cache.big`; PS3 Legacy `cache.big` | i 2789 / 3592 / 3590 / 4072 / 4068; S 8 496 / 8 304 / 8 176 / 8 176 / 7 856; **slack 0 on all five** |

## 1. Inner BIGF re-plan — validated (observed: 129 / 129 identity)

The re-plan derives each later member's placement rule *from the source package* instead of assuming it per era, then reproduces the
package with the texture unchanged. Result: **129 / 129 packages reproduced byte-for-byte**, 0 refused, 0 failed (`bigf_replan`).
Rules found (member order by offset; texture = the RefPack/raw member > 288 B):

| Family (titles) | Members after the texture | Rule per member | Ambiguous? |
|---|---|---|---|
| X-A (Xbox 07 / 08 / 09 / 10, 27 pkgs) | `<n+1>` (288 B record) | starts exactly at the texture's end — which is 64-aligned on every raw texture | **yes** (tight vs align64 give the same source offset) |
| X-A NHL 07 (2 pkgs: `leaguelogossmall`) | `<n+1>` | align64 after the texture end (the end is not aligned) | no |
| X-B / P-B (Xbox 11, PS3 10 / 11 — stride 64) | `sg2`, `1` | `sg2` = align64(texture end); `1` = `sg2` + 64 | no |
| X-C / P-C (Xbox 12 / 15 / Legacy, PS3 Legacy — stride 0) | `sg1`, `sg2`, `1` | `sg1` shares the texture's offset (stays with it); `sg2` = align64(end); `1` at `sg2` (0-byte marker, so tight = align64) | harmless |
| P-A (PS3 08) | `sg1`, `sg2`, `1` | `sg1` at the texture's offset; `sg2` and `1` at the texture's end, which is 64-aligned (raw `RW4ps3`) | **yes** |

Other invariants used and confirmed by the identity result: `size_le` keeps its source relation (`== len` on X-A/P-A, `== len − 16`
elsewhere); the 8-byte index trailer, the pre-data gap (non-zero on P-A: `42 52 54 53 …`, 24 bytes) and every gap before the texture
are copied verbatim; gaps after the texture are re-created (they are zero on all 129 — checked); the 16-byte tail is copied verbatim.

**Correction to handoff 02 §2 ("tail after the last member: 16 zero bytes")**: the tail is **not zero**. It is a 16-byte word that is
constant per `(family, index)` across every title and both platforms (e.g. `leaguelogos/l0` = `2d26295c…6f199a0a` on Xbox 10 / 11 /
12 / 15 / Legacy and PS3 10 / 11 / Legacy; 14 distinct values over the 104 tailed packages: eight values shared by 9 packages each, four by 7, two by 2 —
one value per family/index) and therefore not a hash of the platform-specific bytes; not MD5/SHA-1 of ~1 500 path-string candidates. **Opaque; copy
verbatim on same-asset replacement; unknown for a new asset.** The `size_le` rule (`len − 16`) is unchanged.

**Growth oracles** (`variants`): for N ∈ {1, 63, 64}, the patched table words (`entry_pos`, old → new offset/size), the new `size_le`,
the output length and SHA-256; where the rule is ambiguous both `…/align64` and `…/tight` outputs are given (87 packages). Witnesses:
Xbox NHL 11 `leaguelogos/l0`: +1 → texture 7 583 → 7 584, nothing else moves, length 8 496 unchanged (fits the 33-byte inner slack);
+63 / +64 → `sg2` 8 384 → 8 448, `1` 8 448 → 8 512, `size_le` 8 480 → 8 544, length 8 560. 96 / 129 packages absorb +1 without
package growth (the inner slack of handoff 02 / `r2-fit-audit.json`).

**Refusals the oracle applies (proposal for the writer):** a later member whose source placement is neither tight, align64 nor
align64 + 64 after the previous end; non-zero bytes in a gap that the re-plan would re-create; non-zero bytes between the last member and
the tail; a member that would overlap. None triggered on the 129.

## 2. BIG4 + `.bh` outer placement (observed slots, computed outcomes)

For each of the six archives (`big4_outer`), three outcomes with exact patches and whole-file hashes (`expected_big_sha256`,
`expected_bh_sha256`; `.bh` hashes are exact bytes, `.big` hashes streamed with the patches applied):

| Case | `.big` patched words | `.bh` patched words | Preserved |
|---|---|---|---|
| **fit-in-slot** (N = slack; slack verified zero on all six) | payload `[offset, offset+S+N)`; entry size u32 BE at `index_entry_pos + 4` | record `i` word 4 (size) | `size_le`, `index_size`, trailer, all other entries/records, `.bh` trailer, decoded-length word 8 (the stream still decodes to the same length) |
| **append-and-retarget** (N = slack + 64) | entry offset+size at `index_entry_pos` → `(align64(EOF), S+N)`; header `size_le` (LE @4) → new length; payload at `align64(EOF)` after zero padding | record `i` words 0/4; trailer word 0 (u32 BE) → new `.big` length | old bytes left in place; index order no longer offset order (unobserved in stock — card R2-BIG4-1) |
| **ordered reflow** (same N; not computed for the two 2 GB `data0.big`) | entry size; header `size_le` += Δ; every later entry's offset += Δ (2 879 / 7 760 / 9 395 / 9 395 entries on 07 / 09 / 10 Xbox / 10 PS3); tail `[old slot end, EOF)` copied unchanged after zero padding to the new slot end | every later record's word 0 += Δ; trailer word 0 | gap bytes travel with their entries (the tail copy) |

Δ = `align64(S+N) − align64(S)` = 64 on all four. The `.bh` record's word 8 (decoded length, 74 016 / 24 864 / 24 880 on Xbox, **0** on
PS3 where the members are raw) and word 12 (`0x555d4d7f` = djb2 of the path, identical on all six) are untouched. The `.bh` trailer
word 0 equals the `.big` length on 5 / 5 sidecars checked (`bh_trailers`).

## 3. EB `0x00000400` outer placement (observed slots, computed outcomes)

All five logo slots have **0 slack**, so fit-in-slot exists only for `L ≤ S` (same-length or shorter with zero fill). The oracle gives
the append case (N = slack + 16 = 16): row word 0 (block = `align16(EOF) >> 4`), row word 2 (size), header `archive_size` (u64 BE @0x18)
→ `align16(new end)` (the stock rule, 11 / 11 archives in handoff 02 §1.2), payload at `align16(EOF)`, zero pad to the 16-byte end; `w4`
(0 on all 10 EB archives inspected, 133 k rows) and the path hash untouched. Expected hashes per archive in `eb_outer`.

## 4. NHL 07 `boot.big` cascade (observed)

`boot.big` (1 630 060 B, 27 entries, BIG4 with its own `boot.bh` sibling) carries RefPack'd copies of `data0.bh … data5.bh`,
`gamedata/nis.bh`, `gamedata/aidata.bh`: stored 45 706 / 68 315 / 31 567 / 36 404 / 18 452 / 2 060 / 3 869 / 242 B, decoded 55 872 / 81 568 /
38 768 / 44 224 / 22 352 / 2 384 / 4 112 / 256 B, **decode-equal to the loose files 8 / 8**, each followed by **227–2 036 zero slack
bytes** inside `boot.big` (`nhl07_boot_bh`). So a size change in `dataN.big` cascades: (1) patch `dataN.big` + `dataN.bh` (§2); (2)
RefPack-encode the new `dataN.bh` and place it in `boot.big` — fit-in-slot when the stream is ≤ stored + slack (1 398 B of slack for
`data0.bh`, whose `.bh` changes only two words, so a same-algorithm re-encode is expected close to 45 706 B — Studio's encoder is 0.5–4 %
longer than EA's, i.e. +230–1 830 B: **not guaranteed**), else append/reflow inside `boot.big`; (3) patch `boot.bh` (record and
trailer). `boot.bh` is not embedded anywhere, so the cascade terminates at two levels. NHL 08 / 09 / 10 carry no embedded `.bh`
(handoff 02 §1.1) — no cascade.

## 5. Unknown fields — inventory (observed; preserve verbatim)

| Field | Values seen | Rule |
|---|---|---|
| BIGF index trailer (8 B after the last name) | `L277 15 05 00 00` (Xbox 07 / 08 / 10-part, PS3 08), `L281 15 05 00 00` (Xbox 09), `L286 15 05 00 00` (Xbox 10), `L286 15 05 00 01` (Xbox 15 / Legacy, PS3 10-part / Legacy), `L287 15 05 00 01` (Xbox 11 / 12, PS3 10-part / 11) — constant per title, two titles mix two values | copy; never derived |
| BIGF 16-byte tail | per-asset constant (§1) | copy on same-asset replace; unknown for new assets |
| BIGF pre-data gap | zero except PS3 NHL 08 (`42 52 54 53` + zeros, 24 B) | copy |
| BIG4 `index_size` | ≠ table + 8 on 5 / 5: 6–48 bytes between the trailer and the first entry, of which 8 are non-zero on Xbox 07 / 08 / 09 (the text string of handoff 02 §1.1) | copy; `index_size` untouched by every case above |
| `.bh` record word 8 / word 12; trailer | decoded length (0 for raw) / djb2(path); `[big length][12 × 0]` | word 8 unchanged for same-plaintext; word 12 unchanged for same path; trailer word 0 = new length |
| EB row `w4` | 0 on 133 k rows | copy |
| BIG4 gaps | zero on every slot used here; the one non-zero byte in NHL 09 `boot.big` (handoff 02) is outside these slots | a slot's capacity ends at the first non-zero byte |

## 6. Contract for the developer (proposal; policy, not evidence)

Inputs: source archive + `.bh` (BIG4) or EB archive, entry index, the new package bytes (from the inner re-plan) and their length `L`.
Joins: BIG4 entry `i` ↔ `.bh` record `i` (count equal, offset/size equal — refuse otherwise). Placement: fit when `L ≤ S + verified-zero
slack`; else append (structurally valid, unobserved in stock) or reflow (stock-shaped) — the oracle carries both, the choice is the
runtime question of card R2-BIG4-1. Preserved: everything not named in §2/§3; §5 fields verbatim. Reopen checks: re-parse both files,
directory ↔ `.bh` agreement, the replaced entry decodes to the intended plaintext, every other entry's bytes unchanged (hash the
untouched spans), `.bh` trailer = length. Refusals: `.bh` missing/disagreeing; `L` or offsets > u32; a later member rule outside the
table of §1; NHL 07 `dataN.big` growth unless the §4 cascade is planned as one transaction; chunklzx members (no encoder).

**Test oracle:** feed each JSON case's replacement input to the planner and compare the produced files' SHA-256 with `expected_*`; for
ambiguous packages accept either listed variant but record which; a mismatch on the identity case (T' = T) is a re-plan bug, on a
growth case a placement bug.

## 7. Unknowns (explicit)

1. The tight-vs-align64 rule for the record after a raw texture (X-A, P-A) — undecidable from stock; both outputs given. 2. The
per-asset tail word's derivation. 3. Loader tolerance of append vs reflow and of `.bh` disagreement (handoff 02 cards). 4. EA's RefPack
match finder (the NHL 07 cascade fit depends on it). 5. chunklzx (Xbox 11–Legacy) — unchanged.

## 8. Reproduce, checks, cleanup

```
python local/runs/overnight-research-2026-09-20/probes/r10_placement_oracle.py docs/research/overnight-2026-09-20/data/r10-placement-oracle.json [--skip-hash] [--only bigf|big4|eb|nhl07|fields]
```

Executed (read-only): the oracle in full (≈ 12 min; the identity test was run four times while the rules were corrected — tail not
zero, markers at the target offset, the tight/align64 ambiguity); ad-hoc python for the tail census (14 values / 104 packages) and the
tail-hash hypothesis test (0 hits). **Not executed:** any write, any Studio binary, any launch. Scratchpad files `r10-bigf.json`,
`r10-nohash.json`, `r10-full.log` deleted at closeout.

# Receipt 03 — Priority 2-A: BIG4 ordered reflow and the NHL 07 `boot.big` cascade

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`. Nothing staged, committed, pushed, launched or written live; the real archives
under `X:\Games\Emulation\Roms` were read only (the in-memory hash oracle) or copied to
`%TEMP%` (H:) for the route tests and removed. Claim: `dev-p2a-big4-reflow-nhl07-cascade`.

## 1. Plain-language summary

When a replaced logo package no longer fits its slot in one of the old NHL 07–10 archives, the
archive is now grown the way EA's own archives are shaped: the slot grows by a 64-byte multiple and
everything after it moves up, with the directory, the `.bh` index and the length words following
("ordered reflow"). Until tonight the only growth was to append the member at the end and re-point
it — a shape no shipped archive has. The reflow reproduces the research placement oracle
byte-for-byte on all six stock archives (every fit, append and reflow outcome the oracle computed —
16 whole-file SHA-256 pairs), and the real NHL 10 Xbox 360 and PS3 `fe.big` copies were saved,
verified and restored through the ordinary copied-target save with a reflowed member. NHL 07 is no
longer read-only for growth: its `data0.big` index is mirrored inside `boot.big`, so the re-encoded
index is placed there too and `boot.bh` follows — four files, one transaction, restored together.
The review now states the exact placement it predicts and the save does exactly that (a member
previewed as fitting is refused, not silently grown, if the copy changed underneath).

## 2. What users can now do

- Replace a league/team logo package in a BIG4 archive (Xbox 360 NHL 08–10 `fe.big` /
  `data0.big`, PS3 NHL 08–10) with a larger one; the archive keeps the stock shape.
- On NHL 07, replace a package in `data0.big` even when its stored length changes: the save writes
  `data0.big`, `data0.bh`, `boot.big`, `boot.bh` together and restore puts all four back.
- Read, in the review, whether the member fits, reflows (by how many bytes) or — only when the
  gap after its slot carries non-zero bytes a reflow would erase — is appended.

## 3. Implementation

| Component | Change |
|---|---|
| `crates/tdb-archive-big/src/big4.rs` (`9a030f2594a55a93`) | `Growth::Reflow`; `Big4Placement::Reflow { new_offset, old_slot_end, delta }`; the plan is a list of `Segment`s (verbatim source copy / payload / zero fill) that the stager executes and the verifier proves covers `0..new_len` exactly, every copy byte-equal to its source span (shifted), every zero fill zero, every entry where the plan put it (unchanged entries at `offset + cumulative shift`, `.bh` records likewise). Fit-in-slot and append keep their exact prior bytes. Reflow refuses when the grown entry's own gap has non-zero bytes (never erased; append is named as the alternative). `Big4Replacement.growth: Option<Growth>` — a per-member policy overriding the plan-wide one. Several reflows in one plan accumulate; fits between them land at their shifted offsets; appends go past the reflowed EOF. |
| `crates/tdb-archive-big/tests/big4_oracle.rs` (`42b8a83219f509dd`, `--ignored`) | the research oracle (`r10-placement-oracle.json` `big4_outer`): six real pairs bound read-only, source identities checked, each variant planned with the oracle's input (`stock ++ N zeros`) and staged into a hashing sink — **16/16 expected `.big` + `.bh` SHA-256 reproduced** (fit ×6, append ×6, reflow ×4; the oracle has no reflow for the two 2 GB `data0.big`); NHL 07's reflow additionally staged to disk, verified, re-bound in stock order. `serde_json` dev-dependency added. |
| `crates/studio-domain/src/asset_member_edit.rs` (`a7b5a97473f46854`) | `Big4MemberInfo.placement: Big4Placement { Fit, Reflow, Append }` predicted at preview (reflow when the gap is all zero, else append); `Big4MemberInfo.cascade: Option<Big4Cascade>` — NHL 07 `dataN.big`: `boot.big` + `boot.bh` bound under the root, the mirror member (`data0.bh`) located, its stored bytes checked decode-equal to the loose sidecar (the measured 8/8 precondition), slot capacity measured; refusals `sidecar-cascade` when the boot pair is missing / has no such member / is not decode-equal. `big4_growth_refusal` is now always `None` (kept for callers); `big4_sidecar_is_mirrored_in_boot` names the NHL 07 rule. Prediction text states the boot placement rule (fit ≤ capacity, else reflow of `boot.big`, decided from the actual re-encoded bytes at save). |
| `crates/studio-domain/src/copied_target.rs` (`f79e3b5d54955866`) | `Big4GroupInfo.placement` per member and `.cascade`; `big4_member_replacements` gives every replacement its reviewed policy: `Fit → Refuse("previewed as fitting … preview again")`, `Reflow`, `Append`. |
| `crates/studio-api/src/registry/copied_target.rs` (`a13353a992d48eee`) | plan-wide policy is a refusal (members carry their own); `stage_boot_cascade`: after the groups, the staged `.bh` of every cascading group that actually changed is RefPack-encoded (decode-checked), the boot pair re-bound and its identities compared with the reviews', one `boot.big` plan (member policy reflow, fit when it fits) staged and added as `archive-asset-boot` + `.bh`. |
| `crates/studio-domain/src/txn.rs` (`8bb1447495fabfbc`) | qualification item text for reflow. |
| `crates/studio-domain/src/league_logos.rs` (`9e7764a75fb5649b`) | archive note: reflow wording; NHL 07 cascade wording. |
| `crates/studio-api/tests/big4_asset_member_route.rs` (`065d4ad6e93cb1a0`) | synthetic-pair route asserts the reflow contract (grown entry keeps its offset, later entries + `.bh` records shift by Δ with bytes verbatim, stock order kept, Δ a 64-byte multiple); real-archive route asserts shifts from `slot_end`-derived deltas and byte equality of unshifted entries; **new** `nhl07_data0_growth_cascades_into_boot_big_…`: no boot pair → `sidecar-cascade`; non-decode-equal boot copy → refused; then two members (fit + reflow) → four artifacts, `boot.big`'s `data0.bh` decodes to the new loose `data0.bh`, other boot members verbatim, restore ×4 byte-exact, sidecar faults still refuse before planning; the real route copies `boot.big`/`boot.bh` for NHL 07 and asserts the cascade. |

Research consumed: 2026-09-20 handoff 03 (`research-r10-placement-oracle`, `ready`) §2 (reflow patch
set), §4 (NHL 07 two-level cascade, slack), §5 (fields preserved verbatim), §6 (contract). The
tail-copy rule "gap bytes travel with their entries" is implemented as a verbatim shifted copy.

## 4. Checks executed (developer)

| Command | Result | Log |
|---|---|---|
| `cargo test -p tdb-archive-big --lib big4` | 14 passed (3 new reflow cases: gap bytes travel + follow-up fit/reflow on the reflowed pair; accumulation + mixed policies; non-zero-gap refusal + last-entry reflow; per-replacement policy) | — |
| `cargo test -p tdb-archive-big --test big4_oracle -- --ignored --nocapture` | **1 passed**: 16/16 oracle hashes on the six real archives; NHL 07 reflow staged to disk + verified (588 s) | [p2a-big4-oracle.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2a-big4-oracle.log) |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-api --test big4_asset_member_route -- --ignored --skip every_real_big4` | 5 passed (Xbox 08/10, PS3 08/10 synthetic pairs under the reflow contract; NHL 07 cascade synthetic) | [p2a-big4-route-synthetic-pairs.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2a-big4-route-synthetic-pairs.log), [p2a-nhl07-cascade-synthetic.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2a-nhl07-cascade-synthetic.log) |
| `… -- --ignored every_real_big4` (default targets `xbox-nhl10,ps3-nhl10`, copies of the real `fe.big` + `.bh`) | 1 passed: the small logo reflowed (+1 792 / +1 920 bytes), the logo fit; save, verify, restore byte-exact on both | [p2a-big4-route-real-nhl10.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2a-big4-route-real-nhl10.log) |
| `NHL_STUDIO_BIG4_TARGETS=xbox-nhl07 … every_real_big4` (copy of the real `data0.big` 284 MB + `data0.bh` + `boot.big` + `boot.bh`) | see §4a | [p2a-nhl07-cascade-real.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2a-nhl07-cascade-real.log) |
| `cargo test -p studio-api --test big4_asset_member_route --test copied_target --test asset_member_route` | default cases pass | — |

### 4a. Real NHL 07 result

`xbox-nhl07` (copy of the real `data0.big`, 284 526 259 B, 3 490 entries + `data0.bh` + `boot.big` + `boot.bh`):
the marked `leaguelogos/l0` fit its slot (2 990 B in 5 358 + 18); the noisy `leaguelogossmall/l0`
(33 524 B in 10 453 + 43) **reflowed data0.big by 23 040 bytes** (2 879 later entries and `.bh`
records moved); the re-encoded `data0.bh` came out at **45 731 B** (EA's stored copy: 45 706;
capacity 47 104) and was placed **fit-in-slot** into `boot.big` (length unchanged, the other 26
members verbatim); `boot.bh`'s record follows; both installed packages decoded; **restore of all
four files byte-exact**. Staged 95 s, committed 159 s, restored 214 s. `1 passed`.

Not run: the NHL 08 / 09 real pairs (2 GB `data0.big`; the oracle covered their fit/append hashes),
PS3 08/09 real routes; `cargo fmt` / `clippy` (baseline defects); any launch. Runtime acceptance
of a reflowed archive and of the NHL 07 cascade is card **R2-BIG4-1 rev. 3** (MORNING-REVIEW).

## 5. Coverage (matrix 1.4 archive placement, BIG4 families)

| Family | Before | Now |
|---|---|---|
| Xbox 360 NHL 08–10, PS3 NHL 08–10 (BIG4 + `.bh`) | impl+tested — fit / append (append a non-stock shape) | **impl+tested — fit / ordered reflow (stock shape; oracle-proven on the six real archives; real NHL 10 X360 + PS3 route)**; append retained only for non-zero-gap slots |
| Xbox 360 NHL 07 `dataN.big` | blocked by policy (nothing placed) | **impl+tested — reflow / fit + the `boot.big` / `boot.bh` cascade (synthetic; real `data0.big` + `boot.big` per §4a)** |

## 6. Open / omissions

- Append remains in the engine but is reachable only through a non-zero-gap slot; the earlier
  card's append-vs-reflow A/B is no longer offered from the UI on stock archives. If Codex wants
  the A/B kept, a dialog option can select `Append` (not built — no accepted contract asks for it).
- The boot placement (fit vs reflow inside `boot.big`) is decided at save from the actual RefPack
  stream Studio produces (0.5–4 % longer than EA's per the oracle); the review states the rule and
  the capacity, not the outcome.
- `boot.bh` is not embedded anywhere (the cascade terminates); NHL 08–10 carry no embedded `.bh`.
- No DTO changed (`placement_prediction` is text); bindings unchanged.

## 7. Cleanup

Scratchpad edit scripts `big4_reflow.py`, `big4_reflow_tests.py`, `big4_policy.py`,
`big4_route_tests.py`, `nhl07_cascade.py`, `nhl07_registry.py`, `nhl07_test.py` — deleted at
checkpoint. Test temp roots (`nhlms-big4-oracle-*`, the route `Tmp` roots) are removed by the tests.
No processes left.

## 8. Next

P2-A continued: gen7 `chunklzx` decoder promotion against research handoff 40 (2026-09-18) using
the mounted trees; then P2-B/C/D/E.

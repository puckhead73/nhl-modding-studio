# Coordinator review — developer receipt 27

26 September 2026. Continue the same developer session: the next work extends the current corrections and decoder evaluation. No application implementation changed by this review.

## Disposition

The inspected consistency correction preserves stored-value editing and separates it from the existing fitted Overall inputs. The tournament addition is browse-only: `shown_with_league` is display grouping, while `league_code` remains unset and the backend editor refuses unbound tables. The equipment sweep exercises existing database editing; it does not establish asset selection or rendering. Accepted within this bounded offline review scope, with the follow-ups below. No new blocking application defect identified in these inspected paths. Independent execution results are recorded below.

The seven-title consistency cases are representative coverage, not an all-target sweep. The pre-change fingerprints are sums and counts, not per-player byte comparisons. Reading both formula call sites supports preservation of the prior calculation; the PS4 goalie calibration concern remains separate. Do not add +36 globally merely because a mean bias resembles one coefficient's contribution.

## Bounded follow-ups

1. The decoder evaluation log establishes four successfully decoded Xbox NHL 15 announcer picks and a sample-by-sample comparison for **one** pick (offset 16). Only that comparison establishes max absolute difference 1 LSB, correlation 1.00000 and 95.5% identical samples. Narrow the receipt/recommendation summaries accordingly, then extend comparisons across the actual admitted Speex families during implementation. No listening or fidelity acceptance follows from these results.
2. The ledger's “current authority” still calls tournament browsing “implementation-ready” and “not built”, despite receipt 27 delivering it. Update that row, the gap list and next-work order. The current-authority label alone did not finish reconciliation.
3. The consistency import test discards the returned `ImportOp` and asserts only that the field was not skipped. Source inspection shows `push_semantic_int` appends the encoded write after validation, but the regression should assert the target/value and exercise an applied import through save/reopen. This is a coverage improvement, not a reproduced wrong write.
4. Fix the new redundant `.clone()` warning in `goalie_consistency.rs` while touching that test. Existing unrelated warnings remain baseline work.

## Decoder engineering decisions

**Speex: proceed with pinned libspeex source and a small handwritten binding; no new libclang requirement.** Prefer a tiny C shim compiled against the upstream headers that owns `SpeexBits` and decoder state, with an opaque handle exposed to Rust. This avoids manually duplicating C struct layouts or exposing varargs control calls. Keep a safe Rust owner with deterministic cleanup, checked lengths and explicit error propagation. Pin the upstream version/hash, preserve upstream notices, and document the build recipe. If reusing MPL-covered wrapper files, retain their obligations rather than describing all copied material as BSD-only.

Independent facts checked: the locally cached `speex-sys` 0.4.0 `build.rs` compiles vendored C with `cc` and executes bindgen during the build. Xiph documents Speex under its BSD variant ([official licence overview](https://www.speex.org/fsos/)). The log's four-vector decode and one-vector comparison were inspected, not rerun in this review; the evaluation harness was already removed.

**XMA2: proceed with an optional, separately invoked minimal FFmpeg helper for internal development and review.** An undecided Studio licence is not by itself a reason to block this engineering package. This does not choose Studio's licence or authorize publication. Keep the helper replaceable and Studio usable without it; preserve framed/no-preview behavior when unavailable. Build from a pinned upstream release with a recorded configuration, excluding GPL/nonfree options and unnecessary external dependencies. Verify the resulting binary's reported licence/configuration and the actual enabled codec/demuxer/muxer/protocol dependency closure; the recommendation's short component list is not a proven complete build recipe.

FFmpeg's XMA decoder source carries LGPL-2.1-or-later terms ([upstream source](https://ffmpeg.org/doxygen/trunk/wmaprodec_8c_source.html)). Its complete binary licence depends on enabled components ([upstream licence](https://ffmpeg.org/doxygen/trunk/md_LICENSE.html)). Keep corresponding source, modifications, build instructions and notices ready for the exact binary. The [FFmpeg checklist](https://www.ffmpeg.org/legal.html) discusses library linking; do not mechanically equate every DLL/relinking item with a separate executable. Review the actual distribution arrangement before release. The machine's GPLv3 full build is a research oracle, not the selected bundled artifact. Say “no suitable standalone alternative found in the recorded search”, not “only decoder exists”.

## Independent verification

Inspected: `attributes.rs` classification, formula adapter and attribute assembly; `view.rs` field note, display transform and roster formula caller; `import.rs` semantic lowering; `schedules.rs` table grouping, labels and World Cup naming; the `schedule_edit.rs` unbound-table refusal; DTO/convert fields; `LeagueScheduleCard.tsx`; new consistency, tournament and equipment tests; the existing Overall regression tests; receipt 27, decoder evidence log, current ledger and ready research C16 handoff/data excerpts.

Baseline HEAD: `8d60a6d582c1bf208e0a5bc590b28d5add7d5051`, with 879 dirty/untracked entries at review start. No new active developer claim was found. Research C16 is delivered/ready, not blanket coordinator acceptance; its source-specific label discrepancy is an input for the next reproduction.

Genuine NHL 20 fixture independently hashed: `1e2004784bccfdd6bed6e82533a67595798379f2c2d763c0636d9d370977a57b`, matching the encoding-contract pin. It is distinct from the NHL 19 copy under the committed NHL 20-labelled directory.

Independent command (PowerShell): `$env:NHL_STUDIO_FIXTURES='E:\nhl-studio-fixtures'; cargo test -p studio-domain --test goalie_consistency --test tournament_labels --test equipment_gloves_helmets --test overall_formula -- --include-ignored`.

Results: **13 passed, 0 failed, 0 ignored**, process exit 0. Equipment: 2 passed (161.41 s; all 27 anchor directories, 25 with equipment companions, plus the genuine NHL 20 private fixture). Consistency: 4 passed (92.57 s; seven representative title/platform cases). Overall formula: 4 passed (729.90 s), including the pre-NHL-14 gate, NHL 14 PS3, NHL 15 PS4 skaters and later NHL 16/18/21 populations. Tournament labels: 3 passed (6.33 s; NHL 17 World Cup, NHL 18/19/21 CHL/Spengler, ordinary NHL 21 league table control).

Frontend command `npx vitest run src/components/LeagueScheduleCard.test.tsx src/components/AttributesTab.test.tsx` in `app`: **2 files, 13 tests passed**, exit 0. No desktop, listening or game run. No full suite, typecheck, binding-export check, repo-wide formatting or clippy rerun; developer totals for those remain developer evidence. `zz_diag` excluded by explicit target selection.

Hashes for ten inspected source/test paths are saved in `development-20260926-receipt27-reviewed-sources.json`; they were checked again for drift. This is a review snapshot, not a full stable-build source manifest.

Cleanup: the reported `E:\nhlms-target-check` directory was independently absent. The deleted harness and patch-script paths were not fully enumerated in the receipt, so their cleanup is developer-reported. After the consistency/equipment cases finished, no `nhlms-consistency-*` or `nhlms-gloves-helmets-*` directory remained in the system temporary root. All reviewer-started test processes completed. The existing shared Cargo cache was preserved. No standalone reviewer harness created.

## Roadmap

Consistency display/editing, read-only tournaments and skater equipment database parity advanced. Phases 1–2 remain incomplete. Next: close the small evidence/ledger gaps, reproduce and correct title-inappropriate nationality labels from C16, prepare the stable manual-review checkpoint, then Speex and the bounded XMA helper package. Continue goalie gear and independent roadmap work after actionable League/Team work. Projects/Builds remains deferred; NHL 21 E1 remains accepted.

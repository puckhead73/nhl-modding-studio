# Modding Studio completion: ownership and parallel execution

> **17 September 2026:** [Entity editing and cross-game parity](entity-editing-parity-roadmap.md) supersedes the feature queue below. Existing code ownership remains. Projects/Builds are final optional stretch work; they no longer gate direct-editing progress or release. The new roadmap also governs the explicitly added arena/ice/presentation scope.

Recorded 2026-09-16 from the user's direction and a read-only review of both working trees.
This is the current scheduling and ownership plan. The existing
[completion roadmap](project-completion-roadmap.md) retains C0–C10 scope and acceptance criteria.
This plan supersedes its discovery-first sequence, not its release requirements.

## 1. Operating decision

Three primary workstreams, with Codex coordinating priorities, dependencies and acceptance:

| Stream | Write ownership | Deliverable | Explicit boundary |
|---|---|---|---|
| 1. Studio developer | `E:\Repositories\nhl-modding-studio` application, tests, definitions, CI and implementation documentation | Finished Studio workflows | Sole application-code writer; no new head-fitting or conversion implementation |
| 2. Head converter developer | `E:\Projects\Repositories\nhl-head-mesh-converter` | Complete universal converter, then versioned integration handoff | No Studio edits; complete and qualify the standalone converter first |
| 3. Game research partner | Separate Markdown-only knowledge workspace | User knowledge, exact asset locations, format constraints, game exceptions and bounded unanswered questions | No Studio files, converter implementation, registry TOML, generated evidence or game-file modifications |
| Codex triage | Scheduling/architecture records and reviewed integration decisions | Ready work packages, review decisions, unblock requests | Do not become a second concurrent application developer |

Research workspace: `E:\Projects\Repositories\nhl-modding-research`. Agent 3 created
`KNOWLEDGE.md`; Codex subsequently initialized Git on `codex/game-research` after reviewing that
handoff. There is no initial commit yet. Research writes remain confined to this workspace.
Studio consumes reviewed findings by reference; only stream 1 translates them into code and profiles.
The converter developer likewise incorporates relevant findings in its own repository.

### Agent and Git branch assignments

Use **two separate Claude Code sessions for implementation**, with **Codex handling triage,
user-assisted research and independent review**. These are persistent role assignments, not a
request to have every agent edit every repository. Git branch prefixes do not determine which
assistant may own the work.

| Session | Tool / agent | Repository | Assigned branch | Status and write scope |
|---|---|---|---|---|
| 1 — Studio developer | Claude Code, dedicated Studio session | `E:\Repositories\nhl-modding-studio` | `feat/phase5-asset-tooling` | Existing active branch; sole writer of application code, tests, definitions, CI and implementation receipts |
| 2 — Converter developer | Claude Code, separate converter session | `E:\Projects\Repositories\nhl-head-mesh-converter` | `codex/research` | Existing active unborn branch; Claude owns converter implementation despite the historical branch name |
| 3 — Research partner | Codex, separate task working with the user | `E:\Projects\Repositories\nhl-modding-research` | `codex/game-research` | Git initialized; branch unborn pending initial commit; Markdown findings only |
| Coordinator — triage and acceptance | Codex, this task | Studio planning documents; both codebases read-only for review | Existing `feat/phase5-asset-tooling` for explicitly owned planning documents; no separate implementation branch | Owns this plan and roadmap/working-agreement updates; coordinate edits to those exact files with session 1 |
| 4 — optional QA/release reviewer | Codex, separate review task | Isolated candidate copies plus a separate worktree of the proposed knowledge repository for reports | `codex/qa-review` for Markdown reports | Proposed report branch; application candidates stay at recorded revisions/snapshots and receive no fixes from this agent |

Keep the two existing implementation branches for now. Both working trees contain important
uncommitted work, and the converter has no initial commit. Do not switch or rename branches,
create implementation worktrees from HEAD, or cherry-pick partial source into a fresh branch as a
side effect of dispatch. First establish reviewed, reproducible source baselines in place. Later
package branches/worktrees are optional and must start from that complete baseline.

The research repository is now initialized on `codex/game-research`, without a commit. The proposed
`codex/qa-review` branch/worktree has **not** been created. Establish the initial Markdown commit
before making the QA worktree; each session then uses its own checkout.
Never run two sessions that independently switch branches in the same checkout.
Codex research supplies findings to Claude; Claude implements them on its assigned code branch.
Codex QA supplies defect reports; fixes return to the relevant Claude session. This keeps one code
writer per repository while allowing independent progress and review.

The user explicitly authorized reviewing the converter path above. Older Studio guidance saying
that its path and inspection authorization were unavailable is superseded. This does not authorize
reading unrelated repositories or prematurely copying converter implementation into Studio.

## 2. Reviewed state

### Studio

- Reviewed HEAD: `8d60a6d`, the completed C0-AS7 asset-measurement package.
- Working tree is substantial and dirty: 159 tracked modifications and 157 untracked entries in
  `git status --porcelain` at review time. Untracked directory entries are not file counts.
  Preserve this work; a clean worktree from HEAD would omit important existing implementation.
- Database/editing, roster, historical import, asset codecs and multiple asset previews already
  exist. The old phase roadmap is historical, not a new implementation checklist.
- 27 of 28 database registry units exist: 24 attributed partial adapters and 3 representation-only
  entries. The units include a research copy; they are not the 27 shipping targets. Recompiled's
  database adapter remains absent. NHL19/20 PS4 anchor attribution remains unresolved.
- Four of seven asset-bearing families have profiles: portraits populated in limited scope;
  logos, jerseys and equipment represented. Heads belong to stream 2/integration; audio is required
  and unbuilt; arena artwork is stretch. Registry presence does not establish operational support.
- C0-AS7 already measured Legacy render archives, equipment/jersey/logo identity joins, PS3
  portrait inventories and `patch.big`. Repeating these inventories is not stream 1's next job.
- Concrete current defects/limitations: the receipt reports 257 multi-chunk `chunkzip` failures
  among 329 such PS3 patch members; `eb::decode_payload` still refuses LZX; the archive writer
  exposes in-place replacement and full reflow, not the complete C3a streaming transaction.
- Direct code inspection confirms Texture Lab is `ComingSoon` in `app/src/routes.tsx`.
  `tdb-canonical/src/lib.rs` explicitly describes specification/validation only: no Project UI,
  converter or Build engine. Developer Mode execution remains substantial work.
- The overall list/detail inconsistency is still visible: the list calls `skater_overall`
  directly while attributes apply an era gate. By contrast, `EditSession::set_dressed` already
  delegates the corrected slot-status operation. Reproduce old defect claims before assigning fixes.
- CI currently runs on Ubuntu; a reproducible Windows desktop/package gate remains open.

Sources: [current state](../STATE-OF-THE-REPO.md),
[AS7 receipt](../receipts/c0-as7-asset-evidence-measurements.md), current source and Git inspection.
Receipt test totals are historical results, not freshly rerun tests. This review did not execute
the full application, test suite or game acceptance campaign.

### Head converter

- The specified repository exists. Its `codex/research` branch has no commits; source, tests and
  research are untracked. Establishing a reproducible source baseline is an immediate delivery need.
- Existing fitting, baking, native packing, profile, job/event and preview infrastructure is real.
  NHL19 and NHL21 have recorded user-accepted runtime fixtures; that is not universal qualification.
- Latest September 16 Legacy work still has eye/lash geometry blockers. Claude's eye-evidence
  correction reports 216 tests and is pending independent review; readiness flags remain false.
- `src/application/export.py` already has guarded NHL21 native export, so older claims that native
  application export is entirely absent are stale. It still relies on a research handoff, local run
  paths and an experimental packer; this is not a portable finished application boundary.
- Windows GUI/distribution is not complete. Formal schema and dependency/package delivery gaps
  remain. The canonical contract is proposed v0.2 and native profiles are narrower than the proposed
  full head-profile contract.
- NHL26 work in this repository is useful converter/source research; it does not extend Studio's
  accepted NHL07–21/Legacy/Recompiled destination scope.
- Native probes currently depend on Studio crates. Pin/read them without edits and avoid shared
  build-output races; stream 2 cannot patch Studio dependencies to make a converter test pass.

Use current source and latest status/review entries over historical roadmap prose. Old allocation
of native implementation to Codex must not create two competing converter writers.

## 3. Ready work for tonight

These are bounded packages, not a promise that the entire product fits into one evening.

### S1 — Studio: reproducible baseline and concrete codec defect

1. Record current worktree inputs and existing baseline failures; preserve uncommitted work.
   Identify the exact source set needed for the package. Do not reset/clean or blanket-stage it.
2. Reproduce the C0-AS7 multi-chunk failure from its recorded specimens. Fix framing/decoding in
   the existing codec, with synthetic regressions for the encountered layouts and malformed input.
3. Re-run the named fixture cases where available, recording genuine missing fixtures explicitly.
4. Keep encoding, LZX and archive installation limitations separate; a decoder fix does not close them.

Exit: the recorded failures are explained and corrected or reduced to specific remaining variants;
focused tests pass, unrelated bytes/data remain intact, and the change is reviewable.
No broad archive census, new registry schema or live-game deployment belongs in this package.

### S2 — Studio: first complete Texture Lab workflow

After S1, use already identified, attributable fixtures for one target and archive layout.
First establish the C3/C3a shared change-set/apply boundary and its failure/recovery tests; do not
add a one-off direct-write route that must later be replaced.

Deliver archive entry selection -> lazy decode/preview -> export -> import replacement -> validate
exact original dimensions/format and required mip chain -> pending review -> staged copied output
-> reopen -> restore. Include exact archive/entry identity and fit-versus-relocation reporting.
Split implementation into codec, planner/recovery and UI packages if needed; each has a runnable
demonstration and focused acceptance checks. Growth/relocation cannot be omitted from C3a closure.

Use Standard mode without a Project prerequisite. Prove copied-tree behavior before runtime
qualification. Other targets remain explicitly unqualified; this is an internal milestone.
When a target-specific unknown blocks the slice, give stream 3 the exact question and move to a
ready correctness/UI task rather than reopening general discovery.

### H1 — Converter: close or precisely disposition the current Legacy blocker

Review the latest eye-evidence correction and cap-radius/geometry findings before starting another
fit experiment. Freeze accepted NHL19/NHL21 fixtures as regression references. Pick one bounded
repair supported by the current evidence, produce a new candidate and test its quantized/native
output. Do not treat screenshot alignment or structural tests as animation acceptance.

If the geometry gate needs user runtime observation, prepare the exact candidate/checklist and
continue H2 while awaiting it. Do not pile up unreviewed variants or silently promote readiness.

### H2 — Converter: finish the portable application boundary

Remove fixed research-run paths and experimental-only imports from application execution. Define
versioned request/result schemas, dependency versions, explicit supported profiles and portable
input/output paths. Preserve deterministic manifests, cancellation, progress, refusal of unsafe
outputs and actionable errors. Establish a reproducible source/dependency baseline.

Scope universal completion with an explicit source/target checklist: geometry/UVs, rig and weights,
eyes/mouth/hair companions, LODs, texture bake/mips, native packing, holdout heads and runtime cases.
Finish remaining adapters and standalone delivery against this checklist. A successful NHL21 export
or one accepted head is not the universal-converter exit gate.

### R1 — Research partner: capture known facts first

Start with the user's known asset locations and existing receipts. Maintain a compact table of
game/platform/build, family, archive/member or loose path, ID/reference rule, dimensions/format/mips,
patch precedence, exceptions, source of the claim and next unresolved question.

Label facts as user-reported, specimen-measured, runtime-observed or unresolved. Preserve the user's
knowledge immediately; ask for verification only when a concrete implementation decision needs it.
Do not rediscover a known location merely to turn it into another receipt.

Initial questions for the user-assisted queue:

1. Missing known portrait/crest/thumbnail paths and patch precedence needed by S2/C4.
2. The goalie-stick ID mismatch and logo trailing variants already isolated by AS7.
3. Exact existing save/container exceptions and attributable NHL19/20 source copies.
4. Source schedule locations/initialization rules and audio bank/codec/slot constraints for C6/C7.

Each research note ends with the affected feature, exact implementation implication and any remaining
test. Markdown-only scope means no new scanners, exporters, binary fixtures or registry edits in
this stream. Request a bounded tool change from the relevant developer if genuinely necessary.

## 4. Remaining development after the first slice

Stream 1 owns all rows below; work advances on ready dependencies, not a blanket C0 lock.

| Package group | Completion work | Dependency / research input |
|---|---|---|
| S3 / C1 | Reproduce and close database correctness gaps, overall consistency, companions, clone/batch flows, readable edit review | Existing anchors/audits; unknown semantics go to R1 |
| S4 / C2 | NHL15 PS4 framing, v1 integrity, multipart modes/settings, original-console packaging and Recompiled persistence | Exact fixtures/runtime paths; copied output and recovery first |
| S5 / C3–C4 | Complete recoverable DB/save/asset transactions; portraits/library matching, logos, installable curved nameplates, all required texture profiles | S2 foundation and narrowly scoped format findings |
| S6 / C5 | Appearance and same-game equipment completion; later integrate the completed converter | Shared writes; converter readiness and versioned handoff |
| S7 / C6–C7 | Schedule edit/import/generation, localization value editing, audio decode/encode/replace/install | R1 records; validated codecs and shared writes |
| S8 / C8 | Canonical Project execution/editor, reconciliation/invalid-values UI, immutable revisions/Builds, external-mod ingestion, portable packages | Registry semantics and shared transactions; registry declarations alone are insufficient |
| S9 / C8–C10 | Historical/Legends finish, on-demand components, onboarding, Windows distribution, performance/accessibility, complete QA campaign | Finished workflows and stable candidates |

C9 manual cases run as soon as useful candidates exist. Formal evidence activation remains parked
until the feature-complete RC transition, per existing direction. No broad evidence-infrastructure
hardening detour unless a real defect blocks delivery. Required runtime and original-console gates
remain required; arena authoring, league restructuring and cross-game equipment conversion remain
stretch. An early useful workflow is not an early public release.

## 5. Additional agents that will not collide

**Recommended fourth stream: independent QA/release reviewer.** Read a fixed candidate snapshot;
run tests/builds in separate output locations; inspect user journeys, source completeness, generated
binding drift, Windows packaging and reproducibility. Write Markdown defect reports in its own
workspace. Include expected/actual behavior, exact candidate, reproducer and smallest useful fix.
No code fixes, formatting, lockfile changes, shared build-directory races or live installs.
Review stream 1 and stream 2 at their handoff points, not while files are changing beneath a test.

Optional further bounded research packages, if separate sessions are available:

- **Schedules/localization analyst:** required source schedule initialization and localization limits.
- **Audio analyst:** bank/slot/codec map and import/output constraints for C7.
- **Historical data/Legends analyst:** corpus identity/quality exceptions and roster selection acceptance cases.

These are subdivisions of research with separate Markdown files and assigned targets. Do not give
each a duplicate whole-game scan. A fifth application-code writer would break the chosen ownership
model; frontend, codecs, packaging and tests still land through stream 1.

## 6. Handoff and triage rules

- Keep one active implementation package per codebase; maintain a ready queue and a blocked queue.
  A package states objective, writable paths, inputs, non-goals, dependencies and observable exit.
- Every result is a compact handoff: outcome/demo, files changed, checks actually run, unresolved
  blockers and the next useful action. Avoid chains of paperwork-only micro-packages.
- Research issues name a specific consumer and question. Resolved facts are referenced, not rescanned;
  remeasure only for conflicting evidence, a changed build or an operation-specific uncertainty.
- Codex accepts, requests a bounded correction, or parks a package with a named blocker. Independent
  work continues while the user supplies knowledge or runtime feedback.
- Converter contract can be agreed now without integrating its implementation: version/profile IDs,
  canonical inputs, target/build identity, dependency versions, progress/cancel/error behavior,
  output manifest/hashes and validation results. Converter produces artifacts; Studio owns staged
  review, coordinated installation and restore. Define the final native embedding mechanism at
  handoff; an interim process contract does not waive native integration requirements.
- Integration starts after converter completion: freeze a reproducible version and supported matrix,
  transfer regression fixtures/recipes, then stream 1 implements and qualifies the Studio adapter.
  Prior converter runtime successes remain fixture-specific until Studio integration is tested.
- No commits, pushes, installations or production agent launches were performed by this planning
  review. Preserve existing commit-review rules. The read-only converter reviewer was a bounded
  subtask, not a second developer.

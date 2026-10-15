# NHL 21 Legends — overhaul-led Studio development roadmap

Prepared 17 September 2026. Proposed execution roadmap based on the supplied Legends project and the Studio planning records inspected in this session. This document introduces no new content rulings or runtime compatibility claims.

## Purpose and authority

**Sequencing update, 17 September 2026:** the user requires the persistent Project/Build foundation before overhaul production begins. Follow [Projects and Builds foundation](projects-builds-foundation-roadmap.md), including its first complete editing, revision, generation and deployment gates, before starting the content sequence below. Planning and content decisions may proceed meanwhile; temporary manifests are not a substitute for this prerequisite.

Use NHL 21 Legends as the primary real project that drives Studio workflows from historical research through playable builds. For each workflow, complete a small NHL 21 example, make the operation repeatable in Studio, and test its equivalent across applicable games. Scale content only after the underlying operation works.

The Legends [design baseline](<C:/Users/puckh/Documents/ChatGPT/NHL 21 Legends Project/docs/DESIGN-BASELINE.md>) and [lineage register](<C:/Users/puckh/Documents/ChatGPT/NHL 21 Legends Project/docs/FRANCHISE-LINEAGE.md>) remain authoritative for content. Studio's [completion roadmap](project-completion-roadmap.md) and [ownership plan](parallel-completion-plan-2026-09-16.md) retain product scope, implementation ownership, and release gates. This is an application of those plans, not a replacement for them.

NHL 21 PS4 CUSA18182 is the initial target because that is the audited installation. The canonical Project should preserve target-neutral identities, ratings inputs, collection membership and original assets; NHL 21 files are generated target outputs. Until canonical Project execution is ready, maintain versioned input manifests and reproducible prototypes that can migrate into it.

The overhaul's Franchise mode is unsupported. Studio still requires Franchise/Dynasty/Be A GM save and source-schedule coverage where those modes exist. Test that requirement with separate fixtures, not with a claim that Legends supports Franchise.

## What is already established

- Ten collections, C01–C10, plus Ones Legends. Collections are content groupings, not proof of ten playable league containers.
- C01 has 32 current franchises, including Seattle and Utah. Preserve all eight approved project hierarchies and their NHA/WHA service rights. Utah does not inherit Coyotes history.
- C02 includes the 20 approved identities from the seven non-Detroit hierarchies. Seven additional entries form a proposed 27-team working list, not an accepted final count. Detroit remains combined in C01.
- C03 selects actual NHL seasons; its treatment of the revised Dallas, Ottawa and Winnipeg groupings remains undecided. C03 and C04 stay separate; C02 needs its own collection container.
- C05 uses four actual All-Star game squads, including replacements who played. C06 preserves distinct predecessor national programs; C07 identifies country/year/event squads.
- C08 uses WHA club service only. C09 uses NHA service and historically small rosters, with blank placeholders and line changes off. Placeholder safety is unproven.
- C10 membership is selected last. Ones Legends aims toward approximately 100 players, subject to a measured limit.
- Presentation standards, eligibility, ratings, statistical cutoff and several membership decisions remain open.

## Starting evidence and development gaps

| Evidence reviewed | Planning consequence |
|---|---|
| [Database audit](<C:/Users/puckh/Documents/ChatGPT/NHL 21 Legends Project/audit/NHL21-DATABASE-AUDIT.md>): 31 league records, 433 exhibition teams in 533 allocated rows | Reuse existing categories first; track team allocations and relationships separately from playable mode limits. |
| Same audit: 5,192 spare exhibition biographies, split into 4,343 skaters and 849 goalies; separate index and roster budgets | Budget people, playable versions, roster appearances and companions independently. Duplication policy affects capacity. |
| Same audit: shipped 31- and 32-team NHL schedule templates | Test how Season selects and populates those templates; their existence does not establish a working 32-team overhaul. |
| [Jersey audit](<C:/Users/puckh/Documents/ChatGPT/NHL 21 Legends Project/audit/NHL21-JERSEY-AUDIT.md>): 619 spare stock jersey rows; three rows per custom team | Start within measured structures; asset IDs, menu selection and save persistence need separate proof. |
| [Catalog inventory](<C:/Users/puckh/Documents/ChatGPT/NHL 21 Legends Project/audit/NHL21-CATALOG-INVENTORY.md>) | Reuse the existing catalog-to-asset evidence when mapping equipment, appearances and teams. |
| [Localization audit](<C:/Users/puckh/Documents/ChatGPT/NHL 21 Legends Project/audit/NHL21-LOCALIZATION-AUDIT.md>) and [developer handoff](<C:/Users/puckh/Documents/ChatGPT/NHL 21 Legends Project/audit/localization-developer-handoff/README.md>) | Existing-key replacements and added keys have offline structural proofs. Integrate the supplied reference; archive deployment and runtime lookup are still unproven. |
| Studio implementation inventory and September 16 scheduling records | Database, roster and historical tooling already exist; qualify and finish those workflows rather than rebuilding them. Localization/audio and complete Project/Build execution remain development work. |
| [S2-B review](../validation/s2b-review-and-correction-dispatch.md) | The reviewed copied-target transaction package requires corrections before Workspace/API consumers. Its developer receipt is not acceptance of a finished shared write path. |

These are reviewed records, not a fresh execution of their tests. The September 5 repository inventory is older than subsequent implementation receipts. Recheck the active implementation package before assigning work; individual user-reported head demonstrations do not establish whole-target qualification.

## Step-by-step sequence

Each step produces three things: Legends content or a playable experiment, the reusable Studio workflow it exercises, and target-specific verification records. Milestones below are internal development builds. Studio retains its existing full-project release gate.

### 1. Establish the project baseline and tracking registers

**Work:** Pin hashes and paths for the audited databases, metadata, archives, catalogs and localization files; identify patch precedence and the actual runtime source. Record the historical corpus snapshot and Studio source snapshot, including relevant uncommitted work. Establish a disposable target copy and restoration procedure.

Create linked registers for collections/teams, player identities and versions, content decisions, target mappings/capacity, asset assignments, and workflow/parity tests. Keep canonical content IDs distinct from source-database IDs and game IDs. Do not copy complete game dictionaries or private game data into planning documents.

**Studio:** C0, C3, C8 — source discovery, provenance, pending changes, reproducible inputs, Project/Build foundation.

**Done when:** Another session can identify the same inputs, reproduce a no-change output, and restore the test target. The registers distinguish approved design, recommendations and unknowns.

### 2. Finalize identity mapping and roster policies

**Work:** Map every approved C01/C02 identity to corpus aliases and service windows. Resolve statistical cutoff, the three C03 grouping questions, remaining C02 entries, and New England/Hartford treatment. Then define service thresholds, duplicate allocation, short-history teams, player-version rules and historical roster snapshot dates.

Separate a real person from a playable version: an all-time club selection and a specific-season selection may legitimately have different ratings and appearances. Preserve their common historical identity without automatically merging game biographies.

**Studio:** C1, C8 — historical browsing, identity matching, Legends selection, instance registry and repeatable import review.

**Done when:** Candidate pools are reproducible for the pilot teams and every exclusion or override has a reason. Unresolved policies block only the affected collection, not unrelated work.

### 3. Prove the smallest complete editing and deployment workflow

**Work:** Use two existing NHL 21 teams and a small provisional player sample: at least one skater, one goalie, one historical import and one intentionally distinct version of the same person. Edit a recognizable biography/rating and one existing portrait or logo. Apply the coordinated output to a disposable target, play a game, save/reload, restart and restore.

Temporary ratings and assignments are test inputs, not final roster decisions. Progress from a single DB edit and single asset edit to the combined transaction so failures can be attributed.

**Studio:** C1–C4 — fix correctness gaps encountered in actual use; close the active transaction correction package, archive replacement, edit review, restore and Texture Lab workflow. A manual research patch may prove feasibility but does not finish the Studio feature.

**Done when:** The edits survive runtime and reload, unrelated content is preserved, and the same user workflow runs through Studio. Record the first cross-game equivalent immediately.

### 4. Resolve collection containers and mode feasibility

**Work:** Start from the agreed intentions: C01 → NHL, C03 → Alumni, C05 → four All-Star slots, C06 → International. Inventory candidates for the remaining containers without assigning them by name alone.

Run isolated experiments for an inactive team reused, a new team within allocated slack, 32-team NHL Season initialization, separate C02/C03/C04 browsing, substitution, and supported tournament fields. Probe roster minimums/maximums and save persistence. Test NHA placeholders and Ones roster expansion early because they can change the implementation plan.

Use progressively larger Ones lists and boundary tests; do not promise 100 from an ID field width. For NHA, test injuries, penalties, fatigue, goalie changes and automatic lineup management with line changes off.

**Studio:** C1, C2, C6, C8 — capacity preview, collection mapping, validation and readable explanations of unsupported operations.

**Done when:** Each collection has a tested route or a named blocker with a specific next experiment. Runtime-dependent counts remain provisional until tested. If ten accessible collections or 32-team Season cannot be achieved within existing structures, retain the requirement as blocked and bring a concrete alternative for a design decision.

### 5. Calibrate player selection and ratings

**Work:** Decide peak versus accumulated contribution, era normalization, league strength, missing-data handling and attribute conversion. Separate roster selection scores from in-game attribute ratings. Use a calibration group spanning early hockey, multiple modern eras, goalies, defensive players and short-history franchises.

Compare formula output, Studio displayed overall, game displayed overall and gameplay/simulation behavior. Record manual overrides with reasons. Freeze a versioned profile before mass import; later profile changes must produce a reviewable difference report.

**Studio:** C1, C8 — formula profiles, source provenance, per-target attribute ranges, consistent overall display, bulk preview and deterministic recomputation.

**Done when:** The calibration group is accepted, missing evidence is visible, and recomputing the same inputs produces the same result. This does not require selecting every final roster yet.

### 6. Build the C01/C02 roster foundation

**Work:** Start with a small franchise set that exercises the hard rules. Recommended: Dallas, Winnipeg, Ottawa and Utah, plus an unchanged franchise as a control. Complete their identity-specific C02 counterparts, then expand to the full 32-team C01 and approved C02 field.

Populate biographies, ratings, positions, roster membership, lines, captains, numbers, strategies and supported related fields. Check imported player companions, duplicate versions, roster/index capacity and lineup rules before every batch. Give every team at least valid baseline presentation; full art production comes later.

**Studio:** C1, C8 — Legends builder, batch operations, clone/edit, roster repair, import reconciliation and pending-change review.

**Done when:** The full C01/C02 content pass is repeatable, every team loads, lineups work, and roster save/reload retains the intended identities. Season readiness still depends on step 7.

### 7. Complete Season, tournament, schedule and settings workflows

**Work:** Implement C01's tested Season structure, substitutions and playoff route. Establish C06's tournament experience and the supported C08/C09 season formats as membership becomes ready. Edit/import/generate source schedules, create fresh seasons, compare generated fixtures, and simulate through completion and playoffs.

Test custom games-per-team only where supported. Validate membership order, home/away balance, dates, conflicts and reproducibility. Implement the chosen Franchise unsupported messaging route and NHA guidance; only default settings that have proven write paths.

**Studio:** C2, C6 — source schedules, generator/import preview, settings catalogs and save packaging. Separately test new Franchise initialization with normal Studio fixtures to retain product coverage.

**Done when:** Intended modes complete and reload correctly. A changed source schedule affects new games as intended without unexpectedly modifying existing saves. League topology changes remain a separately tracked stretch requirement if needed.

### 8. Establish and apply the historical presentation standard

**Work:** Decide jersey/equipment eras, portrait style, likeness coverage, number/nameplate treatment, team naming and acceptable fallbacks. First finish one modern and one early-era team, including a goalie, before expanding production.

Exercise portrait matching/import, logos and icon variants, uniforms, nameplates, equipment catalogs and assignments, skin/hair/facial-hair controls, and head assets. Validate selection screens, close-ups, on-ice views, lighting and saved-player reload. Budget jersey rows and resolve catalog/asset references together.

**Studio:** C3–C5 — asset library, Texture Lab, shared install, appearance and same-game equipment workflows. Consume the separately completed converter through its reviewed interface; do not start a competing conversion implementation.

**Done when:** A complete team appearance can be built, previewed, installed, reused and restored through Studio with explicit missing-asset handling. Arena database editing remains required; arena art authoring and cross-game equipment conversion retain their existing stretch status.

### 9. Complete localization, interface branding and audio

**Work:** Rebrand collections and Ones Legends, coordinate team names/abbreviations, and place Franchise/NHA guidance in the relevant screens. Start with existing-key edits using the supplied localization reference. Preserve exact keys, placeholders, null/empty distinction and verified encoding/compression rules.

Qualify added keys separately when a new consumer needs them; define locale coverage and fallback policy before mass text production. Check font coverage and real screen widths. Text replacement does not prove new UI behavior or mode registration.

Trace audio banks and assignments; establish a supported sample for menu, gameplay, commentary/PA and arena audio as applicable. Define treatment of missing historical name calls and inspect mismatched retained calls. Do not assume text changes produce spoken names.

**Studio:** C6, C7 — localization editor, catalog-label coordination, audio preview/encode/replace/install and coordinated changes. Novel UI logic or unsupported audio authoring becomes an explicit scope proposal.

**Done when:** Sample text and audio survive actual archive deployment and restart, language behavior is documented, and reusable workflows exist. Basic labels can be developed earlier for steps 4 and 7; this is the full presentation pass.

### 10. Finish the remaining historical collections and Ones Legends

**Work:** Recommended content order: C03 Pinnacle → C04 Showcase → C05 All-Stars → C06 IIHF Legends → C07 Historic International → C08 WHA → C09 NHA → Ones Legends. Select C10 World Club membership last, then produce it through the established pipeline.

Use the resolved policies, actual participants and event/season snapshots. Preserve national-program eligibility and league-specific contribution rules. Revisit the total capacity budget whenever duplicate player versions or additional teams are approved.

Earlier feasibility work for WHA/NHA/Ones proceeds before this production order; difficult mode questions must not wait until their rosters are complete.

**Studio:** C5, C8 — bulk content production, reusable assets, collection filters, import history and complete canonical editing.

**Done when:** Every approved collection has its intended content and tested modes; all deliberate omissions, substitutions and remaining blockers are visible. C10 remains the final team-selection exercise.

### 11. Complete cross-game parity and reproducible builds

**Work:** Finish all outstanding equivalents from every prior step across the 27 shipping title/platform targets. Reconcile the same canonical test content to each target, review losses and invalid values, generate an immutable Build and verify deployment, reload and restore.

Different supported content limits may require different build mappings. Preserve the complete canonical Project; never reduce it to the oldest target's capabilities. The goal is equivalent supported workflows, not an unproven promise of identical NHL 21 modes everywhere.

**Studio:** C2, C8, C9 — target adapters, reconciliation, invalid-value resolution, committed revisions, immutable Builds, external-mod ingestion, portable packages and original-console deployment. Exercise direct editing without Projects as well as Developer Mode.

**Done when:** Every required applicable feature/target cell has its own acceptance evidence. Recompiled requires independent runtime/persistence tests despite its Legacy ancestry. NHL 07 PS3 remains research-only; NHL 24 remains a head source rather than a full-editor target.

### 12. Full overhaul playthrough and Studio release acceptance

**Work:** Run fresh-install/onboarding, complete Season/tournament progression, all-team exhibition loading, long-session/save tests, settings persistence, asset/audio checks, build switching and recovery. Validate Windows packaging, on-demand components, accessibility, performance, upgrades and user documentation.

**Studio:** C9–C10 — finish required gaps beyond those naturally exercised by Legends, including all required save modes and original-console packaging. No missing fixture becomes not-applicable merely to close the checklist.

**Done when:** Legends meets its approved content/mode requirements and Studio meets its full agreed release scope. Internal successful builds do not override the existing Studio release policy.

## Cross-game testing after every workflow

1. Prove the NHL 21 operation against identified source files and a named output build.
2. Repeat it on a representative other PS4 title, PS3, Xbox 360 and Legacy Recompiled where applicable and fixtures/runtime access permit. Include an early schema target such as NHL 07/09 when that workflow encounters a distinct representation. Choose actual fixtures, not presumed interchangeable titles.
3. Expand the check to every applicable shipping target. Representatives detect problems early; they do not certify neighboring titles.
4. Record exact scope: title/platform, feature, operation, source/build hashes, inputs, expected result, automated result, runtime result, save/reload, restoration and evidence location. Include schema/enum/range conversion and any approved omission or fallback.
5. Distinguish untested, missing fixture/runtime, failed, structurally passed, runtime passed and evidenced non-applicability. Map formal statuses to the existing QA contract rather than inventing a replacement ledger.

Run manual cases as useful builds become available; retain results without starting the broad formal evidence-activation campaign currently deferred by the ownership plan. A blocked older target can stay open while NHL 21 content progresses, but remains a Studio completion obligation unless explicitly excluded.

## Immediate work queue

| Order | Concrete next package | Exit |
|---|---|---|
| L01 | Baseline manifest, collection register and source identity mapping for C01/C02 | Every approved hierarchy has source aliases/windows or a specific unresolved mapping. |
| L02 | Short decision session: cutoff, C03 grouping, player versions/duplicates, eligibility and season snapshot policy | Versioned rules sufficient to produce pilot candidate pools. |
| L03 | Confirm completion of the active S2-B correction and finish the first copied-target Studio apply/restore workflow | Reviewed reusable write path suitable for the NHL 21 pilot. |
| L04 | Two-team playable pilot with one skater, goalie, historical import and duplicate-version case | Edit → build → play → save/reload → restore demonstrated. |
| L05 | Container/mode experiment set: 32-team Season, separate collections, NHA placeholders and Ones growth | Feasibility findings drive mappings before bulk production. |
| L06 | Rating calibration profile, then Dallas/Winnipeg/Ottawa/Utah pilot rosters | Accepted repeatable content production method. |

L01/L02 can proceed while the existing developer completes L03. Basic localization development and small feasibility probes can use already established evidence; full content does not wait for every asset feature. Scope decisions are made just before the work that depends on them, not all at once.

Implementation stays with the current Studio owner; head conversion stays with its external owner; research supplies bounded findings; coordination maintains this roadmap and reviews acceptance. No new implementation sessions are dispatched by this document.

## Decisions to take first

| Decision | Recommendation for discussion — not adopted by this roadmap |
|---|---|
| Statistical cutoff | Use one explicit completed-season cutoff plus a versioned source snapshot. Record exceptions rather than mixing rolling data into frozen ratings. |
| Revised C03 lineages | Retain real single-season teams and decide the Dallas/Ottawa/Winnipeg slot groupings explicitly before selecting winners. |
| Duplicate players | Permit collection-specific versions in the canonical model; decide duplication within each playable competition separately. |
| Eligibility and ratings | Keep service eligibility, roster-selection merit and in-game attributes as three distinct rules. |
| Historical season snapshots | Prefer postseason participants for playoff teams and season-end rosters otherwise, subject to resolving depth and replacement policy. This remains a proposal from the baseline. |
| Presentation coverage | Approve a minimum complete team standard before promising a custom face, arena or equipment model for every entry. |

Estimate calendar effort after L04/L05 establish actual deployment and mode behavior. Until then, acceptance gates provide a more useful schedule than dates built on untested engine assumptions.

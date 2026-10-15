# r74 internal review handoff — 2 October 2026

Candidate frozen for review. This session improved working workflows but **did not complete the full authorized roadmap**. Approved unfinished work below is not reclassified as unsupported. User verification records, including NHL21-E1, remain unchanged.

## Fixed and usable

- NHL21 generic team names: the shared resolver assumed `fullname`, while NHL21 stores its native complete name in `teamname`. Names now resolve from the current database with native fallback precedence, preserving the NHL07 thin-table join. The same value reaches lists, selected headings, player/team references, league members, schedules and arena tenants. No dictionary substitution or hardcoded team list.
- League editor crash: loading → detail changed React's hook count. The state hook is now unconditional, and group selection belongs to its project/league. The actual hook exception was reproduced before correction. An NHL21 API regression opens every league, edits a team name and league key, saves and reopens successfully.
- PS4 asset pages: resource lookup now uses the installation's game root instead of its nested database directory. Assets-only imports still work. Relinking updates live discovery; a late response cannot overwrite a different/reopened project session. Project changes remount routed page state.
- UI: compact header remains usable at 900px; team headings use resolved names; league labels use readable wording; lists have accessible search/sort labels, focus states and useful empty states. Logo failures retain their exact reason and offer Retry. Missing-input pages point to Relink. Long provenance and audio diagnostics remain under details.
- The r72 roster-only installer defect was already fixed in r73. This session retained it and independently reran its regression coverage.

## Launch

Executable: `E:\Repositories\nhl-modding-studio\target\internal-candidates\nhlms-release-candidate-20261002-r74\extracted\NHL Modding Studio rc-20261002-r74\nhl-studio.exe`

Archive: `E:\Repositories\nhl-modding-studio\target\internal-candidates\nhlms-release-candidate-20261002-r74\NHL Modding Studio rc-20261002-r74.zip`

Keep the executable beside its bundled folders. No final-candidate desktop launch or game acceptance is claimed.

## Identity and packaging

- HEAD: `8d60a6d582c1bf208e0a5bc590b28d5add7d5051`, plus 1,052 dirty build-input files. HEAD alone does not identify this build.
- Before/after input manifests are identical: SHA-256 `3ca771052f05511e84f74160dafe080beff292fa0d3b022939b0d5e651260e21`.
- Executable: 53,936,128 bytes; SHA-256 `c0a061f77938955e8db70996b0edee8c593985fcdab1870a518f009bd7334a5e`.
- ZIP: 106,611,222 bytes; SHA-256 `c1b615dd6bf35bd854fc21a989f78a788f9558e7f1c8306ed5aac46f0b86383f`.
- `BUILD-RECORD.json` records definitions, frontend, anchors, inventory, helper and FFmpeg source identities. Extracted archive verification passed all 771 manifest entries. All 21 adversarial/refusal checks passed.
- Extracted definitions/baselines and notices passed; direct test execution from `H:\TEMP` verified working-directory independence. The baseline inventory retains 27 targets with NHL20 fresh creation unavailable because its anchor is an NHL19 alias.
- Packaged helper decoded 83 real, read-only vectors with exact declared counts and research PCM hashes; helper fallback-route check passed. This establishes decoder agreement, not audible/in-game acceptance.
- All 8,661 files inventoried in previous numbered candidate archives and trees were byte-identical before/after. Nothing was committed, pushed, uploaded or published.

## Checks and limits

| Check | Result |
|---|---|
| Frontend | 306 passed in 53 files |
| TypeScript | Passed |
| API full suite | 210 passed, 53 ignored |
| Focused package/input suites | 34 passed; overlaps full suite |
| Domain full suite | 537 passed, 3 failed, 66 ignored initially; all 3 corrected assertions passed targeted reruns |
| Bindings | 348 in sync |
| Baseline inventory | Passed |
| Production build | Passed |
| Browser rendering | Core NHL21 pages at 1200×800/900×650 plus 17 routes at 900×650; no uncaught page errors or document-width overflow |
| Whitespace gate | Not clean: generated binding trailing spaces and other working-tree whitespace remain; see `diff-check.log` |

Domain assertion corrections reflect existing scoped nationality/player-style catalogues and registry count, the current precise texture-layout refusal reason, and newly resolved native World Cup names. They retain exact expected values and rejection checks. Historical failing logs remain. Ignored tests were not silently promoted to passes. Compiler/build warnings remain, including unused code/lifetime warnings and a large frontend chunk.

Native inspection reached an application-approval timeout. The browser fallback uses exported real NHL21 API DTOs, with absent fixture commands explicitly rejecting. It proves component rendering and interaction only, not desktop command integration. Core save/reopen and package/recovery evidence comes separately from API tests. Fresh-name checks cover the first 30 teams and detail/list agreement for each of the 26 admitted targets; they are not 27 game-runtime qualifications.

## Remaining work

Approved but unfinished: player bulk-edit UI, Texture Lab, and package adapters for archive members, native-only assets, structural changes, settings, schedules and dictionary growth. This session did not establish that these are all blocked by missing evidence; they remain implementation work. The broader copy audit also remains incomplete.

Evidence/codec blockers remain for external head-conversion integration, unmeasured audio mutation and unregistered texture placements. NHL20 requires independent source evidence; Recompiled runtime and all game/visual checks remain subject to their existing matrices. External release remains blocked by the unrecorded MIT copyright holder and artwork rights/replacement backlog. Internal bundling follows the existing user decision.

See `COVERAGE.md` for workflow-level limits and `SESSION.md` for continuity. No changes were made to personal projects or original game/save data. Owned preview processes and the Vite server were stopped; preview copies/profile and archive self-test scratch were removed. Screenshots, logs, DTO fixture, manifests, stage tree, extracted candidate and ZIP remain for reproduction and recovery.

## Prioritized visual verification

1. Open a fresh NHL21 project: check native team names, search, selected heading and linked player/league/arena names. Rename a team, Save, close and reopen.
2. Open several leagues from an uncached list; edit a supported field, Save and reopen. Switch projects/leagues and check selection ownership.
3. Relink copied PS4 assets in Settings → General → Project. Confirm logos/file browsing update immediately, and missing paths show an actionable error.
4. At a smaller window, check navigation, Pending Edits, scrolling, keyboard focus and Settings details.
5. Recheck roster-only build/install/restore against an owned save copy; keep destination identity and conflict prompts in view.

NHL21-E1 is preserved as historical evidence. Its naming/league/UI and asset-discovery dependencies changed and need a new user verification pass. No prior user check is marked passed by this handoff.

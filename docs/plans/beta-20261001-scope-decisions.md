# October 1 beta — user decisions, 27 September 2026

Current scope addendum to the entity-editing roadmap and completion design questions. These decisions override conflicting older beta/release sequencing statements only for the scope named below. They do not constitute acceptance of any implementation or authorization to publish a release.

## Accepted decisions

1. **League and team creation, cloning and deletion are not intended for the October 1 beta.** Remove these operations and their prerequisite expansion experiments from beta blockers. Preserve the code/research and later scope; existing league editing and supported schedule workflows remain in scope. Do not expand this exclusion to ordinary league/team edits.
2. **Game executable patching is outside the beta and intended for the final release.** Do not implement executable patches or make executable-patching experiments a beta gate. Preserve a final-release backlog for rules/settings that require this mechanism. This does not remove settings safely editable in supported data files from beta scope.
3. **Distribute the FFmpeg-based XMA helper in the release package archive with the other files.** Prepare one self-contained archive containing Studio, its required resources, the helper, exact corresponding source, licences/notices and reproducible build information. No separate user download/build/install should be necessary for admitted XMA previews. This is the packaging decision, not permission to publish or a declaration that distribution review is complete.

**User clarification:** team creation, cloning and deletion are also deferred. Both league and team lifecycle operations are outside the beta; no scope clarification remains pending. Existing league/team editing remains in scope.

## Packaging implementation direction

Prepare a local release-candidate archive with a manifest; keep the executable-relative `defs` and `helpers` layout functional after extraction. Include exact corresponding helper source and applicable linked-runtime notices, not just upstream URLs. Document the FFmpeg origin, version, configuration, modifications (or none), binary/source hashes and build scripts. Provide an accessible third-party notices entry and draft download-page attribution for eventual publication. Audit actual Studio licence/EULA text for conflicting restrictions without inventing or selecting Studio's licence.

The official [FFmpeg guidance](https://ffmpeg.org/legal.html), checked 27 September, calls for matching source, build details and attribution. Its checklist specifically addresses library linking and is not by itself a complete assessment of this separately invoked executable. Review the actual binary and distribution arrangement; do not equate a licence-report string, a subprocess boundary or a bundled source tarball alone with complete compliance. Bundling source in the same archive is the user's chosen delivery route; a separate source-hosting service is not the default blocker.

Do not require a rename of the helper merely because the FFmpeg checklist discusses DLL names. Retain the established executable name unless a concrete technical or attribution need warrants a documented change; ensure its FFmpeg origin is plainly disclosed.

## Current evidence and priorities

Developer receipts 28–40 remain pending full independent review. Receipt 39 reports a review bundle at `E:\nhlms-review-build-r39-2026-09-27`, containing changes through receipt 38. Receipt 40 is not in that bundle. No new desktop, listening or in-game acceptance follows from this update. NHL 21 E1 remains accepted for its established scope; do not describe all prior user acceptance as absent.

The coordinator inspected the update, receipts 32/34/39/40, R123 and the relevant schedule source on this turn. No test suite, helper build, desktop app or game was executed. This is a scope/dispatch checkpoint, not acceptance of all reported test totals.

Source inspection confirms `schedule_edit.rs::merge_by_date` reorders kept rows and rewrites their indices; R123 R2 requires retained values and positions. Do not accept the divergence by inheritance from the older generator. Resolve preservation or refuse the affected generation case before expanding the contract. Also correct the warning that calls a collision between newly generated and kept games “not created by the generator”; the kept game may pre-exist, but that collision can be newly introduced.

Next priorities: bounded schedule preservation/collision review and correction; release-archive preparation; goalie-mask slot inspection under its measured contract; a versioned review build containing receipt 40 and subsequent verified corrections. Independent review and user observation remain separate work, not reasons to idle the development lane.

## 28 September clarification — artwork in internal builds

The user explicitly permits the existing artwork to remain bundled for internal research/team builds: “for now we will bundle the assets with it, to be replaced before external release.” This is an internal-use packaging decision, not a provenance or external-distribution-rights finding. Preserve the assets for current internal work; no replacement campaign or further artwork-choice question is needed now. Record replacement of undocumented artwork as a pre-external-release task. Keep rights entries unrecorded where no provenance was supplied, retain notices, and keep external release eligibility blocked until replacement or separately documented rights resolve it. No actual sending/upload is authorized by this record.

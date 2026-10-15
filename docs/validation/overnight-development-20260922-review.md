# Coordinator review — overnight developer campaign 2026-09-22

Focused review of R2, real GNF evidence, NHL 07/08 write boundaries and kit completeness. This is not blanket acceptance of eight packages. No production code, installation or anchor edited; no game/desktop launch.

## Accepted scope and independent checks

- R2 arithmetic-order correction: validation now precedes tiled-size calculation, which uses checked arithmetic. `cargo test -p tdb-rx2 --lib patch_descriptor_refuses`: 2 passed, including the exact u32::MAX regression. This closes the prior R2 finding offline.
- Real GNF gap: `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test gnf_real_resize -- --ignored`: 3 passed. Hash-pinned NHL 16/21 inputs pass changed-dimension codec/public-route checks and restoration. This accepts that bounded evidence, not every PS4 title/format, packaging, rendered UI or loader behavior.
- `cargo test -p tdb-speech --test ps4_contexts`: 1 passed, 1 real-tree case ignored. No independent seven-title census acceptance is added.

Six existing tests passed, one ignored. One additional reviewer test failed as described below. Full aggregate suites and expensive censuses were not rerun.

## F1 — P2: missing socks become complete spare kits

`crates/studio-domain/src/kit_files.rs`: `shipped_triples` intersects jerseys with pants, adding socks only if `sock_template_is_measured` discovers a recognized sock filename. An absent sock directory therefore makes completeness easier to satisfy. `team_kits` similarly relaxes its completeness criterion when the sock template cannot be found.

Independent synthetic reproduction: create a disposable rendering tree with only `jersey/texlib_0_7_0.rx2` and `pant/texlib_0_7_0.rx2`, no sock directory. `shipped_triples(root, false, 7)` returns the triple; a test expecting no complete spare fails. No real source was written. Existence-only fixtures are sufficient to reproduce this file-listing decision.

The contract and DTO call these complete three-family kits and potential reassignment targets. Missing/unreadable/unrecognized data must instead yield incomplete or unknown evidence, not complete spares. Determine applicable component rules from a measured title/platform contract, separately from observed file availability. Preserve useful partial listings and name missing evidence.

Also audit the two-file sock form: the current resolver accepts whichever sub-index it finds first. Expose all observed components and do not imply an entire pair is present from one file. Do not invent a requirement or selector meaning; where the contract remains unresolved, say so and keep reassignment unsupported.

## Reporting corrections

Receipt 09 says users can see the kit textures, but its continuation section and receipt 08 correctly say no API seam/UI exists. Correct the opening to backend join implemented/tested; user-facing completion is a next package.

The real GNF tests invoke the API route used by Replace; they do not exercise the dialog. Header reproduction does not imply original pixels survived lossy downscale/upscale. Preserve those distinctions.

Receipt 05's statement that a constant column makes per-arena editing meaningless is unsupported. Constancy is observed data, not a consumer result. Folding constants is a presentation choice. Its new card's read-only disclosure is not a backend-wide write prohibition while the existing raw editor remains available; describe actual scope without silently imposing new restrictions on existing editing.

## Late research addendum impact

Read `docs/validation/research-20260921-pm-delivery-review.md`. The current campaign's NHL 07 descriptor support still refuses the table-at-EOF writer, and NHL 08 changed-dimension replacement is not implemented. These are not newly qualified write paths. No direct r60 consumption was found in the reviewed campaign records. Do not discard the independent GNF results.

Before expanding Xbox mipped writes, audit existing support against independent per-level layout evidence. A shipped slot-size oracle, descriptor reproduction or last-nonzero extent does not establish mip placement. Record supported versus unqualified paths; retain independently proven operations. EA-MP3 header reconstruction remains research, not an implementation contract.

## Fresh-session extended dispatch

Continue the existing extended overnight development authorization in a fresh session, with this order replacing its opening priorities:

1. Fix F1 with a permanent red/green regression. Cover no sock directory, empty directory, incomplete two-file form, unavailable/unknown grammar and complete supported layouts. Ensure all kit/spare completeness claims use the same rule and partial information remains visible. Audit unreadable directories as unknown rather than absent proof. Run focused kit tests; correct the reporting issues above.
2. Publish the correction checkpoint, then complete the read-only kit API/DTO/command/UI path on Uniforms. Bind async results to project/team/root context. Show jersey, pants and all measured sock components, missing/unknown reasons and spare candidates. Do not introduce kit reassignment or replacement without its own write contract. Test the registry seam and UI late-result ownership. Expand actual source coverage to NHL 12/13/14 and PS3 as planned, reporting each target.
3. Publish observations_for_research for measured departures (PS4 condition pointer array, NHL 11 sock sub-index, venue class/colour grouping). Do not edit research-owned files or wait for their acknowledgement.
4. Incorporate the late-addendum audit above before any further Xbox write expansion. Continue other independent ready work while mip-layout research is unresolved.
5. Resume the extended roadmap: remaining actionable Leagues parity, then Teams identity/localization/assets/arena/kits/audio, followed by Players, Equipment and Miscellaneous. Prioritize per-operation parity and supported end-to-end workflows. Projects/Builds remains stretch. No routine approval stop between packages; unresolved external dependencies become explicit blockers/cards, not campaign-wide stoppages.

Preserve prior rules: user owns runtime acceptance; NHL 21 E1 stays closed; no launches/live writes/staging/commits/pushes; source/anchor preservation; explicit claim ownership and immutable async/review identity; meaningful tests, no zz_diag; shared caches untouched; only manifest-owned scratch cleanup. Keep receipts plain-language first, checkpoint matrices/STATUS and deferred cards, then continue as usage allows. End with exact coverage, checks, limitations, cleanup and one next task. Do not describe developer checkpoints as coordinator-accepted.

Reviewer temporary completeness test, its disposable input and generated probe build artifacts removed after recording the reproduction. Shared artifacts retained.

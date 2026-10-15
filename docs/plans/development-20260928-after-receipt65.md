# Developer continuation after receipt 65 — source revalidation, then PS4 hair

Continue the same developer session. Read `docs/validation/development-20260928-receipt65-review.md` first. Keep the existing overnight continuation, sole implementation ownership, immutable sources, copy-only write tests and no commit/push/publication/app-launch/game-launch/listening rules. Do not repeat answered scope/artwork/licence-choice questions.

## 1. Revalidate the actual head source at Apply

The coordinator confirms the original F1 backup-deletion scenario is fixed in the inspected source and 11 independently executed Hockey Shop unit tests. A separate source-check issue was found in receipt 63's head route: `ps4_head_catalogue` reuses a catalogue under a cache key of title, install path, `boot.big` length and modification time. `defs_root`/identity manifest and actual content are absent. Apply calls that cached path again, and fingerprints its old catalogue evidence. Different bytes with the same length/mtime can therefore pass with old source evidence. A metadata-collision probe confirmed unchanged keys with changed bytes; a full Registry stale-vault test is required.

Before extending this route to hair, bind the review to actual source content and the relevant identity-definition content. At Apply, revalidate the current member/archive identity from the actual attached source, not merely filesystem metadata or an old catalogue. Preserve efficient parsing caches if useful, but do not let cache hits bypass source validation. Refuse with no write if source identity, target catalogue or relevant definitions changed. Do not widen the accepted vault identity list to make a refusal pass.

Add a focused test on an owned synthetic/minimal source or disposable real copy: warm cache and review; change a relevant vault member while preserving length and mtime; Apply must refuse with player bytes and pending state unchanged. Also cover a changed identity-definition file/root, ordinary unchanged-cache reuse and normal timestamp-changing replacement. Reuse exact source hashes in the plan/evidence; explain any intentional identity granularity. Avoid copying the whole install or repeatedly reparsing the full player DB for every assertion.

Audit the head and hair write paths first. Other catalogue caches with the same shortcut can be recorded and fixed where they participate in a write; do not turn this into a broad unrelated cache rewrite. Claims that “the row changed” must match fingerprint scope: bind the fields and identity that affect the reviewed operation/disclosures, or narrow wording rather than pretending every unrelated row change is covered.

## 2. Finish test scratch ownership

Change the existing test in `studio-domain/src/assets.rs` that reuses `%TEMP%/nhlms_ps3_dds_portrait` to an exclusive per-run scratch directory with a cleanup guard. It must not overwrite a pre-existing directory's files. Test the relevant behavior without deleting the old unowned folder. Keep all six other unowned H:\TEMP folders intact.

In the new Hockey Shop temporary-file code, `write_new` can fail before it creates a file or after creation. Keep those outcomes distinct before removing a path: an `AlreadyExists` error must never result in deletion of someone else's file that appeared after an earlier existence check. Use an ownership-aware helper/error result or file handle; add a deterministic fault test. Do not claim general concurrent-writer protection from this narrow name-ownership fix. Preserve F1's backup-on-failed-restore behavior and honest cleanup reporting.

## 3. PS4 hair reassignment — proceed after source-check corrections

Use research R15 (`docs/research/overnight-2026-09-20/08-players-head-hair-binding-resolved.md`) and the current vault reader. The class is **`ge_nhl_skaterhairtexturepack`**; the typed attribute is **`GenericHairStyle`**, whose value maps to DB `hairid`. Do not call GenericHairStyle the class or infer ids from record order. Admit NHL 17–21 only, with a unique valid value-to-record join in the open title's current identity-checked source. NHL 15/16 lack this measured class and stay refused. Preserve neutral labels where names are not established.

Reuse the corrected reviewed-plan/apply path. Write only the intended hair field as one ordinary reviewed/undoable change; preserve headid, headtexture and unrelated appearance fields. Refuse absent, ambiguous, negative/out-of-range or malformed values, foreign sources, missing fields and stale reviews without partial changes. A stock exception may warrant a warning, not silent normalization. Do not infer color, geometry, helmet compatibility or actual rendered selection from the texture-pack binding alone.

Test genuine NHL 20 and NHL 21 on owned copies: valid existing target, absent and ambiguous target, wrong/stale vault, field range/presence, one apply/undo/redo and copied save/reopen. Exercise the public Registry path and a useful UI case. Record title/platform/fixture identities; the mislabeled PS4 NHL 20 anchor is NHL 19 base. Runtime rendering remains a user observation.

## 4. Continue productively and package at a useful checkpoint

Keep internal-review builds with the current artwork as the user approved; external replacement and the MIT holder value remain their recorded release inputs. Do not start asset replacement or invent the holder. Preserve r64 and earlier bundles. A new versioned bundle should include verified corrections and hair only when ready, with actual dirty-input hashes, drift check, resource/helper hashes and precise receipt inclusion.

Austrian phased generation, board-art replacement and archive re-key remain withheld pending their specific contracts; no silent beta-scope cut and no writer from gallery evidence. Existing league/team creation/cloning/deletion and executable patching exclusions stand; Projects/Builds stays deferred. Consume new implementation-ready work only within its scope, without editing research/coordinator records or messaging/spawning sessions.

Use focused tests with fixtures configured; exclude `zz_diag`, unrelated formatting and blanket rebuilds. Check disk, reuse shared caches without deleting them, clean only owned scratch with resolved containment/reparse checks. Continue independent authorized packages around a blocked observation or missing answer. End with a plain-language consolidated receipt, source-check and recovery evidence, actual tests/limits, bundle inclusion, complete remaining beta blockers, cleanup and one next task. No claim of app/game acceptance or external release readiness from automated checks.

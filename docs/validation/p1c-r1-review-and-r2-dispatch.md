# P1-C-R1 review — one remaining input-validation correction

Claude developer session: continue the same session. Original F1–F3 are addressed by the reviewed changes and focused regressions. One smaller P2 defect remains before P1-C closes. Research P1-BS-R1 continues independently; the scoped settings decoder package remains next in the developer queue.

## Independent checks

Codex ran `cargo test -p studio-domain --test ps4_groups --test membership --test leagues -- --include-ignored` with `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures`: 25 passed, including real NHL 20 and NHL 19 patch identities. `cargo test -p studio-api --test membership`: 4 passed. `npm --prefix app test -- --run`: 97 passed. Catalogue generator `--check --fixtures E:/nhl-studio-fixtures`: 12 rendered, zero skipped, in sync. The index is empty. Existing compiler/dependency warnings were emitted; this review does not claim a fresh clean clippy/full-suite run. No desktop or game launch.

Reviewed witnessed pair derivation, regular/exceptional validation, recorded/structural title selection, and the membership card's current-project check after summary refresh. These address the previous reproduced failures. Structural identification remains bounded to measured catalogue signatures, not proof about every possible future build; NHL 15 patch coverage remains explicitly untested.

## F1 — P2: explicit division silently discarded for a single-group league

`crates/studio-domain/src/membership.rs:442` checks `conference.or(division)`. If conference is supplied and valid, division is never validated. A real NHL 21 in-memory probe selected Boston and Liiga (`SMLIIGA`, own group 21), then called:

```rust
ws.set_team_membership(bos, liiga, Some(21), Some(6))
```

Observed: `accepted=true, bytes_changed=true`; the requested division 6 is silently replaced with 21. This does not create an invalid stored pair, but contradicts explicit input validation and applies a change the caller's supplied values do not describe. The current UI sends omitted values for this case; the backend/API must still reject conflicting explicit inputs.

## Next Claude prompt — P1-C-R2

Continue this developer session. Fix only F1 above: validate every supplied conference and division independently for single-group targets before planning any writes. Omission may still select the documented own-id arrangement. Do not normalize a conflicting explicit value into success.

Add domain and Registry-seam regressions for correct conference plus wrong division, wrong conference plus correct division, omitted conference plus wrong division, and the accepted omitted/matching combinations. On refusal assert bytes, revision, pending edits and undo/redo state unchanged. Include an existing gen7 single-group target to cover the shared path. Preserve R1's witnessed-pair and exceptional-row behavior.

Run the focused affected suites; update the P1-C record and receipt with a short plain-language summary, precise tests and cleanup. No unrelated code changes, launches, installs, image writes, staging, commits or pushes. Stop for coordinator review after this small correction. P1-BS-1 is the next package once P1-C is closed; do not start the defaults writer.

## Cleanup and roadmap

Reviewer reproduction source and its specifically identified executable/debug/incremental/fingerprint artifacts are disposable after this result is recorded. No source fixture was written. Shared build cache and unowned P1-A temporary directories are preserved. The developer's reported deletion of the shared incremental cache was a cleanup ownership deviation, even though regenerable: future cleanup must remain package-owned, or receive explicit coordinator/user authorization for a shared cache.

Phase 1: P1-A accepted offline; P1-C requires only this bounded correction in the reviewed scope. Settings catalogue research is usable, contract corrections run independently, and the four-field decoder fix is queued next. Localization growth and remaining assets/container parity remain ahead; NHL21-E1 remains accepted and closed. Projects/Builds remain final stretch work.

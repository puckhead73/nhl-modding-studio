# P1-C-R2 acceptance — 19 September 2026

P1-C-R2 is accepted for the bounded offline implementation scope. The single-group branch now checks each supplied slot independently before returning a plan. Codex reviewed the fix and PS4/gen7/Registry regressions; the previously reproduced conflicting-division acceptance is covered, with unchanged-state assertions on refusal and accepted omitted/matching forms. No further correction is requested.

Independent reruns:

- `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test ps4_groups --test membership --test leagues -- --include-ignored`: 11 + 4 + 11 = **26 passed**, zero failures or skips, including real NHL 20 and NHL 19 patch fixtures.
- `cargo test -p studio-api --test membership`: **5 passed**, zero failures or skips.
- Reviewed `membership.rs` SHA-256 `241a92e631d0fe3e81476c66c4713b4b9798a438239e1038fe5ff7c2ffc83982`, matching the receipt prefix. Index empty at review.

Existing warnings in `tdb-loc::retext` and `studio-domain::iso_patch` remain. Unchanged frontend, generator, bindings and full suites were not rerun; their prior checks are recorded in the R1 review. No desktop/game launch or live write. No reviewer probe or new build tree was created; shared cache reused.

This closes the P1-C correction sequence for its reviewed offline scope, not runtime qualification or every research gap. Unknown group names/mechanisms and untested NHL 15 patch coverage remain recorded. NHL21-E1 stays accepted and closed.

The overnight developer brief now starts at **D1 / P1-BS-1 scoped settings decoding**, followed by reader, bounded writer and the remaining queue. D0 is a completed checkpoint, not work to repeat. Use a fresh developer session at this package boundary; research's queue remains independent. Both overnight briefs still authorize continuation without intermediate review stops, with new work developer/researcher-verified and pending subsequent independent review.

# Research receipt 8 review — 28 September 2026

**C39 and C40 accepted for bounded read-route implementation with the amendments below.** Speech catalogue/preview and portrait/head previews only; no replacement writer, loose-loader support or runtime acceptance follows. Existing developer sequence remains: Recompiled correctness guards first, missing reads and the already accepted PS3 writer according to the active dispatch.

Independently read handoffs 15–17 and receipt 8, inspected `studio-domain/src/gen7_speech_preview.rs` bank/stream archive routes and `assets.rs::read_eb_member`, and ran `cargo test -p tdb-archive-big --lib chunklzx_payload_is_decoded_and_a_malformed_one_is_an_error`: **1 passed, 0 failed**. The speech reader currently lacks the `game/` root and chunklzx branch exactly as reported. The expected public preview failure remains unreproduced. The shared EB reader decodes chunklzx, but selects the first exact path match; it does not by itself enforce duplicate admission.

No research census or sampled byte comparison was rerun. The 28 speech comparisons, two size-only streams, 24 portrait/head samples and 219 missing texture libraries remain research evidence, not independently measured totals. Confirmed the exact C38 review folder no longer exists. No game files, application source or research-owned records were written; no launches, listening or publication.

## Contract amendments for development

- Resolve the two supported install-root forms deterministically. Ambiguous competing roots or archive members must not silently choose an arbitrary first match. Reject conflicting duplicates, including duplicates within an archive; byte-equal duplicates may be accepted only with explicit provenance and bounded comparison. Do not eagerly hash/copy the two huge streams just for UI provenance.
- Reuse the shared decoder for small members, while preserving bounded streaming for stored `.sbs` members and validating wrapper, offsets and range reads. Never route an entire large stream through the allocating `read_eb_member` convenience function. Corruption and unsupported wrappers must remain errors, not missing/stored fallbacks.
- Loose-first is an application source-selection policy, not measured Recompiled runtime precedence. Label what was read. A partial loose folder must not hide all archive members: specify per-file fallback on absence, coherent bank/stream selection and refusal of unreadable or incompatible sources. Add a partial-tree regression; do not silently mix conflicting sources.
- A missing texlib is a specific readable-index result. Unreadable/incomplete archives remain unknown. A missing mesh/portrait in an unreadable root is not “not shipped.” Preserve texture/mesh distinctions and all observed generic exceptions.
- Historical equality is not a statement about arbitrarily modified attached bytes. Display actual selected provenance; claim matching a reference only when the current source hash is verified. Keep research sample scope and whole-archive historical comparisons separate.

## Loose-file card requires correction before use

The logs provide a useful lead but not loader precedence. Handoff 15's “first trying” language is stronger than failed-open logs establish. Remove that ordering claim unless corroborated by a successful archive read trace.

The card puts A in two guessed cache roots simultaneously, then treats disappearance of a FAILED log line as proof the loader found the file. Neither establishes the claimed result. Exercise one location at a time and treat missing failure logs as inconclusive without affirmative read evidence or an unmistakable visual positive. Stock visuals plus no failure do not prove the screen uses another asset. The chosen profile asset's display role must be established rather than assumed from its path.

Copying the executable does not isolate its shared Documents user-data root. Prefer a verified isolated user-data configuration/profile. If isolation is unavailable, make that constraint explicit and leave the card pending a deliberate user-controlled backup/restore procedure; no research execution. Preserve pre-existing files and all runtime-modified user data, not just newly placed cache assets. Avoid a whole-install copy with the current disk pressure: specify needed space and a safe user-chosen destination before any runtime task. No cache modification or launch is authorized by this review.

## Status

Developer receipt 66 has already received bounded independent review: F2 and temporary ownership closed in tested scope, hair accepted offline, 21 tests passed. That does not close earlier unreviewed work or desktop/game cards. Research should reconcile its current handoff accordingly. The next research prompt and developer supplement capture these amendments.

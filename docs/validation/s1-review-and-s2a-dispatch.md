# S1 review and S2-A dispatch

Reviewer: Codex. Review of Claude's S1 handoff on `feat/phase5-asset-tooling`,
HEAD `8d60a6d` plus existing working-tree changes. No commit authorization in this record.

## Decision

The decoder implementation passes this bounded review. Close the S1 receipt after the attribution
and documentation corrections below and disposition of the pending domain test run. Do not describe
S1 as committed, runtime-qualified or evidence of a production archive writer.

Independent check:

`cargo test --locked --offline -p tdb-archive-codec -p tdb-archive-big --target-dir local/reviews/s1-codec-target`

Result: 38 codec tests passed; 28 archive tests passed, one ignored; doc tests passed (zero cases).
Separate build output avoided the developer's running domain suite. The existing mounted-fixture
archive test can early-return; its reported pass is not new real-fixture qualification.
Reviewed codec file SHA-256: `419fc62d5a626b6b1bb0aaf5ea8b66b5a6ae680464773eee4b88eacc4c3bfe3d`.
The six-archive specimen totals remain developer-reported; Codex did not repeat that corpus run.

## S1 closure corrections

1. **Correct PS4 attribution.** The newly checked `CUSA18182` boot/overlay corpus is NHL21,
   established from its SFO in `docs/receipts/c0-por1-portrait-measurements.md` section 1.2.
   S1's result paragraph and new codec/test comments incorrectly call it NHL19. Correct those
   claims. Preserve the separately attributed older NHL19 Eichel alignment history, explicitly
   distinguishing its unrerun specimens from this package's NHL21 corpus. Do not infer complete
   build attribution or pristine status from this correction.
2. Replace `STUDIO_DOMAIN_TESTS_RESULT` with the actual result when available. Attribute baseline
   failures/skips separately; a demonstrated new regression must be resolved before closure.
3. Reconcile adjacent stale codec documentation: an encoder exists (unqualified for game writes),
   the padded word is a flag rather than an unused gap, and the legacy helper supplies
   `FLAG_DEFLATE = 1`, not the error-field comment's claimed zero. Restore UTF-8 punctuation in
   this file where the current diff contains mojibake; avoid repository-wide formatting.

## Architect answers

**Q-S1.1 — yes, in the next bounded encoder change.** New deflate output should use the measured
flag `1`; keep reading historical flag-zero output. Add an assertion on encoded header bytes and
independent decode checks, rather than only a writer/reader round trip. The flag correction alone
does not qualify the writer's single-chunk sizing, empty input, other header fields or native loader
acceptance. Keep unchanged native payloads byte-preserved; never recompress solely to normalize flags.
This is authorized implementation scope before using the encoder in staged replacement tests.

**Q-S1.2 — yes.** Add a small explicit private-fixture regression set: the two stored-tail members
and a few PS3 multi-chunk members covering distinct alignment residues. Use `NHL_STUDIO_FIXTURES`
and the existing explicit-selection conventions. Selected missing/unreadable fixtures must fail;
default ignored cases are not passes. Keep binaries outside Git, original sources read-only and
record exact archive/member identity, stored hash, expected decoded size/hash and oracle provenance.
Reuse existing fixture-root configuration; do not set global environment variables or invent a live
game-root fallback. If the root is not provisioned, land the test contract and record execution as
blocked while continuing independent work. No broad fixture census or QA activation is needed.

## Next authorized package: S2-A — archive planning and staged output

After the closure corrections, continue directly to this package; another approval round is not
required merely to begin it. Fix any genuine S1 regression first. Stop at the S2-A review handoff;
no commits, live installs or automatic application to a user's archive are authorized.

Objective: one bounded C3a replacement planner and streamed staged artifact, using the observed
PS3 Legacy `patch.big` layout as the initial specimen, with synthetic failure/regression fixtures.
This is a shared transaction building block, not completion of Q06 or all C3a/PS4 requirements.

### Implementation boundary

- Own archive planning/writing in `tdb-archive-big`, the small encoder correction in
  `tdb-archive-codec`, their focused tests and a compact receipt. Add a narrow domain boundary only
  if necessary for the existing transaction architecture; explain it in the handoff.
- No Texture Lab UI, generated DTOs, registry-schema changes, live deployment, LZX or geometry work.
- Reuse existing parsing and verified fields. The current whole-buffer writer is `EbFile`, not
  `EbWriter`; its `replace_payload` reflows subsequent payloads. Do not use full reflow as the
  production growth strategy or claim that wrapper satisfies C3a.

### Required behavior

1. Bind source archive identity/hash, layout, entry index/path and original span/hash to each
   replacement. Reject ambiguous identity, overlapping/shared spans and unqualified layouts.
2. Report no-op, fit-in-slot or aligned append-and-retarget. Preserve unrelated entries, original
   padding/opaque regions and old payload locations. Growth appends; unrelated entries do not move.
   Use checked 64-bit arithmetic and check on-disk representability before creating output.
3. Coalesce multiple entry replacements against staged state. Refuse duplicate/conflicting edits
   until their semantics are explicit; account for each appended span deterministically.
4. Stream a new artifact from the source without mutating it. Preserve exact no-op bytes, including
   padding and EOF. Reparse output, decode changed entries and compare expected decoded content;
   verify untouched entries/ranges. Do not treat successful decode alone as output correctness.
5. Record exact proposed row/header edits. Unknown nonzero size/integrity fields remain a blocker
   for deployable output; do not assume preserving a suspected checksum is correct. The PS3 initial
   layout does not qualify PS4 20-byte rows, 64-byte alignment or all metadata interpretations.
6. Exercise fit, growth, multiple edits, no-op, aliases, malformed spans, overflow, source conflicts,
   truncated writes and decode/content-verification failures. Use a sparse/synthetic large-offset
   case where practical to validate offsets beyond 4 GB without copying a real large game archive.

### Recovery boundary

S2-A ends with a validated staged artifact and explicit handoff information for transaction apply:
source/output hashes, changed entries/ranges, original EOF, validation result and required recovery
inputs. The source is untouched on every failure. Do not implement a private one-off install swap.

The subsequent S2-B package owns durable apply/restore and integration with the unified DB/save/asset
change set. A range log plus an intact original is not by itself a crash-recovery protocol, and
truncating a temporary file is not proof of recovery across an installation commit boundary.

Patch-versus-loose priority remains unresolved. Explicitly selecting and staging an identified
archive entry does not require deciding runtime precedence. Carry that uncertainty into later install
qualification rather than blocking the planner or inventing a winner.

## Handoff

Report closure corrections, fixture execution status, planner/staged-output demonstration, changed
files, checks run, layout/integrity blockers and the exact S2-B interface. Preserve the dirty tree;
do not stage unrelated work. Baseline cleanup and evidence-infrastructure expansion are out of scope.

# S2-A review and bounded correction dispatch

Reviewer: Codex, Agent #1 task manager. Reviewed `feat/phase5-asset-tooling` at
`8d60a6d` plus the existing dirty working tree. Index empty at review. No application
source changes, staging, commits or installation performed by this review.

## Decision

S1's codec/encoder/fixture closure passes this bounded review. The recorded domain
suite result is developer-reported; it was not independently rerun here.

S2-A needs the two corrections below before acceptance and S2-B dispatch. Its
fit/append architecture is retained. Do not restart the implementation or expand
into UI, deployment, LZX, broad discovery or evidence infrastructure.

## Independent checks

`cargo test --locked --offline -p tdb-archive-codec -p tdb-archive-big --target-dir local/reviews/s1-codec-target`

- Archive: 45 passed, 1 ignored.
- Codec: 45 passed, 6 ignored.
- Doc tests: passed, zero cases.
- With process-local `NHL_STUDIO_FIXTURES=E:/tmp/nhl-studio-fixtures`, explicitly
  selected codec ignored tests: 6 passed. These are actual fixture executions.
- Existing mounted archive tests may early-return; their pass is not independent
  specimen qualification. The four full-archive demonstrations remain
  developer-reported; this review did not repeat them.

Reviewed SHA-256:

| File | SHA-256 |
|---|---|
| `crates/tdb-archive-big/src/stage.rs` | `66956871e10974fb09e91effc41023836668ce2a7499a9c866867e5b750c9ed1` |
| `crates/tdb-archive-big/src/stage/tests.rs` | `10c85cb58e28c6f4319e36b7335cf0245757e934e285068b962e3d16d9a7e270` |
| `crates/tdb-archive-codec/src/lib.rs` | `a5a1951c7d25b0ac8313bca9f08324f3859cfb7032020b3b273c781aa730804b` |
| `crates/tdb-archive-codec/src/fixtures.rs` | `65894f2b36fb947a9885ea672b5d4da73aec5f7b2ec62da8b092602f13854c27` |

## R1 — reject malformed table layouts before parsing (P1)

`BoundSource::bind` at `stage.rs:465` trusts `EbHeader::table_end()`, which sizes
the name/directory region without proving that the fixed file table fits or that
the tables do not overlap. It passes that short prefix to `EbArchive::parse`,
whose file-row reads index outside it.

Independent reproducer: 64 zero bytes, with magic `EB\0\3`, file count 1,
version `0x400`, name-table offset 32, name slot 3, directory slot 1, directory
count 1, and archive size 64. Binding panics at `eb.rs:501`: prefix length 49,
index 49. This is malformed-input handling in the new staging entry point,
even though the unchecked parser helper predates S2-A.

Runnable review-only harness:

`cargo run --offline --manifest-path local/reviews/s2a-probe/Cargo.toml --target-dir local/reviews/s1-codec-target`

The harness catches the panic and prints it; its zero exit is not a passing
acceptance check. Do not copy this catch-and-report behavior into production.

Required correction:

- Validate checked header/file/name/directory table ranges and their supported
  ordering/non-overlap before slicing, allocating a prefix, or invoking the parser.
  Check file bounds and integer conversions. Include the fixed file-table end.
- Apply equivalent protection to the verification reparse path. A malformed
  staged artifact must return an error, never panic or request an allocation
  based solely on unchecked header claims.
- A narrow shared parser/header validation repair in `eb.rs` is authorized when
  it avoids duplicated checks; preserve supported existing readers.
- Add regressions for the exact reproducer, truncated file tables, overlapping
  metadata tables, and header claims extending beyond the input. Retain valid
  fixture behavior. Errors must occur before an output is created for invalid
  source layout.

## R2 — make unresolved qualification visible to the consumer (P1)

`stage.rs:640` creates blockers only for row `+4` and `+12`; `plan` copies them
unchanged at line 924. Thus the real patch archive and recompressed staged
outputs report an empty `deploy_blockers` list, despite the receipt explicitly
carrying unexplained nonzero header `0x10` and unqualified native chunk framing.
The proposed S2-B interface exposes no separate affirmative qualification state.

The original dispatch explicitly requires unknown nonzero size/integrity fields
to remain a blocker for deployable output. Preserving an unexplained header word
does not establish that it remains correct after payload/table/length changes.
An empty row-layout blocker list cannot communicate game acceptance.

Required correction:

- Carry unresolved header/integrity qualification into the machine-readable
  staged handoff, and carry native payload encoding qualification where relevant.
  Either complete the blocker model or separate structural findings from an
  explicit deployment qualification state. Unknown must not read as qualified.
- Keep structural staging usable while deployment is blocked. Do not invent
  header semantics, assume the single-chunk encoder is accepted, or require a
  runtime experiment to finish this correction.
- Ensure the proposed S2-B consumer contract requires affirmative applicable
  qualification as well as structural verification. Synthetic/copied-tree
  transaction testing must remain possible without granting live-game eligibility.
- Add focused tests proving unresolved qualifications survive plan, stage and
  handoff; update the receipt's empty-blocker language and proposed interface.

## Small documentation correction

Repair the broken fixture-root rendering in S1 receipt section 10 (the displayed
`E:\tmp\nhl-studio-fixtures` path is split after `tmp`). This does not reopen S1's
codec acceptance. Keep historical proposals clearly subordinate to the current
dispatch; they do not authorize reflow or an install swap.

## Active assignment: S2-A-R1

Implement R1 and R2, focused tests, and the small receipt correction. Writable
implementation scope remains the archive crates and their focused tests/examples;
receipts may be updated. Codex owns this review/dispatch record. Preserve all
unrelated uncommitted work and the assigned branch. No staging, commits, pushes,
UI, LZX, live installs, or S2-B implementation.

Return a compact handoff with each correction, actual checks, revised machine
handoff/qualification semantics, and remaining blockers. Stop for review.

S2-B stays next in the queue: shared durable apply/restore on copied targets,
with a persisted pre-image or intact backup, journal/commit ordering, restart
recovery and conflict handling. Old fit-in-slot bytes must be saved before any
future overwrite; their old offsets alone cannot restore overwritten data.
Final transaction scope and acceptance tests will be dispatched after this review
closes. No one-off installation path is authorized.

# Coordinator review — R75–R78, 2026-09-23

Focused document/queue review; the 48-vector decode and archive/horn censuses were not independently rerun. No source writes, launches or application changes. Existing H1, diagnostic-audio and read-only-venue boundaries remain.

## Useful developer inputs

R76 reconciles PS3 kit counts under explicit counting rules and records `chunkzip` for the measured members. The developer should correct the `chunklzx` receipt/blocker wording. This removes that specific claimed encoder dependency, not every requirement for archive replacement: inspect the actual codec/placement/transaction path before proposing a bounded write package.

R77 is a substantial diagnostic-decode result on six measured play-by-play sets: X360 NHL 15/Legacy; PS3 NHL 11/13/15/Legacy. It does not yet qualify all titles sharing codec 0x05, announcer banks, music/sfx or stereo. Start/discard handling remains open. Decoder acceptance and sample counts are useful evidence, not audible or game-equivalent validation.

R78 supports exact existing loose-file discovery with per-title residuals. Horn/chant presence must remain separate from what the game selects or plays.

## Corrections to publish before widening the contract

1. R77 §4 and queue row 9 extend applicability to PS3 09–13 and, in §4, announcers, although §3 explicitly calls announcers untested and §1 covers only the six sets above. Narrow current applicability to measured tuples; release additional coverage only after the next survey. Parse field gates are not a substitute for title/bank coverage.
2. R77 summary says the first record carries PCM; table and parser contract say the second EA-frame of block 0. Reconcile to the measured position. State sample-count arithmetic with units (`granules × 576`), and pin the public reference to an immutable revision/URL and fetched-file hash where recoverable. Distinguish correcting the researcher's initial reading from proving the upstream decoder wrong.
3. R78 says city/arena ids "match nothing" and are "not sfx banks", but its own table records many numerical hits. State no established binding from these tests, report those coincidences and scope the searched families. Lack of a direct loose-file match does not identify the alternate consumer. Preserve the exact-match horn listing; do not invent sources for residuals.

## Same-session extended continuation prompt

Read this review and current repository instructions/claims. Publish these narrow corrections first, retaining all raw evidence and dated notes. Continue without a routine review stop.

1. Survey announcer, music and sfx codec-0x05 sources by title/platform/rate/channel/container before applying R77. Use independent vectors, exact source identities and per-class results. Keep unsupported stereo/headers/PCM placement explicit. Add malformed/truncated-input and reservoir/count checks to the diagnostic parser contract. Do not infer coverage from successful ffmpeg exit alone or force nonmatching variants into the play-by-play grammar.
2. Search `sfx.abk` and archives for unresolved horn ids. Publish exact paths/records, joins and alternatives; separate absence in inspected sources from global absence. Keep city/arena/chant mappings candidates unless discriminating evidence establishes them.
3. Publish a developer-facing R76 correction claim: PS3 12/13 kits are chunkzip-wrapped, and the prior chunklzx encoder blocker is inapplicable to these members. Let the developer assess the complete copied-target replacement path; research does not authorize a live write or automatically qualify an encoder.
4. Continue remaining actionable League audio/asset blockers, Teams arena/kits/audio, Players, Equipment and Miscellaneous in established parity order. Coordinate with active claims; work independently when another lane owns a dependency. No repeated broad census where a targeted discriminating test suffices. Projects/Builds remains stretch.

Retain the extended-session rules: usage-aware checkpoints; no launches/listening/live source writes/staging/commits/pushes; research-owned paths; runnable probes, exact scope/residuals and source provenance; manifest-owned cleanup only; no shared-cache deletion. End with plain-language summary, developer queue, actual checks, limitations, cleanup and one next task. H1 still does not enable a full-chain writer; accepted NHL 21 E1 stays closed.

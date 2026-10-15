# Receipt 66 coordinator review — 28 September 2026

## Scope and disposition

Reviewed the current source for vault reading/identity, Registry Check/Apply/cache reuse, the hair catalogue and plan, fingerprint scope, temporary-file creation/cleanup and focused regressions. Receipt 66's narrower claims are appropriate: content validation per call, no concurrent-writer guarantee, and hair texture-pack membership rather than observed rendering. Independent execution results are recorded below and in `development-20260928-receipt66-evidence.json`.

F2's metadata-only cache bypass is corrected in the inspected route: current member bytes and identity-definition bytes are read and hashed before the cache is consulted. Title validation occurs before reuse. Parsed data is reused only for the same content identity. The review binds written cells, source hashes, evidence and disclosures; unrelated player fields are intentionally outside it. The four decompressed relevant members are hashed, not every byte of boot.big; unrelated archive changes are not claimed to invalidate a review. The interval between source reading and database mutation remains outside the tested concurrency scope.

The temporary-name ownership correction distinguishes failed creation from failure after creation. The former cannot cause cleanup of the competing file. This closes the reported AlreadyExists cleanup gap within its stated per-call scope; it does not establish general protection from concurrent path replacement. Original F1 backup preservation remains covered by the unit regressions.

PS4 hair uses the title's typed catalogue, refuses invalid/missing/ambiguous targets, writes hairid alone and preserves head fields. This is an offline existing-style reassignment contract for NHL 17–21. Genuine NHL 20 and NHL 21 integration coverage does not constitute per-title desktop or game qualification of 17–19. NHL 15/16 are explicitly excluded from this operation. The 86 catalogue styles versus research's 54–59 values used by player rows are different counts, not evidence of a contradictory join.

## Bundle and evidence limits

Independently streamed the r66 ZIP: one root, no duplicate filenames, manifest membership complete, all 619 manifest hashes match. Existing extracted files also match those hashes. ZIP contains 620 files including the manifest. Archive SHA-256 is `7072c276c8ebd14f8f23256e090ef6f3dd0588a3948446101ad5546f306f73b5`; extracted executable is `483b498d941d476048fde56af320121499900c0064bf1e0af77dac0d982d8fbb`. Record is `internal-review`, `release_ready=false`.

No fresh extraction, full verifier/selftest, PE imports, licences, build-input drift, full frontend/workspace suite, Windows CI, app launch, listening or game test was independently rerun. The developer's checks remain separately attributed. No blanket acceptance of earlier unreviewed receipts is implied. Internal artwork authorization is unchanged; holder value and external artwork replacement remain release inputs.

## Next work

Continue with `docs/plans/development-20260928-after-receipt66.md`. Recompiled Hockey Shop guard and containment disclosure precede missing read routes and the PS3 writer. The PS3 C37 implementation gate was already lifted by the research-receipt-7 review; receipt 66's blanket board withholding is stale. X360 board gates, archive re-keying and Austrian phase constraints remain.

Read-only mask/gear cache freshness remains recorded; it is not the F2 write-path defect. Prefer shared content validation as those displays are touched and prohibit later writes from trusting those cached results. No broad cache refactor is required before the higher-priority corrections.

Disk free space was 2,816,552,960 bytes during review. Tests used existing caches and owned copies. No user bundle/cache/toolchain/old scratch was removed, no installed source was written, and no application code or research-owned record was edited. Coordinator produced review/evidence/dispatch documents only.

## Independent execution completed

- Hockey Shop unit regressions: 13 passed, 0 failed (0.12 s).
- PS4 hair public-path integration: 1 passed, 0 failed (128.86 s), genuine NHL 20 and NHL 21 copies.
- PS4 vault revalidation public-path integration: 1 passed, 0 failed (69.60 s), including metadata-preserving decodable and undecodable mutations and definition changes.
- PlayerContainmentCard UI: 6 passed, 0 failed (21.60 s total runner duration).

Total: 21 passed, 0 failed. Existing Rust lifetime/dead-code and Vite deprecation warnings appeared. F2 and the temporary-name ownership finding are closed for the tested scope. Hair reassignment is accepted for the bounded offline contract above; user desktop/game cards remain outstanding.

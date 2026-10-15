# Evidence — overnight development campaign, 20 September 2026 (retained logs)

Moved here under [OVERNIGHT-DEV-20260920-R1](../../overnight-dev-20260920-r1-corrections.md) F3
from the developer session scratchpad the campaign's final receipt named
([receipt 10 §8](../../../receipts/overnight-development-2026-09-20/10-final-closeout.md)). The
files are byte-identical to the scratchpad copies (SHA-256 compared on both sides before the
scratchpad scripts were cleaned); nothing was edited, trimmed or re-run. They are **developer
evidence** (the campaign's own runs of the fixture-gated sweeps and the closeout aggregates), not
independent acceptance and not game verification. Test-exe hashes and timings inside the logs
refer to the working tree of that session, which has since changed.

Fixture inputs: the private root `E:\nhl-studio-fixtures` (`NHL_STUDIO_FIXTURES`) — regenerable by
the committed extractors, not in the repository. No log contains a private path outside that root.

| File | SHA-256 | What it records | Consumed by |
|---|---|---|---|
| `logs/c2_real_ps4.log` | `5bac8d97…76b0` | `studio-api --test asset_member_route every_real_ps4 -- --ignored`, one run per title: **NHL 16, 17, 18, 19, 20, 21** (six `=== <title>` / `exit 0` blocks; a logo and an imagery member placed in the real `inxp_10.big` copy, copied-target save, byte-exact restore). NHL 15 is **not** in this log — its run is recorded in receipt 03 §4 only (foreground, 1 passed in 219 s). | receipt 03 §4 ("7/7" = these six + the receipt-03 NHL 15 run); receipt 07 §5; receipt 10 §4 |
| `logs/c3_real_gen7.log` | `507bd93b…df9f` | `studio-api --test gen7_asset_member_route … -- --ignored` on the real EB `cache.big` / `fe.big` copies: **Xbox Legacy, PS3 Legacy, Xbox 11, 12, 13, 14, 15, PS3 12, PS3 15** (nine blocks, all `exit 0`). | receipt 03 §4; receipt 08 §5; receipt 10 §4 ("gen7 EB 9/9") |
| `logs/c4_real_big4.log` | `347566cd…72a2` | `studio-api --test big4_asset_member_route … -- --ignored` on the real BIG4 + `.bh` pairs: **Xbox 10 (exit 0), PS3 10 (exit 101 — the binder defect fixed in C10), Xbox 09, PS3 09, Xbox 07 (planned refusals only), Xbox 08, PS3 08** (all others `exit 0`). | receipt 09 §5 (the defect and its measurement); receipt 10 §6 |
| `logs/c4b_real_big4.log` | `a4653dc1…1087` | The **PS3 NHL 10** rerun with the corrected binder (`exit 0`, 1 passed in 218 s). With `c4_real_big4.log` this is the "BIG4 7/7" of receipt 10 §4. | receipt 09 §5 |
| `logs/c9_real_teamlogos.log` | `62bb9208…a343` | Team-logo archive placement on the real Legacy `cache.big`: **Xbox Legacy, PS3 Legacy** (two blocks, `exit 0`; `t10.big` fit-in-slot). | receipt 08 §5 ("Legacy team logos 2/2") |
| `logs/closeout_tests.log` | `3a4708f9…aff4` | The closeout aggregate over the six touched crates in default mode (08:07–08:21): `tdb-archive-big` 69 passed / 1 ignored, `tdb-archive-codec`, `tdb-loc`, `tdb-rx2`, `studio-api` all `exit 0`; **`studio-domain` `exit 101`** — the `tests/asset_member_undo.rs` compile failure receipt 10 §6 records (the C7 `whole_group` parameter), so this log's studio-domain section is **not** a pass. | receipt 10 §4 |
| `logs/closeout_domain.log` | `77de06c9…872b` | The `studio-domain` rerun after that fix: 53 `test result` lines, **386 passed, 0 failed, 20 ignored** (default mode). | receipt 10 §4 |

Full hashes:

```
5bac8d979153785720226bda193f5d5146cae0def9c4834c7e453d8db18376b0  c2_real_ps4.log
507bd93b5d27dfbe88476e07e51ae9e61042f386c149cb85307783caaadffd9f  c3_real_gen7.log
347566cdae8b0b85965d2dce44a8e38edc47f69d8728ee551a2f6105ad0472a2  c4_real_big4.log
a4653dc171b530e0b6ebd8a58a653e3281089121a30a1a8f18fd0a8fe331e087  c4b_real_big4.log
62bb920870e7d47201f2f283337ca49f280000f941c3cc24cf2dfc6fc044a343  c9_real_teamlogos.log
77de06c959eba7c0f2f16fd29fb725270aa9a30713f632779d5ad4a0e0fe872b  closeout_domain.log
3a4708f96aa4e813fd386fc1feaac1e60eecff31f98e225cf53b67c073beaff4  closeout_tests.log
```

What these logs do **not** establish: loader acceptance of any staged archive (no game launch),
the NHL 15 PS4 run (receipt 03 only), the R1-reviewed conflicts (F1/F2 of the R1 review were found
after these runs and are covered by the R1 regressions, not here), or anything for Phase 2 beyond
the two Legacy team-logo placements.

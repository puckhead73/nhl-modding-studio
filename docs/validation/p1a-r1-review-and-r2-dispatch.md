# P1-A-R1 review — R2 ticket ownership correction

18 September 2026. Continue the same developer session. P1-B remains assigned to the separate research session; do not take over that research or advance to another implementation package.

## Finding: P1 — review IDs collide across workspaces

Both Workspace constructors initialize `next_schedule_review` to 1. `hold_schedule_review` increments only that workspace's counter, and `schedule_apply_review` looks up the bare numeric ID in the destination workspace. No origin namespace is carried or checked. A ticket from another workspace can resolve to a valid local plan and apply it instead of refusing.

Independent reproduction on two in-memory Legacy workspaces:

1. In A, preview NHL game 0 day = 1: ticket 1.
2. In B, preview the same game's day = 2: ticket 1.
3. Call B's `schedule_apply_review` with A's ticket.
4. Apply succeeds and writes B's day = 2 plan. Output: `CONFIRMED: A's ticket 1 accepted by B and applied B's different plan`.

The API takes project ID plus the same bare ticket and performs this lookup, so it inherits the issue. Current domain coverage at the foreign-review assertion uses `foreign + 1000`; that proves unknown-ID refusal, not ownership enforcement. The receipt's foreign-ticket rejection claim is therefore too broad.

## Required bounded correction

Give review tickets an origin identity that cannot alias another workspace's ticket, including replacement/reopened workspace instances. An opaque globally unique token, a validated workspace-instance nonce plus sequence, or a suitably scoped unique allocator is acceptable. Preserve exact server-held plans, input identity/revision checks, single-use application, bounded retention and unchanged-on-refusal behavior. Keep representation lossless through Rust, DTOs and JavaScript; do not route arbitrary 64-bit tokens through lossy Number conversion.

Regressions must mint actual valid reviews concurrently in A and B, then send A's unmodified ticket to B and vice versa. Assert refusal and unchanged bytes, revision, pending state, undo/redo and review usability in both. Repeat with different schedule tables/operations and with an old ticket after workspace replacement/reopen. Exercise the Registry/API seam as well as domain code. Legitimate tickets still apply exactly their own stored plans once.

Deliver P1-A-R2 with a plain-language-first receipt, a dated correction to the prior foreign-ticket claim, affected tests/bindings and cleanup, then stop for review. No live writes, launches, staging, commits or pushes. Do not expand this correction into P1-B.

## Review scope

Final independent rerun: `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test schedule_edit -- --include-ignored`: **12 passed, 0 failed, 0 ignored**, including the 26-target parity case and real NHL 20. API: **1 passed**. UI: **9 passed**. These suites pass but do not cover the reproduced ticket collision.

The earlier stale-baseline/input recomputation defect is addressed by stored plans and revision checks. Nine deferred UI tests and the API schedule test passed independently. The real NHL 20 test also passed in the independent domain run; final suite totals are recorded below when the run finishes. The ownership reproduction used only memory-loaded anchors and a temporary probe; no original or install was written. Probe source and its owned build artifacts were removed after execution. No visual acceptance requested; NHL21-E1 remains closed.

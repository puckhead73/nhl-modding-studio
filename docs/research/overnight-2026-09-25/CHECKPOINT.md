# Restart checkpoint — research lane 2026-09-25 (afternoon closeout 15:35 MDT; **correction checkpoint 22:07 — see [handoff 14](14-correction-checkpoint-r97-r101-review.md)**)

**Morning assignment closed 06:25** (tasks 0–6 of [TASKS.md](TASKS.md)). **Afternoon continuation 13:56 → 15:35** under the user's
instruction to execute the 24 September campaign review (already answered by R88 — standing) and continue the queue, applying the
25 September direction audit as the correction checkpoint. Tasks 7–12 complete; nothing running; nothing left mid-way.

## Completed this afternoon (evidence files)

- **R96 (registered late)** — `probes/r96_decode_matrix.py`, `data/r96-decode-matrix.json`, `r96-run.log` (ran 13:36–13:38 by an unrecorded
  continuation; summarised in handoff 10 §6).
- **R97 correction checkpoint** — [10-correction-checkpoint-direction-audit.md](10-correction-checkpoint-direction-audit.md);
  `probes/r97_chant_id_field_census.py` → `data/r97-chant-id-field-census.json` (18 gen7 anchors, every integer field, 55–66); dated notes on
  CHANT_SELECTION, USER-CARDS (rev. 3), DEVELOPER-QUEUE, REPLACEMENT_READINESS, COVERAGE (+ `gen_docs.py`), handoff 09, STATUS; and on the
  2026-09-24 handoffs 06 / 08, ATLAS, DEVELOPER-QUEUE, USER-CARDS, FINAL-RECEIPT, STATUS.
- **R98 / R98b archive-backed `.ast` framing** — [11-ast-framing-archive-backed-titles.md](11-ast-framing-archive-backed-titles.md);
  `probes/r98_ast_framing_archived.py` → `data/r98-ast-framing-archived.json` + `blockmaps/r98-streams-archived.jsonl`;
  `probes/r98b_x360_09_packet_header.py` → `data/r98b-x360-09-packet-header.json`.
- **R99 team-art archive spans / entry fields** — [12-team-art-archive-spans-and-entry-fields.md](12-team-art-archive-spans-and-entry-fields.md);
  `probes/r99_team_art_archive_spans.py` → `data/r99-team-art-archive-spans.json`.
- **R100 / R101** — [13-archived-stream-decode-and-kit-name-key-positions.md](13-archived-stream-decode-and-kit-name-key-positions.md);
  `probes/r100_archived_decode.py` → `data/r100-archived-decode.json`; `probes/r101_nameplate_kit_key_positions.py` →
  `data/r101-nameplate-kit-key-positions.json`.
- Claims (all `ready`, `local/coordination/overnight-2026-09-25/claims/`): r96, r97, r98, r99, r100, r101. MANIFEST.md addenda 1 / 2.

## Evening continuation (21:58 → 22:22)

- **R102 correction checkpoint** — [handoff 14](14-correction-checkpoint-r97-r101-review.md); probes `r102_eb_layout_constraints.py`,
  `r103_nameplate_rink_discriminating_join.py`, `r103b_loose_archive_copy_identity.py`, `r100b_x360_09_rewrite_pcm_identity.py`; queue split
  into [DEVELOPER-QUEUE.md](DEVELOPER-QUEUE.md) (current C1–C9) and [DEVELOPER-QUEUE-HISTORY.md](DEVELOPER-QUEUE-HISTORY.md).
- **R104–R106 + R104b** — [handoff 15](15-team-art-payload-references-and-inspection-graph.md); probes `r104_team_art_payload_references.py`
  (`--trees`), `r105_logo_family_vs_kit_tuples.py`, `r106_team_art_inspection_graph.py`.
- Claims added: r102, r104, r105, r106 (+ revisions on r96 / r99 / r100 / r101 / r104).
- **Next exact task:** static "rink logo" discrimination via RW4 raster descriptors, then PS3 12 / 13 payload references (TASKS.md).

## Open (next package candidates, not started)

1. **In-payload references of kit members** (jersey `.rx2` / `.rpsgl` → nameplate / crest by path or djb2) on Legacy X360 / PS3, read-only —
   ~~the last unmeasured element of the archive-side rename cascade~~ **one requirement among several** (R102 §F1: destination occupancy,
   other-title coverage, naming / layout rules, recovery, runtime lookup remain separate). Still the next task.
2. ~~PS3 NHL 11 / 12 sock grammar~~ — **closed by existing evidence** (R56 / R74 two-file form; developer resolver) — R102 §F2.
3. X360 NHL 09: 87 residual middle blocks under the frame walk; the two header-word values; the 4 loop bodies under the walk.
4. PS4 `word_12` (sorted, non-hash of 25 variants × 5 algorithms) — needs a new candidate function or an authoring reference.
5. Club-chant selecting consumer — offline exhausted; next lever is an explicit treatment on a disposable copy (not authorised); cards
   T-CHANT-1 rev. 3, A-SPX-1 rev. 3, A-AST-1, A-LOOP-1 stay human-review-pending and block nothing.
6. Other gen7 titles' team-art family censuses (08–11, 14) before any cascade proposal there.

## Exact restart / reproduce commands (repository root; read-only over X: / H: / anchors)

```
python local/runs/overnight-research-2026-09-25/probes/r97_chant_id_field_census.py docs/research/overnight-2026-09-25/data/r97-chant-id-field-census.json
python local/runs/overnight-research-2026-09-25/probes/r98_ast_framing_archived.py docs/research/overnight-2026-09-25/data/r98-ast-framing-archived.json --summary local/runs/overnight-research-2026-09-25/blockmaps/r98-streams-archived.jsonl
python local/runs/overnight-research-2026-09-25/probes/r98b_x360_09_packet_header.py docs/research/overnight-2026-09-25/data/r98b-x360-09-packet-header.json
python local/runs/overnight-research-2026-09-25/probes/r99_team_art_archive_spans.py docs/research/overnight-2026-09-25/data/r99-team-art-archive-spans.json --ps4 "H:/Emulators/ShadPS4/Games/CUSA18182/databigs/inxp_00.big"
python local/runs/overnight-research-2026-09-25/probes/r100_archived_decode.py docs/research/overnight-2026-09-25/data/r100-archived-decode.json --scratch <scratch dir>
python local/runs/overnight-research-2026-09-25/probes/r101_nameplate_kit_key_positions.py docs/research/overnight-2026-09-25/data/r101-nameplate-kit-key-positions.json
python local/runs/overnight-research-2026-09-25/probes/r96_decode_matrix.py docs/research/overnight-2026-09-25/data/r96-decode-matrix.json   # as run at 13:36; not re-run
```
The morning commands (r91–r95, `gen_docs.py`) are unchanged from the 06:25 checkpoint; **do not re-run `gen_docs.py` casually** — it
rewrites PREVIEW_REVIEW / COVERAGE / MANIFEST and re-stamps the claim files (the COVERAGE scope note is kept by the patched script).

## Owned processes / scratch at closeout

No background process of this lane (the r98 and r99 background runs exited 0 at 14:03 / 14:52). Session scratchpad empty (patch scripts,
follow-up scripts, the r98 smoke JSON and the r100 decode scratch directory deleted). Lane run logs kept: `r96-run.log`, `r98-run.log`,
`r99-run.log`, `r100-run.log` (+ the morning's `r91-full.log`, `r92-run.log`). Retained review dependencies unchanged: previews (62 MB,
51 WAV) and representative block maps (46 MB) — regenerable, to be deleted when the review packet closes. Nothing staged, committed or
pushed; `docs/research/` and `local/` untracked; no write to X:, H: (outside the session scratch), `anchors/`, `defs/`, bindings,
developer paths or application code; the developer's working tree as found.

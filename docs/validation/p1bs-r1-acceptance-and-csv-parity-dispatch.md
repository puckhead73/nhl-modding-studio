# P1-BS-R1 acceptance and next research package

Research session: start a fresh session for the next package below. Handoff 22 revision 2 is accepted as a bounded research/implementation contract with the coordinator clarifications here taking precedence. No additional correction-only cycle is required. This accepts stored-default editing scope, not runtime consumption or selected-value serialization.

## Review and binding clarifications

Codex reviewed the revised applicability table, eligibility/preservation contract and runtime card against the earlier findings, inspected the catalogue JSON and NHL 15 metadata, and checked checksum scopes in `tdb-core::TdbFile::recompute_crcs`. No extraction probes, compiled parser tests or runtime experiments were rerun. The 55-file census remains research-lane evidence.

- The eligibility table contains **21 distinct legacy IDs across 20 displayed rows** (67 and 85 share a line): 67, 68, 69, 70, 71, 72, 73, 77, 83, 85, 86, 87, 88, 89, 116, 117, 118, 119, 120, 123, 212. These explicit IDs, expected row names and validated opened-file bounds govern eligibility; the receipt's count of 20 does not remove an ID.
- For a same-length cell edit on a valid baseline, table headers and the file header are unchanged. Their CRCs must therefore remain byte-identical, even if the serializer recomputes them. Only the edited body’s covering CRC may change: the next physical table's `prior_crc`, or the EOF CRC if the edited body is last. Derive the exact offset and verify its value independently; do not allow changes to a header checksum merely because the writer touched it. Do not repair unrelated invalid checksums as a side effect. This tightens handoff §2.5.
- Prior reviewed edits are part of the current baseline, not an exemption from bounds or target-identity validation. Reopening an edited file must remain eligible without retaining old review tickets. Validate the current target and preserve unrelated existing anomalies; do not blanket-skip previously edited cells.
- `HwRX.NVBd` remains a structurally inferred offset interpretation, unlike the stronger executable witnesses for the two `stocksettings` fields. The queued decoder package may apply the scoped interpretation, but its evidence/commentary must preserve that distinction. No claim that every field was externally verified; no general negative-range rewrite. Table-plus-field and matching declaration shape are required, including a regression for unrelated `qbPU.CkjP`.
- A fresh/existing-profile difference supports a profile-associated difference, not proof of a particular persisted-choice mechanism. Treat it as a control result needing matched conditions, not a located setting store. Runtime execution remains with the user.

The machine-readable JSON's introductory prose still says every id matches the executable; the numeric evidence and revision-2 exceptions govern. Carry that wording correction into the next research package rather than repeating extraction.

## Developer queue

P1-C-R2 remains active. After it closes, dispatch P1-BS-1: narrowly scoped decode/encode correction and regression coverage per the prior review plus the clarifications above. Then the read-only catalogue integration, then the accepted one-cell writer scope as separate bounded work. This note does not start a concurrent developer or authorize a defaults writer before its dependencies.

## Next Claude research prompt — P1-BS-CSV: PS4 defaults parity and packaging contract

Start a fresh research session. Read the working agreement, this acceptance, handoff 22 revision 2, handoffs 16/20 for packaging constraints, and the Phase 1 coverage matrix. Claim a distinct research item. The developer owns P1-C-R2 followed by the scoped decoder correction; do not edit application code/defs/bindings or wait on that work.

1. Reuse the existing settings CSV census to close its explicit source gaps: verify the real NHL 20 install's source/member identities and compare relevant NHL 19 patch archives for settings CSV members. Record inspected archives, exact member paths, hashes and duplicates. Presence of a patch copy is not established precedence. Do not repeat the whole executable inventory.
2. Define the NHL 16 and NHL 17–21 equivalents of the accepted NHL 15 stored-default operation. For each title, identify exact setting/context keys, editable cells, contextual range/choice constraints, duplicate/missing/null handling and raw-code meanings. Do not transplant NHL 15 bounds or labels where the CSV schema supplies different ones. Define ambiguous/refused cases and a second-edit/reopen rule. Keep runtime effects unknown.
3. Inspect the existing Studio packing routes read-only and distinguish missing functionality from already available primitives. Specify a one-field CSV preservation contract covering encoding/BOM, quoting, delimiter/newline style, other fields/rows and unknown columns. Identify each measured wrapper/container chain and the requirements for changed compressed length, archive metadata preservation, copied-output reopen and restore. Reuse P1-F rather than inventing a parallel packer. Same plaintext length does not establish same compressed length; do not claim that every text edit necessarily changes length either.
4. Produce per-title offline developer vectors and user-review cards with exact permitted differences and source provenance. Where packer support is absent, publish the smallest implementation dependency. No live placement or game run. Treat selected-value storage and precedence as unresolved, not prerequisites for safely editing a stored default.
5. Incorporate the small acceptance clarifications into handoff 22 and the JSON introductory wording; preserve underlying measured data. Keep Xbox/PS3/Recompiled gaps explicit and unchanged unless this bounded work supplies new evidence; absence in inspected sources is not permanent lack of support.

Deliver one concise handoff, machine-readable per-title contract/source map, checks actually run, and cleanup manifest. Research-owned paths only. No application changes, installs/image writes, launches, staging, commits or pushes. No retained bulk extraction or extra build tree. Stop for coordinator review after the bounded package.

## Roadmap

Phase 1 remains active. P1-A accepted offline; P1-C-R2 is the narrow developer correction. NHL 15 stored-default research/contract is now accepted under this note, with decoding and reader work ahead of the writer. Parallel research advances NHL 16–21 defaults parity and shared packaging requirements. Localization growth and remaining assets/audio work stay in Phase 1. NHL21-E1 remains accepted and closed; Projects/Builds remain final stretch work.

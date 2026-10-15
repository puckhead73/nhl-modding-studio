# CTX0/keyword research review and continuation

Continue in the same research session. Review scope: handoffs 13 rev. 4 and 14, plus the current r20 probe source. No source extraction, runtime execution or independent full census rerun. The read-only catalogue can be scoped to established tables; the complete context-to-sample chain is not accepted as fully parsed.

## F1 — Record bounds are parsed; speech-item extraction remains heuristic

`parse_ctx0_records` reads record pointers and headers, but finds items by scanning every aligned word from record+60 to record end for the namespace and a known MOIR id. It does not follow the recorded speech-list pointer or decode item counts/strides. Inline labels are regex matches in the following span, with candidate-assisted suffix trimming. The 58 fallback context names are also regex-derived.

Consequently scan-to-record hit equality establishes containment, not an independent item-field validation. Keep record bounds/header fields and pointer-derived names as parsed; mark scanned candidate items, fallback names and candidate-assisted labels separately. The all-labels-match result must disclose how labels were normalized using the candidate list.

Before investigating w8 semantics, follow the speech-list pointer and establish element boundaries, count/termination, type/tag fields and name pointers/lengths. Compare independently parsed entries with the old scan, retaining extras, misses and residual bytes. Add malformed/out-of-record/duplicate-pointer checks to the developer contract. If full item structure remains unknown, retain read-only diagnostic candidate listings without promoting them to a parsed context sequence.

## F2 — Keyword membership is useful; control/sample claims need explicit resolution states

The probe itself calls words present in every body control-value candidates, while the handoff and contract declare 287 a control value. Constancy does not establish its semantics. Retain 287 as a recurring word/control candidate, preserve whether it is also present in KWD0, and do not silently exclude or reinterpret it.

Contract 14 must distinguish: raw phrase word; candidate keyword-table match; mapped sample identifiers; bank-resolved sample entries; validated data ranges. Report missing/ambiguous bank IDs explicitly. The cited NHL 15 figure 22,080/26,109 mapped sample IDs in the bank leaves unresolved mappings; a table match alone cannot promise preview. Decoder/playback is separate from this list/inspect contract.

The Xbox NHL 13 mismatch is not resolved until that title's own in-archive KWD0 and matching bank have been checked. Avoid saying the new interpretation already explains it conclusively.

## Next research prompt

Correct F1/F2 in handoffs, probe metadata and the read-only contract. Preserve usable EVT0/MOIR/SPC0/KWD0/SBKR table work; do not hold all listing work hostage to unresolved context semantics.

Extract NHL 13/14 KWD0 members read-only to owned scratch (or stream them), recording archive and member identities; compare against the same title/build speech and bank inputs, rerun keyword/bank resolution, retain full residuals, then delete regenerable extracts.

Next decode the CTX0 speech-list structure independently of namespace scanning. Only after that, or after recording a bounded structural blocker, census header words against crowd contexts and trigger names. Treat value overlap as a candidate association, not a trigger-id proof. GAS instruction decoding and runtime behavior remain out of scope; boards stay on their deferred card.

Publish checkpoints and continue without an intermediate review stop under the existing research campaign. Research-owned paths only; no application changes, launches, install writes, staging or commits. Retain complete evidence, clean obsolete scratch, and end with a plain-language receipt and precise next task.

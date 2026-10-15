# Research follow-up review — growth contract clarification

18 September 2026. Continue the same research session for this bounded clarification. Developer R3 remains active; this dispatch does not authorize new implementation during its correctness correction.

## Accepted research inputs and review scope

Handoffs 18 (RW4ps3 league-logo layout), corrected 17 (source-bound PS4 group mappings), and 19 (goal-horn static evidence) are accepted as implementation/research inputs within their stated limits, not runtime qualification. Codex read the handoffs/status and inspected the machine-readable texture survey: 36 entries, all reporting MAD 0.0, distributed NHL 08: 6, NHL 09: 8, NHL 10: 10, NHL 11: 12. This review did not rerun pixel decoding, executable searches or horn measurements. Do not describe this as every member of a 4×4×3 Cartesian sample; record absent/unmeasured combinations explicitly.

The PS3 codec implementation is now bounded enough to queue after developer R3, alongside the accepted chunklzx work. Use observed signatures/bounds and copied-file checks; no blanket claim that every PS3 texture is linear. PS4 unknown/inferred groups stay so. Hornbank indexing remains unresolved; 39/39 matched NHL 15 arena IDs do not establish later-title sample addressing.

## Growth specification — contradictions to resolve before LOC-W3

Handoff `20-dictionary-growth-invariants.md` is valuable measured input but is not yet a coherent writer contract:

1. I2 requires preserving existing out-of-order rows on NHL 09/10; transformation steps 3/5 and several tests require ascending output. Both cannot hold. Specify a deterministic insertion policy that preserves existing order, define validation relative to baseline anomalies, and label loader compatibility unknown. If evidence cannot justify insertion into an anomalously ordered file, refuse that operation explicitly while supporting less speculative growth operations. Do not infer the engine's lookup algorithm from observed descents alone.
2. Step 4 removes slack/re-emits the pool, while T1 promises byte-identical no-op output on all inputs. Define preservation of untouched input bytes separately from a canonical rebuild. A no-op fast path proves no-op preservation, not correctness of a rebuilding serializer. Require an independent semantic rebuild test plus exact accounting for permitted byte changes; preserve opaque words/padding unless a specific transformation requires them to change.
3. No shared pointers in the sampled shipped dictionaries is an observation, not a universal writer precondition: Studio's LOC-W1 intentionally creates shared text pointers. Support inputs produced by Studio, preserving disclosed shared-reference semantics or explicitly refusing an unsupported operation before mutation. Add repoint → growth and retext → growth tests, null strings, shipped hash collisions and non-ASCII cases. Use row identity for duplicate-hash validation rather than a lookup helper that collapses duplicates.
4. The runtime card should distinguish adding a new key from retargeting a lookup: LOC-W1 changes an existing key's text pointer; it does not make the executable query a new key. State what that experiment proves (larger dictionary plus an existing queried row referencing the appended text) and what stays open (new-key lookup/order). A PS4 stockleagues key change or separately established lookup binding can test the latter when applicable.
5. Archive placement descriptions must preserve known uncertainty: a patch copy is a possible override until precedence is observed, and disc-image relocation differs from loose/archive relocation. Do not claim an extracted tree is ignored by a runtime whose launch source remains unknown. Preserve the current same-length image limitation in the growth plan.

## Next prompt and stop point

Correct handoff 20 and its test matrix, annotate the texture sample coverage in handoff 18, and publish an implementation-ready LOC-W3 contract with explicit supported/refused operations, baseline-preserving invariants, evidence levels and unresolved loader gates. No application edits, launches, live writes, staging, commits or pushes. Reuse existing measurements and probes; do not re-extract whole games or add build caches. Clean only obsolete owned scratch with verified containment; retained specimens need a named active dependency.

Deliver a short plain-language receipt and stop for coordinator review of the contract. This clarification does not block accepted texture/codec/group work after developer R3, and does not reopen NHL21-E1.

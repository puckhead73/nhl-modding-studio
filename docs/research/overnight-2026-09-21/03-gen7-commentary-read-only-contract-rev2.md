# Handoff R26 — Gen7 commentary read-only contract, revision 2 (supersedes handoff 14 of 2026-09-20)

> **Rev. 2c (2026-09-21 pm, 17:16 MDT; campaign [overnight-2026-09-21-pm](../overnight-2026-09-21-pm/00-ATLAS.md)):** on NHL 11 / 12 the condition / expression script ids resolve to the `.gas` blob whose header id word equals the id (keyed lookup, injective, 0 unresolved on six sets — [pm handoff 04](../overnight-2026-09-21-pm/04-ctx0-v31-script-ids-bind-to-gas-blob-id-words.md)); on 13+ that word equals the ordinal, so one lookup serves both versions and the rev. 2b sentence "listing by inline name only" is withdrawn. The record header words get candidate labels (priority-like / time-like / kind) behind disclosure per [pm handoff 05](../overnight-2026-09-21-pm/05-ctx0-header-words-across-versions.md); no selection rule is claimed.
> **Rev. 2b (2026-09-21, later in the session):** `CTX0` record listing coverage extends to **NHL 11 / 12** (v3.1) by [handoff 16](16-ctx0-v31-nhl11-12-context-records.md): same trigger → conditions → item model, inline names, one item per record, group refs; PS3 = X360. The v3.1 script ids are not `.gas` indices (listing by inline name only). Everything else in this revision stands.
> **Rev. 2a (same day, after handoff [04](04-gen7-speech-model-from-ea-source-xml.md)):** the per-word "resolution states" of the
> *Sentences* operation are superseded by the phrase model of handoff 04 — a phrase is one sample group (secret-name tag value) plus event-parameter
> tags, resolved through the graffiti table `(secret tag, value, numParameters)` on **every** title 09–Legacy (NHL 11 / 12 included). See §"Sentences
> (rev. 2a)" below; the source rule, the `CTX0` listing and the refusals are unchanged. Coverage now also includes NHL 09–12 for sentences / graffiti
> (contexts remain NHL 13+).

Research lane, 21 September 2026. A proposal for the developer, scoped to what handoffs [01](01-ctx0-context-records-decoded.md) and
[02](02-nhl13-14-keyword-resolution-and-source-identity.md) of this campaign and handoff 13 rev. 4 (2026-09-20, `EVT0` / `MOIR` / sentence
hash / `KWD0` / `SBKR`) established by parsing. Not Codex-accepted; **no writer**; stops where the evidence stops. Claim `research-r24-ctx0-structure`.

## Coverage (measured sets only)

| Title | Platforms | `EVT0` | `MOIR` | sentences (hash rule) | phrase → graffiti table (handoff 04) | contexts (`CTX0`, handoff 01) | bytecode |
|---|---|---|---|---|---|---|---|
| NHL 09 / 10 | Xbox 360 loose XML; PS3 XML in `data0.big` / `audioboot.big` | XML (`interfaceCrc` = MOIR) | yes | XML (no hash table) | 10 490 / 10 490 · 13 331 / 13 331 | not measured (older `CTX0`?) | — |
| NHL 11 / 12 | PS3 = Xbox 360 (NHL 12 PS3 from `audioboot.big`) | v1.2 (NHL 11 4-byte magic rebuilt) | yes | `SPC0` v4.2 / v4.4 | 14 060 / 14 060 · 15 309 / 15 309 | `CTX0` v(0,0) refused | table differs |
| NHL 13 | Xbox 360 loose = archive; PS3 archive-only | yes | yes | v5.3 | 14 696 / 14 696 | yes (X360 measured; PS3 `audio/context` archive-only, not run) | bounded (r22) |
| NHL 14 | PS3, Xbox 360 (loose = archive) | yes | yes | v5.3 | 15 745 / 15 745 | yes, PS3 = X360 | bounded |
| NHL 15 | PS3, Xbox 360 | yes | yes | v5.3 | 9 829 / 9 829 | yes, PS3 = X360 | bounded |
| NHL Legacy | PS3, Xbox 360 | yes | yes | v5.3 | 11 465 / 11 465 | yes, PS3 = X360 | bounded |
| NHL 07 / 08 | — | older `pbp_eng.{big,csi,cti,evt,idx,nam}` system | — | — | not covered | — | — |

Source rule (handoff 02 §2): read the loose `audio/speech` / `audio/context` files where they exist (byte-equal to the `audioboot.big`
members on every measured tree); on PS3 NHL 13 read the `audioboot.big` members (chunkzip); **never** use the Xbox NHL 14 loose
`pbp_eng_samples.sbs` (truncated) — use the `nocache.big` member. Refuse a stream shorter than the bank's maximum offset.

## Inputs and parsers (all read-only, big-endian)

1–5 as in handoff 14 (`EVT0` events + parameter table; `MOIR` id → [names]; `SPC0` sentences with the `FNV-1(name) mod N` assertion; `KWD0`
keyword → sample ids where the file exists; `SBKR` sample id → `.sbs` offset), with one addition: the `EVT0` parameter table is also the
type space of `GRF0` and of the `CTX0` item parameters.
6. `audio/context/pbp_eng_contexts.bin` (`CTX0` v4.5) → **records** `{id, name, trigger | none, w1, w3, w4, conditions[{name, nargs, args[],
   blob}], items[{kind: speech | context, event id, spelled name, params[{immediate v | call{blob, args[]}}]}], extra[], slot2_block | none}`
   by the pointer walk of handoff 01 §2 (item fixed part 36 B on PS3 / 28 B on X360), with the refusal list of handoff 01 §5.
7. `audio/context/pbp_eng_contexteventdef.csi` (`MOIR`, context namespace) → trigger id → [names]; used to type context items.
8. `audio/context/pbp_eng_contexteventsubscr.bin` (`EVT0`, context namespace; all 178 records in the secondary list) → trigger parameter
   counts (the type-check of context-item parameters).
9. (folded into 4a above — rev. 2a.)

## Operations (list / inspect only)

- **Events:** unchanged from handoff 14 (parameters; "no sentence carries this event name" wording).
- **Sentences (rev. 2a; handoff 04):** list each sentence (= event name; refuse a sentence file whose `FNV-1` slot check fails) with its
  bodies and phrases in order. A phrase shows: *sample group* = the secret-name tag value (with the name from the XML on 09 / 10 and from the
  graffiti entry name on 11 / 12; a number on 13+), *parameters* = the event's parameter names at the referenced indices (typed from
  `EVT0`; a reference whose index exceeds the event's parameter count or whose tag differs from the parameter's id / type is reported),
  *repetition pool* (name on 09 / 10, id elsewhere), and *candidate samples* = the graffiti table `(secret tag, value, numParameters)` → sample
  ids → `SBKR` offsets (a phrase without its table is reported, never silently empty — 0 observed). With event-parameter tags present, label
  the list "candidates; the event's <parameter names> select among them". The `KWD0` keyword index is **not** used for phrase resolution
  (it is the mention index of handoff 07 / 2026-09-20; keep it for the team / arena `audioid` joins).
- **Contexts:** list each record as *trigger → conditions → items*: the trigger name (or "no trigger"); each condition as its pool-1 name
  with its integer arguments and its blob index shown as "expression #k (undecoded)"; each item as `speak <event>` (speech) or `raise
  <trigger>` (context) with its parameter values labelled by the event's EVT0 parameter names in order (`immediate v` or `expression #k(args)`);
  the two-item order as listed. Event ids whose name is shared: show the spelled name as "spelled in record" plus the other candidates.
  Header words w1 / w3 / w4, the extra words and the Legacy slot-w11 block: raw, behind disclosure, labelled undecoded.
- **Trigger index:** contexts grouped by trigger name, with the 6 speech-named triggers (NHL 15 / Legacy) flagged "speech-module event".
- **Samples:** keyword → sample ids → `.sbs` ranges for preview on NHL 15 / Legacy only (handoff 02 §6 of 2026-09-20 preservation rules
  apply to any later write; none here).

## Refusals (developer contract)

File-level and record-level refusals of handoff 01 §5 for `CTX0`; the sentence-file hash assertion; a bank stream shorter than the max
offset; a keyword sample id absent from the bank is reported per keyword (not a refusal); an item whose spelled name is not a candidate of
its id, or a blob index above the `.gas` blob count, is reported per record (0 observed — keep the check). Malformed records are listed by
index with the reason; the remaining records are still shown. Nothing is converted to empty data.

## What this contract does not claim

Which context the engine picks among those whose trigger fired and whose conditions hold (w1 / w4 / windows / passives / run-once are
unobserved), what any `.gas` expression computes, what a raised context trigger does, how the take is chosen among a table's samples (repetition pools, multi-tag intersection — stated by the source vocabulary, unobserved),
and every runtime behaviour. Any UI must say "sentence sharing this event's name", not "the line this
event speaks", and "expression #k", not the condition's truth value.

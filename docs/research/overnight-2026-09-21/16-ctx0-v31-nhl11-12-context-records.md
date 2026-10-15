# Handoff R41 — `CTX0` v3.1 (NHL 11 / 12) context records decoded by pointer walk: the same trigger → conditions → item model as v4.5, with inline names, one item per record and group references; PS3 = Xbox 360 record-for-record

Research lane, 21 September 2026, Priority 0 D follow-up (Q-R24-2: "NHL 11 / 12 `CTX0` v3.1 — measured on two records, not parsed").
Claim `research-r41-ctx0-v31`. Read-only over the loose `audio/context` trees (X360 NHL 11 / 12, PS3 NHL 11); nothing game-verified; no
writer. Extends handoff 01 (v4.5, NHL 13–Legacy) so the gen7 context listing of the contract (handoff 03) covers NHL 11–Legacy.

Machine-readable: [`data/r41-ctx0-v31-records.json`](data/r41-ctx0-v31-records.json) (per tree / set: checks, namespaces, blob
statistics, the first 400 decoded records; the parser reproduces the rest). Probe `r41_ctx0_v31_records.py` (≈ 20 s).

## Plain-language summary

NHL 11 and 12 store the commentary's context rules in an older file version than NHL 13+, but with the same meaning: every record names
its context, points at one trigger (with its parameter slots), lists its conditions (each an inline-named script with the trigger
parameters it reads) and carries exactly one action — a speech event to say (2 796 of 2 833 records on NHL 11), a trigger to raise (3) or
an event of another audio module (34, e.g. injury-story events) — plus the "groups" the context belongs to (run-once sequences such as
the cup presentations). Every trigger and every speech event resolves to its name table by id **and** by the inline name (2 699 / 2 699
and 2 796 / 2 796 on NHL 11; 3 200 / 3 200 and 3 302 / 3 339 on NHL 12); the PS3 file decodes to exactly the Xbox 360 records. The
developer's context listing can therefore show NHL 11 / 12 the same way as 13–Legacy; what is still not decoded is the same set as on
v4.5 (bytecode semantics, the time-like words, selection among triggered contexts).

## 1. Structure (big-endian; absolute file offsets; verified by the checks in §2)

```
file    'CTX0' | u16 3, u16 1 | 0 | u32 n | u32 A | u32 n | 0 0 0 | A × (u32 hash, 0, 0, 0)   ← group table (104 on 11, 123 on 12)
        | n × u32 record ptr (unsorted)
record  w0 w1 (time-like: 6000/300, 5001/120 …) | w2 (1 / 2 / 3) | w3 | w4 | w5 hash | 0 0 0 0 | ff
        | w11 → trigger block or ff (134 / 139 records have no trigger) | w12 → item block | ff
        | w14 = ncond | w15 → ncond × ptr | w16 = ngroups | w17 → ngroups × u32 (offset of a group-table entry) or ff | w18 → name
trigger 0 | u32 ns<<16|id (context MOIR) | nargs | 0 | 0 | nargs × u32 = 1 … nargs (parameter slots) | 'trigger_NAME'
item    X360: 0 | u32 ns<<16|id | 0 | 0 | nparams | 0 | ptr → name | nparams × (kind, ptr)
        PS3:  0 | u32 ns<<16|id | 0 | 0 | 0 | 0 | nparams | 0 | ptr → name | nparams × (kind, ptr)   (the 36 / 28 split of v4.5 again)
        kind 0 → a value word: a trigger slot (its value = slot index) or an immediate (0 in 166 / 166 cases on 11)
        kind 1 → expression `0 | script id | nargs | nargs × (kind, ptr)`
cond    nargs | ptr → nargs × ptr (to trigger slot words; ff when nargs = 0) | script id | 0 | [inline arg ptrs] | 'NAME'
```

## 2. Checks (X360 NHL 11 / PS3 NHL 11 / X360 NHL 12; `pbp_eng`; the `crowds` sets in the JSON)

| Check | Result |
|---|---|
| Records, all named | 2 833 / 2 833 / 3 339 |
| Trigger id ∈ context MOIR **and** inline name = MOIR name | 2 699 / 2 699 / 3 200 (of records with a trigger); slots = 1 … nargs on every one |
| Item id ∈ speech MOIR with the same name | 2 796 / 2 796 / 3 302; raise-trigger items 3 / 3 / 3; other-module items 34 / 34 / 34 (namespaces 18389 ×23 injury stories, 26442 ×6, …) |
| Item namespace word = MOIR header namespace | 2 795 on NHL 11; **0 on NHL 12** (items carry 23929, the MOIR header says 9194 — ids and names agree, the header word is not the reference namespace there) |
| Conditions: head zeros, ptr ff ⇔ nargs 0, arg pointers inline, every arg a trigger slot, named | 11 702 / 11 702 / 13 099 on every check (13 050 named on 12) |
| Script id ↔ condition name | 1 : 1 (1 525 / 1 525 ids on 11; 1 758 on 12); **ids exceed the `.gas` blob count** (max 2 250 vs 1 739 blobs; 2 010 conditions) — the v4.5 rule "id = `.gas` blob index" does not transfer to v3.1; the id → script binding is open (listing does not need it: names are inline) |
| Expression params: head 0, args = slots / immediates | 2 438 / 2 438 / 2 638 |
| Group refs = offsets of group-table entries (36 + 16 k) | 251 refs in the sampled records, all multiples of 4 within the table; e.g. the six cup-presentation contexts share entry 40 |
| PS3 vs X360 NHL 11 (layout-normalised) | **2 833 / 2 833 equal in order** |
| `crowds` sets | 1 164 records on all three trees; items are crowd-module events (ns 19767, 1 164 / 1 164 resolve by name); 7 triggers of 1 078 do not resolve (namespace 19767 ≠ crowds context MOIR — cross-module triggers) |

Not decoded (same as v4.5, Q-R24-1): w0 / w1 / w2 / w3 / w4 / w5, the group entries' three zero words, the time-like semantics, the
script-id → `.gas` binding on v3.1, selection among triggered contexts.

## 3. Contract update (handoff 03 rev. 2a → 2b)

`CTX0` record listing coverage becomes **NHL 11–Legacy** (v3.1 by this parser, v4.5 by handoff 01): trigger name + slots, conditions
by inline name with the slots they read, one item (speech event / raised trigger / other-module event) with its parameters (slot or
expression with script id), group membership by table index. Refusals: files whose version is neither 3.1 nor 4.5; NHL 09 / 10 (no
`audio/context`); the crowds cross-module triggers are listed by id with "no name in this module's table".

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r41_ctx0_v31_records.py docs/research/overnight-2026-09-21/data/r41-ctx0-v31-records.json   (≈ 20 s)
```
Inputs: `audio/context/{pbp_eng,crowds}_{contexts.bin,contexteventdef.csi,contextgasscript.gas}` and `audio/speech/*_speecheventdef.csi`
on the three trees; the 2026-09-20 `r20_gen7_speech_events.parse_moir`.

# Handoff 07 (28 Sep) — NHL 07 / 08 roster heads without a head file: every such row is a "head id = art id" generic player; no generic head family by name, so a smaller in-game card

Research lane, 28 September 2026, published 06:05 MDT (system clock). Prompt package 6 (fallback: portrait / head refusal exceptions).
It narrows the residual left by 27 Sep handoff 17 §10 (R134b). Read-only: archive member names (BIG4 / loose) and the X360 NHL 07 / 08
bio tables. No probe file; the commands are inline in `STATUS.md`, reproducible with the R134 / R113 helpers.

## Plain-language summary

On NHL 07 and 08 about two in five (NHL 07) or one in nine (NHL 08) of the players have a head number with no head file anywhere: not in the
loose folders and not inside the game archives. Every one of those players has a head number equal to their picture number. They are
nearly all lesser-known and European-league players, and their skin colour uses only the lightest few values. That looks like the game
giving generic players a generic face built from the skin colour, but no file family by that name was found, so this stays a hypothesis.
One short in-game look can settle it.

## 1. Measured (X360; PS3 NHL 08 uses the same database, SHA-256 `587204c3…`)

| Title | Bio rows | Rows whose `headid` has no `head_<id>` mesh (loose **or** any archive member) | Of those, `headid == artid` | Skin colour on those rows | Skin colour on rows with a head file |
|---|---|---|---|---|---|
| NHL 07 (`67a00bea…`) | 2 250 | 950 | **949** | 1: 514, 2: 412, 3: 22, 5 / 6: 1 each | 1–10 (2: 497, 3: 449, 1: 282, 4: 31, 5: 27, …) |
| NHL 08 (`587204c3…`) | 2 962 | 336 (ids 3 000–4 429; one row per id) | **336** | 1: 148, 2: 177, 3: 11 | 1–10 (1: 830, 2: 789, 3: 672, 4: 294, …) |

**Archive search.** NHL 08 archives hold `head_<id>` members only for ids below 3 000, plus two in the 3 000s (`head_3524`, `head_3765`,
both used by rows that therefore **have** files). NHL 07 is the same by pattern.

**No appearance parameters.** The NHL 07 / 08 bio tables have no face-shape fields (only `skincolor`, eye colour, hair cards / cap), so
generic faces are not built from stored shape parameters. Example NHL 08 rows: Vitalij Aab (3 000, skin 1), Peter Abstreiter (3 005, 1),
Nils Antons (3 039, 2) — DEL / European-league players.

**Bounded negative:** no member named `head_*`, `commonhead_*` or `generic*` covers these ids. (`commonhead_#` has 2–4 files, named by
number, not by id.) What the game renders for them is **not established**.

## 2. For the developer (no behaviour change requested)

- Keep refusing file-less targets as reassignment targets (receipt 38 refuses NHL 07 / 08 entirely).
- A *listing* can say "generic head (no head file; head id = picture id)" for these rows instead of "missing asset". That is a
  description, not a claim about the render.
- Do not offer "reset to generic" (setting `headid` = `artid`) as an operation until the card below is observed.

## 3. Smaller card P34-G08 (replaces the open "how does NHL 08 build these faces" question)

On a copy of X360 NHL 08, compare in-game faces (player card or on-ice close-up) of:
1. two file-less players with the **same** skin colour (Vitalij Aab and Peter Abstreiter, skin 1);
2. one file-less player with **another** skin colour (Nils Antons, skin 2).

| Observation | Conclusion |
|---|---|
| 1's two faces are identical and differ from 2 only in tone | Generic head chosen by skin colour. The head id is ignored for file-less rows. |
| The faces differ within 1 | Something else varies them (e.g. the id). The generic hypothesis is withdrawn. |

No write is needed; this is an observation of stock data.

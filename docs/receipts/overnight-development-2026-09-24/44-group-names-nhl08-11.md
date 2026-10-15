# Receipt 44 — NHL 08–11 conference / division labels (R133); NHL 07 team-header division defect fixed

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD `8d60a6d`
(unchanged; nothing staged or committed). Claim `dev-group-names-nhl08-11`. Research consumed: R133 (2026-09-26
handoff 26 §4). **After the r43 bundle** (not in any review bundle).

## 1. Plain-language summary

- **Divisions on NHL 08–11** used to show a bare number. They are now labelled by the clubs the game files
  there, for example "Division 3: CBJ · CHI · DET · NSH · STL". This appears on the team header and in the
  league-membership editor. The games' division names are not assigned, because research showed their order
  does not match the numbers.
- **Conferences on NHL 10 / 11** show "Western Conference (inferred)" / "Eastern Conference (inferred)". This
  applies to the NHL and AHL, plus the OHL and WHL on NHL 11: research read the name list in those games'
  programs, and its order fits the numbers. The QMJHL has no conference names and stays a number. NHL 08 / 09
  also stay numbers, because their name lists were not read.
- **Defect fixed (NHL 07):** every club's team header said division "Northeast". The header read an always-zero
  column on the thin team selector instead of the stock team row that holds the membership. Chicago now reads
  Central, Anaheim Pacific and Toronto Northeast. The membership editor already read the right row.
- Moves and writes are unchanged.

## 2. Files

| Path | Change |
|---|---|
| `defs/leagues/gen7-nhl10.toml`, `gen7-nhl11.toml` | `[[groups]]` conference candidates (`rasz` 0 / 1) for NHL, AHL (+ OHL, WHL on 11), labelled "(inferred)", with the R133 comment |
| `crates/studio-domain/src/membership.rs` | `local_division_label` (members sharing league, local conference and division; the All-Star rows stay apart); used by the editor's current state; note reworded |
| `crates/studio-domain/src/view.rs` | team header: member label fallback; NHL 07 reads the stock row first |
| `crates/studio-domain/tests/group_names_r133.rs` (**new**) | below |
| `crates/studio-domain/tests/membership.rs` | two assertions updated for the intended new labels |

## 3. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/r44-group-names.log`)

- `group_names_r133` 2 / 2, including the ignored NHL 07 listing.
  - X360 NHL 08 / 09 / 10 / 11 and PS3 NHL 10 / 11: CHI → "Division 3: CBJ · CHI · DET · NSH · STL", TOR →
    "Division 0: BOS · BUF · MTL · OTT · TOR" (EAS / WES are not mixed in).
  - Conferences are inferred on 10 / 11 and absent on 08 / 09. The editor shows "… (name untraced)".
  - NHL 11 QMJHL: no conference label; division by members.
  - NHL 07: CHI Central, ANA Pacific, TOR Northeast, previously all "Northeast". NHL 12 keeps its group table
    ("CENTRAL DIVISION").
- Regressions:
  - studio-domain `leagues` 11 / 11, `ps4_groups`, `view`, and `membership` 5 / 5. Two assertions expected
    "untraced" on NHL 10 / 11 conferences; they were updated to the intended "(inferred)" labels, and the first
    run's failures are in the log.
  - studio-api `membership` 6 / 6 (1 ignored), `leagues` 3 / 3.
- The anchor directories equal the Xbox / PS3 bytes named in `defs/leagues` (unchanged). No DTO change.

## 4. Limits

- No runtime claim: which screen reads a conference name is untraced (R133 §5). The NHL 08 / 09 name lists
  were not regenerated.
- The division label lists at most 8 clubs, plus a count.

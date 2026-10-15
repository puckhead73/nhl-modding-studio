# League identity — runtime qualification test cards (Phase 1.1)

> **Package:** [L1.1-Q1](../validation/league-resp-1-review-and-qualification-dispatch.md), corrected by
> [L1.1-Q1-R1](../validation/league-l11-q1-review-and-r1-dispatch.md), 2026-09-17.
> **Author role:** developer (handoff only). Reviewer: Codex.
> **Status: no card in this document has been executed.** Nothing here is a result. Every card is
> a procedure with the inputs it needs, what each experiment can establish, what it leaves
> unresolved, and the smallest next action per outcome. Executing a card is a separately authorized
> package; the next anticipated one is **E0 only**, not an E0–E5 run.
> Current per-target evidence: [parity matrix §0](league-identity-parity-matrix.md#0-current-state-per-target-l11-q1-corrected-by-l11-q1-r1-2026-09-17).
> Durable inputs: [`docs/validation/evidence/league-l11-q1/`](../validation/evidence/league-l11-q1/).

---

## 1. Rules that apply to every card

### 1.1 Two gates, one restoration gate

* **G0 — stock viability (exit condition: E0 alone).** The *unmodified* game, in the recorded
  environment, boots and reaches **at least one** of the card's named surfaces showing the
  baseline label. That establishes stock viability *for that surface*. Reaching the second
  predicted surface is requested coverage: if it cannot be reached, G0 still passes for the first
  surface and the record says "second-surface coverage incomplete" — it is **not** diagnosed as an
  emulator defect, a game defect or anything else without further evidence. G0 does not depend on
  any later experiment. **No modified boot (E1+) starts before G0 has passed.**
* **G1 — mod acceptance.** Experiments E1–E4 run only after G0. Each yields observations; verdicts
  are drawn per §1.3.
* **E5 — restoration gate.** The run (and any G1 verdict) **cannot close** until every replaced
  file hashes back to its pre-run value **and** the restore boot shows the baseline on the G0
  surface. E5 is a post-experiment gate, never a precondition of G0.

### 1.2 Environment record

The coordinator's decision: the modified shadPS4 fork on this host is the **provisional initial
diagnostic environment**. A result qualifies *that environment only*; nothing transfers to an
upstream release or a console. Every run records, at run time: emulator executable path, size,
SHA-256; the version / revision / branch / description line from its log; the SHA-256 of its
config file; game `param.sfo` fields (`TITLE_ID`, `CONTENT_ID`, `APP_VER`, `VERSION`); update /
patch folders present beside the game; save data present for the title (with the exact
directories inspected).

### 1.3 Verdict discipline (observation first)

An experiment produces **observations** (what booted, what each named surface showed, hashes).
From observations the record derives, in this order:

1. **Established** — only what the observation directly shows for the exact placement, build,
   environment and surface. A label change on a named surface under a controlled, repeatable
   edit, with unrelated checks unchanged and an exact restore, **is** a positive qualification for
   that placement and surface. A boot, an unchanged label, a crash, an absent effect **establish
   nothing about cause** by themselves.
2. **Unresolved** — every alternative explanation the observation does not exclude. A crash after
   a modified boot is associated with *that run*; it does not distinguish archive metadata from
   the edited DB, from emulator nondeterminism, or from a later consumer. An inert loose file does
   not show loose files are never read (wrong path, another source, cached or mode-local data, an
   unobserved consumer remain). A successful boot does not show the edited member was loaded.
   Two surfaces disagreeing does not show they have different consumers.
3. **Next diagnostic** — the smallest discriminating control that would resolve one alternative
   (§3). Attribution is written **only after** such a control or trace evidence.

Verdict words: **established (positive)**, **inconclusive** (observation made, cause unresolved),
**blocked** (could not observe: G0 failed, inputs mismatch, file refused). "fail" is used only as
"failed to reproduce the baseline after restore" (E5) — never as an attribution.

### 1.4 Hash and artifact discipline

* `SHA256SUMS.pre.txt` for every install file a card may replace, before the first boot. A
  pre-run hash that differs from the card's identity table **blocks** the run (different install).
* Every placed file's SHA-256 is recorded and must equal the Studio transaction's declared output.
* **Unexpected output hash.** The controlled NHL 21 edit has a known expected output
  (§2.2). If Studio produces a different hash, **qualification pauses**: do not place the file.
  Explain the difference (inputs, edit, tool version), rerun the allowed-difference audit on the
  new output (only the intended record differs at record level; derived integrity bytes may
  move; length unchanged), and record the new value as the expected output *for this run* before
  continuing. Never substitute a different edit under the same card to make the numbers match.
* Cold start: the emulator process is fully exited between experiments (no `shadPS4.exe` in the
  process list), so no in-memory copy of a database survives a file swap.
* Restore is byte-exact and hash-verified, then E5 boots.

### 1.5 Evidence layout

`docs/validation/evidence/league-<target>-runtime/` with `screenshots/` (`NN-<experiment>-<surface>.png`,
whole screen), `logs/` (emulator log per boot; Studio transaction ids and outcome text),
`SHA256SUMS.pre.txt` / `.post.txt`, and `run.md` filling the card's result table verbatim.

### 1.6 Never

Never use another title's dictionary, archive or database as a stand-in; never place a Studio
artifact into an install whose original hash was not recorded; never continue past a G0 failure;
never delete or repack archive members to manufacture a fallback experiment; never describe an
unexecuted step as done or an observation as an attribution.

---

## 2. NHL 21 PS4 — executable test card

### 2.1 Identity

| Item | Value (read-only in L1.1-Q1; re-record at run time) |
|---|---|
| Title / platform | NHL 21, PS4 (shipping target `nhl21-ps4`) |
| Installed game | `H:\Emulators\ShadPS4\Games\CUSA18182` |
| `sce_sys/param.sfo` | `TITLE_ID CUSA18182`, `CONTENT_ID EP0006-CUSA18182_00-NHLICEHOCKEY2021`, `APP_VER 01.00`, `VERSION 01.01`, `CATEGORY gd` |
| Update / patch | none found: no `CUSA18182-UPDATE` or `Patch` folder under `H:\Emulators\ShadPS4\Games\`; `H:\Emulators\ShadPS4\Patches\` holds only `NHL19.xml` and `n24-*` entries |
| Emulator (provisional diagnostic environment) | `H:\Emulators\ShadPS4\shadPS4.exe`, 69 042 176 B, file date 2026-09-09; its most recent log (`C:\Users\puckh\AppData\Roaming\shadPS4\log\shad_log.txt`, 2026-09-15) identifies `shadps4 v0.16.1 DMC`, revision `d96133b0aee984ede33a7e6063b60df3ddc12bb2`, branch `dev-next`, `checkpoint/p4-entry-0-gd96133b0a-dirty`, remote `puckhead73/shadPS4` — a locally modified fork; `shadPS4.exe.pre-tp2f` / `shadPS4.exe.tp2f-final.bak` sit beside it. **Record the exe SHA-256 and the log's version line at run time**; that pair identifies the environment the result applies to |
| Emulator config | `C:\Users\puckh\AppData\Roaming\shadPS4\config.toml` (`enableMods = true`; `mods_folder/` empty; `installDirs = []`). Mods-folder semantics are **not established** and the card does not use it |
| Prior NHL 21 runtime evidence | none found in the inspected places: the only emulator log present records an NHL 24 run; no `savedata` directory exists under `C:\Users\puckh\AppData\Roaming\shadPS4\` or `H:\Emulators\ShadPS4\` (searched to depth 5); `sys_modules/CUSA18182` exists (the title has been started at some point on some build). Whether an NHL 21 profile exists elsewhere is unknown; E0 records what the first boot does |
| Original archive | `databigs/inxp_00.big`, 777 373 328 B, SHA-256 `1c41f578a595dbd5aae7ad921445626636f3a842d8fb869bc054d35e3e849e87` |
| Original DB member | entry **2652** `nhl/data/common/gamemodes/database/t3db/nhl_game.db`, raw, 11 209 672 B at 726 230 496, SHA-256 `90ee1ec49d36dbb0f6d9cefe75e259d9661e3aa05338d03347b4842e33c8e9f9` (= anchor; independently re-hashed by Codex); `+0x10` = djb2(path) `0x00429ca7`; `+0x0C` `0xe6955448` (unresolved word) |
| Original dictionary member | entry **1156** `nhl/data/kettle/loc/eng_us.db`, raw, 3 921 060 B at 746 617 088, SHA-256 `24693e2f1ae87bde23066d083458bae2efea9a633269518f4fede2354fbac969` (independently re-hashed by Codex); byte-identical local copy at `databigs/Extracted/nhl/data/kettle/loc/eng_us.db` — this hash-bound local source is the dictionary Studio attaches (not a repository fixture, by coordinator decision) |
| Loose game-tree files | the install ships loose `nhl/data/common/config/*.cfg` and `nhl/data/kettle/config/*.ini`; there is **no** loose `nhl_game.db` in the game tree (the only loose copy is Studio's own extraction under `databigs/Extracted/`, not a game path). Whether the game consults a loose `nhl/data/common/gamemodes/database/t3db/` path at all is the E2 hypothesis |
| Occurrence | `nhl_game.db` and `loc/eng_us.db` exist once each across all 16 `databigs/*.big` |

### 2.2 The edits

Two **distinguishable** existing-key re-points on the same row are defined. X is the primary edit
(every experiment); Y is used only by E3 (precedence) and must be validated offline exactly like X.

| Item | Edit X | Edit Y (E3 only) |
|---|---|---|
| Table / row | `stockleagues` (`fsLK`), the row whose `name` key is `Allsvenskan` — `leagueid 8`, record 8 (resolve by key; never by record index) | same row |
| Field | `name` (`vIys`, 25-byte inline key) | same |
| Before → after | `Allsvenskan` → `TXT_ROW_LEAGUE` (existing key, 14 bytes) | `Allsvenskan` → `TXT_LEAGUE_NORWAY` (existing key, 17 bytes) |
| Baseline label (title's own dictionary, verified) | `Allsvenskan` → **"HockeyAllsvenskan"** | same |
| Expected label after | **"Rest Of World"** (`TXT_ROW_LEAGUE`, verified present on NHL 21) | **"Norway"** (`TXT_LEAGUE_NORWAY`, verified present on NHL 21, hash `838edd73`) |
| Untouched | `leagueid`, `orderid 11`, colours, `artid 19`, `renderid 22`; all 14 member teams keep `xMjj = 8` | same |
| Expected Studio outputs | loose `nhl_game.db` 11 209 672 B, SHA-256 `343404153ae51177…` (identical in the offline campaign and both desktop runs); staged `inxp_00.big` 777 373 328 B, SHA-256 `4876cd2b3f4b…` (SMOKE-1 #23). Record the full hashes from the transaction outcome; any other value → **pause** (§1.4) | no prior output exists: the first Studio run defines the expected values; the allowed-difference audit must pass and the values are recorded before placement |
| Second-run edit (E4 only) | `name` → `Legends League` (14 bytes, **not** a dictionary key) — tests literal fallback (parity matrix §6 Q1); its own offline validation and expected hash are recorded before placement | — |

### 2.3 Predicted surfaces (predictions to test — not results)

Surfaces where NHL 21 is expected to render a league name for the Allsvenskan teams (AIK,
Björklöven, …): the Play Now team-select league filter/tiles; the roster-management team picker;
the Franchise / Season team-select. The card asks for **two** surfaces to be observed and named
exactly as the game names them, each with a whole-screen screenshot; one reachable surface is
enough for G0 (§1.1).

Unrelated data checked on every boot: the NHL team list and one NHL team's name; the SHL and DEL
labels ("SHL", "DEL"); AIK's team name and one AIK player; the profile created at E0 still loads.

### 2.4 Producing the artifacts (Studio, offline — before anything touches the install)

1. Create a **copied-target root** in the SMOKE-1 layout (marked by the engine's helper):
   `<root>/CUSA18182/nhl/data/common/gamemodes/database/t3db/{nhl_game.db, nhl_game-meta.xml}`
   from the anchor and `<root>/CUSA18182/databigs/inxp_00.big` copied from the install. Hash both;
   they must equal §2.1.
2. Launch Studio (`npm run dev` at the repository root), NHL 21 → point the picker at `<root>`,
   open. Leagues → attach the title's own dictionary:
   `H:\Emulators\ShadPS4\Games\CUSA18182\databigs\Extracted\nhl\data\kettle\loc\eng_us.db`; the
   attach confirmation must show SHA-256 `24693e2f…`. The list must show "HockeyAllsvenskan" for
   code 8, "Rest Of World" for code 29 and "Norway" is not a row (it is a key). Screenshot.
3. **Edit X:** league 8 `name` := `TXT_ROW_LEAGUE`; banner "Rest Of World"; Pending Edits exactly one
   cell. Copied-target panel: tick *Also replace the database member…* (defaults
   `databigs/inxp_00.big` · `nhl/data/common/gamemodes/database/t3db/nhl_game.db`), *Review save*,
   *Apply*. Record the transaction id, both output hashes, the qualification text. Compare with the
   expected values (§2.2); on mismatch **pause** (§1.4). Copy both outputs to `artifacts/X/`, hash
   again. Restore the root through the panel (verified back to §2.1 hashes).
4. **Edit Y (only if E3 will be run):** repeat step 3 with `TXT_LEAGUE_NORWAY`; banner "Norway";
   record its outputs as the expected Y values; `artifacts/Y/`; restore the root.
5. **Edit for E4 (only after E1 or E2 established a label change):** repeat with `Legends League`;
   `artifacts/E4/`; restore the root.

Nothing in steps 1–5 modifies the install.

### 2.5 Experiments (in this order, each from a cold start, restore + hash between)

| # | Files in the live install | What it can establish | What it cannot |
|---|---|---|---|
| **E0 — stock boot (G0)** | originals only (hashes = `SHA256SUMS.pre.txt`) | the environment boots this build; the named surface(s) reachable; baseline "HockeyAllsvenskan" seen; what a first boot does about profile/save. G0 passes on one verified surface | anything about Studio or the edit |
| **E1 — archive member only** | `databigs/inxp_00.big` := X archive; **no** loose db in the game tree | a positive: the label on a named surface changes to "Rest Of World" ⇒ the game read the edited member from the archive and consumes `stockleagues.name` (for that surface), and the loader accepted this same-length raw replacement | on a crash/hang/unchanged label: nothing about cause (archive metadata vs edited bytes vs environment vs consumer); on a plain boot: that the member was loaded |
| **E2 — loose game-tree file only** | archive := original (`1c41f578…`, verified); `nhl/data/common/gamemodes/database/t3db/nhl_game.db` := X loose db, placed **in the game tree** (create the directories; note they did not exist) | a positive: label changes ⇒ a loose file at this path is read for this member and takes effect over the packed original (for this surface) | on an unchanged label: whether loose files are ever read (wrong path, other source, cached/mode-local data, unobserved consumer remain open) |
| **E3 — precedence with swapped controls** (only if **both** E1 and E2 established a label change) | **3a:** archive := X (`Rest Of World`), loose := Y (`Norway`). **3b:** archive := Y, loose := X. Both artifacts independently validated offline (§2.4 steps 3–4) | if the surface shows the *loose* edit's label in **both** 3a and 3b ⇒ loose location wins for this member; if it shows the *archive* edit's label in both ⇒ archive wins. Only a consistent result across the swap establishes precedence | if the label follows the *edit* rather than the *location* (e.g. "Rest Of World" in both runs) or the two runs disagree in any other way ⇒ inconclusive (caching, a third source, per-run nondeterminism); if only one of E1/E2 changed the label there is nothing to rank and E3 is not run. **Identical edited bytes in both locations establish nothing** and are not an E3 configuration |
| **E4 — literal fallback** (only after E1 or E2 established a label change) | the placement that established the change, with the E4 artifact (`Legends League`) | what this build shows when `name` is not a dictionary key: the literal, blank, the key text, something else — an observation feeding Phase 1.3 | why; whether other titles behave alike |
| **E5 — restoration gate** | originals only; hashes = pre-run | closes the run: byte-exact restore **and** the G0 surface shows the baseline again, profile loads | — |

Every experiment: cold start → named surfaces → unrelated-data checks → **reload check** (back to
the main menu and re-enter the surface; then quit and boot once more to the same surface — a
label that changes between the two boots is recorded as such and makes the experiment
inconclusive) → screenshots → exit → hash the install → restore → hash again.

### 2.6 Result table (fill verbatim; one row per experiment and per E3 half)

| Exp | Files placed (SHA-256) | Boot | Surface 1 (name, label seen) | Surface 2 | Unrelated checks | Reload / second boot | Restore hash = pre | Observation summary |
|---|---|---|---|---|---|---|---|---|

### 2.7 Verdicts

| Verdict | Condition |
|---|---|
| **G0 pass** | E0 boots to ≥1 named surface showing "HockeyAllsvenskan" (second-surface coverage recorded as complete or incomplete) |
| **G0 blocked** | E0 does not boot or reaches no named surface; or the pre-run hashes differ from §2.1. Stop; report the environment observations; no attribution |
| **Established — identity edit qualified, archive placement** | G0 pass **and** E1 shows "Rest Of World" on ≥1 named surface, unrelated checks unchanged, reload and second boot consistent, **and E5 passed**. Scope: this build, this environment, this placement, the surfaces observed. Surfaces that did not change are recorded as observations (not as evidence of a different consumer) |
| **Established — identity edit qualified, loose placement** | same via E2 |
| **Established — precedence** | E3 consistent across the swap (§2.5), and E5 passed |
| **Inconclusive** | any experiment whose observations do not meet an "established" row: modified boot crashed/hung; label unchanged; label followed the edit rather than the location in E3; reload disagreed with first boot; unrelated data changed. Record observations and the alternatives still open; pick the next diagnostic from §3 |
| **Blocked** | an input refused (e.g. Studio refuses the member as deduplicated/compressed), an unexpected output hash unexplained (§1.4), or a G0 block |
| **E5 fail** | restore hashes match but the restore boot does not reproduce the baseline (or does not boot): the run does not close; the environment state is suspect (profile/save, caches, emulator state) — investigate the environment before any further modified boot; nothing about the edit is concluded from it |

What a fully positive card establishes at most: that one existing-key re-point on
`stockleagues.name` is consumed by the named surfaces on NHL 21 CUSA18182 01.00 under the recorded
emulator build, via the placement(s) that produced it, with (for E1) the loader accepting a
same-length raw member replacement. It does **not** establish the `+0x0C` word's meaning, other
members, different-length or compressed members, save persistence of league data, membership
changes (1.2), text edits (1.3), assets (1.4), console behaviour, or any other title.

---

## 3. Decision table — the next action per observation

| Observation | Established | Unresolved | Smallest next action | Phase |
|---|---|---|---|---|
| E0 boots, one named surface reached with baseline; second not reached | G0 pass for surface 1; environment identity recorded | why surface 2 was not reached (navigation, mode gating, needs a profile/save, emulator) — **not diagnosed** | proceed to E1 on surface 1; record "second-surface coverage incomplete"; add a note on what was tried to reach surface 2 | 1.1 |
| E0 does not boot / no surface | nothing about the edit | environment | G0 blocked; record log/version; decide with the user whether this build stays the diagnostic environment | 1.1 (blocker record) |
| E1: label changed on a named surface, unrelated unchanged, reload consistent | archive placement positive (pending E5) | which other surfaces consume it | E4 (literal fallback) after E5; then the next PS4 card (§4.1) | 1.1 |
| E1: boots, label unchanged | only that the environment tolerated the artifact | whether the member was loaded; whether the surface consumes the field; whether the archive is the read source | run E2; if E2 changes the label, the archive artifact is the suspect (next: a *minimal* archive control — the original archive re-written through the stager with **no** byte change in the member, to see whether the stager's output alone alters behaviour); if E2 is also unchanged, the surface/consumer is the suspect (next: research the consumer, §3 list) | 1.1 → packaging research or 1.2 |
| E1: crash / hang | the modified run did not complete | archive metadata vs edited bytes vs environment nondeterminism vs consumer | (a) reboot E1 once more (nondeterminism check); (b) run E2 — if E2 changes the label, the edited bytes are tolerated and the archive artifact becomes the suspect (next: the no-change stager control above); (c) if E2 also fails, the edited bytes themselves are the suspect (next: re-audit the allowed-difference output; try the artifact in Studio's reopen) | 1.1 → packaging research |
| E2: label changed | loose placement positive (pending E5) | precedence (only if E1 also changed) | E3 if E1 also changed; else E4 | 1.1 |
| E2: boots, label unchanged | only that a loose file at that path did not visibly take effect | wrong path; loose files not consulted for this member; other source; cached/mode-local data; unobserved consumer | if E1 changed the label: record "loose path inert for this surface", continue with archive placement; if E1 did not: consumer research (`leagues_and_teams.json` producer; mode-local `leagueteams`; save/profile copies) | 1.1 / 1.2 |
| Identical edited bytes in both locations (any run) | nothing about precedence | — | not an experiment; discard the observation for precedence purposes; run E3 as specified (X/Y swapped) if both E1 and E2 were positive | — |
| E3: loose label in both 3a and 3b (or archive label in both) | precedence for this member | generality to other members | record; done for 1.1 | 1.1 |
| E3: label follows the edit, or 3a/3b disagree | nothing | caching between boots, a third source, order effects | repeat both halves once from a cold start with the emulator cache cleared (record what "cache" means for this build); if still inconsistent → inconclusive, hand to research | 1.1 (inconclusive) |
| Two surfaces disagree (one changed, one not) | positive for the changed surface only | whether the other surface has a different consumer, a cached value, a mode-local copy, or simply was not refreshed | record both as observations; reload/second-boot check on the unchanged surface; then consumer trace (1.2) — **do not** write "different consumer" | 1.1 (partial) → 1.2 |
| Edited output hash ≠ expected | nothing; qualification **paused** | inputs, edit, tool version differ | do not place; explain the difference; rerun the allowed-difference audit; record the new expected value for this run; only then continue with the same edit | 1.1 |
| Restore hashes match, restore boot fails | nothing about the edit; the environment state is suspect | profile/save/cache state, emulator | investigate the environment (saves created during the run, emulator caches, config drift); reboot stock once more; the run stays open until E5 passes | 1.1 (run not closed) |
| Surface shows `TXT_ROW_LEAGUE`, blank or a placeholder | the field is consumed by that surface (positive for consumption, not for display) | where localization happens and what it does on a miss | Phase 1.3 trace of the resolver before any text edit | 1.3 |
| Input mismatch at run time (hash ≠ §2.1) | nothing | different install/build | blocked; name the file; do not substitute | 1.1 (blocker record) |

Questions that belong elsewhere, so the run does not have to answer them:

| Question | Belongs to |
|---|---|
| Which screens consume `stockleagues.name` vs `leaguegroup`/`conferencegroup`/`divisiongroup`; where `GroupName` comes from; mode-local `leagueteams` copies; new-mode creation | **1.2** |
| Literal fallback behaviour (E4 observes it; what to do about it is 1.3); text under an existing key; adding a key ≤25 bytes; gen7 code→label source | **1.3** |
| `artid`/`renderid` → asset members; league logos/tiles | **1.4** |
| The `+0x0C` sort-key formula, header `0x10`; chunkzip/chunklzx member replacement; different-length members | packaging research (C3 / N19-H1), triggered only by the E1-suspect branches above |
| Repository capture of the NHL 21/19 dictionaries | Codex — decided: **not in this package**; NHL 21 uses its hash-bound local source; NHL 19 needs base/patch provenance first |

### 3.1 Consistency walkthrough (logical check against hypothetical outcomes — not executed)

| Hypothetical | Established | Unresolved | Smallest next action |
|---|---|---|---|
| E0 reaches one surface only | G0 pass (surface 1); environment recorded | second surface | proceed to E1 on surface 1; coverage incomplete |
| E1 crashes; E2 changes the label | loose placement positive (pending E5); the edited bytes are tolerated by the game | whether the crash came from the archive artifact, nondeterminism or the environment | reboot E1 once; then the no-change stager control; E5 before any verdict |
| E1 boots unchanged | environment tolerated the artifact | member loaded? field consumed? read source? | E2 |
| E2 stays unchanged (E1 changed) | archive placement positive | loose path inert for this surface — cause unknown | record; continue archive placement; E4 |
| E2 stays unchanged (E1 unchanged) | nothing | consumer/source | consumer research (1.2) |
| Identical edited copies in both locations | nothing about precedence | — | discard for precedence; E3 with X/Y swap if both singles were positive |
| Two surfaces disagree | positive for the changed surface | other surface's source | reload check; 1.2 trace; no attribution |
| Edited output hash differs | nothing; paused | why | explain; re-audit; re-record; same edit |
| Restore hashes pass, restore boot fails | nothing about the edit | environment state | environment investigation; run stays open |

Gate check: G0 exits on E0 alone (§1.1); E1–E4 require G0; no "established" verdict closes without
E5 (§2.7); E5 never conditions G0.

---

## 4. Target-specific equivalents (roadmap order after NHL 21)

Every card below reuses §1 and the §2.4–2.7 procedure; only the identity, the row and the
inputs differ. Rows are **not** executable until their "inputs still needed" column is empty.

### 4.1 PS4 titles — a direct identity edit exists (`stockleagues`)

| Target | Installed (read-only, `param.sfo`) | Equivalent row X (resolve by key) | Baseline label — **title's own dictionary** | Inputs still needed before the card can run |
|---|---|---|---|---|
| NHL 20 | `CUSA15439`, APP_VER 01.00, 13 archives, `databigs/Extracted` present | `Allsvenskan` → `TXT_ROW_LEAGUE` (Y: `TXT_LEAGUE_NORWAY`, verify present on NHL 20's own dictionary first) | "HockeyAllsvenskan" → "Rest Of World" (committed anchor `62260005…`) | **Provenance:** the NHL 20 anchor is byte-identical to the NHL 19 anchor (C0-REG4-R1). Before any run: parse the installed `inxp_00.big`, find `t3db/nhl_game.db`, hash it, compare with the anchor; if they differ the campaign row is not about this install. Same for `loc/eng_us.db` vs the committed dictionary. Record member entry/offset/kind |
| NHL 19 | `CUSA11126`, APP_VER **01.60**; 25 archives including `data_patch*` files; `Extracted` present; anchor labelled `CUSA11116` (FR9) | `Allsvenskan` → `TXT_ROW_LEAGUE` | **unknown** — no NHL 19 dictionary fixture; the install's own `loc/eng_us.db` must be located (base and patch archives may both carry one) and verified with `tdb-loc` | provenance of both DB and dictionary across **base vs patch archives**: which copy the game uses is a **hypothesis to verify** (patch-first is the usual expectation, not a fact for this title/emulator); the anchor may match neither. Not to be committed until that is established |
| NHL 18 | `CUSA07569`, APP_VER 01.00, 13 archives, `Extracted` present | `Allsvenskan` → `TXT_ROW_LEAGUE` | "HockeyAllsvenskan" → "Rest Of World" (anchor `12f1af46…`, attributed) | member identity/hash check of `t3db/nhl_game.db` and `loc/eng_us.db` in the installed archives against the anchors |
| NHL 17 | `CUSA04326`, APP_VER 01.00, 12 archives | `TXT_LEAGUE_NORWAY` (2-team league) → `TXT_ROW_LEAGUE` (Y: another verified key on NHL 17's own dictionary, e.g. `TXT_LEGEND_TEAMS` → "Legend Teams") | "Norway" → "Rest Of World" (anchor, 30 799 rows) | same member checks; note NHL 17 has `orderid` |
| NHL 16 | `CUSA01909`, APP_VER 01.00, 11 archives | `TXT_LEAGUE_NORWAY` → `TXT_ROW_LEAGUE` | "Norway" → "Rest Of World" (anchor, 27 468 rows) | same; no `orderid`/colours |
| NHL 15 | `CUSA00561`, APP_VER 01.00, VERSION 01.06, 17 archives | `TXT_LEGEND_TEAMS` (1 team) → `TXT_ROW_LEAGUE` | "Legend Teams" → "Rest Of World" (anchor, 24 251 rows) | same; this title's archive set and DB placement have not been measured at all |

For each: the archive entry index/offset/kind of the DB member is **title-specific** and must be
measured on that install (the stager binds by path and refuses deduplicated or compressed
members; a title whose DB member is compressed is **blocked** for E1 until a codec-aware
replacement exists — record it, do not work around it).

### 4.2 PS3 / Xbox 360 / Recompiled — no direct identity field (blocked pending Phase 1.3)

Targets: NHL Legacy (PS3, Xbox 360, Recompiled), NHL 15 · 14 · 13 · 12 · 11 · 10 · 09 · 08
(PS3, Xbox 360), NHL 07 (Xbox 360) — each with its own verdict row in the matrix §0.2.

* **What a Phase 1.1 run can establish here: nothing about identity.** The main DB carries a
  league *code* on the teams and no name; the game's label source for a code is unresolved
  (`nhl_eng_us.db` is the candidate; UI data or the executable are not excluded). The identity
  experiment is a **Phase 1.3** one (change the candidate string, observe the label) and waits for
  the source trace.
* **What is worth doing now, per target, as G0 only:** record the environment (RPCS3 / Xenia /
  Recompiled build, title id, disc/update), boot to the team-select surface, screenshot the league
  labels the game shows for the codes in `defs/leagues/*.toml` — this **corroborates or refutes
  the candidate vocabularies** (NHL 09 code 8 "CEN", NHL 10 code 9 "NAJ/WPR", NHL 07 code 5
  "DAV"): mapping evidence, not an edit. Note where the game's DB is read from (PS3
  `USRDIR/nhlng.db` loose; Xbox 360 packed `.big` — packaging not established).
* Recompiled lacks a fixture: **missing-fixture** until its tree is supplied.
* The equivalent *membership* operation (team code change) is Phase 1.2 and is not a proxy.

### 4.3 What a passing NHL 21 card does and does not carry over

Nothing. Each PS4 card needs its own provenance check, member measurement, dictionary and boot;
each gen7 target needs 1.3 first. Identical anchor bytes (PS3/Xbox 360 pairs; NHL 19/20) mean
one *measurement*, never one *qualification*.

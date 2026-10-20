# NHL 27 backport editor contracts

These contracts define future Studio work. They do not enable write controls or assert working ports. Research implementation: [dossier](nhl27-backport-2026-10-15.md).

Compatibility follow-up: [animation, presentation and play-by-play findings](nhl27-port-compatibility-2026-10-15.md). The five byte-exact camera controls qualify only a narrow container normalization. Recovered source/target layouts prove real commentary field permutations even where object sizes match. No generic EBX signature replacement, texture-header swap, ANT/AF transplant or sound-bank replacement is authorized by those results as a supported Studio operation.

Every executable control needs a source SHA-256, target SHA-256, image VA-to-file mapping, expected original bytes, type/range, consumer evidence, dependencies, persistence domain, conflict identifiers, rollback bytes and validation receipt. Reject unknown builds and ambiguous matches. Keep NHL 24 base and 1.60 independently qualified. Runtime object addresses are not file patch addresses. Apply to disposable copies first; retain originals and record output hashes.

Evidence levels are `lead`, `static-traced`, `isolated-tested`, `runtime-qualified`. A higher level applies only to its named claim, not the surrounding feature. Read-only discovery can display leads; executable writes require a separately qualified target patch recipe. Static constants can be displayed without promising functional changes.

| Editor area | Read-only implementation contract | Write prerequisite |
|---|---|---|
| Strategies | Compare playbook schemas, IDs, descriptions, art, slider ranges and service availability | Extract actual rows; resolve tactics/formation consumers and per-team save fields; qualify one tactic end-to-end |
| Franchise | Show conversation contexts, contract/clause schemas and offline versus connected dependencies | NHL 24-native state/services/frontend adapter; save migration and AI decision checks |
| PWHL | Separate display league ID, gameplay classification, club assets and competition registration | Roster, schedule, presentation and rule dependencies validated together |
| Jailbreak | Show Rule_JailBreak, core 0x436 and settings 0xc70 as distinct fields | Prove mapping and goal/penalty consumer; qualify simulation and saved modes |
| Be a Pro | Compare progression, endorsement, objective and perk state dependencies | Implement compatible NHL 24 state and transaction semantics, then new-career tests before migration |
| Animations/dekes | Compare moves, clips, rigs, graphs, event markers and input/AI selection dependencies | Qualify one complete move including puck synchronization, collision, interruptions, handedness and human/CPU execution |
| Pregame/in-game presentation | Inspect sequences, cameras, overlays, event bindings and team/league asset mappings | Compatible timelines/UI services and renderer assets; validate skip/resume and gameplay handoff across game states |
| Play-by-play | Inspect banks/codecs, phrase/event IDs, pronunciation mappings and scheduler dependencies; separate commentary, arena and narrative audio | NHL 24 event/bank/name adapters with contextual selection, queue/interrupt timing, fallback and language tests |
| League sizes | Display 48-slot structural bound and requested 48-team ceiling | Test schedule generation, alignment, playoffs, UI and season transitions for each supported configuration |
| Short rosters | Display separate eligibility, playable, total and positional constraints | Safe line assignment, special teams, injuries, fatigue, substitutions and simulation with 8–10 skaters |
| Sliders/attributes/tweakables | Name, category, build, type, default, range, curve, exact consumers, evidence level | Derive units/formula/interactions; target-build write recipe and measured gameplay outcomes |
| Assets/capacities | Separate registry IDs, storage widths, bundle membership and runtime capacities | Prove registration and allocation boundaries; no capacity promises inferred solely from ID width |
| Stats/simulation | Show event and simulation leads separately | Trace event eligibility, attribution, commit and seeded simulation outcomes |

For sliders and ratings, each final description must identify input domain, normalization, clamps, lookup/interpolation, multipliers, mode/CPU-human/position gates, update cadence, interactions and observable outputs. Missing pieces remain explicitly unknown. Do not turn an ambiguous in-game label into an equally ambiguous editor tooltip.

Runtime receipts should identify executable hash, asset set, emulator/native build, save seed, reproduction steps, expected result, observed result and rollback. Strategy and rule ports require both played and simulated games where applicable. Capacity changes require boundary cases and save/reload. Frontend success alone cannot qualify engine behavior.

Recommended implementation order: source/build identity and evidence browser; playbook/schema inspector; one shared strategy adapter; PWHL league/rule dependency inspector; proven Jailbreak enforcement; isolated Franchise contracts/conversations; Be a Pro state work. Preserve earlier NHL 24 tweakable tooling, but do not import NHL 27 addresses automatically: the conservative registration matcher produced zero qualified matches.

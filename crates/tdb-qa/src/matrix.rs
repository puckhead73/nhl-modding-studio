//! The QA ledger: one row per (game × platform × feature family). Status is derived **conservatively**
//! from fixture presence plus the committed automated gates: the Phase-1 read matrix (db-presentation)
//! and the Phase-2 scalar mutation matrix (db-operations). `game-verified` is never assigned here — it
//! requires manual in-game evidence — and structural roster operations remain Phase 3.
//!
//! Derivation rules (documented in `docs/audits/qa-ledger.md`):
//!   * db-presentation on a real main-DB anchor → `read-pass` (the matrix gate); recomp has no anchor
//!     → `missing-fixture` (shares the Xbox Legacy schema).
//!   * db-operations on a real main-DB anchor → `edit-roundtrip-pass`, proven by two gates: the Phase-2
//!     scalar mutation matrix (every distinct codec read/write-inverts) and the Phase-3 relational
//!     operation matrix (every implemented roster operation holds the invariant battery + round-trips,
//!     and schema-absent operations refuse cleanly). The blocker now scopes only in-game acceptance
//!     (game-verified) as outstanding.
//!   * save-raw-db → `edit-roundtrip-pass` (Phase-4: the loose-DB save is a byte-identical no-op round
//!     trip); save-inner-container → `edit-roundtrip-pass` where a save fixture ships (Phase-4 container
//!     matrix: detect-by-content → extract → edit → repack → reopen with valid checksums), with the
//!     v1 (NHL10/11) and PS4 experimental caveats preserved in the blocker; onboarding-install →
//!     `discovered` when the backing fixture exists.
//!   * save-outer-package → `blocked` on Xbox (STFS not implemented), else `not-applicable`.
//!   * onboarding-localization → `discovered` if a localization DB ships, else `missing-fixture`.
//!   * every asset family → `missing-fixture` (no asset fixtures live under `anchors/`).
//!   * the PS3 NHL 07 research copy carries its real read status but every row is flagged as a
//!     non-catalog research copy in `blocker`.

use crate::catalog::{CatalogUnit, UnitKind, CATALOG, FAMILIES};

/// Phase-0 establishment date (the read-presentation gate's `last_verified`).
const PHASE0_DATE: &str = "2026-08-01";
/// Phase-3 establishment date — the db-operations `last_verified` (scalar mutation matrix landed
/// 2026-08-02; the relational operation matrix the same day, superseding it as the cited gate).
const PHASE3_DATE: &str = "2026-08-02";
/// Phase-4 establishment date — the save-path `last_verified` (container round-trip matrix).
const PHASE4_DATE: &str = "2026-08-02";
/// The Phase-1 presentation gate: the full 27-anchor read matrix over deterministic representatives
/// (players by position/roster-state/companion; teams), asserting profile targets, editor honesty,
/// FK/enum resolution, and surface-capability agreement. Supersedes the Phase-0 addressing check.
const READ_GATE: &str = "studio_domain::read_matrix::read_matrix_over_all_anchors";
/// The Phase-3 relational-operation gate: every implemented roster operation (player create/clone/
/// delete/move/trade/release/sign/membership; captaincy/jersey/dressed/retired-number; draft-pick;
/// career; strategy) runs through the `EditSession` seam on every anchor, holding the invariant
/// battery + save/reopen and reversing cleanly; schema-absent operations refuse without a half-commit.
const OPERATION_GATE: &str = "studio_domain::operation_matrix::operation_matrix_over_all_anchors";
/// The Phase-4 save-container gate: every committed save fixture is detected by content, its inner DB
/// extracted + validated, an edited payload repacked/re-extracted with valid checksums, ROSTER saves
/// take a real edit through the app seam, and truncated input is refused. Unsupported variants (v5
/// settings, multi-part, NHL15-PS4 framing) are recorded read-only, not promoted.
const SAVE_GATE: &str = "studio_domain::save_matrix::save_matrix_over_all_fixtures";

struct Probes {
    has_main_db: bool,
    has_localization: bool,
    has_saves: bool,
    saves_dir: String,
    localization_path: String,
}

fn probe(repo_root: &std::path::Path, u: &CatalogUnit) -> Probes {
    if u.platform == "recomp" {
        return Probes {
            has_main_db: false,
            has_localization: false,
            has_saves: false,
            saves_dir: String::new(),
            localization_path: String::new(),
        };
    }
    let dbdir = repo_root.join(format!("anchors/{}/{}/databases", u.platform, u.game));
    let loc = ["nhl_eng_us.db", "eng_us.db"]
        .iter()
        .map(|n| (n, dbdir.join(n)))
        .find(|(_, p)| p.is_file());
    let saves = repo_root.join(format!("anchors/{}/{}/savefiles", u.platform, u.game));
    let has_saves = saves.is_dir()
        && std::fs::read_dir(&saves)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);
    Probes {
        has_main_db: repo_root.join(u.schema_target).is_file(),
        has_localization: loc.is_some(),
        localization_path: loc
            .map(|(n, _)| format!("anchors/{}/{}/databases/{}", u.platform, u.game, n))
            .unwrap_or_default(),
        has_saves,
        saves_dir: if has_saves {
            format!("anchors/{}/{}/savefiles", u.platform, u.game)
        } else {
            String::new()
        },
    }
}

struct Cell {
    fixture: String,
    status: &'static str,
    automated_test: &'static str,
    manual_case: &'static str,
    last_verified: &'static str,
    evidence: String,
    blocker: String,
}

fn cell(u: &CatalogUnit, family: &str, p: &Probes) -> Cell {
    // Assets: no asset fixtures ship under anchors/ — honest missing across every family/unit.
    if family.starts_with("assets-") {
        return Cell {
            fixture: String::new(),
            status: "missing-fixture",
            automated_test: "",
            manual_case: "",
            last_verified: "",
            evidence: "no asset fixtures under anchors/ (asset codec fixtures live in crate tests)"
                .into(),
            blocker: "no committed asset fixture".into(),
        };
    }

    let main_fx = u.schema_target.to_string();
    let mut c = match family {
        "db-presentation" => {
            if p.has_main_db {
                Cell {
                    fixture: main_fx,
                    status: "read-pass",
                    automated_test: READ_GATE,
                    manual_case: "",
                    last_verified: PHASE0_DATE,
                    evidence: format!(
                        "schema/{}-{}.json; surface-capabilities.csv; 27-anchor read matrix",
                        u.platform, u.game
                    ),
                    blocker: String::new(),
                }
            } else {
                Cell {
                    fixture: main_fx,
                    status: "missing-fixture",
                    automated_test: "",
                    manual_case: "",
                    last_verified: "",
                    evidence: "no independent DB anchor".into(),
                    blocker: "shares Xbox-derived Legacy schema; no own anchor".into(),
                }
            }
        }
        "db-operations" => {
            if p.has_main_db {
                Cell {
                    fixture: main_fx,
                    status: "edit-roundtrip-pass",
                    automated_test: OPERATION_GATE,
                    manual_case: "",
                    last_verified: PHASE3_DATE,
                    evidence: format!(
                        "schema/{}-{}.json; mutation-matrix.csv; operation-matrix.csv; \
                         27-anchor scalar mutation + relational operation matrices",
                        u.platform, u.game
                    ),
                    blocker: "scalar/string/enum/FK codecs (Phase 2) + structural roster operations \
                              (Phase 3) edit-roundtrip-proven; in-game acceptance is game-verified (manual)"
                        .into(),
                }
            } else {
                Cell {
                    fixture: main_fx,
                    status: "missing-fixture",
                    automated_test: "",
                    manual_case: "",
                    last_verified: "",
                    evidence: "no independent DB anchor (shares Xbox-derived Legacy schema)".into(),
                    blocker: "no own anchor".into(),
                }
            }
        }
        "save-raw-db" => Cell {
            fixture: main_fx,
            status: if p.has_main_db { "edit-roundtrip-pass" } else { "not-applicable" },
            automated_test: if p.has_main_db { SAVE_GATE } else { "" },
            manual_case: "",
            last_verified: if p.has_main_db { PHASE4_DATE } else { "" },
            evidence: "loose-DB save is a byte-identical no-op round trip (Phase 2 mutation matrix + save-matrix.csv)".into(),
            blocker: if p.has_main_db {
                "loose-DB round trip proven; in-game acceptance is game-verified (manual)".into()
            } else {
                String::new()
            },
        },
        "save-inner-container" => {
            if p.has_saves {
                Cell {
                    fixture: p.saves_dir.clone(),
                    status: "edit-roundtrip-pass",
                    automated_test: SAVE_GATE,
                    manual_case: "",
                    last_verified: PHASE4_DATE,
                    evidence: "save-matrix.csv: roster container detected-by-content, extract→edit→repack→reopen with valid checksums".into(),
                    blocker: "roster inner-container round-trip proven (Phase 4); non-roster/multi-part/v5-settings saves read-only; \
                              in-game acceptance is game-verified (manual)"
                        .into(),
                }
            } else {
                Cell {
                    fixture: String::new(),
                    status: "missing-fixture",
                    automated_test: "",
                    manual_case: "",
                    last_verified: "",
                    evidence: "no committed save fixture".into(),
                    blocker: "save is user-supplied; no anchor fixture".into(),
                }
            }
        }
        "save-outer-package" => match u.platform {
            "xbox360" => Cell {
                fixture: String::new(),
                status: "blocked",
                automated_test: "",
                manual_case: "",
                last_verified: "",
                evidence: "anchors expose inner save payloads; STFS outer package not implemented".into(),
                blocker: "Xbox STFS packaging not implemented".into(),
            },
            _ => Cell {
                fixture: String::new(),
                status: "not-applicable",
                automated_test: "",
                manual_case: "",
                last_verified: "",
                evidence: "anchors expose inner save payloads; no outer package layer handled here".into(),
                blocker: String::new(),
            },
        },
        "onboarding-install" => Cell {
            fixture: format!("anchors/{}/{}/", u.platform, u.game),
            status: "discovered",
            automated_test: "",
            manual_case: "",
            last_verified: "",
            evidence: "studio-domain discovery install detection".into(),
            blocker: String::new(),
        },
        "onboarding-localization" => {
            if p.has_localization {
                Cell {
                    fixture: p.localization_path.clone(),
                    status: "discovered",
                    automated_test: "",
                    manual_case: "",
                    last_verified: "",
                    evidence: "localization DB present".into(),
                    blocker: String::new(),
                }
            } else {
                Cell {
                    fixture: String::new(),
                    status: "missing-fixture",
                    automated_test: "",
                    manual_case: "",
                    last_verified: "",
                    evidence: "localization DB not shipped (meta may be present)".into(),
                    blocker: "no localization DB anchor".into(),
                }
            }
        }
        _ => Cell {
            fixture: String::new(),
            status: "discovered",
            automated_test: "",
            manual_case: "",
            last_verified: "",
            evidence: String::new(),
            blocker: String::new(),
        },
    };

    // Recomp: no anchor tree — install detection exists, but saves/localization are user-supplied.
    if u.platform == "recomp" {
        match family {
            "onboarding-install" => {
                c.status = "discovered";
                c.evidence = "discovery.rs detect_recomp (Xbox-derived tree)".into();
            }
            "onboarding-localization" => {
                c.status = "not-applicable";
                c.evidence = "uses the Xbox-derived game tree's localization".into();
                c.blocker = String::new();
            }
            "save-raw-db" | "save-inner-container" => {
                c.status = "missing-fixture";
                c.evidence = "recomp save is user-supplied; no committed fixture".into();
                c.blocker = "no recomp save fixture".into();
            }
            _ => {}
        }
    }

    // NHL 10/11 gen7 inner container: v1 write path is experimental.
    if family == "save-inner-container"
        && (u.game == "nhl10" || u.game == "nhl11")
        && (u.platform == "ps3" || u.platform == "xbox360")
    {
        c.blocker =
            "v1 container write experimental (tag_1c unreversed) — needs in-game validation".into();
    }
    // PS4 save repack is structurally complete but unverified on-console.
    if family == "save-inner-container" && u.platform == "ps4" {
        c.blocker = "PS4 save repack unverified on-console/emulator".into();
    }

    // Research copy: flag every row as a non-catalog platform.
    if u.kind == UnitKind::Research {
        let note = "research copy — not a supported catalog platform (compat matrix '—')";
        c.blocker = if c.blocker.is_empty() {
            note.into()
        } else {
            format!("{note}; {}", c.blocker)
        };
    }

    c
}

fn csv_cell(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Build the QA-matrix CSV. Rows are emitted in catalog order × family order (both fixed) — no sort
/// needed, and repeated runs are byte-identical.
pub fn build_csv(repo_root: &std::path::Path) -> String {
    let mut s = String::from(
        "game,platform,generation,fixture,status,automated_test,manual_case,last_verified,evidence,blocker\n",
    );
    for u in CATALOG {
        let p = probe(repo_root, u);
        for fam in FAMILIES {
            let c = cell(u, fam.id, &p);
            s.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                csv_cell(u.game),
                csv_cell(u.platform),
                csv_cell(u.generation),
                csv_cell(&c.fixture),
                csv_cell(c.status),
                csv_cell(c.automated_test),
                csv_cell(c.manual_case),
                csv_cell(c.last_verified),
                csv_cell(&c.evidence),
                csv_cell(&c.blocker),
            ));
        }
    }
    s
}

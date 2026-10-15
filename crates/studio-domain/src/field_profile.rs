//! **Server-driven per-game field profile** — the curated Player/Team editor layout, assembled in
//! the domain layer from the *loaded* schema so the frontend renders exactly the fields a title
//! carries and nothing it doesn't.
//!
//! ## Why this exists
//!
//! The curated detail sections used to hardcode a fixed set of **modern studio field ids** in React.
//! On a title whose schema differs (gen6 NHL07 especially) that produced three defects: phantom "not
//! in this game" cells for fields the title lacks, *hidden* real fields that live under a different
//! id, and derived values scaled by a modern assumption. This module moves the section layout here
//! and resolves every cell by **metadata `name`** (with a small synonym list per logical cell),
//! which is stable across the title range where the studio id is not — e.g. the draft year is
//! `yeardrafted` on NHL07/10 and `draftyear` on NHL14+, two different ids but one meaning.
//!
//! ## Evidence-first, never title-keyed
//!
//! A cell is emitted only when the loaded record actually carries a field with one of its names; a
//! section with no surviving cells is dropped. Nothing here branches on the title id — the same
//! spec run against any anchor yields that anchor's real surface. This is the version-aware hard
//! rule expressed as data: the profile *is* the probe result.

use crate::view::{FieldView, PlayerData};

/// How a cell should be edited in the UI. `Field` lets the frontend pick a control from the field's
/// own shape (enum→dropdown, 0/1→checkbox, string→text, else number); the rest are explicit hints
/// the shape alone can't convey.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellControl {
    /// A plain editable field — control inferred from the field shape.
    Field,
    /// A team foreign-key picker. `offset` is what the raw encodes (`raw - offset` = team id); only
    /// emitted for fields whose FK semantics are verified (see the builders).
    Team {
        offset: u8,
        none_label: Option<String>,
    },
    /// The home-arena picker (`kOtt.buDa → StEO`).
    Arena,
    /// A read-only derived value (cap hit, contract status, multi-year, archetype) with no single
    /// backing field to edit — `derived_display` holds the resolved text.
    Derived,
    /// A team colour (Phase 2.1, C8 overnight 2026-09-20): either one packed 24-bit word
    /// (`components` = the one field; `(r << 16) | (g << 8) | b` — measured: NHL 16's
    /// `feprimarycolor_r/g/b` triple `dd 9b 26` for Boston is NHL 17–21's `feprimarycolor`
    /// `0xdd9b26`, and NHL 15's `primarycolor` triple `ff ad 0f` is NHL 17's `presprimarycolor`
    /// `0xffad0f`) or three byte fields (`components` = `_r`, `_g`, `_b`, NHL 11–15 / Legacy / PS4
    /// 15–16). The UI edits the three components as **one** operation (`set_cells_int`).
    Color,
}

/// One labelled cell of a curated section.
#[derive(Clone, Debug)]
pub struct ProfileCell {
    /// The human label (the localized surface — never a raw id).
    pub label: String,
    pub control: CellControl,
    /// The resolved field to read/edit, when the cell is backed by one. `None` for `Derived`.
    pub field: Option<FieldView>,
    /// The record index the cell's `set_field` must target. Usually the entity's own record, but for
    /// a NHL07 team-name cell it is the joined stock/custom row that actually holds the name.
    pub record: u32,
    /// Resolved display text for a `Derived` cell.
    pub derived_display: Option<String>,
    /// When `Some`, this cell's value semantics are not yet ground-truthed for the loaded title —
    /// the UI shows the raw value plus this note (see [`crate::view`] Stage-4 probes).
    pub unverified: Option<String>,
    /// `Color` cells: the fields that make up the colour, in order — one packed word, or `_r`,
    /// `_g`, `_b`. Empty for every other control.
    pub color_components: Vec<FieldView>,
}

/// A titled group of cells (Biographical, Roster, Contract, …).
#[derive(Clone, Debug)]
pub struct ProfileSection {
    pub title: String,
    pub cells: Vec<ProfileCell>,
}

/// The full curated layout for one entity — the sections the UI renders in order.
#[derive(Clone, Debug, Default)]
pub struct EntityFieldProfile {
    pub sections: Vec<ProfileSection>,
}

impl EntityFieldProfile {
    fn push_section(&mut self, title: &str, cells: Vec<ProfileCell>) {
        if !cells.is_empty() {
            self.sections.push(ProfileSection {
                title: title.to_string(),
                cells,
            });
        }
    }
}

/// Find a field by any of its candidate metadata names (first match wins, in `names` order).
fn by_names<'a>(fields: &'a [FieldView], names: &[&str]) -> Option<&'a FieldView> {
    names
        .iter()
        .find_map(|n| fields.iter().find(|f| f.name == *n))
}

/// A plain editable-field cell, emitted iff one of `names` is present in `fields`.
fn field_cell(
    fields: &[FieldView],
    record: u32,
    label: &str,
    names: &[&str],
) -> Option<ProfileCell> {
    by_names(fields, names).map(|f| ProfileCell {
        label: label.to_string(),
        control: CellControl::Field,
        field: Some(f.clone()),
        record,
        derived_display: None,
        unverified: None,
        color_components: Vec::new(),
    })
}

/// A team-FK picker cell, emitted iff one of `names` is present. `offset` is the raw encoding
/// (`raw - offset` = team id); `none_label` names the empty option.
fn team_cell(
    fields: &[FieldView],
    record: u32,
    label: &str,
    names: &[&str],
    offset: u8,
    none_label: Option<&str>,
) -> Option<ProfileCell> {
    by_names(fields, names).map(|f| ProfileCell {
        label: label.to_string(),
        control: CellControl::Team {
            offset,
            none_label: none_label.map(str::to_string),
        },
        field: Some(f.clone()),
        record,
        derived_display: None,
        unverified: None,
        color_components: Vec::new(),
    })
}

/// A read-only derived cell, emitted iff `gate` is present in `fields` (the field the value is
/// derived from). `display` is the already-resolved text.
fn derived_cell(
    fields: &[FieldView],
    label: &str,
    gate: &[&str],
    display: Option<String>,
) -> Option<ProfileCell> {
    by_names(fields, gate).map(|_| ProfileCell {
        label: label.to_string(),
        control: CellControl::Derived,
        field: None,
        record: 0,
        derived_display: display,
        unverified: None,
        color_components: Vec::new(),
    })
}

fn yes_no(b: Option<bool>) -> Option<String> {
    b.map(|b| {
        if b {
            "Yes".to_string()
        } else {
            "No".to_string()
        }
    })
}

/// Everything the player profile needs beyond the bio field list, resolved by the caller
/// ([`crate::view::player_view`]).
pub struct PlayerProfileCtx<'a> {
    pub bio_record: u32,
    /// The probed `dXSB` main-team encoding offset (0 on NHL 09/10, 1 on Legacy/11→21).
    pub main_team_offset: u8,
    /// Whether the ratings companion carries the style/archetype enum (`QgFs`).
    pub has_style: bool,
    /// The resolved archetype label for the derived Archetype cell.
    pub style_label: Option<&'a str>,
    /// Resolved contract/derived values (cap hit, status, multi-year) for the derived cells.
    pub data: &'a PlayerData,
    /// Optional per-cell "unverified" notes keyed by cell label — the Stage-4 probes attach these.
    pub unverified: &'a dyn Fn(&str) -> Option<String>,
}

/// Build the curated Player Data layout from the bio field views + resolved context. Each cell is
/// emitted only when its backing field is present in `bio_fields`, so absent-schema fields simply
/// never render (no phantom cells), and gen6-only fields (`yeardrafted`/`positiondrafted`) surface
/// automatically via the name synonyms.
pub fn player_field_profile(
    bio_fields: &[FieldView],
    ctx: &PlayerProfileCtx,
) -> EntityFieldProfile {
    let rec = ctx.bio_record;
    let mut p = EntityFieldProfile::default();

    p.push_section(
        "Biographical Data",
        [
            field_cell(bio_fields, rec, "First Name", &["firstname"]),
            field_cell(bio_fields, rec, "Last Name", &["lastname"]),
            field_cell(bio_fields, rec, "Height", &["height"]),
            field_cell(bio_fields, rec, "Weight", &["weight"]),
            field_cell(bio_fields, rec, "Birth Year", &["year"]),
            field_cell(bio_fields, rec, "Birth Month", &["month"]),
            field_cell(bio_fields, rec, "Birth Day", &["day"]),
            field_cell(bio_fields, rec, "Birthplace", &["birthplace"]),
            field_cell(bio_fields, rec, "Birth State", &["state", "statename"]),
            field_cell(bio_fields, rec, "Nationality", &["intlcountry"]),
            field_cell(bio_fields, rec, "Position", &["position"]),
            field_cell(bio_fields, rec, "Handedness", &["handedness"]),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );

    let mut roster = Vec::new();
    roster.extend(field_cell(bio_fields, rec, "Jersey Number", &["jersey"]));
    roster.extend(team_cell(
        bio_fields,
        rec,
        "Main Team",
        &["team"],
        ctx.main_team_offset,
        None,
    ));
    roster.extend(team_cell(
        bio_fields,
        rec,
        "Junior Team",
        &["juniorteam"],
        1,
        None,
    ));
    roster.extend(field_cell(bio_fields, rec, "Active", &["active"]));
    if ctx.has_style {
        roster.push(ProfileCell {
            label: "Archetype".to_string(),
            control: CellControl::Derived,
            field: None,
            record: 0,
            derived_display: ctx.style_label.map(str::to_string),
            unverified: None,
            color_components: Vec::new(),
        });
    }
    p.push_section("Roster Data", roster);

    p.push_section(
        "Contract Data",
        [
            field_cell(bio_fields, rec, "Contract Amount", &["contractdollars"]),
            team_cell(
                bio_fields,
                rec,
                "Pro Team",
                &["proteam"],
                1,
                Some("(none — No NHL Rights)"),
            ),
            derived_cell(
                bio_fields,
                "Cap Hit",
                &["contractdollars"],
                ctx.data.cap_hit.clone(),
            ),
            field_cell(bio_fields, rec, "Length Of Contract", &["contractlength"]),
            derived_cell(
                bio_fields,
                "Contract Status",
                &["contractdollars"],
                ctx.data.contract_status.clone(),
            ),
            field_cell(bio_fields, rec, "Year UFA", &["yearufa"]),
            field_cell(bio_fields, rec, "Two-Way", &["contractis2way"]),
            derived_cell(
                bio_fields,
                "Multi-Year",
                &["contractlength"],
                yes_no(ctx.data.is_multi_year),
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );

    p.push_section(
        "Rookie Status",
        [
            field_cell(bio_fields, rec, "NHL Rookie", &["rookie"]),
            field_cell(bio_fields, rec, "AHL Rookie", &["rookieahl"]),
            field_cell(bio_fields, rec, "CHL Rookie", &["rookiechl"]),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );

    p.push_section(
        "Draft Data",
        [
            team_cell(
                bio_fields,
                rec,
                "Draft Team",
                &["draftteam"],
                1,
                Some("(undrafted)"),
            ),
            field_cell(bio_fields, rec, "Draft Year", &["draftyear", "yeardrafted"]),
            field_cell(bio_fields, rec, "Draft Round", &["draftround"]),
            field_cell(
                bio_fields,
                rec,
                "Draft Position",
                &["draftposition", "positiondrafted"],
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );

    // Attach any Stage-4 "unverified" notes to the cells they apply to.
    for s in &mut p.sections {
        for c in &mut s.cells {
            if c.unverified.is_none() {
                c.unverified = (ctx.unverified)(&c.label);
            }
        }
    }

    p
}

/// Everything the team profile needs, resolved by [`crate::view::team_view`].
pub struct TeamProfileCtx<'a> {
    /// The name/classification field views from wherever the names live — `kOtt` on fat schemas, the
    /// joined `HqzU`/`fLEY` stock/custom row on NHL07's thin index.
    pub identity_fields: &'a [FieldView],
    pub identity_record: u32,
    /// The raw `kOtt` field views (structural + fat-schema classification/relations).
    pub kott_fields: &'a [FieldView],
    pub kott_record: u32,
    /// Whether the team→arena link (`kOtt.buDa`) exists.
    pub has_arena: bool,
}

impl<'a> TeamProfileCtx<'a> {
    /// Resolve a cell's field from whichever source carries the name — identity source first (so
    /// NHL07 names/classification target the stock/custom row), then the raw `kOtt` row.
    fn pick(&self, names: &[&str]) -> Option<(&'a FieldView, u32)> {
        if let Some(f) = by_names(self.identity_fields, names) {
            return Some((f, self.identity_record));
        }
        by_names(self.kott_fields, names).map(|f| (f, self.kott_record))
    }

    /// A plain field cell from the resolving source.
    fn field(&self, label: &str, names: &[&str]) -> Option<ProfileCell> {
        self.pick(names).map(|(f, rec)| ProfileCell {
            label: label.to_string(),
            control: CellControl::Field,
            field: Some(f.clone()),
            record: rec,
            derived_display: None,
            unverified: None,
            color_components: Vec::new(),
        })
    }

    /// A relation cell. Team-FK semantics are only verified on the fat `kOtt` row, so a field that
    /// resolves off the thin-index stock/custom source is exposed as a plain (raw) field instead of
    /// a resolved team picker — honest rather than a mis-resolved dropdown.
    fn relation(&self, label: &str, names: &[&str]) -> Option<ProfileCell> {
        self.pick(names).map(|(f, rec)| {
            let control = if f.studio_table == tdb_relations::entity::ids::TEAM {
                CellControl::Team {
                    offset: 1,
                    none_label: Some("(none)".to_string()),
                }
            } else {
                CellControl::Field
            };
            ProfileCell {
                label: label.to_string(),
                control,
                field: Some(f.clone()),
                record: rec,
                derived_display: None,
                unverified: None,
                color_components: Vec::new(),
            }
        })
    }

    /// A colour cell for `stem` on the raw `kOtt` row: the packed word `<stem>` (PS4 17+) or the
    /// `<stem>_r/_g/_b` triple (NHL 11–15 / Legacy / PS4 15–16). `None` when the title has neither.
    fn color(&self, label: &str, stems: &[&str]) -> Option<ProfileCell> {
        for stem in stems {
            if let Some(packed) = by_names(self.kott_fields, &[stem]) {
                return Some(ProfileCell {
                    label: label.to_string(),
                    control: CellControl::Color,
                    field: Some(packed.clone()),
                    record: self.kott_record,
                    derived_display: None,
                    unverified: None,
                    color_components: vec![packed.clone()],
                });
            }
            let (r, g, b) = (format!("{stem}_r"), format!("{stem}_g"), format!("{stem}_b"));
            if let (Some(r), Some(g), Some(b)) = (by_names(self.kott_fields, &[&r]), by_names(self.kott_fields, &[&g]), by_names(self.kott_fields, &[&b])) {
                return Some(ProfileCell {
                    label: label.to_string(),
                    control: CellControl::Color,
                    field: Some(r.clone()),
                    record: self.kott_record,
                    derived_display: None,
                    unverified: None,
                    color_components: vec![r.clone(), g.clone(), b.clone()],
                });
            }
        }
        None
    }
}

/// Build the curated Team layout. Name/classification cells route through the identity source so
/// NHL07's stock/custom-table names become editable; the Venue picker is emitted only when the
/// arena link exists.
pub fn team_field_profile(ctx: &TeamProfileCtx) -> EntityFieldProfile {
    let mut p = EntityFieldProfile::default();

    p.push_section(
        "Identity",
        [
            ctx.field("Full Name", &["fullname"]),
            // PS4 NHL 17+ split the name: `cityname` + `teamname` (the display name) and a
            // `nickname` (NHL 16+; commentary/tweets); NHL 16 keeps `shortname` + `nickname`.
            ctx.field("City", &["cityname"]),
            ctx.field("Team Name", &["teamname"]),
            ctx.field("Nickname", &["nickname"]),
            ctx.field("Abbreviation", &["abbrname"]),
            ctx.field("Short Name", &["shortname"]),
            ctx.field("Art Abbr", &["artabbr"]),
            ctx.field("State / Region", &["state", "statename"]),
            ctx.field("Coach ID", &["coachheadid", "coachid"]),
            // Phase 2.1 (21 Sep 2026): the art id is never edited as a bare cell — its
            // consumers (jersey / award rows, art files keyed by the number) would dangle. The
            // supported change is the art-id swap (`team_reid`); the cell is read-only here.
            ctx.field("Art ID", &["artid"]).map(|mut c| {
                if let Some(f) = c.field.as_mut() {
                    // The identity source is the team table (`exhibitionteams`) or NHL 07's
                    // stock / custom team table — all guarded.
                    if f.name == "artid" {
                        f.editable = false;
                    }
                }
                c
            }),
            ctx.field("Audio ID", &["audioid"]),
            ctx.field("Audio ID 2", &["audioid2"]),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );

    p.push_section(
        "Classification",
        [
            // The league identity is the `league` code (a foreign key into `stockleagues` on
            // PS4, a vocabulary code elsewhere). `leaguegroup` is a separate, unlabelled group-id
            // namespace and is exposed under its own name, never as "League".
            ctx.field("League", &["league"]),
            ctx.field("League group", &["leaguegroup"]),
            ctx.field("Conference", &["conferencegroup", "conference"]),
            ctx.field("Division", &["divisiongroup", "division"]),
            ctx.field("Play Style", &["playstyle"]),
            ctx.field("Active", &["active"]),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );

    if ctx.has_arena {
        if let Some(f) = by_names(ctx.kott_fields, &["arenaid"]) {
            p.push_section(
                "Venue",
                vec![ProfileCell {
                    label: "Arena".to_string(),
                    control: CellControl::Arena,
                    field: Some(f.clone()),
                    record: ctx.kott_record,
                    derived_display: None,
                    unverified: None,
                    color_components: Vec::new(),
                }],
            );
        }
    }

    p.push_section(
        "Relations",
        [
            ctx.relation("AHL Affiliate", &["ahlaffiliate"]),
            ctx.relation("Rival Team", &["rivalteam"]),
            ctx.relation("Rival Team 2", &["rivalteam2"]),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );

    // Team colours (2.1). Which title carries which shape is measured on the anchors: NHL 08–10
    // none on the team row; NHL 11–15 / Legacy (Xbox, PS3) and PS4 15 two `_r/_g/_b` triples;
    // PS4 16 six triples (front-end / presentation / overlay primary + secondary); PS4 17–21 nine
    // packed words (those plus the tertiaries). Stanchion,
    // goal-post and visor colours are presentation (2.5) and are not listed here.
    let colours: Vec<ProfileCell> = [
        ("Primary", &["primarycolor"][..]),
        ("Secondary", &["secondarycolor"]),
        ("Front-end primary", &["feprimarycolor"]),
        ("Front-end secondary", &["fesecondarycolor"]),
        ("Front-end tertiary", &["fetertiarycolor"]),
        ("Presentation primary", &["presprimarycolor"]),
        ("Presentation secondary", &["pressecondarycolor"]),
        ("Presentation tertiary", &["prestertiarycolor"]),
        ("Overlay primary", &["overlayprimarycolor"]),
        ("Overlay secondary", &["overlaysecondarycolor"]),
        ("Overlay tertiary", &["overlaytertiarycolor"]),
    ]
    .iter()
    .filter_map(|(label, stems)| ctx.color(label, stems))
    .collect();
    if !colours.is_empty() {
        p.push_section("Colours", colours);
    }

    p
}

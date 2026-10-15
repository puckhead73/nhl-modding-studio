// Central navigation model — the grouped, two-level IA (primary sections + subtabs).
//
// Adopting the prior app's master-detail grouped-tab structure early (before the surface count grows
// to full parity) so later work slots a subtab into an existing section here instead of crowding a
// flat top bar. Paths are relative to `/app`; section indexes redirect to a default subtab.

export type NavSub = { to: string; label: string };

export type NavSection = {
  key: string;
  label: string;
  /** Default subtab a bare section click lands on (relative to `/app`). Empty sections have none. */
  index?: string;
  subtabs: NavSub[];
};

export const SECTIONS: NavSection[] = [
  {
    key: "database",
    label: "Game Database",
    index: "database/teams",
    subtabs: [
      // Lines lives inside the Team detail (a DetailTabs subtab), not as a primary tab.
      { to: "database/leagues", label: "Leagues" },
      { to: "database/teams", label: "Teams" },
      { to: "database/players", label: "Players" },
      { to: "database/arenas", label: "Arenas" },
      // Phase 1.2 (D2): the read-only mode-setup settings catalogue (NHL 15 tables / 16–21 CSV).
      { to: "database/game-settings", label: "Game Settings" },
      { to: "database/transactions", label: "Transactions" },
    ],
  },
  {
    key: "historical",
    label: "Historical",
    index: "historical/players",
    subtabs: [
      { to: "historical/players", label: "Players" },
      { to: "historical/teams", label: "Teams" },
      { to: "historical/builder", label: "Roster Builder" },
      { to: "historical/formulas", label: "Formulas" },
    ],
  },
  // Legends — the team builder. Each subtab is a curated build strategy; the roster is
  // auto-populated by ranking historical players on calculated OVR (the attribute formulas feeding
  // the EA overall formula), independent of the loaded title.
  {
    key: "legends",
    label: "Legends",
    index: "legends/picks",
    subtabs: [{ to: "legends/picks", label: "Puck's Picks" }],
  },
  // Hockey Shop — per-slot equipment browse / preview / (later) store + install.
  { key: "shop", label: "Hockey Shop", index: "shop", subtabs: [] },
  {
    key: "assets",
    label: "Assets",
    index: "assets/players",
    subtabs: [
      { to: "assets/players", label: "Player Art" },
      { to: "assets/export", label: "Export" },
      { to: "assets/appearance", label: "Appearance" },
      { to: "assets/outfits", label: "Team Outfits" },
      { to: "assets/files", label: "File Browser" },
      { to: "assets/textures", label: "Texture Lab" },
    ],
  },
  {
    key: "settings",
    label: "Settings",
    index: "settings/general",
    subtabs: [
      { to: "settings/general", label: "General" },
      { to: "settings/paths", label: "Paths" },
      // The project Dashboard (overview + validation audit) lives here as a subtab — the app
      // routes to it automatically when the on-load audit finds errors.
      { to: "settings/dashboard", label: "Dashboard" },
    ],
  },
];

/**
 * The Save Editor's trimmed navigation: the database surfaces (no game settings, which are game
 * tunables), the pending-changes review and the integrity audit. No historical import, Hockey Shop
 * or asset tools — a save session has no game installation.
 */
export const SAVE_SECTIONS: NavSection[] = [
  {
    key: "database",
    label: "Save Data",
    index: "database/teams",
    subtabs: [
      { to: "database/leagues", label: "Leagues" },
      { to: "database/teams", label: "Teams" },
      { to: "database/players", label: "Players" },
      { to: "database/arenas", label: "Arenas" },
      { to: "database/transactions", label: "Transactions" },
      { to: "database/pending", label: "Pending Changes" },
    ],
  },
  {
    key: "settings",
    label: "Validation",
    index: "settings/dashboard",
    subtabs: [{ to: "settings/dashboard", label: "Validation" }],
  },
];

/** The section that owns a given `/app/...` pathname (defaults to the first section). */
export function activeSection(pathname: string, sections: NavSection[] = SECTIONS): NavSection {
  const seg = pathname.split("/")[2] ?? "";
  return sections.find((s) => s.key === seg) ?? sections[0];
}

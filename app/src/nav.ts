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
  // Project overview + validation — a leaf primary tab (no subtabs), leftmost like the reference app.
  { key: "dashboard", label: "Dashboard", index: "dashboard", subtabs: [] },
  {
    key: "database",
    label: "Game Database",
    index: "database/teams",
    subtabs: [
      // Lines lives inside the Team detail (a DetailTabs subtab), not as a primary tab.
      { to: "database/teams", label: "Teams" },
      { to: "database/players", label: "Players" },
      { to: "database/arenas", label: "Arenas" },
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
  {
    key: "assets",
    label: "Assets",
    index: "assets/players",
    subtabs: [
      { to: "assets/players", label: "Player Art" },
      { to: "assets/export", label: "Export" },
      { to: "assets/outfits", label: "Team Outfits" },
      { to: "assets/files", label: "File Browser" },
      { to: "assets/textures", label: "Texture Lab" },
    ],
  },
  { key: "builds", label: "Builds", index: "builds", subtabs: [] },
  { key: "settings", label: "Settings", index: "settings", subtabs: [] },
];

/** The section that owns a given `/app/...` pathname (defaults to the first section). */
export function activeSection(pathname: string): NavSection {
  const seg = pathname.split("/")[2] ?? "";
  return SECTIONS.find((s) => s.key === seg) ?? SECTIONS[0];
}

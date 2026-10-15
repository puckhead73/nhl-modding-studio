// Shared team-label helper for the EA-project team pickers (Roster Builder + Send-to-Builder).
//
// Team names resolve differently across the title range: most titles carry a `fullname` on `kOtt`,
// but some store the readable name elsewhere and only `abbrname` is populated on the row — leaving
// `name` empty. A blank option label makes the team unpickable, so always fall back to the best
// available human reference (name → abbreviation → record number) rather than rendering nothing.

/** The minimal shape a team picker needs to label an option. */
export type LabelableTeam = {
  name: string;
  abbrev?: string | null;
  record?: number | null;
};

export function teamLabel(t: LabelableTeam): string {
  const name = t.name?.trim();
  if (name) return name;
  const abbrev = t.abbrev?.trim();
  if (abbrev) return abbrev;
  return t.record != null ? `Team #${t.record}` : "Unnamed team";
}

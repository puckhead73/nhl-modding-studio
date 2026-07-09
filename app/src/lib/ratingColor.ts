// Tier-based colour coding for player overalls + attributes on the EA 0..99 scale. EA-DB ratings are
// native EA values (36..99), independent of any FHM normalisation, so a stored 92 always reads the
// same tone. Thresholds mirror the prior app's nominal-cap scale:
//
//   95+  Generational  purple
//   88+  Elite         blue
//   80+  First Liner   green
//   70+  Average       yellow
//   60+  Depth         orange
//   <60  Non-NHL       red

export type RatingTier =
  | "generational"
  | "elite"
  | "first-liner"
  | "average"
  | "depth"
  | "non-nhl";

const THRESHOLDS: [number, number, number, number, number] = [95, 88, 80, 70, 60];

/** Classify a 0..99 EA rating into its tier, or `null` for a missing/NaN value. */
export function ratingTier(value: number | null | undefined): RatingTier | null {
  if (value === null || value === undefined || Number.isNaN(value)) return null;
  const [gen, elite, first, avg, depth] = THRESHOLDS;
  if (value >= gen) return "generational";
  if (value >= elite) return "elite";
  if (value >= first) return "first-liner";
  if (value >= avg) return "average";
  if (value >= depth) return "depth";
  return "non-nhl";
}

const TIER_TEXT_CLASS: Record<RatingTier, string> = {
  generational: "text-purple-400",
  elite: "text-sky-400",
  "first-liner": "text-emerald-400",
  average: "text-yellow-300",
  depth: "text-orange-400",
  "non-nhl": "text-rose-400",
};

/** Tailwind text-colour class for a 0..99 EA rating; muted grey for null/missing. */
export function ratingToneClass(value: number | null | undefined): string {
  const tier = ratingTier(value);
  return tier === null ? "text-slate-300" : TIER_TEXT_CLASS[tier];
}

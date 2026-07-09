// Position presentation, keyed by the resolved position enum label (C / LW / RW / D / G).
// Badge tones: forwards read warm, defence cool, goalies distinct — shared by the player list
// and the player header so a position always looks the same everywhere.

export const POS_BADGE: Record<string, string> = {
  C: "bg-sky-900/60 text-sky-300",
  LW: "bg-amber-900/50 text-amber-300",
  RW: "bg-amber-900/50 text-amber-300",
  D: "bg-cyan-900/50 text-cyan-300",
  G: "bg-violet-900/50 text-violet-300",
};

/** Full position names for roomier surfaces (the header); unknown labels fall back to themselves. */
export const POS_FULL: Record<string, string> = {
  C: "Center",
  LW: "Left Wing",
  RW: "Right Wing",
  D: "Defenseman",
  G: "Goalie",
};

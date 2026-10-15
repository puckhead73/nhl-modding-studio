import type { CartItem } from "./rosterCart";
import type { PlayerSummaryDto } from "../bindings/PlayerSummaryDto";
export const POSITION_GROUPS = ["forward", "defenseman", "goalie"] as const;
export type PositionGroup = typeof POSITION_GROUPS[number];
export const GROUP_LABELS = { forward: "Forwards", defenseman: "Defensemen", goalie: "Goalies" };
export function positionGroup(position: string | null, goalie = false): PositionGroup {
  if (goalie || /^(G|GK|Goalie|Goaltender|goalie)$/i.test(position ?? "")) return "goalie";
  return /^(D|LD|RD|Defenseman|Defensemen|Defenceman)$/i.test(position ?? "") ? "defenseman" : "forward";
}
export type EstimatedRating = { overall: number | null; group: PositionGroup };
export const ratingKey = (person: number, season: number) => `${person}:${season}`;
export function byOverall(a: number | null | undefined, b: number | null | undefined) { return (b ?? -1) - (a ?? -1); }

/** Initial suggestions only. Reserve all explicit selections before pairing each position group. */
export function suggestReplacements(items: CartItem[], roster: PlayerSummaryDto[], ratings: Map<string, EstimatedRating>) {
  const used = new Set(items.filter(i => !i.assignmentPending && i.matchEaPlayerId != null).map(i => i.matchEaPlayerId!));
  const patches = new Map<string, Partial<CartItem>>();
  const rating = (i: CartItem) => ratings.get(ratingKey(i.personId, i.seasonYear));
  const pending = items.filter(i => i.assignmentPending && i.targetTeamId != null)
    .sort((a, b) => byOverall(rating(a)?.overall, rating(b)?.overall) || a.personName.localeCompare(b.personName) || a.id.localeCompare(b.id));
  for (const item of pending) {
    const group = rating(item)?.group ?? positionGroup(item.position);
    const candidates = roster.filter(p => p.team_id === item.targetTeamId && !used.has(p.player_id) && positionGroup(p.position_label, p.is_goalie) === group)
      .sort((a, b) => byOverall(a.overall, b.overall) || a.player_id - b.player_id);
    const choice = candidates[0];
    if (choice) used.add(choice.player_id);
    patches.set(item.id, { matchEaPlayerId: choice?.player_id ?? null, assignmentPending: false, noPositionSlot: !choice });
  }
  return patches;
}

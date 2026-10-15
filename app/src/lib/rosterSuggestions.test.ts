import { describe, expect, it } from "vitest";
import { positionGroup, ratingKey, suggestReplacements, type EstimatedRating } from "./rosterSuggestions";
import type { CartItem } from "./rosterCart";
import type { PlayerSummaryDto } from "../bindings/PlayerSummaryDto";

export const incoming = (id: number, position = "C", targetTeamId = 1): CartItem => ({ id: String(id), personId: id, seasonYear: 2007, personName: `Player ${id}`, seasonLabel: "2006–07", position, targetTeamId, matchEaPlayerId: null, jersey: null, assignmentPending: true });
export const player = (id: number, position: string, overall: number | null, team_id = 1): PlayerSummaryDto => ({ player_id: id, position_label: position, overall, team_id, is_goalie: position === "G", first_name: "Existing", last_name: String(id), jersey_number: null, style_label: null, is_active: true, rights_team_id: null, bio_record: id, cap_hit_dollars: null, contract_years: null, contract_status: null, is_two_way: null });
const ratings = (items: CartItem[], values: (number | null)[]) => new Map<string, EstimatedRating>(items.map((i, n) => [ratingKey(i.personId, i.seasonYear), { overall: values[n], group: positionGroup(i.position) }]));

describe("initial roster suggestions", () => {
  it("pairs descending OVR separately for forwards, defensemen and goalies", () => {
    const items = [incoming(1), incoming(2, "D"), incoming(3, "G"), incoming(4), incoming(5, "D"), incoming(6, "G")];
    const roster = [player(11, "C", 70), player(12, "LD", 75), player(13, "G", 80), player(14, "RW", 99), player(15, "RD", 95), player(16, "G", 90)];
    const result = suggestReplacements(items, roster, ratings(items, [80, 99, 75, 95, 80, 99]));
    expect(items.map(i => result.get(i.id)?.matchEaPlayerId)).toEqual([11, 15, 13, 14, 12, 16]);
  });
  it("flags overflow without borrowing other positions or teams, and ranks unknown OVR last", () => {
    const items = [incoming(1), incoming(2), incoming(3, "G")];
    const result = suggestReplacements(items, [player(11, "C", 80), player(12, "D", 99), player(13, "C", 99, 2)], ratings(items, [null, 90, 99]));
    expect(result.get("1")).toMatchObject({ matchEaPlayerId: null, noPositionSlot: true, assignmentPending: false });
    expect(result.get("2")?.matchEaPlayerId).toBe(11);
    expect(result.get("3")?.noPositionSlot).toBe(true);
  });
  it("reserves manual choices and never overwrites initialized rows", () => {
    const items = [{ ...incoming(1), assignmentPending: false, matchEaPlayerId: 11 }, incoming(2), incoming(3, "C", 2)];
    const roster = [player(11, "C", 99), player(12, "C", 80), player(13, "C", 90, 2)];
    const scores = ratings(items, [99, 90, 80]);
    const result = suggestReplacements(items, roster, scores);
    expect(result.has("1")).toBe(false);
    expect(result.get("2")?.matchEaPlayerId).toBe(12);
    expect(result.get("3")?.matchEaPlayerId).toBe(13);
    expect(suggestReplacements(items.map(i => ({ ...i, ...result.get(i.id) })), roster, scores).size).toBe(0);
  });
});

// @vitest-environment jsdom
//
// Phase 2.1 (21 Sep 2026) — the identity card: the dependency listing renders the contract's
// dispositions; the swap dispatches with the exact `{projectId, teamA, teamB}` and shows the
// applied summary; a completion that lands after the card moved to another team never shows on
// that team (generation + context binding) while the owning project's summary still refreshes;
// a refusal is shown as text. Coordinator review R1 F2: every completion path — success,
// failure, the summary refresh and its failure — owns its card by the immutable dispatch context
// (cross-project same-team, cross-team, away-and-back, unmount). jsdom only.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
const held: Array<{ cmd: string; resolve: (v: unknown) => void; reject: (e: unknown) => void }> = [];
const gated = new Set<string>();
let failSwap: string | null = null;
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    if (gated.has(cmd)) return new Promise((resolve, reject) => held.push({ cmd, resolve, reject }));
    if (cmd === "team_identity_dependencies") {
      return Promise.resolve({
        team_id: args.teamId,
        team_name: `Team ${args.teamId}`,
        artid: 10 + args.teamId,
        index_dependencies: [{ table: "exhibitionrostertable", field: "team", rows: 25 }],
        artid_dependencies: [{ table: "stockteamjerseys", field: "teamartid", rows: 3 }],
        teams_sharing_artid: [],
        index_change: "refused: renumbering a team is not offered",
        artid_change: "the art id is not edited as a cell",
      });
    }
    if (cmd === "team_artid_swap") {
      if (failSwap) return Promise.reject(new Error(failSwap));
      return Promise.resolve({ team_a: args.teamA, team_b: args.teamB, team_a_name: "A", team_b_name: "B", artid_a: 13, artid_b: 14, tables: [["exhibitionteams", 2], ["stockteamjerseys", 6]], total_cells: 8, summary: "swap art ids of A (13) and B (14)", outcome: { edits: 8, repairs: [] } });
    }
    if (cmd === "project_summary") return Promise.resolve({ project_id: args.projectId, dirty: true });
    return Promise.resolve(null);
  },
}));
const updates: unknown[] = [];
vi.mock("../lib/project", () => ({
  useProject: () => ({ update: (s: unknown) => updates.push(s) }),
}));

import { TeamIdentityCard } from "./TeamIdentityCard";

const teams = [
  { team_id: 3, record: 3, name: "Team 3", abbrev: "T3", league: 0, league_label: null },
  { team_id: 4, record: 4, name: "Team 4", abbrev: "T4", league: 0, league_label: null },
  { team_id: 5, record: 5, name: "Team 5", abbrev: "T5", league: 0, league_label: null },
] as any;

function mount(teamId = 3, projectId = 7) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const tree = (t: number, p: number) => (
    <QueryClientProvider client={qc}>
      <TeamIdentityCard projectId={p} teamId={t} teams={teams} />
    </QueryClientProvider>
  );
  const r = render(tree(teamId, projectId));
  return { ...r, retarget: (t: number, p = projectId) => r.rerender(tree(t, p)) };
}

const depsCalls = () => calls.filter((c) => c.cmd === "team_identity_dependencies");
const swapCalls = () => calls.filter((c) => c.cmd === "team_artid_swap");

/// Choose `other` and dispatch a (gated) swap; returns once the request is held.
async function dispatchHeld(other: string) {
  await waitFor(() => expect(screen.getByTestId("identity-swap-other")).toBeTruthy());
  fireEvent.change(screen.getByTestId("identity-swap-other"), { target: { value: other } });
  fireEvent.click(screen.getByTestId("identity-swap"));
  await waitFor(() => expect(held.filter((h) => h.cmd === "team_artid_swap")).toHaveLength(1));
}

describe("TeamIdentityCard", () => {
  beforeEach(() => {
    calls.length = 0;
    held.length = 0;
    gated.clear();
    updates.length = 0;
    failSwap = null;
  });
  afterEach(cleanup);

  it("lists the dependencies with the contract's dispositions and swaps with the chosen team", async () => {
    mount();
    await waitFor(() => expect(screen.getByTestId("identity-index")).toBeTruthy());
    expect(screen.getByTestId("identity-index").textContent).toContain("refused: renumbering");
    expect(screen.getByTestId("identity-index").textContent).toContain("exhibitionrostertable.team: 25 rows");
    expect(screen.getByTestId("identity-artid").textContent).toContain("stockteamjerseys.teamartid: 3 rows");
    fireEvent.change(screen.getByTestId("identity-swap-other"), { target: { value: "4" } });
    fireEvent.click(screen.getByTestId("identity-swap"));
    await waitFor(() => expect(screen.getByTestId("identity-swap-result")).toBeTruthy());
    const swap = calls.find((c) => c.cmd === "team_artid_swap")!;
    expect(swap.args).toEqual({ projectId: 7, teamA: 3, teamB: 4 });
    expect(screen.getByTestId("identity-swap-result").textContent).toContain("swap art ids of A (13) and B (14) — 8 cells written as one step");
    await waitFor(() => expect(updates).toEqual([{ project_id: 7, dirty: true }]));
  });

  it("a swap completing after the card moved to another team does not show there, and the owning project still refreshes", async () => {
    gated.add("team_artid_swap");
    const m = mount(3);
    await waitFor(() => expect(screen.getByTestId("identity-swap-other")).toBeTruthy());
    fireEvent.change(screen.getByTestId("identity-swap-other"), { target: { value: "5" } });
    fireEvent.click(screen.getByTestId("identity-swap"));
    await waitFor(() => expect(held).toHaveLength(1));
    m.retarget(4);
    await waitFor(() => expect(calls.filter((c) => c.cmd === "team_identity_dependencies").length).toBe(2));
    held[0].resolve({ team_a: 3, team_b: 5, team_a_name: "A", team_b_name: "C", artid_a: 13, artid_b: 15, tables: [], total_cells: 2, summary: "late", outcome: { edits: 2, repairs: [] } });
    await waitFor(() => expect(updates).toEqual([{ project_id: 7, dirty: true }]));
    expect(screen.queryByTestId("identity-swap-result")).toBeNull();
    expect(swapCalls().map((c) => c.args)).toEqual([{ projectId: 7, teamA: 3, teamB: 5 }]);
  });

  // Coordinator review R1 F2 (21 Sep 2026): completion callbacks must own their card by the
  // immutable dispatch context (project, team, generation), not by the values a later render
  // happens to have. The reviewer's reproduction: a failure from project 7 landing on project 8's
  // card for the same team id.
  it("a swap failing after the card moved to another project (same team id) shows nothing there", async () => {
    gated.add("team_artid_swap");
    const m = mount(3, 7);
    await dispatchHeld("4");
    expect(swapCalls()[0].args).toEqual({ projectId: 7, teamA: 3, teamB: 4 });
    m.retarget(3, 8);
    await waitFor(() => expect(depsCalls()).toHaveLength(2));
    expect(depsCalls()[1].args).toEqual({ projectId: 8, teamId: 3 });
    await waitFor(() => expect(screen.getByTestId("identity-swap-other")).toBeTruthy());
    held[0].reject(new Error("project-seven-only-error"));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("identity-swap-error")).toBeNull();
    expect(screen.queryByTestId("identity-swap-result")).toBeNull();
    expect(updates).toHaveLength(0);
    expect(swapCalls()).toHaveLength(1);
    // The other project's card is not "pending" on project 7's request either.
    expect((screen.getByTestId("identity-swap-other") as HTMLSelectElement).disabled).toBe(false);
  });

  it("a swap failing after the card moved to another team (same project) shows nothing there", async () => {
    gated.add("team_artid_swap");
    const m = mount(3, 7);
    await dispatchHeld("4");
    m.retarget(5, 7);
    await waitFor(() => expect(depsCalls()).toHaveLength(2));
    held[0].reject(new Error("team-three-only-error"));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("identity-swap-error")).toBeNull();
    expect(updates).toHaveLength(0);
  });

  it("switching away and back before a completion lands shows neither the result nor the error, while the owning project still refreshes", async () => {
    gated.add("team_artid_swap");
    const m = mount(3, 7);
    await dispatchHeld("4");
    m.retarget(5, 7);
    await waitFor(() => expect(depsCalls()).toHaveLength(2));
    m.retarget(3, 7);
    await waitFor(() => expect(depsCalls()).toHaveLength(3));
    await waitFor(() => expect(screen.getByTestId("identity-swap-other")).toBeTruthy());
    // The card was reset on the way back: fresh selection, nothing pending on it.
    expect((screen.getByTestId("identity-swap-other") as HTMLSelectElement).value).toBe("");
    expect((screen.getByTestId("identity-swap-other") as HTMLSelectElement).disabled).toBe(false);
    held[0].resolve({ team_a: 3, team_b: 4, team_a_name: "A", team_b_name: "B", artid_a: 13, artid_b: 14, tables: [], total_cells: 2, summary: "late", outcome: { edits: 2, repairs: [] } });
    await waitFor(() => expect(updates).toEqual([{ project_id: 7, dirty: true }]));
    expect(screen.queryByTestId("identity-swap-result")).toBeNull();
    expect(screen.queryByTestId("identity-swap-error")).toBeNull();
    // A second dispatch from the fresh card carries the fresh context.
    gated.delete("team_artid_swap");
    fireEvent.change(screen.getByTestId("identity-swap-other"), { target: { value: "5" } });
    fireEvent.click(screen.getByTestId("identity-swap"));
    await waitFor(() => expect(screen.getByTestId("identity-swap-result")).toBeTruthy());
    expect(swapCalls().map((c) => c.args)).toEqual([
      { projectId: 7, teamA: 3, teamB: 4 },
      { projectId: 7, teamA: 3, teamB: 5 },
    ]);
  });

  it("switching away and back before a failure lands shows no error", async () => {
    gated.add("team_artid_swap");
    const m = mount(3, 7);
    await dispatchHeld("4");
    m.retarget(5, 7);
    await waitFor(() => expect(depsCalls()).toHaveLength(2));
    m.retarget(3, 7);
    await waitFor(() => expect(depsCalls()).toHaveLength(3));
    held[0].reject(new Error("stale-failure"));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("identity-swap-error")).toBeNull();
    expect(updates).toHaveLength(0);
  });

  it("a completion after unmount touches nothing", async () => {
    gated.add("team_artid_swap");
    const m = mount(3, 7);
    await dispatchHeld("4");
    m.unmount();
    held[0].resolve({ team_a: 3, team_b: 4, team_a_name: "A", team_b_name: "B", artid_a: 13, artid_b: 14, tables: [], total_cells: 2, summary: "late", outcome: { edits: 2, repairs: [] } });
    await new Promise((r) => setTimeout(r, 20));
    expect(updates).toHaveLength(0);
    expect(screen.queryByTestId("identity-swap-result")).toBeNull();
    // And a failure after unmount likewise.
    calls.length = 0;
    held.length = 0;
    const m2 = mount(3, 7);
    await dispatchHeld("4");
    m2.unmount();
    held[0].reject(new Error("after-unmount"));
    await new Promise((r) => setTimeout(r, 20));
    expect(updates).toHaveLength(0);
    expect(screen.queryByTestId("identity-swap-error")).toBeNull();
  });

  it("a successful swap whose summary refresh fails reports that on the owning card only", async () => {
    gated.add("project_summary");
    mount(3, 7);
    await waitFor(() => expect(screen.getByTestId("identity-swap-other")).toBeTruthy());
    fireEvent.change(screen.getByTestId("identity-swap-other"), { target: { value: "4" } });
    fireEvent.click(screen.getByTestId("identity-swap"));
    await waitFor(() => expect(screen.getByTestId("identity-swap-result")).toBeTruthy());
    await waitFor(() => expect(held.filter((h) => h.cmd === "project_summary")).toHaveLength(1));
    held.find((h) => h.cmd === "project_summary")!.reject(new Error("summary-unavailable"));
    await waitFor(() => expect(screen.getByTestId("identity-swap-error").textContent).toContain("summary-unavailable"));
    expect(screen.getByTestId("identity-swap-result").textContent).toContain("swap art ids of A (13) and B (14)");
    expect(updates).toHaveLength(0);
  });

  it("a failed summary refresh landing after the card moved to another project shows nothing there", async () => {
    gated.add("project_summary");
    const m = mount(3, 7);
    await waitFor(() => expect(screen.getByTestId("identity-swap-other")).toBeTruthy());
    fireEvent.change(screen.getByTestId("identity-swap-other"), { target: { value: "4" } });
    fireEvent.click(screen.getByTestId("identity-swap"));
    await waitFor(() => expect(held.filter((h) => h.cmd === "project_summary")).toHaveLength(1));
    m.retarget(3, 8);
    await waitFor(() => expect(depsCalls().some((c) => c.args.projectId === 8)).toBe(true));
    await waitFor(() => expect(screen.getByTestId("identity-swap-other")).toBeTruthy());
    held.find((h) => h.cmd === "project_summary")!.reject(new Error("summary-unavailable"));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("identity-swap-error")).toBeNull();
    expect(screen.queryByTestId("identity-swap-result")).toBeNull();
    expect(updates).toHaveLength(0);
  });

  it("shows a refusal as text", async () => {
    failSwap = "Team 3's art id 255 is a spare-kit id";
    mount();
    await waitFor(() => expect(screen.getByTestId("identity-swap-other")).toBeTruthy());
    fireEvent.change(screen.getByTestId("identity-swap-other"), { target: { value: "4" } });
    fireEvent.click(screen.getByTestId("identity-swap"));
    await waitFor(() => expect(screen.getByTestId("identity-swap-error").textContent).toContain("spare-kit"));
    expect(updates).toHaveLength(0);
  });
});

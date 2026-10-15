// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { RosterCartProvider } from "../lib/rosterCart";
import { RosterBuilderPage } from "./RosterBuilder";
const call = vi.hoisted(() => vi.fn());
vi.mock("../lib/client", () => ({ call }));
vi.mock("../lib/project", () => ({ useProject: () => ({ project: { project_id: 1 }, update: vi.fn() }) }));
afterEach(() => { cleanup(); localStorage.clear(); call.mockReset(); });

it("initializes ranked groups, leaves overflow advisory, filters replacements and preserves manual choices", async () => {
  localStorage.setItem("nhl-modding-studio.roster-cart.v1", JSON.stringify([
    [1, "Low forward", "C"], [2, "High forward", "RW"], [3, "Defense", "D"], [4, "Goalie", "G"],
  ].map(([id, personName, position]) => ({ id: String(id), personId: id, personName, position, seasonYear: 2007, seasonLabel: "2007", targetTeamId: 1, matchEaPlayerId: null, jersey: null, assignmentPending: true }))));
  const profile = { name: "User formula" };
  const roster = [[11, "C", 90], [12, "D", 85], [13, "G", 99]].map(([id, position, overall]) => ({ player_id: id, first_name: "Existing", last_name: String(id), position_label: position, overall, team_id: 1, is_goalie: position === "G" }));
  call.mockImplementation((cmd, args) => {
    if (cmd === "get_active_formula") return Promise.resolve(profile);
    if (cmd === "compute_ratings") return Promise.resolve({ overall: [80, 95, 90, 99][args.personId - 1], position: ["forward", "forward", "defenseman", "goalie"][args.personId - 1] });
    if (cmd === "list_players") return Promise.resolve(roster);
    if (cmd === "list_teams") return Promise.resolve([{ team_id: 1, name: "Test team" }]);
    if (cmd === "match_ea_player") return Promise.resolve([]);
    throw new Error(`Unexpected command ${cmd}`);
  });
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={qc}><RosterCartProvider><RosterBuilderPage /></RosterCartProvider></QueryClientProvider>);
  await waitFor(() => expect((screen.getByRole("button", { name: "Preview" }) as HTMLButtonElement).disabled).toBe(false));
  const row = (name: string) => within(screen.getByText(name).closest("tr")!);
  const replacement = (name: string) => row(name).getAllByRole("combobox")[1] as HTMLSelectElement;
  expect(replacement("High forward").value).toBe("11");
  expect(replacement("Defense").value).toBe("12");
  expect(replacement("Goalie").value).toBe("13");
  expect(row("Low forward").getByText(/No existing slot available/)).toBeTruthy();
  const labels = screen.getAllByRole("row").map(r => r.textContent);
  expect(labels.findIndex(s => s?.includes("High forward"))).toBeLessThan(labels.findIndex(s => s?.includes("Low forward")));
  expect(screen.getByText("Forwards")).toBeTruthy();
  expect(screen.getByText("Defensemen")).toBeTruthy();
  expect(screen.getByText("Goalies")).toBeTruthy();
  expect([...replacement("High forward").options].some(o => o.value === "13")).toBe(false);
  expect([...replacement("High forward").options].some(o => o.value === "12")).toBe(true);
  expect([...replacement("Goalie").options].some(o => o.value === "11")).toBe(false);
  fireEvent.change(replacement("High forward"), { target: { value: "" } });
  await qc.invalidateQueries({ queryKey: ["list_players"] });
  await waitFor(() => expect(replacement("High forward").value).toBe(""));
  expect(call).toHaveBeenCalledWith("compute_ratings", { personId: 2, seasonYear: 2007, profile });
  expect(call.mock.calls.some(([cmd]) => cmd === "import_preview" || cmd === "import_commit")).toBe(false);
});

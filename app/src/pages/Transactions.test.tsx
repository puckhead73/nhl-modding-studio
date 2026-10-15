// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
import { TransactionsPage } from "./Transactions";

const mock = vi.hoisted(() => ({ call: vi.fn(), update: vi.fn() }));
vi.mock("../lib/client", () => ({ call: mock.call }));
vi.mock("../lib/project", () => ({ useProject: () => ({ project: { project_id: 7 }, update: mock.update }) }));
afterEach(() => { cleanup(); vi.resetAllMocks(); });

it.each([[0, 1], [1, 0]])("trades team zero in either column (%i, %i)", async (left, right) => {
  let finish!: (value: unknown) => void;
  mock.call.mockImplementation(async (cmd: string) => {
    if (cmd === "list_teams") return [0,1].map(team_id => ({ team_id, name: `Club ${team_id}`, abbrev: `C${team_id}`, league: 0, league_label: "NHL" }));
    if (cmd === "list_players") return [0,1].map(team_id => ({ player_id: 100 + team_id, team_id, first_name: `First${team_id}`, last_name: `Last${team_id}`, overall: 80, jersey_number: 10 + team_id }));
    if (cmd === "list_free_agents") return [];
    if (cmd === "trade_slots") return new Promise(resolve => { finish = resolve; });
    if (cmd === "project_summary") return { project_id: 7 };
    throw new Error(`Unexpected command ${cmd}`);
  });
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={client}><MemoryRouter><TransactionsPage /></MemoryRouter></QueryClientProvider>);
  fireEvent.change(await screen.findByLabelText("Team A team"), { target: { value: String(left) } });
  fireEvent.change(screen.getByLabelText("Team B team"), { target: { value: String(right) } });
  const playerName = await screen.findByText(`Last${left}`);
  expect(playerName.closest("a")).toBeNull();
  fireEvent.click(playerName);
  fireEvent.click(screen.getByText(`First${right}`));
  const trade = screen.getByRole("button", { name: "⇄ Trade" }) as HTMLButtonElement;
  await waitFor(() => expect(trade.disabled).toBe(false));
  fireEvent.click(trade);
  await waitFor(() => expect(mock.call).toHaveBeenCalledWith("trade_slots", {
    projectId: 7, aTeam: left, aPlayer: 100 + left, bTeam: right, bPlayer: 100 + right,
  }));
  const pending = client.getQueryData<Array<{ player_id: number; team_id: number }>>(["players", 7])!;
  expect(pending.find(p => p.player_id === 100 + left)?.team_id).toBe(right);
  expect(pending.find(p => p.player_id === 100 + right)?.team_id).toBe(left);
  finish({ edits: 2, repairs: [] });
});

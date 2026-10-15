// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { CareerStatsTab } from "./CareerStatsTab";
import type { PlayerViewDto } from "../bindings/PlayerViewDto";

const mocks = vi.hoisted(() => ({ call: vi.fn(async () => ({})), update: vi.fn() }));
vi.mock("../lib/client", () => ({ call: mocks.call }));
vi.mock("../lib/project", () => ({ useProject: () => ({ update: mocks.update }) }));
afterEach(() => { cleanup(); mocks.call.mockClear(); });

it.each([true, false])("adds a season using this schema's team-code capability (%s)", async (supported) => {
  const client = new QueryClient();
  render(<QueryClientProvider client={client}><CareerStatsTab projectId={1} player={{ kind: "skater", player_id: 2 } as PlayerViewDto} loading={false} career={{ sections: [{ kind: "skater_season", title: "Regular season", table: "tsSy", columns: [], rows: [], team_code_supported: supported }] }} /></QueryClientProvider>);
  fireEvent.click(screen.getByRole("button", { name: /Add Season/i }));
  const add=screen.getByRole("button", { name: "Add" }) as HTMLButtonElement;
  if (supported) {
    expect(add.disabled).toBe(true);
    fireEvent.change(screen.getByLabelText("Team code"), { target: { value: "PIT" } });
  } else {
    expect(screen.queryByLabelText("Team code")).toBeNull();
    expect(add.disabled).toBe(false);
  }
  fireEvent.click(add);
  await waitFor(() => expect(mocks.call).toHaveBeenCalledWith("add_career_season", { projectId: 1, table: "tsSy", playerId: 2, yearEnd: 2016, league: 0, team: supported ? "PIT" : "" }));
  client.clear();
});

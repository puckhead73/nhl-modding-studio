// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
vi.mock("../lib/project", () => ({ useProject: () => ({ project: { project_id: 1 } }), useIsSaveEditor: () => false }));
vi.mock("../lib/breadcrumbs", () => ({ useCrumbLabel: () => {} }));
vi.mock("../components/LeagueLogoCard", () => ({ LeagueLogoCard: () => null }));
vi.mock("../components/LeagueScheduleCard", () => ({ LeagueScheduleCard: () => null }));
vi.mock("./Leagues", () => ({ DictionaryAttach: () => null }));
vi.mock("../lib/client", () => ({ call: async (cmd: string) => cmd === "list_leagues" ? { dictionary: null } : {
  summary: { code: 0, record: null, display: "National Hockey League", label: "National Hockey League", member_teams: 0 },
  fields: [], members: [], structure: null, name_edit_blocker: "No editable row on this target.",
} }));
import { LeagueDetail } from "./LeagueDetail";
afterEach(cleanup);
it("loads an uncached league through the loading-to-detail transition", async () => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={client}><MemoryRouter initialEntries={["/leagues/0"]}><Routes><Route path="/leagues/:id" element={<LeagueDetail />} /></Routes></MemoryRouter></QueryClientProvider>);
  expect(await screen.findByRole("heading", { name: "National Hockey League" })).toBeTruthy();
  expect(screen.getByText("No editable row on this target.")).toBeTruthy();
});

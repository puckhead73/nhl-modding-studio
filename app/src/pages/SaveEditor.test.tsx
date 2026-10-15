// @vitest-environment jsdom
//
// Save Editor landing page: a chosen file is identified by the backend; the page offers the
// leading game/platform candidates, gates a cautioned file behind an explicit acknowledgement,
// shows a refusal without an Open action, and opens the session with the exact chosen target.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { SaveFileIdentityDto } from "../bindings/SaveFileIdentityDto";

const calls: Array<{ cmd: string; args: any }> = [];
const handlers: Record<string, (args: any) => unknown> = {};
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: unknown) => {
    calls.push({ cmd, args });
    const h = handlers[cmd];
    if (!h) return Promise.reject(new Error(`unmocked command ${cmd}`));
    return Promise.resolve().then(() => h(args));
  },
}));
const opened: unknown[][] = [];
vi.mock("../lib/project", () => ({
  useProject: () => ({ open: (...a: unknown[]) => opened.push(a) }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: () => Promise.resolve("C:/xenia/ROSTER") }));

import { SaveEditorPage } from "./SaveEditor";

function identity(over: Partial<SaveFileIdentityDto> = {}): SaveFileIdentityDto {
  const cand = (platform: string, label: string) => ({
    game_id: "nhl14", platform, title: "NHL 14", platform_label: label, exact_schema: true,
    evidence: ["Its database layout matches NHL 14 exactly."], leading: true,
  });
  return {
    path: "C:/xenia/ROSTER", file_name: "ROSTER", format: "container",
    format_label: "PS3 / Xbox 360 save (container v4, compressed)", kind: "roster", kind_label: "Roster save",
    container_version: "v4", checksums_valid: true, writable: true, write_experimental: false, change_list: false,
    game_confident: true, platform_confident: false,
    candidates: [cand("xbox", "Xbox 360"), cand("ps3", "PlayStation 3")],
    decision: "allowed", decision_reason: null, notes: [], path_hints: [], own_tables: 38, supplied_tables: 96,
    ...over,
  };
}

function renderPage() {
  render(<QueryClientProvider client={new QueryClient()}><MemoryRouter initialEntries={["/save-editor"]}><Routes>
    <Route path="/save-editor" element={<SaveEditorPage />} />
    <Route path="/app/database/teams" element={<p>Teams destination</p>} />
  </Routes></MemoryRouter></QueryClientProvider>);
}

beforeEach(() => {
  calls.length = 0;
  opened.length = 0;
  for (const k of Object.keys(handlers)) delete handlers[k];
});
afterEach(cleanup);

it("offers the platforms a shared layout leaves open and opens the chosen one", async () => {
  handlers.save_editor_identify = () => identity();
  handlers.save_editor_open = (a) => ({ summary: { project_id: 7 }, file: identity(), game_id: a.gameId, platform: a.platform, title: "NHL 14", platform_label: "PlayStation 3", read_only: false, supplied_tables: 96 });
  renderPage();
  fireEvent.click(screen.getByRole("button", { name: "Choose file…" }));
  await screen.findByText(/same on several platforms/);
  expect(screen.getByText("Roster save")).toBeTruthy();
  fireEvent.click(screen.getByRole("radio", { name: /PlayStation 3/ }));
  fireEvent.click(screen.getByRole("button", { name: "Open in Save Editor" }));
  await screen.findByText("Teams destination");
  const open = calls.find((c) => c.cmd === "save_editor_open")!;
  expect(open.args).toEqual({ path: "C:/xenia/ROSTER", gameId: "nhl14", platform: "ps3", acknowledged: false, metaOverride: null });
  expect(opened).toHaveLength(1);
  expect(opened[0][1]).toBe("C:/xenia/ROSTER");
});

it("keeps a cautioned save closed until the user acknowledges it", async () => {
  handlers.save_editor_identify = () => identity({ kind: "beagm", kind_label: "Franchise / Be a GM save", decision: "caution", decision_reason: "This is a franchise / be a gm save.", platform_confident: true, candidates: [identity().candidates[0]] });
  renderPage();
  fireEvent.click(screen.getByRole("button", { name: "Choose file…" }));
  await screen.findByText("This is a franchise / be a gm save.");
  const button = screen.getByRole("button", { name: "Open in Save Editor" }) as HTMLButtonElement;
  expect(button.disabled).toBe(true);
  fireEvent.click(screen.getByRole("checkbox"));
  await waitFor(() => expect(button.disabled).toBe(false));
});

it("shows a refusal without an Open action", async () => {
  handlers.save_editor_identify = () => identity({ change_list: true, decision: "refused", decision_reason: "This roster save records changes against the game's own database.", candidates: [identity().candidates[0]] });
  renderPage();
  fireEvent.click(screen.getByRole("button", { name: "Choose file…" }));
  await screen.findByText(/records changes/);
  expect(screen.queryByRole("button", { name: "Open in Save Editor" })).toBeNull();
});

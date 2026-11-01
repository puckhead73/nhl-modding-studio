// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.open }));
import { ProjectProvider, useProject } from "../lib/project";
import { Nhl24Roster } from "./Nhl24Roster";
afterEach(() => { cleanup(); vi.clearAllMocks(); localStorage.clear(); });

const status = (extra: object = {}) => ({ install: "F:/NHL24", overlay_files: [], other_mod_files: 6, emulator_running: null, open_project: null, ...extra });

function Opened() {
  const { project, dbPath } = useProject();
  return <p>opened {project?.project_id} {dbPath}</p>;
}

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={client}><ProjectProvider><MemoryRouter initialEntries={["/nhl24"]}>
    <Routes>
      <Route path="/nhl24" element={<Nhl24Roster />} />
      <Route path="/app/database/teams" element={<Opened />} />
    </Routes>
  </MemoryRouter></ProjectProvider></QueryClientProvider>);
}

it("opens the roster as a project and navigates to the editors", async () => {
  mocks.invoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
    if (cmd === "nhl24_roster_status") return status();
    if (cmd === "nhl24_roster_open") {
      expect(args).toEqual({ install: "F:/NHL24", oodle: "C:/oo2core_8_win64.dll" });
      return { summary: { project_id: 7 }, db_path: "F:/NHL24/studio/working/nhl_game.db", from_overlay: false, status: status({ open_project: 7 }) };
    }
    throw new Error(`Unexpected ${cmd}`);
  });
  renderPage();
  fireEvent.change(screen.getByLabelText("NHL 24 folder"), { target: { value: "F:/NHL24" } });
  fireEvent.change(screen.getByLabelText("Oodle decoder"), { target: { value: "C:/oo2core_8_win64.dll" } });
  expect(await screen.findByText(/game loads its own roster database/)).toBeTruthy();
  expect(screen.getByText(/6 files from other mods/)).toBeTruthy();
  fireEvent.click(screen.getByText("Open roster"));
  expect(await screen.findByText("opened 7 F:/NHL24/studio/working/nhl_game.db")).toBeTruthy();
});

it("restores the game's own roster and blocks restore while open or running", async () => {
  let restored = false;
  mocks.invoke.mockImplementation(async (cmd: string) => {
    if (cmd === "nhl24_roster_status") return restored ? status() : status({ overlay_files: ["Patch/Gen4b/globals.toc", "cas"], emulator_running: "nhl24_fix.exe" });
    if (cmd === "nhl24_roster_restore") { restored = true; return status(); }
    throw new Error(`Unexpected ${cmd}`);
  });
  renderPage();
  fireEvent.change(screen.getByLabelText("NHL 24 folder"), { target: { value: "F:/NHL24" } });
  expect(await screen.findByText(/2 Studio files in game-mods/)).toBeTruthy();
  expect(screen.getByText(/nhl24_fix.exe is running/)).toBeTruthy();
  fireEvent.click(screen.getByText("Restore the game's own roster"));
  await waitFor(() => expect(screen.getByText(/game loads its own roster database/)).toBeTruthy());
  expect(mocks.invoke).toHaveBeenCalledWith("nhl24_roster_restore", { install: "F:/NHL24" });
});

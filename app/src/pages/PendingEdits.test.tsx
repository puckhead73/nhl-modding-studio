// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
const call = vi.hoisted(() => vi.fn());
const reveal = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
vi.mock("@tauri-apps/plugin-opener", () => ({ revealItemInDir: reveal }));
vi.mock("../lib/client", () => ({ call }));
vi.mock("../lib/project", () => ({ useProject: () => ({ project: { project_id: 1, dirty: false }, info: { saves: 2 }, update: vi.fn(), updateInfo: vi.fn() }) }));
vi.mock("../lib/settings", () => ({ useSettings: () => ({ settings: { projectsRoot: "D:/Projects" } }) }));
import { PendingEdits } from "./PendingEdits";
import { loadSentBuilds } from "../lib/modManager";
afterEach(() => { cleanup(); call.mockReset(); reveal.mockClear(); localStorage.clear(); });
it("shows cumulative saved changes collapsed with editor links and persists details", async () => {
  const details = { name: "QA", version: "1.0", author: "", description: "" };
  call.mockImplementation((cmd, args) => Promise.resolve(cmd === "project_set_details" ? args.details : {
    details, dirty: false, notices: [], changes: [{ id: "jersey", groups: ["NHL", "Vancouver", "Players", "Pavol Demitra", "Player data"], label: "Jersey number", before: "38", after: "90", route: "/app/database/players/613" }],
  }));
  render(<MemoryRouter><QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><PendingEdits /></QueryClientProvider></MemoryRouter>);
  await screen.findByText("Project Details");
  expect(screen.queryByText("Deploy to the game")).toBeNull();
  expect((screen.getByRole("button", { name: "BUILD" }) as HTMLButtonElement).disabled).toBe(false);
  expect((screen.getByRole("button", { name: "Package Dev Project Files" }) as HTMLButtonElement).disabled).toBe(false);
  const summary = screen.getByText("NHL");
  expect(summary.closest("details")!.open).toBe(false);
  // All levels are initially collapsed; opening a branch reveals its original/current values.
  for (const label of ["NHL", "Vancouver", "Players", "Pavol Demitra", "Player data"]) fireEvent.click(screen.getByText(label));
  expect(screen.getByRole("link", { name: /Jersey number/ }).getAttribute("href")).toBe("/app/database/players/613");
  fireEvent.change(screen.getByLabelText("Author"), { target: { value: "Mod team" } });
  fireEvent.click(screen.getByRole("button", { name: "Save details" }));
  await screen.findByText("Details saved");
  expect(call).toHaveBeenCalledWith("project_set_details", { projectId: 1, details: { ...details, author: "Mod team" } });
});

it.each(["BUILD", "Package Dev Project Files"])("%s saves details and edits before packaging", async (button) => {
  const details = { name: "QA", version: "1", author: "Team", description: "Test" };
  const command = button === "BUILD" ? "project_build" : "project_archive";
  call.mockImplementation((cmd, args) => {
    if (cmd === "project_overview") return Promise.resolve({ details, dirty: true, changes: [], notices: [] });
    if (cmd === "project_set_details") return Promise.resolve(args.details);
    if (cmd === "project_save") return Promise.resolve({ saved: true, summary: {} });
    if (cmd === "project_info") return Promise.resolve({});
    if (cmd === command) return Promise.resolve(button === "BUILD" ? { packages: [{ path: "D:/Projects/Builds/QA_xbox360_nhl07.nhlmod", targets: [{ platform: "xbox" }] }], skipped: [] } : { path: "D:/Projects/Project Archives/QA.nhlproject" });
    return Promise.reject(new Error(`Unexpected ${cmd}`));
  });
  render(<MemoryRouter><QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><PendingEdits /></QueryClientProvider></MemoryRouter>);
  await screen.findByText("Project Details");
  fireEvent.change(screen.getByLabelText("Version"), { target: { value: "2" } });
  fireEvent.click(screen.getByRole("button", { name: button }));
  await screen.findByText(button === "BUILD" ? "Build complete." : "Project archive ready.");
  const commands = call.mock.calls.map(([cmd]) => cmd);
  expect(commands.indexOf("project_set_details")).toBeLessThan(commands.indexOf("project_save"));
  expect(commands.indexOf("project_save")).toBeLessThan(commands.indexOf(command));
  expect(call).toHaveBeenCalledWith("project_set_details", { projectId: 1, details: { ...details, version: "2" } });
  if (button === "BUILD") {
    fireEvent.click(screen.getByRole("button", { name: "Open containing folder" }));
    expect(reveal).toHaveBeenCalledWith(["D:/Projects/Builds/QA_xbox360_nhl07.nhlmod"]);
    fireEvent.click(screen.getByRole("button", { name: "Send to Mod Manager" }));
    await waitFor(() => expect(loadSentBuilds()).toEqual(["D:/Projects/Builds/QA_xbox360_nhl07.nhlmod"]));
  }
});

it("does not build when saving fails", async () => {
  call.mockImplementation((cmd, args) => cmd === "project_save" ? Promise.reject(new Error("Save failed")) : Promise.resolve(cmd === "project_set_details" ? args.details : { details: { name: "QA", version: "1", author: "", description: "" }, changes: [], notices: [] }));
  render(<MemoryRouter><QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><PendingEdits /></QueryClientProvider></MemoryRouter>);
  await screen.findByText("Project Details");
  fireEvent.click(screen.getByRole("button", { name: "BUILD" }));
  await screen.findByRole("alert");
  expect(call.mock.calls.some(([cmd]) => cmd === "project_build")).toBe(false);
});

it("removes an edit immediately while reverting and restores it on failure", async () => {
  let reject!: (e: Error) => void;
  const data = { details: { name: "QA", version: "1", author: "", description: "" }, dirty: false, notices: [], changes: [{ id: "ubPc:281:sVRt", groups: [], label: "Jersey number", before: "38", after: "91", route: "/app/database/players/613" }] };
  call.mockImplementation(cmd => cmd === "project_revert_change" ? new Promise((_resolve, fail) => { reject = fail; }) : Promise.resolve(cmd === "project_overview" ? data : {}));
  render(<MemoryRouter><QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><PendingEdits /></QueryClientProvider></MemoryRouter>);
  fireEvent.click(await screen.findByRole("button", { name: "Revert Jersey number" }));
  await screen.findByText("No changes yet. Your edits will appear here.");
  expect(call).toHaveBeenCalledWith("project_revert_change", { projectId: 1, changeId: "ubPc:281:sVRt" });
  reject(new Error("Restore refused"));
  await screen.findByRole("alert");
  await screen.findByRole("button", { name: "Revert Jersey number" });
});

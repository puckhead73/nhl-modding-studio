// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const call = vi.hoisted(() => vi.fn());
vi.mock("../lib/client", () => ({ call }));
vi.mock("../lib/project", () => ({ useProject: () => ({ project: null }) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
import { ModManagerPage } from "./ModManager";
import { MANAGER_KEY, sendBuilds } from "../lib/modManager";
import type { InstallRecordDto } from "../bindings/InstallRecordDto";
const mods = [
  { path: "D:/Mods/seven.nhlmod", name: "Seven roster", version: "1.2", author: "Mod Team", description: "Test roster description", game_id: "nhl07", platforms: ["xbox"], created: "2026-10-06T00:00:00Z" },
  { path: "D:/Mods/eight.nhlmod", name: "Eight PS3", version: "2", author: "Other", description: "Other roster", game_id: "nhl08", platforms: ["ps3"], created: "" },
  { path: "D:/Mods/eight-xbox.nhlmod", name: "Eight Xbox", version: "2", author: "Other", description: "Other roster", game_id: "nhl08", platforms: ["xbox"], created: "" },
].map((m, i) => ({ ...m, package_id: `package-${i}` }));
beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  call.mockImplementation(async (cmd, args) => {
    if (cmd === "package_inspect") return { package_id: mods.find(m => m.path === args.path)?.package_id ?? mods[0].package_id, changes: [] };
    if (cmd === "mod_library_scan") return { mods, warnings: [] };
    if (cmd === "package_installs") return [];
    if (cmd === "mod_manager_check_target") return args.root;
    throw new Error(`Unexpected ${cmd}`);
  });
});
afterEach(() => { cleanup(); localStorage.clear(); vi.clearAllMocks(); });
function Destination() { return <pre data-testid="install-input">{JSON.stringify(useLocation().state)}</pre>; }
function mount() {
  return render(<MemoryRouter initialEntries={["/manager"]}><QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><Routes><Route path="/manager" element={<ModManagerPage />} /><Route path="/manager/install" element={<Destination />} /></Routes></QueryClientProvider></MemoryRouter>);
}
function configured() {
  localStorage.setItem(MANAGER_KEY, JSON.stringify({ downloads: "D:/Mods", selected: "nhl07:xbox", targets: [{ gameId: "nhl07", platform: "xbox", root: "D:/NHL07" }] }));
}
function record(over: Partial<InstallRecordDto> = {}): InstallRecordDto {
  return { record_dir: "D:/Records/1", seq: 1, at: "", state: "committed", package_name: "Seven roster", package_version: "1.2", package_id: mods[0].package_id, game_id: "nhl07", platform: "xbox", picked_root: "d:\\nhl07\\", restorable: true, recovery: false, items: [], ...over };
}
it.each([false, true])("restores previous mods newest first before activation; restore failure=%s", async fail => {
  configured();
  let records = [record({ seq: 2, record_dir: "newest", package_id: "older-mod-2" }), record({ seq: 1, record_dir: "oldest", package_id: "older-mod-1" }), record({ picked_root: "D:/Other", record_dir: "untouched" })];
  call.mockImplementation(async (cmd, args) => {
    if (cmd === "package_inspect") return { package_id: mods.find(m => m.path === args.path)?.package_id ?? mods[0].package_id, changes: [] };
    if (cmd === "mod_library_scan") return { mods, warnings: [] };
    if (cmd === "package_installs") return records;
    if (cmd === "package_restore") {
      if (fail) return { state: "restore-incomplete", detail: "Restore refused" };
      records = records.filter(r => r.record_dir !== args.recordDir);
      return { state: "restored" };
    }
    if (cmd === "package_preflight") return { handle: 7, package_id: mods[0].package_id, installable: true, reasons: [], conflicts: [] };
    if (cmd === "package_apply") { records.push(record()); return { state: "committed" }; }
    throw new Error(cmd);
  });
  mount(); fireEvent.click(await screen.findByRole("button", { name: "Activate" }));
  if (fail) {
    expect((await screen.findByRole("alert")).textContent).toContain("Restore refused");
    expect(call.mock.calls.some(([cmd]) => cmd === "package_preflight" || cmd === "package_apply")).toBe(false);
  } else {
    await screen.findByText("Mod activated.");
    expect(call.mock.calls.filter(([cmd]) => cmd === "package_restore").map(([, args]) => args.recordDir)).toEqual(["newest", "oldest"]);
    const commands = call.mock.calls.map(([cmd]) => cmd);
    expect(commands.lastIndexOf("package_restore")).toBeLessThan(commands.indexOf("package_preflight"));
    expect(records.some(r => r.record_dir === "untouched")).toBe(true);
  }
});
it("keeps sent builds in the library outside the downloads folder without activating them", async () => {
  configured();
  const path = "D:/Projects/Builds/sent.nhlmod";
  sendBuilds([path]);
  call.mockImplementation(async (cmd, args) => cmd === "package_inspect" ? { package_id: mods[0].package_id, changes: [] } : cmd === "package_installs" ? [] : args.root === "D:/Mods" ? { mods: [], warnings: [] } : { mods: [{ ...mods[0], name: "Sent build", path }], warnings: [] });
  mount();
  await screen.findByRole("heading", { name: "Sent build" });
  expect(call).toHaveBeenCalledWith("mod_library_scan", { root: "D:/Projects/Builds" });
  expect(call.mock.calls.some(([cmd]) => cmd === "package_apply")).toBe(false);
});
it("shows preparation and installation progress without allowing a second activation", async () => {
  configured();
  let finishPreparation!: (value: unknown) => void;
  let finishInstallation!: (value: unknown) => void;
  const preparation = new Promise(resolve => { finishPreparation = resolve; });
  const installation = new Promise(resolve => { finishInstallation = resolve; });
  call.mockImplementation(async (cmd, args) => {
    if (cmd === "package_inspect") return { package_id: mods.find(m => m.path === args.path)?.package_id ?? mods[0].package_id, changes: [] };
    if (cmd === "mod_library_scan") return { mods, warnings: [] };
    if (cmd === "package_installs") return [];
    if (cmd === "package_preflight") return preparation;
    if (cmd === "package_apply") return installation;
    throw new Error(cmd);
  });
  mount(); fireEvent.click(await screen.findByRole("button", { name: "Activate" }));
  await screen.findByText("Preparing mod…");
  expect((screen.getByRole("button", { name: "Activating…" }) as HTMLButtonElement).disabled).toBe(true);
  expect(call.mock.calls.some(([cmd]) => cmd === "package_apply")).toBe(false);
  finishPreparation({ handle: 7, package_id: mods[0].package_id, installable: true, conflicts: [], reasons: [] });
  await screen.findByText("Installing mod…");
  finishInstallation({ state: "committed" });
  await screen.findByText("Mod activated.");
  expect(screen.queryByText("Installing mod…")).toBeNull();
});
it("activates with one click, shows both Installed pills, then deactivates through restore", async () => {
  configured(); let records: InstallRecordDto[] = [];
  call.mockImplementation(async (cmd, args) => {
    if (cmd === "package_inspect") return { package_id: mods.find(m => m.path === args.path)?.package_id ?? mods[0].package_id, changes: [] };
    if (cmd === "mod_library_scan") return { mods, warnings: [] };
    if (cmd === "package_installs") return records;
    if (cmd === "package_preflight") return { handle: 7, package_id: mods[0].package_id, installable: true, reasons: [], conflicts: [] };
    if (cmd === "package_apply") { records = [record()]; return { state: "committed" }; }
    if (cmd === "package_restore") { records = [record({ state: "restored", restorable: false })]; return { state: "restored" }; }
    throw new Error(cmd);
  });
  mount();
  fireEvent.click(await screen.findByRole("button", { name: "Activate" }));
  await screen.findByRole("button", { name: "Deactivate" });
  expect(screen.getAllByText("Installed")).toHaveLength(2);
  expect(call).toHaveBeenCalledWith("package_preflight", { packagePath: mods[0].path, gameId: "nhl07", platform: "xbox", root: "D:/NHL07", savePath: null, rosterSave: null, allowConflicts: false });
  expect(call).toHaveBeenCalledWith("package_apply", { handle: 7 });
  fireEvent.click(screen.getByRole("button", { name: "Deactivate" }));
  await screen.findByRole("button", { name: "Activate" });
  expect(screen.queryByText("Installed")).toBeNull();
  expect(call).toHaveBeenCalledWith("package_restore", { recordDir: "D:/Records/1" });
});
it("does not mark another folder, platform, or package version installed", async () => {
  configured();
  call.mockImplementation(async cmd => cmd === "package_inspect" ? { package_id: mods[0].package_id, changes: [] } : cmd === "mod_library_scan" ? { mods, warnings: [] } : [record({ picked_root: "D:/Other" }), record({ platform: "ps3" }), record({ package_id: "other-version" })]);
  mount(); await screen.findByRole("button", { name: "Activate" });
  expect(screen.queryByText("Installed")).toBeNull();
});
it("refuses conflicts without applying or showing an Installed pill", async () => {
  configured();
  call.mockImplementation(async (cmd, args) => {
    if (cmd === "package_inspect") return { package_id: mods.find(m => m.path === args.path)?.package_id ?? mods[0].package_id, changes: [] };
    if (cmd === "mod_library_scan") return { mods, warnings: [] };
    if (cmd === "package_installs") return [];
    if (cmd === "package_preflight") return { package_id: mods[0].package_id, installable: false, reasons: ["Game files conflict"], conflicts: [] };
    throw new Error(cmd);
  });
  mount(); fireEvent.click(await screen.findByRole("button", { name: "Activate" }));
  expect((await screen.findByRole("alert")).textContent).toContain("Game files conflict");
  expect(call.mock.calls.some(([cmd]) => cmd === "package_apply")).toBe(false);
  expect(screen.queryByText("Installed")).toBeNull();
});
it("retains Installed status when restoration fails", async () => {
  configured();
  call.mockImplementation(async (cmd, args) => {
    if (cmd === "package_inspect") return { package_id: mods.find(m => m.path === args.path)?.package_id ?? mods[0].package_id, changes: [] };
    if (cmd === "mod_library_scan") return { mods, warnings: [] };
    if (cmd === "package_installs") return [record()];
    throw new Error("Game file changed after installation");
  });
  mount(); fireEvent.click(await screen.findByRole("button", { name: "Deactivate" }));
  await screen.findByRole("alert");
  expect(screen.getAllByText("Installed")).toHaveLength(2);
  expect(screen.getByRole("button", { name: "Deactivate" })).toBeTruthy();
});
it("does not turn Activate into Deactivate if installation status changes before the click", async () => {
  configured(); let checks = 0;
  call.mockImplementation(async (cmd, args) => {
    if (cmd === "package_inspect") return { package_id: mods.find(m => m.path === args.path)?.package_id ?? mods[0].package_id, changes: [] };
    if (cmd === "mod_library_scan") return { mods, warnings: [] };
    if (cmd === "package_installs") return ++checks === 1 ? [] : [record()];
    throw new Error(`Unexpected write: ${cmd}`);
  });
  mount(); fireEvent.click(await screen.findByRole("button", { name: "Activate" }));
  await screen.findByRole("button", { name: "Deactivate" });
  expect(call.mock.calls.some(([cmd]) => cmd === "package_restore" || cmd === "package_apply")).toBe(false);
});
it("onboards downloads then an executable folder and never activates without a click", async () => {
  mount();
  expect(screen.getByRole("dialog")).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Downloaded mods folder"), { target: { value: "D:/Mods" } });
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  await screen.findByLabelText("Game installation folder");
  expect(within(screen.getByLabelText("Platform")).getAllByRole("option")).toHaveLength(1);
  expect(screen.getByText(/\.xex/)).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Game installation folder"), { target: { value: "D:/NHL07" } });
  fireEvent.click(screen.getByRole("button", { name: "Finish" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  const details = within(screen.getByRole("region", { name: "Mod details" }));
  expect(await details.findByText("Seven roster")).toBeTruthy();
  expect(details.getByText(/Mod Team/)).toBeTruthy();
  expect(screen.queryByText("Eight PS3")).toBeNull();
  expect(JSON.parse(localStorage.getItem(MANAGER_KEY)!).targets[0].root).toBe("D:/NHL07");
  expect(call.mock.calls.some(([cmd]) => cmd === "package_apply" || cmd === "package_preflight")).toBe(false);
  expect(screen.getByRole("button", { name: "Activate" })).toBeTruthy();
});
it("restores settings, filters by game AND platform, refreshes the library, and reports damaged files", async () => {
  localStorage.setItem(MANAGER_KEY, JSON.stringify({ downloads: "D:/Mods", selected: "nhl07:xbox", targets: [
    { gameId: "nhl07", platform: "xbox", root: "D:/NHL07" }, { gameId: "nhl08", platform: "ps3", root: "D:/NHL08" },
  ] }));
  mount();
  await screen.findByRole("button", { name: /Seven roster/ });
  expect(screen.queryByRole("dialog")).toBeNull();
  fireEvent.change(screen.getByLabelText("Installation target"), { target: { value: "nhl08:ps3" } });
  expect(screen.getByRole("button", { name: /Eight PS3/ })).toBeTruthy();
  expect(screen.queryByText("Eight Xbox")).toBeNull();
  expect(screen.queryByText("Seven roster")).toBeNull();
  call.mockResolvedValueOnce({ mods, warnings: ["Couldn’t read broken.nhlmod"] });
  fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
  await screen.findByText("1 library warning(s)");
  expect(screen.getByRole("button", { name: /Eight PS3/ })).toBeTruthy();
});
it("leaves setup open when the installation folder is invalid", async () => {
  mount();
  fireEvent.change(screen.getByLabelText("Downloaded mods folder"), { target: { value: "D:/Mods" } });
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  await screen.findByLabelText("Game installation folder");
  call.mockRejectedValueOnce(new Error("Choose the folder containing the game's .xex file."));
  fireEvent.change(screen.getByLabelText("Game installation folder"), { target: { value: "D:/Wrong" } });
  fireEvent.click(screen.getByRole("button", { name: "Finish" }));
  expect((await screen.findByRole("alert")).textContent).toContain(".xex");
  expect(screen.getByRole("dialog")).toBeTruthy();
  expect(localStorage.getItem(MANAGER_KEY)).toBeNull();
});

it("reviews packaged changes without navigating into Studio or installing", async () => {
  configured();
  call.mockImplementation(async (cmd) => {
    if (cmd === "mod_library_scan") return { mods, warnings: [] };
    if (cmd === "package_installs") return [];
    if (cmd === "package_inspect") return { package_id: mods[0].package_id, changes: [
      { id: "1", groups: ["Database", "Player data", "Mike Modano"], label: "Draft Year", before: "1988", after: "1989", route: null },
      { id: "2", groups: ["Art assets", "player"], label: "portrait", before: "Original texture", after: "Replacement", route: null },
    ] };
    throw new Error(cmd);
  });
  mount();
  const changes = await screen.findByRole("region", { name: "Mod changes" });
  await within(changes).findByText("Mike Modano");
  const summaries = changes.querySelectorAll("summary");
  expect(summaries.length).toBeGreaterThan(2);
  fireEvent.click(summaries[0]);
  expect(within(changes).getByText("1988")).toBeTruthy();
  expect(within(changes).getByText("1989")).toBeTruthy();
  expect(within(changes).queryAllByRole("link")).toHaveLength(0);
  expect(within(changes).queryByRole("button", { name: /Revert/ })).toBeNull();
  expect(call.mock.calls.some(([cmd]) => cmd === "package_apply" || cmd === "package_preflight")).toBe(false);
});
it("shows inspection failure and retries without blocking library navigation", async () => {
  configured(); let attempts = 0;
  call.mockImplementation(async cmd => {
    if (cmd === "mod_library_scan") return { mods, warnings: [] };
    if (cmd === "package_installs") return [];
    if (cmd === "package_inspect") { if (++attempts === 1) throw new Error("Unreadable package"); return { package_id: mods[0].package_id, changes: [] }; }
    throw new Error(cmd);
  });
  mount(); await screen.findByText(/Unreadable package/);
  fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  await waitFor(() => expect(screen.queryByText(/Unreadable package/)).toBeNull());
  expect(attempts).toBe(2);
});

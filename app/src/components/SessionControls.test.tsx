// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const mock = vi.hoisted(() => ({ call: vi.fn(), close: vi.fn(), destroy: vi.fn(), minutes: 10, dirty: true, listener: null as null | ((e: { preventDefault: () => void }) => void) }));
vi.mock("../lib/client", () => ({ call: mock.call }));
vi.mock("./ProjectOverviewLink", () => ({ ProjectOverviewLink: () => <a href="/app/database/pending">Project Overview</a> }));
vi.mock("../lib/project", () => ({ useProject: () => ({ project: { project_id: 1, dirty: mock.dirty, undo_depth: mock.dirty ? 1 : 0, redo_depth: mock.dirty ? 0 : 1 }, info: { dir: "Project" }, close: mock.close, update: vi.fn(), updateInfo: vi.fn() }) }));
vi.mock("../lib/settings", () => ({ useSettings: () => ({ settings: { autosaveMinutes: mock.minutes } }), validAutosaveMinutes: (n: number) => n }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ destroy: mock.destroy, onCloseRequested: async (listener: typeof mock.listener) => { mock.listener = listener; return () => { mock.listener = null; }; } }) }));
import { SessionControls } from "./SessionControls";
import { WindowCloseGuard } from "./WindowCloseGuard";
beforeEach(() => {
  vi.useFakeTimers(); mock.minutes = 10; mock.dirty = true;
  mock.call.mockImplementation(async cmd => cmd === "project_save" ? { saved: true, summary: { project_id: 1 } } : cmd === "close_project" ? { outcome: "closed" } : {});
  mock.destroy.mockResolvedValue(undefined);
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); vi.useRealTimers(); vi.clearAllMocks(); });
function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const view = (editor = true) => <MemoryRouter><QueryClientProvider client={qc}><WindowCloseGuard />{editor && <SessionControls />}</QueryClientProvider></MemoryRouter>;
  const result = render(view());
  return { ...result, refresh: () => result.rerender(view()), showMenu: () => result.rerender(view(false)) };
}
async function flush() { await act(async () => { await vi.advanceTimersByTimeAsync(1); }); }

it("offers Undo and Redo beside Save Session and follows available history", async () => {
  const view = mount();
  expect((screen.getByRole("button", { name: "Redo" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Undo" })); await flush();
  expect(mock.call).toHaveBeenCalledWith("undo", { projectId: 1 });
  mock.dirty = false; view.refresh();
  expect((screen.getByRole("button", { name: "Undo" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Redo" })); await flush();
  expect(mock.call).toHaveBeenCalledWith("redo", { projectId: 1 });
});

it("moves history buttons to the tab-row destination", () => {
  const destination = document.createElement("div"); document.body.append(destination);
  render(<MemoryRouter><QueryClientProvider client={new QueryClient()}><SessionControls historyTarget={destination} /></QueryClientProvider></MemoryRouter>);
  expect(destination.contains(screen.getByRole("button", { name: "Undo" }))).toBe(true);
  expect(destination.contains(screen.getByRole("button", { name: "Redo" }))).toBe(true);
  expect(destination.contains(screen.getByRole("button", { name: "Save Session" }))).toBe(false);
  destination.remove();
});

it("allows native close on the main menu after leaving the editor", async () => {
  const view = mount(); await flush();
  const preventDefault = vi.fn();
  act(() => mock.listener!({ preventDefault }));
  expect(preventDefault).toHaveBeenCalledOnce();
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  view.showMenu(); await flush();
  preventDefault.mockClear();
  act(() => mock.listener!({ preventDefault }));
  expect(preventDefault).not.toHaveBeenCalled();
  expect(screen.queryByText("Close project session?")).toBeNull();
});

it("allows native close before any project has entered the editor", async () => {
  render(<WindowCloseGuard />); await flush();
  const preventDefault = vi.fn();
  act(() => mock.listener!({ preventDefault }));
  expect(preventDefault).not.toHaveBeenCalled();
});

it("keeps Save Session visible for clean sessions; autosaves dirty sessions at the configured interval", async () => {
  const view = mount();
  expect(screen.getByRole("button", { name: "Save Session" })).toBeTruthy();
  await act(async () => { await vi.advanceTimersByTimeAsync(599999); });
  expect(mock.call).not.toHaveBeenCalled();
  await act(async () => { await vi.advanceTimersByTimeAsync(1); });
  expect(mock.call.mock.calls.filter(([c]) => c === "project_save")).toHaveLength(1);
  await flush();
  expect(screen.getByText("Project has been saved")).toBeTruthy();
  mock.minutes = 2; view.refresh();
  await act(async () => { await vi.advanceTimersByTimeAsync(120000); });
  expect(mock.call.mock.calls.filter(([c]) => c === "project_save")).toHaveLength(2);
  mock.dirty = false; view.refresh();
  await act(async () => { await vi.advanceTimersByTimeAsync(120000); });
  expect(mock.call.mock.calls.filter(([c]) => c === "project_save")).toHaveLength(2);
  expect(screen.getByRole("button", { name: "Save Session" })).toBeTruthy();
});

it("shows a three-second save notice, updates Last Save and orders the header controls", async () => {
  mount(); await flush();
  expect(screen.getByText("Last Save: Unsaved")).toBeTruthy();
  const save = screen.getByRole("button", { name: "Save Session" });
  expect(save.nextElementSibling?.textContent).toBe("Last Save: Unsaved");
  expect(save.nextElementSibling?.nextElementSibling?.textContent).toBe("Project Overview");
  fireEvent.click(save); await flush();
  expect(screen.getByText("Project has been saved")).toBeTruthy();
  expect(screen.queryByText("Last Save: Unsaved")).toBeNull();
  await act(async () => { await vi.advanceTimersByTimeAsync(3000); });
  expect(screen.queryByText("Project has been saved")).toBeNull();
  mock.call.mockRejectedValueOnce(new Error("Disk full"));
  fireEvent.click(screen.getByRole("button", { name: "Save Session" })); await flush();
  expect(screen.queryByText("Project has been saved")).toBeNull();
});

it("intercepts window X, supports Cancel, and saves before destroying the window", async () => {
  mount(); await flush();
  const preventDefault = vi.fn();
  act(() => mock.listener!({ preventDefault }));
  expect(preventDefault).toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(mock.close).not.toHaveBeenCalled();
  act(() => mock.listener!({ preventDefault }));
  fireEvent.click(screen.getByRole("button", { name: "Save and Close" }));
  await flush();
  const commands = mock.call.mock.calls.map(([c]) => c);
  expect(commands.indexOf("project_save")).toBeLessThan(commands.indexOf("close_project"));
  expect(mock.destroy).toHaveBeenCalledOnce();
});

it("retains the session after save failure and discards only after explicit selection", async () => {
  mount(); await flush();
  mock.call.mockRejectedValueOnce(new Error("Disk full"));
  fireEvent.click(screen.getByRole("button", { name: "Close" }));
  fireEvent.click(screen.getByRole("button", { name: "Save and Close" }));
  await flush();
  expect(screen.getByRole("alert").textContent).toContain("Disk full");
  expect(mock.close).not.toHaveBeenCalled();
  expect(mock.call.mock.calls.some(([c]) => c === "close_project")).toBe(false);
  fireEvent.click(screen.getByRole("button", { name: "Close Without Saving" }));
  await flush();
  expect(mock.close).toHaveBeenCalledOnce();
  expect(mock.destroy).not.toHaveBeenCalled();
});

it("does not overlap manual save with autosave and stops timers on unmount", async () => {
  let resolve!: (value: unknown) => void;
  mock.call.mockImplementation(cmd => cmd === "project_save" ? new Promise(r => { resolve = r; }) : Promise.resolve({}));
  const view = mount();
  fireEvent.click(screen.getByRole("button", { name: "Save Session" })); await flush();
  await act(async () => { await vi.advanceTimersByTimeAsync(600000); });
  expect(mock.call.mock.calls.filter(([c]) => c === "project_save")).toHaveLength(1);
  await act(async () => { resolve({ saved: true, summary: {} }); }); await flush();
  view.unmount();
  await act(async () => { await vi.advanceTimersByTimeAsync(600000); });
  expect(mock.call.mock.calls.filter(([c]) => c === "project_save")).toHaveLength(1);
});

it("shows autosave failures and retries at the next interval", async () => {
  mount();
  mock.call.mockRejectedValueOnce(new Error("Disk unavailable"));
  await act(async () => { await vi.advanceTimersByTimeAsync(600000); });
  await flush();
  expect(screen.getByRole("alert").textContent).toContain("Disk unavailable");
  expect(mock.close).not.toHaveBeenCalled();
  await act(async () => { await vi.advanceTimersByTimeAsync(600000); });
  expect(mock.call.mock.calls.filter(([c]) => c === "project_save")).toHaveLength(2);
});

it("keeps the window open when the backend refuses to close a busy project", async () => {
  mount(); await flush();
  mock.call.mockImplementation(async cmd => cmd === "close_project" ? { outcome: "busy", reason: "Build running" } : {});
  act(() => mock.listener!({ preventDefault: vi.fn() }));
  fireEvent.click(screen.getByRole("button", { name: "Close Without Saving" }));
  await flush();
  expect(screen.getByRole("alert").textContent).toContain("Build running");
  expect(mock.destroy).not.toHaveBeenCalled();
  expect(mock.close).not.toHaveBeenCalled();
});


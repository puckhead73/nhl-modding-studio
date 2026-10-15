// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ call: vi.fn(), update: vi.fn(), dirty: false }));
vi.mock("../lib/client", () => ({ call: mocks.call }));
vi.mock("../lib/project", () => ({ useProject: () => ({ project: { project_id: 1, dirty: mocks.dirty }, info: { working_dir: "D:/Project/working" }, update: mocks.update }) }));
import { ProjectRecovery } from "./ProjectRecovery";
const status = (blocked: boolean) => ({ apply_allowed: !blocked, active_txn_id: blocked ? "interrupted" : null, pending_handles: [], reservations: [], busy_projects: [], guidance: "Ready" });
afterEach(() => { cleanup(); vi.resetAllMocks(); mocks.dirty = false; });
function mount() { render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProjectRecovery /></QueryClientProvider>); }
it("recovers an interrupted save and refreshes the session", async () => {
  let blocked = true;
  mocks.call.mockImplementation(async cmd => {
    if (cmd === "copied_target_status") return status(blocked);
    if (cmd === "copied_target_recover") { blocked = false; return { affected: [], status: status(false) }; }
    if (cmd === "project_summary") return { project_id: 1, dirty: false };
    throw Error(cmd);
  });
  mount(); fireEvent.click(await screen.findByText("Recover interrupted save"));
  await screen.findByRole("status");
  expect(mocks.update).toHaveBeenCalledWith({ project_id: 1, dirty: false });
  expect(screen.queryByText("Recover interrupted save")).toBeNull();
});
it("does not offer recovery for an active operation", async () => {
  mocks.call.mockResolvedValue({ ...status(true), pending_handles: [{ txn_id: "interrupted", in_progress: true }] });
  mount(); await waitFor(() => expect(mocks.call).toHaveBeenCalled());
  expect(screen.queryByRole("button")).toBeNull();
});
it("protects unsaved edits before recovery", async () => {
  mocks.dirty = true; mocks.call.mockResolvedValue(status(true));
  mount();
  expect((await screen.findByText("Recover interrupted save") as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText(/Close without saving/)).toBeTruthy();
});

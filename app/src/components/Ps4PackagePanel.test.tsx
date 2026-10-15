// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { StrictMode } from "react";

const handlers: Record<string, (args: any) => unknown> = {};
const calls: string[] = [];
vi.mock("../lib/client", () => ({ call: (name: string, args: unknown) => {
  calls.push(name);
  return Promise.resolve().then(() => handlers[name]?.(args));
} }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: () => Promise.resolve(null) }));
import { Ps4PackagePanel } from "./Ps4PackagePanel";
const job = (state: string) => ({ job: 1, root: "E:/new disc", state, bytes_written: 123, current: "copying" });
function mount() {
  const ready = vi.fn();
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const result = render(<StrictMode><QueryClientProvider client={client}><Ps4PackagePanel gameId="nhl21" onReady={ready} /></QueryClientProvider></StrictMode>);
  fireEvent.change(screen.getByLabelText("PS4 base PKG"), { target: { value: "X:/NHL21.pkg" } });
  fireEvent.change(screen.getByLabelText("PS4 package output folder"), { target: { value: "E:/new disc" } });
  return { ...result, ready };
}
beforeEach(() => { calls.length = 0; for (const key of Object.keys(handlers)) delete handlers[key]; });
afterEach(cleanup);
it("hands completed extracted sources to import after explicit selection", async () => {
  handlers.pkg_extract_start = () => job("done");
  const report = { install_dir: "E:/new disc/CUSA18182", nhlng_path: "E:/new disc/CUSA18182/db/nhlng.db" };
  handlers.detect_install = () => report;
  const { ready } = mount();
  fireEvent.click(screen.getByText("Extract PS4 package"));
  await waitFor(() => expect((screen.getByText("Use extracted files") as HTMLButtonElement).disabled).toBe(false));
  expect(ready).not.toHaveBeenCalled();
  fireEvent.click(screen.getByText("Use extracted files"));
  await waitFor(() => expect(ready).toHaveBeenCalledWith(report));
});
it.each(["failed", "cancelled"])("never opens a %s partial disc", async (state) => {
  handlers.pkg_extract_start = () => job(state);
  const { ready } = mount();
  fireEvent.click(screen.getByText("Extract PS4 package"));
  await waitFor(() => expect(screen.getByRole("status").textContent).toContain(state));
  expect((screen.getByText("Use extracted files") as HTMLButtonElement).disabled).toBe(true);
  expect(ready).not.toHaveBeenCalled();
  expect(calls).not.toContain("detect_install");
});
it("locks source fields while copying and supports cancellation", async () => {
  handlers.pkg_extract_start = () => job("running");
  handlers.extract_status = () => job("running");
  handlers.extract_cancel = () => job("cancelled");
  mount(); fireEvent.click(screen.getByText("Extract PS4 package"));
  await screen.findByText("Cancel extraction");
  expect((screen.getByLabelText("PS4 base PKG") as HTMLInputElement).disabled).toBe(true);
  fireEvent.click(screen.getByText("Cancel extraction"));
  await waitFor(() => expect(screen.getByRole("status").textContent).toContain("cancelled"));
});
it("does not publish a late discovery after unmount", async () => {
  handlers.pkg_extract_start = () => job("done");
  let finish!: (value: unknown) => void;
  handlers.detect_install = () => new Promise(resolve => { finish = resolve; });
  const { ready, unmount } = mount();
  fireEvent.click(screen.getByText("Extract PS4 package"));
  await waitFor(() => expect((screen.getByText("Use extracted files") as HTMLButtonElement).disabled).toBe(false));
  fireEvent.click(screen.getByText("Use extracted files"));
  await waitFor(() => expect(finish).toBeDefined());
  unmount(); finish({ install_dir: "E:/disc", nhlng_path: "E:/disc/db/nhlng.db" });
  await new Promise(resolve => setTimeout(resolve, 0));
  expect(ready).not.toHaveBeenCalled();
});

it("extracts a packed database before importing without recopying the PKG", async () => {
  handlers.pkg_extract_start = () => job("done");
  const root = "E:/new disc/CUSA18182";
  let extracted = false;
  const report = () => ({ game_dir: root, install_dir: extracted ? root + "/db" : null, nhlng_path: extracted ? `${root}/db/nhlng.db` : null, extract_needed: !extracted });
  handlers.detect_install = report;
  handlers.extract_running_for = () => null;
  handlers.extract_preflight = () => ({ root, state: extracted ? "complete" : "needed", reason: "Packed database", archives: [], members_total: 1, members_present: extracted ? 1 : 0, members_to_write: extracted ? 0 : 1, bytes_to_write: 123, free_bytes: 10000, collisions: [] });
  handlers.extract_start = (args) => {
    expect(args).toEqual({ root, platform: "ps4", destination: root });
    return { ...job("running"), job: 2, root, archives_done: 0, archives_total: 1, files_written: 0, files_retained: 0, problems: [] };
  };
  handlers.extract_status = () => ({ ...job(extracted ? "done" : "running"), job: 2, root, archives_done: extracted ? 1 : 0, archives_total: 1, files_written: extracted ? 1 : 0, files_retained: 0, problems: [] });
  const { ready } = mount();
  fireEvent.click(screen.getByText("Extract PS4 package"));
  await waitFor(() => expect((screen.getByText("Use extracted files") as HTMLButtonElement).disabled).toBe(false));
  fireEvent.click(screen.getByText("Use extracted files"));
  await screen.findByText(/database is inside a game container/);
  expect(ready).not.toHaveBeenCalled();
  await waitFor(() => expect((screen.getByText("Extract now") as HTMLButtonElement).disabled).toBe(false));
  fireEvent.click(screen.getByText("Extract now"));
  await waitFor(() => expect((screen.getByText("Use extracted files") as HTMLButtonElement).disabled).toBe(true));
  expect((screen.getByLabelText("PS4 package output folder") as HTMLInputElement).disabled).toBe(true);
  extracted = true;
  await waitFor(() => expect((screen.getByText("Use extracted files") as HTMLButtonElement).disabled).toBe(false), { timeout: 2000 });
  fireEvent.click(screen.getByText("Use extracted files"));
  await waitFor(() => expect(ready).toHaveBeenCalledWith(report()));
  expect(calls.filter(c => c === "pkg_extract_start")).toHaveLength(1);
});

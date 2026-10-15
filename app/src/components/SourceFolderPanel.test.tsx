// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { SourceFolderPanel } from "./SourceFolderPanel";

const mocks = vi.hoisted(() => ({ call: vi.fn() }));
vi.mock("../lib/client", () => ({ call: mocks.call }));
// Extraction itself is covered by the shared panel and real backend tests.
vi.mock("./ExtractionPanel", () => ({ ExtractionPanel: ({ root }: { root: string }) => <div>Extract destination: {root}</div> }));
afterEach(() => { cleanup(); mocks.call.mockReset(); });
function mount(platform: "xbox" | "ps4" = "xbox") {
  const ready = vi.fn();
  const busy = vi.fn();
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
  const view = render(<QueryClientProvider client={client}><SourceFolderPanel root="H:/moved disc" gameId={platform === "ps4" ? "nhl15" : "nhl12"} platform={platform} onReady={ready} onBusyChange={busy} /></QueryClientProvider>);
  return { ...view, ready, busy };
}
it("discovers source files without supplying a save or deployment destination", async () => {
  const report = { install_dir: "H:/moved disc", nhlng_path: "H:/moved disc/db/nhlng.db" };
  mocks.call.mockResolvedValue(report);
  const { ready } = mount();
  expect(mocks.call).not.toHaveBeenCalled();
  fireEvent.click(screen.getByText("Find database in this folder"));
  await waitFor(() => expect(ready).toHaveBeenCalledWith(report));
  expect(mocks.call).toHaveBeenCalledExactlyOnceWith("detect_install", { gameId: "nhl12", platform: "xbox", root: "H:/moved disc" });
});
it("offers packed-container extraction then rescans before importing", async () => {
  mocks.call.mockResolvedValueOnce({ install_dir: "H:/moved disc/USRDIR", nhlng_path: null, extract_needed: true });
  const { ready } = mount();
  fireEvent.click(screen.getByText("Find database in this folder"));
  await screen.findByText("Extract destination: H:/moved disc/USRDIR");
  expect(ready).not.toHaveBeenCalled();
  const report = { install_dir: "H:/moved disc/USRDIR", nhlng_path: "H:/moved disc/USRDIR/db/nhlng.db" };
  mocks.call.mockResolvedValueOnce(report);
  fireEvent.click(screen.getByText("Find database in this folder"));
  await waitFor(() => expect(ready).toHaveBeenCalledWith(report));
});
it("ignores discovery that finishes after changing the source", async () => {
  let resolve!: (v: unknown) => void;
  mocks.call.mockImplementation(() => new Promise(r => { resolve = r; }));
  const { ready, unmount } = mount();
  fireEvent.click(screen.getByText("Find database in this folder"));
  await waitFor(() => expect(resolve).toBeDefined());
  unmount(); resolve({ nhlng_path: "old source/db/nhlng.db" });
  await new Promise(r => setTimeout(r, 0));
  expect(ready).not.toHaveBeenCalled();
});
it("offers extraction for a PS4 title folder before its database directory exists", async () => {
  mocks.call.mockResolvedValue({ install_dir: null, game_dir: "H:/moved disc/CUSA00561", nhlng_path: null, extract_needed: true });
  const { ready } = mount("ps4");
  fireEvent.click(screen.getByText("Find database in this folder"));
  await screen.findByText("Extract destination: H:/moved disc/CUSA00561");
  expect(ready).not.toHaveBeenCalled();
  expect(screen.queryByText(/No database was found here/)).toBeNull();
});

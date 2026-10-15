// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ invoke: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: mocks.save }));
import { FrostbiteBrowser } from "./FrostbiteBrowser";
afterEach(() => { cleanup(); vi.clearAllMocks(); });
it("opens without a database, retains layer identity and exports the selected chunk", async () => {
  mocks.invoke.mockImplementation(async (cmd: string) => {
    if (cmd === "frostbite_inspect") return { root: "game", layers: ["Data", "Patch"].map(name => ({ name, head: 1, base: null, superbundles: ["Gen4b/ui"], initfs: "encrypted (opaque)", chunkmanifest: true })), warnings: ["Read-only catalogue"] };
    if (cmd === "frostbite_bundles") return { total: 1, matched: 1, toc_chunks: 1, items: [{ ordinal: 9, name: "gen4b/kit", spans: 2 }] };
    if (cmd === "frostbite_assets") return { name: "gen4b/kit", assets: [{ kind: "ebx", name: "jersey", size: 3, sha1: "abcd", resource_type: null }], spans: [{ cas: 1, offset: 0, size: 11 }] };
    if (cmd === "frostbite_chunks") return { matched: 1, items: [{ ordinal: 4, guid: "abcdef", span: { cas: 0x10001, offset: 50, size: 11 } }] };
    if (cmd === "frostbite_export_chunk") return 11;
    throw new Error(`Unexpected ${cmd}`);
  });
  mocks.save.mockResolvedValue("output/chunk.bin");
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={client}><MemoryRouter><FrostbiteBrowser /></MemoryRouter></QueryClientProvider>);
  fireEvent.change(screen.getByLabelText("Frostbite game folder"), { target: { value: "game" } });
  fireEvent.click(screen.getByText("Open assets"));
  fireEvent.click(await screen.findByText("gen4b/kit"));
  expect(await screen.findByText("jersey")).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Content layer"), { target: { value: "Patch" } });
  fireEvent.change(screen.getByLabelText("Entry kind"), { target: { value: "chunks" } });
  await screen.findByText("abcdef");
  fireEvent.click(screen.getByText("Export raw"));
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("frostbite_export_chunk", { root: "game", layer: "Patch", superbundle: "Gen4b/ui", chunk: 4, decoded: false, destination: "output/chunk.bin" }));
  expect(mocks.invoke.mock.calls.every(([cmd]) => !String(cmd).includes("project_create"))).toBe(true);
});

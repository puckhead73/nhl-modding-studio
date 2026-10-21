// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { HeadLibraryPanel } from "./HeadLibraryPanel";

const { invoke, open } = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open }));
vi.mock("./HeadMeshViewer", () => ({
  LoadedViewer: ({ data }: { data: { submeshes: unknown[] } }) =>
    <div data-testid="preview">{data.submeshes.length} meshes</div>,
}));
vi.mock("three/examples/jsm/loaders/GLTFLoader.js", async () => {
  const T = await import("three");
  return { GLTFLoader: class {
    async parseAsync() {
      const geometry = new T.BufferGeometry();
      geometry.setAttribute("position", new T.Float32BufferAttribute([0,0,0,1,0,0,0,1,0], 3));
      const scene = new T.Group();
      scene.add(new T.Mesh(geometry, new T.MeshBasicMaterial()));
      return { scene };
    }
  } };
});

function asset(kind: string) {
  return { id: kind, class: kind, stem: `test_${kind}`, mesh_file: `test_${kind}.glb`,
    game: "nhl24", platform: "ps4", origin: "player", lod: 0, sources: "nhl24/ps4" };
}
function mount(name = "") {
  return render(<QueryClientProvider client={new QueryClient({ defaultOptions: {
    queries: { retry: false, gcTime: 0 },
  } })}><HeadLibraryPanel name={name} personKey="historical:42" /></QueryClientProvider>);
}
beforeEach(() => {
  localStorage.clear(); invoke.mockReset(); open.mockReset();
  invoke.mockImplementation(async (command: string, args: Record<string, unknown>) => {
    if (command === "head_library_status") return { available: true, assets: 3, root: "/library" };
    if (command === "head_library_search") return [asset(String(args.class))];
    if (command === "head_library_detail") return [{ id: 1, game: "nhl24", platform: "ps4", identity: "test", textures: [] }];
    if (command === "head_library_file") return { base64: "AAAA" };
    if (command === "head_library_export") return { destination: args.destination };
    throw new Error(command);
  });
});
afterEach(cleanup);

describe("library appearance composition", () => {
  it("distinguishes patch texture sets and their version", async () => {
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command: string, args: Record<string, unknown>) =>
      command === "head_library_detail" ? [{ id: 1, game: "nhl24", platform: "ps4", identity: "test", textures: [], original: { layer: "Patch", patch_version: "1.6" } }] : original(command, args));
    mount();
    expect(await screen.findByRole("option", { name: "nhl24 / ps4 / Patch 1.6 — test" })).toBeTruthy();
  });
  it("loads all API batches into a newest-first scroll list without page buttons", async () => {
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command: string, args: Record<string, unknown>) => {
      if (command === "head_library_search") return args.offset === 0
        ? Array.from({ length: 100 }, (_, i) => ({ ...asset("head"), id: `old-${i}`, game: "nhl07", platform: "xbox360" }))
        : [{ ...asset("head"), id: "new", game: "nhl27", platform: "ps5" }];
      return original(command, args);
    });
    mount();
    await screen.findByRole("button", { name: "NHL27 - PS5" });
    expect(screen.getAllByRole("button", { name: /^NHL/ })[0].textContent).toBe("NHL27 - PS5");
    expect(screen.getAllByRole("button", { name: /^NHL/ })).toHaveLength(101);
    expect(screen.queryByText("Next")).toBeNull(); expect(screen.queryByText("Previous")).toBeNull();
  });
  it("sends accent-insensitive names to the library search", async () => {
    mount("Jarom\u00edr J\u00e1gr");
    await screen.findByText("1 meshes");
    expect(invoke).toHaveBeenCalledWith("head_library_search", { query: "jaromir jagr", class: "head", offset: 0 });
  });
  it("automatically previews the first result when a player opens", async () => {
    mount("Keith Tkachuk");
    await screen.findByText("1 meshes");
    expect(screen.getByRole("button", { name: "NHL24 - PS4" }).closest("button")?.getAttribute("aria-pressed")).toBe("true");
  });
  it("explains missing matches while keeping a visible preview placeholder", async () => {
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command: string, args: Record<string, unknown>) =>
      command === "head_library_search" ? [] : original(command, args));
    mount("Troy Brouwer");
    await screen.findByText("No matching headmeshes. Try a surname or another spelling.");
    expect(screen.getByText("Select a headmesh from the library to open the 3D preview.")).toBeTruthy();
  });
  it("selects the searched player's textures when geometry is shared across identities", async () => {
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command: string, args: Record<string, unknown>) =>
      command === "head_library_detail" ? [
        { id: 1, identity: "other_player_01_01_1980", game: "nhl24", platform: "ps4", textures: [] },
        { id: 2, identity: "ekblad_aaron_07_02_1996", game: "nhl24", platform: "ps4", textures: [] },
      ] : original(command, args));
    mount("Aaron Ekblad");
    fireEvent.click(await screen.findByRole("button", { name: "NHL24 - PS4" }));
    await waitFor(() => expect((screen.getByLabelText("Source texture set") as HTMLSelectElement).value).toBe("1"));
    open.mockResolvedValue("C:/exports");
    fireEvent.click(screen.getByText("Export GLB + DDS package"));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("head_library_export", expect.objectContaining({ appearance: 2 })));
  });
  it("restores head, hair and facial hair together and includes them in export", async () => {
    const first = mount();
    fireEvent.click(await screen.findByRole("button", { name: "NHL24 - PS4" }));
    await screen.findByText("1 meshes");
    for (const kind of ["hair", "facialhair"]) {
      fireEvent.change(screen.getByLabelText("Asset type"), { target: { value: kind } });
      fireEvent.click(await screen.findByText("Preview with selected head"));
      await screen.findByText(kind === "hair" ? "2 meshes" : "3 meshes");
    }
    fireEvent.click(screen.getByText("Remember appearance"));
    expect(screen.getByRole("status").textContent).toContain("saved");
    first.unmount();
    mount();
    await screen.findByText("3 meshes");
    open.mockResolvedValue("C:/exports");
    fireEvent.click(screen.getByText("Export GLB + DDS package"));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("head_library_export",
      expect.objectContaining({ id: "head", companions: ["hair", "facialhair"] })));
  });
  it("does not start an export when the destination dialog is cancelled", async () => {
    mount();
    fireEvent.click(await screen.findByRole("button", { name: "NHL24 - PS4" }));
    await screen.findByText("1 meshes");
    open.mockResolvedValue(null);
    fireEvent.click(screen.getByText("Export GLB + DDS package"));
    await waitFor(() => expect(open).toHaveBeenCalled());
    expect(invoke.mock.calls.some(([command]) => command === "head_library_export")).toBe(false);
  });
});

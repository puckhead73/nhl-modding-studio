// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { UniformsTab } from "./UniformsTab";

const mocks = vi.hoisted(() => ({
  pick: vi.fn(),
  call: vi.fn(async (_cmd: string, _args: unknown): Promise<unknown> => []),
  root: { install_dir: "H:/copied disc", platform: "ps3", rendering_override: null as string | null, fe_override: null },
}));
vi.mock("../lib/client", () => ({ call: mocks.call }));
vi.mock("../lib/project", () => ({
  useAssetRootArgs: () => ({ installDir: mocks.root.install_dir, rootArgs: mocks.root }),
  useProject: () => ({}),
}));
vi.mock("../lib/sidebarCollapse", () => ({ useSidebarCollapse: () => () => {} }));
vi.mock("./TextureReplace", () => ({ useTextureReplace: () => ({ pick: mocks.pick }), TextureThumbnail: ({ relPath }: { relPath: string }) => <img alt={relPath} /> }));
vi.mock("./NameplateEditor", () => ({ NameplateEditor: ({ relPath, referenceRelPath, onClose }: { relPath: string; referenceRelPath: string | null; onClose: () => void }) => <div role="dialog" aria-label="Layout editor"><span>{relPath}</span><span>{referenceRelPath}</span><button onClick={onClose}>Close editor</button></div> }));
vi.mock("./TeamKitsCard", () => ({ TeamKitsCard: () => null }));
vi.mock("./JerseyMetadataCard", () => ({ JerseyMetadataCard: () => null }));
vi.mock("./KitMeshViewer", () => ({ TeamKitViewer: () => null }));
afterEach(() => { cleanup(); mocks.call.mockReset(); mocks.pick.mockReset(); mocks.call.mockResolvedValue([]); mocks.root = { ...mocks.root, platform: "ps3", rendering_override: null }; });

it("shows the replaceable layout sheet and keeps editing exclusive to the preview button", async () => {
  const body = "rendering/jersey/texlib_0_1_0.rpsgl";
  const sheet = "rendering/jersey/name_0_1_0_cm.rpsgl";
  mocks.call.mockImplementation(async cmd => cmd === "team_outfits" ? [
    { piece: "jersey", file_name: "texlib_0_1_0.rpsgl", rel_path: body, label: "Jersey color map", texture_index: 1, is_nameplate: false },
    { piece: "name", file_name: "name_0_1_0_cm.rpsgl", rel_path: sheet, label: "Layout sheet", texture_index: 0, is_nameplate: true },
  ] : []);
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<MemoryRouter><QueryClientProvider client={client}><UniformsTab projectId={1} teamId={0} /></QueryClientProvider></MemoryRouter>);
  // The thumbnail must use the layout sheet itself, not another jersey body preview.
  await screen.findByRole("img", { name: sheet });
  const card = screen.getByText("Layout sheet").parentElement!;
  fireEvent.click(within(card).getByRole("button", { name: /Replace/ }));
  expect(mocks.pick).toHaveBeenCalledWith(sheet, "Layout sheet", 0);
  expect(screen.queryByRole("button", { name: /Edit locations/ })).toBeNull();
  expect(screen.queryByRole("button", { name: /Replace raw sheet/ })).toBeNull();
  expect(screen.getAllByRole("button", { name: "Edit Jersey Layout" })).toHaveLength(1);
  fireEvent.click(screen.getByRole("button", { name: "Edit Jersey Layout" }));
  await screen.findByRole("dialog", { name: "Layout editor" });
  expect(screen.getByText(sheet)).toBeTruthy();
  expect(screen.getByText(body)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Close editor" }));
  client.clear();
});

it("requests ambiguous PS4 filenames only after opting in and preserves the warning", async () => {
  mocks.root.platform = "ps4";
  mocks.call.mockImplementation(async (cmd,args) => cmd === "team_outfits" && (args as {includeAmbiguous?: boolean}).includeAmbiguous
    ? [{piece:"jersey", file_name:"jersey_nineties_anaheim_away_cm.gnf",rel_path:"jersey.gnf",label:"Unverified filename match · Jersey",texture_index:0,is_nameplate:false}] : []);
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<MemoryRouter><QueryClientProvider client={client}><UniformsTab projectId={1} teamId={0} /></QueryClientProvider></MemoryRouter>);
  await screen.findByText("No matched uniform kits");
  expect(screen.getByRole("link", { name: "Open asset extraction settings" }).getAttribute("href")).toBe("/app/settings/general");
  expect(mocks.call).toHaveBeenCalledWith("team_outfits",{projectId:1,teamId:0,roots:mocks.root});
  fireEvent.click(screen.getByLabelText("Show ambiguous filename matches"));
  await screen.findByText("Unverified filename match · Jersey");
  expect(screen.getByText(/These filenames may match several teams/)).toBeTruthy();
  expect(mocks.call).toHaveBeenCalledWith("team_outfits",{projectId:1,teamId:0,roots:mocks.root,includeAmbiguous:true});
  client.clear();
});

it("waits for PS4 textures before selecting a named family and retains manual selections", async () => {
  mocks.root.platform = "ps4";
  let finish!: (value: unknown) => void;
  const pending = new Promise((resolve) => { finish = resolve; });
  mocks.call.mockImplementation(async (cmd) => {
    if (cmd === "list_team_jerseys") return [{ style_raw: 0, variant_raw: 0, is_default: true }];
    if (cmd === "team_outfits") return pending;
    return [];
  });
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<MemoryRouter><QueryClientProvider client={client}><UniformsTab projectId={1} teamId={0} /></QueryClientProvider></MemoryRouter>);
  await screen.findByText("Finding this team's uniform textures…");
  finish(["jersey", "font"].map((piece) => ({ piece, file_name: `${piece}_nineties_anaheim_away_cm.gnf`, rel_path: `${piece}.gnf`, label: `${piece} away map`, texture_index: 0, is_nameplate: false })));
  const select = await screen.findByRole("combobox");
  await waitFor(() => expect((select as HTMLSelectElement).value).toBe("gnf:nineties_anaheim_away"));
  expect(screen.getByText("jersey away map")).toBeTruthy();
  expect(screen.queryByText(/Link this project's extracted game folder/)).toBeNull();
  expect(select.textContent).not.toContain("Infinity");
  fireEvent.change(select, { target: { value: "0_0" } });
  await waitFor(() => expect((select as HTMLSelectElement).value).toBe("0_0"));
  client.clear();
});

it("rescans a previously empty folder when returning after external extraction", async () => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const view = (visible: boolean) => <MemoryRouter><QueryClientProvider client={client}>{visible && <UniformsTab projectId={1} teamId={0} />}</QueryClientProvider></MemoryRouter>;
  const mounted = render(view(true));
  await screen.findByText("No matched uniform kits");
  expect(screen.getByRole("link", { name: "Open asset extraction settings" }).getAttribute("href")).toBe("/app/settings/general");
  await waitFor(() => expect(mocks.call.mock.calls.filter(([cmd]) => cmd === "team_outfits")).toHaveLength(1));
  // Leave the tab while retaining the same project/query cache, then return.
  mounted.rerender(view(false));
  mounted.rerender(view(true));
  await waitFor(() => expect(mocks.call.mock.calls.filter(([cmd]) => cmd === "team_outfits")).toHaveLength(2));
  client.clear();
});

it("does not reuse the previous rendering-folder inventory after an override change", async () => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const view = () => <MemoryRouter><QueryClientProvider client={client}><UniformsTab projectId={1} teamId={0} /></QueryClientProvider></MemoryRouter>;
  const mounted = render(view());
  await screen.findByText("No matched uniform kits");
  expect(screen.getByRole("link", { name: "Open asset extraction settings" }).getAttribute("href")).toBe("/app/settings/general");
  mocks.root = { ...mocks.root, rendering_override: "H:/relocated rendering" };
  mounted.rerender(view());
  await waitFor(() => expect(mocks.call).toHaveBeenCalledWith("team_outfits", {
    projectId: 1, teamId: 0, roots: mocks.root,
  }));
  expect(mocks.call.mock.calls.filter(([cmd]) => cmd === "team_outfits")).toHaveLength(2);
  client.clear();
});

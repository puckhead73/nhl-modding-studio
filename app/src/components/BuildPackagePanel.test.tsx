// @vitest-environment jsdom
//
// ONB-003 §4 — the Build mod package panel: the inventory drives the choices, blocked categories
// cannot be included, a console is declared only when every included category is supported on
// it, and the build request names what is included *and* what is explicitly excluded.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PackageInventoryDto } from "../bindings/PackageInventoryDto";

const calls: Array<{ cmd: string; args: any }> = [];
const handlers: Record<string, (args: any) => unknown> = {};
let projectDirty = false;
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: unknown) => {
    calls.push({ cmd, args });
    const h = handlers[cmd];
    if (!h) return Promise.reject(new Error(`unmocked command ${cmd}`));
    return Promise.resolve().then(() => h(args));
  },
}));
vi.mock("../lib/project", () => ({
  useProject: () => ({ project: { project_id: 5, dirty: projectDirty }, info: { game_id: "nhllegacy", platform: "xbox", dir: "D:\\p", saves: 1 } }),
}));

vi.mock("../lib/settings", () => ({ useSettings: () => ({ settings: { projectsRoot: "D:/Projects" } }) }));

import { BuildPackagePanel } from "./BuildPackagePanel";

function inventory(): PackageInventoryDto {
  return {
    dirty: false,
    saves: 1,
    categories: [
      { category: "cells", count: 2, status: "ready", items: ["Players › A › position: C → LW", "Players › B › position: D → G"], blockers: [] },
      { category: "dictionary", count: 1, status: "ready", items: ["TXT_LEAGUE_NORWAY: \"Norway\" → \"Nordic\""], blockers: [] },
      { category: "textures", count: 0, status: "empty", items: [], blockers: [] },
      { category: "members", count: 1, status: "blocked", items: [], blockers: ["databigs/x.big › logo: no installer adapter yet"] },
      { category: "native", count: 0, status: "empty", items: [], blockers: [] },
      { category: "structural", count: 0, status: "empty", items: [], blockers: [] },
    ],
    targets: [
      { platform: "xbox", status: "supported", components: [{ category: "cells", status: "supported", reason: "author" }, { category: "dictionary", status: "supported", reason: "key-based" }] },
      { platform: "ps3", status: "supported", components: [{ category: "cells", status: "supported", reason: "byte-identical" }, { category: "dictionary", status: "supported", reason: "key-based" }] },
      { platform: "recomp", status: "unsupported", components: [{ category: "cells", status: "supported", reason: "byte-identical" }, { category: "dictionary", status: "unsupported", reason: "no bundled dictionary" }] },
    ],
    blockers: [],
    default_name: "NHL Legacy mod",
    default_version: "1.0.1",
    exports_dir: "D:\\p\\exports",
    base: [],
  };
}

function renderPanel() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const view = render(
    <QueryClientProvider client={qc}>
      <BuildPackagePanel />
    </QueryClientProvider>,
  );
  return { ...view, qc };
}
const lastCall = (cmd: string) => [...calls].reverse().find((c) => c.cmd === cmd);

beforeEach(() => {
  projectDirty = false;
  calls.length = 0;
  for (const k of Object.keys(handlers)) delete handlers[k];
  handlers.package_inventory = () => inventory();
  handlers.package_build = () => ({ path: "D:\\p\\exports\\NHL Legacy mod-1.0.1.nhlmod", package_id: "p".repeat(64), len: 2048, sha256: "s".repeat(64), entries: 1, categories: [{ category: "cells", count: 2 }], targets: [{ platform: "xbox", status: "supported", components: [] }] });
});
afterEach(() => cleanup());

describe("ONB-003 build panel", () => {
  it("includes ready categories, keeps blocked ones out explicitly, offers only supported consoles and sends include + exclude", async () => {
    renderPanel();
    await screen.findByTestId("build-package");
    const boxes = screen.getAllByRole("checkbox") as HTMLInputElement[];
    const byLabel = (re: RegExp) => boxes.find((b) => b.parentElement?.textContent?.match(re))!;
    expect(byLabel(/Database cell edits/).checked).toBe(true);
    expect(byLabel(/Dictionary texts/).checked).toBe(true);
    expect(byLabel(/Archive-member replacements/).disabled).toBe(true);
    expect(screen.getByText(/no installer adapter yet/)).toBeTruthy();
    // Recompiled is blocked while the dictionary edit is included; it opens up once it is left out.
    expect(byLabel(/Recompiled/).disabled).toBe(true);
    fireEvent.click(byLabel(/Dictionary texts/));
    await waitFor(() => expect(byLabel(/Recompiled/).disabled).toBe(false));
    fireEvent.click(byLabel(/PS3/));
    fireEvent.click(byLabel(/Recompiled/));
    expect((screen.getByLabelText("Package name") as HTMLInputElement).value).toBe("NHL Legacy mod");
    fireEvent.change(screen.getByLabelText("Package version"), { target: { value: "2.0" } });
    fireEvent.click(screen.getByRole("button", { name: "Build package" }));
    await waitFor(() => expect(lastCall("package_build")).toBeTruthy());
    const req = lastCall("package_build")!.args.request;
    expect(req.name).toBe("NHL Legacy mod");
    expect(req.out_path).toBe("D:/Projects/Builds/NHL Legacy mod_ps3-recomp-xbox360_nhllegacy.nhlmod");
    expect(req.version).toBe("2.0");
    expect(req.include).toEqual(["cells"]);
    expect(req.exclude.sort()).toEqual(["dictionary", "members"]);
    expect(req.platforms.sort()).toEqual(["ps3", "recomp", "xbox"]);
    await screen.findByTestId("build-result");
  });

  it("an unsaved project cannot build", async () => {
    handlers.package_inventory = () => ({ ...inventory(), dirty: true, blockers: ["the project has unsaved edits; save the project first (a package carries saved edits only)"] });
    renderPanel();
    await screen.findByText(/unsaved edits/);
    expect((screen.getByRole("button", { name: "Build package" }) as HTMLButtonElement).disabled).toBe(true);
    expect(lastCall("package_build")).toBeUndefined();
  });

  it("does not build from a clean inventory response when the session has unsaved edits", async () => {
    projectDirty = true;
    renderPanel();
    await screen.findByTestId("build-package");
    await waitFor(() => expect((screen.getByLabelText("Package name") as HTMLInputElement).value).not.toBe(""));
    const button = screen.getByRole("button", { name: "Build package" }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    fireEvent.click(button);
    expect(lastCall("package_build")).toBeUndefined();
  });

  it("blocks a previously ready build during inventory refresh and after refresh failure, then recovers", async () => {
    const { qc } = renderPanel();
    const button = await screen.findByRole("button", { name: "Build package" }) as HTMLButtonElement;
    await waitFor(() => expect(button.disabled).toBe(false));

    // Keep a real refetch pending while the query retains its previous successful data.
    let failRefresh!: (reason: Error) => void;
    handlers.package_inventory = () => new Promise((_, reject) => { failRefresh = reject; });
    let refresh!: Promise<void>;
    await act(async () => {
      refresh = qc.invalidateQueries({ queryKey: ["package_inventory", 5] });
    });
    await waitFor(() => expect(button.disabled).toBe(true));
    fireEvent.click(button);
    expect(lastCall("package_build")).toBeUndefined();

    await act(async () => {
      failRefresh(new Error("inventory unavailable"));
      await refresh;
    });
    await screen.findByText(/inventory unavailable/);
    expect(button.disabled).toBe(true);
    fireEvent.click(button);
    expect(lastCall("package_build")).toBeUndefined();

    handlers.package_inventory = () => inventory();
    await act(async () => { await qc.invalidateQueries({ queryKey: ["package_inventory", 5] }); });
    await waitFor(() => expect(button.disabled).toBe(false));
  });
});

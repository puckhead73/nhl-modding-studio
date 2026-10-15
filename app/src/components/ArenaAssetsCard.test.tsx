// @vitest-environment jsdom
//
// Phase 2.5 (22 Sep 2026) — the read-only arena-assets card: each family carries its binding class,
// resolved files carry their evidence and the key they came from, spare shipped ids are listed on
// demand, a candidate family is never shown as established, a PS4 payload lists venue directories
// without claiming a binding, and a late answer for another arena is never painted. jsdom only.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ArenaAssetsDto } from "../bindings/ArenaAssetsDto";

type Pending = { cmd: string; args: any; resolve: (v: any) => void; reject: (e: any) => void };
const calls: Pending[] = [];
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) =>
    new Promise((resolve, reject) => {
      calls.push({ cmd, args, resolve, reject });
    }),
}));
const roots = { install_dir: "H:/extracted", platform: "xbox", rendering_override: null, fe_override: null };
vi.mock("../lib/project", () => ({
  useAssetRootArgs: () => ({ installDir: "H:/extracted", rootArgs: roots }),
}));

import { ArenaAssetsCard } from "./ArenaAssetsCard";

function gen7(arenaId: number): ArenaAssetsDto {
  return {
    arena_id: arenaId,
    arena_art_id: 12,
    contract_status: "gen7",
    contract: "venue model `stadium/stadium_<arenaartid>_container_<n>.rx2` and jumbotron … (research R62)",
    rendering_root: "H:/extracted/rendering",
    families: [
      {
        family: "venue_model",
        label: "Venue model",
        dir: "stadium",
        binding: "established",
        rule: "`stadium/stadium_<arenaartid>_container_<n>` — research R62 §1",
        files: [{ key: 12, key_source: "arenaartid", file_stem: "stadium_12_container_0", path: "H:/extracted/rendering/stadium/stadium_12_container_0.rx2", rel_path: "rendering/stadium/stadium_12_container_0.rx2", status: "present", reason: null }],
        shipped_files: 64,
        spare_keys: [66, 254, 600],
        notes: [],
      },
      {
        family: "centre_ice_logo",
        label: "Centre-ice logo",
        dir: "icesurface",
        binding: "via_home_team",
        rule: "`icesurface/centerlogo_<team art id>_cm` — keyed by the home team's art id, not the arena",
        files: [{ key: 40, key_source: "home team Anaheim Ducks (art id 40)", file_stem: "centerlogo_40_cm", path: null, rel_path: null, status: "missing", reason: "`icesurface/centerlogo_40_cm` is not in the listed directory" }],
        shipped_files: 231,
        spare_keys: [],
        notes: [],
      },
      {
        family: "bounce_map",
        label: "Ice bounce map",
        dir: "icesurface",
        binding: "candidate",
        rule: "`icesurface/icesurface_<bouncemap>_bm` — a candidate, not a binding",
        files: [{ key: 3, key_source: "bouncemap (candidate key)", file_stem: "icesurface_3_bm", path: "x", rel_path: "rendering/icesurface/icesurface_3_bm.rx2", status: "present", reason: null }],
        shipped_files: 37,
        spare_keys: [],
        notes: [],
      },
      {
        family: "ice_layers",
        label: "Other ice layers",
        dir: "icesurface",
        binding: "no_established_binding",
        rule: "small id spaces several arena columns cover — no established binding (R62 §3)",
        files: [],
        shipped_files: 47,
        spare_keys: [],
        notes: ["shipped: 37 centre-line, 6 line-art, 4 lightmap, 156 hotspot files"],
      },
    ],
    venue_directories: [],
    ps4_venue: null,
    notes: [`arena ${arenaId} note`, "shown, never written: …"],
  };
}

function ps4(arenaId: number): ArenaAssetsDto {
  return {
    arena_id: arenaId,
    arena_art_id: 7,
    contract_status: "ps4_vault_reference",
    contract: "PS4 venues are named directories under `rendering/stadiums/` selected through the Attrib / VLT vault …",
    rendering_root: "H:/extracted/nhl/data/kettle/rendering",
    families: [
      {
        family: "common_ice",
        label: "Shared ice families (`stadiums/common/`)",
        dir: "stadiums/common",
        binding: "candidate",
        rule: "numbered files; `arenaartid` fits the id space on most rows but so do other columns — a candidate key, not a binding (R64 §2)",
        files: [],
        shipped_files: 884,
        spare_keys: [],
        notes: ["283 `ice_centerlogo*` files", "this arena's `arenaartid` is 7; whether it selects any of these files is not established"],
      },
    ],
    venue_directories: [
      ["anaheim", 40],
      ["eashl_tier3", 12],
    ],
    ps4_venue: {
      key_label: "vault level name (observed on every shipped title; not a documented selector)",
      class: "ambiguous",
      reason: "2 level records carry id 0: `anaheim`, `common_ice_int`; the vault holds no attribute that settles which record the game uses, so every candidate is listed and none is chosen (research R73 §1)",
      candidates: [
        { level_name: "anaheim", dir: "stadiums/anaheim", folder: "present", members: 40, texpacks: [{ family: "center_logo", key: 0, in_set: true }, { family: "center_line", key: 13, in_set: true }], rink_id: 0, arena_id_attr: 0 },
        { level_name: "common_ice_int", dir: "stadiums/common_ice_int", folder: "missing", members: null, texpacks: [{ family: "center_logo", key: 100, in_set: false }], rink_id: null, arena_id_attr: 0 },
      ],
      shared_content: [["common", 12], ["unused", null]],
      dependencies: "a level record binds: the venue folder (by name), the centre-logo / centre-line / ice-line texture packs …",
      typed_refs: { ge_crowdplacement: 76 },
      reassignment_supported: false,
      reassignment_reason: "re-pointing `arenaartid` is a cascade, not a cell edit …",
    },
    notes: ["PS4 venues are named directories … Studio does not read the vault yet", "2 shipped venue directories under `stadiums/`; none is named by an arena row …"],
  };
}

function mount(arenaId: number) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const ui = (a: number) => (
    <QueryClientProvider client={qc}>
      <ArenaAssetsCard projectId={3} arenaId={a} />
    </QueryClientProvider>
  );
  const r = render(ui(arenaId));
  return { ...r, rerenderArena: (a: number) => r.rerender(ui(a)) };
}

describe("ArenaAssetsCard", () => {
  beforeEach(() => {
    calls.length = 0;
  });
  afterEach(cleanup);

  it("shows each family's binding class, the resolved files with their evidence, and the spare venues on demand", async () => {
    mount(5);
    await waitFor(() => expect(calls.length).toBe(1));
    expect(calls[0].cmd).toBe("arena_assets");
    expect(calls[0].args).toEqual({ projectId: 3, roots, arenaId: 5 });
    calls[0].resolve(gen7(5));
    await waitFor(() => expect(screen.getByTestId("arena-assets-card")).toBeTruthy());

    expect(screen.getByTestId("arena-assets-contract-status").textContent).toContain("gen7");
    expect(screen.getByTestId("arena-family-binding-venue_model").textContent).toBe("established");
    expect(screen.getByTestId("arena-file-status-venue_model-stadium_12_container_0").textContent).toBe("present");
    expect(screen.getByTestId("arena-file-venue_model-stadium_12_container_0").textContent).toContain("arenaartid");
    // The team-keyed family says where its key came from, and reports the miss with its reason.
    expect(screen.getByTestId("arena-family-binding-centre_ice_logo").textContent).toBe("via home team");
    expect(screen.getByTestId("arena-file-centre_ice_logo-centerlogo_40_cm").textContent).toContain("home team Anaheim Ducks (art id 40)");
    expect(screen.getByTestId("arena-file-status-centre_ice_logo-centerlogo_40_cm").textContent).toBe("missing");
    expect(screen.getByTestId("arena-file-centre_ice_logo-centerlogo_40_cm").textContent).toContain("not in the listed directory");
    // A candidate is labelled a candidate — never "established".
    expect(screen.getByTestId("arena-family-binding-bounce_map").textContent).toBe("candidate key");
    expect(screen.getByTestId("arena-family-binding-ice_layers").textContent).toBe("no established binding");
    expect(screen.getByTestId("arena-family-notes-ice_layers").textContent).toContain("156 hotspot");
    // Spares fold open.
    const toggle = screen.getByTestId("arena-spares-toggle-venue_model");
    expect(toggle.textContent).toContain("3 shipped ids no arena row names");
    expect(screen.queryByTestId("arena-spares-venue_model")).toBeNull();
    fireEvent.click(toggle);
    await waitFor(() => expect(screen.getByTestId("arena-spares-venue_model").textContent).toBe("66, 254, 600"));
    expect(screen.queryByTestId("arena-assets-venue-dirs")).toBeNull();
  });

  it("lists PS4 venue directories and the vault reference without claiming a binding", async () => {
    mount(9);
    await waitFor(() => expect(calls.length).toBe(1));
    calls[0].resolve(ps4(9));
    await waitFor(() => expect(screen.getByTestId("arena-assets-card")).toBeTruthy());
    expect(screen.getByTestId("arena-assets-contract-status").textContent).toContain("vault reference");
    expect(screen.getByTestId("arena-assets-venue-dirs").textContent).toContain("anaheim (40)");
    expect(screen.getByTestId("arena-assets-venue-dirs").textContent).toContain("vault level record");
    // R73 listing: the class and reason, every candidate with its folder status and texpack keys, shared content once, no re-point.
    expect(screen.getByTestId("arena-venue-levels").getAttribute("data-class")).toBe("ambiguous");
    expect(screen.getByTestId("arena-venue-class").textContent).toContain("none chosen");
    expect(screen.getByTestId("arena-venue-reason").textContent).toContain("none is chosen");
    expect(screen.getByTestId("arena-venue-level-anaheim").getAttribute("data-folder")).toBe("present");
    expect(screen.getByTestId("arena-venue-level-anaheim").textContent).toContain("folder present (40 files)");
    expect(screen.getByTestId("arena-venue-level-common_ice_int").textContent).toContain("folder missing from the attached tree");
    expect(screen.getByTestId("arena-venue-level-common_ice_int").textContent).toContain("no texpack with this key");
    expect(screen.getByTestId("arena-venue-shared").textContent).toContain("common (12) · unused");
    expect(screen.getByTestId("arena-venue-reassignment").textContent).toContain("Re-pointing is not offered");
    expect(screen.queryByText(/re-point|reassign/i, { selector: "button" })).toBeNull();
    expect(screen.getByTestId("arena-family-binding-common_ice").textContent).toBe("candidate key");
    expect(screen.getByTestId("arena-family-notes-common_ice").textContent).toContain("not established");
    expect(screen.queryByText("established", { exact: true })).toBeNull();
  });

  it("never paints a late answer for another arena as this arena's evidence", async () => {
    const m = mount(1);
    await waitFor(() => expect(calls.length).toBe(1));
    m.rerenderArena(2);
    await waitFor(() => expect(calls.length).toBe(2));
    calls[0].resolve(gen7(1));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("arena-assets-card")).toBeNull();
    calls[1].resolve(gen7(2));
    await waitFor(() => expect(screen.getByTestId("arena-assets-card")).toBeTruthy());
    expect(screen.getByTestId("arena-assets-card").getAttribute("data-arena-id")).toBe("2");
    expect(screen.getByTestId("arena-assets-notes").textContent).toContain("arena 2 note");
    expect(screen.getByTestId("arena-assets-notes").textContent).not.toContain("arena 1 note");
  });

  it("says when the arena is not in the table and shows the backend's error", async () => {
    mount(4);
    await waitFor(() => expect(calls.length).toBe(1));
    calls[0].resolve(null);
    await waitFor(() => expect(screen.getByTestId("arena-assets-none")).toBeTruthy());
    cleanup();
    calls.length = 0;
    mount(4);
    await waitFor(() => expect(calls.length).toBe(1));
    calls[0].reject(new Error("the open project has no recorded title"));
    await waitFor(() => expect(screen.getByTestId("arena-assets-error").textContent).toContain("no recorded title"));
  });
});

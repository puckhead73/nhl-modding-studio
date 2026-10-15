// @vitest-environment jsdom
//
// Presentation 5.2 (receipt 49) — the read-only venue-art gallery: materials with their slot rasters, an
// on-demand preview that names the listed package hash, PS3 rasters without preview, an unmeasured title's note.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
let art: any = null;
let preview: any = null;
let boardPreview: any = null;
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: () => Promise.resolve("C:/art/board.png") }));
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    if (cmd === "arena_venue_raster") return preview instanceof Error ? Promise.reject(preview) : Promise.resolve(preview);
    if (cmd === "ps3_board_replacement_preview") return Promise.resolve(boardPreview);
    if (cmd === "asset_member_apply_review") return Promise.resolve({ plan: boardPreview.plan, pending: 1, summary: { project_id: 3 } });
    return Promise.resolve(art);
  },
}));
const roots = { install_dir: "X:/NHL Legacy", platform: "xbox", rendering_override: null, fe_override: null };
vi.mock("../lib/project", () => ({ useAssetRootArgs: () => ({ installDir: "X:/NHL Legacy", rootArgs: roots }), useProject: () => ({ update: () => {} }) }));

import { ArenaVenueArtCard } from "./ArenaVenueArtCard";

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <ArenaVenueArtCard projectId={3} arenaId={7} />
    </QueryClientProvider>,
  );
}

const pkg = (previewable: boolean) => ({
  file: "stadium/stadium_1_container_0.rx2",
  bytes: 12_000_000n,
  sha256: "ab".repeat(32),
  platform: previewable ? "xbox" : "ps3",
  other_materials: 51,
  notes: [],
  materials: [
    { name: "adbd_WPG", class: "Board ad", shader: "env_adbd.FX", slots: [{ raster: 23 }, { raw: "0x4758" }] },
    { name: "powerring_atlanta1", class: "Power ring", shader: null, slots: [{ raw: "0x400000" }, { unreadable: "the parameter block (288 bytes) does not reach +288" }] },
  ],
  rasters: Array.from({ length: 24 }, (_, k) => ({ ordinal: k, descriptor: 100 + k, width: 512, height: k === 23 ? 1024 : 512, format: "DXT1", previewable, note: previewable ? null : "PS3 venue rasters are listed by size only", used_by: k === 23 ? ["adbd_WPG (slot 0)"] : [] })),
});

describe("ArenaVenueArtCard", () => {
  afterEach(() => {
    cleanup();
    calls.length = 0;
  });

  it("lists presentation materials and previews a raster by the listed hash", async () => {
    art = { arena_art_id: 1n, measured: true, scope: "research R127f …", notes: [], packages: [pkg(true)] };
    preview = { width: 512, height: 1024, png_data_url: "data:image/png;base64,AAAA" };
    mount();
    expect(await screen.findByText("adbd_WPG")).toBeTruthy();
    expect(screen.getByText("stored 0x400000 (no raster)")).toBeTruthy();
    expect(screen.getByText("not readable")).toBeTruthy();
    fireEvent.click(screen.getByTestId("venue-open-adbd_WPG"));
    const img = (await screen.findByTestId("venue-preview")) as HTMLImageElement;
    expect(img.src).toContain("data:image/png");
    const c = calls.find((x) => x.cmd === "arena_venue_raster")!;
    expect(c.args).toMatchObject({ projectId: 3, arenaId: 7, file: "stadium/stadium_1_container_0.rx2", sha256: "ab".repeat(32), ordinal: 23 });
    expect(screen.getByText(/named by adbd_WPG \(slot 0\)/)).toBeTruthy();
  });

  it("shows a refused preview's reason (file changed since listing)", async () => {
    art = { arena_art_id: 1n, measured: true, scope: "s", notes: [], packages: [pkg(true)] };
    preview = new Error("… changed since it was listed; list the arena again");
    mount();
    fireEvent.click(await screen.findByTestId("venue-open-adbd_WPG"));
    await waitFor(() => expect(screen.getByTestId("venue-preview-error").textContent).toMatch(/changed since it was listed/));
  });

  it("PS3 rasters have no preview button; an unmeasured title explains itself", async () => {
    art = { arena_art_id: 1n, measured: true, scope: "s", notes: [], packages: [pkg(false)] };
    mount();
    expect(await screen.findByText("adbd_WPG")).toBeTruthy();
    expect(screen.queryByTestId("venue-open-adbd_WPG")).toBeNull();
    cleanup();
    art = { arena_art_id: 1n, measured: false, scope: "s", notes: ["the venue packages of nhl12 on xbox are not measured"], packages: [] };
    mount();
    expect((await screen.findByTestId("venue-unmeasured")).textContent).toMatch(/not measured/);
  });
  it("offers Replace on a PS3 board raster: preview names package and descriptor, Apply joins the pending changes (receipt 67)", async () => {
    art = { arena_art_id: 1n, measured: true, scope: "s", notes: [], packages: [{ ...pkg(false), file: "stadium/stadium_321_container_0.rpsgl" }] };
    boardPreview = {
      plan: { ticket: "ns/9", archive_relative: "cacherender.big", member_path: "rendering/stadium/stadium_321_container_0.rpsgl", container: "ps3-venue-dxt1", width: 512, height: 1024, stored_form: "chunkzip", source_stored_len: 10n, new_stored_len: 9n, shared_with: [], scope: "this-member", placement_prediction: "fits the member's slot", description: "cacherender.big + rendering/stadium/stadium_321_container_0.rpsgl: board raster 123", paired_loose: "rendering/stadium/stadium_321_container_0.rpsgl", loose_must_be_absent: null, notes: ["pair: both are written as one transaction"] },
      refusal_kind: null,
      refusal: null,
      topology: "pair",
      format: "DXT1",
      levels: 11,
      materials: ["adbd_WPG", "adbd_other"],
    };
    mount();
    fireEvent.click(await screen.findByTestId("venue-replace-adbd_WPG"));
    expect((await screen.findByTestId("board-summary")).textContent).toMatch(/DXT1 512×1024 · 11 levels · loose package \+ archive member, written together/);
    expect(screen.getByTestId("board-materials").textContent).toMatch(/adbd_WPG, adbd_other — all of them change/);
    const c = calls.find((x) => x.cmd === "ps3_board_replacement_preview")!;
    expect(c.args).toMatchObject({ projectId: 3, installRoot: "X:/NHL Legacy", package: "stadium_321_container_0.rpsgl", descriptor: 123, sourceImagePath: "C:/art/board.png" });
    fireEvent.click(screen.getByTestId("board-apply"));
    await screen.findByTestId("board-result");
    expect(calls.find((x) => x.cmd === "asset_member_apply_review")!.args).toEqual({ projectId: 3, ticket: "ns/9" });
    cleanup();
    // Xbox 360 boards stay read-only (their writer is withheld).
    art = { arena_art_id: 1n, measured: true, scope: "s", notes: [], packages: [pkg(true)] };
    mount();
    expect(await screen.findByText("adbd_WPG")).toBeTruthy();
    expect(screen.queryByTestId("venue-replace-adbd_WPG")).toBeNull();
  });
});
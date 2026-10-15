// @vitest-environment jsdom
//
// P1-G — the league-imagery card: families, readings, consumers, member rows with existence
// pills, and the Replace action routed to the shared texture surface (component evidence only).
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { LeagueImageryDto } from "../bindings/LeagueImageryDto";

const calls: Array<{ cmd: string; args: any }> = [];
const handlers: Record<string, (args: any) => unknown> = {};
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: unknown) => {
    calls.push({ cmd, args });
    const h = handlers[cmd];
    if (!h) return Promise.reject(new Error(`unmocked command ${cmd}`));
    return Promise.resolve().then(() => h(args));
  },
}));
let installDir: string | null = "E:/x/nhllegacy";
vi.mock("../lib/project", () => ({
  useProject: () => ({ install: installDir ? { install_dir: installDir } : null, iso: null, update: () => {} }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: () => Promise.resolve("C:/tmp/d0.png"),
  save: () => Promise.resolve(null),
}));

import { LeagueImageryCard } from "./LeagueImageryCard";

function dto(): LeagueImageryDto {
  return {
    game_id: "nhllegacy",
    platform: "xbox360",
    registry: "boot.big: fe/ion/game/pods/helpers/generalhelpers.big (getArtAssetPath)",
    families: [
      {
        family: "divisionlogos",
        label: "Playoff-tree conference / league logos",
        reading: "d0 NHL Western Conference, d1 NHL Eastern Conference",
        consumers: ["screens/dynasty/dm36_playofftree"],
        texture_facts: ["0:256x256:0x86"],
        members: [
          { id: 0n, rel_path: "fe/ion/artassets/divisionlogos/d0.big", exists: true, fr_rel_path: null, fr_exists: null, archive: null },
          { id: 1n, rel_path: "fe/ion/artassets/divisionlogos/d1.big", exists: false, fr_rel_path: null, fr_exists: null, archive: null },
        ],
      },
      {
        family: "large_league_logo",
        label: "Large league logo",
        reading: "",
        consumers: [],
        texture_facts: [],
        members: [{ id: 7n, rel_path: "nhl/ui/kettle_assets/leaguelogos/size512/l7.gnf", exists: true, fr_rel_path: "nhl/ui/kettle_assets/leaguelogos/size512/l7_fr.gnf", fr_exists: true, archive: "databigs/inxp_10.big" }],
      },
    ],
    notes: ["the id ↔ league mapping is computed in game code (unknown)"],
  };
}

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <LeagueImageryCard projectId={1} />
    </QueryClientProvider>,
  );
}

describe("LeagueImageryCard", () => {
  beforeEach(() => {
    calls.length = 0;
    for (const k of Object.keys(handlers)) delete handlers[k];
    handlers.league_imagery = () => dto();
    handlers.texture_preview = () => ({ data_url: "data:image/png;base64,", width: 256, height: 256, container: "portrait" });
  });
  afterEach(() => cleanup());

  it("lists families with readings, consumers, member rows and existence pills", async () => {
    mount();
    await screen.findByText("Playoff-tree conference / league logos");
    expect(screen.getByText(/d0 NHL Western Conference/)).toBeTruthy();
    expect(screen.getByText(/Screens: screens\/dynasty\/dm36_playofftree/)).toBeTruthy();
    expect(screen.getByText(/No screen references this family/)).toBeTruthy();
    expect(screen.getByText(/fe\/ion\/artassets\/divisionlogos\/d1\.big/)).toBeTruthy();
    expect(screen.getByText("not extracted here")).toBeTruthy();
    expect(screen.getByText(/256x256 0x86/)).toBeTruthy();
    expect(screen.getByText(/French twin/)).toBeTruthy();
    expect(screen.getByText(/computed in game code/)).toBeTruthy();
    expect(calls[0]).toEqual({ cmd: "league_imagery", args: { projectId: 1, installRoot: "E:/x/nhllegacy" } });
    // Replace buttons only for members that exist; the French twin gets its own.
    expect(screen.getAllByText("Replace…")).toHaveLength(2);
    expect(screen.getAllByText("Replace French…")).toHaveLength(1);
  });

  it("routes Replace through the shared texture surface with the member path", async () => {
    mount();
    await screen.findByText("Playoff-tree conference / league logos");
    fireEvent.click(screen.getAllByText("Replace…")[0]);
    await waitFor(() => expect(calls.some((c) => c.cmd === "texture_preview" && c.args.relPath === "fe/ion/artassets/divisionlogos/d0.big")).toBe(true));
  });

  it("offers the archive route only for members that ship in a packed archive, and previews through it", async () => {
    handlers.asset_member_preview = () => ({
      plan: { ticket: "ns/1", archive_relative: "databigs/inxp_10.big", member_path: "nhl/ui/kettle_assets/leaguelogos/size512/l7.gnf", container: "gnf", width: 512, height: 512, stored_form: "chunkzip", source_stored_len: 100n, new_stored_len: 120n, shared_with: ["nhl/ui/kettle_assets/leaguelogos/size512/l7_fr.gnf"], placement_prediction: "the member shares its stored bytes with 1 — a private copy is appended", description: "d" },
      refusal_kind: null,
      refusal: null,
    });
    handlers.asset_member_apply_review = (a: any) => ({ plan: (handlers.asset_member_preview!(a) as any).plan, pending: 1, summary: { project_id: 1, dirty: true } });
    mount();
    await screen.findByText("Playoff-tree conference / league logos");
    expect(screen.getAllByText("Replace in archive…")).toHaveLength(1);
    expect(screen.getAllByText("French in archive…")).toHaveLength(1);
    fireEvent.click(screen.getByText("Replace in archive…"));
    await screen.findByTestId("archive-member-dialog");
    expect(screen.getByText(/l7_fr\.gnf/)).toBeTruthy();
    const c = calls.find((c) => c.cmd === "asset_member_preview")!;
    expect(c.args).toEqual({ projectId: 1, installRoot: "E:/x/nhllegacy", archiveRelative: "databigs/inxp_10.big", memberPath: "nhl/ui/kettle_assets/leaguelogos/size512/l7.gnf", sourceImagePath: "C:/tmp/d0.png", wholeGroup: false });
    fireEvent.click(screen.getByText("Apply"));
    await screen.findByTestId("archive-member-result");
    expect(calls.some((c) => c.cmd === "asset_member_apply_review" && c.args.ticket === "ns/1")).toBe(true);
  });

  it("explains an absent inventory", async () => {
    handlers.league_imagery = () => null;
    mount();
    await screen.findByText(/No league-keyed imagery inventory/);
  });
});

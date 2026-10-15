// @vitest-environment jsdom
//
// C9 (Phase 2.4, overnight 2026-09-20) — the Team logos card: every art-id-keyed family the
// domain measured for the title as a row with existence pill and thumbnail, the loose Replace…
// routed through the shared texture surface with the member path, and Replace in archive… only
// where a packed archive is named (component evidence only; the family table is the Rust suite's).
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TeamLogoSetDto } from "../bindings/TeamLogoSetDto";

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
vi.mock("../lib/project", () => ({
  useProject: () => ({
    install: { install_dir: "E:/x/nhllegacy" },
    iso: null,
    update: () => {},
  }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: () => Promise.resolve("C:/tmp/t10.png"),
  save: () => Promise.resolve(null),
}));

import { TeamLogoCard } from "./TeamLogoCard";

function dto(): TeamLogoSetDto {
  return {
    artid: 10n,
    evidence: "measured",
    variants: [
      {
        family: "teamlogos",
        rel_path: "fe/ion/artassets/teamlogos/t10.big",
        exists: true,
        archive: "cache.big",
      },
      {
        family: "teamlogossmall",
        rel_path: "fe/ion/artassets/teamlogossmall/s10.big",
        exists: false,
        archive: "cache.big",
      },
      {
        family: "teamlogosreflection",
        rel_path: "fe/ion/artassets/teamlogosreflection/r10.big",
        exists: null,
        archive: null,
      },
    ],
    notes: ["which screen draws which family is research unknown"],
  };
}

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <TeamLogoCard projectId={1} teamId={3} />
    </QueryClientProvider>,
  );
}

describe("TeamLogoCard", () => {
  beforeEach(() => {
    calls.length = 0;
    for (const k of Object.keys(handlers)) delete handlers[k];
    handlers.team_logo_set = () => dto();
    handlers.texture_preview = () => ({
      data_url: "data:image/png;base64,",
      width: 256,
      height: 256,
      container: "portrait",
    });
  });
  afterEach(() => cleanup());

  it("lists every measured family with its path, existence pill and the routes it can take", async () => {
    mount();
    await screen.findByTestId("team-logo-set");
    expect(calls[0]).toEqual({
      cmd: "team_logo_set",
      args: { projectId: 1, teamId: 3, installRoot: "E:/x/nhllegacy" },
    });
    expect(screen.getByText(/fe\/ion\/artassets\/teamlogossmall\/s10\.big/)).toBeTruthy();
    expect(screen.getByText("not extracted here")).toBeTruthy();
    expect(screen.getByText("no install root")).toBeTruthy();
    // Loose Replace only for the file that exists; the archive route for the two members that ship in cache.big.
    expect(screen.getAllByText("Replace…")).toHaveLength(1);
    expect(screen.getAllByText("Replace in archive…")).toHaveLength(2);
  });

  it("routes Replace through the shared texture surface with the member path", async () => {
    mount();
    await screen.findByTestId("team-logo-set");
    fireEvent.click(screen.getByText("Replace…"));
    await waitFor(() =>
      expect(calls.some((c) => c.cmd === "texture_preview" && c.args.relPath === "fe/ion/artassets/teamlogos/t10.big")).toBe(true),
    );
  });

  it("uses the discovered loose path for previews but keeps the archive member unchanged", async () => {
    const member = "nhl/ui/assets/teamlogos/size256/t0.png";
    const loose = `databigs/Extracted/${member}`;
    handlers.team_logo_set = () => ({ ...dto(), variants: [{
      family: "size256", rel_path: member, loose_rel_path: loose,
      exists: true, archive: "databigs/inxp10.big",
    }] });
    handlers.asset_member_preview = () => ({ plan: null, refusal_kind: "test", refusal: "preview only" });
    mount();
    await screen.findByTestId("team-logo-set");
    fireEvent.click(screen.getByText("Replace…"));
    await waitFor(() => expect(calls.some(c => c.cmd === "texture_preview" && c.args.relPath === loose)).toBe(true));
    fireEvent.click(screen.getByText("Replace in archive…"));
    await waitFor(() => expect(calls.some(c => c.cmd === "asset_member_preview" && c.args.memberPath === member)).toBe(true));
    expect(calls.filter(c => c.cmd === "texture_preview").every(c => c.args.relPath === loose)).toBe(true);
  });

  it("previews the archive route with the archive and member the row names", async () => {
    handlers.asset_member_preview = () => ({
      plan: {
        ticket: "ns/9",
        archive_relative: "cache.big",
        member_path: "fe/ion/artassets/teamlogossmall/s10.big",
        container: "portrait",
        width: 128,
        height: 64,
        stored_form: "raw",
        source_stored_len: 4080n,
        new_stored_len: 4080n,
        shared_with: [],
        scope: "this-member",
        placement_prediction: "fits its slot",
        description: "d",
      },
      refusal_kind: null,
      refusal: null,
    });
    mount();
    await screen.findByTestId("team-logo-set");
    fireEvent.click(screen.getAllByText("Replace in archive…")[1]);
    await screen.findByTestId("archive-member-dialog");
    const c = calls.find((c) => c.cmd === "asset_member_preview")!;
    expect(c.args).toEqual({
      projectId: 1,
      installRoot: "E:/x/nhllegacy",
      archiveRelative: "cache.big",
      memberPath: "fe/ion/artassets/teamlogossmall/s10.big",
      sourceImagePath: "C:/tmp/t10.png",
      wholeGroup: false,
    });
  });

  it("renders nothing when the team has no art id", async () => {
    handlers.team_logo_set = () => null;
    const { container } = mount();
    await waitFor(() => expect(calls.some((c) => c.cmd === "team_logo_set")).toBe(true));
    expect(container.textContent).toBe("");
  });
});

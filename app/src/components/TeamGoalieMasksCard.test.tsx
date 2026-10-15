// @vitest-environment jsdom
//
// Equipment 4.5 G2 (receipt 42) — the team goalie mask paints card: slots with starter / backup paints,
// file counts, a reported (not refused) PS4 shipped exception, nothing on titles without slots, errors
// as text. jsdom only.
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
let response: any = null;
let failWith: string | null = null;
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    if (failWith) return Promise.reject(new Error(failWith));
    return Promise.resolve(response);
  },
}));

import { TeamGoalieMasksCard } from "./TeamGoalieMasksCard";

const roots = { install_dir: "X:/NHL Legacy", platform: "xbox", rendering_override: null, fe_override: null } as any;

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <TeamGoalieMasksCard projectId={3} teamId={12} roots={roots} />
    </QueryClientProvider>,
  );
}

const paint = (slot: number, role: string, value: number, status: string, file_count = 0) => ({
  slot,
  role,
  field_name: `goaliemask${slot}_${role}`,
  field_ref: "kOtt.qHNn",
  value,
  files: file_count ? [`goaliehelmet/goaliehelmet_1_${value}_cm.rx2`] : [],
  file_count,
  status,
  note: `paint ${value}: ${status}`,
});

describe("TeamGoalieMasksCard", () => {
  beforeEach(() => {
    calls.length = 0;
    response = null;
    failWith = null;
  });
  afterEach(cleanup);

  it("shows each slot's starter and backup paint with its file state", async () => {
    response = {
      applies: true,
      catalogue_ids: 230,
      notes: ["7 numbered mask slots …; which uniform uses which slot … is not established"],
      paints: [paint(0, "starter", 41, "files", 3), paint(0, "backup", 0, "empty"), paint(1, "starter", 248, "shipped_exception"), paint(1, "backup", 77, "no_files")],
    };
    mount();
    await waitFor(() => expect(screen.getByTestId("team-goalie-masks")).toBeTruthy());
    expect(calls).toEqual([{ cmd: "team_goalie_masks", args: { projectId: 3, teamId: 12, roots } }]);
    expect(screen.getByTestId("team-mask-0-starter").textContent).toContain("paint 41");
    expect(screen.getByTestId("team-mask-0-starter").textContent).toContain("3 files");
    expect(screen.getByTestId("team-mask-0-backup").textContent).toContain("none");
    expect(screen.getByTestId("team-mask-1-starter").textContent).toContain("shipped exception");
    expect(screen.getByTestId("team-mask-1-backup").textContent).toContain("no files");
    expect(screen.getByTestId("team-goalie-masks").textContent).toContain("not established");
  });

  it("renders nothing on a title without team mask slots, and errors as text", async () => {
    response = { applies: false, catalogue_ids: 0, notes: [], paints: [] };
    const { container } = mount();
    await waitFor(() => expect(calls.length).toBe(1));
    expect(container.textContent).toBe("");
    cleanup();
    failWith = "no team with id 12";
    mount();
    await waitFor(() => expect(screen.getByTestId("team-goalie-masks-error").textContent).toContain("no team with id 12"));
  });
});

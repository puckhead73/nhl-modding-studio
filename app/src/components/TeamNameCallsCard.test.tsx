// @vitest-environment jsdom
//
// Phase 2.7 (21 Sep 2026) — the name-calls card: groups render with their bank resolution and the
// keyword count; a team without a call says so with the domain's note; a title outside coverage
// shows the reason; no install root → nothing (and no call). jsdom only.
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
let response: any = null;
let failWith: string | null = null;
let installDir: string | null = "X:/install";
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    if (failWith) return Promise.reject(new Error(failWith));
    return Promise.resolve(response);
  },
}));
vi.mock("../lib/project", () => ({
  useProject: () => ({ install: installDir ? { install_dir: installDir } : null, iso: null }),
}));

import { TeamNameCallsCard } from "./TeamNameCallsCard";

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <TeamNameCallsCard projectId={7} teamId={3} />
    </QueryClientProvider>,
  );
}

describe("TeamNameCallsCard", () => {
  beforeEach(() => {
    calls.length = 0;
    response = null;
    failWith = null;
    installDir = "X:/install";
  });
  afterEach(cleanup);

  it("lists the groups with their bank resolution and the keyword mentions", async () => {
    response = {
      team_id: 3,
      team_name: "Boston Bruins",
      team_audio_id: 114,
      team_tag: 1,
      tables: [
        { num_tags: 1, sample_ids: [10, 11, 12], sample_count: 3, bank_resolved: 3 },
        { num_tags: 2, sample_ids: [20], sample_count: 70, bank_resolved: 69 },
      ],
      sample_ids_distinct: 73,
      bank_resolved_distinct: 72,
      keyword_mentions: 5,
      notes: ["read-only: …"],
    };
    mount();
    await waitFor(() => expect(screen.getByTestId("name-calls")).toBeTruthy());
    expect(calls).toEqual([{ cmd: "team_name_calls", args: { projectId: 7, teamId: 3, installRoot: "X:/install" } }]);
    expect(screen.getByTestId("name-calls-summary").textContent).toContain("73 distinct samples in 2 groups — 72 in the sample bank");
    expect(screen.getByTestId("name-calls-group-1").textContent).toContain("team alone");
    expect(screen.getByTestId("name-calls-group-1").textContent).toContain("3 samples, 3 in the bank");
    expect(screen.getByTestId("name-calls-group-2").textContent).toContain("team + 1 parameter");
    expect(screen.getByTestId("name-calls-group-2").textContent).toContain("… (+69)");
    expect(screen.getByTestId("name-calls").textContent).toContain("keyword index: 5 mentions");
  });

  it("a team without a call says so with the domain's note", async () => {
    response = { team_id: 3, team_name: "T", team_audio_id: 333, team_tag: 1, tables: [], sample_ids_distinct: 0, bank_resolved_distinct: 0, keyword_mentions: 0, notes: ["no play-by-play table is keyed by audio id 333 — …"] };
    mount();
    await waitFor(() => expect(screen.getByTestId("name-calls-none")).toBeTruthy());
    expect(screen.getByTestId("name-calls").textContent).toContain("no play-by-play table is keyed by audio id 333");
  });

  it("a title outside coverage shows the reason", async () => {
    failWith = "nhl09 ships the speech tables in EA's XML source form; the XML reader is not built yet";
    mount();
    await waitFor(() => expect(screen.getByTestId("name-calls-unavailable")).toBeTruthy());
    expect(screen.getByTestId("name-calls-unavailable").textContent).toContain("XML reader is not built yet");
  });

  it("renders nothing and calls nothing without an install root", async () => {
    installDir = null;
    const r = mount();
    await new Promise((res) => setTimeout(res, 20));
    expect(r.container.textContent).toBe("");
    expect(calls).toHaveLength(0);
  });
});

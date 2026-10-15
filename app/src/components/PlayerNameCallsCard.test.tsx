// @vitest-environment jsdom
//
// Phase 3.5 (21 Sep 2026) — the player recorded-name card: both sides render with their bank
// resolution; a catalogue name without a recording says so; no announcer recording says so; a
// coverage refusal shows the reason; no install root → nothing (and no call). jsdom only.
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

import { PlayerNameCallsCard } from "./PlayerNameCallsCard";

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <PlayerNameCallsCard projectId={7} playerId={42} />
    </QueryClientProvider>,
  );
}

const base = {
  player_id: 42,
  first_name: "Sidney",
  last_name: "Crosby",
  pbp_player_tag: 2,
  announcer_audio_id: 1234,
  announcer_tag: 2163,
  notes: ["read-only: …"],
};

describe("PlayerNameCallsCard", () => {
  beforeEach(() => {
    calls.length = 0;
    response = null;
    failWith = null;
    installDir = "X:/install";
  });
  afterEach(cleanup);

  it("renders both sides with their bank resolution", async () => {
    response = {
      ...base,
      pbp_names: [
        { name: "Crosby", is_first_name: false, bank: 3, audio_id: 30100, tables: [{ num_tags: 1, sample_ids: [1, 2], sample_count: 2, bank_resolved: 2 }], sample_ids_distinct: 2, bank_resolved_distinct: 2 },
        { name: "Sidney", is_first_name: true, bank: 3, audio_id: 30101, tables: [], sample_ids_distinct: 0, bank_resolved_distinct: 0 },
      ],
      announcer_tables: [{ num_tags: 2, sample_ids: [9], sample_count: 40, bank_resolved: 39 }],
      announcer_sample_ids_distinct: 40,
      announcer_bank_resolved_distinct: 39,
    };
    mount();
    await waitFor(() => expect(screen.getByTestId("player-calls")).toBeTruthy());
    expect(calls).toEqual([{ cmd: "player_name_calls", args: { projectId: 7, playerId: 42, installRoot: "X:/install" } }]);
    expect(screen.getByTestId("player-calls-name-last").textContent).toContain("Crosby");
    expect(screen.getByTestId("player-calls-name-last").textContent).toContain("2 distinct samples — all in the sample bank");
    expect(screen.getByTestId("pbp-last-group-1").textContent).toContain("name alone");
    expect(screen.getByTestId("player-calls-name-first").textContent).toContain("in the catalogue, no recording");
    expect(screen.getByTestId("player-calls-announcer-summary").textContent).toContain("40 distinct samples — 39 in the sample bank");
    expect(screen.getByTestId("ann-group-2").textContent).toContain("name + 1 parameter");
    expect(screen.getByTestId("ann-group-2").textContent).toContain("… (+39)");
    expect(screen.getByTestId("player-calls-announcer").textContent).toContain("playerid tag 2163");
  });

  it("says so when neither side has a recording", async () => {
    response = { ...base, pbp_names: [], announcer_tables: [], announcer_sample_ids_distinct: 0, announcer_bank_resolved_distinct: 0, notes: ["no announcer table is keyed by audio id 1234: …"] };
    mount();
    await waitFor(() => expect(screen.getByTestId("player-calls-pbp-none")).toBeTruthy());
    expect(screen.getByTestId("player-calls-announcer-none")).toBeTruthy();
    expect(screen.getByTestId("player-calls").parentElement!.textContent).toContain("no announcer table is keyed by audio id 1234");
  });

  it("a title outside coverage shows the reason", async () => {
    failWith = "PS4 titles use the SBle / pbp_sentencedata chain";
    mount();
    await waitFor(() => expect(screen.getByTestId("player-calls-unavailable").textContent).toContain("SBle"));
  });

  it("renders nothing and calls nothing without an install root", async () => {
    installDir = null;
    const r = mount();
    await new Promise((res) => setTimeout(res, 20));
    expect(r.container.textContent).toBe("");
    expect(calls).toHaveLength(0);
  });
});

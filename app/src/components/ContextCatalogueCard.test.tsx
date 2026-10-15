// @vitest-environment jsdom
//
// Phase 1.5 / 2.7 (21 Sep 2026) — the context listing card: rows render the trigger, the item
// pills ("speaks" / "raises") and the condition count; a row expands to its conditions and item
// parameters; the query is sent; a coverage refusal shows the reason; nothing without an install
// root. jsdom only.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
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

import { ContextCatalogueCard } from "./ContextCatalogueCard";

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <ContextCatalogueCard projectId={7} />
    </QueryClientProvider>,
  );
}

describe("ContextCatalogueCard", () => {
  beforeEach(() => {
    calls.length = 0;
    response = null;
    failWith = null;
    installDir = "X:/install";
  });
  afterEach(cleanup);

  it("lists contexts with trigger, items and conditions and expands a row", async () => {
    response = {
      game_id: "nhl15",
      platform: "xbox",
      context_dir: "X:/install/audio/context",
      records: 3866,
      triggers: 153,
      condition_names: 1980,
      speech_items: 4007,
      context_items: 515,
      query: "",
      matching: 2,
      truncated: false,
      contexts: [
        { index: 1, name: "Cup_End_Cup_Presentation_NHL", trigger: "trigger_CUP_ENDINGS", conditions: ["cup_type_Stanley_Cup", "+_Context_RUN_ONCE"], items: [{ kind: "speech", name: "CUP_END_CUP_NHL", event_id: 100, params: ["0"] }] },
        { index: 179, name: "Allow_Passives_Dynasty", trigger: null, conditions: [], items: [{ kind: "context", name: "trigger_DYNASTY_PASSIVES", event_id: 5, params: [] }] },
      ],
      notes: ["read-only: …"],
    };
    mount();
    await waitFor(() => expect(screen.getByTestId("context-list")).toBeTruthy());
    expect(calls[0]).toEqual({ cmd: "context_catalogue", args: { projectId: 7, installRoot: "X:/install", query: "", limit: 200 } });
    expect(screen.getByTestId("context-1").textContent).toContain("on trigger_CUP_ENDINGS");
    expect(screen.getByTestId("context-1").textContent).toContain("speaks CUP_END_CUP_NHL");
    expect(screen.getByTestId("context-1").textContent).toContain("2 conditions");
    expect(screen.getByTestId("context-179").textContent).toContain("no trigger");
    expect(screen.getByTestId("context-179").textContent).toContain("raises trigger_DYNASTY_PASSIVES");
    fireEvent.click(screen.getByTestId("context-1"));
    expect(screen.getByTestId("context-detail-1").textContent).toContain("cup_type_Stanley_Cup · +_Context_RUN_ONCE");
    expect(screen.getByTestId("context-detail-1").textContent).toContain("with 0");
    fireEvent.change(screen.getByTestId("context-query"), { target: { value: "cup" } });
    await waitFor(() => expect(calls.some((c) => c.args.query === "cup")).toBe(true));
  });

  it("shows a coverage refusal and renders nothing without an install root", async () => {
    failWith = "nhl13 PS3 ships its context tables only inside audioboot.big; the archive route is not built";
    mount();
    await waitFor(() => expect(screen.getByTestId("context-unavailable").textContent).toContain("archive route"));
    cleanup();
    installDir = null;
    const r = mount();
    await new Promise((res) => setTimeout(res, 20));
    expect(r.container.textContent).toBe("");
  });
});

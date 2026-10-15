// @vitest-environment jsdom
//
// Phase 1.5 (21 Sep 2026) — the read-only commentary catalogue card: the request carries the
// project, root, query and league toggle; an event without a sentence says "no sentence carries
// this event name" (never "silent"); a sentence expands to its phrases with the candidate-sample
// wording; unresolved bank ids are shown, not hidden; a refusal is shown as text. jsdom only.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
let failWith: string | null = null;
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    if (failWith) return Promise.reject(new Error(failWith));
    return Promise.resolve({
      game_id: "nhl15",
      platform: "xbox",
      speech_dir: "X:/NHL 15/audio/speech",
      sources: [{ role: "events", path: "X:/NHL 15/audio/speech/pbp_eng_speecheventsubscr.bin", len: 100, sha256: "ab".repeat(32) }],
      stats: { events: 3714, events_with_a_sentence: 1489, events_without_a_sentence: 2225, parameters: 108, sentences: 1489, sentences_slot_ok: 1489, bodies: 3337, phrases: 9829, phrases_with_table: 9829, graffiti_tables: 8368, bank_samples: 45442, keywords: 2121, ambiguous_event_ids: 189 },
      notes: ["read-only: sentences are listed as sharing an event's name"],
      query: args.query ?? "",
      league_only: args.leagueOnly ?? false,
      events_matching: 2,
      sentences_matching: 1,
      events: [
        { id: 29243, name: "CUP_END_CUP_NHL", parameters: ["team_ID", "league"], other_names_for_id: [], sentences: 1 },
        { id: 19835, name: "CUP_END_CUP_AHL", parameters: ["team_ID", "cup_endings_sequence_number"], other_names_for_id: ["OTHER_NAME"], sentences: 0 },
      ],
      sentences: [
        {
          name: "CUP_END_CUP_NHL",
          event_id: 29243,
          slot: 12,
          slot_ok: true,
          bodies: [
            { id: 7, phrases: [{ pool_id: 12, group_tag: 287, group_value: 41, group_name: null, parameters: ["team_ID"], table_found: true, candidate_count: 3, candidate_sample_ids: [10, 11, 12], bank_resolved: 2, unresolved_sample_ids: [12] }] },
          ],
        },
      ],
      truncated: false,
    });
  },
}));
vi.mock("../lib/project", () => ({
  useProject: () => ({ install: { install_dir: "X:/NHL 15" }, iso: null }),
}));

import { SpeechCatalogueCard } from "./SpeechCatalogueCard";

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <SpeechCatalogueCard projectId={7} />
    </QueryClientProvider>,
  );
}

describe("SpeechCatalogueCard", () => {
  beforeEach(() => {
    calls.length = 0;
    failWith = null;
  });
  afterEach(cleanup);

  it("requests the catalogue for the project and root with the league filter on, and words absence as the contract says", async () => {
    mount();
    await waitFor(() => expect(screen.getByTestId("speech-matching")).toBeTruthy());
    expect(calls[0]).toEqual({ cmd: "speech_catalogue", args: { projectId: 7, installRoot: "X:/NHL 15", query: "", leagueOnly: true, limit: 300 } });
    expect(screen.getByText(/3714 events, 1489 sentences/)).toBeTruthy();
    expect(screen.getByTestId("speech-event-CUP_END_CUP_AHL").textContent).toContain("no sentence carries this event name");
    expect(screen.getByTestId("speech-event-CUP_END_CUP_AHL").textContent).toContain("id shared with 1 other name");
    expect(screen.queryByText(/silent/)).toBeNull();
    // The sentence expands to its phrases with the candidate wording and the unresolved ids.
    fireEvent.click(screen.getByText("CUP_END_CUP_NHL", { selector: "button" }));
    expect(screen.getByText(/the event's team_ID select among the candidates/)).toBeTruthy();
    expect(screen.getByText(/3 candidate samples, 2 in the bank/)).toBeTruthy();
    expect(screen.getByText(/1 not in the bank: 12/)).toBeTruthy();
    expect(screen.getByText(/pool 12/)).toBeTruthy();
  });

  it("re-requests with the typed query and the league toggle off", async () => {
    mount();
    await waitFor(() => expect(calls.length).toBe(1));
    fireEvent.change(screen.getByTestId("speech-query"), { target: { value: "cup" } });
    await waitFor(() => expect(calls.length).toBe(2));
    expect(calls[1].args).toEqual({ projectId: 7, installRoot: "X:/NHL 15", query: "cup", leagueOnly: true, limit: 300 });
    fireEvent.click(screen.getByTestId("speech-league-only"));
    await waitFor(() => expect(calls.length).toBe(3));
    expect(calls[2].args.leagueOnly).toBe(false);
  });

  it("shows a refusal as text", async () => {
    failWith = "nhl12 PS3 ships its speech tables only inside audioboot.big";
    mount();
    await waitFor(() => expect(screen.getByTestId("speech-error").textContent).toContain("audioboot.big"));
  });
});

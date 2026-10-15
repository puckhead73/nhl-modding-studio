// @vitest-environment jsdom
//
// P4.3 (overnight 2026-09-24) — the gen7 diagnostic preview: nothing is requested until asked; the
// request names the exact (project, root, bank stem, sample id); the reply shows the fixed label,
// the diagnostics and the identities; a reply for a sample the user moved away from is never
// painted; a refusal is shown as text; selecting a sample id chip on the name-calls card mounts it.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
const handlers: Record<string, (args: any) => unknown> = {};
type Deferred<T> = { promise: Promise<T>; resolve: (v: T) => void };
function deferred<T>(): Deferred<T> {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    const h = handlers[cmd];
    if (!h) return Promise.reject(new Error(`unmocked ${cmd}`));
    return Promise.resolve().then(() => h(args));
  },
}));
vi.mock("../lib/project", () => ({
  useProject: () => ({ install: { install_dir: "X:/NHL 15/NHL 15" }, iso: null }),
}));

import { Gen7SpeechPreview } from "./Gen7SpeechPreview";
import { TeamNameCallsCard } from "./TeamNameCallsCard";
import { SpeechCatalogueCard } from "./SpeechCatalogueCard";
import { PlayerNameCallsCard } from "./PlayerNameCallsCard";

const preview = (sid: number) => ({
  game_id: "nhl15",
  platform: "xbox",
  bank_stem: "pbp_eng",
  bank_path: "X:/NHL 15/NHL 15/audio/speech/pbp_eng_samples.sbr",
  bank_len: 684592,
  bank_sha256: "5951d9474cb5b4d2825a693a4f1f29557f659cce4b293324e87682c64f2861c2",
  stream_path: "X:/NHL 15/NHL 15/audio/speech/pbp_eng_samples.sbs",
  stream_len: 731829012,
  sample_id: sid,
  offset: 16,
  sns_len: 12345,
  sns_sha256: "1d72faddc98b5e7cb8b986fad92abd351b3475d7722b30f4d5584ceb87d6332c",
  codec: "ealayer3-v1",
  sample_rate: 44100,
  channels: 1,
  num_samples: 80418,
  blocks: 9,
  granules: 144,
  granules_per_block: [18, 16, 16, 16, 16, 16, 16, 16, 14],
  rebuilt_frames: 72,
  max_main_data_begin: 511,
  pcm_frame_samples: 47,
  surplus_samples: 2573,
  decoded_samples_per_channel: 82944,
  first_sample_over_1_lsb: 1030,
  label: "diagnostic preview — start handling unresolved (two warm-up granules + 47 PCM samples; not game-verified)",
  route_detail: null as string | null,
  tuple_source: "R77",
  wav_data_url: "data:audio/wav;base64,UklGRg==",
  notes: ["Replacement is not supported"],
});

function mountPreview(sampleId: number | null) {
  const ui = (sid: number | null) => <Gen7SpeechPreview projectId={7} installRoot="X:/NHL 15/NHL 15" bankStem="pbp_eng" sampleId={sid} />;
  const r = render(ui(sampleId));
  return { ...r, reselect: (sid: number | null) => r.rerender(ui(sid)) };
}

beforeEach(() => {
  calls.length = 0;
  for (const k of Object.keys(handlers)) delete handlers[k];
  handlers.gen7_speech_preview = (a: any) => preview(a.sampleId);
});
afterEach(cleanup);

describe("Gen7SpeechPreview", () => {
  it("requests nothing until asked, names the exact selection, and shows the label, diagnostics and identities", async () => {
    mountPreview(1030);
    expect(calls).toHaveLength(0);
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-ready");
    expect(calls).toHaveLength(1);
    expect(calls[0].args).toEqual({ projectId: 7, installRoot: "X:/NHL 15/NHL 15", bankStem: "pbp_eng", sampleId: 1030 });
    expect(screen.getByTestId("gen7-preview-label").textContent).toMatch(/start handling unresolved.*not game-verified/);
    expect(screen.getByTestId("gen7-preview-diag").textContent).toMatch(/144 granules \(18\/16\/16\/16\/16\/16\/16\/16\/14\) → 72 frames, main_data_begin ≤ 511 · PCM frame 47 samples · decoded 82944 per channel \(surplus 2573\) · first > 1 LSB at 1030 · tuple R77/);
    expect(screen.getByTestId("gen7-preview-identity").textContent).toMatch(/pbp_eng_samples.sbr 684592 B sha256 5951d9474cb5b4d2… · pbp_eng_samples.sbs 731829012 B · sample @ 16 · SNS 12345 B sha256 1d72faddc98b5e7c…/);
    expect((screen.getByTestId("gen7-preview-audio") as HTMLAudioElement).getAttribute("src")).toBe("data:audio/wav;base64,UklGRg==");
  });

  it("shows the EA-Speex route's own diagnostic line instead of the granule fields (receipt 29)", async () => {
    handlers.gen7_speech_preview = (a: any) => ({
      ...preview(a.sampleId),
      bank_stem: "ann_eng",
      codec: "ea-speex",
      sample_rate: 32000,
      num_samples: 90795,
      blocks: 15,
      granules: 0,
      granules_per_block: [],
      rebuilt_frames: 0,
      max_main_data_begin: 0,
      pcm_frame_samples: 0,
      surplus_samples: 0,
      decoded_samples_per_channel: 90795,
      label: "diagnostic preview — not game-verified (EA-Speex through libspeex 1.2.1: the first 509 samples and the tail trim are dropped by the count rule, not a heard result)",
      route_detail: "143 Speex frames (11/10/10/10/10/10/10/10/10/10/10/10/10/10/2) × 640 = 91520 decoded; first 509 and tail 216 dropped → 90795 samples · libspeex speex-1.2.1 ultra-wideband, enhancer on",
      tuple_source: "R83",
    });
    mountPreview(16);
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-ready");
    expect(screen.getByTestId("gen7-preview-label").textContent).toMatch(/EA-Speex.*not game-verified|not game-verified.*EA-Speex/);
    const diag = screen.getByTestId("gen7-preview-diag").textContent ?? "";
    expect(diag).toMatch(/ea-speex · 32000 Hz · 1 ch · SNR 90795 samples · 15 blocks · 143 Speex frames .* first 509 and tail 216 dropped → 90795 samples · libspeex speex-1\.2\.1/);
    expect(diag).not.toMatch(/granules|main_data_begin/);
    expect(diag).toMatch(/tuple R83/);
  });

  it("drops a reply for a sample the user moved away from, and shows a refusal as text", async () => {
    const held = deferred<any>();
    handlers.gen7_speech_preview = (a: any) => (a.sampleId === 1030 ? held.promise : Promise.reject(new Error("nhl11 / xbox / pbp_eng: not a measured EALayer3 v1 tuple (research R77 / R80)")));
    const v = mountPreview(1030);
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    expect(screen.getByTestId("gen7-preview-request").textContent).toBe("Decoding…");
    v.reselect(2044);
    await waitFor(() => expect(screen.getByTestId("gen7-preview-request").textContent).toBe("Decode diagnostic preview"));
    await act(async () => {
      held.resolve(preview(1030));
      await held.promise;
    });
    expect(screen.queryByTestId("gen7-preview-ready")).toBeNull();
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-error");
    expect(screen.getByTestId("gen7-preview-error").textContent).toMatch(/not a measured EALayer3 v1 tuple/);
    v.reselect(null);
    expect(screen.queryByTestId("gen7-preview")).toBeNull();
  });

  it("cancels an in-flight EA-XMA decode on the server, never paints its late reply, and can decode again (receipt 33)", async () => {
    const held = deferred<any>();
    const xma = (sid: number) => ({
      ...preview(sid),
      bank_stem: "ann_eng",
      codec: "ea-xma",
      sample_rate: 24000,
      num_samples: 32057,
      blocks: 8,
      granules: 0,
      granules_per_block: [],
      decoded_samples_per_channel: 32057,
      label: "diagnostic preview — not game-verified (EA-XMA through the external XMA2 helper; the decoder's sample count is checked against the declared count, which is not a listening result)",
      route_detail: "8 XMA2 packets (64 frames) from 8 SNS blocks → RIFF XMA2 → nhlms-xma2-helper.exe (dc3533ed3234…) → 32057 samples per channel = SNR count",
      tuple_source: "R84",
    });
    let first = true;
    handlers.gen7_speech_preview = (a: any) => {
      if (first) {
        first = false;
        return held.promise;
      }
      return xma(a.sampleId);
    };
    handlers.gen7_speech_preview_cancel = () => true;
    mountPreview(2);
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    expect(screen.getByTestId("gen7-preview-request").textContent).toBe("Decoding…");
    fireEvent.click(screen.getByTestId("gen7-preview-cancel"));
    await screen.findByTestId("gen7-preview-cancelled");
    expect(calls.filter((c) => c.cmd === "gen7_speech_preview_cancel").map((c) => c.args)).toEqual([{ projectId: 7 }]);
    expect(screen.queryByTestId("gen7-preview-cancel")).toBeNull();
    await act(async () => {
      held.resolve(xma(2));
      await held.promise;
    });
    expect(screen.queryByTestId("gen7-preview-ready")).toBeNull();
    expect(screen.getByTestId("gen7-preview-cancelled")).toBeTruthy();
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-ready");
    expect(screen.getByTestId("gen7-preview-diag").textContent).toMatch(/ea-xma · 24000 Hz · 1 ch · SNR 32057 samples · 8 blocks · 8 XMA2 packets .* nhlms-xma2-helper\.exe .* = SNR count .*tuple R84/);
    expect(screen.queryByTestId("gen7-preview-cancelled")).toBeNull();
  });

  it("stops the server-side decode when the selection changes mid-decode", async () => {
    const held = deferred<any>();
    handlers.gen7_speech_preview = () => held.promise;
    handlers.gen7_speech_preview_cancel = () => true;
    const v = mountPreview(2);
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    v.reselect(3);
    await waitFor(() => expect(calls.filter((c) => c.cmd === "gen7_speech_preview_cancel")).toHaveLength(1));
    expect(screen.getByTestId("gen7-preview-request").textContent).toBe("Decode diagnostic preview");
  });

  it("replaces an earlier success with the refusal when the helper has gone, and offers no request for an unavailable row", async () => {
    let helperGone = false;
    handlers.gen7_speech_preview = (a: any) =>
      helperGone ? Promise.reject(new Error("sample 2: NHL_STUDIO_XMA2_HELPER names C:/x/nhlms-xma2-helper.exe, which is not a file")) : preview(a.sampleId);
    const v = mountPreview(2);
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-ready");
    helperGone = true;
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-error");
    expect(screen.queryByTestId("gen7-preview-ready")).toBeNull();
    expect(screen.queryByTestId("gen7-preview-audio")).toBeNull();
    expect(screen.getByTestId("gen7-preview-error").textContent).toMatch(/which is not a file/);
    cleanup();
    calls.length = 0;
    render(<Gen7SpeechPreview projectId={7} installRoot="X:/NHL 10/NHL 10" bankStem="ann_eng" sampleId={2} unavailable="no XMA2 decoder helper is installed" />);
    expect(screen.getByTestId("gen7-preview-unavailable").textContent).toMatch(/No preview for this sample: no XMA2 decoder helper is installed/);
    expect(screen.queryByTestId("gen7-preview-request")).toBeNull();
    expect(calls).toHaveLength(0);
    void v;
  });

  it("mounts from a selected sample id chip on the name-calls card", async () => {
    handlers.team_name_calls = () => ({
      team_id: 3,
      team_name: "Boston Bruins",
      team_audio_id: 114,
      team_tag: 1,
      tables: [{ num_tags: 1, sample_ids: [1030, 2044], sample_count: 2, bank_resolved: 2 }],
      sample_ids_distinct: 2,
      bank_resolved_distinct: 2,
      keyword_mentions: 5,
      notes: [],
    });
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <TeamNameCallsCard projectId={7} teamId={3} />
      </QueryClientProvider>,
    );
    await screen.findByTestId("name-calls-sample-2044");
    expect(screen.queryByTestId("gen7-preview")).toBeNull();
    fireEvent.click(screen.getByTestId("name-calls-sample-2044"));
    await screen.findByTestId("gen7-preview");
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-ready");
    const req = calls.filter((c) => c.cmd === "gen7_speech_preview");
    expect(req).toHaveLength(1);
    expect(req[0].args).toEqual({ projectId: 7, installRoot: "X:/NHL 15/NHL 15", bankStem: "pbp_eng", sampleId: 2044 });
  });
  it("mounts from a phrase's candidate sample id chip on the commentary catalogue card", async () => {
    handlers.speech_catalogue = () => ({
      game_id: "nhl15",
      platform: "xbox",
      speech_dir: "X:/NHL 15/NHL 15/audio/speech",
      sources: [],
      stats: { events: 1, events_with_a_sentence: 1, events_without_a_sentence: 0, parameters: 1, sentences: 1, sentences_slot_ok: 1, bodies: 1, phrases: 1, phrases_with_table: 1, candidate_samples: 3, bank_resolved: 3 },
      notes: [],
      query: "",
      league_only: true,
      events_matching: 1,
      sentences_matching: 1,
      events: [{ id: 29243, name: "CUP_END_CUP_NHL", parameters: ["team_ID"], other_names_for_id: [], sentences: 1 }],
      sentences: [
        {
          name: "CUP_END_CUP_NHL",
          event_id: 29243,
          slot: 12,
          slot_ok: true,
          bodies: [{ id: 7, phrases: [{ pool_id: 12, group_tag: 287, group_value: 41, group_name: null, parameters: ["team_ID"], table_found: true, candidate_count: 3, candidate_sample_ids: [10, 11, 1030], bank_resolved: 3, unresolved_sample_ids: [] }] }],
        },
      ],
      truncated: false,
    });
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <SpeechCatalogueCard projectId={7} />
      </QueryClientProvider>,
    );
    await screen.findByTestId("speech-sentence-CUP_END_CUP_NHL");
    fireEvent.click(screen.getByText("CUP_END_CUP_NHL", { selector: "button" }));
    await screen.findByTestId("speech-sample-1030");
    expect(screen.queryByTestId("gen7-preview")).toBeNull();
    fireEvent.click(screen.getByTestId("speech-sample-1030"));
    await screen.findByTestId("gen7-preview");
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-ready");
    const req = calls.filter((c) => c.cmd === "gen7_speech_preview");
    expect(req).toHaveLength(1);
    expect(req[0].args).toEqual({ projectId: 7, installRoot: "X:/NHL 15/NHL 15", bankStem: "pbp_eng", sampleId: 1030 });
  });
  it("mounts from the player card with the announcer bank for an announcer chip and the play-by-play bank for a name chip", async () => {
    handlers.player_name_calls = () => ({
      player_id: 5,
      player_name: "Patrice Bergeron",
      bio_audio_id: 77,
      pbp_names: [{ name: "Bergeron", is_first_name: false, bank: 1, audio_id: 501, tables: [{ num_tags: 1, sample_ids: [3001], sample_count: 1, bank_resolved: 1 }], sample_ids_distinct: 1, bank_resolved_distinct: 1 }],
      pbp_sample_ids_distinct: 1,
      pbp_bank_resolved_distinct: 1,
      announcer_audio_id: 77,
      announcer_tables: [{ num_tags: 1, sample_ids: [45], sample_count: 1, bank_resolved: 1 }],
      announcer_sample_ids_distinct: 1,
      announcer_bank_resolved_distinct: 1,
      keyword_mentions: 0,
      notes: [],
    });
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <PlayerNameCallsCard projectId={7} playerId={5} />
      </QueryClientProvider>,
    );
    await screen.findByTestId("ann-sample-45");
    fireEvent.click(screen.getByTestId("ann-sample-45"));
    await screen.findByTestId("gen7-preview");
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-ready");
    let req = calls.filter((c) => c.cmd === "gen7_speech_preview");
    expect(req[0].args).toEqual({ projectId: 7, installRoot: "X:/NHL 15/NHL 15", bankStem: "ann_eng", sampleId: 45 });
    fireEvent.click(screen.getByTestId("pbp-last-sample-3001"));
    await waitFor(() => expect(screen.queryByTestId("gen7-preview-ready")).toBeNull());
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-ready");
    req = calls.filter((c) => c.cmd === "gen7_speech_preview");
    expect(req[1].args).toEqual({ projectId: 7, installRoot: "X:/NHL 15/NHL 15", bankStem: "pbp_eng", sampleId: 3001 });
  });
});

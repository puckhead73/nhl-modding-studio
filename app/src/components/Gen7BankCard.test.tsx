// @vitest-environment jsdom
//
// P4.6 (overnight 2026-09-24) — the gen7 bank card: the request names project, root, bank stem and
// query; rows show the header-measured codec with a previewable pill; selecting a row mounts the
// diagnostic preview with the same stem; switching the stem re-requests and drops the selection; a
// refusal (tuple outside the measured set) shows as text; nothing without an install root.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
const handlers: Record<string, (args: any) => unknown> = {};
let installDir: string | null = "X:/NHL 15/NHL 15";
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    const h = handlers[cmd];
    if (!h) return Promise.reject(new Error(`unmocked ${cmd}`));
    return Promise.resolve().then(() => h(args));
  },
}));
vi.mock("../lib/project", () => ({
  useProject: () => ({ install: installDir ? { install_dir: installDir } : null, iso: null }),
}));

import { Gen7BankCard } from "./Gen7BankCard";

const listing = (stem: string) => ({
  game_id: "nhl15",
  platform: "xbox",
  bank_stem: stem,
  tuple_source: stem === "music_samples" ? "R80" : "R77",
  bank: `X:/NHL 15/NHL 15/audio/${stem === "music_samples" ? "music/music_samples" : `speech/${stem}_samples`}.sbr (684592 B, loose)`,
  bank_len: 684592,
  bank_sha256: "5951d9474cb5b4d2825a693a4f1f29557f659cce4b293324e87682c64f2861c2",
  stream: "…sbs (731829012 B, loose)",
  total: 263,
  matching: 263,
  truncated: true,
  codec_census: [{ codec: "ealayer3-v1", count: 2 }],
  preview_unavailable: null as string | null,
  rows: [
    { sample_id: 634, offset: 16, codec: "ealayer3-v1", codec_code: 5, sample_rate: 44100, channels: stem === "music_samples" ? 2 : 1, num_samples: 4558848, problem: null, previewable: true },
    { sample_id: 635, offset: 2071168, codec: "ealayer3-v1", codec_code: 5, sample_rate: 44100, channels: 2, num_samples: 100, problem: null, previewable: stem === "music_samples" },
  ],
  notes: ["Listing and diagnostic preview only."],
});
const preview = (stem: string, sid: number) => ({
  game_id: "nhl15", platform: "xbox", bank_stem: stem, bank_path: "b", bank_len: 1, bank_sha256: "00", stream_path: "s", stream_len: 1, sample_id: sid, offset: 16, sns_len: 1, sns_sha256: "00",
  codec: "ealayer3-v1", sample_rate: 44100, channels: 2, num_samples: 100, blocks: 1, granules: 2, granules_per_block: [2], rebuilt_frames: 1, max_main_data_begin: 0, pcm_frame_samples: 47, surplus_samples: 1199,
  decoded_samples_per_channel: 1152, first_sample_over_1_lsb: null, label: "diagnostic preview — start handling unresolved (two warm-up granules + 47 PCM samples; not game-verified)", route_detail: null, tuple_source: "R80",
  wav_data_url: "data:audio/wav;base64,UklGRg==", notes: [],
});

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <Gen7BankCard projectId={7} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  calls.length = 0;
  installDir = "X:/NHL 15/NHL 15";
  for (const k of Object.keys(handlers)) delete handlers[k];
  handlers.gen7_bank_samples = (a: any) => listing(a.bankStem);
  handlers.gen7_speech_preview = (a: any) => preview(a.bankStem, a.sampleId);
});
afterEach(cleanup);

describe("Gen7BankCard", () => {
  it("lists the bank with the header census, previews a selected row with the same stem, and re-requests on a stem switch", async () => {
    mount();
    await screen.findByTestId("gen7-bank-census");
    expect(calls[0]).toEqual({ cmd: "gen7_bank_samples", args: { projectId: 7, installRoot: "X:/NHL 15/NHL 15", bankStem: "pbp_eng", query: "", limit: 500 } });
    expect(screen.getByTestId("gen7-bank-census").textContent).toMatch(/263 samples · ealayer3-v1 2 · first 2 of 263 shown · tuple R77/);
    expect(screen.queryByTestId("gen7-preview")).toBeNull();
    fireEvent.click(screen.getByTestId("gen7-bank-row-634"));
    await screen.findByTestId("gen7-preview");
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-ready");
    expect(calls.filter((c) => c.cmd === "gen7_speech_preview")[0].args).toEqual({ projectId: 7, installRoot: "X:/NHL 15/NHL 15", bankStem: "pbp_eng", sampleId: 634 });
    // Switch to the music bank: a new listing request, the selection dropped, no stale preview.
    fireEvent.change(screen.getByTestId("gen7-bank-stem"), { target: { value: "music_samples" } });
    await waitFor(() => expect(calls.filter((c) => c.cmd === "gen7_bank_samples").map((c) => c.args.bankStem)).toEqual(["pbp_eng", "music_samples"]));
    await waitFor(() => expect(screen.queryByTestId("gen7-preview")).toBeNull());
    await screen.findByTestId("gen7-bank-row-635");
    fireEvent.click(screen.getByTestId("gen7-bank-row-635"));
    await screen.findByTestId("gen7-preview");
    fireEvent.click(screen.getByTestId("gen7-preview-request"));
    await screen.findByTestId("gen7-preview-ready");
    expect(calls.filter((c) => c.cmd === "gen7_speech_preview")[1].args).toEqual({ projectId: 7, installRoot: "X:/NHL 15/NHL 15", bankStem: "music_samples", sampleId: 635 });
  });

  it("says when EA-XMA rows cannot preview for lack of a helper, and a selected row shows why instead of a request (receipt 33)", async () => {
    handlers.gen7_bank_samples = (a: any) => ({
      ...listing(a.bankStem),
      game_id: "nhl10",
      tuple_source: "R84",
      codec_census: [{ codec: "ea-xma", count: 1 }],
      preview_unavailable: "no XMA2 decoder helper is installed (Studio looks for helpers\\nhlms-xma2-helper.exe beside its executable, or the file named by NHL_STUDIO_XMA2_HELPER); EA-XMA entries stay listed and framed without a preview",
      rows: [{ sample_id: 2, offset: 16, codec: "ea-xma", codec_code: 3, sample_rate: 24000, channels: 1, num_samples: 32057, problem: null, previewable: false }],
    });
    mount();
    await screen.findByTestId("gen7-bank-preview-unavailable");
    expect(screen.getByTestId("gen7-bank-preview-unavailable").textContent).toMatch(/Listed and framed only — no XMA2 decoder helper is installed/);
    fireEvent.click(screen.getByTestId("gen7-bank-row-2"));
    await screen.findByTestId("gen7-preview-unavailable");
    expect(screen.getByTestId("gen7-preview-unavailable").textContent).toMatch(/no XMA2 decoder helper is installed/);
    expect(screen.queryByTestId("gen7-preview-request")).toBeNull();
    expect(calls.filter((c) => c.cmd === "gen7_speech_preview")).toHaveLength(0);
  });

  it("names the codec of a selected row that is outside the tuple's class", async () => {
    mount();
    await screen.findByTestId("gen7-bank-row-635");
    fireEvent.click(screen.getByTestId("gen7-bank-row-635"));
    await screen.findByTestId("gen7-preview-unavailable");
    expect(screen.getByTestId("gen7-preview-unavailable").textContent).toMatch(/ealayer3-v1 samples have no preview route on this measured tuple \(R77\)/);
  });

  it("shows the tuple refusal as text and renders nothing without an install root", async () => {
    handlers.gen7_bank_samples = () => Promise.reject(new Error("nhl11 / xbox / pbp_eng: not a measured EALayer3 v1 tuple (research R77 / R80)"));
    mount();
    await screen.findByTestId("gen7-bank-unavailable");
    expect(screen.getByTestId("gen7-bank-unavailable").textContent).toMatch(/not a measured EALayer3 v1 tuple/);
    cleanup();
    installDir = null;
    calls.length = 0;
    const { container } = mount();
    expect(container.textContent).toBe("");
    expect(calls).toHaveLength(0);
  });
});

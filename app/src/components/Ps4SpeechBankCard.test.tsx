// @vitest-environment jsdom
//
// P4 (overnight 2026-09-24) — the PS4 speech bank card: banks listed with their sample counts, the
// first usable bank's samples fetched with the codec census, a preview requested explicitly with
// the exact (bank, sample, rendering) and shown with its diagnostic label and identity; a reply for
// a selection the user moved away from is never painted; a coverage refusal shows the reason;
// nothing without an install root. jsdom only (no audio is played here).
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
const handlers: Record<string, (args: any) => unknown> = {};
type Deferred<T> = { promise: Promise<T>; resolve: (v: T) => void; reject: (e: unknown) => void };
function deferred<T>(): Deferred<T> {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}
let installDir: string | null = "H:/extracted";
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

import { Ps4SpeechBankCard } from "./Ps4SpeechBankCard";

const BANK = { file: "pbp_samples.sbr", sbr_len: 864352, sbr_sha256: "abcdef0123456789abcdef0123456789", sbs_present: true, sbs_len: 392937488, dsets: 3188, samples: 3863, note: null };
const EMPTY = { file: "empty.sbr", sbr_len: 100, sbr_sha256: "00", sbs_present: false, sbs_len: null, dsets: 0, samples: 0, note: "not read: bad magic" };
const banks = () => ({ game_id: "nhl17", speech_dir: "H:/extracted/nhl/data/kettle/audio/speech/eng", banks: [EMPTY, BANK], notes: ["Listing and diagnostic preview only."] });
const samples = () => ({
  bank: BANK,
  total: 3863,
  matching: 3863,
  truncated: true,
  codec_census: [
    { codec: "ea-mp3", count: 2 },
    { codec: "ea-opus", count: 1 },
  ],
  rows: [
    { sid: 15360, offset: 100, duration_ms: 1600, codec: "ea-mp3", codec_code: 11, sample_rate: 48000, channels: 1, num_samples: 77428, problem: null, previewable: true },
    { sid: 15935, offset: 200, duration_ms: 900, codec: "ea-opus", codec_code: 12, sample_rate: 48000, channels: 1, num_samples: 40000, problem: null, previewable: false },
  ],
  notes: [],
});
const preview = (sid: number, rendering: string) => ({
  bank: BANK,
  sid,
  offset: 100,
  sns_len: 20000,
  sns_sha256: "50854ef9d1b2fe1662cb2696c8e3cad3a93bd5a734c39c8b98ddc6064b2af72d",
  codec: "ea-mp3",
  sample_rate: 48000,
  channels: 1,
  num_samples: 77428,
  blocks: 8,
  frames: 68,
  frames_per_block: [9, 9, 9, 9, 9, 9, 9, 5],
  trim: 908,
  embedded_pcm_samples: 1152,
  first_sample_over_1_lsb: 267,
  rendering,
  label: rendering === "candidate_a" ? "diagnostic preview (start rendered by candidate A; not game-verified)" : "diagnostic preview (MPEG frames only; not game-verified)",
  wav_data_url: "data:audio/wav;base64,UklGRg==",
  notes: ["Replacement is not supported."],
});

function mount(projectId = 7) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const ui = (id: number) => (
    <QueryClientProvider client={qc}>
      <Ps4SpeechBankCard projectId={id} />
    </QueryClientProvider>
  );
  const r = render(ui(projectId));
  return { ...r, reproject: (id: number) => r.rerender(ui(id)) };
}

beforeEach(() => {
  calls.length = 0;
  installDir = "H:/extracted";
  for (const k of Object.keys(handlers)) delete handlers[k];
  handlers.ps4_speech_banks = banks;
  handlers.ps4_speech_samples = samples;
  handlers.ps4_speech_preview = (a: any) => preview(a.sid, a.rendering);
});
afterEach(cleanup);

describe("Ps4SpeechBankCard", () => {
  it("lists banks, fetches the first usable bank's samples with the census, and previews the selected sample with its label and identity", async () => {
    mount();
    await screen.findByTestId("ps4-speech-census");
    expect(calls.find((c) => c.cmd === "ps4_speech_banks")!.args).toEqual({ projectId: 7, installRoot: "H:/extracted" });
    expect(calls.find((c) => c.cmd === "ps4_speech_samples")!.args).toEqual({ projectId: 7, installRoot: "H:/extracted", bankFile: "pbp_samples.sbr", query: "", limit: 500 });
    expect(screen.getByTestId("ps4-speech-census").textContent).toMatch(/3863 samples · ea-mp3 2 · ea-opus 1 · first 2 of 3863 shown/);
    expect((screen.getByTestId("ps4-speech-bank") as HTMLSelectElement).value).toBe("pbp_samples.sbr");
    // Nothing is decoded until asked.
    fireEvent.click(screen.getByTestId("ps4-speech-row-15360"));
    expect(calls.filter((c) => c.cmd === "ps4_speech_preview")).toHaveLength(0);
    fireEvent.click(screen.getByTestId("ps4-speech-preview"));
    await screen.findByTestId("ps4-speech-preview-ready");
    const req = calls.filter((c) => c.cmd === "ps4_speech_preview");
    expect(req).toHaveLength(1);
    expect(req[0].args).toEqual({ projectId: 7, installRoot: "H:/extracted", bankFile: "pbp_samples.sbr", sid: 15360, rendering: "candidate_a" });
    expect(screen.getByTestId("ps4-speech-preview-label").textContent).toMatch(/candidate A; not game-verified/);
    expect(screen.getByTestId("ps4-speech-preview-diag").textContent).toMatch(/68 frames \(9\/9\/9\/9\/9\/9\/9\/5\) · tail trim 908 · embedded 1152 · first > 1 LSB at 267/);
    expect(screen.getByTestId("ps4-speech-preview-identity").textContent).toMatch(/pbp_samples.sbr sha256 abcdef0123456789… · sample 15360 @ 100 · SNS 20000 B sha256 50854ef9d1b2fe16…/);
    expect((screen.getByTestId("ps4-speech-audio") as HTMLAudioElement).getAttribute("src")).toBe("data:audio/wav;base64,UklGRg==");
    // Switching the rendering retires the shown preview; a new request names the new rendering.
    fireEvent.change(screen.getByTestId("ps4-speech-rendering"), { target: { value: "mpeg_only" } });
    await waitFor(() => expect(screen.queryByTestId("ps4-speech-preview-ready")).toBeNull());
    fireEvent.click(screen.getByTestId("ps4-speech-preview"));
    await screen.findByTestId("ps4-speech-preview-ready");
    expect(calls.filter((c) => c.cmd === "ps4_speech_preview")[1].args.rendering).toBe("mpeg_only");
    expect(screen.getByTestId("ps4-speech-preview-label").textContent).toMatch(/MPEG frames only/);
  });

  it("never paints a preview for a sample the user moved away from, and shows a refusal for a non-EA-MP3 sample", async () => {
    const held = deferred<any>();
    handlers.ps4_speech_preview = (a: any) => (a.sid === 15360 ? held.promise : Promise.reject(new Error("pbp_samples.sbr sample 15935: SNR header: codec 0x0c (ea-opus) … no preview route exists for this codec")));
    mount();
    await screen.findByTestId("ps4-speech-census");
    fireEvent.click(screen.getByTestId("ps4-speech-row-15360"));
    fireEvent.click(screen.getByTestId("ps4-speech-preview"));
    expect(screen.getByTestId("ps4-speech-preview").textContent).toBe("Decoding…");
    // Move to the Opus sample before the first reply lands.
    fireEvent.click(screen.getByTestId("ps4-speech-row-15935"));
    await waitFor(() => expect(screen.getByTestId("ps4-speech-preview").textContent).toBe("Decode diagnostic preview"));
    await act(async () => {
      held.resolve(preview(15360, "candidate_a"));
      await held.promise;
    });
    expect(screen.queryByTestId("ps4-speech-preview-ready")).toBeNull();
    fireEvent.click(screen.getByTestId("ps4-speech-preview"));
    await screen.findByTestId("ps4-speech-preview-error");
    expect(screen.getByTestId("ps4-speech-preview-error").textContent).toMatch(/ea-opus/);
    expect(screen.queryByTestId("ps4-speech-audio")).toBeNull();
  });

  it("shows the coverage refusal, and renders nothing without an install root", async () => {
    handlers.ps4_speech_banks = () => Promise.reject(new Error("nhl15 on Xbox: the SND10 bank listing covers PS4 NHL 15–21 only"));
    mount();
    await screen.findByTestId("ps4-speech-unavailable");
    expect(screen.getByTestId("ps4-speech-unavailable").textContent).toMatch(/PS4 NHL 15–21 only/);
    cleanup();
    installDir = null;
    calls.length = 0;
    const { container } = mount();
    expect(container.textContent).toBe("");
    expect(calls).toHaveLength(0);
  });
});

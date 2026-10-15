// @vitest-environment jsdom
//
// 25 Sep 2026 — the gen7 sound-bank card: the bank list names project and root; choosing a bank
// requests its picker tree; pickers show their owner names and each entry's codec / kind /
// framing; an entry without a decode route says "no preview" and its refusal sits in the tooltip;
// selecting a previewable entry mounts the diagnostic preview, which requests only on the button
// and shows the fixed label; a coverage refusal shows as text; nothing without an install root.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
const handlers: Record<string, (args: any) => unknown> = {};
let installDir: string | null = "X:/NHL 15/PS3_GAME/USRDIR";
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

import { Gen7SfxBankCard } from "./Gen7SfxBankCard";

const bankList = {
  game_id: "nhl15",
  platform: "ps3",
  population: "R89 / R91 (loose PS3 NHL 11 / 14 / 15 / Legacy)",
  dir: "X:/NHL 15/PS3_GAME/USRDIR/audio/sfx",
  banks: [
    { name: "000_goal.abk", path: "…/000_goal.abk", len: 316544, platform: "PS3", records: 1, stream_records: 0, ram_records: 1, lists: 1, names: 25, ast: null, ast_len: null, ast_id_matches: null, problem: null },
    { name: "crowds.abk", path: "…/crowds.abk", len: 5606300, platform: "PS3", records: 204, stream_records: 86, ram_records: 118, lists: 44, names: 311, ast: "…/crowds.ast", ast_len: 30080896, ast_id_matches: true, problem: null },
  ],
  notes: ["Listing only."],
};
const tree = {
  game_id: "nhl15",
  platform: "ps3",
  bank: "crowds.abk",
  bank_path: "…/crowds.abk",
  bank_len: 5606300,
  bank_sha256: "4382317d3de2c754cf477274ea3ade7aef5b9fe40746e8c6ec28247dfe7a4594",
  bank_wrapper: "loose",
  bank_stored_sha256: "4382317d3de2c754cf477274ea3ade7aef5b9fe40746e8c6ec28247dfe7a4594",
  bank_copies: 1,
  bank_platform: "PS3",
  ast_path: "X:/NHL 15/PS3_GAME/USRDIR/audio/sfx/crowds.ast",
  ast_len: 30080896,
  ast_id: 0x460ed680,
  ast_id_matches: true,
  records: 204,
  stream_records: 86,
  ram_records: 118,
  lists: 44,
  names: 311,
  handler_nodes: 81,
  pickers: [
    {
      owners: ["hiAmbColorSamplePicker02_msg"],
      owner_class: "message (_msg)",
      list_offset: 0x49d64,
      empty_slots: 0,
      entries: [
        { record: 21, weight: 100, kind: "stream", codec: "ealayer3-v1", codec_code: 5, sample_rate: 44100, channel_config: 0, sub_streams: 1, num_samples: 129775, stream_offset: 4096, blocks: 113, stream_bytes: 66048, framing: "framed; block sample words sum to the record's count", x: 0, s: 4, previewable: true, preview_refusal: null },
      ],
    },
    {
      owners: ["base_value_reset_send"],
      owner_class: "sound (_snd)",
      list_offset: 0x49c60,
      empty_slots: 0,
      entries: [
        { record: 0, weight: 100, kind: "stream-loop", codec: "ealayer3-v1", codec_code: 5, sample_rate: 44100, channel_config: 3, sub_streams: 2, num_samples: 689988, stream_offset: 64, blocks: 1, stream_bytes: 1472, framing: "loop stub (29 samples, 1472 B) + body framed", x: 0xdf1f, s: 8, previewable: true, preview_refusal: null },
        { record: 7, weight: 100, kind: "stream", codec: "ealayer3-v1", codec_code: 5, sample_rate: 44100, channel_config: 7, sub_streams: null, num_samples: 1000, stream_offset: 8192, blocks: 1, stream_bytes: 512, framing: "framed; block sample words sum to the record's count", x: 0, s: 0, previewable: false, preview_refusal: "channel config 7 is not a measured value (0 / 1 / 3 / 5)" },
      ],
    },
  ],
  notes: ["Pickers are the named nodes…"],
};
const preview = {
  game_id: "nhl15", platform: "ps3", bank: "crowds.abk", bank_sha256: "4382317d", record: 21, kind: "stream", owners: ["hiAmbColorSamplePicker02_msg"],
  source: "…/crowds.ast stream at 4096 (113 blocks, 66048 B)", source_sha256: "00", codec: "ealayer3-v1", sample_rate: 44100, channels: 1, num_samples: 129775, blocks: 113, granules: 227, rebuilt_frames: 114, decoded_frames_per_channel: 130752, sub_stream: null, sub_streams: 1, loop_start: null,
  label: "diagnostic preview — R92 candidate construction (PCM frame + decoder output from sample 1 152, trimmed to the declared count); not game-verified",
  wav_data_url: "data:audio/wav;base64,UklGRg==", notes: ["Diagnostic only."],
};

function mount(url = "/") {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <MemoryRouter initialEntries={[url]}>
      <QueryClientProvider client={qc}>
        <Gen7SfxBankCard projectId={7} />
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  calls.length = 0;
  installDir = "X:/NHL 15/PS3_GAME/USRDIR";
  handlers.gen7_sfx_banks = () => bankList;
  handlers.gen7_sfx_bank_tree = () => tree;
  handlers.gen7_sfx_preview = () => preview;
});
afterEach(cleanup);

describe("Gen7SfxBankCard", () => {
  it("lists the banks, then the chosen bank's pickers with owners, framing and the no-preview marker", async () => {
    mount();
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-bank")).toBeTruthy());
    expect(calls[0]).toEqual({ cmd: "gen7_sfx_banks", args: { projectId: 7, installRoot: "X:/NHL 15/PS3_GAME/USRDIR" } });
    expect(screen.getByTestId("gen7-sfx-census").textContent).toContain("205 records");
    fireEvent.change(screen.getByTestId("gen7-sfx-bank"), { target: { value: "crowds.abk" } });
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-pickers")).toBeTruthy());
    expect(calls.find((c) => c.cmd === "gen7_sfx_bank_tree")?.args).toEqual({ projectId: 7, installRoot: "X:/NHL 15/PS3_GAME/USRDIR", bank: "crowds.abk" });
    expect(screen.getByText("hiAmbColorSamplePicker02_msg")).toBeTruthy();
    expect(screen.getByText("base_value_reset_send")).toBeTruthy();
    expect(screen.getByTestId("gen7-sfx-tree-facts").textContent).toContain("id matches");
    const odd = screen.getByTestId("gen7-sfx-entry-7");
    expect(odd.getAttribute("title")).toContain("not a measured value");
    expect(odd.getAttribute("title")).toContain("R94");
    expect(screen.getByTestId("gen7-sfx-no-preview-7")).toBeTruthy();
    expect(screen.queryByTestId("gen7-sfx-no-preview-0")).toBeNull();
    expect(screen.queryByTestId("gen7-sfx-no-preview-21")).toBeNull();
    expect(calls.some((c) => c.cmd === "gen7_sfx_preview")).toBe(false);
  });

  it("requests the diagnostic preview only on the button and shows the fixed label", async () => {
    mount();
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-bank")).toBeTruthy());
    fireEvent.change(screen.getByTestId("gen7-sfx-bank"), { target: { value: "crowds.abk" } });
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-entry-21")).toBeTruthy());
    fireEvent.click(screen.getByTestId("gen7-sfx-entry-21"));
    expect(screen.getByTestId("gen7-sfx-preview")).toBeTruthy();
    expect(calls.some((c) => c.cmd === "gen7_sfx_preview")).toBe(false);
    fireEvent.click(screen.getByTestId("gen7-sfx-preview-request"));
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-preview-audio")).toBeTruthy());
    expect(calls.find((c) => c.cmd === "gen7_sfx_preview")?.args).toEqual({ projectId: 7, installRoot: "X:/NHL 15/PS3_GAME/USRDIR", bank: "crowds.abk", record: 21, subStream: null });
    expect(screen.queryByTestId("gen7-sfx-pair")).toBeNull();
    expect(screen.getByTestId("gen7-sfx-preview-label").textContent).toContain("not game-verified");
  });

  it("decodes a 4-channel loop one stereo pair at a time and shows the loop point", async () => {
    handlers.gen7_sfx_preview = (args: any) => ({ ...preview, record: 0, kind: "stream-loop", sub_stream: args.subStream, sub_streams: 2, loop_start: 29 });
    mount();
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-bank")).toBeTruthy());
    fireEvent.change(screen.getByTestId("gen7-sfx-bank"), { target: { value: "crowds.abk" } });
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-entry-0")).toBeTruthy());
    fireEvent.click(screen.getByTestId("gen7-sfx-entry-0"));
    fireEvent.change(screen.getByTestId("gen7-sfx-pair"), { target: { value: "1" } });
    fireEvent.click(screen.getByTestId("gen7-sfx-preview-request"));
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-preview-audio")).toBeTruthy());
    const sent = calls.filter((c) => c.cmd === "gen7_sfx_preview");
    expect(sent[sent.length - 1]?.args.subStream).toBe(1);
    expect(screen.getByTestId("gen7-sfx-preview").textContent).toContain("stereo pair 2 of 2");
    expect(screen.getByTestId("gen7-sfx-preview").textContent).toContain("loop point at sample 29");
  });

  it("shows a refusal from the preview route in words", async () => {
    handlers.gen7_sfx_preview = () => {
      throw new Error("crowds.abk record 21: framing refused: stream at 4096: block sample words sum to 5, the record says 129775");
    };
    mount();
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-bank")).toBeTruthy());
    fireEvent.change(screen.getByTestId("gen7-sfx-bank"), { target: { value: "crowds.abk" } });
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-entry-21")).toBeTruthy());
    fireEvent.click(screen.getByTestId("gen7-sfx-entry-21"));
    fireEvent.click(screen.getByTestId("gen7-sfx-preview-request"));
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-preview-refusal").textContent).toContain("framing refused"));
  });

  it("opens the bank named in the URL (the arena page's horn link) without a manual pick", async () => {
    mount("/leagues/commentary?bank=crowds.abk");
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-pickers")).toBeTruthy());
    expect(calls.find((c) => c.cmd === "gen7_sfx_bank_tree")?.args.bank).toBe("crowds.abk");
    expect((screen.getByTestId("gen7-sfx-bank") as HTMLSelectElement).value).toBe("crowds.abk");
  });

  it("names an archive-backed bank's member and stored form", async () => {
    handlers.gen7_sfx_bank_tree = () => ({ ...tree, bank_path: "nocache.big:audio/sfx/crowds.abk", bank_wrapper: "chunkzip", bank_stored_sha256: "ab12" });
    mount("/leagues/commentary?bank=crowds.abk");
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-member")).toBeTruthy());
    expect(screen.getByTestId("gen7-sfx-member").textContent).toContain("archive member (chunkzip)");
    expect(screen.getByTestId("gen7-sfx-member").getAttribute("title")).toContain("nocache.big:audio/sfx/crowds.abk");
  });

  it("shows the coverage refusal as text and nothing without an install root", async () => {
    handlers.gen7_sfx_banks = () => {
      throw new Error("nhl09 Xbox 360 keeps its banks in the audio archives with an XMA packet-header variant (R98b); nothing is listed rather than guessed");
    };
    mount();
    await waitFor(() => expect(screen.getByTestId("gen7-sfx-unavailable").textContent).toContain("audio archives"));
    cleanup();
    installDir = null;
    const { container } = mount();
    expect(container.textContent).toBe("");
  });
});

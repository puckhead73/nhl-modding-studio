// P4.3 (overnight 2026-09-24) — the **diagnostic** gen7 EALayer3 v1 preview of one speech sample
// (research R77 / R79 / R80): requested explicitly for a selected sample id, decoded through the
// measured-tuple route, shown with its fixed label ("start handling unresolved … not game-verified"),
// its grammar / rebuild diagnostics and the bank + stream + sample identities. One owned <audio>
// element; a change of sample, bank, root or project pauses and releases it and drops any in-flight
// reply, so nothing plays for a sample that is no longer selected. Receipt 33: EA-XMA previews run
// an external helper, so an in-flight decode can be cancelled (and is, on a change of selection or
// unmount) — the server stops the helper; a row the listing marked unavailable shows its reason
// instead of a request button.
import { useEffect, useRef, useState } from "react";
import { call } from "../lib/client";
import { Button } from "./primitives";
import type { Gen7SpeechPreviewDto } from "../bindings/Gen7SpeechPreviewDto";

/** A loose file shows as its file name; an archive member keeps the whole source description. */
function srcLabel(p: string): string {
  return p.includes(" (member at ") ? p : (p.split(/[\\/]/).pop() ?? p);
}

type State =
  | { key: string; status: "loading" }
  | { key: string; status: "ready"; data: Gen7SpeechPreviewDto }
  | { key: string; status: "error"; message: string }
  | { key: string; status: "cancelled" };

export function Gen7SpeechPreview({
  projectId,
  installRoot,
  bankStem,
  sampleId,
  unavailable = null,
}: {
  projectId: number;
  installRoot: string;
  bankStem: "pbp_eng" | "ann_eng" | "music_samples";
  sampleId: number | null;
  /** Why this sample cannot be previewed (from the listing); no request is offered then. */
  unavailable?: string | null;
}) {
  const [state, setState] = useState<State | null>(null);
  const genRef = useRef(0);
  const inflightRef = useRef(false);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const key = `${projectId}|${installRoot}|${bankStem}|${sampleId ?? ""}`;
  const release = () => {
    const a = audioRef.current;
    if (a) {
      try {
        a.pause();
        a.removeAttribute("src");
        a.load();
      } catch {
        /* jsdom / detached */
      }
    }
  };
  /** Stop the server-side decode of a request still in flight (the helper is killed there). */
  const stopInflight = () => {
    if (!inflightRef.current) return;
    inflightRef.current = false;
    void call("gen7_speech_preview_cancel", { projectId }).catch(() => undefined);
  };
  useEffect(() => {
    genRef.current += 1;
    stopInflight();
    release();
    setState(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  useEffect(
    () => () => {
      stopInflight();
      release();
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const request = async () => {
    if (sampleId == null || unavailable != null) return;
    const gen = ++genRef.current;
    release();
    setState({ key, status: "loading" });
    inflightRef.current = true;
    try {
      const data = await call("gen7_speech_preview", { projectId, installRoot, bankStem, sampleId });
      if (gen !== genRef.current) return;
      inflightRef.current = false;
      setState({ key, status: "ready", data });
    } catch (e) {
      if (gen !== genRef.current) return;
      inflightRef.current = false;
      setState({ key, status: "error", message: String(e) });
    }
  };
  const cancel = () => {
    genRef.current += 1;
    stopInflight();
    release();
    setState({ key, status: "cancelled" });
  };

  if (sampleId == null) return null;
  const shown = state && state.key === key ? state : null;
  return (
    <div className="mt-2 space-y-1 rounded border border-slate-800 bg-slate-900/40 p-2 text-xs" data-testid="gen7-preview">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-slate-400">
          Sample <span className="font-mono text-slate-200">{sampleId}</span> · bank <span className="font-mono text-slate-200">{bankStem}</span>
        </span>
        {unavailable == null && (
          <Button onClick={() => void request()} disabled={shown?.status === "loading"} data-testid="gen7-preview-request">
            {shown?.status === "loading" ? "Decoding…" : "Decode diagnostic preview"}
          </Button>
        )}
        {shown?.status === "loading" && (
          <Button onClick={cancel} data-testid="gen7-preview-cancel">
            Cancel
          </Button>
        )}
      </div>
      {unavailable != null && (
        <p className="rounded border border-slate-700 px-2 py-1 text-slate-300" data-testid="gen7-preview-unavailable">
          No preview for this sample: {unavailable}
        </p>
      )}
      {shown?.status === "cancelled" && (
        <p className="text-slate-400" data-testid="gen7-preview-cancelled">
          Decode cancelled — the decoder was stopped and nothing is playing.
        </p>
      )}
      {shown?.status === "error" && (
        <p className="rounded border border-rose-800 bg-rose-950/40 px-2 py-1 text-rose-200" data-testid="gen7-preview-error">
          {shown.message}
        </p>
      )}
      {shown?.status === "ready" && (
        <div className="space-y-1" data-testid="gen7-preview-ready">
          <p className="rounded border border-amber-800 bg-amber-950/40 px-2 py-1 text-amber-200" data-testid="gen7-preview-label">
            {shown.data.label}
          </p>
          <audio ref={audioRef} controls src={shown.data.wav_data_url} data-testid="gen7-preview-audio" className="w-full" />
          <details className="text-xs"><summary className="cursor-pointer text-slate-400">Audio diagnostics</summary>
          {shown.data.route_detail != null ? (
            <p className="text-slate-400" data-testid="gen7-preview-diag">
              {shown.data.codec} · {shown.data.sample_rate} Hz · {shown.data.channels} ch · SNR {shown.data.num_samples} samples · {shown.data.blocks} blocks · {shown.data.route_detail}
              {shown.data.first_sample_over_1_lsb != null ? ` · first > 1 LSB at ${shown.data.first_sample_over_1_lsb}` : ""} · tuple {shown.data.tuple_source}
            </p>
          ) : (
            <p className="text-slate-400" data-testid="gen7-preview-diag">
              {shown.data.codec} · {shown.data.sample_rate} Hz · {shown.data.channels} ch · SNR {shown.data.num_samples} samples · {shown.data.blocks} blocks · {shown.data.granules} granules (
              {shown.data.granules_per_block.join("/")}) → {shown.data.rebuilt_frames} frames, main_data_begin ≤ {shown.data.max_main_data_begin} · PCM frame {shown.data.pcm_frame_samples} samples · decoded{" "}
              {shown.data.decoded_samples_per_channel} per channel (surplus {shown.data.surplus_samples})
              {shown.data.first_sample_over_1_lsb != null ? ` · first > 1 LSB at ${shown.data.first_sample_over_1_lsb}` : ""} · tuple {shown.data.tuple_source}
            </p>
          )}
          <p className="font-mono text-[10px] text-slate-500" data-testid="gen7-preview-identity">
            {srcLabel(shown.data.bank_path)} {shown.data.bank_len} B sha256 {shown.data.bank_sha256.slice(0, 16)}… · {srcLabel(shown.data.stream_path)} {shown.data.stream_len} B · sample @ {shown.data.offset} · SNS{" "}
            {shown.data.sns_len} B sha256 {shown.data.sns_sha256.slice(0, 16)}…
          </p>
          </details>
          {shown.data.notes.map((n) => (
            <p key={n} className="text-slate-500">
              {n}
            </p>
          ))}
        </div>
      )}
    </div>
  );
}

import { assetRoot } from "../lib/assetRoot";
// P4 (overnight 2026-09-24) — PS4 NHL 15–21 speech banks: the SND10 `.sbr` banks of the extracted
// kettle speech directory, one bank's samples with the codec measured from each sample's SNR
// header, and the **diagnostic** EA-MP3 preview (research R67 / R71 contract rev. 2). The preview
// is never presented as game audio: its label names the rendering ("candidate A" replaces the
// decoder's first frame with the 1 152 embedded samples — statically supported, not
// listening-verified) and every reply carries the bank's identity (length + sha256) and the
// sample's own SNS hash.
//
// Ownership: the preview is bound to (project, install root, bank file + sha, sample id,
// rendering) and to a request generation; a reply for another selection is dropped, and the
// playing <audio> element is paused and released whenever the selection, bank, root or project
// changes or the card unmounts (nothing keeps playing for a sample that is no longer shown).
import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, Card, Pill } from "./primitives";
import type { Ps4SpeechPreviewDto } from "../bindings/Ps4SpeechPreviewDto";

type Rendering = "mpeg_only" | "candidate_a";

export function Ps4SpeechBankCard({ projectId }: { projectId: number }) {
  const { install, iso } = useProject();
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  const [bank, setBank] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<number | null>(null);
  const [rendering, setRendering] = useState<Rendering>("candidate_a");

  const banks = useQuery({
    queryKey: ["ps4-speech-banks", projectId, installRoot],
    queryFn: () => call("ps4_speech_banks", { projectId, installRoot: installRoot! }),
    enabled: installRoot != null,
    retry: false,
  });
  const activeBank = bank ?? banks.data?.banks.find((b) => b.samples > 0 && b.sbs_present)?.file ?? null;
  const samples = useQuery({
    queryKey: ["ps4-speech-samples", projectId, installRoot, activeBank, query],
    queryFn: () => call("ps4_speech_samples", { projectId, installRoot: installRoot!, bankFile: activeBank!, query, limit: 500 }),
    enabled: installRoot != null && activeBank != null,
    retry: false,
  });

  // The preview: requested explicitly, bound to its selection key, played through one owned element.
  const [preview, setPreview] = useState<{ key: string; status: "loading" } | { key: string; status: "ready"; data: Ps4SpeechPreviewDto } | { key: string; status: "error"; message: string } | null>(null);
  const genRef = useRef(0);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const selectionKey = `${projectId}|${installRoot ?? ""}|${activeBank ?? ""}|${selected ?? ""}|${rendering}`;
  const release = () => {
    const a = audioRef.current;
    if (a) {
      try {
        a.pause();
        a.removeAttribute("src");
        a.load();
      } catch {
        /* jsdom / detached element */
      }
    }
  };
  useEffect(() => {
    // Any change of selection retires the in-flight or playing preview.
    genRef.current += 1;
    release();
    setPreview(null);
  }, [selectionKey]);
  useEffect(() => () => release(), []);

  const requestPreview = async () => {
    if (installRoot == null || activeBank == null || selected == null) return;
    const gen = ++genRef.current;
    const key = selectionKey;
    release();
    setPreview({ key, status: "loading" });
    try {
      const data = await call("ps4_speech_preview", { projectId, installRoot, bankFile: activeBank, sid: selected, rendering });
      if (gen !== genRef.current) return; // the selection moved on; this reply is not for what is shown
      setPreview({ key, status: "ready", data });
    } catch (e) {
      if (gen !== genRef.current) return;
      setPreview({ key, status: "error", message: String(e) });
    }
  };

  if (installRoot == null) return null;
  if (banks.isError) {
    return (
      <Card title="PS4 speech banks (listing + diagnostic preview)">
        <p className="text-xs text-slate-500" data-testid="ps4-speech-unavailable">
          {String(banks.error)}
        </p>
      </Card>
    );
  }
  const b = banks.data;
  const s = samples.data;
  const shown = preview && preview.key === selectionKey ? preview : null;
  return (
    <Card title="PS4 speech banks (listing + diagnostic preview)" subtitle={b ? `${b.banks.length} banks in ${b.speech_dir}` : undefined}>
      {b && (
        <div className="mb-2 flex flex-wrap items-center gap-3 text-xs">
          <label className="text-slate-400">
            Bank{" "}
            <select
              data-testid="ps4-speech-bank"
              className="rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-slate-100"
              value={activeBank ?? ""}
              onChange={(e) => {
                setBank(e.target.value);
                setSelected(null);
              }}
            >
              {b.banks.map((k) => (
                <option key={k.file} value={k.file} disabled={!k.sbs_present || k.samples === 0}>
                  {k.file} · {k.samples} samples{k.sbs_present ? "" : " · .sbs missing"}
                  {k.note ? ` · ${k.note}` : ""}
                </option>
              ))}
            </select>
          </label>
          <input
            data-testid="ps4-speech-query"
            className="w-40 rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-slate-100"
            placeholder="sample id contains…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          {s && (
            <span className="text-slate-500" data-testid="ps4-speech-census">
              {s.total} samples · {s.codec_census.map((c) => `${c.codec} ${c.count}`).join(" · ")}
              {s.truncated ? ` · first ${s.rows.length} of ${s.matching} shown` : ""}
            </span>
          )}
        </div>
      )}
      {samples.isError && (
        <p className="text-xs text-rose-300" data-testid="ps4-speech-samples-error">
          {String(samples.error)}
        </p>
      )}
      {s && (
        <div className="grid gap-3 md:grid-cols-[1fr_1fr]">
          <ul className="max-h-72 space-y-0.5 overflow-auto text-xs" data-testid="ps4-speech-rows">
            {s.rows.map((r) => (
              <li key={r.sid}>
                <button
                  type="button"
                  onClick={() => setSelected(r.sid)}
                  data-testid={`ps4-speech-row-${r.sid}`}
                  className={`flex w-full items-center gap-2 rounded px-2 py-0.5 text-left hover:bg-slate-800 ${selected === r.sid ? "bg-slate-800 text-slate-100" : "text-slate-300"}`}
                >
                  <span className="w-16 font-mono">{r.sid}</span>
                  <Pill tone={r.previewable ? "ok" : r.problem ? "error" : "warn"}>{r.codec ?? "unread"}</Pill>
                  <span className="text-slate-500">
                    {r.duration_ms != null ? `${(r.duration_ms / 1000).toFixed(2)} s` : ""}
                    {r.sample_rate ? ` · ${r.sample_rate} Hz` : ""}
                    {r.channels ? ` · ${r.channels} ch` : ""}
                    {r.problem ? ` · ${r.problem}` : ""}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <div className="space-y-2 text-xs" data-testid="ps4-speech-preview-pane">
            {selected == null ? (
              <p className="text-slate-500">Select a sample. Only EA-MP3 samples (codec 0x0b, 48 kHz mono) have a preview route; other codecs are listed, not decoded.</p>
            ) : (
              <>
                <div className="flex flex-wrap items-center gap-2">
                  <label className="text-slate-400">
                    Start{" "}
                    <select data-testid="ps4-speech-rendering" className="rounded border border-slate-700 bg-slate-900 px-1 py-0.5 text-slate-100" value={rendering} onChange={(e) => setRendering(e.target.value as Rendering)}>
                      <option value="candidate_a">candidate A (embedded PCM replaces frame 0)</option>
                      <option value="mpeg_only">MPEG frames only</option>
                    </select>
                  </label>
                  <Button onClick={() => void requestPreview()} disabled={shown?.status === "loading"} data-testid="ps4-speech-preview">
                    {shown?.status === "loading" ? "Decoding…" : "Decode diagnostic preview"}
                  </Button>
                </div>
                {shown?.status === "error" && (
                  <p className="rounded border border-rose-800 bg-rose-950/40 px-2 py-1 text-rose-200" data-testid="ps4-speech-preview-error">
                    {shown.message}
                  </p>
                )}
                {shown?.status === "ready" && (
                  <div className="space-y-1" data-testid="ps4-speech-preview-ready">
                    <p className="rounded border border-amber-800 bg-amber-950/40 px-2 py-1 text-amber-200" data-testid="ps4-speech-preview-label">
                      {shown.data.label}
                    </p>
                    <audio ref={audioRef} controls src={shown.data.wav_data_url} data-testid="ps4-speech-audio" className="w-full" />
                    <p className="text-slate-400" data-testid="ps4-speech-preview-diag">
                      {shown.data.codec} · {shown.data.sample_rate} Hz · {shown.data.channels} ch · {shown.data.num_samples} samples · {shown.data.blocks} blocks · {shown.data.frames} frames (
                      {shown.data.frames_per_block.join("/")}) · tail trim {shown.data.trim} · embedded {shown.data.embedded_pcm_samples}
                      {shown.data.first_sample_over_1_lsb != null ? ` · first > 1 LSB at ${shown.data.first_sample_over_1_lsb}` : ""}
                    </p>
                    <p className="font-mono text-[10px] text-slate-500" data-testid="ps4-speech-preview-identity">
                      {shown.data.bank.file} sha256 {shown.data.bank.sbr_sha256.slice(0, 16)}… · sample {shown.data.sid} @ {shown.data.offset} · SNS {shown.data.sns_len} B sha256 {shown.data.sns_sha256.slice(0, 16)}…
                    </p>
                    {shown.data.notes.map((n) => (
                      <p key={n} className="text-slate-500">
                        {n}
                      </p>
                    ))}
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      )}
      {(b?.notes ?? []).map((n) => (
        <p key={n} className="mt-2 text-[11px] text-slate-500">
          {n}
        </p>
      ))}
    </Card>
  );
}

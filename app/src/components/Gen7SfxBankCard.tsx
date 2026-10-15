import { assetRoot } from "../lib/assetRoot";
// 25 Sep 2026 — gen7 (PS3 / Xbox 360) **sound banks** (`.abk` + `.ast`) on the research's measured
// populations (R82 / R89 / R91 / R92; since 26 Sep also the archive-backed PS3 NHL 09 / 10 / 12 / 13
// from each title's own archive members, R98 / R100): the bank listing with record counts and the `.ast`
// id check; a bank's named picker tree (the `_msg` sample pickers and `_snd` parameter nodes that
// own each sample list) with every entry's codec / rate / count / framing; and, for PS3 EALayer3
// entries, the **diagnostic** decode (R92 candidate construction, labelled "not game-verified"); since
// 26 Sep a 4- / 6-channel entry decodes one stereo pair at a time (inferred de-interleave) and a loop
// plays stub + body once with its loop point shown. Xbox 360 banks list and frame but have no decoder;
// Xbox 360 NHL 09 shows its refusal in words. Nothing here writes or
// claims which sample the game plays.
import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useSearchParams } from "react-router-dom";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, Card, Pill } from "./primitives";
import type { Gen7SfxEntryDto } from "../bindings/Gen7SfxEntryDto";
import type { Gen7SfxPreviewDto } from "../bindings/Gen7SfxPreviewDto";

function fileName(p: string): string {
  return p.split(/[\\/]/).pop() ?? p;
}

export function Gen7SfxBankCard({ projectId }: { projectId: number }) {
  const { install, iso } = useProject();
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  // A bank named in the URL (`?bank=000_goal.abk`, from the arena page's horn link) opens directly.
  const [params] = useSearchParams();
  const [bank, setBank] = useState<string | null>(params.get("bank"));
  const [selected, setSelected] = useState<number | null>(null);
  const banks = useQuery({
    queryKey: ["gen7-sfx-banks", projectId, installRoot],
    queryFn: () => call("gen7_sfx_banks", { projectId, installRoot: installRoot! }),
    enabled: installRoot != null,
    retry: false,
  });
  const tree = useQuery({
    queryKey: ["gen7-sfx-tree", projectId, installRoot, bank],
    queryFn: () => call("gen7_sfx_bank_tree", { projectId, installRoot: installRoot!, bank: bank! }),
    enabled: installRoot != null && bank != null,
    retry: false,
  });
  if (installRoot == null) return null;
  const list = banks.data;
  const t = tree.data;
  return (
    <Card
      title="Gen7 sound banks (.abk listing, picker tree, diagnostic preview)"
      subtitle={list ? `${list.banks.length} banks in ${list.dir} · population ${list.population}` : undefined}
    >
      {banks.isError && (
        <p className="text-xs text-amber-300" data-testid="gen7-sfx-unavailable">
          {String(banks.error)}
        </p>
      )}
      {list && (
        <div className="mb-2 flex flex-wrap items-center gap-3 text-xs">
          <label className="text-slate-400">
            Bank{" "}
            <select
              data-testid="gen7-sfx-bank"
              className="rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-slate-100"
              value={bank ?? ""}
              onChange={(e) => {
                setBank(e.target.value || null);
                setSelected(null);
              }}
            >
              <option value="">— choose a bank —</option>
              {list.banks.map((b) => (
                <option key={b.name} value={b.name} disabled={b.problem != null}>
                  {b.name} · {b.records} rec ({b.stream_records} stream / {b.ram_records} ram) · {b.lists} lists
                  {b.ast ? (b.ast_id_matches ? " · .ast ✓" : " · .ast id mismatch") : ""}
                  {b.problem ? ` · ${b.problem}` : ""}
                </option>
              ))}
            </select>
          </label>
          <span className="text-slate-500" data-testid="gen7-sfx-census">
            {list.banks.reduce((n, b) => n + b.records, 0)} records · {list.banks.filter((b) => b.ast).length} with an .ast ·{" "}
            {list.banks.filter((b) => b.problem).length} unreadable
          </span>
        </div>
      )}
      {tree.isError && (
        <p className="text-xs text-rose-300" data-testid="gen7-sfx-tree-error">
          {String(tree.error)}
        </p>
      )}
      {t && (
        <>
          <p className="mb-1 text-[11px] text-slate-500" data-testid="gen7-sfx-tree-facts">
            {t.bank_platform} bank · {t.records} records ({t.stream_records} stream / {t.ram_records} ram) · {t.lists} sample lists · {t.names} names ·{" "}
            {t.ast_path ? `${fileName(t.ast_path)} (${t.ast_id_matches ? "id matches" : "id MISMATCH"})` : "no .ast (ram only)"} · sha256 {t.bank_sha256.slice(0, 12)}…
            {t.bank_wrapper !== "loose" && (
              <span data-testid="gen7-sfx-member" title={`${t.bank_path} · stored sha256 ${t.bank_stored_sha256}`}>
                {" "}· archive member ({t.bank_wrapper}){t.bank_copies > 1 ? ` · ${t.bank_copies} identical copies` : ""}
              </span>
            )}
          </p>
          <ul className="max-h-80 space-y-1 overflow-auto text-xs" data-testid="gen7-sfx-pickers">
            {t.pickers.map((p) => (
              <li key={p.list_offset} className="rounded border border-slate-800 bg-slate-900/40 px-2 py-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-mono text-slate-200">{p.owners.length ? p.owners.join(" · ") : "(unowned list)"}</span>
                  {p.owner_class && <Pill tone="warn">{p.owner_class}</Pill>}
                  <span className="text-slate-500">
                    {p.entries.length} entr{p.entries.length === 1 ? "y" : "ies"}
                    {p.empty_slots ? ` · ${p.empty_slots} empty slot(s)` : ""}
                  </span>
                </div>
                <ul className="mt-1 space-y-0.5">
                  {p.entries.map((e) => (
                    <EntryRow key={e.record} entry={e} selected={selected === e.record} onSelect={() => setSelected(e.record)} />
                  ))}
                </ul>
              </li>
            ))}
          </ul>
          <SfxPreview
            projectId={projectId}
            installRoot={installRoot}
            bank={t.bank}
            record={selected}
            subStreams={t.pickers.flatMap((p) => p.entries).find((e) => e.record === selected)?.sub_streams ?? 1}
          />
          {t.notes.map((n) => (
            <p key={n} className="mt-2 text-[11px] text-slate-500">
              {n}
            </p>
          ))}
        </>
      )}
      {list?.notes.map((n) => (
        <p key={n} className="mt-2 text-[11px] text-slate-500">
          {n}
        </p>
      ))}
    </Card>
  );
}

function EntryRow({ entry: e, selected, onSelect }: { entry: Gen7SfxEntryDto; selected: boolean; onSelect: () => void }) {
  const seconds = e.sample_rate ? (e.num_samples / e.sample_rate).toFixed(2) : "?";
  const title = [
    e.framing ?? "",
    e.preview_refusal ?? "",
    `X 0x${e.x.toString(16)} · S ${e.s} (unidentified words, carried as read — R94)`,
    e.stream_offset != null ? `.ast start ${e.stream_offset}` : "ram record (inline)",
  ]
    .filter(Boolean)
    .join("\n");
  return (
    <li>
      <button
        type="button"
        onClick={onSelect}
        title={title}
        data-testid={`gen7-sfx-entry-${e.record}`}
        className={`flex w-full flex-wrap items-center gap-2 rounded px-2 py-0.5 text-left hover:bg-slate-800 ${selected ? "bg-slate-800 text-slate-100" : "text-slate-300"}`}
      >
        <span className="w-12 font-mono">#{e.record}</span>
        <Pill tone={e.previewable ? "ok" : e.framing?.startsWith("framing refused") ? "error" : "warn"}>{e.codec}</Pill>
        <span className="text-slate-500">
          {e.kind} · w{e.weight} · {e.sample_rate} Hz · cc {e.channel_config}
          {e.sub_streams && e.sub_streams > 1 ? ` (${e.sub_streams} streams)` : ""} · {e.num_samples} samples ≈ {seconds} s
          {e.blocks != null ? ` · ${e.blocks} blocks` : ""}
        </span>
        {!e.previewable && e.preview_refusal && (
          <span className="text-[10px] uppercase tracking-wide text-slate-500" data-testid={`gen7-sfx-no-preview-${e.record}`}>
            no preview
          </span>
        )}
      </button>
    </li>
  );
}

type PState = { key: string; status: "loading" } | { key: string; status: "ready"; data: Gen7SfxPreviewDto } | { key: string; status: "error"; message: string };

function SfxPreview({ projectId, installRoot, bank, record, subStreams }: { projectId: number; installRoot: string; bank: string; record: number | null; subStreams: number }) {
  const [state, setState] = useState<PState | null>(null);
  // Channel config 3 / 5: which stereo pair (0-based) to decode; one-stream entries send none.
  const [pair, setPair] = useState(0);
  const genRef = useRef(0);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const key = `${projectId}|${installRoot}|${bank}|${record ?? ""}|${subStreams > 1 ? pair : ""}`;
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
  useEffect(() => {
    genRef.current += 1;
    release();
    setState(null);
  }, [key]);
  useEffect(() => setPair(0), [record]);
  useEffect(() => () => release(), []);
  if (record == null) return null;
  const request = async () => {
    const gen = ++genRef.current;
    release();
    setState({ key, status: "loading" });
    try {
      const data = await call("gen7_sfx_preview", { projectId, installRoot, bank, record, subStream: subStreams > 1 ? pair : null });
      if (gen !== genRef.current) return;
      setState({ key, status: "ready", data });
    } catch (e) {
      if (gen !== genRef.current) return;
      setState({ key, status: "error", message: String(e) });
    }
  };
  const shown = state && state.key === key ? state : null;
  return (
    <div className="mt-2 rounded border border-slate-800 bg-slate-950/60 p-2 text-xs" data-testid="gen7-sfx-preview">
      <div className="flex items-center gap-2">
        <span className="text-slate-300">
          {bank} · record #{record}
        </span>
        {subStreams > 1 && (
          <label className="text-slate-400" title="4- / 6-channel entries are 2 / 3 interleaved stereo streams; the split (EA frame i → stream i mod N) is the research's inference, not reference-validated">
            stereo pair{" "}
            <select data-testid="gen7-sfx-pair" className="rounded border border-slate-700 bg-slate-900 px-1 text-slate-100" value={pair} onChange={(e) => setPair(Number(e.target.value))}>
              {Array.from({ length: subStreams }, (_, k) => (
                <option key={k} value={k}>
                  {k + 1} of {subStreams}
                </option>
              ))}
            </select>
          </label>
        )}
        <Button onClick={request} disabled={shown?.status === "loading"} data-testid="gen7-sfx-preview-request">
          Decode diagnostic preview
        </Button>
      </div>
      {shown?.status === "error" && (
        <p className="mt-1 text-rose-300" data-testid="gen7-sfx-preview-refusal">
          {shown.message}
        </p>
      )}
      {shown?.status === "ready" && (
        <div className="mt-1 space-y-1">
          <p className="text-amber-300" data-testid="gen7-sfx-preview-label">
            {shown.data.label}
          </p>
          <audio ref={audioRef} controls src={shown.data.wav_data_url} data-testid="gen7-sfx-preview-audio" className="w-full" />
          <p className="text-slate-400">
            {shown.data.codec} · {shown.data.sample_rate} Hz · {shown.data.channels} ch · {shown.data.num_samples} samples · {shown.data.blocks} block(s) → {shown.data.granules} granules → {shown.data.rebuilt_frames} frames
            {shown.data.owners.length ? ` · picker ${shown.data.owners.join(" · ")}` : ""}
            {shown.data.sub_stream != null ? ` · stereo pair ${shown.data.sub_stream + 1} of ${shown.data.sub_streams}` : ""}
            {shown.data.loop_start != null ? ` · loop point at sample ${shown.data.loop_start} (played once)` : ""}
          </p>
          <p className="text-slate-500">
            {shown.data.source} · sha256 {shown.data.source_sha256.slice(0, 12)}…
          </p>
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

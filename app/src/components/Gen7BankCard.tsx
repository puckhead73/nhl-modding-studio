import { assetRoot } from "../lib/assetRoot";
// P4.6 (overnight 2026-09-24) — gen7 (PS3 / Xbox 360) sample banks listed from their SBKR table:
// play-by-play, announcer and music, each on the research's **measured tuples** only (R77 / R80);
// every row carries the codec / rate / channels / count read from its own SNR header, and a row
// can be selected for the **diagnostic** EALayer3 v1 preview (label: "start handling unresolved …
// not game-verified"). A title / bank outside the measured set shows the refusal instead of a
// guess; nothing here writes or claims which sample the game plays. Receipt 33: Xbox 360 NHL 09 / 10
// speech and NHL 09–12 music are EA-XMA (R84) and preview through the XMA2 helper; without one the
// card says so and no row is previewable, and a selected row that cannot preview shows why.
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Card, Pill } from "./primitives";
import { Gen7SpeechPreview } from "./Gen7SpeechPreview";

type Stem = "pbp_eng" | "ann_eng" | "music_samples";
const STEMS: Array<{ stem: Stem; label: string }> = [
  { stem: "pbp_eng", label: "play-by-play (pbp_eng)" },
  { stem: "ann_eng", label: "announcer (ann_eng)" },
  { stem: "music_samples", label: "music (music_samples)" },
];

export function Gen7BankCard({ projectId }: { projectId: number }) {
  const { install, iso } = useProject();
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  const [stem, setStem] = useState<Stem>("pbp_eng");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<number | null>(null);
  const q = useQuery({
    queryKey: ["gen7-bank", projectId, installRoot, stem, query],
    queryFn: () => call("gen7_bank_samples", { projectId, installRoot: installRoot!, bankStem: stem, query, limit: 500 }),
    enabled: installRoot != null,
    retry: false,
  });
  if (installRoot == null) return null;
  const d = q.data;
  const selectedRow = d?.rows.find((r) => r.sample_id === selected) ?? null;
  return (
    <Card title="Gen7 sample banks (listing + diagnostic preview)" subtitle={d ? `${d.bank} · stream ${d.stream}` : undefined}>
      <div className="mb-2 flex flex-wrap items-center gap-3 text-xs">
        <label className="text-slate-400">
          Bank{" "}
          <select
            data-testid="gen7-bank-stem"
            className="rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-slate-100"
            value={stem}
            onChange={(e) => {
              setStem(e.target.value as Stem);
              setSelected(null);
            }}
          >
            {STEMS.map((s) => (
              <option key={s.stem} value={s.stem}>
                {s.label}
              </option>
            ))}
          </select>
        </label>
        <input
          data-testid="gen7-bank-query"
          className="w-40 rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-slate-100"
          placeholder="sample id contains…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        {d && (
          <span className="text-slate-500" data-testid="gen7-bank-census">
            {d.total} samples · {d.codec_census.map((c) => `${c.codec} ${c.count}`).join(" · ")}
            {d.truncated ? ` · first ${d.rows.length} of ${d.matching} shown` : ""} · tuple {d.tuple_source}
          </span>
        )}
      </div>
      {q.isError && (
        <p className="text-xs text-amber-300" data-testid="gen7-bank-unavailable">
          {String(q.error)}
        </p>
      )}
      {d?.preview_unavailable != null && (
        <p className="mb-2 rounded border border-amber-800 bg-amber-950/40 px-2 py-1 text-xs text-amber-200" data-testid="gen7-bank-preview-unavailable">
          Listed and framed only — {d.preview_unavailable}
        </p>
      )}
      {d && (
        <>
          <ul className="max-h-72 space-y-0.5 overflow-auto text-xs" data-testid="gen7-bank-rows">
            {d.rows.map((r) => (
              <li key={r.sample_id}>
                <button
                  type="button"
                  onClick={() => setSelected(r.sample_id)}
                  data-testid={`gen7-bank-row-${r.sample_id}`}
                  className={`flex w-full items-center gap-2 rounded px-2 py-0.5 text-left hover:bg-slate-800 ${selected === r.sample_id ? "bg-slate-800 text-slate-100" : "text-slate-300"}`}
                >
                  <span className="w-16 font-mono">{r.sample_id}</span>
                  <Pill tone={r.previewable ? "ok" : r.problem ? "error" : "warn"}>{r.codec ?? "unread"}</Pill>
                  <span className="text-slate-500">
                    {r.sample_rate ? `${r.sample_rate} Hz` : ""}
                    {r.channels ? ` · ${r.channels} ch` : ""}
                    {r.num_samples != null && r.sample_rate ? ` · ${(r.num_samples / r.sample_rate).toFixed(2)} s` : ""}
                    {` · @ ${r.offset}`}
                    {r.problem ? ` · ${r.problem}` : ""}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <Gen7SpeechPreview projectId={projectId} installRoot={installRoot} bankStem={stem} sampleId={selected} unavailable={selectedRow && !selectedRow.previewable ? (d.preview_unavailable ?? selectedRow.problem ?? `${selectedRow.codec ?? "unread"} samples have no preview route on this measured tuple (${d.tuple_source})`) : null} />
          {d.notes.map((n) => (
            <p key={n} className="mt-2 text-[11px] text-slate-500">
              {n}
            </p>
          ))}
        </>
      )}
    </Card>
  );
}

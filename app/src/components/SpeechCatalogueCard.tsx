import { assetRoot } from "../lib/assetRoot";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Card, Pill } from "./primitives";
import type { SpeechSentenceDto } from "../bindings/SpeechSentenceDto";
import { Gen7SpeechPreview } from "./Gen7SpeechPreview";

/// Phase 1.5 / 2.7 / 3.5 (21 Sep 2026) — the **read-only** gen7 commentary catalogue (PS3 / Xbox
/// 360 NHL 11–Legacy): speech events with their parameters, the sentences that share an event's
/// name with their bodies and phrases, each phrase resolved to its sample group's candidate samples
/// and to the bank. Wording follows the research contract: a sentence "shares this event's name"
/// (never "is the line this event speaks"), an event without one has "no sentence carries this
/// event name" (never "silent"), a phrase's samples are candidates "the event's parameters select
/// among". Nothing here writes a sample. Since 24 Sep 2026 (P4.5) a phrase's candidate sample id
/// can be selected for the **diagnostic** EALayer3 v1 preview on the measured tuples (research
/// R77 / R80) — labelled "start handling unresolved; not game-verified"; titles outside them refuse.
export function SpeechCatalogueCard({ projectId }: { projectId: number }) {
  const { install, iso } = useProject();
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  const [query, setQuery] = useState("");
  const [leagueOnly, setLeagueOnly] = useState(true);
  const [open, setOpen] = useState<string | null>(null);
  const cat = useQuery({
    queryKey: ["speech-catalogue", projectId, installRoot, query, leagueOnly],
    queryFn: () => call("speech_catalogue", { projectId, installRoot: installRoot!, query, leagueOnly, limit: 300 }),
    enabled: installRoot != null,
  });
  if (installRoot == null) {
    return (
      <Card title="Commentary (read-only)" subtitle="Open an install root to list the title's speech data.">
        <p className="text-xs text-slate-500">The catalogue reads the loose audio/speech files of the install.</p>
      </Card>
    );
  }
  const controls = (
    <div className="mb-3 flex flex-wrap items-center gap-3 text-xs">
      <input
        data-testid="speech-query"
        className="w-64 rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-slate-100"
        placeholder="event / sentence name contains…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        spellCheck={false}
      />
      <label className="flex items-center gap-1 text-slate-300">
        <input data-testid="speech-league-only" type="checkbox" checked={leagueOnly} onChange={(e) => setLeagueOnly(e.target.checked)} />
        league / competition names only
      </label>
    </div>
  );
  if (cat.isLoading) return <Card title="Commentary (read-only)">{controls}<p className="text-xs text-slate-500">Reading the speech tables…</p></Card>;
  if (cat.error) {
    return (
      <Card title="Commentary (read-only)">
        {controls}
        <p data-testid="speech-error" className="text-xs text-rose-300">{String(cat.error)}</p>
      </Card>
    );
  }
  const d = cat.data!;
  const s = d.stats;
  return (
    <Card
      title="Commentary (read-only)"
      subtitle={`${d.game_id} (${d.platform}) — ${s.events} events, ${s.sentences} sentences (${s.events_with_a_sentence} events have one; ${s.events_without_a_sentence} have none), ${s.phrases} phrases (${s.phrases_with_table} with a sample-group table), ${s.graffiti_tables} graffiti tables, ${s.bank_samples} bank samples${s.keywords ? `, ${s.keywords} keywords` : ""}`}
    >
      {controls}
      <p className="mb-2 text-xs text-slate-500" data-testid="speech-sources" title={d.sources.map((f) => `${f.role}: ${f.path} (${f.len} B, ${f.sha256.slice(0, 16)}…)`).join("\n")}>
        Sources: {d.speech_dir} — {d.sources.length} files (hover for identities).
      </p>
      {d.notes.map((n, i) => (
        <p key={i} className="mb-1 text-xs text-amber-300/80">
          {n}
        </p>
      ))}
      <p className="mb-2 text-xs text-slate-400" data-testid="speech-matching">
        {d.events_matching} events and {d.sentences_matching} sentences match{d.truncated ? ` (showing the first ${d.events.length} / ${d.sentences.length}; narrow the query)` : ""}.
      </p>
      <section className="mb-4">
        <h3 className="text-sm font-semibold text-slate-100">Events</h3>
        <table className="mt-1 w-full text-left text-xs">
          <thead className="text-slate-500">
            <tr>
              <th className="py-1 pr-2">event</th>
              <th className="py-1 pr-2">id</th>
              <th className="py-1 pr-2">parameters</th>
              <th className="py-1 pr-2">sentences sharing this name</th>
            </tr>
          </thead>
          <tbody>
            {d.events.map((e) => (
              <tr key={`${e.id}/${e.name}`} className="border-t border-slate-800 align-top" data-testid={`speech-event-${e.name}`}>
                <td className="py-1 pr-2 font-mono text-slate-200">
                  {e.name}
                  {e.other_names_for_id.length > 0 && (
                    <span className="ml-1 text-slate-500" title={`this 16-bit id also names: ${e.other_names_for_id.join(", ")}`}>
                      (id shared with {e.other_names_for_id.length} other name{e.other_names_for_id.length > 1 ? "s" : ""})
                    </span>
                  )}
                </td>
                <td className="py-1 pr-2 font-mono text-slate-400">{e.id}</td>
                <td className="py-1 pr-2 text-slate-300">{e.parameters.join(", ") || "—"}</td>
                <td className="py-1 pr-2">
                  {e.sentences > 0 ? (
                    <button className="text-sky-300 hover:underline" onClick={() => { setOpen(e.name); setQuery(e.name); }}>
                      {e.sentences}
                    </button>
                  ) : (
                    <span className="text-slate-500">no sentence carries this event name</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
      <section>
        <h3 className="text-sm font-semibold text-slate-100">Sentences sharing an event's name</h3>
        {d.sentences.map((st) => (
          <SentenceRow key={`${st.slot}/${st.name}`} s={st} open={open === st.name} onToggle={() => setOpen(open === st.name ? null : st.name)} projectId={projectId} installRoot={installRoot} />
        ))}
        {d.sentences.length === 0 && <p className="text-xs text-slate-500">No sentence matches.</p>}
      </section>
    </Card>
  );
}

function SentenceRow({ s, open, onToggle, projectId, installRoot }: { s: SpeechSentenceDto; open: boolean; onToggle: () => void; projectId: number; installRoot: string }) {
  const phrases = s.bodies.reduce((n, b) => n + b.phrases.length, 0);
  const [selected, setSelected] = useState<number | null>(null);
  return (
    <div className="border-t border-slate-800 py-1 text-xs" data-testid={`speech-sentence-${s.name}`}>
      <button className="font-mono text-slate-200 hover:underline" onClick={onToggle}>
        {s.name}
      </button>
      <span className="ml-2 text-slate-500">
        {s.bodies.length} bod{s.bodies.length === 1 ? "y" : "ies"}, {phrases} phrase{phrases === 1 ? "" : "s"}
        {s.event_id != null ? ` · event ${s.event_id}` : " · no event of this name"}
        {!s.slot_ok && <Pill tone="warn">hash slot mismatch</Pill>}
      </span>
      {open && (
        <div className="mt-1 space-y-2 pl-4">
          {s.bodies.map((b, bi) => (
            <div key={b.id}>
              <div className="text-slate-400">body {bi + 1} (id {b.id})</div>
              <ol className="list-decimal pl-5">
                {b.phrases.map((p, pi) => (
                  <li key={pi} className="text-slate-300">
                    sample group <span className="font-mono">{p.group_name ?? `${p.group_tag}:${p.group_value}`}</span>
                    {p.parameters.length > 0 && <span> — the event's {p.parameters.join(", ")} select among the candidates</span>}
                    {" · "}
                    {p.table_found ? (
                      <span>
                        {p.candidate_count} candidate sample{p.candidate_count === 1 ? "" : "s"}, {p.bank_resolved} in the bank
                        <span className="ml-1 font-mono text-slate-500">
                          {p.candidate_sample_ids.slice(0, 8).map((sid) => (
                            <button
                              key={sid}
                              type="button"
                              onClick={() => setSelected(sid)}
                              title="Select for a diagnostic preview"
                              data-testid={`speech-sample-${sid}`}
                              className={`mr-1 rounded px-1 hover:bg-slate-800 hover:text-sky-300 ${selected === sid ? "bg-slate-800 text-sky-300" : ""}`}
                            >
                              {sid}
                            </button>
                          ))}
                          {p.candidate_sample_ids.length > 8 ? `… (+${p.candidate_sample_ids.length - 8})` : ""}
                        </span>
                        {p.unresolved_sample_ids.length > 0 && <span className="text-amber-300"> ({p.unresolved_sample_ids.length} not in the bank: {p.unresolved_sample_ids.slice(0, 6).join(", ")}{p.unresolved_sample_ids.length > 6 ? "…" : ""})</span>}
                      </span>
                    ) : (
                      <span className="text-amber-300">no graffiti table for this group (reported, not hidden)</span>
                    )}
                    <span className="text-slate-500"> · pool {p.pool_id}</span>
                  </li>
                ))}
              </ol>
            </div>
          ))}
          <Gen7SpeechPreview projectId={projectId} installRoot={installRoot} bankStem="pbp_eng" sampleId={selected} />
        </div>
      )}
    </div>
  );
}

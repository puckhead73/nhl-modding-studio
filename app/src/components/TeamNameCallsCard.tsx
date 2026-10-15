import { assetRoot } from "../lib/assetRoot";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Card, Pill } from "./primitives";
import { Gen7SpeechPreview } from "./Gen7SpeechPreview";

/// Phase 2.7 (21 Sep 2026) — the **read-only** name calls of a team on the gen7 titles (PS3 /
/// Xbox 360 NHL 11–Legacy): the play-by-play samples tagged with the team's `audioid` under the
/// `team_ID` parameter (research 2026-09-21 handoff 07 §1–2), grouped by how many parameters the
/// samples carry besides the team, each group resolved against the sample bank, plus the keyword
/// index's mention count where the title ships one. Which line plays, and when, is runtime;
/// nothing is written here. Since 24 Sep 2026 (P4.3) a sample id can be selected for a
/// **diagnostic** EALayer3 v1 preview on the measured tuples (research R77 / R80) — labelled
/// "start handling unresolved; not game-verified"; the route refuses titles outside them. Titles
/// outside the measured coverage show the reason instead of a guess.
export function TeamNameCallsCard({ projectId, teamId }: { projectId: number; teamId: number }) {
  const { install, iso } = useProject();
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  const [selected, setSelected] = useState<number | null>(null);
  const q = useQuery({
    queryKey: ["team-name-calls", projectId, teamId, installRoot],
    queryFn: () => call("team_name_calls", { projectId, teamId, installRoot: installRoot! }),
    enabled: installRoot != null,
    retry: false,
  });
  if (installRoot == null) return null;
  if (q.isLoading) return null;
  if (q.isError) {
    return (
      <Card title="Name calls (commentary, read-only)">
        <p className="text-xs text-slate-500" data-testid="name-calls-unavailable">
          {String(q.error)}
        </p>
      </Card>
    );
  }
  const d = q.data!;
  const audio = d.team_audio_id == null ? "—" : String(d.team_audio_id);
  return (
    <Card
      title="Name calls (commentary, read-only)"
      subtitle={`Audio id ${audio}${d.team_tag == null ? "" : ` · play-by-play team_ID tag ${d.team_tag}`}. The recorded samples the play-by-play tags with this team; which one plays is runtime.`}
    >
      <div className="text-xs" data-testid="name-calls">
        {d.tables.length > 0 ? (
          <>
            <p className="text-slate-300">
              <span data-testid="name-calls-summary">
                {d.sample_ids_distinct} distinct sample{d.sample_ids_distinct === 1 ? "" : "s"} in {d.tables.length} group{d.tables.length === 1 ? "" : "s"}
                {d.bank_resolved_distinct === d.sample_ids_distinct ? " — all in the sample bank" : ` — ${d.bank_resolved_distinct} in the sample bank`}
              </span>
              {d.keyword_mentions != null && (
                <span className="ml-2 text-slate-500">
                  · keyword index: {d.keyword_mentions} mention{d.keyword_mentions === 1 ? "" : "s"}
                </span>
              )}
            </p>
            <ul className="mt-2 space-y-1">
              {d.tables.map((t) => (
                <li key={t.num_tags} className="text-slate-300" data-testid={`name-calls-group-${t.num_tags}`}>
                  <Pill tone={t.bank_resolved === t.sample_count ? "ok" : "warn"}>
                    {t.num_tags === 1 ? "team alone" : `team + ${t.num_tags - 1} parameter${t.num_tags === 2 ? "" : "s"}`}
                  </Pill>{" "}
                  {t.sample_count} sample{t.sample_count === 1 ? "" : "s"}, {t.bank_resolved} in the bank
                  <span className="ml-2 font-mono text-slate-500">
                    {t.sample_ids.map((sid) => (
                      <button
                        key={sid}
                        type="button"
                        onClick={() => setSelected(sid)}
                        title="Select for a diagnostic preview"
                        data-testid={`name-calls-sample-${sid}`}
                        className={`mr-1 rounded px-1 hover:bg-slate-800 hover:text-sky-300 ${selected === sid ? "bg-slate-800 text-sky-300" : ""}`}
                      >
                        {sid}
                      </button>
                    ))}
                    {t.sample_count > t.sample_ids.length ? ` … (+${t.sample_count - t.sample_ids.length})` : ""}
                  </span>
                </li>
              ))}
            </ul>
            <Gen7SpeechPreview projectId={projectId} installRoot={installRoot} bankStem="pbp_eng" sampleId={selected} />
          </>
        ) : (
          <p className="text-amber-300" data-testid="name-calls-none">
            No recorded call for this team.
          </p>
        )}
        <ul className="mt-2 text-slate-500">
          {d.notes.map((n, i) => (
            <li key={i}>{n}</li>
          ))}
        </ul>
      </div>
    </Card>
  );
}

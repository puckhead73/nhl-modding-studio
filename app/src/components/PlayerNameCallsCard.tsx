import { assetRoot } from "../lib/assetRoot";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import type { NameCallTableDto } from "../bindings/NameCallTableDto";
import { Card, Pill } from "./primitives";
import { Gen7SpeechPreview } from "./Gen7SpeechPreview";

/// Phase 3.5 (21 Sep 2026) — a player's **read-only** recorded names on the gen7 titles (PS3 /
/// Xbox 360 NHL 11–Legacy), research 2026-09-21 handoff 07: the play-by-play's `playernames`
/// rows spelling the player's first / last name with their `player_ID` sample tables, and the
/// rink announcer's tables keyed by the bio audio id. Which line plays is runtime; nothing is
/// decoded, played or written. Titles outside the measured coverage show the reason.
export function PlayerNameCallsCard({ projectId, playerId }: { projectId: number; playerId: number }) {
  const { install, iso } = useProject();
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  // P4.5 (24 Sep 2026): a sample id chip selects a diagnostic EALayer3 v1 preview — play-by-play
  // samples from the `pbp_eng` bank, announcer samples from `ann_eng`; the route refuses tuples
  // outside the research's measured set.
  const [selected, setSelected] = useState<{ stem: "pbp_eng" | "ann_eng"; sid: number } | null>(null);
  const q = useQuery({
    queryKey: ["player-name-calls", projectId, playerId, installRoot],
    queryFn: () => call("player_name_calls", { projectId, playerId, installRoot: installRoot! }),
    enabled: installRoot != null,
    retry: false,
  });
  if (installRoot == null || q.isLoading) return null;
  if (q.isError) {
    return (
      <Card title="Recorded name (commentary, read-only)">
        <p className="text-xs text-slate-500" data-testid="player-calls-unavailable">
          {String(q.error)}
        </p>
      </Card>
    );
  }
  const d = q.data!;
  const groups = (tables: NameCallTableDto[], prefix: string, stem: "pbp_eng" | "ann_eng" = prefix === "ann" ? "ann_eng" : "pbp_eng") => (
    <ul className="mt-1 space-y-0.5">
      {tables.map((t) => (
        <li key={t.num_tags} className="text-slate-300" data-testid={`${prefix}-group-${t.num_tags}`}>
          <Pill tone={t.bank_resolved === t.sample_count ? "ok" : "warn"}>{t.num_tags === 1 ? "name alone" : `name + ${t.num_tags - 1} parameter${t.num_tags === 2 ? "" : "s"}`}</Pill>{" "}
          {t.sample_count} sample{t.sample_count === 1 ? "" : "s"}, {t.bank_resolved} in the bank
          <span className="ml-2 font-mono text-slate-500">
            {t.sample_ids.map((sid) => (
              <button
                key={sid}
                type="button"
                onClick={() => setSelected({ stem, sid })}
                title="Select for a diagnostic preview"
                data-testid={`${prefix}-sample-${sid}`}
                className={`mr-1 rounded px-1 hover:bg-slate-800 hover:text-sky-300 ${selected?.sid === sid && selected.stem === stem ? "bg-slate-800 text-sky-300" : ""}`}
              >
                {sid}
              </button>
            ))}
            {t.sample_count > t.sample_ids.length ? ` … (+${t.sample_count - t.sample_ids.length})` : ""}
          </span>
        </li>
      ))}
    </ul>
  );
  const ann = d.announcer_audio_id == null ? "—" : String(d.announcer_audio_id);
  return (
    <Card title="Recorded name (commentary, read-only)" subtitle="The samples the play-by-play tags with this player's name rows and the announcer tags with the bio audio id; which one plays is runtime.">
      <div className="grid gap-4 text-xs sm:grid-cols-2" data-testid="player-calls">
        <section data-testid="player-calls-pbp">
          <h3 className="font-semibold text-slate-100">Play-by-play {d.pbp_player_tag != null && <span className="font-normal text-slate-500">· player_ID tag {d.pbp_player_tag}</span>}</h3>
          {d.pbp_names.length === 0 ? (
            <p className="text-amber-300" data-testid="player-calls-pbp-none">
              No name row in the play-by-play catalogue spells this player's name.
            </p>
          ) : (
            d.pbp_names.map((n) => (
              <div key={`${n.is_first_name ? "f" : "l"}-${String(n.audio_id)}`} className="mt-1" data-testid={`player-calls-name-${n.is_first_name ? "first" : "last"}`}>
                <span className="text-slate-200">{n.name}</span>{" "}
                <span className="text-slate-500">
                  ({n.is_first_name ? "first name" : "last name"}, audio id {String(n.audio_id)}, bank {String(n.bank)})
                </span>
                {n.tables.length === 0 ? (
                  <p className="text-amber-300">in the catalogue, no recording (no play-by-play table keyed by this id)</p>
                ) : (
                  <>
                    <p className="text-slate-400">
                      {n.sample_ids_distinct} distinct sample{n.sample_ids_distinct === 1 ? "" : "s"}
                      {n.bank_resolved_distinct === n.sample_ids_distinct ? " — all in the sample bank" : ` — ${n.bank_resolved_distinct} in the sample bank`}
                    </p>
                    {groups(n.tables, `pbp-${n.is_first_name ? "first" : "last"}`)}
                  </>
                )}
              </div>
            ))
          )}
        </section>
        <section data-testid="player-calls-announcer">
          <h3 className="font-semibold text-slate-100">
            Arena announcer <span className="font-normal text-slate-500">· audio id {ann}{d.announcer_tag != null ? ` · playerid tag ${d.announcer_tag}` : ""}</span>
          </h3>
          {d.announcer_tables.length === 0 ? (
            <p className="text-amber-300" data-testid="player-calls-announcer-none">
              No announcer recording for this player.
            </p>
          ) : (
            <>
              <p className="text-slate-400" data-testid="player-calls-announcer-summary">
                {d.announcer_sample_ids_distinct} distinct sample{d.announcer_sample_ids_distinct === 1 ? "" : "s"}
                {d.announcer_bank_resolved_distinct === d.announcer_sample_ids_distinct ? " — all in the sample bank" : ` — ${d.announcer_bank_resolved_distinct} in the sample bank`}
              </p>
              {groups(d.announcer_tables, "ann")}
            </>
          )}
        </section>
      </div>
      <ul className="mt-2 text-xs text-slate-500">
        {d.notes.map((n, i) => (
          <li key={i}>{n}</li>
        ))}
      </ul>
      {selected && <Gen7SpeechPreview projectId={projectId} installRoot={installRoot} bankStem={selected.stem} sampleId={selected.sid} />}
    </Card>
  );
}

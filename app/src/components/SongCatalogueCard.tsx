import { assetRoot } from "../lib/assetRoot";
// Misc 5.4 / Teams 2.7 (PS4, 22 Sep 2026) — the read-only "Selectable songs (catalogue)" card.
//
// Lists what the NHL 17–21 catalogue offers as goal / power-play / win (and, from 19, intro)
// songs: identifier, title, composer, genre, eligibility. Titles and genres are the catalogue's
// dictionary keys resolved server-side through **this project's own attached dictionary**
// (coordinator review of receipts 10–17, item 2); an unresolved key is shown as the key with its
// state, never guessed, and the reply says which dictionary it resolved through. It never says
// which song the team *has*: the team's song cells are raw ids with no catalogue join (research
// R54 §2), and no bank binding exists, so there is no preview and no replacement. Titles without a
// catalogue show the reason in one line.

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Card, Pill } from "./primitives";

const LIMIT = 200;

const LOOKUP_NOTE: Record<string, string> = {
  exact: "resolved through this project's attached dictionary",
  "by-hash": "resolved by hash through this project's attached dictionary",
  absent: "not in this project's attached dictionary — shown as the key",
  "no-dictionary": "no dictionary is attached to this project — shown as the key",
  "not-a-key": "not a dictionary key",
};

export function SongCatalogueCard({ projectId }: { projectId: number }) {
  const { install, iso } = useProject();
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  const [query, setQuery] = useState("");
  const q = useQuery({
    queryKey: ["song-catalogue", projectId, installRoot, query],
    queryFn: () => call("song_catalogue", { projectId, installRoot: installRoot!, query, limit: LIMIT }),
    enabled: installRoot != null,
    retry: false,
    staleTime: Infinity,
  });
  if (installRoot == null) return null;
  if (q.isLoading) return null;
  if (q.isError) {
    return (
      <Card title="Selectable songs (catalogue, read-only)">
        <p className="text-xs text-rose-300" data-testid="songs-error">
          {String(q.error)}
        </p>
      </Card>
    );
  }
  const d = q.data!;
  if (!d.available) {
    return (
      <Card title="Selectable songs (catalogue, read-only)">
        <p className="text-xs text-slate-500" data-testid="songs-unavailable">
          {d.reason}
        </p>
      </Card>
    );
  }
  return (
    <Card
      title="Selectable songs (catalogue, read-only)"
      subtitle={`${d.rows_total} songs from ${d.composers} composers in the title's catalogue. What the catalogue offers — not which song this team has: the team's song cells are raw ids with no catalogue join.`}
    >
      <div className="space-y-2 text-xs" data-testid="songs-card">
        <div className="flex flex-wrap items-center gap-2">
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Filter by title key, identifier, composer or genre…"
            className="min-w-[16rem] flex-1 rounded border border-slate-700 bg-slate-950/60 px-2 py-1 text-slate-200"
            data-testid="songs-filter"
          />
          <span className="text-slate-500" data-testid="songs-matching">
            {d.matching} matching{d.truncated ? ` · showing the first ${d.rows.length}` : ""}
          </span>
        </div>
        <p className="text-slate-500" data-testid="songs-dictionary">
          {d.dictionary
            ? `Titles and genres resolved through this project's dictionary (${d.dictionary.locale}, ${d.dictionary.rows} rows).`
            : "No dictionary is attached to this project: titles and genres are shown as their keys."}
        </p>
        <div className="flex flex-wrap gap-1" data-testid="songs-genres">
          {d.genre_counts.map((g) => (
            <Pill key={g.genre}>
              <span title={`${g.key} — ${LOOKUP_NOTE[g.lookup] ?? g.lookup}`} data-testid={`songs-genre-${g.genre}`} className={g.label ? "" : "font-mono"}>
                {g.label ?? g.key}
              </span>{" "}
              {g.rows}
            </Pill>
          ))}
        </div>
        <table className="w-full text-left">
          <thead className="text-slate-500">
            <tr>
              <th className="pr-2 font-normal">Title</th>
              <th className="pr-2 font-normal">Composer</th>
              <th className="pr-2 font-normal">Genre</th>
              <th className="pr-2 font-normal">Eligible</th>
            </tr>
          </thead>
          <tbody>
            {d.rows.map((r) => (
              <tr key={`${r.index}-${r.identifier}`} data-testid={`songs-row-${r.identifier}`} className="border-t border-slate-800/60">
                <td
                  className={"pr-2 " + (r.title ? "text-slate-200" : "font-mono text-amber-200/80")}
                  title={`${r.name} — ${LOOKUP_NOTE[r.title_lookup] ?? r.title_lookup} · identifier ${r.identifier} · audioid ${r.audioid} · art id ${r.artid}`}
                  data-testid={`songs-title-${r.identifier}`}
                  data-lookup={r.title_lookup}
                >
                  {r.title ?? r.name}
                </td>
                <td className="pr-2 text-slate-300">{r.composer || "—"}</td>
                <td
                  className={"pr-2 " + (r.genre_label ? "text-slate-400" : "font-mono text-amber-200/80")}
                  title={r.genre_key ? `${r.genre_key} — ${LOOKUP_NOTE[r.genre_lookup ?? ""] ?? r.genre_lookup}` : undefined}
                >
                  {r.genre_label ?? r.genre_key ?? `genre ${r.genre}`}
                </td>
                <td className="pr-2 text-slate-400">
                  {[r.is_goal && "goal", r.is_powerplay && "power play", r.is_win && "win", r.is_intro === true && "intro"].filter(Boolean).join(" · ") || "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <ul className="list-disc space-y-0.5 pl-4 text-slate-500" data-testid="songs-notes">
          {d.notes.map((n, i) => (
            <li key={i}>{n}</li>
          ))}
        </ul>
      </div>
    </Card>
  );
}

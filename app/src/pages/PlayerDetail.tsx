import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { FieldRow } from "../components/FieldRow";
import { Card, Pill, Spinner } from "../components/primitives";

export function PlayerDetail() {
  const { id } = useParams<{ id: string }>();
  const { project } = useProject();
  const projectId = project!.project_id;
  const playerId = Number(id);

  const view = useQuery({
    queryKey: ["player", projectId, playerId],
    queryFn: () => call("player_view", { projectId, playerId }),
    enabled: Number.isFinite(playerId),
  });
  const career = useQuery({
    queryKey: ["career", projectId, playerId],
    queryFn: () => call("career_stats", { projectId, playerId }),
    enabled: Number.isFinite(playerId),
  });

  if (view.isLoading) return <Spinner label="Loading…" />;
  if (view.isError) return <p className="text-rose-400">{String(view.error)}</p>;
  const player = view.data;
  if (!player) return <p className="text-sm text-slate-400">Select a player.</p>;

  return (
    <div className="max-w-3xl space-y-3">
      <h2 className="flex items-center gap-2 text-lg font-semibold text-slate-100">
        Player #{player.player_id}
        <Pill tone="info">{player.kind}</Pill>
      </h2>

      <Card title="Identity">
        {player.identity.map((f) => (
          <FieldRow key={f.studio_field} projectId={projectId} record={player.bio_record} field={f} />
        ))}
      </Card>

      <Card title="Roster memberships">
        {player.memberships.length === 0 && (
          <p className="text-xs text-slate-400">Not on any roster.</p>
        )}
        {player.memberships.map((m, i) => (
          <div key={i} className="flex gap-4 py-0.5 text-sm text-slate-200">
            <span>Team {m.team_id}</span>
            <span>Jersey {m.jersey ?? "—"}</span>
            <span>Captaincy {m.captaincy ?? "—"}</span>
            <span>Status {m.roster_status ?? "—"}</span>
          </div>
        ))}
      </Card>

      {career.data && career.data.rows.length > 0 && (
        <Card title="Career stats" subtitle="stock, read-only">
          <div className="overflow-auto">
            <table className="w-full border-collapse text-sm">
              <thead>
                <tr className="bg-slate-800/60 text-xs text-slate-300">
                  {career.data.columns.map((c) => (
                    <th key={c} className="border-b border-slate-800 px-2 py-1 text-left">
                      {c}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {career.data.rows.map((row, i) => (
                  <tr key={i} className="text-slate-200">
                    {row.map((cell, j) => (
                      <td key={j} className="border-b border-slate-800/60 px-2 py-1">
                        {cell}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      )}

      <details className="rounded-lg border border-slate-800 bg-slate-900/60 px-4 py-3">
        <summary className="cursor-pointer text-sm font-semibold text-slate-100">
          Ratings ({player.ratings.length})
        </summary>
        {player.ratings_record === null && player.ratings.length > 0 && (
          <p className="mt-1 text-xs text-slate-400">No companion record — ratings read-only.</p>
        )}
        <div className="mt-2">
          {player.ratings.map((f) => (
            <FieldRow
              key={f.studio_field}
              projectId={projectId}
              record={player.ratings_record ?? player.bio_record}
              field={{ ...f, editable: f.editable && player.ratings_record !== null }}
            />
          ))}
        </div>
      </details>

      <details className="rounded-lg border border-slate-800 bg-slate-900/60 px-4 py-3">
        <summary className="cursor-pointer text-sm font-semibold text-slate-100">
          All bio fields ({player.all_fields.length})
        </summary>
        <div className="mt-2">
          {player.all_fields.map((f) => (
            <FieldRow key={f.studio_field} projectId={projectId} record={player.bio_record} field={f} />
          ))}
        </div>
      </details>
    </div>
  );
}
